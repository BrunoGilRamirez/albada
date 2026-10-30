use crate::commands::dto::{CreateProjectDto, UpdateProjectDto};
use crate::domain::models::Project;
use crate::domain::types::{Priority, ProjectStatus};
use crate::error::AppError;
use crate::repos::project_repo::{ProjectRepo, SqliteProjectRepo};
use crate::state::Db;
use tauri::State;

#[tauri::command]
pub fn create_project(db: State<'_, Db>, dto: CreateProjectDto) -> Result<Project, AppError> {
    db.with_conn(|conn| {
        SqliteProjectRepo.create(
            conn,
            &dto.workspace_id,
            &dto.goal_id,
            &dto.title,
            &dto.description_markdown,
            &dto.why_it_matters_markdown,
            &dto.definition_of_done_markdown,
        )
    })
}

#[tauri::command]
pub fn get_project(db: State<'_, Db>, id: String) -> Result<Project, AppError> {
    db.with_conn(|conn| SqliteProjectRepo.get(conn, &id))
}

#[tauri::command]
pub fn list_projects(db: State<'_, Db>, workspace_id: String) -> Result<Vec<Project>, AppError> {
    db.with_conn(|conn| SqliteProjectRepo.list(conn, &workspace_id))
}

#[tauri::command]
pub fn list_projects_for_goal(
    db: State<'_, Db>,
    goal_id: String,
) -> Result<Vec<Project>, AppError> {
    db.with_conn(|conn| SqliteProjectRepo.list_for_goal(conn, &goal_id))
}

#[tauri::command]
pub fn update_project(
    db: State<'_, Db>,
    id: String,
    dto: UpdateProjectDto,
) -> Result<Project, AppError> {
    let status: ProjectStatus = dto.status.parse()?;
    let priority: Priority = dto.priority.parse()?;
    db.with_conn(|conn| {
        SqliteProjectRepo.update(
            conn,
            &id,
            &dto.title,
            &dto.description_markdown,
            &dto.why_it_matters_markdown,
            &dto.definition_of_done_markdown,
            status,
            priority,
            &dto.target_date,
        )
    })
}

#[tauri::command]
pub fn archive_project(db: State<'_, Db>, id: String) -> Result<Project, AppError> {
    db.with_conn(|conn| SqliteProjectRepo.archive(conn, &id))
}

/// Link an existing project to a goal (or detach it by omitting `goal_id`).
#[tauri::command]
pub fn move_project(
    db: State<'_, Db>,
    id: String,
    goal_id: Option<String>,
) -> Result<Project, AppError> {
    crate::log_command!(
        "move_project",
        db.with_conn(|conn| SqliteProjectRepo.move_goal(conn, &id, &goal_id))
    )
}
