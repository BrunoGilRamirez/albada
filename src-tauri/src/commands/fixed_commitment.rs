use crate::commands::dto::{CreateFixedCommitmentDto, UpdateFixedCommitmentDto};
use crate::domain::models::FixedCommitment;
use crate::domain::types::FixedCommitmentType;
use crate::error::AppError;
use crate::repos::fixed_commitment_repo::{FixedCommitmentRepo, SqliteFixedCommitmentRepo};
use crate::state::Db;
use tauri::State;

/// Create an immovable commitment. Commitments are not tasks and never affect
/// progress.
#[tauri::command]
pub fn create_fixed_commitment(
    db: State<'_, Db>,
    dto: CreateFixedCommitmentDto,
) -> Result<FixedCommitment, AppError> {
    let commitment_type: FixedCommitmentType = dto.commitment_type.parse()?;
    crate::log_command!(
        "create_fixed_commitment",
        db.with_conn(|conn| {
            SqliteFixedCommitmentRepo.create(
                conn,
                &dto.workspace_id,
                &dto.title,
                &dto.description_markdown,
                commitment_type,
                &dto.start_at,
                &dto.end_at,
                &dto.duration_minutes,
                &dto.recurrence_rule,
                dto.movable,
                &dto.privacy_scope,
            )
        })
    )
}

#[tauri::command]
pub fn get_fixed_commitment(db: State<'_, Db>, id: String) -> Result<FixedCommitment, AppError> {
    db.with_conn(|conn| SqliteFixedCommitmentRepo.get(conn, &id))
}

#[tauri::command]
pub fn list_fixed_commitments(
    db: State<'_, Db>,
    workspace_id: String,
    active_only: bool,
) -> Result<Vec<FixedCommitment>, AppError> {
    db.with_conn(|conn| SqliteFixedCommitmentRepo.list(conn, &workspace_id, active_only))
}

#[tauri::command]
pub fn update_fixed_commitment(
    db: State<'_, Db>,
    id: String,
    dto: UpdateFixedCommitmentDto,
) -> Result<FixedCommitment, AppError> {
    let commitment_type: FixedCommitmentType = dto.commitment_type.parse()?;
    db.with_conn(|conn| {
        SqliteFixedCommitmentRepo.update(
            conn,
            &id,
            &dto.title,
            &dto.description_markdown,
            commitment_type,
            &dto.start_at,
            &dto.end_at,
            &dto.duration_minutes,
            &dto.recurrence_rule,
            dto.movable,
        )
    })
}

#[tauri::command]
pub fn set_fixed_commitment_active(
    db: State<'_, Db>,
    id: String,
    active: bool,
) -> Result<FixedCommitment, AppError> {
    crate::log_command!(
        "set_fixed_commitment_active",
        db.with_conn(|conn| SqliteFixedCommitmentRepo.set_active(conn, &id, active))
    )
}

#[tauri::command]
pub fn delete_fixed_commitment(db: State<'_, Db>, id: String) -> Result<(), AppError> {
    crate::log_command!(
        "delete_fixed_commitment",
        db.with_conn(|conn| SqliteFixedCommitmentRepo.delete(conn, &id))
    )
}
