use rusqlite::{params, Connection, OptionalExtension};

use crate::domain::models::YearTheme;
use crate::error::AppError;
use crate::id;
use crate::time;

pub trait YearThemeRepo {
    fn create(
        &self,
        conn: &Connection,
        workspace_id: &str,
        year: i64,
        name: &str,
        description_markdown: &Option<String>,
    ) -> Result<YearTheme, AppError>;
    fn get(&self, conn: &Connection, id: &str) -> Result<YearTheme, AppError>;
    fn list(&self, conn: &Connection, workspace_id: &str) -> Result<Vec<YearTheme>, AppError>;
    fn update(
        &self,
        conn: &Connection,
        id: &str,
        name: &str,
        description_markdown: &Option<String>,
    ) -> Result<YearTheme, AppError>;
    fn archive(&self, conn: &Connection, id: &str) -> Result<YearTheme, AppError>;
}

fn row_to_year_theme(row: &rusqlite::Row<'_>) -> Result<YearTheme, rusqlite::Error> {
    Ok(YearTheme {
        id: row.get(0)?,
        workspace_id: row.get(1)?,
        year: row.get(2)?,
        name: row.get(3)?,
        description_markdown: row.get(4)?,
        status: row.get(5)?,
        created_at: row.get(6)?,
        updated_at: row.get(7)?,
        archived_at: row.get(8)?,
    })
}

const SELECT_COLS: &str = "id, workspace_id, year, name, description_markdown, status, created_at, updated_at, archived_at";

pub struct SqliteYearThemeRepo;

impl YearThemeRepo for SqliteYearThemeRepo {
    fn create(
        &self,
        conn: &Connection,
        workspace_id: &str,
        year: i64,
        name: &str,
        description_markdown: &Option<String>,
    ) -> Result<YearTheme, AppError> {
        crate::domain::validation::validate_title(name)?;
        if !(1900..=9999).contains(&year) {
            return Err(AppError::validation("The year is invalid."));
        }
        let id = id::new_uuid();
        let now = time::now_rfc3339();
        conn.execute(
            "INSERT INTO year_themes (id, workspace_id, year, name, description_markdown, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?6)",
            params![id, workspace_id, year, name.trim(), description_markdown, now],
        )
        .map_err(map_constraint_violation)?;
        self.get(conn, &id)
    }

    fn get(&self, conn: &Connection, id: &str) -> Result<YearTheme, AppError> {
        conn.query_row(
            &format!("SELECT {SELECT_COLS} FROM year_themes WHERE id = ?1"),
            [id],
            row_to_year_theme,
        )
        .optional()?
        .ok_or_else(|| AppError::not_found("The year theme was not found."))
    }

    fn list(&self, conn: &Connection, workspace_id: &str) -> Result<Vec<YearTheme>, AppError> {
        let mut stmt = conn.prepare(&format!(
            "SELECT {SELECT_COLS} FROM year_themes WHERE workspace_id = ?1 AND archived_at IS NULL ORDER BY year DESC"
        ))?;
        let rows = stmt.query_map([workspace_id], row_to_year_theme)?;
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
        name: &str,
        description_markdown: &Option<String>,
    ) -> Result<YearTheme, AppError> {
        crate::domain::validation::validate_title(name)?;
        let updated = conn.execute(
            "UPDATE year_themes SET name = ?2, description_markdown = ?3, updated_at = ?4 WHERE id = ?1",
            params![id, name.trim(), description_markdown, time::now_rfc3339()],
        )?;
        if updated == 0 {
            return Err(AppError::not_found("The year theme was not found."));
        }
        self.get(conn, id)
    }

    fn archive(&self, conn: &Connection, id: &str) -> Result<YearTheme, AppError> {
        let updated = conn.execute(
            "UPDATE year_themes SET status = 'archived', archived_at = ?2, updated_at = ?2 WHERE id = ?1",
            params![id, time::now_rfc3339()],
        )?;
        if updated == 0 {
            return Err(AppError::not_found("The year theme was not found."));
        }
        self.get(conn, id)
    }
}

/// Map a SQLite constraint violation to a validation error.
pub(crate) fn map_constraint_violation(err: rusqlite::Error) -> AppError {
    if let rusqlite::Error::SqliteFailure(e, _) = &err {
        if e.code == rusqlite::ErrorCode::ConstraintViolation {
            return AppError::validation("The value conflicts with an existing record.");
        }
    }
    err.into()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::connection;
    use crate::db::migrations::run_migrations;
    use crate::repos::workspace_repo::{SqliteWorkspaceRepo, WorkspaceRepo};

    fn test_conn() -> Connection {
        let conn = connection::open_in_memory().expect("open in-memory connection");
        run_migrations(&conn).expect("run migrations");
        conn
    }

    fn with_workspace(conn: &Connection) -> String {
        SqliteWorkspaceRepo
            .create(conn, "Personal")
            .expect("create workspace")
            .id
    }

    #[test]
    fn duplicate_year_in_workspace_is_rejected() {
        let conn = test_conn();
        let ws_id = with_workspace(&conn);
        let repo = SqliteYearThemeRepo;
        repo.create(&conn, &ws_id, 2026, "Courage", &None)
            .expect("first");
        assert!(repo.create(&conn, &ws_id, 2026, "Second", &None).is_err());
    }

    #[test]
    fn list_and_archive() {
        let conn = test_conn();
        let ws_id = with_workspace(&conn);
        let repo = SqliteYearThemeRepo;
        let t1 = repo
            .create(&conn, &ws_id, 2026, "Courage", &None)
            .expect("create");
        repo.archive(&conn, &t1.id).expect("archive");
        let list = repo.list(&conn, &ws_id).expect("list");
        assert!(list.is_empty());
    }
}
