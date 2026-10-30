//! Thin command boundary for reminder-rule settings (Phase 8.4).
//!
//! Commands only parse identifiers and convert the transport DTO; enum,
//! time, quiet-hour/window, daily-limit, and snooze-option validation lives
//! in the domain and repository layers below this boundary.

use chrono::Utc;
use tauri::State;

use crate::commands::dto::UpdateReminderRuleDto;
use crate::domain::notifications::{
    NotificationChannel, ReminderEscalation, ReminderRule, ReminderRuleConfig,
};
use crate::error::AppError;
use crate::repos::notification_repo;
use crate::state::Db;

/// List every reminder rule configured for the workspace, including disabled
/// rules and current-UTC-day delivery counts so settings can show usage.
#[tauri::command]
pub fn list_reminder_rules(
    db: State<'_, Db>,
    workspace_id: String,
) -> Result<Vec<ReminderRule>, AppError> {
    crate::id::parse_uuid(&workspace_id)?;
    let now = Utc::now();
    crate::log_command!(
        "list_reminder_rules",
        db.with_conn(|conn| notification_repo::list_rules_for_workspace(conn, &workspace_id, now))
    )
}

/// Update one reminder rule's editable configuration. The update is validated
/// and persisted transactionally; unset profile fields preserve existing user
/// values and receive defaults only when an enabled rule still has no value.
/// Rule type, workspace, recurrence, and audit columns are not editable.
#[tauri::command]
pub fn update_reminder_rule(
    db: State<'_, Db>,
    id: String,
    dto: UpdateReminderRuleDto,
) -> Result<ReminderRule, AppError> {
    crate::id::parse_uuid(&id)?;
    let config = reminder_rule_config_from_dto(dto)?;
    let now = Utc::now();
    crate::log_command!(
        "update_reminder_rule",
        db.with_conn(|conn| notification_repo::update_rule(conn, &id, &config, now))
    )
}

/// Convert the transport DTO into the domain config, parsing enum values at
/// the boundary as established by the journal/export commands.
fn reminder_rule_config_from_dto(
    dto: UpdateReminderRuleDto,
) -> Result<ReminderRuleConfig, AppError> {
    Ok(ReminderRuleConfig {
        enabled: dto.enabled,
        preferred_time: dto.preferred_time,
        time_window_start: dto.time_window_start,
        time_window_end: dto.time_window_end,
        quiet_hours_start: dto.quiet_hours_start,
        quiet_hours_end: dto.quiet_hours_end,
        deadline_notification_window_days: dto.deadline_notification_window_days,
        max_reminders_per_day: dto.max_reminders_per_day,
        snooze_minutes: dto.snooze_minutes,
        channel: dto.channel.parse::<NotificationChannel>()?,
        escalation: dto.escalation.parse::<ReminderEscalation>()?,
        privacy_safe_message: dto.privacy_safe_message,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::dto::UpdateReminderRuleDto;

    #[test]
    fn dto_preserves_unset_times_for_authoritative_native_defaulting() {
        let config = reminder_rule_config_from_dto(UpdateReminderRuleDto {
            enabled: true,
            preferred_time: None,
            time_window_start: None,
            time_window_end: None,
            quiet_hours_start: None,
            quiet_hours_end: None,
            deadline_notification_window_days: 3,
            max_reminders_per_day: 5,
            snooze_minutes: vec![10, 30, 60],
            channel: "desktop".to_string(),
            escalation: "none".to_string(),
            privacy_safe_message: "A safe reminder".to_string(),
        })
        .expect("convert valid DTO");

        assert_eq!(config.preferred_time, None);
        assert_eq!(config.quiet_hours_start, None);
        assert_eq!(config.quiet_hours_end, None);
    }
}
