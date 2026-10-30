use crate::domain::models::JournalTaskReference;
use crate::domain::types::{JournalReferenceEntityType, JournalReferenceRelation};
use crate::error::AppError;
use crate::repos::journal_reference_repo::{
    JournalTaskReferenceRepo, SqliteJournalTaskReferenceRepo,
};
use crate::state::Db;
use tauri::State;

#[tauri::command]
pub fn add_journal_reference(
    db: State<'_, Db>,
    entry_id: String,
    entity_type: String,
    entity_id: String,
    relation: String,
) -> Result<JournalTaskReference, AppError> {
    let entity_type: JournalReferenceEntityType = entity_type.parse()?;
    let relation: JournalReferenceRelation = relation.parse()?;
    crate::log_command!(
        "add_journal_reference",
        db.with_conn(|conn| {
            SqliteJournalTaskReferenceRepo.create(
                conn,
                &entry_id,
                entity_type,
                &entity_id,
                relation,
            )
        })
    )
}

#[tauri::command]
pub fn remove_journal_reference(db: State<'_, Db>, id: String) -> Result<(), AppError> {
    crate::log_command!(
        "remove_journal_reference",
        db.with_conn(|conn| SqliteJournalTaskReferenceRepo.remove(conn, &id))
    )
}

#[tauri::command]
pub fn list_references_for_entry(
    db: State<'_, Db>,
    entry_id: String,
) -> Result<Vec<JournalTaskReference>, AppError> {
    db.with_conn(|conn| SqliteJournalTaskReferenceRepo.list_for_entry(conn, &entry_id))
}
