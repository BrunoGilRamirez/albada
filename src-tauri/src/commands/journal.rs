use crate::commands::dto::{CreateJournalEntryDto, UpdateJournalEntryDto};
use crate::domain::models::JournalEntry;
use crate::domain::types::JournalSource;
use crate::error::AppError;
use crate::repos::journal_entry_repo::{JournalEntryRepo, SqliteJournalEntryRepo};
use crate::state::Db;
use tauri::State;

#[tauri::command]
pub fn create_journal_entry(
    db: State<'_, Db>,
    dto: CreateJournalEntryDto,
) -> Result<JournalEntry, AppError> {
    log::debug!(
        "create_journal_entry payload: workspace_id={:?} occurred_on={:?} title={:?}",
        dto.workspace_id,
        dto.occurred_on,
        dto.title
    );
    let source: JournalSource = dto.source.as_deref().unwrap_or("user").parse()?;
    let empty_values = std::collections::BTreeMap::new();
    let empty_emotions = Vec::new();
    crate::log_command!(
        "create_journal_entry",
        db.with_conn(|conn| {
            SqliteJournalEntryRepo.create(
                conn,
                &dto.workspace_id,
                &dto.occurred_on,
                &dto.title,
                &dto.content_markdown,
                &dto.template_id,
                &dto.template_version,
                dto.template_values.as_ref().unwrap_or(&empty_values),
                &dto.mood_label,
                &dto.energy_label,
                dto.emotions.as_deref().unwrap_or(&empty_emotions),
                dto.export_eligible.unwrap_or(true),
                source,
            )
        })
    )
}

#[tauri::command]
pub fn get_journal_entry(db: State<'_, Db>, id: String) -> Result<JournalEntry, AppError> {
    db.with_conn(|conn| SqliteJournalEntryRepo.get(conn, &id))
}

#[tauri::command]
pub fn update_journal_entry(
    db: State<'_, Db>,
    id: String,
    dto: UpdateJournalEntryDto,
) -> Result<JournalEntry, AppError> {
    crate::log_command!(
        "update_journal_entry",
        db.with_conn(|conn| {
            SqliteJournalEntryRepo.update(
                conn,
                &id,
                &dto.occurred_on,
                &dto.title,
                &dto.content_markdown,
                &dto.template_id,
                &dto.template_version,
                dto.template_values.as_ref(),
                &dto.mood_label,
                &dto.energy_label,
                dto.emotions.as_deref(),
                dto.export_eligible,
            )
        })
    )
}

#[tauri::command]
pub fn list_journal_entries(
    db: State<'_, Db>,
    workspace_id: String,
    occurred_on_from: Option<String>,
    occurred_on_to: Option<String>,
) -> Result<Vec<JournalEntry>, AppError> {
    db.with_conn(|conn| {
        let mut entries = SqliteJournalEntryRepo.list(conn, &workspace_id)?;
        if let Some(from) = occurred_on_from {
            crate::time::parse_date(&from)?;
            entries.retain(|e| e.occurred_on >= from);
        }
        if let Some(to) = occurred_on_to {
            crate::time::parse_date(&to)?;
            entries.retain(|e| e.occurred_on <= to);
        }
        Ok(entries)
    })
}

#[tauri::command]
pub fn archive_journal_entry(db: State<'_, Db>, id: String) -> Result<JournalEntry, AppError> {
    crate::log_command!(
        "archive_journal_entry",
        db.with_conn(|conn| SqliteJournalEntryRepo.archive(conn, &id))
    )
}

#[tauri::command]
pub fn archive_journal_entries(
    db: State<'_, Db>,
    workspace_id: String,
    entry_ids: Vec<String>,
) -> Result<usize, AppError> {
    archive_journal_entries_for_db(db.inner(), workspace_id, entry_ids)
}

fn archive_journal_entries_for_db(
    db: &Db,
    workspace_id: String,
    entry_ids: Vec<String>,
) -> Result<usize, AppError> {
    crate::log_command!(
        "archive_journal_entries",
        db.with_conn(|conn| {
            SqliteJournalEntryRepo.archive_many(conn, &workspace_id, &entry_ids)
        })
    )
}

