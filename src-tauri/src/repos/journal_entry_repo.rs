use rusqlite::{params, Connection, OptionalExtension};
use serde_json::Value;
use std::collections::BTreeMap;

use crate::domain::models::JournalEntry;
use crate::domain::types::JournalSource;
use crate::domain::validation::{
    validate_journal_content_markdown, validate_journal_emotions, validate_journal_template_pair,
    validate_journal_template_values, validate_title,
};
use crate::error::AppError;
use crate::id;
use crate::time;

use super::year_theme_repo::map_constraint_violation;

pub trait JournalEntryRepo {
    #[allow(clippy::too_many_arguments)]
    fn create(
        &self,
        conn: &Connection,
        workspace_id: &str,
        occurred_on: &str,
        title: &str,
        content_markdown: &str,
        template_id: &Option<String>,
        template_version: &Option<String>,
        template_values: &BTreeMap<String, Value>,
        mood_label: &Option<String>,
        energy_label: &Option<String>,
        emotions: &[String],
        export_eligible: bool,
        source: JournalSource,
    ) -> Result<JournalEntry, AppError>;
    fn get(&self, conn: &Connection, id: &str) -> Result<JournalEntry, AppError>;
    fn list(&self, conn: &Connection, workspace_id: &str) -> Result<Vec<JournalEntry>, AppError>;
    /// List non-archived journal entry IDs in chronological order (occurred_on ASC, title ASC, id ASC),
    /// with pagination for low-RAM chunked resolution.
    fn list_ids_paged(
        &self,
        conn: &Connection,
        workspace_id: &str,
        limit: usize,
        offset: usize,
    ) -> Result<Vec<String>, AppError>;
    /// List a page of non-archived journal entries in chronological order (occurred_on ASC, title ASC, id ASC).
    fn list_page(
        &self,
        conn: &Connection,
        workspace_id: &str,
        limit: usize,
        offset: usize,
    ) -> Result<Vec<JournalEntry>, AppError>;
    /// List a page of non-archived journal entries within a date range (occurred_on BETWEEN from AND to),
    /// in chronological order (occurred_on ASC, title ASC, id ASC).
    fn list_range_page(
        &self,
        conn: &Connection,
        workspace_id: &str,
        from: &str,
        to: &str,
        limit: usize,
        offset: usize,
    ) -> Result<Vec<JournalEntry>, AppError>;
    /// Count non-archived journal entries for a workspace.
    fn count(&self, conn: &Connection, workspace_id: &str) -> Result<i64, AppError>;
    #[allow(clippy::too_many_arguments)]
    fn update(
        &self,
        conn: &Connection,
        id: &str,
        occurred_on: &str,
        title: &str,
        content_markdown: &str,
        template_id: &Option<String>,
        template_version: &Option<String>,
        template_values: Option<&BTreeMap<String, Value>>,
        mood_label: &Option<String>,
        energy_label: &Option<String>,
        emotions: Option<&[String]>,
        export_eligible: Option<bool>,
    ) -> Result<JournalEntry, AppError>;
    fn archive(&self, conn: &Connection, id: &str) -> Result<JournalEntry, AppError>;
    fn archive_many(
        &self,
        conn: &Connection,
        workspace_id: &str,
        entry_ids: &[String],
    ) -> Result<usize, AppError>;
    fn set_export_eligible_many(
        &self,
        conn: &Connection,
        workspace_id: &str,
        entry_ids: &[String],
        export_eligible: bool,
    ) -> Result<usize, AppError>;
}

const ENTRY_COLS: &str = "id, workspace_id, occurred_on, title, content_markdown, template_id, template_version, template_values_json, mood_label, energy_label, emotions_json, export_eligible, source, created_at, updated_at, archived_at";

fn row_to_entry(row: &rusqlite::Row<'_>) -> Result<JournalEntry, rusqlite::Error> {
    let physical_template_id: Option<String> = row.get(5)?;
    let physical_template_version: Option<String> = row.get(6)?;
    let (template_id, template_version) = match (physical_template_id, physical_template_version) {
        (Some(id), Some(version)) => (Some(id), Some(version)),
        _ => (None, None),
    };
    let template_values_json: String = row.get(7)?;
    let emotions_json: String = row.get(10)?;
    let source: String = row.get(12)?;
    let template_values = serde_json::from_str(&template_values_json).map_err(|error| {
        rusqlite::Error::FromSqlConversionFailure(7, rusqlite::types::Type::Text, Box::new(error))
    })?;
    let emotions = serde_json::from_str(&emotions_json).map_err(|error| {
        rusqlite::Error::FromSqlConversionFailure(10, rusqlite::types::Type::Text, Box::new(error))
    })?;
    Ok(JournalEntry {
        id: row.get(0)?,
        workspace_id: row.get(1)?,
        occurred_on: row.get(2)?,
        title: row.get(3)?,
        content_markdown: row.get(4)?,
        // An old physical ID without an exact recorded version is provenance,
        // not a complete product/API template identity.
        template_id,
        template_version,
        template_values,
        mood_label: row.get(8)?,
        energy_label: row.get(9)?,
        emotions,
        export_eligible: row.get::<_, i64>(11)? != 0,
        source: source.parse().map_err(|_| {
            rusqlite::Error::InvalidColumnType(
                12,
                "source".to_string(),
                rusqlite::types::Type::Text,
            )
        })?,
        created_at: row.get(13)?,
        updated_at: row.get(14)?,
        archived_at: row.get(15)?,
    })
}

