use std::ffi::OsString;
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use crate::domain::models::Workspace;
use crate::error::AppError;
use crate::repos::workspace_repo::{SqliteWorkspaceRepo, WorkspaceRepo};
use crate::state::Db;
use rusqlite::OptionalExtension;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tauri::{AppHandle, Manager, State};

/// A user-created backup or an application-created pre-restore recovery copy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BackupKind {
    User,
    Recovery,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum BackupValidationStatus {
    Valid,
    Unvalidated,
    Invalid,
}

/// A backup history row. Metadata is absent only for an incomplete or damaged
/// legacy file; it remains visible but cannot pass restore validation.
#[derive(Debug, Clone, Serialize)]
pub struct BackupDto {
    pub path: String,
    pub created_at: String,
    pub size_bytes: u64,
    pub checksum: Option<String>,
    pub schema_version: Option<i64>,
    pub source_workspace_id: Option<String>,
    pub kind: BackupKind,
    pub validation_status: BackupValidationStatus,
}

/// Read-only preflight result. A valid result's checksum is the confirmation
/// token the UI must pass to `restore_backup` after the user confirms.
#[derive(Debug, Clone, Serialize)]
pub struct BackupValidationDto {
    pub status: BackupValidationStatus,
    pub checksum: Option<String>,
    pub schema_version: Option<i64>,
    pub size_bytes: Option<u64>,
    pub kind: Option<BackupKind>,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct BackupManifest {
    format_version: u32,
    created_at: String,
    source_workspace_id: String,
    schema_version: i64,
    checksum: String,
    kind: BackupKind,
}

/// Backup file name prefix and suffix used in the application data directory.
/// Files created before the Albada rename used `projectlife-*` names; those
/// legacy prefixes stay recognizable so migrated copies remain listed and
/// restorable from Settings.
pub const BACKUP_PREFIX: &str = "albada-";
pub const RECOVERY_BACKUP_PREFIX: &str = "albada-recovery-";
const LEGACY_BACKUP_PREFIX: &str = "projectlife-";
const LEGACY_RECOVERY_BACKUP_PREFIX: &str = "projectlife-recovery-";
pub const BACKUP_SUFFIX: &str = ".db.backup";
const MANIFEST_SUFFIX: &str = ".manifest.json";
const MANIFEST_FORMAT_VERSION: u32 = 1;

/// Create and validate a user backup from the SQLite online backup API.
pub fn create_backup_at(data_dir: &Path, db: &Db) -> Result<BackupDto, AppError> {
    // Keep the shared application connection available while SQLite copies and
    // validates the database. The long-running backup work uses its own
    // connection; only the final reminder timestamp touches the shared state.
    let db_path = data_dir.join(crate::db::path::DEFAULT_DB_FILE_NAME);
    let mut source = crate::db::connection::open(&db_path)?;
    let backup = create_backup_from_connection(data_dir, &mut source, BackupKind::User)?;
    let workspace_id = backup
        .source_workspace_id
        .as_deref()
        .ok_or_else(AppError::unknown)?;
    db.with_conn_mut(|conn| {
        crate::domain::settings::record_successful_backup(conn, workspace_id, &backup.created_at)
    })?;
    Ok(backup)
}

fn create_backup_from_connection(
    data_dir: &Path,
    conn: &mut rusqlite::Connection,
    kind: BackupKind,
) -> Result<BackupDto, AppError> {
    let workspace = SqliteWorkspaceRepo.ensure_default(conn)?;
    let schema_version = crate::db::migrations::migration_head()?;
    let backup_id = crate::id::new_uuid();
    let prefix = match kind {
        BackupKind::User => BACKUP_PREFIX,
        BackupKind::Recovery => RECOVERY_BACKUP_PREFIX,
    };
    let backup_path = data_dir.join(format!("{prefix}{backup_id}{BACKUP_SUFFIX}"));
    let staged_path = data_dir.join(format!(".backup-stage-{backup_id}.db"));
    let staged_manifest_path = data_dir.join(format!(".backup-stage-{backup_id}.manifest"));
    let created_at = crate::time::now_rfc3339();

    let result = (|| {
        let mut destination = rusqlite::Connection::open(&staged_path)?;
        {
            let backup = rusqlite::backup::Backup::new(conn, &mut destination)?;
            backup.run_to_completion(5, std::time::Duration::from_millis(250), None)?;
        }
        destination.pragma_update(None, "journal_mode", "DELETE")?;
        destination
            .close()
            .map_err(|(_, error)| AppError::from(error))?;

        let checksum = sha256_file(&staged_path)?;
        let manifest = BackupManifest {
            format_version: MANIFEST_FORMAT_VERSION,
            created_at: created_at.clone(),
            source_workspace_id: workspace.id.clone(),
            schema_version,
            checksum: checksum.clone(),
            kind,
        };
        validate_database_copy(data_dir, &staged_path, &manifest)?;

        let manifest_json = serde_json::to_vec(&manifest)
            .map_err(|_| AppError::database("Backup metadata could not be prepared."))?;
        let mut manifest_file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&staged_manifest_path)?;
        manifest_file.write_all(&manifest_json)?;
        manifest_file.sync_all()?;

        fs::rename(&staged_path, &backup_path)?;
        if let Err(error) = fs::rename(&staged_manifest_path, manifest_path(&backup_path)) {
            let _ = fs::remove_file(&backup_path);
            return Err(AppError::from(error));
        }

        let size_bytes = fs::metadata(&backup_path)?.len();
        Ok(BackupDto {
            path: backup_path.to_string_lossy().to_string(),
            created_at,
            size_bytes,
            checksum: Some(checksum),
            schema_version: Some(schema_version),
            source_workspace_id: Some(workspace.id),
            kind,
            validation_status: BackupValidationStatus::Valid,
        })
    })();

