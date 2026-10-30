use rusqlite::{params, Connection, OptionalExtension};

use crate::domain::models::Project;
use crate::domain::types::{Priority, ProjectStatus};
use crate::error::AppError;
use crate::id;
use crate::repos::goal_repo::{GoalRepo, SqliteGoalRepo};
use crate::time;

use super::year_theme_repo::map_constraint_violation;

pub trait ProjectRepo {
    fn create(
        &self,
        conn: &Connection,
        workspace_id: &str,
        goal_id: &Option<String>,
        title: &str,
        description_markdown: &Option<String>,
        why_it_matters_markdown: &Option<String>,
        definition_of_done_markdown: &Option<String>,
    ) -> Result<Project, AppError>;
    fn get(&self, conn: &Connection, id: &str) -> Result<Project, AppError>;
    fn list(&self, conn: &Connection, workspace_id: &str) -> Result<Vec<Project>, AppError>;
    fn list_for_goal(&self, conn: &Connection, goal_id: &str) -> Result<Vec<Project>, AppError>;
    fn update(
        &self,
        conn: &Connection,
        id: &str,
        title: &str,
        description_markdown: &Option<String>,
        why_it_matters_markdown: &Option<String>,
        definition_of_done_markdown: &Option<String>,
        status: ProjectStatus,
        priority: Priority,
        target_date: &Option<String>,
    ) -> Result<Project, AppError>;
    fn archive(&self, conn: &Connection, id: &str) -> Result<Project, AppError>;
    /// Link a project to a goal (or detach it by passing `None`).
    fn move_goal(
        &self,
        conn: &Connection,
        id: &str,
        goal_id: &Option<String>,
    ) -> Result<Project, AppError>;
}

fn row_to_project(row: &rusqlite::Row<'_>) -> Result<Project, rusqlite::Error> {
    let status: String = row.get(5)?;
    let priority: String = row.get(6)?;
    Ok(Project {
        id: row.get(0)?,
        workspace_id: row.get(1)?,
        goal_id: row.get(2)?,
        title: row.get(3)?,
        description_markdown: row.get(4)?,
        why_it_matters_markdown: row.get(8)?,
        definition_of_done_markdown: row.get(9)?,
        status: status.parse().map_err(|_| {
            rusqlite::Error::InvalidColumnType(5, "status".to_string(), rusqlite::types::Type::Text)
        })?,
        priority: priority.parse().map_err(|_| {
            rusqlite::Error::InvalidColumnType(
                6,
                "priority".to_string(),
                rusqlite::types::Type::Text,
            )
        })?,
        target_date: row.get(7)?,
        created_at: row.get(10)?,
        updated_at: row.get(11)?,
        archived_at: row.get(12)?,
    })
}

const SELECT_COLS: &str = "id, workspace_id, goal_id, title, description_markdown, status, priority, target_date, why_it_matters_markdown, definition_of_done_markdown, created_at, updated_at, archived_at";

pub struct SqliteProjectRepo;

impl ProjectRepo for SqliteProjectRepo {
    fn create(
        &self,
        conn: &Connection,
        workspace_id: &str,
        goal_id: &Option<String>,
        title: &str,
        description_markdown: &Option<String>,
        why_it_matters_markdown: &Option<String>,
        definition_of_done_markdown: &Option<String>,
    ) -> Result<Project, AppError> {
        crate::domain::validation::validate_title(title)?;
        let id = id::new_uuid();
        let now = time::now_rfc3339();
        conn.execute(
            "INSERT INTO projects (id, workspace_id, goal_id, title, description_markdown, why_it_matters_markdown, definition_of_done_markdown, status, priority, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 'inbox', 'none', ?8, ?8)",
            params![id, workspace_id, goal_id, title.trim(), description_markdown, why_it_matters_markdown, definition_of_done_markdown, now],
        )
        .map_err(map_constraint_violation)?;
        self.get(conn, &id)
    }

    fn get(&self, conn: &Connection, id: &str) -> Result<Project, AppError> {
        conn.query_row(
            &format!("SELECT {SELECT_COLS} FROM projects WHERE id = ?1"),
            [id],
            row_to_project,
        )
        .optional()?
        .ok_or_else(|| AppError::not_found("The project was not found."))
    }

    fn list(&self, conn: &Connection, workspace_id: &str) -> Result<Vec<Project>, AppError> {
        let mut stmt = conn.prepare(&format!(
            "SELECT {SELECT_COLS} FROM projects WHERE workspace_id = ?1 AND archived_at IS NULL ORDER BY created_at"
        ))?;
        let rows = stmt.query_map([workspace_id], row_to_project)?;
        let mut result = Vec::new();
        for row in rows {
            result.push(row?);
        }
        Ok(result)
    }

    fn list_for_goal(&self, conn: &Connection, goal_id: &str) -> Result<Vec<Project>, AppError> {
        let mut stmt = conn.prepare(&format!(
            "SELECT {SELECT_COLS} FROM projects WHERE goal_id = ?1 AND archived_at IS NULL ORDER BY created_at"
        ))?;
        let rows = stmt.query_map([goal_id], row_to_project)?;
        let mut result = Vec::new();
        for row in rows {
            result.push(row?);
        }
        Ok(result)
    }