pub struct SqliteJournalEntryRepo;

fn validate_batch_entry_ids(
    conn: &Connection,
    workspace_id: &str,
    entry_ids: &[String],
) -> Result<(), AppError> {
    if entry_ids.is_empty() {
        return Err(AppError::validation("Select at least one journal entry."));
    }

    let mut seen = std::collections::HashSet::with_capacity(entry_ids.len());
    for entry_id in entry_ids {
        if !seen.insert(entry_id) {
            return Err(AppError::validation(
                "A journal entry may only be selected once.",
            ));
        }
        let exists: Option<i64> = conn
            .query_row(
                "SELECT 1 FROM journal_entries
                 WHERE id = ?1 AND workspace_id = ?2 AND archived_at IS NULL",
                params![entry_id, workspace_id],
                |row| row.get(0),
            )
            .optional()?;
        if exists.is_none() {
            return Err(AppError::not_found(
                "One or more selected journal entries are unavailable.",
            ));
        }
    }
    Ok(())
}

impl JournalEntryRepo for SqliteJournalEntryRepo {
    #[allow(clippy::too_many_arguments)]
    fn create(
        &self,
        conn: &Connection,
        workspace_id: &str,
        occurred_on: &str,
        title: &str,
        content_markdown: &str,
        template_id: &Option<String>,
        template_version: &Option<String>,
        template_values: &BTreeMap<String, Value>,
        mood_label: &Option<String>,
        energy_label: &Option<String>,
        emotions: &[String],
        export_eligible: bool,
        source: JournalSource,
    ) -> Result<JournalEntry, AppError> {
        validate_title(title)?;
        time::parse_date(occurred_on)?;
        // Imports preserve their source verbatim; user/agent writes may not
        // create system metadata comments.
        validate_journal_content_markdown(content_markdown, source == JournalSource::Import)?;
        validate_journal_template_pair(template_id, template_version)?;
        validate_journal_emotions(emotions)?;
        validate_journal_template_values(
            template_values,
            template_id.as_deref(),
            template_version.as_deref(),
        )?;
        let id = id::new_uuid();
        let now = time::now_rfc3339();
        let template_values_json = serde_json::to_string(template_values).map_err(|_| {
            AppError::validation("Journal template values must be valid JSON data.")
        })?;
        let emotions_json = serde_json::to_string(emotions)
            .map_err(|_| AppError::validation("Journal emotions could not be stored."))?;
        conn.execute(
            "INSERT INTO journal_entries
             (id, workspace_id, occurred_on, title, content_markdown, template_id, template_version,
              template_values_json, mood_label, energy_label, emotions_json, export_eligible, source,
              created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?14)",
            params![
                id,
                workspace_id,
                occurred_on,
                title.trim(),
                content_markdown,
                template_id,
                template_version,
                template_values_json,
                mood_label,
                energy_label,
                emotions_json,
                i64::from(export_eligible),
                source.to_string(),
                now
            ],
        )
        .map_err(map_constraint_violation)?;
        self.get(conn, &id)
    }

    fn get(&self, conn: &Connection, id: &str) -> Result<JournalEntry, AppError> {
        conn.query_row(
            &format!("SELECT {ENTRY_COLS} FROM journal_entries WHERE id = ?1"),
            [id],
            row_to_entry,
        )
        .optional()?
        .ok_or_else(|| AppError::not_found("The journal entry was not found."))
    }

    fn list(&self, conn: &Connection, workspace_id: &str) -> Result<Vec<JournalEntry>, AppError> {
        let mut stmt = conn.prepare(&format!(
            "SELECT {ENTRY_COLS} FROM journal_entries WHERE workspace_id = ?1 AND archived_at IS NULL ORDER BY occurred_on DESC, created_at DESC"
        ))?;
        let rows = stmt.query_map([workspace_id], row_to_entry)?;
        let mut result = Vec::new();
        for row in rows {
            result.push(row?);
        }
        Ok(result)
    }

