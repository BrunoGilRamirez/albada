//! SQLite persistence for reminder rules and notification delivery history.

use chrono::{DateTime, Utc};
use rusqlite::{params, Connection, OptionalExtension};

use crate::domain::notifications::{
    parse_rule_time, validate_snooze_options, DueNotification, NotificationChannel,
    NotificationDelivery, NotificationDeliveryFilters, NotificationDeliveryKind,
    NotificationDeliveryStatus, NotificationTarget, NotificationTargetType, ReminderEscalation,
    ReminderRule, ReminderRuleConfig, ReminderType,
};
use crate::error::AppError;
use crate::time;

const RULE_COLS: &str = "id, workspace_id, type, enabled, recurrence_rule, preferred_time,
                time_window_start, time_window_end, quiet_hours_start, quiet_hours_end,
                max_reminders_per_day, snooze_minutes_json, channel, escalation,
                privacy_safe_message, deadline_notification_window_days";

const DELIVERY_COLS: &str = "id, workspace_id, delivery_kind, reminder_rule_id,
                    target_entity_type, target_entity_id,
                    scheduled_at, delivered_at, status, channel, safe_message,
                    error_message, created_at, cycle_key";

/// Raw rule row before domain parsing, so nested count queries never run
/// while a statement on the same connection is still active.
struct RuleRow {
    id: String,
    workspace_id: String,
    reminder_type: String,
    enabled: bool,
    recurrence_rule: Option<String>,
    preferred_time: Option<String>,
    time_window_start: Option<String>,
    time_window_end: Option<String>,
    quiet_hours_start: Option<String>,
    quiet_hours_end: Option<String>,
    max_reminders_per_day: i64,
    snooze_minutes_json: String,
    channel: String,
    escalation: String,
    privacy_safe_message: String,
    deadline_notification_window_days: i64,
}

struct DeliveryRow {
    id: String,
    workspace_id: String,
    delivery_kind: String,
    reminder_rule_id: Option<String>,
    target_entity_type: Option<String>,
    target_entity_id: Option<String>,
    scheduled_at: String,
    delivered_at: Option<String>,
    status: String,
    channel: String,
    message: String,
    error_message: Option<String>,
    created_at: String,
    cycle_key: Option<String>,
}

fn read_rule_row(row: &rusqlite::Row<'_>) -> Result<RuleRow, rusqlite::Error> {
    Ok(RuleRow {
        id: row.get(0)?,
        workspace_id: row.get(1)?,
        reminder_type: row.get(2)?,
        enabled: row.get(3)?,
        recurrence_rule: row.get(4)?,
        preferred_time: row.get(5)?,
        time_window_start: row.get(6)?,
        time_window_end: row.get(7)?,
        quiet_hours_start: row.get(8)?,
        quiet_hours_end: row.get(9)?,
        max_reminders_per_day: row.get(10)?,
        snooze_minutes_json: row.get(11)?,
        channel: row.get(12)?,
        escalation: row.get(13)?,
        privacy_safe_message: row.get(14)?,
        deadline_notification_window_days: row.get(15)?,
    })
}

fn read_delivery_row(row: &rusqlite::Row<'_>) -> Result<DeliveryRow, rusqlite::Error> {
    Ok(DeliveryRow {
        id: row.get(0)?,
        workspace_id: row.get(1)?,
        delivery_kind: row.get(2)?,
        reminder_rule_id: row.get(3)?,
        target_entity_type: row.get(4)?,
        target_entity_id: row.get(5)?,
        scheduled_at: row.get(6)?,
        delivered_at: row.get(7)?,
        status: row.get(8)?,
        channel: row.get(9)?,
        message: row.get(10)?,
        error_message: row.get(11)?,
        created_at: row.get(12)?,
        cycle_key: row.get(13)?,
    })
}

fn build_rule(
    conn: &Connection,
    row: RuleRow,
    now: DateTime<Utc>,
) -> Result<ReminderRule, AppError> {
    let (deliveries_today, has_delivery_today) = delivery_count_for_local_day(conn, &row.id, now)?;
    let max_reminders_per_day = u32::try_from(row.max_reminders_per_day)
        .map_err(|_| AppError::validation("The daily reminder limit is invalid."))?;
    let deadline_notification_window_days = u32::try_from(row.deadline_notification_window_days)
        .map_err(|_| AppError::validation("The deadline notification window is invalid."))?;
    crate::domain::notifications::validate_deadline_notification_window(
        deadline_notification_window_days,
    )?;
    Ok(ReminderRule {
        id: row.id,
        workspace_id: row.workspace_id,
        reminder_type: row.reminder_type.parse::<ReminderType>()?,
        enabled: row.enabled,
        recurrence_rule: row.recurrence_rule,
        preferred_time: parse_rule_time(row.preferred_time)?,
        time_window_start: parse_rule_time(row.time_window_start)?,
        time_window_end: parse_rule_time(row.time_window_end)?,
        quiet_hours_start: parse_rule_time(row.quiet_hours_start)?,
        quiet_hours_end: parse_rule_time(row.quiet_hours_end)?,
        max_reminders_per_day,
        snooze_minutes: parse_snooze_minutes_json(&row.snooze_minutes_json)?,
        channel: row.channel.parse::<NotificationChannel>()?,
        escalation: row.escalation.parse::<ReminderEscalation>()?,
        privacy_safe_message: row.privacy_safe_message,
        deadline_notification_window_days,
        deliveries_today,
        has_delivery_today,
    })
}

fn build_delivery(row: DeliveryRow) -> Result<NotificationDelivery, AppError> {
    let parse_timestamp = |value: &str| {
        DateTime::parse_from_rfc3339(value)
            .map(|timestamp| timestamp.with_timezone(&Utc))
            .map_err(|_| AppError::database("A stored notification timestamp is invalid."))
    };
    let scheduled_at = parse_timestamp(&row.scheduled_at)?;
    let delivered_at = row
        .delivered_at
        .as_deref()
        .map(parse_timestamp)
        .transpose()?;
    let target = match (row.target_entity_type, row.target_entity_id) {
        (Some(entity_type), Some(entity_id)) => Some(NotificationTarget {
            entity_type: entity_type
                .parse::<NotificationTargetType>()
                .map_err(|_| AppError::database("A stored notification target type is invalid."))?,
            entity_id,
        }),
        _ => None,
    };
    Ok(NotificationDelivery {
        id: row.id,
        workspace_id: row.workspace_id,
        delivery_kind: row.delivery_kind.parse::<NotificationDeliveryKind>()?,
        reminder_rule_id: row.reminder_rule_id,
        target,
        scheduled_at,
        delivered_at,
        status: row.status.parse::<NotificationDeliveryStatus>()?,
        channel: row.channel.parse::<NotificationChannel>()?,
        message: row.message,
        error_message: row.error_message,
        created_at: row.created_at,
        cycle_key: row.cycle_key,
    })
}

/// Parse and validate the stored `snooze_minutes_json` array.
pub fn parse_snooze_minutes_json(value: &str) -> Result<Vec<u32>, AppError> {
    let options: Vec<u32> = serde_json::from_str(value)
        .map_err(|_| AppError::database("The stored snooze options are invalid."))?;
    validate_snooze_options(&options)?;
    Ok(options)
}