    if result.is_err() {
        let _ = fs::remove_file(&staged_path);
        let _ = fs::remove_file(&staged_manifest_path);
    }
    result
}

/// List user and recovery copies in creation order, retaining broken files in
/// history so the UI can distinguish them from validated restore candidates.
pub fn list_backups_at(data_dir: &Path) -> Result<Vec<BackupDto>, AppError> {
    let mut backups = Vec::new();
    for entry in fs::read_dir(data_dir)? {
        let entry = entry?;
        let path = entry.path();
        if !entry.file_type()?.is_file() {
            continue;
        }
        let name = path
            .file_name()
            .and_then(|part| part.to_str())
            .unwrap_or_default();
        if !is_backup_name(name) {
            continue;
        }
        let metadata = fs::metadata(&path)?;
        let kind = if is_recovery_backup_name(name) {
            BackupKind::Recovery
        } else {
            BackupKind::User
        };
        let manifest = read_manifest(&path).ok();
        backups.push(BackupDto {
            created_at: manifest
                .as_ref()
                .map(|item| item.created_at.clone())
                .unwrap_or_else(|| modified_at(&metadata)),
            path: path.to_string_lossy().to_string(),
            size_bytes: metadata.len(),
            checksum: manifest.as_ref().map(|item| item.checksum.clone()),
            schema_version: manifest.as_ref().map(|item| item.schema_version),
            source_workspace_id: manifest
                .as_ref()
                .map(|item| item.source_workspace_id.clone()),
            kind,
            validation_status: if manifest.is_some() {
                BackupValidationStatus::Unvalidated
            } else {
                BackupValidationStatus::Invalid
            },
        });
    }
    backups.sort_by(|a, b| b.created_at.cmp(&a.created_at));
    Ok(backups)
}

/// Delete a backup file and its sidecar manifest from the application data
/// directory. The active database and files outside that directory are never
/// eligible for deletion.
pub fn delete_backup_at(data_dir: &Path, backup_path: &Path) -> Result<(), AppError> {
    let data_dir = data_dir
        .canonicalize()
        .map_err(|_| AppError::validation("The backup directory is unavailable."))?;
    let backup_path = backup_path
        .canonicalize()
        .map_err(|_| AppError::validation("The backup file was not found."))?;
    let name = backup_path
        .file_name()
        .and_then(|part| part.to_str())
        .unwrap_or_default();
    if backup_path.parent() != Some(data_dir.as_path()) || !is_backup_name(name) {
        return Err(AppError::validation(
            "Only backups created by this application can be deleted.",
        ));
    }
    let active_database = data_dir.join(crate::db::path::DEFAULT_DB_FILE_NAME);
    if same_path(&backup_path, &active_database) {
        return Err(AppError::validation(
            "The active database cannot be deleted as a backup.",
        ));
    }
    fs::remove_file(&backup_path)?;
    let _ = fs::remove_file(manifest_path(&backup_path));
    Ok(())
}

