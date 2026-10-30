use rusqlite::{params, Connection, OptionalExtension};

use crate::error::AppError;
use crate::{id, time};

/// One import run. Not a domain entity; this is import tracking/provenance.
#[derive(Debug, Clone)]
pub struct ImportBatch {
    pub id: String,
    pub workspace_id: String,
    pub source_root: String,
    pub started_at: String,
    pub completed_at: Option<String>,
    pub status: String,
    pub summary_json: Option<String>,
    pub created_at: String,
}

const COLS: &str =
    "id, workspace_id, source_root, started_at, completed_at, status, summary_json, created_at";

fn row_to_batch(row: &rusqlite::Row<'_>) -> Result<ImportBatch, rusqlite::Error> {
    Ok(ImportBatch {
        id: row.get(0)?,
        workspace_id: row.get(1)?,
        source_root: row.get(2)?,
        started_at: row.get(3)?,
        completed_at: row.get(4)?,
        status: row.get(5)?,
        summary_json: row.get(6)?,
        created_at: row.get(7)?,
    })
}

pub trait ImportBatchRepo {
    fn create(
        &self,
        conn: &Connection,
        workspace_id: &str,
        source_root: &str,
    ) -> Result<ImportBatch, AppError>;
    fn get(&self, conn: &Connection, id: &str) -> Result<ImportBatch, AppError>;
    fn complete(
        &self,
        conn: &Connection,
        id: &str,
        summary_json: &str,
    ) -> Result<ImportBatch, AppError>;
    /// Mark a batch as failed with a fixed, content-safe summary. Used when the
    /// blocking worker cannot finish the import; the batch must never be left
    /// permanently `running`.
    fn fail(
        &self,
        conn: &Connection,
        id: &str,
        summary_json: &str,
    ) -> Result<ImportBatch, AppError>;
}

pub struct SqliteImportBatchRepo;

impl ImportBatchRepo for SqliteImportBatchRepo {
    fn create(
        &self,
        conn: &Connection,
        workspace_id: &str,
        source_root: &str,
    ) -> Result<ImportBatch, AppError> {
        let id = id::new_uuid();
        let now = time::now_rfc3339();
        conn.execute(
            "INSERT INTO import_batches (id, workspace_id, source_root, started_at, status, created_at)
             VALUES (?1, ?2, ?3, ?4, 'running', ?4)",
            params![id, workspace_id, source_root, now],
        )?;
        self.get(conn, &id)
    }

    fn get(&self, conn: &Connection, id: &str) -> Result<ImportBatch, AppError> {
        conn.query_row(
            &format!("SELECT {COLS} FROM import_batches WHERE id = ?1"),
            [id],
            row_to_batch,
        )
        .optional()?
        .ok_or_else(|| AppError::not_found("The import batch was not found."))
    }

    fn complete(
        &self,
        conn: &Connection,
        id: &str,
        summary_json: &str,
    ) -> Result<ImportBatch, AppError> {
        let updated = conn.execute(
            "UPDATE import_batches SET completed_at = ?2, status = 'completed', summary_json = ?3 WHERE id = ?1",
            params![id, time::now_rfc3339(), summary_json],
        )?;
        if updated == 0 {
            return Err(AppError::not_found("The import batch was not found."));
        }
        self.get(conn, id)
    }

    fn fail(
        &self,
        conn: &Connection,
        id: &str,
        summary_json: &str,
    ) -> Result<ImportBatch, AppError> {
        let updated = conn.execute(
            "UPDATE import_batches SET completed_at = ?2, status = 'failed', summary_json = ?3 WHERE id = ?1",
            params![id, time::now_rfc3339(), summary_json],
        )?;
        if updated == 0 {
            return Err(AppError::not_found("The import batch was not found."));
        }
        self.get(conn, id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::test_conn;

    #[test]
    fn fail_marks_batch_failed_with_summary() {
        let conn = test_conn();
        let batch = SqliteImportBatchRepo
            .create(&conn, "w1", "C:/legacy")
            .expect("create batch");
        assert_eq!(batch.status, "running");

        let failed = SqliteImportBatchRepo
            .fail(&conn, &batch.id, "{}")
            .expect("fail batch");
        assert_eq!(failed.status, "failed");
        assert!(failed.completed_at.is_some());
        assert_eq!(failed.summary_json.as_deref(), Some("{}"));
    }

    #[test]
    fn fail_on_missing_batch_is_not_found() {
        let conn = test_conn();
        let result = SqliteImportBatchRepo.fail(&conn, "missing", "{}");
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().code, crate::error::ErrorCode::NotFound);
    }
}
