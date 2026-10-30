use crate::commands::dto::{CreateQuarterDto, UpdateQuarterDto};
use crate::domain::models::Quarter;
use crate::error::AppError;
use crate::repos::quarter_repo::{QuarterRepo, SqliteQuarterRepo};
use crate::state::Db;
use tauri::State;

#[tauri::command]
pub fn create_quarter(db: State<'_, Db>, dto: CreateQuarterDto) -> Result<Quarter, AppError> {
    db.with_conn(|conn| {
        SqliteQuarterRepo.create(
            conn,
            &dto.year_theme_id,
            &dto.code,
            &dto.start_date,
            &dto.end_date,
        )
    })
}

#[tauri::command]
pub fn get_quarter(db: State<'_, Db>, id: String) -> Result<Quarter, AppError> {
    db.with_conn(|conn| SqliteQuarterRepo.get(conn, &id))
}

#[tauri::command]
pub fn list_quarters(db: State<'_, Db>, year_theme_id: String) -> Result<Vec<Quarter>, AppError> {
    db.with_conn(|conn| SqliteQuarterRepo.list(conn, &year_theme_id))
}

#[tauri::command]
pub fn update_quarter(
    db: State<'_, Db>,
    id: String,
    dto: UpdateQuarterDto,
) -> Result<Quarter, AppError> {
    db.with_conn(|conn| {
        SqliteQuarterRepo.update(conn, &id, &dto.start_date, &dto.end_date, &dto.status)
    })
}