    fn list_ids_paged(
        &self,
        conn: &Connection,
        workspace_id: &str,
        limit: usize,
        offset: usize,
    ) -> Result<Vec<String>, AppError> {
        let mut stmt = conn.prepare(
            "SELECT id FROM journal_entries WHERE workspace_id = ?1 AND archived_at IS NULL ORDER BY occurred_on ASC, title ASC, id ASC LIMIT ?2 OFFSET ?3",
        )?;
        let rows = stmt.query_map(params![workspace_id, limit as i64, offset as i64], |row| {
            row.get(0)
        })?;
        let mut result = Vec::new();
        for row in rows {
            result.push(row?);
        }
        Ok(result)
    }

    fn list_page(
        &self,
        conn: &Connection,
        workspace_id: &str,
        limit: usize,
        offset: usize,
    ) -> Result<Vec<JournalEntry>, AppError> {
        let mut stmt = conn.prepare(&format!(
            "SELECT {ENTRY_COLS} FROM journal_entries WHERE workspace_id = ?1 AND archived_at IS NULL ORDER BY occurred_on ASC, title ASC, id ASC LIMIT ?2 OFFSET ?3"
        ))?;
        let rows = stmt.query_map(
            params![workspace_id, limit as i64, offset as i64],
            row_to_entry,
        )?;
        let mut result = Vec::new();
        for row in rows {
            result.push(row?);
        }
        Ok(result)
    }

    fn list_range_page(
        &self,
        conn: &Connection,
        workspace_id: &str,
        from: &str,
        to: &str,
        limit: usize,
        offset: usize,
    ) -> Result<Vec<JournalEntry>, AppError> {
        let mut stmt = conn.prepare(&format!(
            "SELECT {ENTRY_COLS} FROM journal_entries WHERE workspace_id = ?1 AND archived_at IS NULL AND occurred_on BETWEEN ?2 AND ?3 ORDER BY occurred_on ASC, title ASC, id ASC LIMIT ?4 OFFSET ?5"
        ))?;
        let rows = stmt.query_map(
            params![workspace_id, from, to, limit as i64, offset as i64],
            row_to_entry,
        )?;
        let mut result = Vec::new();
        for row in rows {
            result.push(row?);
        }
        Ok(result)
    }

