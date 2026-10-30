use rusqlite::{params, Connection, OptionalExtension};

use crate::domain::models::QuarterFocus;
use crate::domain::types::Priority;
use crate::error::AppError;
use crate::id;
use crate::time;

use super::year_theme_repo::map_constraint_violation;

pub trait QuarterFocusRepo {
    fn create(
        &self,
        conn: &Connection,
        quarter_id: &str,
        goal_id: &Option<String>,
        project_id: &Option<String>,
        priority: Priority,
        intention_markdown: &Option<String>,
        success_signal_markdown: &Option<String>,
    ) -> Result<QuarterFocus, AppError>;
    fn get(&self, conn: &Connection, id: &str) -> Result<QuarterFocus, AppError>;
    fn list_for_quarter(
        &self,
        conn: &Connection,
        quarter_id: &str,
    ) -> Result<Vec<QuarterFocus>, AppError>;
    fn update(
        &self,
        conn: &Connection,
        id: &str,
        priority: Priority,
        intention_markdown: &Option<String>,
        success_signal_markdown: &Option<String>,
        status: &str,
    ) -> Result<QuarterFocus, AppError>;
}

fn row_to_quarter_focus(row: &rusqlite::Row<'_>) -> Result<QuarterFocus, rusqlite::Error> {
    Ok(QuarterFocus {
        id: row.get(0)?,
        quarter_id: row.get(1)?,
        goal_id: row.get(2)?,
        project_id: row.get(3)?,
        priority: row.get::<_, String>(4)?.parse().map_err(|_| {
            rusqlite::Error::InvalidColumnType(
                4,
                "priority".to_string(),
                rusqlite::types::Type::Text,
            )
        })?,
        intention_markdown: row.get(5)?,
        success_signal_markdown: row.get(6)?,
        status: row.get(7)?,
        created_at: row.get(8)?,
        updated_at: row.get(9)?,
    })
}

const SELECT_COLS: &str = "id, quarter_id, goal_id, project_id, priority, intention_markdown, success_signal_markdown, status, created_at, updated_at";

pub struct SqliteQuarterFocusRepo;

impl QuarterFocusRepo for SqliteQuarterFocusRepo {
    fn create(
        &self,
        conn: &Connection,
        quarter_id: &str,
        goal_id: &Option<String>,
        project_id: &Option<String>,
        priority: Priority,
        intention_markdown: &Option<String>,
        success_signal_markdown: &Option<String>,
    ) -> Result<QuarterFocus, AppError> {
        if goal_id.is_none() && project_id.is_none() {
            return Err(AppError::validation(
                "Quarter focus must reference a goal or a project.",
            ));
        }
        if goal_id.is_some() && project_id.is_some() {
            return Err(AppError::validation(
                "Quarter focus must reference exactly one of a goal or a project.",
            ));
        }
        let id = id::new_uuid();
        let now = time::now_rfc3339();
        conn.execute(
            "INSERT INTO quarter_focus (id, quarter_id, goal_id, project_id, priority, intention_markdown, success_signal_markdown, status, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 'proposed', ?8, ?8)",
            params![id, quarter_id, goal_id, project_id, priority.to_string(), intention_markdown, success_signal_markdown, now],
        )
        .map_err(map_constraint_violation)?;
        self.get(conn, &id)
    }

    fn get(&self, conn: &Connection, id: &str) -> Result<QuarterFocus, AppError> {
        conn.query_row(
            &format!("SELECT {SELECT_COLS} FROM quarter_focus WHERE id = ?1"),
            [id],
            row_to_quarter_focus,
        )
        .optional()?
        .ok_or_else(|| AppError::not_found("The quarter focus was not found."))
    }

    fn list_for_quarter(
        &self,
        conn: &Connection,
        quarter_id: &str,
    ) -> Result<Vec<QuarterFocus>, AppError> {
        let mut stmt = conn.prepare(&format!(
            "SELECT {SELECT_COLS} FROM quarter_focus WHERE quarter_id = ?1 ORDER BY created_at"
        ))?;
        let rows = stmt.query_map([quarter_id], row_to_quarter_focus)?;
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
        priority: Priority,
        intention_markdown: &Option<String>,
        success_signal_markdown: &Option<String>,
        status: &str,
    ) -> Result<QuarterFocus, AppError> {
        let updated = conn.execute(
            "UPDATE quarter_focus
             SET priority = ?2, intention_markdown = ?3, success_signal_markdown = ?4, status = ?5, updated_at = ?6
             WHERE id = ?1",
            params![id, priority.to_string(), intention_markdown, success_signal_markdown, status, time::now_rfc3339()],
        )?;
        if updated == 0 {
            return Err(AppError::not_found("The quarter focus was not found."));
        }
        self.get(conn, id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::connection;
    use crate::db::migrations::run_migrations;
    use crate::domain::models::{Goal, Project};
    use crate::repos::goal_repo::{GoalRepo, SqliteGoalRepo};
    use crate::repos::project_repo::{ProjectRepo, SqliteProjectRepo};
    use crate::repos::quarter_repo::{QuarterRepo, SqliteQuarterRepo};
    use crate::repos::workspace_repo::{SqliteWorkspaceRepo, WorkspaceRepo};
    use crate::repos::year_theme_repo::{SqliteYearThemeRepo, YearThemeRepo};

    fn test_conn() -> (Connection, String, String) {
        let conn = connection::open_in_memory().expect("open in-memory connection");
        run_migrations(&conn).expect("run migrations");
        let ws_id = SqliteWorkspaceRepo
            .create(&conn, "Personal")
            .expect("ws")
            .id;
        let theme = SqliteYearThemeRepo
            .create(&conn, &ws_id, 2026, "Courage", &None)
            .expect("theme");
        let quarter = SqliteQuarterRepo
            .create(&conn, &theme.id, "Q1", "2026-01-01", "2026-03-31")
            .expect("quarter");
        (conn, ws_id, quarter.id)
    }

    fn seed_goal_and_project(conn: &Connection, ws_id: &str) -> (Goal, Project) {
        let goal = SqliteGoalRepo
            .create(conn, ws_id, &None, "Learn Rust", &None, &None, &None)
            .expect("goal");
        let project = SqliteProjectRepo
            .create(
                conn,
                ws_id,
                &Some(goal.id.clone()),
                "Build a small app",
                &None,
                &None,
                &None,
            )
            .expect("project");
        (goal, project)
    }

    #[test]
    fn focus_requires_one_reference() {
        let (conn, ws_id, quarter_id) = test_conn();
        let (goal, project) = seed_goal_and_project(&conn, &ws_id);

        assert!(SqliteQuarterFocusRepo
            .create(
                &conn,
                &quarter_id,
                &None,
                &None,
                Priority::Normal,
                &None,
                &None
            )
            .is_err());
        assert!(SqliteQuarterFocusRepo
            .create(
                &conn,
                &quarter_id,
                &Some(goal.id.clone()),
                &Some(project.id.clone()),
                Priority::Normal,
                &None,
                &None
            )
            .is_err());
    }

    #[test]
    fn focus_on_goal_is_valid() {
        let (conn, ws_id, quarter_id) = test_conn();
        let (goal, _) = seed_goal_and_project(&conn, &ws_id);

        let focus = SqliteQuarterFocusRepo
            .create(
                &conn,
                &quarter_id,
                &Some(goal.id.clone()),
                &None,
                Priority::High,
                &None,
                &None,
            )
            .expect("create focus");
        assert_eq!(focus.goal_id, Some(goal.id));
    }
}
