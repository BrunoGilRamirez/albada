use crate::commands::dto::{CreateAvailabilityProfileDto, UpdateAvailabilityProfileDto};
use crate::domain::models::AvailabilityProfile;
use crate::error::AppError;
use crate::repos::availability_profile_repo::{
    AvailabilityProfileRepo, SqliteAvailabilityProfileRepo,
};
use crate::state::Db;
use tauri::State;

/// Create an availability profile for a weekday. Availability is suggestion
/// input only.
#[tauri::command]
pub fn create_availability_profile(
    db: State<'_, Db>,
    dto: CreateAvailabilityProfileDto,
) -> Result<AvailabilityProfile, AppError> {
    crate::log_command!(
        "create_availability_profile",
        db.with_conn(|conn| {
            SqliteAvailabilityProfileRepo.create(
                conn,
                &dto.workspace_id,
                dto.weekday,
                &dto.start_time,
                &dto.end_time,
                &dto.preferred_block_length_minutes,
                &dto.energy_expectation,
                &dto.active_from,
                &dto.active_until,
                &dto.note_markdown,
            )
        })
    )
}

#[tauri::command]
pub fn get_availability_profile(
    db: State<'_, Db>,
    id: String,
) -> Result<AvailabilityProfile, AppError> {
    db.with_conn(|conn| SqliteAvailabilityProfileRepo.get(conn, &id))
}

#[tauri::command]
pub fn list_availability_profiles(
    db: State<'_, Db>,
    workspace_id: String,
) -> Result<Vec<AvailabilityProfile>, AppError> {
    db.with_conn(|conn| SqliteAvailabilityProfileRepo.list(conn, &workspace_id))
}

#[tauri::command]
pub fn update_availability_profile(
    db: State<'_, Db>,
    id: String,
    dto: UpdateAvailabilityProfileDto,
) -> Result<AvailabilityProfile, AppError> {
    db.with_conn(|conn| {
        SqliteAvailabilityProfileRepo.update(
            conn,
            &id,
            dto.weekday,
            &dto.start_time,
            &dto.end_time,
            &dto.preferred_block_length_minutes,
            &dto.energy_expectation,
            &dto.active_from,
            &dto.active_until,
            &dto.note_markdown,
        )
    })
}

#[tauri::command]
pub fn delete_availability_profile(db: State<'_, Db>, id: String) -> Result<(), AppError> {
    crate::log_command!(
        "delete_availability_profile",
        db.with_conn(|conn| SqliteAvailabilityProfileRepo.delete(conn, &id))
    )
}
