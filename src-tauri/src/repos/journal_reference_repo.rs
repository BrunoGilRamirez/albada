use rusqlite::{params, Connection, OptionalExtension};

use crate::domain::models::JournalTaskReference;
use crate::domain::types::{JournalReferenceEntityType, JournalReferenceRelation};
use crate::error::AppError;
use crate::id;
use crate::time;

use super::year_theme_repo::map_constraint_violation;

pub trait JournalTaskReferenceRepo {
    fn create(
        &self,
        conn: &Connection,
        journal_entry_id: &str,
        entity_type: JournalReferenceEntityType,
        entity_id: &str,
        relation: JournalReferenceRelation,
    ) -> Result<JournalTaskReference, AppError>;
    fn list_for_entry(
        &self,
        conn: &Connection,
        journal_entry_id: &str,
    ) -> Result<Vec<JournalTaskReference>, AppError>;
    fn remove(&self, conn: &Connection, id: &str) -> Result<(), AppError>;
}

const REFERENCE_COLS: &str = "id, journal_entry_id, entity_type, entity_id, relation, created_by, confidence, created_at, removed_at";

fn row_to_reference(row: &rusqlite::Row<'_>) -> Result<JournalTaskReference, rusqlite::Error> {
    let entity_type: String = row.get(2)?;
    let relation: String = row.get(4)?;
    Ok(JournalTaskReference {
        id: row.get(0)?,
        journal_entry_id: row.get(1)?,
        entity_type: entity_type.parse().map_err(|_| {
            rusqlite::Error::InvalidColumnType(
                2,
                "entity_type".to_string(),
                rusqlite::types::Type::Text,
            )
        })?,
        entity_id: row.get(3)?,
        relation: relation.parse().map_err(|_| {
            rusqlite::Error::InvalidColumnType(
                4,
                "relation".to_string(),
                rusqlite::types::Type::Text,
            )
        })?,
        created_by: row.get(5)?,
        confidence: row.get(6)?,
        created_at: row.get(7)?,
        removed_at: row.get(8)?,
    })
}

pub struct SqliteJournalTaskReferenceRepo;

impl JournalTaskReferenceRepo for SqliteJournalTaskReferenceRepo {
    fn create(
        &self,
        conn: &Connection,
        journal_entry_id: &str,
        entity_type: JournalReferenceEntityType,
        entity_id: &str,
        relation: JournalReferenceRelation,
    ) -> Result<JournalTaskReference, AppError> {
        let entry_exists: i64 = conn.query_row(
            "SELECT EXISTS (SELECT 1 FROM journal_entries WHERE id = ?1)",
            [journal_entry_id],
            |row| row.get(0),
        )?;
        if entry_exists == 0 {
            return Err(AppError::not_found("The journal entry was not found."));
        }

        let id = id::new_uuid();
        let now = time::now_rfc3339();
        conn.execute(
            "INSERT INTO journal_task_references (id, journal_entry_id, entity_type, entity_id, relation, created_by, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, 'user', ?6)",
            params![
                id,
                journal_entry_id,
                entity_type.to_string(),
                entity_id,
                relation.to_string(),
                now
            ],
        )
        .map_err(map_constraint_violation)?;

        conn.query_row(
            &format!("SELECT {REFERENCE_COLS} FROM journal_task_references WHERE id = ?1"),
            [id],
            row_to_reference,
        )
        .optional()?
        .ok_or_else(|| AppError::not_found("The journal reference was not found."))
    }

    fn list_for_entry(
        &self,
        conn: &Connection,
        journal_entry_id: &str,
    ) -> Result<Vec<JournalTaskReference>, AppError> {
        let mut stmt = conn.prepare(&format!(
            "SELECT {REFERENCE_COLS} FROM journal_task_references WHERE journal_entry_id = ?1 AND removed_at IS NULL ORDER BY created_at"
        ))?;
        let rows = stmt.query_map([journal_entry_id], row_to_reference)?;
        let mut result = Vec::new();
        for row in rows {
            result.push(row?);
        }
        Ok(result)
    }

    fn remove(&self, conn: &Connection, id: &str) -> Result<(), AppError> {
        let updated = conn.execute(
            "UPDATE journal_task_references SET removed_at = ?2 WHERE id = ?1 AND removed_at IS NULL",
            params![id, time::now_rfc3339()],
        )?;
        if updated == 0 {
            return Err(AppError::not_found("The journal reference was not found."));
        }
        Ok(())
    }
}