/// Load enabled rules and compute per-rule delivery counts for `now`'s UTC
/// calendar date. All delivery statuses count toward the daily cap.
pub fn list_enabled_rules(
    conn: &Connection,
    now: DateTime<Utc>,
) -> Result<Vec<ReminderRule>, AppError> {
    let sql = format!("SELECT {RULE_COLS} FROM reminder_rules WHERE enabled = 1 ORDER BY id");
    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map([], read_rule_row)?;
    let rows: Vec<RuleRow> = rows.collect::<Result<_, _>>()?;
    drop(stmt);
    rows.into_iter()
        .map(|row| build_rule(conn, row, now))
        .collect()
}

/// Load every rule configured for a workspace, including disabled rules, for
/// the reminder-settings command (Phase 8.4).
pub fn list_rules_for_workspace(
    conn: &Connection,
    workspace_id: &str,
    now: DateTime<Utc>,
) -> Result<Vec<ReminderRule>, AppError> {
    let sql =
        format!("SELECT {RULE_COLS} FROM reminder_rules WHERE workspace_id = ?1 ORDER BY type, id");
    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map([workspace_id], read_rule_row)?;
    let rows: Vec<RuleRow> = rows.collect::<Result<_, _>>()?;
    drop(stmt);
    rows.into_iter()
        .map(|row| build_rule(conn, row, now))
        .collect()
}

/// Load one rule by id with current-day delivery counts.
pub fn get_rule(conn: &Connection, id: &str, now: DateTime<Utc>) -> Result<ReminderRule, AppError> {
    let sql = format!("SELECT {RULE_COLS} FROM reminder_rules WHERE id = ?1");
    let mut stmt = conn.prepare(&sql)?;
    let row = stmt.query_map([id], read_rule_row)?.next().transpose()?;
    drop(stmt);
    let row = row.ok_or_else(|| AppError::not_found("The reminder rule was not found."))?;
    build_rule(conn, row, now)
}

/// Load the workspace focus reminder rule, including a disabled one, for block
/// event gating. Ordered by `id` with `LIMIT 1` so the choice is deterministic
/// even if duplicate types ever exist.
pub fn focus_rule_for_workspace(
    conn: &Connection,
    workspace_id: &str,
    now: DateTime<Utc>,
) -> Result<Option<ReminderRule>, AppError> {
    let sql = format!(
        "SELECT {RULE_COLS} FROM reminder_rules
         WHERE workspace_id = ?1 AND type = 'focus' ORDER BY id LIMIT 1"
    );
    let mut stmt = conn.prepare(&sql)?;
    let row = stmt
        .query_map([workspace_id], read_rule_row)?
        .next()
        .transpose()?;
    drop(stmt);
    row.map(|row| build_rule(conn, row, now)).transpose()
}