/// Validate a user-selected backup in a private isolated copy without changing
/// the selected file or live database.
pub fn validate_backup_at(data_dir: &Path, backup_path: &Path) -> BackupValidationDto {
    match validate_backup_candidate(data_dir, backup_path) {
        Ok((manifest, size_bytes)) => BackupValidationDto {
            status: BackupValidationStatus::Valid,
            checksum: Some(manifest.checksum),
            schema_version: Some(manifest.schema_version),
            size_bytes: Some(size_bytes),
            kind: Some(manifest.kind),
            message: "Backup validation succeeded. It is ready for explicit restore confirmation."
                .to_string(),
        },
        Err(error) => BackupValidationDto {
            status: BackupValidationStatus::Invalid,
            checksum: None,
            schema_version: None,
            size_bytes: None,
            kind: None,
            message: error.message,
        },
    }
}

fn validate_backup_candidate(
    data_dir: &Path,
    backup_path: &Path,
) -> Result<(BackupManifest, u64), AppError> {
    if !backup_path.is_file() {
        return Err(AppError::validation("The backup file was not found."));
    }
    let manifest = read_manifest(backup_path)?;
    if manifest.format_version != MANIFEST_FORMAT_VERSION {
        return Err(AppError::validation(
            "The backup metadata format is not supported.",
        ));
    }
    if !is_backup_name(
        backup_path
            .file_name()
            .and_then(|part| part.to_str())
            .unwrap_or_default(),
    ) || !kind_matches_file_name(backup_path, manifest.kind)
    {
        return Err(AppError::validation("The backup metadata is invalid."));
    }
    if sha256_file(backup_path)? != manifest.checksum {
        return Err(AppError::validation(
            "The backup checksum does not match its metadata.",
        ));
    }
    validate_database_copy(data_dir, backup_path, &manifest)?;
    Ok((manifest, fs::metadata(backup_path)?.len()))
}

fn validate_database_copy(
    data_dir: &Path,
    source: &Path,
    manifest: &BackupManifest,
) -> Result<(), AppError> {
    let current_head = crate::db::migrations::migration_head()?;
    if manifest.schema_version != current_head {
        return Err(AppError::validation(
            "The backup schema version is not supported by this application.",
        ));
    }

    let isolated_path = data_dir.join(format!(".backup-validation-{}.db", crate::id::new_uuid()));
    let result = (|| {
        fs::copy(source, &isolated_path)?;
        if sha256_file(&isolated_path)? != manifest.checksum {
            return Err(AppError::validation(
                "The backup checksum does not match its metadata.",
            ));
        }
        validate_sqlite_file(&isolated_path, current_head)
    })();
    remove_database_and_sidecars(&isolated_path);
    result
}

fn validate_sqlite_file(path: &Path, current_head: i64) -> Result<(), AppError> {
    let conn = rusqlite::Connection::open_with_flags(
        path,
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY | rusqlite::OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )
    .map_err(|_| AppError::validation("The backup is not a readable SQLite database."))?;
    conn.pragma_update(None, "foreign_keys", "ON")
        .map_err(|_| AppError::validation("The backup SQLite checks could not be enabled."))?;
    let foreign_keys: i64 = conn
        .pragma_query_value(None, "foreign_keys", |row| row.get(0))
        .map_err(|_| AppError::validation("The backup SQLite checks could not be enabled."))?;
    if foreign_keys != 1 {
        return Err(AppError::validation(
            "The backup foreign-key checks could not be enabled.",
        ));
    }

    let integrity: String = conn
        .pragma_query_value(None, "integrity_check", |row| row.get(0))
        .map_err(|_| AppError::validation("The backup SQLite integrity check failed."))?;
    if integrity != "ok" {
        return Err(AppError::validation(
            "The backup SQLite integrity check failed.",
        ));
    }
    let foreign_key_error: Option<(String, i64, String, i64)> = conn
        .prepare("PRAGMA foreign_key_check")
        .and_then(|mut stmt| {
            stmt.query_row([], |row| {
                Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?))
            })
            .optional()
        })
        .map_err(|_| AppError::validation("The backup foreign-key check failed."))?;
    if foreign_key_error.is_some() {
        return Err(AppError::validation(
            "The backup contains invalid foreign-key relationships.",
        ));
    }

    for table in REQUIRED_TABLES {
        let exists: bool = conn
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = ?1)",
                [table],
                |row| row.get(0),
            )
            .map_err(|_| AppError::validation("The backup is missing required database tables."))?;
        if !exists {
            return Err(AppError::validation(
                "The backup is missing required database tables.",
            ));
        }
    }

    let (version, count): (i64, i64) = conn
        .query_row(
            "SELECT COALESCE(MAX(version), 0), COUNT(*) FROM _migrations",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .map_err(|_| AppError::validation("The backup migration history is invalid."))?;
    if version != current_head || count != current_head {
        return Err(AppError::validation(
            "The backup does not match the current database schema version.",
        ));
    }
    Ok(())
}

