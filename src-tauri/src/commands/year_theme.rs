use crate::commands::dto::{CreateYearThemeDto, UpdateYearThemeDto};
use crate::domain::models::YearTheme;
use crate::error::AppError;
use crate::repos::year_theme_repo::{SqliteYearThemeRepo, YearThemeRepo};
use crate::state::Db;
use tauri::State;

#[tauri::command]
pub fn create_year_theme(
    db: State<'_, Db>,
    dto: CreateYearThemeDto,
) -> Result<YearTheme, AppError> {
    db.with_conn(|conn| {
        SqliteYearThemeRepo.create(
            conn,
            &dto.workspace_id,
            dto.year,
            &dto.name,
            &dto.description_markdown,
        )
    })
}

#[tauri::command]
pub fn get_year_theme(db: State<'_, Db>, id: String) -> Result<YearTheme, AppError> {
    db.with_conn(|conn| SqliteYearThemeRepo.get(conn, &id))
}

#[tauri::command]
pub fn list_year_themes(
    db: State<'_, Db>,
    workspace_id: String,
) -> Result<Vec<YearTheme>, AppError> {
    db.with_conn(|conn| SqliteYearThemeRepo.list(conn, &workspace_id))
}

#[tauri::command]
pub fn update_year_theme(
    db: State<'_, Db>,
    id: String,
    dto: UpdateYearThemeDto,
) -> Result<YearTheme, AppError> {
    db.with_conn(|conn| SqliteYearThemeRepo.update(conn, &id, &dto.name, &dto.description_markdown))
}

#[tauri::command]
pub fn archive_year_theme(db: State<'_, Db>, id: String) -> Result<YearTheme, AppError> {
    db.with_conn(|conn| SqliteYearThemeRepo.archive(conn, &id))
}
