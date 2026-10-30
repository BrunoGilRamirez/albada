use crate::commands::dto::{CreateQuarterFocusDto, UpdateQuarterFocusDto};
use crate::domain::models::QuarterFocus;
use crate::domain::types::Priority;
use crate::error::AppError;
use crate::repos::quarter_focus_repo::{QuarterFocusRepo, SqliteQuarterFocusRepo};
use crate::state::Db;
use tauri::State;

#[tauri::command]
pub fn create_quarter_focus(
    db: State<'_, Db>,
    dto: CreateQuarterFocusDto,
) -> Result<QuarterFocus, AppError> {
    let priority: Priority = dto.priority.parse()?;
    db.with_conn(|conn| {
        SqliteQuarterFocusRepo.create(
            conn,
            &dto.quarter_id,
            &dto.goal_id,
            &dto.project_id,
            priority,
            &dto.intention_markdown,
            &dto.success_signal_markdown,
        )
    })
}

#[tauri::command]
pub fn get_quarter_focus(db: State<'_, Db>, id: String) -> Result<QuarterFocus, AppError> {
    db.with_conn(|conn| SqliteQuarterFocusRepo.get(conn, &id))
}

#[tauri::command]
pub fn list_quarter_focus(
    db: State<'_, Db>,
    quarter_id: String,
) -> Result<Vec<QuarterFocus>, AppError> {
    db.with_conn(|conn| SqliteQuarterFocusRepo.list_for_quarter(conn, &quarter_id))
}

#[tauri::command]
pub fn update_quarter_focus(
    db: State<'_, Db>,
    id: String,
    dto: UpdateQuarterFocusDto,
) -> Result<QuarterFocus, AppError> {
    let priority: Priority = dto.priority.parse()?;
    db.with_conn(|conn| {
        SqliteQuarterFocusRepo.update(
            conn,
            &id,
            priority,
            &dto.intention_markdown,
            &dto.success_signal_markdown,
            &dto.status,
        )
    })
}
