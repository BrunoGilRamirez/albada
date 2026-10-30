use crate::commands::dto::{CreateSessionDto, FinishSessionDto};
use crate::domain::models::TaskSession;
use crate::error::AppError;
use crate::repos::session_repo::{
    finish_session, pause_session, skip_session, start_session, SessionRepo, SqliteSessionRepo,
};
use crate::state::Db;
use tauri::State;

/// Create a session for a task. A session records a work opportunity; finishing
/// it never completes the task.
#[tauri::command]
pub fn create_session(db: State<'_, Db>, dto: CreateSessionDto) -> Result<TaskSession, AppError> {
    crate::log_command!(
        "create_session",
        db.with_conn(|conn| {
            SqliteSessionRepo.create(conn, &dto.task_id, &dto.block_id, &dto.daily_plan_task_id)
        })
    )
}

#[tauri::command]
pub fn get_session(db: State<'_, Db>, id: String) -> Result<TaskSession, AppError> {
    db.with_conn(|conn| SqliteSessionRepo.get(conn, &id))
}

#[tauri::command]
pub fn start_session_cmd(db: State<'_, Db>, id: String) -> Result<TaskSession, AppError> {
    crate::log_command!(
        "start_session",
        db.with_conn(|conn| start_session(conn, &id))
    )
}

#[tauri::command]
pub fn pause_session_cmd(db: State<'_, Db>, id: String) -> Result<TaskSession, AppError> {
    crate::log_command!(
        "pause_session",
        db.with_conn(|conn| pause_session(conn, &id))
    )
}

/// Finish a session, recording the outcome and next step. The task stays
/// open.
#[tauri::command]
pub fn finish_session_cmd(
    db: State<'_, Db>,
    id: String,
    dto: FinishSessionDto,
) -> Result<TaskSession, AppError> {
    crate::log_command!(
        "finish_session",
        db.with_conn(|conn| {
            finish_session(conn, &id, &dto.outcome_markdown, &dto.next_step_markdown)
        })
    )
}

#[tauri::command]
pub fn skip_session_cmd(db: State<'_, Db>, id: String) -> Result<TaskSession, AppError> {
    crate::log_command!("skip_session", db.with_conn(|conn| skip_session(conn, &id)))
}

#[tauri::command]
pub fn list_sessions_for_task(
    db: State<'_, Db>,
    task_id: String,
) -> Result<Vec<TaskSession>, AppError> {
    db.with_conn(|conn| SqliteSessionRepo.list_for_task(conn, &task_id))
}

#[tauri::command]
pub fn list_sessions_for_block(
    db: State<'_, Db>,
    block_id: String,
) -> Result<Vec<TaskSession>, AppError> {
    db.with_conn(|conn| SqliteSessionRepo.list_for_block(conn, &block_id))
}

#[tauri::command]
pub fn list_sessions_for_day_task(
    db: State<'_, Db>,
    daily_plan_task_id: String,
) -> Result<Vec<TaskSession>, AppError> {
    db.with_conn(|conn| SqliteSessionRepo.list_for_daily_plan_task(conn, &daily_plan_task_id))
}
