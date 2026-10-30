use std::path::PathBuf;

use crate::error::AppError;
use tauri::{AppHandle, Manager};

/// Default database file name for the MVP.
pub const DEFAULT_DB_FILE_NAME: &str = "albada.sqlite";

/// Resolve the absolute database path under the Tauri app data directory.
///
/// A workspace-level override is a later concern; for now the default path is
/// always used and the directory is created when missing.
pub fn database_path(app_handle: &AppHandle) -> Result<PathBuf, AppError> {
    let data_dir = app_handle.path().app_data_dir()?;
    std::fs::create_dir_all(&data_dir)?;
    Ok(data_dir.join(DEFAULT_DB_FILE_NAME))
}