    fn count(&self, conn: &Connection, workspace_id: &str) -> Result<i64, AppError> {
        let count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM journal_entries WHERE workspace_id = ?1 AND archived_at IS NULL",
            [workspace_id],
            |row| row.get(0),
        )?;
        Ok(count)
    }

    #[allow(clippy::too_many_arguments)]
    fn update(
        &self,
        conn: &Connection,
        id: &str,
        occurred_on: &str,
        title: &str,
        content_markdown: &str,
        template_id: &Option<String>,
        template_version: &Option<String>,
        template_values: Option<&BTreeMap<String, Value>>,
        mood_label: &Option<String>,
        energy_label: &Option<String>,
        emotions: Option<&[String]>,
        export_eligible: Option<bool>,
    ) -> Result<JournalEntry, AppError> {
        validate_title(title)?;
        time::parse_date(occurred_on)?;
        if let Some(emotions) = emotions {
            validate_journal_emotions(emotions)?;
        }
        let template_values_json = template_values
            .map(serde_json::to_string)
            .transpose()
            .map_err(|_| {
                AppError::validation("Journal template values must be valid JSON data.")
            })?;
        let emotions_json = emotions
            .map(serde_json::to_string)
            .transpose()
            .map_err(|_| AppError::validation("Journal emotions could not be stored."))?;
        let tx = conn.unchecked_transaction()?;
        let existing: Option<(Option<String>, Option<String>, String)> = tx
            .query_row(
                "SELECT template_id, template_version, content_markdown
                 FROM journal_entries WHERE id = ?1",
                [id],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .optional()?;
        let (existing_template_id, existing_template_version, existing_content) =
            existing.ok_or_else(|| AppError::not_found("The journal entry was not found."))?;

        let (updated_template_id, updated_template_version) =
            match (template_id.as_ref(), template_version.as_ref()) {
                (None, None) => (None, None),
                (Some(id), Some(version)) => {
                    validate_journal_template_pair(template_id, template_version)?;
                    (Some(id), Some(version))
                }
                // Old clients may echo the current ID without a version. Treat an
                // exact match as a no-op, including for a masked legacy ID-only row.
                (Some(id), None) if existing_template_id.as_ref() == Some(id) => (None, None),
                _ => {
                    return Err(AppError::validation(
                        "Journal template ID and version must be supplied together.",
                    ));
                }
            };
        if let Some(template_values) = template_values {
            validate_journal_template_values(
                template_values,
                updated_template_id
                    .or(existing_template_id.as_ref())
                    .map(String::as_str),
                updated_template_version
                    .or(existing_template_version.as_ref())
                    .map(String::as_str),
            )?;
        }
        validate_journal_content_markdown(
            content_markdown,
            content_markdown == existing_content.as_str(),
        )?;
        let updated = tx.execute(
            "UPDATE journal_entries
             SET occurred_on = ?2, title = ?3, content_markdown = ?4,
                 template_id = COALESCE(?5, template_id),
                 template_version = COALESCE(?6, template_version),
                 template_values_json = COALESCE(?7, template_values_json),
                 mood_label = COALESCE(?8, mood_label),
                 energy_label = COALESCE(?9, energy_label),
                 emotions_json = COALESCE(?10, emotions_json),
                 export_eligible = COALESCE(?11, export_eligible),
                 updated_at = ?12
             WHERE id = ?1",
            params![
                id,
                occurred_on,
                title.trim(),
                content_markdown,
                updated_template_id,
                updated_template_version,
                template_values_json,
                mood_label,
                energy_label,
                emotions_json,
                export_eligible.map(i64::from),
                time::now_rfc3339()
            ],
        )?;
        if updated == 0 {
            return Err(AppError::not_found("The journal entry was not found."));
        }
        tx.commit()?;
        self.get(conn, id)
    }

    fn archive(&self, conn: &Connection, id: &str) -> Result<JournalEntry, AppError> {
        let updated = conn.execute(
            "UPDATE journal_entries SET archived_at = ?2, updated_at = ?2 WHERE id = ?1",
            params![id, time::now_rfc3339()],
        )?;
        if updated == 0 {
            return Err(AppError::not_found("The journal entry was not found."));
        }
        self.get(conn, id)
    }

    fn archive_many(
        &self,
        conn: &Connection,
        workspace_id: &str,
        entry_ids: &[String],
    ) -> Result<usize, AppError> {
        let tx = conn.unchecked_transaction()?;
        validate_batch_entry_ids(&tx, workspace_id, entry_ids)?;
        let now = time::now_rfc3339();
        for entry_id in entry_ids {
            let updated = tx.execute(
                "UPDATE journal_entries SET archived_at = ?2, updated_at = ?2
                 WHERE id = ?1 AND workspace_id = ?3 AND archived_at IS NULL",
                params![entry_id, now, workspace_id],
            )?;
            if updated != 1 {
                return Err(AppError::not_found(
                    "One or more selected journal entries are unavailable.",
                ));
            }
        }
        tx.commit()?;
        Ok(entry_ids.len())
    }

    fn set_export_eligible_many(
        &self,
        conn: &Connection,
        workspace_id: &str,
        entry_ids: &[String],
        export_eligible: bool,
    ) -> Result<usize, AppError> {
        let tx = conn.unchecked_transaction()?;
        validate_batch_entry_ids(&tx, workspace_id, entry_ids)?;
        let now = time::now_rfc3339();
        for entry_id in entry_ids {
            let updated = tx.execute(
                "UPDATE journal_entries SET export_eligible = ?2, updated_at = ?3
                 WHERE id = ?1 AND workspace_id = ?4 AND archived_at IS NULL",
                params![entry_id, i64::from(export_eligible), now, workspace_id],
            )?;
            if updated != 1 {
                return Err(AppError::not_found(
                    "One or more selected journal entries are unavailable.",
                ));
            }
        }
        tx.commit()?;
        Ok(entry_ids.len())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::types::{
        JournalReferenceEntityType, JournalReferenceRelation, JournalSource, Priority, TaskStatus,
    };
    use crate::repos::journal_reference_repo::{
        JournalTaskReferenceRepo, SqliteJournalTaskReferenceRepo,
    };
    use crate::repos::task_repo::{complete_task, SqliteTaskRepo, TaskRepo};
    use crate::test_utils::test_conn;

    fn create_entry(conn: &Connection) -> JournalEntry {
        let values = BTreeMap::new();
        SqliteJournalEntryRepo
            .create(
                conn,
                "w1",
                "2026-09-04",
                "A quiet day",
                "Some reflection.",
                &None,
                &None,
                &values,
                &None,
                &None,
                &[],
                true,
                JournalSource::User,
            )
            .expect("create entry")
    }

    fn entry_by_id(conn: &Connection, id: &str) -> JournalEntry {
        SqliteJournalEntryRepo
            .get(conn, id)
            .expect("load synthetic entry")
    }

    fn create_task(conn: &Connection, title: &str) -> String {
        SqliteTaskRepo
            .create(
                conn,
                "w1",
                &None,
                &None,
                &None,
                title,
                &None,
                &None,
                Priority::Normal,
                &None,
                &None,
                &None,
            )
            .expect("create task")
            .id
    }

    #[test]
    fn entry_can_be_created_without_daily_plan() {
        let conn = test_conn();
        let entry = create_entry(&conn);
        assert_eq!(entry.occurred_on, "2026-09-04");
        assert!(entry.export_eligible);
        assert!(entry.emotions.is_empty());
        assert_eq!(entry.source, JournalSource::User);
        assert!(entry.archived_at.is_none());
    }

    #[test]
    fn batch_archive_archives_selected_entries_without_changing_export_eligibility() {
        let conn = test_conn();
        let eligible = create_entry(&conn);
        let private = create_entry(&conn);
        conn.execute(
            "UPDATE journal_entries SET export_eligible = 0 WHERE id = ?1",
            [&private.id],
        )
        .expect("seed private entry");

        let archived = SqliteJournalEntryRepo
            .archive_many(&conn, "w1", &[eligible.id.clone(), private.id.clone()])
            .expect("archive selected entries");

        assert_eq!(archived, 2);
        let eligible = entry_by_id(&conn, &eligible.id);
        let private = entry_by_id(&conn, &private.id);
        assert!(eligible.archived_at.is_some());
        assert!(eligible.export_eligible);
        assert!(private.archived_at.is_some());
        assert!(!private.export_eligible);
    }

    #[test]
    fn batch_export_eligibility_sets_both_values() {
        let conn = test_conn();
        let make_exportable = create_entry(&conn);
        let make_private = create_entry(&conn);
        conn.execute(
            "UPDATE journal_entries SET export_eligible = 0 WHERE id = ?1",
            [&make_exportable.id],
        )
        .expect("seed private entry");

        assert_eq!(
            SqliteJournalEntryRepo
                .set_export_eligible_many(
                    &conn,
                    "w1",
                    std::slice::from_ref(&make_exportable.id),
                    true,
                )
                .expect("make entry exportable"),
            1
        );
        assert_eq!(
            SqliteJournalEntryRepo
                .set_export_eligible_many(
                    &conn,
                    "w1",
                    std::slice::from_ref(&make_private.id),
                    false,
                )
                .expect("make entry private"),
            1
        );
        assert!(entry_by_id(&conn, &make_exportable.id).export_eligible);
        assert!(!entry_by_id(&conn, &make_private.id).export_eligible);
    }

    #[test]
    fn batch_operations_reject_missing_foreign_and_archived_ids_before_mutation() {
        let conn = test_conn();
        let live = create_entry(&conn);
        let foreign = create_entry(&conn);
        let archived = create_entry(&conn);
        let now = crate::time::now_rfc3339();
        conn.execute(
            "INSERT INTO workspaces (id, name, created_at, updated_at) VALUES ('w2', 'Other', ?1, ?1)",
            [&now],
        )
        .expect("seed second synthetic workspace");
        conn.execute(
            "UPDATE journal_entries SET workspace_id = 'w2' WHERE id = ?1",
            [&foreign.id],
        )
        .expect("move synthetic entry to second workspace");
        SqliteJournalEntryRepo
            .archive(&conn, &archived.id)
            .expect("archive stale synthetic entry");

        let missing = "missing-entry".to_owned();
        for invalid_id in [&foreign.id, &archived.id, &missing] {
            assert!(SqliteJournalEntryRepo
                .archive_many(&conn, "w1", &[live.id.clone(), invalid_id.clone()])
                .is_err());
            assert!(SqliteJournalEntryRepo
                .set_export_eligible_many(
                    &conn,
                    "w1",
                    &[live.id.clone(), invalid_id.clone()],
                    false,
                )
                .is_err());
            let unchanged = entry_by_id(&conn, &live.id);
            assert!(unchanged.archived_at.is_none());
            assert!(unchanged.export_eligible);
        }

        assert!(SqliteJournalEntryRepo
            .archive_many(&conn, "w1", &[])
            .is_err());
        assert!(SqliteJournalEntryRepo
            .set_export_eligible_many(&conn, "w1", &[], false)
            .is_err());
    }

    #[test]
    fn batch_archive_trigger_failure_rolls_back_all_changes() {
        let conn = test_conn();
        let first = create_entry(&conn);
        let second = create_entry(&conn);
        conn.execute(
            "UPDATE journal_entries SET title = 'Rollback second' WHERE id = ?1",
            [&second.id],
        )
        .expect("identify second synthetic entry");
        conn.execute_batch(
            "CREATE TRIGGER reject_second_journal_archive
             BEFORE UPDATE OF archived_at ON journal_entries
             WHEN OLD.title = 'Rollback second'
             BEGIN SELECT RAISE(ABORT, 'synthetic trigger failure'); END;",
        )
        .expect("create rollback trigger");

        assert!(SqliteJournalEntryRepo
            .archive_many(&conn, "w1", &[first.id.clone(), second.id.clone()])
            .is_err());
        assert!(entry_by_id(&conn, &first.id).archived_at.is_none());
        assert!(entry_by_id(&conn, &second.id).archived_at.is_none());
    }

    #[test]
    fn batch_eligibility_trigger_failure_rolls_back_all_changes() {
        let conn = test_conn();
        let first = create_entry(&conn);
        let second = create_entry(&conn);
        conn.execute(
            "UPDATE journal_entries SET export_eligible = 0 WHERE id = ?1",
            [&second.id],
        )
        .expect("seed second entry as private");
        conn.execute_batch(
            "CREATE TRIGGER reject_second_journal_eligibility
             BEFORE UPDATE OF export_eligible ON journal_entries
             WHEN OLD.id = (SELECT id FROM journal_entries WHERE export_eligible = 0 LIMIT 1)
             BEGIN SELECT RAISE(ABORT, 'synthetic trigger failure'); END;",
        )
        .expect("create rollback trigger");

        assert!(SqliteJournalEntryRepo
            .set_export_eligible_many(&conn, "w1", &[first.id.clone(), second.id.clone()], true,)
            .is_err());
        assert!(entry_by_id(&conn, &first.id).export_eligible);
        assert!(!entry_by_id(&conn, &second.id).export_eligible);
    }

    #[test]
    fn structured_metadata_round_trips_with_order_and_stable_object_storage() {
        let conn = test_conn();
        let values = BTreeMap::new();
        let original_markdown = "Authored bytes\r\n\nAn authored paragraph.";
        let entry = SqliteJournalEntryRepo
            .create(
                &conn,
                "w1",
                "2026-09-04",
                "Structured",
                original_markdown,
                &Some("foundation-journal".to_owned()),
                &Some("1.0.1".to_owned()),
                &values,
                &Some("good".to_owned()),
                &Some("7".to_owned()),
                &["joy".to_owned(), "calm".to_owned()],
                true,
                JournalSource::User,
            )
            .expect("create structured entry");
        let loaded = SqliteJournalEntryRepo
            .get(&conn, &entry.id)
            .expect("reload entry");
        assert_eq!(loaded.template_id.as_deref(), Some("foundation-journal"));
        assert_eq!(loaded.template_version.as_deref(), Some("1.0.1"));
        assert_eq!(loaded.template_values, values);
        assert_eq!(loaded.emotions, vec!["joy".to_owned(), "calm".to_owned()]);
        assert_eq!(
            loaded.content_markdown.as_bytes(),
            original_markdown.as_bytes()
        );
        let serialized = serde_json::to_value(&loaded).expect("serialize journal entry");
        assert_eq!(serialized["template_values"], serde_json::json!({}));
        assert_eq!(serialized["export_eligible"], true);
        assert!(serialized.get("privacy_scope").is_none());
        assert!(serialized.get("retroactive").is_none());
        let stored_json: String = conn
            .query_row(
                "SELECT template_values_json FROM journal_entries WHERE id = ?1",
                [&entry.id],
                |row| row.get(0),
            )
            .expect("read stored object");
        assert_eq!(stored_json, "{}");
    }

    #[test]
    fn template_pair_is_validated_and_legacy_id_only_is_masked() {
        let conn = test_conn();
        let values = BTreeMap::new();
        let invalid = SqliteJournalEntryRepo.create(
            &conn,
            "w1",
            "2026-09-04",
            "Invalid pair",
            "Body",
            &Some("foundation-journal".to_owned()),
            &None,
            &values,
            &None,
            &None,
            &[],
            true,
            JournalSource::User,
        );
        assert!(invalid.is_err());

        let entry = create_entry(&conn);
        let partial_update = SqliteJournalEntryRepo.update(
            &conn,
            &entry.id,
            "2026-09-04",
            "Invalid update pair",
            "Body",
            &Some("foundation-journal".to_owned()),
            &None,
            None,
            &None,
            &None,
            None,
            None,
        );
        assert!(partial_update.is_err());
        conn.execute(
            "UPDATE journal_entries SET template_id = 'legacy-template' WHERE id = ?1",
            [&entry.id],
        )
        .expect("seed old physical ID-only row");
        SqliteJournalEntryRepo
            .update(
                &conn,
                &entry.id,
                "2026-09-04",
                "Legacy ID echoed by old client",
                "Some reflection.",
                &Some("legacy-template".to_owned()),
                &None,
                None,
                &None,
                &None,
                None,
                None,
            )
            .expect("preserve legacy ID-only pair without inferring a version");
        let legacy = SqliteJournalEntryRepo
            .get(&conn, &entry.id)
            .expect("read legacy row");
        assert!(legacy.template_id.is_none());
        assert!(legacy.template_version.is_none());
    }

    #[test]
    fn new_writes_do_not_store_generated_joi_markers() {
        let conn = test_conn();
        let values = BTreeMap::new();
        let result = SqliteJournalEntryRepo.create(
            &conn,
            "w1",
            "2026-09-04",
            "Marker input",
            "<!-- joi:field id=\"entry.mood\" -->good<!-- joi:end-field -->",
            &None,
            &None,
            &values,
            &None,
            &None,
            &[],
            true,
            JournalSource::User,
        );
        assert!(result.is_err());
    }

    #[test]
    fn imported_marker_source_is_preserved_and_only_unchanged_legacy_body_can_be_updated() {
        let conn = test_conn();
        let values = BTreeMap::new();
        let source = "Authored\n<!-- joi:field id=\"entry.mood\" -->good<!-- joi:end-field -->";
        let entry = SqliteJournalEntryRepo
            .create(
                &conn,
                "w1",
                "2026-09-04",
                "Imported",
                source,
                &None,
                &None,
                &values,
                &None,
                &None,
                &[],
                false,
                JournalSource::Import,
            )
            .expect("preserve imported source");
        assert_eq!(entry.content_markdown.as_bytes(), source.as_bytes());

        SqliteJournalEntryRepo
            .update(
                &conn,
                &entry.id,
                "2026-09-04",
                "Renamed imported entry",
                source,
                &None,
                &None,
                None,
                &None,
                &None,
                None,
                None,
            )
            .expect("unchanged legacy body remains editable");
        let changed = source.replace("good", "low");
        assert!(SqliteJournalEntryRepo
            .update(
                &conn,
                &entry.id,
                "2026-09-04",
                "Changed marker",
                &changed,
                &None,
                &None,
                None,
                &None,
                &None,
                None,
                None,
            )
            .is_err());
        let stored = SqliteJournalEntryRepo
            .get(&conn, &entry.id)
            .expect("legacy source remains unchanged");
        assert_eq!(stored.content_markdown.as_bytes(), source.as_bytes());
    }

    #[test]
    fn template_values_reject_unapproved_and_dedicated_field_ids() {
        let conn = test_conn();
        for field_id in [
            "entry.free",
            "entry.mood",
            "entry.energy",
            "entry.emotions",
            "unbundled.field",
        ] {
            let mut values = BTreeMap::new();
            values.insert(field_id.to_owned(), serde_json::json!("value"));
            let result = SqliteJournalEntryRepo.create(
                &conn,
                "w1",
                "2026-09-04",
                "Unapproved value",
                "Body",
                &Some("foundation-journal".to_owned()),
                &Some("1.0.1".to_owned()),
                &values,
                &None,
                &None,
                &[],
                true,
                JournalSource::User,
            );
            assert!(result.is_err(), "field ID {field_id} must be rejected");
        }
    }

    #[test]
    fn update_preserves_structured_values_not_supplied_by_older_clients() {
        let conn = test_conn();
        let values = BTreeMap::new();
        let entry = SqliteJournalEntryRepo
            .create(
                &conn,
                "w1",
                "2026-09-04",
                "Keep metadata",
                "Body",
                &Some("foundation-journal".to_owned()),
                &Some("1.0.1".to_owned()),
                &values,
                &Some("good".to_owned()),
                &None,
                &["calm".to_owned()],
                true,
                JournalSource::User,
            )
            .expect("create entry");
        conn.execute(
            "UPDATE journal_entries
             SET template_values_json = '{\"future.note\":\"Response\"}' WHERE id = ?1",
            [&entry.id],
        )
        .expect("seed a synthetic future template value");
        SqliteJournalEntryRepo
            .update(
                &conn,
                &entry.id,
                "2026-09-05",
                "Edited title",
                "Edited body",
                &None,
                &None,
                None,
                &None,
                &None,
                None,
                None,
            )
            .expect("update legacy client fields");
        let updated = SqliteJournalEntryRepo
            .get(&conn, &entry.id)
            .expect("reload entry");
        assert_eq!(
            updated.template_values.get("future.note"),
            Some(&serde_json::json!("Response"))
        );
        assert_eq!(updated.template_version.as_deref(), Some("1.0.1"));
        assert_eq!(updated.emotions, vec!["calm".to_owned()]);
        assert_eq!(updated.mood_label.as_deref(), Some("good"));
    }

    #[test]
    fn entry_can_reference_task_without_changing_status() {
        let conn = test_conn();
        let entry = create_entry(&conn);
        let task_id = create_task(&conn, "Write a letter");

        let reference = SqliteJournalTaskReferenceRepo
            .create(
                &conn,
                &entry.id,
                JournalReferenceEntityType::Task,
                &task_id,
                JournalReferenceRelation::ReflectsOn,
            )
            .expect("create reference");

        assert_eq!(reference.entity_id, task_id);
        assert_eq!(reference.relation, JournalReferenceRelation::ReflectsOn);

        let task = SqliteTaskRepo.get(&conn, &task_id).expect("get task");
        assert_eq!(task.status, TaskStatus::Inbox);
    }

    #[test]
    fn deleting_task_does_not_delete_entry() {
        let conn = test_conn();
        let entry = create_entry(&conn);
        let task_id = create_task(&conn, "Temporary task");

        SqliteJournalTaskReferenceRepo
            .create(
                &conn,
                &entry.id,
                JournalReferenceEntityType::Task,
                &task_id,
                JournalReferenceRelation::Mentions,
            )
            .expect("create reference");

        SqliteTaskRepo
            .delete_subtree(&conn, &task_id)
            .expect("delete task");

        let after = SqliteJournalEntryRepo
            .get(&conn, &entry.id)
            .expect("get entry");
        assert_eq!(after.id, entry.id);
        assert_eq!(after.title, "A quiet day");
    }

    #[test]
    fn archiving_task_does_not_alter_entry() {
        let conn = test_conn();
        let entry = create_entry(&conn);
        let task_id = create_task(&conn, "Task to archive");

        SqliteJournalTaskReferenceRepo
            .create(
                &conn,
                &entry.id,
                JournalReferenceEntityType::Task,
                &task_id,
                JournalReferenceRelation::RelatedTo,
            )
            .expect("create reference");

        // Complete then archive the task.
        crate::domain::task_closure::transition_status(
            &conn,
            &task_id,
            TaskStatus::InProgress,
            &None,
            crate::domain::types::HistorySource::User,
        )
        .expect("start task");
        complete_task(&conn, &task_id, &None).expect("complete task");
        SqliteTaskRepo
            .archive(&conn, &task_id)
            .expect("archive task");

        let after = SqliteJournalEntryRepo
            .get(&conn, &entry.id)
            .expect("get entry");
        assert_eq!(after.title, "A quiet day");
        assert!(after.archived_at.is_none());
    }

    #[test]
    fn deleting_entry_does_not_alter_task() {
        let conn = test_conn();
        let entry = create_entry(&conn);
        let task_id = create_task(&conn, "Keep me");

        SqliteJournalTaskReferenceRepo
            .create(
                &conn,
                &entry.id,
                JournalReferenceEntityType::Task,
                &task_id,
                JournalReferenceRelation::Mentions,
            )
            .expect("create reference");

        conn.execute("DELETE FROM journal_entries WHERE id = ?1", [&entry.id])
            .expect("delete entry");

        let task = SqliteTaskRepo.get(&conn, &task_id).expect("get task");
        assert_eq!(task.status, TaskStatus::Inbox);
        assert_eq!(task.title, "Keep me");
    }

    #[test]
    fn soft_removing_reference_keeps_entry_and_task() {
        let conn = test_conn();
        let entry = create_entry(&conn);
        let task_id = create_task(&conn, "Referenced task");

        let reference = SqliteJournalTaskReferenceRepo
            .create(
                &conn,
                &entry.id,
                JournalReferenceEntityType::Task,
                &task_id,
                JournalReferenceRelation::Mentions,
            )
            .expect("create reference");

        SqliteJournalTaskReferenceRepo
            .remove(&conn, &reference.id)
            .expect("remove reference");

        let remaining = SqliteJournalTaskReferenceRepo
            .list_for_entry(&conn, &entry.id)
            .expect("list references");
        assert!(remaining.is_empty());

        assert!(SqliteJournalEntryRepo.get(&conn, &entry.id).is_ok());
        assert!(SqliteTaskRepo.get(&conn, &task_id).is_ok());
    }
}
