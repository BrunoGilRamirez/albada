use rusqlite::{params, Connection, OptionalExtension};

use crate::domain::models::Goal;
use crate::domain::types::{GoalStatus, Priority};
use crate::error::AppError;
use crate::id;
use crate::time;

use super::year_theme_repo::map_constraint_violation;

pub trait GoalRepo {
    fn create(
        &self,
        conn: &Connection,
        workspace_id: &str,
        year_theme_id: &Option<String>,
        title: &str,
        description_markdown: &Option<String>,
        why_it_matters_markdown: &Option<String>,
        success_definition_markdown: &Option<String>,
    ) -> Result<Goal, AppError>;
    fn get(&self, conn: &Connection, id: &str) -> Result<Goal, AppError>;
    fn list(&self, conn: &Connection, workspace_id: &str) -> Result<Vec<Goal>, AppError>;
    fn update(
        &self,
        conn: &Connection,
        id: &str,
        title: &str,
        description_markdown: &Option<String>,
        why_it_matters_markdown: &Option<String>,
        success_definition_markdown: &Option<String>,
        status: GoalStatus,
        priority: Priority,
        target_date: &Option<String>,
    ) -> Result<Goal, AppError>;
    fn archive(&self, conn: &Connection, id: &str) -> Result<Goal, AppError>;
}

fn row_to_goal(row: &rusqlite::Row<'_>) -> Result<Goal, rusqlite::Error> {
    let status: String = row.get(5)?;
    let priority: String = row.get(6)?;
    Ok(Goal {
        id: row.get(0)?,
        workspace_id: row.get(1)?,
        year_theme_id: row.get(2)?,
        title: row.get(3)?,
        description_markdown: row.get(4)?,
        why_it_matters_markdown: row.get(8)?,
        success_definition_markdown: row.get(9)?,
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

const SELECT_COLS: &str = "id, workspace_id, year_theme_id, title, description_markdown, status, priority, target_date, why_it_matters_markdown, success_definition_markdown, created_at, updated_at, archived_at";

pub struct SqliteGoalRepo;

impl GoalRepo for SqliteGoalRepo {
    fn create(
        &self,
        conn: &Connection,
        workspace_id: &str,
        year_theme_id: &Option<String>,
        title: &str,
        description_markdown: &Option<String>,
        why_it_matters_markdown: &Option<String>,
        success_definition_markdown: &Option<String>,
    ) -> Result<Goal, AppError> {
        crate::domain::validation::validate_title(title)?;
        let id = id::new_uuid();
        let now = time::now_rfc3339();
        conn.execute(
            "INSERT INTO goals (id, workspace_id, year_theme_id, title, description_markdown, why_it_matters_markdown, success_definition_markdown, status, priority, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 'inbox', 'none', ?8, ?8)",
            params![id, workspace_id, year_theme_id, title.trim(), description_markdown, why_it_matters_markdown, success_definition_markdown, now],
        )
        .map_err(map_constraint_violation)?;
        self.get(conn, &id)
    }

    fn get(&self, conn: &Connection, id: &str) -> Result<Goal, AppError> {
        conn.query_row(
            &format!("SELECT {SELECT_COLS} FROM goals WHERE id = ?1"),
            [id],
            row_to_goal,
        )
        .optional()?
        .ok_or_else(|| AppError::not_found("The goal was not found."))
    }

    fn list(&self, conn: &Connection, workspace_id: &str) -> Result<Vec<Goal>, AppError> {
        let mut stmt = conn.prepare(&format!(
            "SELECT {SELECT_COLS} FROM goals WHERE workspace_id = ?1 AND archived_at IS NULL ORDER BY created_at"
        ))?;
        let rows = stmt.query_map([workspace_id], row_to_goal)?;
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
        success_definition_markdown: &Option<String>,
        status: GoalStatus,
        priority: Priority,
        target_date: &Option<String>,
    ) -> Result<Goal, AppError> {
        crate::domain::validation::validate_title(title)?;
        let updated = conn.execute(
            "UPDATE goals
             SET title = ?2, description_markdown = ?3, why_it_matters_markdown = ?4,
                 success_definition_markdown = ?5, status = ?6, priority = ?7, target_date = ?8, updated_at = ?9
             WHERE id = ?1",
            params![id, title.trim(), description_markdown, why_it_matters_markdown, success_definition_markdown, status.to_string(), priority.to_string(), target_date, time::now_rfc3339()],
        )?;
        if updated == 0 {
            return Err(AppError::not_found("The goal was not found."));
        }
        self.get(conn, id)
    }

    fn archive(&self, conn: &Connection, id: &str) -> Result<Goal, AppError> {
        let updated = conn.execute(
            "UPDATE goals SET status = 'archived', archived_at = ?2, updated_at = ?2 WHERE id = ?1",
            params![id, time::now_rfc3339()],
        )?;
        if updated == 0 {
            return Err(AppError::not_found("The goal was not found."));
        }
        self.get(conn, id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::connection;
    use crate::db::migrations::run_migrations;
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
    fn goal_crud_and_archive() {
        let (conn, ws_id) = test_conn();
        let repo = SqliteGoalRepo;
        let goal = repo
            .create(&conn, &ws_id, &None, "Learn Rust", &None, &None, &None)
            .expect("create");
        assert_eq!(goal.status, GoalStatus::Inbox);

        let updated = repo
            .update(
                &conn,
                &goal.id,
                "Master Rust",
                &None,
                &None,
                &None,
                GoalStatus::Active,
                Priority::High,
                &Some("2026-12-31".into()),
            )
            .expect("update");
        assert_eq!(updated.status, GoalStatus::Active);
        assert_eq!(updated.priority, Priority::High);

        let archived = repo.archive(&conn, &goal.id).expect("archive");
        assert_eq!(archived.status, GoalStatus::Archived);
        assert!(archived.archived_at.is_some());

        let list = repo.list(&conn, &ws_id).expect("list");
        assert!(list.is_empty());
    }
}