const REQUIRED_TABLES: &[&str] = &[
    "workspaces",
    "year_themes",
    "quarters",
    "quarter_focus",
    "goals",
    "projects",
    "tasks",
    "task_dependencies",
    "task_status_history",
    "task_links",
    "daily_plans",
    "daily_plan_tasks",
    "blocks",
    "task_sessions",
    "fixed_commitments",
    "availability_profiles",
    "journal_entries",
    "journal_task_references",
    "therapy_collections",
    "therapy_records",
    "therapy_task_references",
    "import_batches",
    "imported_records",
    "export_jobs",
    "export_artifacts",
    "reminder_rules",
    "notification_deliveries",
    "_migrations",
];

/// Restore only a validated candidate whose checksum was returned by the
/// preflight command and confirmed by the user at the UI boundary.
pub fn restore_backup_at(
    data_dir: &Path,
    db: &Db,
    backup_path: &Path,
    expected_checksum: &str,
) -> Result<Workspace, AppError> {
    restore_backup_at_with(data_dir, db, backup_path, expected_checksum, || Ok(()))
}

fn restore_backup_at_with(
    data_dir: &Path,
    db: &Db,
    backup_path: &Path,
    expected_checksum: &str,
    after_install: impl FnOnce() -> Result<(), AppError>,
) -> Result<Workspace, AppError> {
    let (manifest, _) = validate_backup_candidate(data_dir, backup_path)?;
    if !constant_time_eq(&manifest.checksum, expected_checksum) {
        return Err(AppError::validation(
            "The backup changed after preflight validation. Validate it again before restoring.",
        ));
    }
    let target = data_dir.join(crate::db::path::DEFAULT_DB_FILE_NAME);
    if same_path(backup_path, &target) {
        return Err(AppError::validation(
            "The active database cannot be selected as its own backup.",
        ));
    }

    let stage = data_dir.join(format!(".restore-stage-{}.db", crate::id::new_uuid()));
    fs::copy(backup_path, &stage)?;
    if let Err(error) = validate_database_copy(data_dir, &stage, &manifest) {
        remove_database_and_sidecars(&stage);
        return Err(error);
    }

    let mut guard = db.0.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    // This separately identified, verified recovery copy is not a user backup
    // and never updates the user's last-successful-backup preference.
    let recovery_backup =
        match create_backup_from_connection(data_dir, &mut guard, BackupKind::Recovery) {
            Ok(backup) => backup,
            Err(error) => {
                remove_database_and_sidecars(&stage);
                return Err(error);
            }
        };

    let rollback_id = crate::id::new_uuid();
    let previous = data_dir.join(format!(".restore-previous-{rollback_id}.db"));
    let previous_sidecars: Vec<(PathBuf, PathBuf)> = ["-wal", "-shm"]
        .iter()
        .map(|suffix| {
            let live = append_suffix(&target, suffix);
            let prior = append_suffix(&previous, suffix);
            (live, prior)
        })
        .collect();

    let replacement = match crate::db::connection::open_in_memory() {
        Ok(conn) => conn,
        Err(error) => {
            remove_database_and_sidecars(&stage);
            return Err(error);
        }
    };
    let old_conn = std::mem::replace(&mut *guard, replacement);
    if let Err((old_conn, error)) = old_conn.close() {
        *guard = old_conn;
        remove_database_and_sidecars(&stage);
        return Err(AppError::from(error));
    }

    if let Err(error) = fs::rename(&target, &previous) {
        if let Ok(reopened) = crate::db::connection::open(&target) {
            *guard = reopened;
        }
        remove_database_and_sidecars(&stage);
        return Err(AppError::from(error));
    }

    let mut moved_sidecars = Vec::new();
    for (live, prior) in &previous_sidecars {
        if live.exists() {
            if let Err(error) = fs::rename(live, prior) {
                let rollback = rollback_swap(
                    &target,
                    &previous,
                    &moved_sidecars,
                    &stage,
                    Path::new(&recovery_backup.path),
                );
                *guard = open_existing_database(&target)?;
                if rollback.is_err() {
                    return Err(AppError::io(
                        "Restore failed. The previous database remains available in recovery history.",
                    ));
                }
                return Err(AppError::from(error));
            }
            moved_sidecars.push((live.clone(), prior.clone()));
        }
    }

    let installed = (|| {
        fs::rename(&stage, &target)?;
        after_install()?;
        let new_conn = crate::db::connection::open(&target)?;
        crate::db::migrations::run_migrations(&new_conn)?;
        let workspace = SqliteWorkspaceRepo.ensure_default(&new_conn)?;
        Ok((workspace, new_conn))
    })();
    match installed {
        Ok((workspace, new_conn)) => {
            *guard = new_conn;
            let _ = fs::remove_file(&previous);
            for (_, prior) in moved_sidecars {
                let _ = fs::remove_file(prior);
            }
            Ok(workspace)
        }
        Err(error) => {
            let rollback = rollback_swap(
                &target,
                &previous,
                &moved_sidecars,
                &stage,
                Path::new(&recovery_backup.path),
            );
            *guard = open_existing_database(&target).map_err(|_| {
                AppError::io(
                    "Restore failed. The previous database copy was retained for recovery.",
                )
            })?;
            if rollback.is_err() {
                Err(AppError::io(
                    "Restore failed. The previous database remains available in recovery history.",
                ))
            } else {
                Err(error)
            }
        }
    }
}

