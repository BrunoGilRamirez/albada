use chrono::Utc;
use tauri::{AppHandle, State};
use tauri_plugin_notification::NotificationExt;

use crate::domain::notifications::{self, DueNotification};
use crate::error::AppError;
use crate::repos::notification_repo;
use crate::state::Db;

const APP_NOTIFICATION_TITLE: &str = "Albada";

/// Persist eligible reminder and backup-reminder deliveries and return due,
/// pre-approved safe messages for the frontend notification plugin.
///
/// This poll is also the bounded retention lifecycle point (Phase 8.7.4): it
/// expires stale scheduled rows and then removes terminal history older than
/// seven days. There is no background worker, and the cleanup never touches
/// scheduled or snoozed rows or the durable dedupe table.
///
/// It is also the block-expiry lifecycle point (Phase 8.7.8): planned blocks
/// past start plus estimated duration become inactive here, with no background
/// worker.
#[tauri::command]
pub fn get_due_notifications(
    db: State<'_, Db>,
    runtime_locale: Option<String>,
) -> Result<Vec<DueNotification>, AppError> {
    let now = Utc::now();
    db.with_conn_mut(|conn| {
        notifications::evaluate_notification_rules_with_locale(
            conn,
            now,
            runtime_locale.as_deref(),
        )?;
        notifications::evaluate_backup_reminders_with_locale(conn, now, runtime_locale.as_deref())?;
        crate::domain::block_notifications::evaluate_block_notifications_with_locale(
            conn,
            now,
            runtime_locale.as_deref(),
        )?;
        notification_repo::expire_stale_deliveries(conn, now)?;
        notification_repo::prune_expired_history(conn, now)?;
        crate::repos::block_repo::expire_elapsed_blocks(conn, now)?;
        notification_repo::list_due_deliveries(conn, now)
    })
}

/// Record the outcome of a native notification send. Failed delivery is
/// technical history only and never changes task state.
#[tauri::command]
pub fn record_notification_delivery(
    db: State<'_, Db>,
    id: String,
    delivered: bool,
) -> Result<(), AppError> {
    let now = Utc::now();
    let delivered_at = delivered.then_some(now);
    db.with_conn(|conn| notification_repo::record_delivery_result(conn, &id, delivered_at, now))
}

/// Show an already scheduled, due safe message through Tauri's native API.
///
/// The title is fixed and the body must exactly match a due message created by
/// the safe-message domain boundary. Frontend delivery uses the plugin JS API
/// directly so permission can be requested there first.
#[tauri::command]
pub fn show_notification(
    app: AppHandle,
    db: State<'_, Db>,
    title: String,
    body: String,
) -> Result<(), AppError> {
    if title != APP_NOTIFICATION_TITLE || body.trim().is_empty() {
        return Err(AppError::validation(
            "Only an approved local notification can be shown.",
        ));
    }
    let is_approved =
        db.with_conn(|conn| notification_repo::is_due_safe_message(conn, &body, Utc::now()))?;
    if !is_approved {
        return Err(AppError::validation(
            "Only an approved local notification can be shown.",
        ));
    }

    app.notification()
        .builder()
        .title(title)
        .body(body)
        .show()
        .map_err(|error| {
            log::error!("Native notification failed: {error}");
            AppError::io("The local notification could not be delivered.")
        })
}