/// Validate and persist the editable rule configuration. `recurrence_rule`,
/// ids, and timestamps are not editable through this boundary: recurrence has
/// no defined syntax in the MVP and identity/audit columns are system-owned.
pub fn update_rule(
    conn: &Connection,
    id: &str,
    config: &ReminderRuleConfig,
    now: DateTime<Utc>,
) -> Result<ReminderRule, AppError> {
    config.validate()?;

    // Treat null profile fields as absent rather than as instructions to erase
    // an established user value. When enabling a rule, fill only values that
    // are still absent. The lookup and update share a transaction so an
    // enabled rule cannot be persisted with a partial default profile.
    let tx = conn.unchecked_transaction()?;
    let existing: Option<(Option<String>, Option<String>, Option<String>)> = tx
        .query_row(
            "SELECT preferred_time, quiet_hours_start, quiet_hours_end
             FROM reminder_rules WHERE id = ?1",
            [id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .optional()?;
    let (existing_preferred, existing_quiet_start, existing_quiet_end) =
        existing.ok_or_else(|| AppError::not_found("The reminder rule was not found."))?;

    let profile_value = |requested: &Option<String>, existing: Option<String>, default: &str| {
        requested
            .clone()
            .or(existing)
            .or_else(|| config.enabled.then(|| default.to_string()))
    };
    let preferred_time = profile_value(
        &config.preferred_time,
        existing_preferred,
        crate::domain::notifications::DEFAULT_PREFERRED_NOTIFICATION_TIME,
    );
    let quiet_hours_start = profile_value(
        &config.quiet_hours_start,
        existing_quiet_start,
        crate::domain::notifications::DEFAULT_QUIET_HOURS_START,
    );
    let quiet_hours_end = profile_value(
        &config.quiet_hours_end,
        existing_quiet_end,
        crate::domain::notifications::DEFAULT_QUIET_HOURS_END,
    );

    let mut effective_config = config.clone();
    effective_config.preferred_time = preferred_time.clone();
    effective_config.quiet_hours_start = quiet_hours_start.clone();
    effective_config.quiet_hours_end = quiet_hours_end.clone();
    effective_config.validate()?;

    let snooze_json = serde_json::to_string(&config.snooze_minutes)
        .map_err(|_| AppError::database("The snooze options could not be stored."))?;
    let updated = tx.execute(
        "UPDATE reminder_rules
         SET enabled = ?2, preferred_time = ?3, time_window_start = ?4,
         time_window_end = ?5, quiet_hours_start = ?6, quiet_hours_end = ?7,
              max_reminders_per_day = ?8, snooze_minutes_json = ?9, channel = ?10,
              escalation = ?11, privacy_safe_message = ?12,
              deadline_notification_window_days = ?13, updated_at = ?14
          WHERE id = ?1",
        params![
            id,
            config.enabled,
            preferred_time,
            config.time_window_start,
            config.time_window_end,
            quiet_hours_start,
            quiet_hours_end,
            config.max_reminders_per_day,
            snooze_json,
            config.channel.as_str(),
            config.escalation.as_str(),
            config.privacy_safe_message.trim(),
            config.deadline_notification_window_days,
            time::now_rfc3339(),
        ],
    )?;
    if updated == 0 {
        return Err(AppError::not_found("The reminder rule was not found."));
    }
    tx.commit()?;
    get_rule(conn, id, now)
}

/// Read only the configured snooze durations for a rule. Used by the snooze
/// command to validate the user's explicit choice.
pub fn snooze_options_for_rule(conn: &Connection, rule_id: &str) -> Result<Vec<u32>, AppError> {
    let value: Option<String> = conn
        .query_row(
            "SELECT snooze_minutes_json FROM reminder_rules WHERE id = ?1",
            [rule_id],
            |row| row.get(0),
        )
        .optional()?;
    let value = value.ok_or_else(|| AppError::not_found("The reminder rule was not found."))?;
    parse_snooze_minutes_json(&value)
}

fn delivery_count_for_local_day(
    conn: &Connection,
    reminder_rule_id: &str,
    now: DateTime<Utc>,
) -> Result<(u32, bool), AppError> {
    let mut stmt = conn.prepare(
        "SELECT scheduled_at FROM notification_deliveries
         WHERE delivery_kind = 'reminder_rule' AND reminder_rule_id = ?1",
    )?;
    let rows = stmt.query_map([reminder_rule_id], |row| row.get::<_, String>(0))?;
    let today = now.date_naive();
    let mut count = 0_u32;
    for row in rows {
        let value = row?;
        let timestamp = DateTime::parse_from_rfc3339(&value)
            .map_err(|_| AppError::database("A stored notification timestamp is invalid."))?;
        if timestamp.with_timezone(&Utc).date_naive() == today {
            count = count.saturating_add(1);
        }
    }
    Ok((count, count > 0))
}

/// Insert one scheduled notification delivery into the existing history table.
pub fn insert_delivery(conn: &Connection, delivery: &NotificationDelivery) -> Result<(), AppError> {
    let (target_type, target_id) = delivery
        .target
        .as_ref()
        .map(|target| {
            (
                Some(target.entity_type.as_str()),
                Some(target.entity_id.as_str()),
            )
        })
        .unwrap_or((None, None));
    let scheduled_at = delivery
        .scheduled_at
        .to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
    conn.execute(
        "INSERT INTO notification_deliveries (
            id, workspace_id, delivery_kind, reminder_rule_id,
            target_entity_type, target_entity_id,
            scheduled_at, status, channel, safe_message, created_at, cycle_key
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
        params![
            delivery.id,
            delivery.workspace_id,
            delivery.delivery_kind.as_str(),
            delivery.reminder_rule_id,
            target_type,
            target_id,
            scheduled_at,
            delivery.status.as_str(),
            delivery.channel.as_str(),
            delivery.message,
            delivery.created_at,
            delivery.cycle_key,
        ],
    )?;
    Ok(())
}

/// Return true only when this call newly claimed the durable event key.
///
/// Block start/boundary/overrun events are deduplicated outside visible
/// history so a claimed event never replays after poll repeats, app
/// restarts, history clearing, or pruning.
pub fn claim_event_key(
    conn: &Connection,
    workspace_id: &str,
    event_key: &str,
    now: DateTime<Utc>,
) -> Result<bool, AppError> {
    conn.execute(
        "INSERT OR IGNORE INTO notification_delivery_dedupe
            (workspace_id, event_key, created_at)
         VALUES (?1, ?2, ?3)",
        params![
            workspace_id,
            event_key,
            now.to_rfc3339_opts(chrono::SecondsFormat::Secs, true)
        ],
    )?;
    Ok(conn.changes() == 1)
}

/// Insert one scheduled block event delivery. The row carries the `block`
/// delivery kind, no reminder rule, no cycle key, and the block target.
pub fn insert_block_event_delivery(
    conn: &Connection,
    workspace_id: &str,
    block_id: &str,
    scheduled_at: DateTime<Utc>,
    message: &str,
    created_at: DateTime<Utc>,
) -> Result<NotificationDelivery, AppError> {
    let delivery = NotificationDelivery {
        id: crate::id::new_uuid(),
        workspace_id: workspace_id.to_string(),
        delivery_kind: NotificationDeliveryKind::Block,
        reminder_rule_id: None,
        target: Some(NotificationTarget {
            entity_type: NotificationTargetType::Block,
            entity_id: block_id.to_string(),
        }),
        scheduled_at,
        delivered_at: None,
        status: NotificationDeliveryStatus::Scheduled,
        channel: NotificationChannel::Desktop,
        message: message.to_string(),
        error_message: None,
        created_at: created_at.to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
        cycle_key: None,
    };
    insert_delivery(conn, &delivery)?;
    Ok(delivery)
}

/// Terminate still-pending block event deliveries for a block. Fixed technical
/// reason only; never user content.
pub fn fail_pending_block_event_deliveries(
    conn: &Connection,
    block_id: &str,
    reason: &str,
) -> Result<u64, AppError> {
    let updated = conn.execute(
        "UPDATE notification_deliveries
         SET status = 'failed', error_message = ?2
         WHERE delivery_kind = 'block' AND target_entity_id = ?1
           AND status = 'scheduled'",
        params![block_id, reason],
    )?;
    Ok(updated as u64)
}

/// Load one delivery-history row by id.
pub fn get_delivery(conn: &Connection, id: &str) -> Result<NotificationDelivery, AppError> {
    let sql = format!("SELECT {DELIVERY_COLS} FROM notification_deliveries WHERE id = ?1");
    let mut stmt = conn.prepare(&sql)?;
    let row = stmt
        .query_map([id], read_delivery_row)?
        .next()
        .transpose()?;
    drop(stmt);
    let row = row.ok_or_else(|| AppError::not_found("The notification was not found."))?;
    build_delivery(row)
}

/// List delivery history, always scoped to one workspace through the owning
/// reminder rule. Optional filters refine the result; rows come back newest
/// first, capped by `filters.limit` (validated at the command boundary).
pub fn list_deliveries(
    conn: &Connection,
    filters: &NotificationDeliveryFilters,
) -> Result<Vec<NotificationDelivery>, AppError> {
    let from = filters.from.map(|date| format!("{date}T00:00:00Z"));
    let to = filters.to.map(|date| format!("{date}T23:59:59Z"));
    let status = filters.status.map(|status| status.as_str());
    let mut stmt = conn.prepare(
        "SELECT d.id, d.workspace_id, d.delivery_kind, d.reminder_rule_id,
                d.target_entity_type, d.target_entity_id, d.scheduled_at,
                d.delivered_at, d.status, d.channel, d.safe_message,
                d.error_message, d.created_at, d.cycle_key
         FROM notification_deliveries d
         LEFT JOIN reminder_rules r ON r.id = d.reminder_rule_id
         WHERE d.workspace_id = ?1
           AND (?2 IS NULL OR d.reminder_rule_id = ?2)
           AND (?3 IS NULL OR d.delivery_kind = ?3)
           AND (?4 IS NULL OR d.status = ?4)
           AND (?5 IS NULL OR d.scheduled_at >= ?5)
           AND (?6 IS NULL OR d.scheduled_at <= ?6)
         ORDER BY d.scheduled_at DESC, d.id DESC
         LIMIT ?7",
    )?;
    let rows = stmt.query_map(
        params![
            filters.workspace_id,
            filters.reminder_rule_id,
            filters.delivery_kind.map(NotificationDeliveryKind::as_str),
            status,
            from,
            to,
            filters.limit
        ],
        read_delivery_row,
    )?;
    let rows: Vec<DeliveryRow> = rows.collect::<Result<_, _>>()?;
    drop(stmt);
    rows.into_iter().map(build_delivery).collect()
}

/// Load the most recent backup delivery for a workspace by scheduled slot.
/// Used by the backup-reminder dispatcher to derive the next cycle anchor.
pub fn latest_backup_delivery(
    conn: &Connection,
    workspace_id: &str,
) -> Result<Option<NotificationDelivery>, AppError> {
    let sql = format!(
        "SELECT {DELIVERY_COLS} FROM notification_deliveries
         WHERE workspace_id = ?1 AND delivery_kind = 'backup'
         ORDER BY scheduled_at DESC, id DESC LIMIT 1"
    );
    let mut stmt = conn.prepare(&sql)?;
    let row = stmt
        .query_map([workspace_id], read_delivery_row)?
        .next()
        .transpose()?;
    drop(stmt);
    row.map(build_delivery).transpose()
}

/// Check whether a backup delivery already exists for the given cycle key.
/// A pending, delivered, dismissed, or failed row all satisfy the dedupe.
pub fn backup_cycle_exists(
    conn: &Connection,
    workspace_id: &str,
    cycle_key: &str,
) -> Result<bool, AppError> {
    let exists: bool = conn.query_row(
        "SELECT EXISTS(
            SELECT 1 FROM notification_deliveries
            WHERE workspace_id = ?1 AND delivery_kind = 'backup' AND cycle_key = ?2
         )",
        params![workspace_id, cycle_key],
        |row| row.get(0),
    )?;
    Ok(exists)
}

/// Maximum age a terminal history row may reach before the bounded retention
/// cleanup removes it. The cutoff is strict: `history_at < now - 7*24h`.
pub const HISTORY_RETENTION_DAYS: i64 = 7;

/// Return the most recent recorded backup cycle slot recovered from the
/// durable `notification_delivery_dedupe` table.
///
/// Retention pruning and history clearing may remove terminal `backup`
/// delivery rows; the durable `backup:<cycle_key>` rows (seeded by migration
/// 041 and claimed by the dispatcher with every new delivery) survive
/// cleanup, so the cycle anchor cannot regress into re-delivering a due
/// cycle early after history was cleared or pruned.
pub fn latest_backup_cycle_key_slot(
    conn: &Connection,
    workspace_id: &str,
) -> Result<Option<DateTime<Utc>>, AppError> {
    let key: Option<String> = conn
        .query_row(
            "SELECT event_key FROM notification_delivery_dedupe
             WHERE workspace_id = ?1 AND event_key LIKE 'backup:%'
             ORDER BY event_key DESC LIMIT 1",
            [workspace_id],
            |row| row.get(0),
        )
        .optional()?;
    let slot = key
        .as_deref()
        .and_then(|event_key| event_key.strip_prefix("backup:"))
        .and_then(|slot| DateTime::parse_from_rfc3339(slot).ok())
        .map(|timestamp| timestamp.with_timezone(&Utc));
    Ok(slot)
}

/// Clear one workspace's terminal in-app notification history.
///
/// Removes only terminal (`delivered | dismissed | failed`) rows whose
/// `history_at` (`delivered_at`, otherwise `scheduled_at`, otherwise
/// `created_at`) is at or before the clear timestamp. Scheduled and snoozed
/// rows are preserved — including past-due scheduled rows still needed by due
/// processing and future snoozes — so pending delivery processing keeps
/// working. The durable `notification_delivery_dedupe` table is never touched,
/// and source entities, rules, and future scheduled deliveries are unaffected.
/// Returns the number of removed rows.
pub fn clear_workspace_history(
    conn: &Connection,
    workspace_id: &str,
    now: DateTime<Utc>,
) -> Result<u64, AppError> {
    let clear_at = now.to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
    let updated = conn.execute(
        "DELETE FROM notification_deliveries
         WHERE workspace_id = ?1
           AND status IN ('delivered','dismissed','failed')
           AND COALESCE(delivered_at, scheduled_at, created_at) <= ?2",
        params![workspace_id, clear_at],
    )?;
    Ok(updated as u64)
}

/// Remove terminal history rows older than the seven-day retention maximum.
///
/// Bounded lifecycle cleanup, invoked beside `expire_stale_deliveries` in the
/// existing startup/notification poll; there is no background worker. The
/// cutoff is strict (`history_at < now - 7*24h`), spans all workspaces, and
/// never touches scheduled or snoozed rows or the durable dedupe table.
/// Returns the number of removed rows.
pub fn prune_expired_history(conn: &Connection, now: DateTime<Utc>) -> Result<u64, AppError> {
    let cutoff = (now - chrono::Duration::hours(24 * HISTORY_RETENTION_DAYS))
        .to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
    let updated = conn.execute(
        "DELETE FROM notification_deliveries
         WHERE status IN ('delivered','dismissed','failed')
           AND COALESCE(delivered_at, scheduled_at, created_at) < ?1",
        params![cutoff],
    )?;
    Ok(updated as u64)
}

/// Move a delivery to a terminal-of-day status, guarded so only a still
/// `scheduled` row is rewritten. Returns not-found when the row is missing or
/// another action already changed it, preventing duplicate history rewrites.
fn mark_delivery_status(
    conn: &Connection,
    id: &str,
    status: NotificationDeliveryStatus,
) -> Result<(), AppError> {
    let updated = conn.execute(
        "UPDATE notification_deliveries SET status = ?1
         WHERE id = ?2 AND status = 'scheduled'",
        params![status.as_str(), id],
    )?;
    if updated == 0 {
        return Err(AppError::not_found(
            "The scheduled notification is no longer available.",
        ));
    }
    Ok(())
}

/// Mark a scheduled delivery dismissed. The row stays in history.
pub fn mark_delivery_dismissed(conn: &Connection, id: &str) -> Result<(), AppError> {
    mark_delivery_status(conn, id, NotificationDeliveryStatus::Dismissed)
}

/// Mark a scheduled delivery snoozed. The row stays in history as the record
/// of the original schedule while the snooze command inserts the successor.
pub fn mark_delivery_snoozed(conn: &Connection, id: &str) -> Result<(), AppError> {
    mark_delivery_status(conn, id, NotificationDeliveryStatus::Snoozed)
}

/// Load desktop notifications that are due today and still scheduled,
/// including target metadata for deep-linking.
///
/// Older scheduled deliveries are expired by `expire_stale_deliveries`
/// rather than replayed after the application has been closed, preventing
/// a backlog from becoming a notification storm.
pub fn list_due_deliveries(
    conn: &Connection,
    now: DateTime<Utc>,
) -> Result<Vec<DueNotification>, AppError> {
    let start_of_day = now
        .date_naive()
        .and_hms_opt(0, 0, 0)
        .expect("midnight is a valid time")
        .and_utc()
        .to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
    let due_at = now.to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
    let mut stmt = conn.prepare(
        "SELECT d.id, d.safe_message, COALESCE(r.type, d.delivery_kind),
                d.target_entity_type, d.target_entity_id
         FROM notification_deliveries d
         LEFT JOIN reminder_rules r ON r.id = d.reminder_rule_id
         WHERE d.status = 'scheduled'
           AND d.channel IN ('desktop', 'both')
           AND d.scheduled_at >= ?1
           AND d.scheduled_at <= ?2
         ORDER BY d.scheduled_at, d.id",
    )?;
    let rows = stmt.query_map([start_of_day, due_at], |row| {
        Ok(DueNotification {
            id: row.get(0)?,
            safe_message: row.get(1)?,
            reminder_type: row.get(2)?,
            target_entity_type: row.get(3)?,
            target_entity_id: row.get(4)?,
        })
    })?;
    rows.collect::<Result<Vec<_>, _>>().map_err(AppError::from)
}

/// Mark notifications scheduled before today as failed/expired. They are not
/// silently moved to the current day and never affect task status.
pub fn expire_stale_deliveries(conn: &Connection, now: DateTime<Utc>) -> Result<u64, AppError> {
    let start_of_day = now
        .date_naive()
        .and_hms_opt(0, 0, 0)
        .expect("midnight is a valid time")
        .and_utc()
        .to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
    let updated = conn.execute(
        "UPDATE notification_deliveries
         SET status = 'failed', error_message = 'The scheduled notification expired before delivery.'
         WHERE status = 'scheduled' AND scheduled_at < ?1",
        [start_of_day],
    )?;
    Ok(updated as u64)
}

/// Check if a safe message is due for delivery today.
///
/// Returns true if there is a scheduled delivery with the given safe_message
/// that has not yet been delivered/dismissed/snoozed today.
pub fn is_due_safe_message(
    conn: &Connection,
    message: &str,
    now: DateTime<Utc>,
) -> Result<bool, AppError> {
    let start_of_day = now
        .date_naive()
        .and_hms_opt(0, 0, 0)
        .expect("midnight is a valid time")
        .and_utc()
        .to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
    let due_at = now.to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
    let exists: bool = conn.query_row(
        "SELECT EXISTS(
            SELECT 1 FROM notification_deliveries
            WHERE status = 'scheduled'
              AND channel IN ('desktop', 'both')
              AND safe_message = ?1
              AND scheduled_at >= ?2
              AND scheduled_at <= ?3
         )",
        params![message, start_of_day, due_at],
        |row| row.get(0),
    )?;
    Ok(exists)
}

