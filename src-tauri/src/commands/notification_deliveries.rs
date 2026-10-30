//! Thin command boundary for notification delivery history and the explicit
//! dismiss/snooze actions (Phase 8.4).
//!
//! Snoozing reschedules only the notification delivery, never a task or plan,
//! and the domain keeps the original row as history. Returned rows contain
//! only lock-screen-safe message text and fixed technical errors; journal and
//! therapy content can never reach this boundary.

use chrono::Utc;
use tauri::State;

use crate::commands::dto::NotificationDeliveryFiltersDto;
use crate::domain::notifications::{
    self, NotificationDelivery, NotificationDeliveryFilters, NotificationDeliveryKind,
    NotificationDeliveryStatus,
};
use crate::error::AppError;
use crate::repos::notification_repo;
use crate::state::Db;
use crate::time;

/// Default number of history rows returned when no limit is provided.
const DEFAULT_HISTORY_LIMIT: i64 = 100;
/// Hard cap for a single history page.
const MAX_HISTORY_LIMIT: i64 = 500;

/// List notification delivery history for one workspace, newest first.
#[tauri::command]
pub fn list_notification_deliveries(
    db: State<'_, Db>,
    filters: NotificationDeliveryFiltersDto,
) -> Result<Vec<NotificationDelivery>, AppError> {
    crate::id::parse_uuid(&filters.workspace_id)?;
    let reminder_rule_id = match filters.reminder_rule_id.as_deref() {
        Some(rule_id) => {
            crate::id::parse_uuid(rule_id)?;
            Some(rule_id.to_string())
        }
        None => None,
    };
    let status = filters
        .status
        .as_deref()
        .map(|status| status.parse::<NotificationDeliveryStatus>())
        .transpose()?;
    let delivery_kind = filters
        .delivery_kind
        .as_deref()
        .map(str::parse::<NotificationDeliveryKind>)
        .transpose()?;
    let from = filters.from.as_deref().map(time::parse_date).transpose()?;
    let to = filters.to.as_deref().map(time::parse_date).transpose()?;
    if let (Some(from), Some(to)) = (from, to) {
        if from > to {
            return Err(AppError::validation(
                "The delivery history start date must not be after the end date.",
            ));
        }
    }
    let limit = filters.limit.unwrap_or(DEFAULT_HISTORY_LIMIT);
    if !(1..=MAX_HISTORY_LIMIT).contains(&limit) {
        return Err(AppError::validation(format!(
            "The delivery history limit must be between 1 and {MAX_HISTORY_LIMIT}."
        )));
    }

    db.with_conn(|conn| {
        notification_repo::list_deliveries(
            conn,
            &NotificationDeliveryFilters {
                workspace_id: filters.workspace_id,
                reminder_rule_id,
                delivery_kind,
                status,
                from,
                to,
                limit,
            },
        )
    })
}

/// Dismiss a scheduled notification. The delivery row is kept as dismissed
/// history; no task, plan, or session state changes.
#[tauri::command]
pub fn dismiss_notification(
    db: State<'_, Db>,
    id: String,
) -> Result<NotificationDelivery, AppError> {
    crate::id::parse_uuid(&id)?;
    crate::log_command!(
        "dismiss_notification",
        db.with_tx(|conn| notifications::dismiss_delivery(conn, &id))
    )
}

/// Snooze a scheduled notification by one of the rule's configured durations.
/// The original row becomes snoozed history and a new scheduled delivery is
/// created for `now + minutes`.
#[tauri::command]
pub fn snooze_notification(
    db: State<'_, Db>,
    id: String,
    minutes: u32,
) -> Result<NotificationDelivery, AppError> {
    crate::id::parse_uuid(&id)?;
    let now = Utc::now();
    crate::log_command!(
        "snooze_notification",
        db.with_tx(|conn| notifications::snooze_delivery(conn, &id, minutes, now))
    )
}

/// Clear one workspace's in-app notification history (Phase 8.7.4).
///
/// Only terminal (`delivered`, `dismissed`, `failed`) rows at or before the
/// clear timestamp are removed; scheduled and snoozed rows, future scheduled
/// deliveries, delivery-cycle deduplication keys, and all source blocks,
/// tasks, and projects are untouched. Returns the number of removed rows.
/// This clears the in-app history only and never dismisses Windows toast
/// notifications, which remain a separate operating-system interaction.
#[tauri::command]
pub fn clear_notification_history(
    db: State<'_, Db>,
    workspace_id: String,
) -> Result<u64, AppError> {
    crate::id::parse_uuid(&workspace_id)?;
    let now = Utc::now();
    crate::log_command!(
        "clear_notification_history",
        db.with_tx(|conn| notification_repo::clear_workspace_history(conn, &workspace_id, now))
    )
}