    fn update(
        &self,
        conn: &Connection,
        id: &str,
        title: &str,
        description_markdown: &Option<String>,
        why_it_matters_markdown: &Option<String>,
        definition_of_done_markdown: &Option<String>,
        status: ProjectStatus,
        priority: Priority,
        target_date: &Option<String>,
    ) -> Result<Project, AppError> {
        crate::domain::validation::validate_title(title)?;
        let updated = conn.execute(
            "UPDATE projects
             SET title = ?2, description_markdown = ?3, why_it_matters_markdown = ?4,
                 definition_of_done_markdown = ?5, status = ?6, priority = ?7, target_date = ?8, updated_at = ?9
             WHERE id = ?1",
            params![id, title.trim(), description_markdown, why_it_matters_markdown, definition_of_done_markdown, status.to_string(), priority.to_string(), target_date, time::now_rfc3339()],
        )?;
        if updated == 0 {
            return Err(AppError::not_found("The project was not found."));
        }
        self.get(conn, id)
    }

    fn archive(&self, conn: &Connection, id: &str) -> Result<Project, AppError> {
        let updated = conn.execute(
            "UPDATE projects SET status = 'archived', archived_at = ?2, updated_at = ?2 WHERE id = ?1",
            params![id, time::now_rfc3339()],
        )?;
        if updated == 0 {
            return Err(AppError::not_found("The project was not found."));
        }
        self.get(conn, id)
    }

    fn move_goal(
        &self,
        conn: &Connection,
        id: &str,
        goal_id: &Option<String>,
    ) -> Result<Project, AppError> {
        let project = self.get(conn, id)?;
        if let Some(goal_id) = goal_id {
            let goal = SqliteGoalRepo.get(conn, goal_id)?;
            if goal.workspace_id != project.workspace_id {
                return Err(AppError::validation(
                    "The goal belongs to a different workspace.",
                ));
            }
        }
        let updated = conn.execute(
            "UPDATE projects SET goal_id = ?2, updated_at = ?3 WHERE id = ?1",
            params![id, goal_id, time::now_rfc3339()],
        )?;
        if updated == 0 {
            return Err(AppError::not_found("The project was not found."));
        }
        log::info!("project {id}: linked to goal {goal_id:?}");
        self.get(conn, id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::connection;
    use crate::db::migrations::run_migrations;
    use crate::repos::goal_repo::{GoalRepo, SqliteGoalRepo};
    use crate::repos::workspace_repo::{SqliteWorkspaceRepo, WorkspaceRepo};

    fn test_conn() -> (Connection, String) {
        let conn = connection::open_in_memory().expect("open in-memory connection");
        run_migrations(&conn).expect("run migrations");
        let ws_id = SqliteWorkspaceRepo
            .create(&conn, "Personal")
            .expect("ws")
            .id;
        (conn, ws_id)
    }

    #[test]
    fn goal_contains_multiple_projects() {
        let (conn, ws_id) = test_conn();
        let goal = SqliteGoalRepo
            .create(&conn, &ws_id, &None, "Learn Rust", &None, &None, &None)
            .expect("goal");
        let repo = SqliteProjectRepo;
        repo.create(
            &conn,
            &ws_id,
            &Some(goal.id.clone()),
            "Project A",
            &None,
            &None,
            &None,
        )
        .expect("project a");
        repo.create(
            &conn,
            &ws_id,
            &Some(goal.id.clone()),
            "Project B",
            &None,
            &None,
            &None,
        )
        .expect("project b");
        let projects = repo.list_for_goal(&conn, &goal.id).expect("list");
        assert_eq!(projects.len(), 2);
    }

    #[test]
    fn project_crud() {
        let (conn, ws_id) = test_conn();
        let repo = SqliteProjectRepo;
        let project = repo
            .create(
                &conn,
                &ws_id,
                &None,
                "Standalone project",
                &None,
                &None,
                &None,
            )
            .expect("create");
        let updated = repo
            .update(
                &conn,
                &project.id,
                "Renamed",
                &None,
                &None,
                &None,
                ProjectStatus::Active,
                Priority::Normal,
                &None,
            )
            .expect("update");
        assert_eq!(updated.status, ProjectStatus::Active);
    }

    #[test]
    fn project_can_be_linked_to_goal_and_detached() {
        let (conn, ws_id) = test_conn();
        let goal = SqliteGoalRepo
            .create(&conn, &ws_id, &None, "Learn Rust", &None, &None, &None)
            .expect("goal");
        let repo = SqliteProjectRepo;
        let project = repo
            .create(
                &conn,
                &ws_id,
                &None,
                "Practice project",
                &None,
                &None,
                &None,
            )
            .expect("create");
        let linked = repo
            .move_goal(&conn, &project.id, &Some(goal.id.clone()))
            .expect("link");
        assert_eq!(linked.goal_id.as_deref(), Some(goal.id.as_str()));
        let detached = repo.move_goal(&conn, &project.id, &None).expect("detach");
        assert_eq!(detached.goal_id, None);
    }

    #[test]
    fn linking_to_missing_goal_is_rejected() {
        let (conn, ws_id) = test_conn();
        let repo = SqliteProjectRepo;
        let project = repo
            .create(
                &conn,
                &ws_id,
                &None,
                "Practice project",
                &None,
                &None,
                &None,
            )
            .expect("create");
        assert!(repo
            .move_goal(&conn, &project.id, &Some("missing".to_string()))
            .is_err());
    }
}
