use crate::domain::progress::{self, ProgressResult, ProgressScope};
use crate::error::AppError;
use crate::state::Db;
use tauri::State;

/// Compute weighted progress for a project from its eligible leaves.
#[tauri::command]
pub fn get_project_progress(
    db: State<'_, Db>,
    project_id: String,
) -> Result<ProgressResult, AppError> {
    crate::log_command!(
        "get_project_progress",
        db.with_conn(|conn| progress::calculate_progress(
            conn,
            &ProgressScope::Project(&project_id)
        ))
    )
}

/// Compute weighted progress for a goal from its direct and project leaves.
#[tauri::command]
pub fn get_goal_progress(db: State<'_, Db>, goal_id: String) -> Result<ProgressResult, AppError> {
    crate::log_command!(
        "get_goal_progress",
        db.with_conn(|conn| progress::calculate_progress(conn, &ProgressScope::Goal(&goal_id)))
    )
}

/// Compute weighted progress for a task (itself, or its descendant leaves).
#[tauri::command]
pub fn get_task_progress(db: State<'_, Db>, task_id: String) -> Result<ProgressResult, AppError> {
    crate::log_command!(
        "get_task_progress",
        db.with_conn(|conn| progress::calculate_progress(conn, &ProgressScope::Task(&task_id)))
    )
}
