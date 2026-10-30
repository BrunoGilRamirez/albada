use crate::commands::dto::{CreateGoalDto, UpdateGoalDto};
use crate::domain::models::Goal;
use crate::domain::types::{GoalStatus, Priority};
use crate::error::AppError;
use crate::repos::goal_repo::{GoalRepo, SqliteGoalRepo};
use crate::state::Db;
use tauri::State;

#[tauri::command]
pub fn create_goal(db: State<'_, Db>, dto: CreateGoalDto) -> Result<Goal, AppError> {
    db.with_conn(|conn| {
        SqliteGoalRepo.create(
            conn,
            &dto.workspace_id,
            &dto.year_theme_id,
            &dto.title,
            &dto.description_markdown,
            &dto.why_it_matters_markdown,
            &dto.success_definition_markdown,
        )
    })
}

#[tauri::command]
pub fn get_goal(db: State<'_, Db>, id: String) -> Result<Goal, AppError> {
    db.with_conn(|conn| SqliteGoalRepo.get(conn, &id))
}

#[tauri::command]
pub fn list_goals(db: State<'_, Db>, workspace_id: String) -> Result<Vec<Goal>, AppError> {
    db.with_conn(|conn| SqliteGoalRepo.list(conn, &workspace_id))
}

#[tauri::command]
pub fn update_goal(db: State<'_, Db>, id: String, dto: UpdateGoalDto) -> Result<Goal, AppError> {
    let status: GoalStatus = dto.status.parse()?;
    let priority: Priority = dto.priority.parse()?;
    db.with_conn(|conn| {
        SqliteGoalRepo.update(
            conn,
            &id,
            &dto.title,
            &dto.description_markdown,
            &dto.why_it_matters_markdown,
            &dto.success_definition_markdown,
            status,
            priority,
            &dto.target_date,
        )
    })
}

#[tauri::command]
pub fn archive_goal(db: State<'_, Db>, id: String) -> Result<Goal, AppError> {
    db.with_conn(|conn| SqliteGoalRepo.archive(conn, &id))
}