/// Record one delivery outcome. The update is conditional on `scheduled`, so
/// duplicate acknowledgements cannot rewrite delivery history.
pub fn record_delivery_result(
    conn: &Connection,
    id: &str,
    delivered_at: Option<DateTime<Utc>>,
    now: DateTime<Utc>,
) -> Result<(), AppError> {
    let start_of_day = now
        .date_naive()
        .and_hms_opt(0, 0, 0)
        .expect("midnight is a valid time")
        .and_utc()
        .to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
    let due_at = now.to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
    let (status, timestamp, error_message) = match delivered_at {
        Some(at) => (
            "delivered",
            Some(at.to_rfc3339_opts(chrono::SecondsFormat::Secs, true)),
            None,
        ),
        None => (
            "failed",
            None,
            Some("The local notification could not be delivered."),
        ),
    };
    let updated = conn.execute(
        "UPDATE notification_deliveries
         SET status = ?1, delivered_at = ?2, error_message = ?3
         WHERE id = ?4 AND status = 'scheduled'
           AND scheduled_at >= ?5 AND scheduled_at <= ?6",
        params![status, timestamp, error_message, id, start_of_day, due_at],
    )?;
    if updated == 0 {
        return Err(AppError::not_found(
            "The scheduled notification is no longer available.",
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use chrono::{NaiveDate, NaiveTime, TimeZone};

    use super::*;
    use crate::test_utils::test_conn;

    fn insert_delivery_fixture(conn: &Connection, id: &str, scheduled_at: &str) {
        let reminder_type = if id == "old" { "planning" } else { "focus" };
        conn.execute(
            "INSERT INTO reminder_rules (
                id, workspace_id, type, privacy_safe_message, created_at, updated_at
             ) VALUES (?1, 'w1', ?2, 'A safe reminder', ?3, ?3)",
            params![format!("rule-{id}"), reminder_type, scheduled_at],
        )
        .expect("insert reminder rule");
        conn.execute(
            "INSERT INTO notification_deliveries (
                id, workspace_id, delivery_kind, reminder_rule_id,
                scheduled_at, channel, safe_message, created_at
             ) VALUES (?1, 'w1', 'reminder_rule', ?2, ?3, 'desktop', 'A safe reminder', ?3)",
            params![id, format!("rule-{id}"), scheduled_at],
        )
        .expect("insert notification delivery");
    }

    #[test]
    fn due_query_expires_previous_days_and_records_only_delivery_status() {
        let conn = test_conn();
        insert_delivery_fixture(&conn, "old", "2026-10-04T09:00:00Z");
        insert_delivery_fixture(&conn, "due", "2026-10-05T09:00:00Z");
        let now = Utc
            .with_ymd_and_hms(2026, 10, 5, 10, 0, 0)
            .single()
            .expect("valid test timestamp");

        assert_eq!(expire_stale_deliveries(&conn, now).expect("expire old"), 1);
        let due = list_due_deliveries(&conn, now).expect("load due deliveries");
        assert_eq!(due.len(), 1);
        assert_eq!(due[0].id, "due");
        assert_eq!(due[0].safe_message, "A safe reminder");

        record_delivery_result(&conn, "due", Some(now), now).expect("record delivery");
        assert!(list_due_deliveries(&conn, now)
            .expect("no longer due")
            .is_empty());
        let statuses: Vec<(String, String)> = conn
            .prepare("SELECT id, status FROM notification_deliveries ORDER BY id")
            .expect("prepare status query")
            .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
            .expect("query statuses")
            .collect::<Result<_, _>>()
            .expect("collect statuses");
        assert_eq!(
            statuses,
            vec![
                ("due".into(), "delivered".into()),
                ("old".into(), "failed".into())
            ]
        );
    }

    #[test]
    fn safe_message_validation_requires_a_due_scheduled_desktop_delivery() {
        let conn = test_conn();
        insert_delivery_fixture(&conn, "later", "2026-10-05T11:00:00Z");
        let now = Utc
            .with_ymd_and_hms(2026, 10, 5, 10, 0, 0)
            .single()
            .expect("valid test timestamp");

        assert!(!is_due_safe_message(&conn, "A safe reminder", now).expect("future message"));
        assert!(!is_due_safe_message(&conn, "arbitrary text", now).expect("unknown message"));
    }

    fn insert_rule(conn: &Connection, id: &str, workspace_id: &str, rule_type: &str, enabled: i64) {
        conn.execute(
            "INSERT OR IGNORE INTO workspaces (id, name, created_at, updated_at)
             VALUES (?1, 'Other', '2026-10-05T00:00:00Z', '2026-10-05T00:00:00Z')",
            [workspace_id],
        )
        .expect("seed workspace for rule");
        conn.execute(
            "INSERT INTO reminder_rules (
                id, workspace_id, type, enabled, snooze_minutes_json,
                privacy_safe_message, created_at, updated_at
             ) VALUES (?1, ?2, ?3, ?4, '[15,30]', 'A safe reminder',
                       '2026-10-05T00:00:00Z', '2026-10-05T00:00:00Z')",
            params![id, workspace_id, rule_type, enabled],
        )
        .expect("insert rule");
    }

    #[test]
    fn list_rules_for_workspace_is_scoped_and_includes_disabled_rules() {
        let conn = test_conn();
        insert_rule(&conn, "r1", "w1", "planning", 1);
        insert_rule(&conn, "r2", "w1", "focus", 0);
        insert_rule(&conn, "r3", "w2", "review", 1);
        let now = Utc
            .with_ymd_and_hms(2026, 10, 5, 10, 0, 0)
            .single()
            .expect("valid test timestamp");

        let rules = list_rules_for_workspace(&conn, "w1", now).expect("list rules");
        let ids: Vec<&str> = rules.iter().map(|rule| rule.id.as_str()).collect();
        assert_eq!(ids, vec!["r2", "r1"]);
        assert_eq!(rules[0].snooze_minutes, vec![15, 30]);
        assert_eq!(rules[0].escalation, ReminderEscalation::None);
        assert!(!rules[0].enabled, "disabled rules are listed for settings");

        let enabled = list_enabled_rules(&conn, now).expect("list enabled");
        assert_eq!(
            enabled.len(),
            2,
            "enabled listing spans workspaces and skips disabled"
        );
    }

    #[test]
    fn update_rule_persists_validated_config_and_rejects_unknown_ids() {
        let conn = test_conn();
        insert_rule(&conn, "r1", "w1", "planning", 1);
        let now = Utc
            .with_ymd_and_hms(2026, 10, 5, 10, 0, 0)
            .single()
            .expect("valid test timestamp");
        let config = ReminderRuleConfig {
            enabled: false,
            preferred_time: Some("09:30".to_string()),
            time_window_start: None,
            time_window_end: None,
            quiet_hours_start: Some("23:00".to_string()),
            quiet_hours_end: Some("06:30".to_string()),
            deadline_notification_window_days: 6,
            max_reminders_per_day: 3,
            snooze_minutes: vec![5, 45],
            channel: NotificationChannel::Desktop,
            escalation: ReminderEscalation::Gentle,
            privacy_safe_message: "  A planned pause helps  ".to_string(),
        };

        let updated = update_rule(&conn, "r1", &config, now).expect("update rule");
        assert!(!updated.enabled);
        assert_eq!(updated.max_reminders_per_day, 3);
        assert_eq!(updated.snooze_minutes, vec![5, 45]);
        assert_eq!(updated.escalation, ReminderEscalation::Gentle);
        assert_eq!(updated.deadline_notification_window_days, 6);
        assert_eq!(updated.privacy_safe_message, "A planned pause helps");
        assert_eq!(
            updated.preferred_time,
            Some(NaiveTime::from_hms_opt(9, 30, 0).expect("valid time"))
        );

        assert!(update_rule(&conn, "missing", &config, now).is_err());
        let mut invalid = config.clone();
        invalid.quiet_hours_start = Some("25:00".to_string());
        assert!(update_rule(&conn, "r1", &invalid, now).is_err());
        let mut inverted = config.clone();
        inverted.time_window_start = Some("10:00".to_string());
        inverted.time_window_end = Some("10:00".to_string());
        assert!(update_rule(&conn, "r1", &inverted, now).is_err());
    }

    #[test]
    fn snooze_options_and_rule_lookup_reject_unknown_ids() {
        let conn = test_conn();
        insert_rule(&conn, "r1", "w1", "planning", 1);

        assert_eq!(
            snooze_options_for_rule(&conn, "r1").expect("options"),
            vec![15, 30]
        );
        assert!(snooze_options_for_rule(&conn, "missing").is_err());

        let now = Utc
            .with_ymd_and_hms(2026, 10, 5, 10, 0, 0)
            .single()
            .expect("valid test timestamp");
        assert!(get_rule(&conn, "missing", now).is_err());
    }

    #[test]
    fn list_deliveries_filters_by_workspace_status_rule_and_limit() {
        let conn = test_conn();
        insert_rule(&conn, "r1", "w1", "planning", 1);
        insert_rule(&conn, "r2", "w2", "focus", 1);
        for (id, rule, workspace_id, day, status) in [
            ("d1", "r1", "w1", "2026-10-04", "delivered"),
            ("d2", "r1", "w1", "2026-10-05", "scheduled"),
            ("d3", "r1", "w1", "2026-10-05", "dismissed"),
            ("d4", "r2", "w2", "2026-10-05", "scheduled"),
        ] {
            conn.execute(
                "INSERT INTO notification_deliveries (
                    id, workspace_id, delivery_kind, reminder_rule_id,
                    scheduled_at, status, channel,
                    safe_message, created_at
                 ) VALUES (?1, ?2, 'reminder_rule', ?3, ?4, ?5,
                           'desktop', 'A safe reminder', ?4)",
                params![id, workspace_id, rule, format!("{day}T09:00:00Z"), status],
            )
            .expect("insert delivery");
        }

        let all = list_deliveries(
            &conn,
            &NotificationDeliveryFilters {
                workspace_id: "w1".to_string(),
                reminder_rule_id: None,
                delivery_kind: None,
                status: None,
                from: None,
                to: None,
                limit: 100,
            },
        )
        .expect("list w1 history");
        let ids: Vec<&str> = all.iter().map(|delivery| delivery.id.as_str()).collect();
        assert_eq!(
            ids,
            vec!["d3", "d2", "d1"],
            "workspace-scoped, newest first, other workspaces excluded"
        );

        let scheduled = list_deliveries(
            &conn,
            &NotificationDeliveryFilters {
                workspace_id: "w1".to_string(),
                reminder_rule_id: None,
                delivery_kind: None,
                status: Some(NotificationDeliveryStatus::Scheduled),
                from: Some(NaiveDate::from_ymd_opt(2026, 10, 5).expect("valid date")),
                to: None,
                limit: 100,
            },
        )
        .expect("list scheduled");
        assert_eq!(scheduled.len(), 1);
        assert_eq!(scheduled[0].id, "d2");

        let one = list_deliveries(
            &conn,
            &NotificationDeliveryFilters {
                workspace_id: "w1".to_string(),
                reminder_rule_id: Some("r1".to_string()),
                delivery_kind: Some(NotificationDeliveryKind::ReminderRule),
                status: None,
                from: None,
                to: Some(NaiveDate::from_ymd_opt(2026, 10, 4).expect("valid date")),
                limit: 1,
            },
        )
        .expect("list limited");
        assert_eq!(one.len(), 1);
        assert_eq!(one[0].id, "d1");
    }

    #[test]
    fn delivery_lookup_and_status_guards_keep_history_consistent() {
        let conn = test_conn();
        insert_rule(&conn, "r1", "w1", "planning", 1);
        conn.execute(
            "INSERT INTO notification_deliveries (
                id, workspace_id, delivery_kind, reminder_rule_id,
                scheduled_at, status, channel,
                safe_message, created_at
             ) VALUES ('d1', 'w1', 'reminder_rule', 'r1', '2026-10-05T09:00:00Z',
                       'delivered', 'desktop', 'A safe reminder', '2026-10-05T08:00:00Z')",
            [],
        )
        .expect("insert delivered row");

        let delivery = get_delivery(&conn, "d1").expect("get delivery");
        assert_eq!(delivery.status, NotificationDeliveryStatus::Delivered);
        assert!(delivery.delivered_at.is_none());
        assert!(get_delivery(&conn, "missing").is_err());

        assert!(
            mark_delivery_dismissed(&conn, "d1").is_err(),
            "only scheduled rows move"
        );
        let status: String = conn
            .query_row(
                "SELECT status FROM notification_deliveries WHERE id = 'd1'",
                [],
                |row| row.get(0),
            )
            .expect("read status");
        assert_eq!(status, "delivered", "guarded update rewrote nothing");
    }

    #[test]
    fn workspace_history_and_due_queries_include_ruleless_backup_deliveries() {
        let conn = test_conn();
        conn.execute(
            "INSERT INTO notification_deliveries (
                 id, workspace_id, delivery_kind, scheduled_at, status, channel,
                 safe_message, created_at
             ) VALUES ('backup-1', 'w1', 'backup', '2026-10-05T09:00:00Z',
                       'scheduled', 'desktop', 'A generic safe reminder',
                       '2026-10-05T08:00:00Z')",
            [],
        )
        .expect("insert ruleless backup delivery");
        let filters = NotificationDeliveryFilters {
            workspace_id: "w1".to_string(),
            reminder_rule_id: None,
            delivery_kind: Some(NotificationDeliveryKind::Backup),
            status: None,
            from: None,
            to: None,
            limit: 10,
        };
        let history = list_deliveries(&conn, &filters).expect("list backup history");
        assert_eq!(history.len(), 1);
        assert_eq!(history[0].workspace_id, "w1");
        assert_eq!(history[0].delivery_kind, NotificationDeliveryKind::Backup);
        assert_eq!(history[0].reminder_rule_id, None);

        let now = Utc
            .with_ymd_and_hms(2026, 10, 5, 10, 0, 0)
            .single()
            .expect("valid timestamp");
        let due = list_due_deliveries(&conn, now).expect("list ruleless due delivery");
        assert_eq!(due.len(), 1);
        assert_eq!(due[0].reminder_type, "backup");
        assert_eq!(due[0].target_entity_type, None);
        assert_eq!(due[0].target_entity_id, None);
    }

    fn empty_profile_config(enabled: bool) -> ReminderRuleConfig {
        ReminderRuleConfig {
            enabled,
            preferred_time: None,
            time_window_start: None,
            time_window_end: None,
            quiet_hours_start: None,
            quiet_hours_end: None,
            deadline_notification_window_days: 3,
            max_reminders_per_day: 5,
            snooze_minutes: vec![10, 30, 60],
            channel: NotificationChannel::Desktop,
            escalation: ReminderEscalation::None,
            privacy_safe_message: "A safe reminder".to_string(),
        }
    }

    #[test]
    fn enabling_an_unconfigured_rule_applies_default_profile_transactionally() {
        let conn = test_conn();
        insert_rule(&conn, "r1", "w1", "planning", 0);
        let now = Utc
            .with_ymd_and_hms(2026, 10, 5, 10, 0, 0)
            .single()
            .expect("valid test timestamp");

        let enabled = update_rule(&conn, "r1", &empty_profile_config(true), now)
            .expect("enable rule with no profile times");

        assert!(enabled.enabled);
        assert_eq!(
            enabled.preferred_time,
            Some(NaiveTime::from_hms_opt(9, 0, 0).expect("default preferred time"))
        );
        assert_eq!(
            enabled.quiet_hours_start,
            Some(NaiveTime::from_hms_opt(22, 0, 0).expect("default quiet start"))
        );
        assert_eq!(
            enabled.quiet_hours_end,
            Some(NaiveTime::from_hms_opt(7, 0, 0).expect("default quiet end"))
        );

        let stored: (String, String, String) = conn
            .query_row(
                "SELECT preferred_time, quiet_hours_start, quiet_hours_end
                 FROM reminder_rules WHERE id = 'r1'",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .expect("read persisted default profile");
        assert_eq!(stored, ("09:00".into(), "22:00".into(), "07:00".into()));
    }

    #[test]
    fn disabling_and_reenabling_preserves_user_profile_values() {
        let conn = test_conn();
        insert_rule(&conn, "r1", "w1", "planning", 1);
        conn.execute(
            "UPDATE reminder_rules
             SET preferred_time = '08:30', quiet_hours_start = '23:15',
                 quiet_hours_end = '06:15'
             WHERE id = 'r1'",
            [],
        )
        .expect("set user-configured profile");
        let now = Utc
            .with_ymd_and_hms(2026, 10, 5, 10, 0, 0)
            .single()
            .expect("valid test timestamp");

        let disabled = update_rule(&conn, "r1", &empty_profile_config(false), now)
            .expect("disable reminder rule");
        assert!(!disabled.enabled);
        assert_eq!(
            disabled.preferred_time,
            Some(NaiveTime::from_hms_opt(8, 30, 0).expect("custom preferred time"))
        );
        assert_eq!(
            disabled.quiet_hours_start,
            Some(NaiveTime::from_hms_opt(23, 15, 0).expect("custom quiet start"))
        );
        assert_eq!(
            disabled.quiet_hours_end,
            Some(NaiveTime::from_hms_opt(6, 15, 0).expect("custom quiet end"))
        );

        let reenabled = update_rule(&conn, "r1", &empty_profile_config(true), now)
            .expect("re-enable reminder rule");
        assert!(reenabled.enabled);
        assert_eq!(reenabled.preferred_time, disabled.preferred_time);
        assert_eq!(reenabled.quiet_hours_start, disabled.quiet_hours_start);
        assert_eq!(reenabled.quiet_hours_end, disabled.quiet_hours_end);
    }

    // --- History clearing and seven-day retention (Phase 8.7.4) ---

    /// Ruleless synthetic delivery row; backup/block kinds need no rule or
    /// target and their CHECK constraints are satisfied by omission.
    fn seed_notification(
        conn: &Connection,
        id: &str,
        workspace_id: &str,
        delivery_kind: &str,
        status: &str,
        scheduled_at: &str,
        delivered_at: Option<&str>,
        cycle_key: Option<&str>,
    ) {
        conn.execute(
            "INSERT INTO notification_deliveries (
                id, workspace_id, delivery_kind, scheduled_at, delivered_at,
                status, channel, safe_message, created_at, cycle_key
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, 'desktop', 'A safe reminder', ?4, ?7)",
            params![
                id,
                workspace_id,
                delivery_kind,
                scheduled_at,
                delivered_at,
                status,
                cycle_key
            ],
        )
        .expect("insert notification delivery");
    }

    fn seed_other_workspace(conn: &Connection, workspace_id: &str) {
        conn.execute(
            "INSERT OR IGNORE INTO workspaces (id, name, created_at, updated_at)
             VALUES (?1, 'Other', '2026-10-21T00:00:00Z', '2026-10-21T00:00:00Z')",
            [workspace_id],
        )
        .expect("seed workspace");
    }

    fn remaining_delivery_ids(conn: &Connection) -> Vec<String> {
        conn.prepare("SELECT id FROM notification_deliveries ORDER BY id")
            .expect("prepare remaining ids query")
            .query_map([], |row| row.get(0))
            .expect("query remaining ids")
            .collect::<Result<_, _>>()
            .expect("collect remaining ids")
    }

    fn retention_test_now() -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 10, 21, 12, 0, 0)
            .single()
            .expect("valid test timestamp")
    }

    #[test]
    fn retention_prunes_only_terminal_rows_older_than_the_strict_boundary() {
        let conn = test_conn();
        let now = retention_test_now();

        // Terminal row exactly at the cutoff: kept (the cutoff is strict `<`).
        seed_notification(
            &conn,
            "kept-boundary",
            "w1",
            "backup",
            "delivered",
            "2026-10-14T12:00:00Z",
            Some("2026-10-14T12:00:00Z"),
            Some("k1"),
        );
        // One second older: removed.
        seed_notification(
            &conn,
            "pruned-oldest",
            "w1",
            "backup",
            "delivered",
            "2026-10-14T11:59:59Z",
            Some("2026-10-14T11:59:59Z"),
            Some("k2"),
        );
        // Failed row whose history_at falls back to scheduled_at: removed.
        seed_notification(
            &conn,
            "pruned-failed",
            "w1",
            "backup",
            "failed",
            "2026-10-05T09:00:00Z",
            None,
            Some("k3"),
        );
        // A past-due still-scheduled row is never deleted, even far past the
        // boundary, because due processing may still need it.
        seed_notification(
            &conn,
            "kept-scheduled",
            "w1",
            "backup",
            "scheduled",
            "2026-10-01T09:00:00Z",
            None,
            Some("k4"),
        );
        // Snoozed rows are never hidden or deleted (retention or clearing).
        seed_notification(
            &conn,
            "kept-snoozed",
            "w1",
            "backup",
            "snoozed",
            "2026-10-01T09:00:00Z",
            None,
            Some("k5"),
        );
        // history_at prefers delivered_at: an old scheduled_at with a recent
        // delivered_at stays within retention.
        seed_notification(
            &conn,
            "kept-recent-delivery",
            "w1",
            "backup",
            "delivered",
            "2026-10-05T09:00:00Z",
            Some("2026-10-19T12:00:00Z"),
            Some("k6"),
        );

        let pruned = prune_expired_history(&conn, now).expect("run retention cleanup");
        assert_eq!(pruned, 2, "only terminal rows past the strict cutoff go");
        assert_eq!(
            remaining_delivery_ids(&conn),
            vec![
                "kept-boundary".to_string(),
                "kept-recent-delivery".to_string(),
                "kept-scheduled".to_string(),
                "kept-snoozed".to_string(),
            ]
        );

        // Retention cleanup is idempotent.
        let pruned_again = prune_expired_history(&conn, now).expect("run cleanup again");
        assert_eq!(pruned_again, 0);
    }

    #[test]
    fn clear_history_is_workspace_scoped_and_preserves_pending_rows() {
        let conn = test_conn();
        seed_other_workspace(&conn, "w2");
        seed_notification(
            &conn,
            "c1-done",
            "w1",
            "backup",
            "delivered",
            "2026-10-01T09:00:00Z",
            Some("2026-10-01T09:30:00Z"),
            Some("k1"),
        );
        seed_notification(
            &conn,
            "c2-dismissed",
            "w1",
            "backup",
            "dismissed",
            "2026-10-02T09:00:00Z",
            None,
            Some("k2"),
        );
        seed_notification(
            &conn,
            "c3-failed",
            "w1",
            "backup",
            "failed",
            "2026-10-03T09:00:00Z",
            None,
            Some("k3"),
        );
        // Pending rows survive clearing: a future scheduled delivery and its
        // snoozed predecessor.
        seed_notification(
            &conn,
            "c4-pending",
            "w1",
            "backup",
            "scheduled",
            "2026-10-21T12:30:00Z",
            None,
            Some("k4"),
        );
        seed_notification(
            &conn,
            "c5-snoozed",
            "w1",
            "backup",
            "snoozed",
            "2026-10-04T09:00:00Z",
            None,
            Some("k5"),
        );
        // Another workspace's rows are never touched by clearing w1.
        seed_notification(
            &conn,
            "w2-done",
            "w2",
            "backup",
            "delivered",
            "2026-10-01T09:00:00Z",
            Some("2026-10-01T09:30:00Z"),
            Some("k1"),
        );
        seed_notification(
            &conn,
            "w2-pending",
            "w2",
            "backup",
            "scheduled",
            "2026-10-05T09:00:00Z",
            None,
            Some("k2"),
        );

        let now = retention_test_now();
        let cleared = clear_workspace_history(&conn, "w1", now).expect("clear w1 history");
        assert_eq!(cleared, 3, "only terminal w1 rows are cleared");
        assert_eq!(
            remaining_delivery_ids(&conn),
            vec![
                "c4-pending".to_string(),
                "c5-snoozed".to_string(),
                "w2-done".to_string(),
                "w2-pending".to_string(),
            ]
        );

        // Clearing again is a safe no-op within the same clear timestamp.
        let cleared_again = clear_workspace_history(&conn, "w1", now).expect("clear again");
        assert_eq!(cleared_again, 0);
    }

    #[test]
    fn clear_and_prune_preserve_durable_dedupe_keys_and_source_entities() {
        let conn = test_conn();
        let now = retention_test_now();
        let now_text = "2026-10-21T12:00:00Z";

        insert_rule(&conn, "r1", "w1", "planning", 1);
        // Terminal rule-based delivery far outside retention: pruned.
        conn.execute(
            "INSERT INTO notification_deliveries (
                id, workspace_id, delivery_kind, reminder_rule_id,
                scheduled_at, status, channel, safe_message, created_at
             ) VALUES ('rule-old', 'w1', 'reminder_rule', 'r1',
                       '2026-10-01T09:00:00Z', 'delivered', 'desktop',
                       'A safe reminder', '2026-10-01T08:00:00Z')",
            [],
        )
        .expect("insert rule delivery");
        // Pending row survives both cleanup paths (block deliveries carry the
        // required block target).
        conn.execute(
            "INSERT INTO notification_deliveries (
                id, workspace_id, delivery_kind, target_entity_type,
                target_entity_id, scheduled_at, status, channel,
                safe_message, created_at
             ) VALUES ('block-pending', 'w1', 'block', 'block', 'block-1',
                       '2026-10-21T12:30:00Z', 'scheduled', 'desktop',
                       'A safe reminder', '2026-10-21T12:00:00Z')",
            [],
        )
        .expect("insert pending block delivery");
        // A previously claimed event key must never be re-claimable after
        // history was cleared or pruned.
        assert!(
            claim_event_key(&conn, "w1", "block:block-1:start", now).expect("claim event key"),
            "fresh event key is claimable"
        );

        // Source entities that cleanup must never mutate.
        conn.execute(
            "INSERT INTO blocks (id, workspace_id, date, type, created_at, updated_at)
             VALUES ('block-1', 'w1', '2026-10-21', 'focus', ?1, ?1)",
            [now_text],
        )
        .expect("insert source block");
        conn.execute(
            "INSERT INTO projects (id, workspace_id, title, created_at, updated_at)
             VALUES ('project-1', 'w1', 'A project', ?1, ?1)",
            [now_text],
        )
        .expect("insert source project");
        conn.execute(
            "INSERT INTO tasks (id, workspace_id, title, created_at, updated_at)
             VALUES ('task-1', 'w1', 'A task', ?1, ?1)",
            [now_text],
        )
        .expect("insert source task");

        prune_expired_history(&conn, now).expect("run retention cleanup");
        clear_workspace_history(&conn, "w1", now).expect("clear w1 history");

        let durable_rows: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM notification_delivery_dedupe",
                [],
                |row| row.get(0),
            )
            .expect("count durable dedupe keys");
        assert_eq!(
            durable_rows, 1,
            "cleanup never touches the durable dedupe table"
        );
        assert!(
            !claim_event_key(&conn, "w1", "block:block-1:start", now)
                .expect("claim the same event key again"),
            "a claimed event key stays claimed across cleanup"
        );

        // Source entities are untouched.
        let table_counts: Vec<(&str, i64)> = [
            ("workspaces", "SELECT COUNT(*) FROM workspaces"),
            ("reminder_rules", "SELECT COUNT(*) FROM reminder_rules"),
            ("blocks", "SELECT COUNT(*) FROM blocks"),
            ("projects", "SELECT COUNT(*) FROM projects"),
            ("tasks", "SELECT COUNT(*) FROM tasks"),
        ]
        .into_iter()
        .map(|(table, sql)| {
            let count = conn
                .query_row(sql, [], |row| row.get(0))
                .unwrap_or_else(|error| panic!("count {table}: {error}"));
            (table, count)
        })
        .collect();
        for (table, count) in table_counts {
            assert_eq!(count, 1, "source table {table} is unmutated by cleanup");
        }

        let remaining = remaining_delivery_ids(&conn);
        assert_eq!(
            remaining,
            vec!["block-pending".to_string()],
            "pending rows survive retention and clearing"
        );
    }
}
