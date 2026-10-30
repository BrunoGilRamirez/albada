use crate::domain::models::TherapyTaskReference;
use crate::domain::types::TherapyReferenceRelation;
use crate::error::AppError;
use crate::repos::therapy_reference_repo::{
    SqliteTherapyTaskReferenceRepo, TherapyTaskReferenceRepo,
};
use crate::state::Db;
use tauri::State;

#[tauri::command]
pub fn add_therapy_reference(
    db: State<'_, Db>,
    record_id: String,
    task_id: Option<String>,
    relation: String,
    note_markdown: Option<String>,
) -> Result<TherapyTaskReference, AppError> {
    let relation: TherapyReferenceRelation = relation.parse()?;
    crate::log_command!(
        "add_therapy_reference",
        db.with_conn(|conn| {
            SqliteTherapyTaskReferenceRepo.create(
                conn,
                &record_id,
                &task_id,
                relation,
                &note_markdown,
            )
        })
    )
}

#[tauri::command]
pub fn remove_therapy_reference(db: State<'_, Db>, id: String) -> Result<(), AppError> {
    crate::log_command!(
        "remove_therapy_reference",
        db.with_conn(|conn| SqliteTherapyTaskReferenceRepo.remove(conn, &id))
    )
}

#[tauri::command]
pub fn list_references_for_record(
    db: State<'_, Db>,
    record_id: String,
) -> Result<Vec<TherapyTaskReference>, AppError> {
    db.with_conn(|conn| SqliteTherapyTaskReferenceRepo.list_for_record(conn, &record_id))
}