fn rollback_swap(
    target: &Path,
    previous: &Path,
    sidecars: &[(PathBuf, PathBuf)],
    stage: &Path,
    recovery_backup: &Path,
) -> Result<(), AppError> {
    remove_database_and_sidecars(target);
    if fs::rename(previous, target).is_ok() {
        for (live, prior) in sidecars {
            if prior.exists() {
                let _ = fs::rename(prior, live);
            }
        }
    } else {
        // The pre-restore backup is an independent verified recovery path if
        // the same-directory rollback rename itself fails.
        let fallback = append_suffix(target, ".recovery-stage");
        fs::copy(recovery_backup, &fallback)?;
        remove_database_and_sidecars(target);
        fs::rename(fallback, target)?;
        for (_, prior) in sidecars {
            let _ = fs::remove_file(prior);
        }
    }
    remove_database_and_sidecars(stage);
    if !target.is_file() {
        return Err(AppError::io(
            "The previous database could not be restored from recovery copies.",
        ));
    }
    Ok(())
}

fn open_existing_database(path: &Path) -> Result<rusqlite::Connection, AppError> {
    if !path.is_file() {
        return Err(AppError::io(
            "The active database file is unavailable; a recovery copy was retained.",
        ));
    }
    crate::db::connection::open(path)
}

/// Create a backup in the app-data directory. The pre-restore safety copy is
/// retained in the same history and has a distinct `recovery` kind.
#[tauri::command]
pub async fn create_backup(db: State<'_, Db>, app: AppHandle) -> Result<BackupDto, AppError> {
    let data_dir = app.path().app_data_dir()?;
    let worker_db = db.clone_handle();
    tauri::async_runtime::spawn_blocking(move || {
        crate::log_command!("create_backup", create_backup_at(&data_dir, &worker_db))
    })
    .await
    .map_err(|_| AppError::unknown())?
}

#[tauri::command]
pub fn list_backups(app: AppHandle) -> Result<Vec<BackupDto>, AppError> {
    let data_dir = app.path().app_data_dir()?;
    list_backups_at(&data_dir)
}

