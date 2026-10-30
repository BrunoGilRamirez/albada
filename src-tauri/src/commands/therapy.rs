use crate::commands::dto::{
    CreateTherapyCollectionDto, CreateTherapyRecordDto, UpdateTherapyCollectionDto,
    UpdateTherapyRecordDto,
};
use crate::domain::models::{TherapyCollection, TherapyRecord};
use crate::domain::types::{TherapyPrivacyScope, TherapyRecordStatus, TherapyRecordType};
use crate::error::AppError;
use crate::repos::therapy_collection_repo::{SqliteTherapyCollectionRepo, TherapyCollectionRepo};
use crate::repos::therapy_record_repo::{SqliteTherapyRecordRepo, TherapyRecordRepo};
use crate::state::Db;
use tauri::State;

#[tauri::command]
pub fn create_therapy_collection(
    db: State<'_, Db>,
    dto: CreateTherapyCollectionDto,
) -> Result<TherapyCollection, AppError> {
    let privacy_scope: TherapyPrivacyScope = dto
        .privacy_scope
        .as_deref()
        .unwrap_or("therapy_private")
        .parse()?;
    crate::log_command!(
        "create_therapy_collection",
        db.with_conn(|conn| {
            SqliteTherapyCollectionRepo.create(
                conn,
                &dto.workspace_id,
                &dto.name,
                privacy_scope,
                &dto.encryption_status,
            )
        })
    )
}

#[tauri::command]
pub fn list_therapy_collections(
    db: State<'_, Db>,
    workspace_id: String,
) -> Result<Vec<TherapyCollection>, AppError> {
    db.with_conn(|conn| SqliteTherapyCollectionRepo.list(conn, &workspace_id))
}

#[tauri::command]
pub fn update_therapy_collection(
    db: State<'_, Db>,
    id: String,
    dto: UpdateTherapyCollectionDto,
) -> Result<TherapyCollection, AppError> {
    let privacy_scope: TherapyPrivacyScope = dto
        .privacy_scope
        .as_deref()
        .unwrap_or("therapy_private")
        .parse()?;
    crate::log_command!(
        "update_therapy_collection",
        db.with_conn(|conn| {
            SqliteTherapyCollectionRepo.update(
                conn,
                &id,
                &dto.name,
                privacy_scope,
                &dto.encryption_status,
            )
        })
    )
}

#[tauri::command]
pub fn archive_therapy_collection(
    db: State<'_, Db>,
    id: String,
) -> Result<TherapyCollection, AppError> {
    crate::log_command!(
        "archive_therapy_collection",
        db.with_conn(|conn| SqliteTherapyCollectionRepo.archive(conn, &id))
    )
}

#[tauri::command]
pub fn create_therapy_record(
    db: State<'_, Db>,
    dto: CreateTherapyRecordDto,
) -> Result<TherapyRecord, AppError> {
    let record_type: TherapyRecordType = dto.record_type.parse()?;
    if let Some(date) = &dto.occurred_on {
        crate::time::parse_date(date)?;
    }
    crate::log_command!(
        "create_therapy_record",
        db.with_conn(|conn| {
            SqliteTherapyRecordRepo.create(
                conn,
                &dto.therapy_collection_id,
                record_type,
                &dto.title,
                &dto.content_markdown,
                &dto.occurred_on,
            )
        })
    )
}

#[tauri::command]
pub fn get_therapy_record(db: State<'_, Db>, id: String) -> Result<TherapyRecord, AppError> {
    db.with_conn(|conn| SqliteTherapyRecordRepo.get(conn, &id))
}

#[tauri::command]
pub fn update_therapy_record(
    db: State<'_, Db>,
    id: String,
    dto: UpdateTherapyRecordDto,
) -> Result<TherapyRecord, AppError> {
    let record_type: TherapyRecordType = dto.record_type.parse()?;
    let status: TherapyRecordStatus = dto.status.parse()?;
    if let Some(date) = &dto.occurred_on {
        crate::time::parse_date(date)?;
    }
    crate::log_command!(
        "update_therapy_record",
        db.with_conn(|conn| {
            SqliteTherapyRecordRepo.update(
                conn,
                &id,
                record_type,
                &dto.title,
                &dto.content_markdown,
                &dto.occurred_on,
                status,
            )
        })
    )
}

#[tauri::command]
pub fn list_therapy_records(
    db: State<'_, Db>,
    collection_id: Option<String>,
) -> Result<Vec<TherapyRecord>, AppError> {
    match collection_id {
        Some(id) => db.with_conn(|conn| SqliteTherapyRecordRepo.list_for_collection(conn, &id)),
        None => Err(AppError::validation(
            "collection_id is required to list therapy records.",
        )),
    }
}

#[tauri::command]
pub fn archive_therapy_record(db: State<'_, Db>, id: String) -> Result<TherapyRecord, AppError> {
    crate::log_command!(
        "archive_therapy_record",
        db.with_conn(|conn| SqliteTherapyRecordRepo.archive(conn, &id))
    )
}
