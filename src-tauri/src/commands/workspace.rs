use crate::commands::dto::{
    CreateWorkspaceDto, UpdateBackupReminderPreferenceDto, UpdateWorkspaceDto,
};
use crate::domain::models::Workspace;
use crate::domain::settings;
use crate::error::AppError;
use crate::repos::workspace_repo::{SqliteWorkspaceRepo, WorkspaceRepo};
use crate::state::Db;
use tauri::State;

#[tauri::command]
pub fn create_workspace(db: State<'_, Db>, dto: CreateWorkspaceDto) -> Result<Workspace, AppError> {
    crate::log_command!(
        "create_workspace",
        db.with_conn(|conn| SqliteWorkspaceRepo.create(conn, &dto.name))
    )
}

#[tauri::command]
pub fn get_workspace(db: State<'_, Db>, id: String) -> Result<Workspace, AppError> {
    db.with_conn(|conn| SqliteWorkspaceRepo.get(conn, &id))
}

#[tauri::command]
pub fn list_workspaces(db: State<'_, Db>) -> Result<Vec<Workspace>, AppError> {
    db.with_conn(|conn| SqliteWorkspaceRepo.list(conn))
}

#[tauri::command]
pub fn update_workspace(
    db: State<'_, Db>,
    id: String,
    dto: UpdateWorkspaceDto,
) -> Result<Workspace, AppError> {
    db.with_conn(|conn| SqliteWorkspaceRepo.update_name(conn, &id, &dto.name))
}

/// Persist a supported manual locale preference for one workspace.
#[tauri::command]
pub fn update_workspace_locale(
    db: State<'_, Db>,
    workspace_id: String,
    locale: String,
) -> Result<Workspace, AppError> {
    crate::log_command!(
        "update_workspace_locale",
        db.with_conn(|conn| SqliteWorkspaceRepo.update_locale(conn, &workspace_id, &locale))
    )
}

/// Return the default workspace, creating it on first run.
#[tauri::command]
pub fn ensure_default_workspace(db: State<'_, Db>) -> Result<Workspace, AppError> {
    crate::log_command!(
        "ensure_default_workspace",
        db.with_conn(|conn| SqliteWorkspaceRepo.ensure_default(conn))
    )
}

/// Current warning window (in days) for deadline conditions.
#[tauri::command]
pub fn get_warning_window(db: State<'_, Db>, workspace_id: String) -> Result<u32, AppError> {
    db.with_conn(|conn| settings::load(conn, &workspace_id).map(|s| s.warning_window_days))
}

/// Update the warning window (0-90 days) used to derive deadline conditions.
#[tauri::command]
pub fn update_warning_window(
    db: State<'_, Db>,
    workspace_id: String,
    days: u32,
) -> Result<Workspace, AppError> {
    crate::log_command!(
        "update_warning_window",
        db.with_conn(|conn| settings::update_warning_window(conn, &workspace_id, days))
    )
}

/// Read the separate manual-backup reminder preference for a workspace.
#[tauri::command]
pub fn get_backup_reminder_preference(
    db: State<'_, Db>,
    workspace_id: String,
) -> Result<crate::domain::settings::BackupReminderPreference, AppError> {
    crate::id::parse_uuid(&workspace_id)?;
    db.with_conn(|conn| crate::domain::settings::load_backup_reminder(conn, &workspace_id))
}

/// Update only the user-editable backup-reminder fields. No backup is created.
#[tauri::command]
pub fn update_backup_reminder_preference(
    db: State<'_, Db>,
    workspace_id: String,
    dto: UpdateBackupReminderPreferenceDto,
) -> Result<crate::domain::settings::BackupReminderPreference, AppError> {
    crate::id::parse_uuid(&workspace_id)?;
    crate::log_command!(
        "update_backup_reminder_preference",
        db.with_conn(|conn| {
            crate::domain::settings::update_backup_reminder(
                conn,
                &workspace_id,
                dto.enabled,
                dto.interval_days,
            )
        })
    )
}