#[tauri::command]
pub fn delete_backup(app: AppHandle, path: String) -> Result<(), AppError> {
    let data_dir = app.path().app_data_dir()?;
    crate::log_command!(
        "delete_backup",
        delete_backup_at(&data_dir, Path::new(&path))
    )
}

#[tauri::command]
pub fn validate_backup(app: AppHandle, path: String) -> BackupValidationDto {
    let data_dir = match app.path().app_data_dir() {
        Ok(path) => path,
        Err(_) => {
            return BackupValidationDto {
                status: BackupValidationStatus::Invalid,
                checksum: None,
                schema_version: None,
                size_bytes: None,
                kind: None,
                message: "The backup could not be validated in the local application data area."
                    .to_string(),
            }
        }
    };
    validate_backup_at(&data_dir, Path::new(&path))
}

#[tauri::command]
pub fn restore_backup(
    db: State<'_, Db>,
    app: AppHandle,
    path: String,
    expected_checksum: String,
) -> Result<Workspace, AppError> {
    let data_dir = app.path().app_data_dir()?;
    crate::log_command!(
        "restore_backup",
        restore_backup_at(&data_dir, &db, Path::new(&path), &expected_checksum,)
    )
}

fn read_manifest(path: &Path) -> Result<BackupManifest, AppError> {
    let contents = fs::read_to_string(manifest_path(path))
        .map_err(|_| AppError::validation("The backup metadata is missing or unreadable."))?;
    serde_json::from_str(&contents)
        .map_err(|_| AppError::validation("The backup metadata is invalid."))
}

fn manifest_path(path: &Path) -> PathBuf {
    let mut name = path
        .file_name()
        .map(OsString::from)
        .unwrap_or_else(|| OsString::from("backup"));
    name.push(MANIFEST_SUFFIX);
    path.with_file_name(name)
}

fn is_backup_name(name: &str) -> bool {
    (name.starts_with(BACKUP_PREFIX) || name.starts_with(LEGACY_BACKUP_PREFIX))
        && name.ends_with(BACKUP_SUFFIX)
}

fn is_recovery_backup_name(name: &str) -> bool {
    name.starts_with(RECOVERY_BACKUP_PREFIX) || name.starts_with(LEGACY_RECOVERY_BACKUP_PREFIX)
}

fn kind_matches_file_name(path: &Path, kind: BackupKind) -> bool {
    let name = path
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or_default();
    match kind {
        BackupKind::User => !is_recovery_backup_name(name),
        BackupKind::Recovery => is_recovery_backup_name(name),
    }
}

fn modified_at(metadata: &fs::Metadata) -> String {
    metadata
        .modified()
        .map(|time| {
            let datetime: chrono::DateTime<chrono::Utc> = time.into();
            datetime.to_rfc3339_opts(chrono::SecondsFormat::Secs, true)
        })
        .unwrap_or_else(|_| "unknown".to_string())
}

fn sha256_file(path: &Path) -> Result<String, AppError> {
    let mut file = File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let read = file.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    Ok(hex::encode(hasher.finalize()))
}

fn constant_time_eq(left: &str, right: &str) -> bool {
    left.len() == right.len()
        && left
            .bytes()
            .zip(right.bytes())
            .fold(0_u8, |difference, (a, b)| difference | (a ^ b))
            == 0
}

fn same_path(left: &Path, right: &Path) -> bool {
    match (left.canonicalize(), right.canonicalize()) {
        (Ok(left), Ok(right)) => left == right,
        _ => left == right,
    }
}

fn append_suffix(path: &Path, suffix: &str) -> PathBuf {
    let mut name = path
        .file_name()
        .map(OsString::from)
        .unwrap_or_else(|| OsString::from("database"));
    name.push(suffix);
    path.with_file_name(name)
}

