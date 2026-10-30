use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;

use crate::domain::models::Workspace;
use crate::error::AppError;
use crate::repos::workspace_repo::{SqliteWorkspaceRepo, WorkspaceRepo};

pub const DEFAULT_WARNING_WINDOW_DAYS: u32 = 3;
pub const MAX_WARNING_WINDOW_DAYS: u32 = 90;
pub const DEFAULT_BACKUP_REMINDER_INTERVAL_DAYS: u32 = 7;
pub const MIN_BACKUP_REMINDER_INTERVAL_DAYS: u32 = 1;
pub const MAX_BACKUP_REMINDER_INTERVAL_DAYS: u32 = 30;
/// Default block overrun notification interval in minutes.
pub const DEFAULT_BLOCK_OVERRUN_INTERVAL_MINUTES: u32 = 10;
/// Smallest accepted block overrun interval in minutes. Only zero is rejected
/// today; an upper bound is unresolved blocker B3.
pub const MIN_BLOCK_OVERRUN_INTERVAL_MINUTES: u32 = 1;

/// Separate workspace preference for a gentle manual-backup reminder. This
/// setting never creates a backup; the success timestamp is system-owned.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BackupReminderPreference {
    pub enabled: bool,
    pub interval_days: u32,
    pub last_successful_backup_at: Option<String>,
}

impl Default for BackupReminderPreference {
    fn default() -> Self {
        Self {
            enabled: false,
            interval_days: DEFAULT_BACKUP_REMINDER_INTERVAL_DAYS,
            last_successful_backup_at: None,
        }
    }
}

/// Workspace preference for block overrun notifications (Phase 8.7.3). Only
/// the overrun interval is stored here; user-adjustable min/max bounds are
/// unresolved blocker B3.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BlockNotificationPreference {
    pub overrun_interval_minutes: u32,
}

impl Default for BlockNotificationPreference {
    fn default() -> Self {
        Self {
            overrun_interval_minutes: DEFAULT_BLOCK_OVERRUN_INTERVAL_MINUTES,
        }
    }
}

/// Workspace preferences stored in `workspaces.settings_json`.
///
/// Unknown keys are retained so updating one preference cannot erase another
/// setting introduced by a later feature.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkspaceSettings {
    #[serde(default = "default_warning_window_days")]
    pub warning_window_days: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub backup_reminder: Option<BackupReminderPreference>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub block_notifications: Option<BlockNotificationPreference>,
    /// Preserve future and unrelated workspace preference keys on updates.
    #[serde(flatten)]
    extra: BTreeMap<String, Value>,
}

fn default_warning_window_days() -> u32 {
    DEFAULT_WARNING_WINDOW_DAYS
}

impl Default for WorkspaceSettings {
    fn default() -> Self {
        Self {
            warning_window_days: DEFAULT_WARNING_WINDOW_DAYS,
            backup_reminder: None,
            block_notifications: None,
            extra: BTreeMap::new(),
        }
    }
}

/// Read the workspace settings, falling back to defaults when `settings_json`
/// is empty or not valid JSON.
pub fn load(conn: &Connection, workspace_id: &str) -> Result<WorkspaceSettings, AppError> {
    let json: Option<String> = conn
        .query_row(
            "SELECT settings_json FROM workspaces WHERE id = ?1",
            [workspace_id],
            |row| row.get(0),
        )
        .optional()?
        .ok_or_else(|| AppError::not_found("The workspace was not found."))?;

    let settings = match json.as_deref() {
        Some(text) if !text.trim().is_empty() => {
            match serde_json::from_str::<WorkspaceSettings>(text) {
                Ok(settings) => settings,
                Err(err) => {
                    log::warn!(
                    "workspace {workspace_id}: unparseable settings_json, using defaults: {err}"
                );
                    WorkspaceSettings::default()
                }
            }
        }
        _ => WorkspaceSettings::default(),
    };
    log::debug!(
        "workspace {workspace_id}: settings warning_window_days={}",
        settings.warning_window_days
    );
    Ok(settings)
}

