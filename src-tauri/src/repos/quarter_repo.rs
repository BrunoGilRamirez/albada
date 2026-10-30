use rusqlite::{params, Connection, OptionalExtension};

use crate::domain::models::Quarter;
use crate::error::AppError;
use crate::id;
use crate::time;

use super::year_theme_repo::map_constraint_violation;

pub trait QuarterRepo {
    fn create(
        &self,
        conn: &Connection,
        year_theme_id: &str,
        code: &str,
        start_date: &str,
        end_date: &str,
    ) -> Result<Quarter, AppError>;
    fn get(&self, conn: &Connection, id: &str) -> Result<Quarter, AppError>;
    fn list(&self, conn: &Connection, year_theme_id: &str) -> Result<Vec<Quarter>, AppError>;
    fn update(
        &self,
        conn: &Connection,
        id: &str,
        start_date: &str,
        end_date: &str,
        status: &str,
    ) -> Result<Quarter, AppError>;
}

fn row_to_quarter(row: &rusqlite::Row<'_>) -> Result<Quarter, rusqlite::Error> {
    Ok(Quarter {
        id: row.get(0)?,
        year_theme_id: row.get(1)?,
        code: row.get(2)?,
        start_date: row.get(3)?,
        end_date: row.get(4)?,
        status: row.get(5)?,
        created_at: row.get(6)?,
        updated_at: row.get(7)?,
    })
}

const SELECT_COLS: &str =
    "id, year_theme_id, code, start_date, end_date, status, created_at, updated_at";

pub struct SqliteQuarterRepo;

impl QuarterRepo for SqliteQuarterRepo {
    fn create(
        &self,
        conn: &Connection,
        year_theme_id: &str,
        code: &str,
        start_date: &str,
        end_date: &str,
    ) -> Result<Quarter, AppError> {
        let normalized = code.trim().to_uppercase();
        if !["Q1", "Q2", "Q3", "Q4"].contains(&normalized.as_str()) {
            return Err(AppError::validation(
                "The quarter code must be Q1, Q2, Q3, or Q4.",
            ));
        }
        if start_date.is_empty() || end_date.is_empty() {
            return Err(AppError::validation("Quarter dates are required."));
        }
        if start_date > end_date {
            return Err(AppError::validation(
                "The quarter start date cannot be after its end date.",
            ));
        }
        let id = id::new_uuid();
        let now = time::now_rfc3339();
        conn.execute(
            "INSERT INTO quarters (id, year_theme_id, code, start_date, end_date, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?6)",
            params![id, year_theme_id, normalized, start_date, end_date, now],
        )
        .map_err(map_constraint_violation)?;
        self.get(conn, &id)
    }

    fn get(&self, conn: &Connection, id: &str) -> Result<Quarter, AppError> {
        conn.query_row(
            &format!("SELECT {SELECT_COLS} FROM quarters WHERE id = ?1"),
            [id],
            row_to_quarter,
        )
        .optional()?
        .ok_or_else(|| AppError::not_found("The quarter was not found."))
    }

    fn list(&self, conn: &Connection, year_theme_id: &str) -> Result<Vec<Quarter>, AppError> {
        let mut stmt = conn.prepare(&format!(
            "SELECT {SELECT_COLS} FROM quarters WHERE year_theme_id = ?1 ORDER BY code"
        ))?;
        let rows = stmt.query_map([year_theme_id], row_to_quarter)?;
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
        start_date: &str,
        end_date: &str,
        status: &str,
    ) -> Result<Quarter, AppError> {
        let updated = conn.execute(
            "UPDATE quarters SET start_date = ?2, end_date = ?3, status = ?4, updated_at = ?5 WHERE id = ?1",
            params![id, start_date, end_date, status, time::now_rfc3339()],
        )?;
        if updated == 0 {
            return Err(AppError::not_found("The quarter was not found."));
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
    use crate::repos::year_theme_repo::{SqliteYearThemeRepo, YearThemeRepo};

    fn test_conn() -> (Connection, String) {
        let conn = connection::open_in_memory().expect("open in-memory connection");
        run_migrations(&conn).expect("run migrations");
        let ws_id = SqliteWorkspaceRepo
            .create(&conn, "Personal")
            .expect("ws")
            .id;
        let theme = SqliteYearThemeRepo
            .create(&conn, &ws_id, 2026, "Courage", &None)
            .expect("theme");
        (conn, theme.id)
    }

    #[test]
    fn invalid_code_is_rejected() {
        let (conn, theme_id) = test_conn();
        assert!(SqliteQuarterRepo
            .create(&conn, &theme_id, "Q5", "2026-01-01", "2026-03-31")
            .is_err());
    }

    #[test]
    fn duplicate_quarter_is_rejected() {
        let (conn, theme_id) = test_conn();
        let repo = SqliteQuarterRepo;
        repo.create(&conn, &theme_id, "Q1", "2026-01-01", "2026-03-31")
            .expect("first");
        assert!(repo
            .create(&conn, &theme_id, "q1", "2026-01-01", "2026-03-31")
            .is_err());
    }
}
