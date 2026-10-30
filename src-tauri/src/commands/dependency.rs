use crate::commands::dto::{AddDependencyDto, RemoveDependencyDto};
use crate::domain::dependency_graph::PrerequisiteTask;
use crate::domain::models::TaskDependency;
use crate::error::AppError;
use crate::repos::dependency_repo::{DependencyRepo, SqliteDependencyRepo};
use crate::state::Db;
use tauri::State;

#[tauri::command]
pub fn add_dependency(
    db: State<'_, Db>,
    dto: AddDependencyDto,
) -> Result<TaskDependency, AppError> {
    log::info!(
        "add_dependency: blocked_task_id={} prerequisite_task_id={}",
        dto.blocked_task_id,
        dto.prerequisite_task_id
    );
    crate::log_command!(
        "add_dependency",
        db.with_tx(|tx| {
            SqliteDependencyRepo.add_dependency(
                tx,
                &dto.blocked_task_id,
                &dto.prerequisite_task_id,
                &dto.note_markdown,
            )
        })
    )
}

#[tauri::command]
pub fn remove_dependency(
    db: State<'_, Db>,
    id: String,
    dto: RemoveDependencyDto,
) -> Result<(), AppError> {
    log::info!("remove_dependency: dependency_id={}", id);
    crate::log_command!(
        "remove_dependency",
        db.with_conn(|conn| {
            SqliteDependencyRepo.remove_dependency(conn, &id, &dto.reason_markdown)
        })
    )
}

#[tauri::command]
pub fn list_blocking_tasks(
    db: State<'_, Db>,
    task_id: String,
) -> Result<Vec<PrerequisiteTask>, AppError> {
    db.with_conn(|conn| SqliteDependencyRepo.list_blocking_tasks(conn, &task_id))
}

#[tauri::command]
pub fn list_all_prerequisites(
    db: State<'_, Db>,
    task_id: String,
) -> Result<Vec<PrerequisiteTask>, AppError> {
    db.with_conn(|conn| crate::domain::dependency_graph::all_active_prerequisites(conn, &task_id))
}

#[tauri::command]
pub fn list_dependencies_for_task(
    db: State<'_, Db>,
    task_id: String,
) -> Result<Vec<TaskDependency>, AppError> {
    db.with_conn(|conn| SqliteDependencyRepo.list_for_task(conn, &task_id))
}