#[tauri::command]
pub fn set_journal_entries_export_eligible(
    db: State<'_, Db>,
    workspace_id: String,
    entry_ids: Vec<String>,
    export_eligible: bool,
) -> Result<usize, AppError> {
    set_journal_entries_export_eligible_for_db(db.inner(), workspace_id, entry_ids, export_eligible)
}

fn set_journal_entries_export_eligible_for_db(
    db: &Db,
    workspace_id: String,
    entry_ids: Vec<String>,
    export_eligible: bool,
) -> Result<usize, AppError> {
    crate::log_command!(
        "set_journal_entries_export_eligible",
        db.with_conn(|conn| {
            SqliteJournalEntryRepo.set_export_eligible_many(
                conn,
                &workspace_id,
                &entry_ids,
                export_eligible,
            )
        })
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::types::JournalSource;
    use crate::repos::journal_entry_repo::JournalEntryRepo;
    use crate::state::Db;
    use crate::test_utils::test_conn;
    use serde::Deserialize;
    use std::collections::BTreeMap;
    use std::sync::{Arc, Mutex};

    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct ArchiveEntriesArgs {
        workspace_id: String,
        entry_ids: Vec<String>,
    }

    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct SetExportEligibleArgs {
        workspace_id: String,
        entry_ids: Vec<String>,
        export_eligible: bool,
    }

    fn test_db() -> Db {
        Db(Arc::new(Mutex::new(test_conn())))
    }

    fn create_entry(db: &Db, title: &str, export_eligible: bool) -> String {
        let values = BTreeMap::new();
        db.with_conn(|conn| {
            SqliteJournalEntryRepo
                .create(
                    conn,
                    "w1",
                    "2026-09-04",
                    title,
                    "Synthetic test content.",
                    &None,
                    &None,
                    &values,
                    &None,
                    &None,
                    &[],
                    export_eligible,
                    JournalSource::User,
                )
                .map(|entry| entry.id)
        })
        .expect("create synthetic journal entry")
    }

    #[test]
    fn batch_commands_accept_camel_case_arguments_and_return_changed_count() {
        let db = test_db();
        let archive_ids = vec![
            create_entry(&db, "Archive one", true),
            create_entry(&db, "Archive two", false),
        ];
        let archive_args: ArchiveEntriesArgs = serde_json::from_value(serde_json::json!({
            "workspaceId": "w1",
            "entryIds": archive_ids,
        }))
        .expect("deserialize archive command arguments");
        assert_eq!(
            archive_journal_entries_for_db(&db, archive_args.workspace_id, archive_args.entry_ids,)
                .expect("archive selected entries"),
            2
        );

        let eligible_id = create_entry(&db, "Make exportable", false);
        let eligible_args: SetExportEligibleArgs = serde_json::from_value(serde_json::json!({
            "workspaceId": "w1",
            "entryIds": [eligible_id],
            "exportEligible": true,
        }))
        .expect("deserialize eligibility command arguments");
        assert_eq!(
            set_journal_entries_export_eligible_for_db(
                &db,
                eligible_args.workspace_id,
                eligible_args.entry_ids,
                eligible_args.export_eligible,
            )
            .expect("set selected entry exportable"),
            1
        );
    }

    #[test]
    fn batch_eligibility_command_accepts_false_argument() {
        let db = test_db();
        let entry_id = create_entry(&db, "Make private", true);
        let args: SetExportEligibleArgs = serde_json::from_value(serde_json::json!({
            "workspaceId": "w1",
            "entryIds": [entry_id],
            "exportEligible": false,
        }))
        .expect("deserialize eligibility command arguments");
        assert_eq!(
            set_journal_entries_export_eligible_for_db(
                &db,
                args.workspace_id,
                args.entry_ids,
                args.export_eligible,
            )
            .expect("set selected entry private"),
            1
        );
    }
}
