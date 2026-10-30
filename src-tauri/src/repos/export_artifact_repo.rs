use rusqlite::{params, Connection};

use crate::domain::models::ExportArtifact;
use crate::error::AppError;
use crate::id;
use crate::time;

pub trait ExportArtifactRepo {
    fn create(
        &self,
        conn: &Connection,
        export_job_id: &str,
        format: &str,
        path: &str,
        checksum: &Option<String>,
        size_bytes: &Option<i64>,
    ) -> Result<ExportArtifact, AppError>;
    fn list_for_job(
        &self,
        conn: &Connection,
        export_job_id: &str,
    ) -> Result<Vec<ExportArtifact>, AppError>;
    fn get(&self, conn: &Connection, id: &str) -> Result<ExportArtifact, AppError>;
}

const SELECT_COLS: &str =
    "id, export_job_id, format, path, checksum, size_bytes, created_at, deleted_at";

fn row_to_artifact(row: &rusqlite::Row<'_>) -> Result<ExportArtifact, rusqlite::Error> {
    Ok(ExportArtifact {
        id: row.get(0)?,
        export_job_id: row.get(1)?,
        format: row.get(2)?,
        path: row.get(3)?,
        checksum: row.get(4)?,
        size_bytes: row.get(5)?,
        created_at: row.get(6)?,
        deleted_at: row.get(7)?,
    })
}

pub struct SqliteExportArtifactRepo;

impl ExportArtifactRepo for SqliteExportArtifactRepo {
    fn create(
        &self,
        conn: &Connection,
        export_job_id: &str,
        format: &str,
        path: &str,
        checksum: &Option<String>,
        size_bytes: &Option<i64>,
    ) -> Result<ExportArtifact, AppError> {
        let id = id::new_uuid();
        let now = time::now_rfc3339();
        conn.execute(
            "INSERT INTO export_artifacts (id, export_job_id, format, path, checksum, size_bytes, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![id, export_job_id, format, path, checksum, size_bytes, now],
        )?;
        self.get(conn, &id)
    }

    fn list_for_job(
        &self,
        conn: &Connection,
        export_job_id: &str,
    ) -> Result<Vec<ExportArtifact>, AppError> {
        let mut stmt = conn.prepare(&format!(
            "SELECT {SELECT_COLS} FROM export_artifacts WHERE export_job_id = ?1 AND deleted_at IS NULL ORDER BY created_at"
        ))?;
        let rows = stmt.query_map([export_job_id], row_to_artifact)?;
        let mut result = Vec::new();
        for row in rows {
            result.push(row?);
        }
        Ok(result)
    }

    fn get(&self, conn: &Connection, id: &str) -> Result<ExportArtifact, AppError> {
        conn.query_row(
            &format!("SELECT {SELECT_COLS} FROM export_artifacts WHERE id = ?1"),
            [id],
            row_to_artifact,
        )
        .map_err(Into::into)
    }
}

impl SqliteExportArtifactRepo {
    pub fn get(&self, conn: &Connection, id: &str) -> Result<ExportArtifact, AppError> {
        conn.query_row(
            &format!("SELECT {SELECT_COLS} FROM export_artifacts WHERE id = ?1"),
            [id],
            row_to_artifact,
        )
        .map_err(Into::into)
    }
}