fn remove_database_and_sidecars(path: &Path) {
    let _ = fs::remove_file(path);
    for suffix in ["-wal", "-shm"] {
        let _ = fs::remove_file(append_suffix(path, suffix));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::repos::task_repo::{SqliteTaskRepo, TaskRepo};

    fn temp_data_dir() -> PathBuf {
        use std::sync::atomic::{AtomicU64, Ordering};
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let n = COUNTER.fetch_add(1, Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!("joi-backup-{}-{n}", std::process::id()));
        fs::create_dir_all(&dir).expect("create temp dir");
        dir
    }

    fn seed_task(db: &Db, title: &str) -> String {
        db.with_conn(|conn| {
            let ws = SqliteWorkspaceRepo.ensure_default(conn)?;
            let task = SqliteTaskRepo.create(
                conn,
                &ws.id,
                &None,
                &None,
                &None,
                title,
                &None,
                &None,
                crate::domain::types::Priority::Normal,
                &None,
                &None,
                &None,
            )?;
            Ok(task.id)
        })
        .expect("create synthetic task")
    }

    #[test]
    fn backup_preflight_restore_and_recovery_history_are_integrity_checked() {
        let data_dir = temp_data_dir();
        let db_path = data_dir.join(crate::db::path::DEFAULT_DB_FILE_NAME);
        let db = Db::open(&db_path).expect("open db");
        let task_id = seed_task(&db, "Synthetic backup task");

        let backup = create_backup_at(&data_dir, &db).expect("create and validate backup");
        assert!(backup.size_bytes > 0);
        assert_eq!(backup.kind, BackupKind::User);
        assert_eq!(backup.validation_status, BackupValidationStatus::Valid);
        let settings_json: String = db
            .with_conn(|conn| {
                Ok(
                    conn.query_row("SELECT settings_json FROM workspaces LIMIT 1", [], |row| {
                        row.get(0)
                    })?,
                )
            })
            .expect("read verified backup settings");
        let settings: serde_json::Value =
            serde_json::from_str(&settings_json).expect("parse verified backup settings");
        let last_successful_backup =
            settings["backup_reminder"]["last_successful_backup_at"].as_str();
        assert_eq!(last_successful_backup, Some(backup.created_at.as_str()));
        let preflight = validate_backup_at(&data_dir, Path::new(&backup.path));
        assert_eq!(preflight.status, BackupValidationStatus::Valid);
        assert_eq!(preflight.checksum, backup.checksum);

        db.with_conn(|conn| {
            conn.execute("DELETE FROM tasks WHERE id = ?1", [&task_id])?;
            Ok(())
        })
        .expect("remove task from active database");
        restore_backup_at(
            &data_dir,
            &db,
            Path::new(&backup.path),
            backup.checksum.as_deref().expect("checksum"),
        )
        .expect("restore confirmed backup");
        let restored_count: i64 = db
            .with_conn(|conn| {
                Ok(conn.query_row(
                    "SELECT COUNT(*) FROM tasks WHERE id = ?1",
                    [&task_id],
                    |row| row.get(0),
                )?)
            })
            .expect("read restored task");
        assert_eq!(restored_count, 1);

        let backups = list_backups_at(&data_dir).expect("list backup history");
        assert_eq!(backups.len(), 2);
        assert!(backups.iter().any(|item| item.kind == BackupKind::Recovery));
        let _ = fs::remove_dir_all(&data_dir);
    }

    #[test]
    fn failed_preflight_and_mid_swap_restore_preserve_active_database() {
        let data_dir = temp_data_dir();
        let db_path = data_dir.join(crate::db::path::DEFAULT_DB_FILE_NAME);
        let db = Db::open(&db_path).expect("open db");
        let keep_id = seed_task(&db, "Keep active database");
        let backup = create_backup_at(&data_dir, &db).expect("create backup");
        let checksum = backup.checksum.clone().expect("backup checksum");

        let corrupt_path = Path::new(&backup.path);
        OpenOptions::new()
            .append(true)
            .open(corrupt_path)
            .expect("open synthetic backup")
            .write_all(b"corrupt synthetic bytes")
            .expect("corrupt synthetic backup");
        assert_eq!(
            validate_backup_at(&data_dir, corrupt_path).status,
            BackupValidationStatus::Invalid
        );
        assert!(restore_backup_at(&data_dir, &db, corrupt_path, &checksum).is_err());

        // A valid second backup allows deterministic injection of a failure
        // after the replacement file is staged, exercising the rollback path.
        let valid_backup = create_backup_at(&data_dir, &db).expect("create valid backup");
        db.with_conn(|conn| {
            conn.execute(
                "UPDATE tasks SET title = 'Active remains' WHERE id = ?1",
                [&keep_id],
            )?;
            Ok(())
        })
        .expect("change synthetic live row");
        let expected = valid_backup.checksum.as_deref().expect("checksum");
        let result = restore_backup_at_with(
            &data_dir,
            &db,
            Path::new(&valid_backup.path),
            expected,
            || Err(AppError::validation("Synthetic restore failure.")),
        );
        assert!(result.is_err());
        let live_title: String = db
            .with_conn(|conn| {
                Ok(
                    conn.query_row("SELECT title FROM tasks WHERE id = ?1", [&keep_id], |row| {
                        row.get(0)
                    })?,
                )
            })
            .expect("read still-active database");
        assert_eq!(live_title, "Active remains");
        let _ = fs::remove_dir_all(&data_dir);
    }

    #[test]
    fn list_backups_keeps_recovery_and_user_kinds_distinct() {
        let data_dir = temp_data_dir();
        let db_path = data_dir.join(crate::db::path::DEFAULT_DB_FILE_NAME);
        let db = Db::open(&db_path).expect("open db");
        let _task_id = seed_task(&db, "Synthetic task");
        let workspace_id: String = db
            .with_conn(|conn| Ok(SqliteWorkspaceRepo.ensure_default(conn)?.id))
            .expect("read default workspace id");
        create_backup_at(&data_dir, &db).expect("create user backup");
        let last_successful_before_recovery = db
            .with_conn(|conn| crate::domain::settings::load_backup_reminder(conn, &workspace_id))
            .expect("read user backup timestamp");
        db.with_conn_mut(|conn| {
            create_backup_from_connection(&data_dir, conn, BackupKind::Recovery).map(|_| ())
        })
        .expect("create identified recovery backup");
        let last_successful_after_recovery = db
            .with_conn(|conn| crate::domain::settings::load_backup_reminder(conn, &workspace_id))
            .expect("read timestamp after recovery backup");
        assert_eq!(
            last_successful_after_recovery.last_successful_backup_at,
            last_successful_before_recovery.last_successful_backup_at
        );

        let backups = list_backups_at(&data_dir).expect("list backups");
        assert_eq!(backups.len(), 2);
        assert!(backups.iter().any(|item| item.kind == BackupKind::User));
        assert!(backups.iter().any(|item| item.kind == BackupKind::Recovery));
        let _ = fs::remove_dir_all(&data_dir);
    }

    #[test]
    fn deleting_backup_removes_file_and_manifest() {
        let data_dir = temp_data_dir();
        let db_path = data_dir.join(crate::db::path::DEFAULT_DB_FILE_NAME);
        let db = Db::open(&db_path).expect("open db");
        let backup = create_backup_at(&data_dir, &db).expect("create backup");
        let backup_path = Path::new(&backup.path);
        let manifest = manifest_path(backup_path);

        delete_backup_at(&data_dir, backup_path).expect("delete backup");

        assert!(!backup_path.exists());
        assert!(!manifest.exists());
        assert!(list_backups_at(&data_dir).expect("list backups").is_empty());
        let _ = fs::remove_dir_all(&data_dir);
    }

    #[test]
    fn list_backups_recognizes_legacy_projectlife_names() {
        let data_dir = temp_data_dir();
        fs::write(
            data_dir.join("projectlife-legacy-user.db.backup"),
            b"legacy user",
        )
        .expect("seed legacy user backup");
        fs::write(
            data_dir.join("projectlife-recovery-legacy.db.backup"),
            b"legacy recovery",
        )
        .expect("seed legacy recovery backup");
        fs::write(data_dir.join("albada-new-user.db.backup"), b"new user")
            .expect("seed new backup");

        let backups = list_backups_at(&data_dir).expect("list backups");
        assert_eq!(backups.len(), 3);
        let find = |name: &str| {
            backups
                .iter()
                .find(|item| item.path.ends_with(name))
                .expect("listed backup")
        };
        assert_eq!(
            find("projectlife-legacy-user.db.backup").kind,
            BackupKind::User
        );
        assert_eq!(
            find("projectlife-recovery-legacy.db.backup").kind,
            BackupKind::Recovery
        );
        assert_eq!(find("albada-new-user.db.backup").kind, BackupKind::User);
        let _ = fs::remove_dir_all(&data_dir);
    }
}
