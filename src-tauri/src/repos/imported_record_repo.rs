use rusqlite::{params, Connection, OptionalExtension};

use crate::error::AppError;
use crate::{id, time};

/// Provenance for one record created or matched by an import batch.
#[derive(Debug, Clone)]
pub struct ImportedRecord {
    pub id: String,
    pub import_batch_id: String,
    pub entity_type: String,
    pub entity_id: String,
    pub source_path: String,
    pub source_date: Option<String>,
    pub confidence: String,
    pub warnings_json: Option<String>,
    pub imported_at: String,
    pub source_key: Option<String>,
    pub content_hash: Option<String>,
}

const COLS: &str = "id, import_batch_id, entity_type, entity_id, source_path, source_date, confidence, warnings_json, imported_at, source_key, content_hash";

fn row_to_record(row: &rusqlite::Row<'_>) -> Result<ImportedRecord, rusqlite::Error> {
    Ok(ImportedRecord {
        id: row.get(0)?,
        import_batch_id: row.get(1)?,
        entity_type: row.get(2)?,
        entity_id: row.get(3)?,
        source_path: row.get(4)?,
        source_date: row.get(5)?,
        confidence: row.get(6)?,
        warnings_json: row.get(7)?,
        imported_at: row.get(8)?,
        source_key: row.get(9)?,
        content_hash: row.get(10)?,
    })
}

pub trait ImportedRecordRepo {
    #[allow(clippy::too_many_arguments)]
    fn insert(
        &self,
        conn: &Connection,
        import_batch_id: &str,
        entity_type: &str,
        entity_id: &str,
        source_path: &str,
        source_date: &Option<String>,
        confidence: &str,
        warnings_json: &Option<String>,
        source_key: &str,
        content_hash: &str,
    ) -> Result<ImportedRecord, AppError>;
    fn find_existing(
        &self,
        conn: &Connection,
        workspace_id: &str,
        source_key: &str,
    ) -> Result<Option<ImportedRecord>, AppError>;
    fn find_by_source_key(
        &self,
        conn: &Connection,
        source_key: &str,
    ) -> Result<Option<ImportedRecord>, AppError>;
    fn update_meta(
        &self,
        conn: &Connection,
        id: &str,
        source_date: &Option<String>,
        confidence: &str,
        warnings_json: &Option<String>,
    ) -> Result<ImportedRecord, AppError>;
    fn list_for_batch(
        &self,
        conn: &Connection,
        import_batch_id: &str,
    ) -> Result<Vec<ImportedRecord>, AppError>;
}

pub struct SqliteImportedRecordRepo;

impl ImportedRecordRepo for SqliteImportedRecordRepo {
    #[allow(clippy::too_many_arguments)]
    fn insert(
        &self,
        conn: &Connection,
        import_batch_id: &str,
        entity_type: &str,
        entity_id: &str,
        source_path: &str,
        source_date: &Option<String>,
        confidence: &str,
        warnings_json: &Option<String>,
        source_key: &str,
        content_hash: &str,
    ) -> Result<ImportedRecord, AppError> {
        let id = id::new_uuid();
        let now = time::now_rfc3339();
        conn.execute(
            "INSERT INTO imported_records (id, import_batch_id, entity_type, entity_id, source_path, source_date, confidence, warnings_json, imported_at, source_key, content_hash)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
            params![
                id,
                import_batch_id,
                entity_type,
                entity_id,
                source_path,
                source_date,
                confidence,
                warnings_json,
                now,
                source_key,
                content_hash
            ],
        )?;
        conn.query_row(
            &format!("SELECT {COLS} FROM imported_records WHERE id = ?1"),
            [id],
            row_to_record,
        )
        .optional()?
        .ok_or_else(|| AppError::not_found("The imported record was not found."))
    }

    fn find_existing(
        &self,
        conn: &Connection,
        workspace_id: &str,
        source_key: &str,
    ) -> Result<Option<ImportedRecord>, AppError> {
        conn.query_row(
            "SELECT r.id, r.import_batch_id, r.entity_type, r.entity_id, r.source_path, r.source_date, r.confidence, r.warnings_json, r.imported_at, r.source_key, r.content_hash
             FROM imported_records r
             JOIN import_batches b ON b.id = r.import_batch_id
             WHERE b.workspace_id = ?1 AND r.source_key = ?2
             ORDER BY r.imported_at DESC
             LIMIT 1",
            params![workspace_id, source_key],
            row_to_record,
        )
        .optional()
        .map_err(Into::into)
    }

    fn find_by_source_key(
        &self,
        conn: &Connection,
        source_key: &str,
    ) -> Result<Option<ImportedRecord>, AppError> {
        conn.query_row(
            &format!("SELECT {COLS} FROM imported_records WHERE source_key = ?1 ORDER BY imported_at DESC LIMIT 1"),
            [source_key],
            row_to_record,
        )
        .optional()
        .map_err(Into::into)
    }

    fn update_meta(
        &self,
        conn: &Connection,
        id: &str,
        source_date: &Option<String>,
        confidence: &str,
        warnings_json: &Option<String>,
    ) -> Result<ImportedRecord, AppError> {
        let updated = conn.execute(
            "UPDATE imported_records SET source_date = ?2, confidence = ?3, warnings_json = ?4 WHERE id = ?1",
            params![id, source_date, confidence, warnings_json],
        )?;
        if updated == 0 {
            return Err(AppError::not_found("The imported record was not found."));
        }
        conn.query_row(
            &format!("SELECT {COLS} FROM imported_records WHERE id = ?1"),
            [id],
            row_to_record,
        )
        .optional()?
        .ok_or_else(|| AppError::not_found("The imported record was not found."))
    }

    fn list_for_batch(
        &self,
        conn: &Connection,
        import_batch_id: &str,
    ) -> Result<Vec<ImportedRecord>, AppError> {
        let mut stmt = conn.prepare(&format!(
            "SELECT {COLS} FROM imported_records WHERE import_batch_id = ?1 ORDER BY source_path"
        ))?;
        let rows = stmt.query_map([import_batch_id], row_to_record)?;
        let mut result = Vec::new();
        for row in rows {
            result.push(row?);
        }
        Ok(result)
    }
}