/// Validate that a warning window is within the allowed range (0-90 days).
pub fn validate_warning_window(days: u32) -> Result<(), AppError> {
    if days > MAX_WARNING_WINDOW_DAYS {
        return Err(AppError::validation(format!(
            "The warning window must be between 0 and {MAX_WARNING_WINDOW_DAYS} days."
        )));
    }
    Ok(())
}

/// Persist a new warning window, returning the updated workspace.
pub fn update_warning_window(
    conn: &Connection,
    workspace_id: &str,
    days: u32,
) -> Result<Workspace, AppError> {
    validate_warning_window(days)?;
    let mut settings = load(conn, workspace_id)?;
    settings.warning_window_days = days;
    save(conn, workspace_id, &settings)?;
    log::info!("workspace {workspace_id}: warning window set to {days} days");
    SqliteWorkspaceRepo.get(conn, workspace_id)
}

fn save(
    conn: &Connection,
    workspace_id: &str,
    settings: &WorkspaceSettings,
) -> Result<(), AppError> {
    let json = serde_json::to_string(&settings).map_err(|err| {
        log::error!("workspace {workspace_id}: failed to serialize settings: {err}");
        AppError::unknown()
    })?;
    let updated = conn.execute(
        "UPDATE workspaces SET settings_json = ?2, updated_at = ?3 WHERE id = ?1",
        params![workspace_id, json, crate::time::now_rfc3339()],
    )?;
    if updated == 0 {
        return Err(AppError::not_found("The workspace was not found."));
    }
    Ok(())
}

pub fn load_backup_reminder(
    conn: &Connection,
    workspace_id: &str,
) -> Result<BackupReminderPreference, AppError> {
    Ok(load(conn, workspace_id)?
        .backup_reminder
        .unwrap_or_default())
}

pub fn validate_backup_reminder_interval(days: u32) -> Result<(), AppError> {
    if !(MIN_BACKUP_REMINDER_INTERVAL_DAYS..=MAX_BACKUP_REMINDER_INTERVAL_DAYS).contains(&days) {
        return Err(AppError::validation(format!(
            "The backup reminder interval must be between {MIN_BACKUP_REMINDER_INTERVAL_DAYS} and {MAX_BACKUP_REMINDER_INTERVAL_DAYS} days."
        )));
    }
    Ok(())
}

pub fn update_backup_reminder(
    conn: &Connection,
    workspace_id: &str,
    enabled: bool,
    interval_days: u32,
) -> Result<BackupReminderPreference, AppError> {
    validate_backup_reminder_interval(interval_days)?;
    let mut settings = load(conn, workspace_id)?;
    let mut preference = settings.backup_reminder.unwrap_or_default();
    preference.enabled = enabled;
    preference.interval_days = interval_days;
    settings.backup_reminder = Some(preference.clone());
    save(conn, workspace_id, &settings)?;
    Ok(preference)
}

/// Validate the block overrun interval. Zero is rejected and only zero: an
/// upper bound is unresolved blocker B3, so do not invent one.
pub fn validate_block_overrun_interval_minutes(minutes: u32) -> Result<(), AppError> {
    if minutes < MIN_BLOCK_OVERRUN_INTERVAL_MINUTES {
        return Err(AppError::validation(format!(
            "The block overrun interval must be at least {MIN_BLOCK_OVERRUN_INTERVAL_MINUTES} minute."
        )));
    }
    Ok(())
}

/// Load the block notification preference. A stored interval that is absent or
/// invalid (including zero) falls back to the default rather than failing the
/// notification poll.
pub fn load_block_notification_preference(
    conn: &Connection,
    workspace_id: &str,
) -> Result<BlockNotificationPreference, AppError> {
    let preference = load(conn, workspace_id)?
        .block_notifications
        .unwrap_or_default();
    if validate_block_overrun_interval_minutes(preference.overrun_interval_minutes).is_err() {
        return Ok(BlockNotificationPreference::default());
    }
    Ok(preference)
}

