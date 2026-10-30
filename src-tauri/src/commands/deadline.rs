use crate::domain::deadline::{self, DeadlineStatusDto, DeadlineTaskInfo};
use crate::domain::settings;
use crate::error::AppError;
use crate::repos::goal_repo::{GoalRepo, SqliteGoalRepo};
use crate::repos::project_repo::{ProjectRepo, SqliteProjectRepo};
use crate::repos::task_repo::{SqliteTaskRepo, TaskRepo};
use crate::state::Db;
use tauri::State;

/// Load the deadline warning window for a workspace.
fn warning_settings(conn: &rusqlite::Connection, workspace_id: &str) -> Result<u32, AppError> {
    settings::load(conn, workspace_id).map(|s| s.warning_window_days)
}

/// Derived deadline status for a single task.
#[tauri::command]
pub fn get_task_deadline_status(
    db: State<'_, Db>,
    task_id: String,
) -> Result<DeadlineStatusDto, AppError> {
    crate::log_command!(
        "get_task_deadline_status",
        db.with_conn(|conn| {
            let task = SqliteTaskRepo.get(conn, &task_id)?;
            let warning_days = warning_settings(conn, &task.workspace_id)?;
            let now = crate::time::today_utc();
            let status = deadline::deadline_status_for_task(conn, &task_id, &now, warning_days)?;
            let at_risk = if status >= deadline::DeadlineStatus::Approaching {
                vec![DeadlineTaskInfo {
                    task_id: task.id.clone(),
                    title: task.title.clone(),
                    status: task.status.to_string(),
                    deadline_at: task.target_date,
                    deadline_status: status,
                }]
            } else {
                Vec::new()
            };
            Ok(DeadlineStatusDto {
                status,
                at_risk_tasks: at_risk,
                warning_window_days: warning_days,
            })
        })
    )
}

/// Aggregated deadline status across a project's eligible leaves.
#[tauri::command]
pub fn get_project_deadline_status(
    db: State<'_, Db>,
    project_id: String,
) -> Result<DeadlineStatusDto, AppError> {
    crate::log_command!(
        "get_project_deadline_status",
        db.with_conn(|conn| {
            let workspace_id = SqliteProjectRepo.get(conn, &project_id)?.workspace_id;
            let warning_days = warning_settings(conn, &workspace_id)?;
            let now = crate::time::today_utc();
            deadline::deadline_status_for_project(conn, &project_id, &now, warning_days)
        })
    )
}

/// Aggregated deadline status across a goal's direct tasks and project leaves.
#[tauri::command]
pub fn get_goal_deadline_status(
    db: State<'_, Db>,
    goal_id: String,
) -> Result<DeadlineStatusDto, AppError> {
    crate::log_command!(
        "get_goal_deadline_status",
        db.with_conn(|conn| {
            let workspace_id = SqliteGoalRepo.get(conn, &goal_id)?.workspace_id;
            let warning_days = warning_settings(conn, &workspace_id)?;
            let now = crate::time::today_utc();
            deadline::deadline_status_for_goal(conn, &goal_id, &now, warning_days)
        })
    )
}

/// Every open task in the workspace that is at risk, for the dashboard widget.
#[tauri::command]
pub fn list_at_risk_tasks(
    db: State<'_, Db>,
    workspace_id: String,
) -> Result<Vec<DeadlineTaskInfo>, AppError> {
    crate::log_command!(
        "list_at_risk_tasks",
        db.with_conn(|conn| {
            let warning_days = warning_settings(conn, &workspace_id)?;
            let now = crate::time::today_utc();
            deadline::at_risk_tasks(conn, &workspace_id, &now, warning_days)
        })
    )
}