/// Persist the block overrun interval while preserving all other workspace
/// preference keys. User-adjustable min/max bounds are unresolved blocker B3;
/// this setter currently validates only "positive integer minutes".
pub fn update_block_notification_preference(
    conn: &Connection,
    workspace_id: &str,
    overrun_interval_minutes: u32,
) -> Result<BlockNotificationPreference, AppError> {
    validate_block_overrun_interval_minutes(overrun_interval_minutes)?;
    let mut settings = load(conn, workspace_id)?;
    let preference = BlockNotificationPreference {
        overrun_interval_minutes,
    };
    settings.block_notifications = Some(preference.clone());
    save(conn, workspace_id, &settings)?;
    Ok(preference)
}

/// Record backup success only after SQLite validation and checksum verification
/// have both completed.
pub fn record_successful_backup(
    conn: &Connection,
    workspace_id: &str,
    created_at: &str,
) -> Result<(), AppError> {
    let mut settings = load(conn, workspace_id)?;
    let mut preference = settings.backup_reminder.unwrap_or_default();
    preference.last_successful_backup_at = Some(created_at.to_string());
    settings.backup_reminder = Some(preference);
    save(conn, workspace_id, &settings)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::test_conn;

    #[test]
    fn defaults_to_three_days() {
        let conn = test_conn();
        let settings = load(&conn, "w1").expect("load");
        assert_eq!(settings.warning_window_days, 3);
    }

    #[test]
    fn round_trips_warning_window() {
        let conn = test_conn();
        update_warning_window(&conn, "w1", 7).expect("update");
        let settings = load(&conn, "w1").expect("load");
        assert_eq!(settings.warning_window_days, 7);
    }

    #[test]
    fn window_over_ninety_days_is_rejected() {
        let conn = test_conn();
        assert!(update_warning_window(&conn, "w1", 91).is_err());
        let settings = load(&conn, "w1").expect("load");
        assert_eq!(settings.warning_window_days, 3, "unchanged on rejection");
    }

    #[test]
    fn invalid_json_falls_back_to_defaults() {
        let conn = test_conn();
        conn.execute(
            "UPDATE workspaces SET settings_json = 'not-json' WHERE id = 'w1'",
            [],
        )
        .expect("corrupt settings");
        let settings = load(&conn, "w1").expect("load");
        assert_eq!(settings.warning_window_days, DEFAULT_WARNING_WINDOW_DAYS);
    }

    #[test]
    fn updating_preferences_preserves_unrelated_json_keys_and_backup_state() {
        let conn = test_conn();
        conn.execute(
            "UPDATE workspaces SET settings_json =
                 '{\"warning_window_days\":11,\"future_preference\":{\"keep\":true}}'
             WHERE id = 'w1'",
            [],
        )
        .expect("seed unrelated workspace preference");

        let preference = update_backup_reminder(&conn, "w1", true, 12)
            .expect("update backup reminder preference");
        assert!(preference.enabled);
        assert_eq!(preference.interval_days, 12);
        assert_eq!(preference.last_successful_backup_at, None);
        update_warning_window(&conn, "w1", 4).expect("update warning window");
        record_successful_backup(&conn, "w1", "2026-10-07T09:00:00Z")
            .expect("record verified backup success");

        let stored: String = conn
            .query_row(
                "SELECT settings_json FROM workspaces WHERE id = 'w1'",
                [],
                |row| row.get(0),
            )
            .expect("read stored preferences");
        let value: Value = serde_json::from_str(&stored).expect("parse stored settings");
        assert_eq!(value["warning_window_days"], 4);
        assert_eq!(value["future_preference"]["keep"], true);
        assert_eq!(value["backup_reminder"]["enabled"], true);
        assert_eq!(value["backup_reminder"]["interval_days"], 12);
        assert_eq!(
            value["backup_reminder"]["last_successful_backup_at"],
            "2026-10-07T09:00:00Z"
        );
    }

    #[test]
    fn backup_reminder_interval_is_limited_to_one_through_thirty_days() {
        let conn = test_conn();
        for interval in [1, 30] {
            update_backup_reminder(&conn, "w1", true, interval).expect("valid interval");
        }
        assert!(update_backup_reminder(&conn, "w1", true, 0).is_err());
        assert!(update_backup_reminder(&conn, "w1", true, 31).is_err());
        assert_eq!(
            load_backup_reminder(&conn, "w1")
                .expect("load unchanged preference")
                .interval_days,
            30
        );
    }
}
