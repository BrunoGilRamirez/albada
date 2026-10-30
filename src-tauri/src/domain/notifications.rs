//! Local reminder eligibility and delivery scheduling.
//!
//! Time-of-day values are interpreted in UTC, matching the current workspace
//! default and the repository's timestamp conventions. `recurrence_rule` is
//! retained as rule data, but is not interpreted here because the project has
//! not defined a recurrence syntax.

use std::collections::BTreeSet;

use chrono::{DateTime, NaiveDate, NaiveTime, Timelike, Utc};
use serde::{Deserialize, Serialize};

use crate::domain::settings::BackupReminderPreference;
use crate::error::AppError;
use crate::repos::notification_repo;
use crate::repos::workspace_repo::{SqliteWorkspaceRepo, WorkspaceRepo};
use crate::time;

/// Highest accepted value for `max_reminders_per_day`. Zero means "never
/// notify"; anything above this bound is a configuration mistake, not intent.
pub const MAX_REMINDERS_PER_DAY_LIMIT: u32 = 24;
/// Smallest accepted snooze duration in minutes.
pub const SNOOZE_MIN_MINUTES: u32 = 1;
/// Largest accepted snooze duration in minutes (12 hours).
pub const SNOOZE_MAX_MINUTES: u32 = 720;
/// Maximum number of snooze durations one rule may configure.
pub const MAX_SNOOZE_OPTIONS: usize = 8;
/// Maximum length of the configured lock-screen-safe message.
pub const MAX_SAFE_MESSAGE_CHARS: usize = 500;
/// Deadline alerts can look ahead from one through seven days independently
/// from the wider dashboard warning window.
pub const MIN_DEADLINE_NOTIFICATION_WINDOW_DAYS: u32 = 1;
pub const MAX_DEADLINE_NOTIFICATION_WINDOW_DAYS: u32 = 7;
pub const DEFAULT_DEADLINE_NOTIFICATION_WINDOW_DAYS: u32 = 3;
/// Default profile for enabled reminder rules when notification times are not
/// configured. Deadline evaluation remains app-open and independent of this time.
pub const DEFAULT_PREFERRED_NOTIFICATION_TIME: &str = "09:00";
pub const DEFAULT_QUIET_HOURS_START: &str = "22:00";
pub const DEFAULT_QUIET_HOURS_END: &str = "07:00";

/// Exact English seed values identify application defaults. Any changed value
/// is user-configured and remains verbatim.
pub const DEFAULT_REMINDER_MESSAGES: [(&str, &str); 5] = [
    ("planning", "Time to plan your day"),
    ("focus", "Ready for a focus block?"),
    ("journaling", "A short journal entry helps reflection"),
    ("deadline", "You have tasks approaching their deadline"),
    ("review", "Weekly review time"),
];

/// A reminder category supported by the MVP.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReminderType {
    Planning,
    Focus,
    Journaling,
    Review,
    Deadline,
}

impl ReminderType {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Planning => "planning",
            Self::Focus => "focus",
            Self::Journaling => "journaling",
            Self::Review => "review",
            Self::Deadline => "deadline",
        }
    }
}

impl std::str::FromStr for ReminderType {
    type Err = AppError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "planning" => Ok(Self::Planning),
            "focus" => Ok(Self::Focus),
            "journaling" => Ok(Self::Journaling),
            "review" => Ok(Self::Review),
            "deadline" => Ok(Self::Deadline),
            _ => Err(AppError::validation("The reminder type is invalid.")),
        }
    }
}

/// Configured output channel. Mobile and both are representable in the schema,
/// although native delivery is outside this scheduling task.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NotificationChannel {
    Desktop,
    Mobile,
    Both,
}

impl NotificationChannel {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Desktop => "desktop",
            Self::Mobile => "mobile",
            Self::Both => "both",
        }
    }
}

impl std::str::FromStr for NotificationChannel {
    type Err = AppError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "desktop" => Ok(Self::Desktop),
            "mobile" => Ok(Self::Mobile),
            "both" => Ok(Self::Both),
            _ => Err(AppError::validation("The notification channel is invalid.")),
        }
    }
}

/// Escalation configured for a rule (`none | gentle` per migration 029). The
/// MVP delivery loop does not interpret escalation yet; it is stored, echoed,
/// and validated so settings can persist it without silent data loss.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReminderEscalation {
    None,
    Gentle,
}

impl ReminderEscalation {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Gentle => "gentle",
        }
    }
}

impl std::str::FromStr for ReminderEscalation {
    type Err = AppError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "none" => Ok(Self::None),
            "gentle" => Ok(Self::Gentle),
            _ => Err(AppError::validation("The reminder escalation is invalid.")),
        }
    }
}

/// Rule data plus delivery-count context for the current UTC calendar day.
/// `deliveries_today` is calculated from history and is not persisted on the
/// rule itself.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReminderRule {
    pub id: String,
    pub workspace_id: String,
    pub reminder_type: ReminderType,
    pub enabled: bool,
    pub recurrence_rule: Option<String>,
    pub preferred_time: Option<NaiveTime>,
    pub time_window_start: Option<NaiveTime>,
    pub time_window_end: Option<NaiveTime>,
    pub quiet_hours_start: Option<NaiveTime>,
    pub quiet_hours_end: Option<NaiveTime>,
    pub deadline_notification_window_days: u32,
    pub max_reminders_per_day: u32,
    /// Allowed snooze durations in minutes, from `snooze_minutes_json`.
    pub snooze_minutes: Vec<u32>,
    pub channel: NotificationChannel,
    pub escalation: ReminderEscalation,
    /// Explicitly configured lock-screen-safe text. This is the only
    /// user-authored text that `build_safe_message` may use, and it is never
    /// combined with notification context.
    pub privacy_safe_message: String,
    pub deliveries_today: u32,
    /// Any delivery today suppresses another automatic evaluation for the same
    /// rule, even if its configured daily maximum is greater than one. The
    /// current schema has no repeat interval, so this prevents poll-loop storms.
    pub has_delivery_today: bool,
}

/// Editable rule configuration sent across the reminder-settings command
/// boundary (Phase 8.4). Time-of-day values are raw `HH:MM` strings so the
/// DTO stays a thin transport form; `validate` parses and enforces the domain
/// rules before anything is persisted.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReminderRuleConfig {
    pub enabled: bool,
    pub preferred_time: Option<String>,
    pub time_window_start: Option<String>,
    pub time_window_end: Option<String>,
    pub quiet_hours_start: Option<String>,
    pub quiet_hours_end: Option<String>,
    pub deadline_notification_window_days: u32,
    pub max_reminders_per_day: u32,
    pub snooze_minutes: Vec<u32>,
    pub channel: NotificationChannel,
    pub escalation: ReminderEscalation,
    pub privacy_safe_message: String,
}

impl ReminderRuleConfig {
    /// Validate every editable field. Invalid values are rejected rather than
    /// stored or guessed at, matching the established validation patterns.
    pub fn validate(&self) -> Result<(), AppError> {
        parse_rule_time(self.preferred_time.clone())?;
        let window_start = parse_rule_time(self.time_window_start.clone())?;
        let window_end = parse_rule_time(self.time_window_end.clone())?;
        let quiet_start = parse_rule_time(self.quiet_hours_start.clone())?;
        let quiet_end = parse_rule_time(self.quiet_hours_end.clone())?;

        // The scheduler treats a start-equals-end interval as never matching.
        // Reject the degenerate configuration instead of silently storing a
        // window that can never be due or quiet. Overnight intervals
        // (start > end) are valid and remain supported.
        if window_start.is_some() && window_start == window_end {
            return Err(AppError::validation(
                "The reminder time window needs different start and end times.",
            ));
        }
        if quiet_start.is_some() && quiet_start == quiet_end {
            return Err(AppError::validation(
                "The quiet hours need different start and end times. Overnight windows are supported.",
            ));
        }

        validate_deadline_notification_window(self.deadline_notification_window_days)?;

        if self.max_reminders_per_day > MAX_REMINDERS_PER_DAY_LIMIT {
            return Err(AppError::validation(format!(
                "The daily reminder limit must be between 0 and {MAX_REMINDERS_PER_DAY_LIMIT}."
            )));
        }
        validate_snooze_options(&self.snooze_minutes)?;

        let message = self.privacy_safe_message.trim();
        if message.is_empty() {
            return Err(AppError::validation(
                "A privacy-safe notification message is required.",
            ));
        }
        if message.chars().count() > MAX_SAFE_MESSAGE_CHARS {
            return Err(AppError::validation(format!(
                "The privacy-safe message is too long (maximum {MAX_SAFE_MESSAGE_CHARS} characters)."
            )));
        }
        Ok(())
    }
}

pub fn validate_deadline_notification_window(days: u32) -> Result<(), AppError> {
    if !(MIN_DEADLINE_NOTIFICATION_WINDOW_DAYS..=MAX_DEADLINE_NOTIFICATION_WINDOW_DAYS)
        .contains(&days)
    {
        return Err(AppError::validation(format!(
            "The deadline notification window must be between {MIN_DEADLINE_NOTIFICATION_WINDOW_DAYS} and {MAX_DEADLINE_NOTIFICATION_WINDOW_DAYS} days."
        )));
    }
    Ok(())
}

/// Snooze durations are explicit user choices: each must be within
/// 1..=720 minutes, unique, and limited to a small number of options so a
/// rule cannot accumulate an unbounded reschedule menu.
pub fn validate_snooze_options(options: &[u32]) -> Result<(), AppError> {
    if options.len() > MAX_SNOOZE_OPTIONS {
        return Err(AppError::validation(format!(
            "A reminder rule can configure at most {MAX_SNOOZE_OPTIONS} snooze options."
        )));
    }
    let mut seen = BTreeSet::new();
    for minutes in options {
        if !(SNOOZE_MIN_MINUTES..=SNOOZE_MAX_MINUTES).contains(minutes) {
            return Err(AppError::validation(format!(
                "Snooze minutes must be between {SNOOZE_MIN_MINUTES} and {SNOOZE_MAX_MINUTES}."
            )));
        }
        if !seen.insert(*minutes) {
            return Err(AppError::validation(
                "Snooze options must not contain duplicate durations.",
            ));
        }
    }
    Ok(())
}

/// Non-sensitive facts that may be used to construct a notification message.
///
/// This type intentionally has no text fields, identifiers, or serializable
/// payload. In particular, callers cannot pass a task title, journal excerpt,
/// therapy content, or arbitrary context through this boundary.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NotificationContext {
    Generic,
    Deadline {
        overdue_tasks: i64,
        approaching_tasks: i64,
    },
}

/// Build the only message form permitted for a local notification.
///
/// Generic categories use the rule's explicitly designated safe message. A
/// deadline message is generated from a clamped count and never uses the
/// configured message or any other context. Negative counts are treated as
/// zero so malformed or untrusted aggregate results cannot produce unsafe
/// output or misleading signed text.
pub fn build_safe_message(rule: &ReminderRule, context: NotificationContext) -> String {
    build_safe_message_for_locale(rule, context, "en-US")
}

/// Resolve a workspace preference to one of the bundled notification locales.
/// System mode uses the runtime-resolved locale; absent, unsupported, or stale
/// values safely fall back to English.
pub fn resolve_notification_locale(
    workspace_locale: &str,
    runtime_locale: Option<&str>,
) -> &'static str {
    match workspace_locale {
        "en-US" => "en-US",
        "es-ES" => "es-ES",
        "de-DE" => "de-DE",
        "fr-FR" => "fr-FR",
        "system" => supported_runtime_locale(runtime_locale),
        _ => "en-US",
    }
}

fn supported_runtime_locale(locale: Option<&str>) -> &'static str {
    match locale {
        Some("en-US") => "en-US",
        Some("es-ES") => "es-ES",
        Some("de-DE") => "de-DE",
        Some("fr-FR") => "fr-FR",
        _ => "en-US",
    }
}

/// Build a privacy-safe message in a previously resolved locale. Only an exact
/// known English seed value for its matching reminder type is translated.
pub fn build_safe_message_for_locale(
    rule: &ReminderRule,
    context: NotificationContext,
    locale: &str,
) -> String {
    let locale = supported_runtime_locale(Some(locale));
    match context {
        NotificationContext::Generic => {
            if is_seeded_default_message(rule) {
                localized_default_message(rule.reminder_type, locale).to_string()
            } else {
                rule.privacy_safe_message.clone()
            }
        }
        NotificationContext::Deadline {
            overdue_tasks,
            approaching_tasks,
        } => build_deadline_message(overdue_tasks, approaching_tasks, locale),
    }
}

fn is_seeded_default_message(rule: &ReminderRule) -> bool {
    DEFAULT_REMINDER_MESSAGES
        .iter()
        .any(|(reminder_type, message)| {
            *reminder_type == rule.reminder_type.as_str() && *message == rule.privacy_safe_message
        })
}

fn localized_default_message(reminder_type: ReminderType, locale: &str) -> &'static str {
    match (reminder_type, locale) {
        (ReminderType::Planning, "es-ES") => "Es hora de planificar tu día",
        (ReminderType::Planning, "de-DE") => "Zeit, deinen Tag zu planen",
        (ReminderType::Planning, "fr-FR") => "Il est temps de planifier votre journée",
        (ReminderType::Focus, "es-ES") => "¿Listo para un bloque de concentración?",
        (ReminderType::Focus, "de-DE") => "Bereit für einen Fokusblock?",
        (ReminderType::Focus, "fr-FR") => "Prêt pour un bloc de concentration ?",
        (ReminderType::Journaling, "es-ES") => "Una breve entrada de diario ayuda a reflexionar",
        (ReminderType::Journaling, "de-DE") => "Ein kurzer Tagebucheintrag hilft beim Nachdenken",
        (ReminderType::Journaling, "fr-FR") => "Une courte entrée de journal aide à réfléchir",
        (ReminderType::Deadline, "es-ES") => "Tienes tareas próximas a su fecha límite",
        (ReminderType::Deadline, "de-DE") => "Bei einigen Aufgaben rückt die Frist näher",
        (ReminderType::Deadline, "fr-FR") => "Certaines tâches approchent de leur échéance",
        (ReminderType::Review, "es-ES") => "Hora de la revisión semanal",
        (ReminderType::Review, "de-DE") => "Zeit für den Wochenrückblick",
        (ReminderType::Review, "fr-FR") => "C'est l'heure du bilan hebdomadaire",
        _ => DEFAULT_REMINDER_MESSAGES
            .iter()
            .find(|(kind, _)| *kind == reminder_type.as_str())
            .map(|(_, message)| *message)
            .unwrap_or("A gentle reminder"),
    }
}

fn build_deadline_message(overdue_tasks: i64, approaching_tasks: i64, locale: &str) -> String {
    let overdue = overdue_tasks.max(0);
    let approaching = approaching_tasks.max(0);
    let mut parts = Vec::new();
    if overdue > 0 {
        parts.push(match (locale, overdue == 1) {
            ("es-ES", true) => format!("{overdue} tarea está vencida"),
            ("es-ES", false) => format!("{overdue} tareas están vencidas"),
            ("de-DE", true) => format!("{overdue} Aufgabe ist überfällig"),
            ("de-DE", false) => format!("{overdue} Aufgaben sind überfällig"),
            ("fr-FR", true) => format!("{overdue} tâche est en retard"),
            ("fr-FR", false) => format!("{overdue} tâches sont en retard"),
            (_, true) => format!("{overdue} task is overdue"),
            (_, false) => format!("{overdue} tasks are overdue"),
        });
    }
    if approaching > 0 {
        parts.push(match (locale, approaching == 1) {
            ("es-ES", true) => format!("{approaching} tarea se acerca a su fecha límite"),
            ("es-ES", false) => format!("{approaching} tareas se acercan a su fecha límite"),
            ("de-DE", true) => format!("{approaching} Aufgabe nähert sich ihrer Frist"),
            ("de-DE", false) => format!("{approaching} Aufgaben nähern sich ihrer Frist"),
            ("fr-FR", true) => format!("{approaching} tâche approche de son échéance"),
            ("fr-FR", false) => format!("{approaching} tâches approchent de leur échéance"),
            (_, true) => format!("{approaching} task is approaching its deadline"),
            (_, false) => format!("{approaching} tasks are approaching their deadline"),
        });
    }
    parts.join("; ")
}

/// Entity categories that may be associated with a reminder. Journal and
/// therapy entities are deliberately not target types in this scheduler.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NotificationTargetType {
    Task,
    Project,
    Goal,
    Block,
    Session,
}

impl NotificationTargetType {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Task => "task",
            Self::Project => "project",
            Self::Goal => "goal",
            Self::Block => "block",
            Self::Session => "session",
        }
    }
}

impl std::str::FromStr for NotificationTargetType {
    type Err = AppError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "task" => Ok(Self::Task),
            "project" => Ok(Self::Project),
            "goal" => Ok(Self::Goal),
            "block" => Ok(Self::Block),
            "session" => Ok(Self::Session),
            _ => Err(AppError::validation(
                "The notification target type is invalid.",
            )),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NotificationTarget {
    pub entity_type: NotificationTargetType,
    pub entity_id: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NotificationDeliveryStatus {
    Scheduled,
    Delivered,
    Snoozed,
    Dismissed,
    Failed,
}

/// Origin category for a workspace-scoped notification history row. Backup is
/// not a ReminderType and has no ReminderRule identity. Block events carry no
/// rule identity either; they are their own terminal origin so they never
/// consume or inherit reminder-rule daily-cap accounting (blocker B4).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NotificationDeliveryKind {
    ReminderRule,
    Backup,
    Block,
}

impl NotificationDeliveryKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::ReminderRule => "reminder_rule",
            Self::Backup => "backup",
            Self::Block => "block",
        }
    }
}

impl std::str::FromStr for NotificationDeliveryKind {
    type Err = AppError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "reminder_rule" => Ok(Self::ReminderRule),
            "backup" => Ok(Self::Backup),
            "block" => Ok(Self::Block),
            _ => Err(AppError::validation(
                "The notification delivery kind is invalid.",
            )),
        }
    }
}

impl NotificationDeliveryStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Scheduled => "scheduled",
            Self::Delivered => "delivered",
            Self::Snoozed => "snoozed",
            Self::Dismissed => "dismissed",
            Self::Failed => "failed",
        }
    }
}

impl std::str::FromStr for NotificationDeliveryStatus {
    type Err = AppError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "scheduled" => Ok(Self::Scheduled),
            "delivered" => Ok(Self::Delivered),
            "snoozed" => Ok(Self::Snoozed),
            "dismissed" => Ok(Self::Dismissed),
            "failed" => Ok(Self::Failed),
            _ => Err(AppError::validation(
                "The notification delivery status is invalid.",
            )),
        }
    }
}

/// One row of notification delivery history. `message` is the lock-screen-safe
/// text copied from the rule at schedule time; `error_message` is always a
/// fixed technical template, never user or task content. `cycle_key` is the
/// backup-reminder cycle identifier (the effective scheduled slot); rule
/// deliveries always keep it `None`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NotificationDelivery {
    pub id: String,
    pub workspace_id: String,
    pub delivery_kind: NotificationDeliveryKind,
    pub reminder_rule_id: Option<String>,
    pub target: Option<NotificationTarget>,
    pub scheduled_at: DateTime<Utc>,
    pub delivered_at: Option<DateTime<Utc>>,
    pub status: NotificationDeliveryStatus,
    pub channel: NotificationChannel,
    pub message: String,
    pub error_message: Option<String>,
    pub created_at: String,
    /// Backup-reminder cycle key (effective scheduled slot). Always `None`
    /// for rule deliveries.
    pub cycle_key: Option<String>,
}

/// Parsed, validated filters for the delivery-history listing (Phase 8.4).
/// Workspace scoping is mandatory; `from`/`to` are inclusive UTC calendar
/// days; `limit` is bounded by the command boundary.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NotificationDeliveryFilters {
    pub workspace_id: String,
    pub reminder_rule_id: Option<String>,
    pub delivery_kind: Option<NotificationDeliveryKind>,
    pub status: Option<NotificationDeliveryStatus>,
    pub from: Option<NaiveDate>,
    pub to: Option<NaiveDate>,
    pub limit: i64,
}

/// Minimal frontend payload for a due, privacy-safe notification.
///
/// Includes optional target metadata (entity_type + entity_id) so the frontend
/// can navigate to the relevant section/task when a notification is clicked.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DueNotification {
    pub id: String,
    pub safe_message: String,
    pub reminder_type: String,
    /// Optional target entity type and id for deep-linking.
    /// When present, the frontend may use this to navigate to the relevant
    /// route and highlight the target element.
    pub target_entity_type: Option<String>,
    pub target_entity_id: Option<String>,
}

/// Returns whether the rule may produce a notification at `now`.
///
/// Quiet-hour intervals are start-inclusive/end-exclusive and support overnight
/// windows. A time window uses the same interval semantics. When a preferred
/// time is configured, eligibility starts at that minute; otherwise the entire
/// configured time window is eligible. Invalid inverted preferred/window
/// combinations simply remain ineligible rather than being guessed.
pub fn should_notify(rule: &ReminderRule, now: DateTime<Utc>) -> bool {
    if !rule.enabled
        || rule.max_reminders_per_day == 0
        || rule.deliveries_today >= rule.max_reminders_per_day
    {
        return false;
    }

    // Without a configured time there is no due instant. The schema's
    // recurrence_rule has no defined syntax yet, so do not guess its cadence or
    // turn newly seeded, enabled rules into immediate startup notifications.
    if rule.recurrence_rule.is_some()
        || (rule.preferred_time.is_none()
            && rule.time_window_start.is_none()
            && rule.time_window_end.is_none())
    {
        return false;
    }

    let current_time = now.time();
    if interval_contains(rule.quiet_hours_start, rule.quiet_hours_end, current_time) {
        return false;
    }

    if let (Some(start), Some(end)) = (rule.time_window_start, rule.time_window_end) {
        if start == end || !interval_contains(Some(start), Some(end), current_time) {
            return false;
        }
        if let Some(preferred) = rule.preferred_time {
            if !interval_contains(Some(start), Some(end), preferred) {
                return false;
            }
        }
    } else if rule.time_window_start.is_some() || rule.time_window_end.is_some() {
        // A partial window is an inclusive one-sided bound.
        if rule
            .time_window_start
            .is_some_and(|start| current_time < start)
            || rule.time_window_end.is_some_and(|end| current_time >= end)
        {
            return false;
        }
    }

    if let Some(preferred) = rule.preferred_time {
        if current_time < preferred {
            return false;
        }
    }

    true
}

/// Whether `at` falls inside the rule's configured quiet-hours interval.
///
/// Quiet-hour intervals are start-inclusive/end-exclusive and support overnight
/// windows, matching `interval_contains`.
pub fn is_quiet_at(rule: &ReminderRule, at: DateTime<Utc>) -> bool {
    interval_contains(rule.quiet_hours_start, rule.quiet_hours_end, at.time())
}

fn interval_contains(start: Option<NaiveTime>, end: Option<NaiveTime>, time: NaiveTime) -> bool {
    match (start, end) {
        (Some(start), Some(end)) if start < end => time >= start && time < end,
        (Some(start), Some(end)) if start > end => time >= start || time < end,
        (Some(_), Some(_)) => false,
        (Some(start), None) => time >= start,
        (None, Some(end)) => time < end,
        (None, None) => false,
    }
}

/// Insert delivery history for the supplied rule and message. This does not
/// dispatch a native notification; it records a scheduled delivery only.
pub fn schedule_notification(
    conn: &rusqlite::Connection,
    rule: &ReminderRule,
    target: Option<&NotificationTarget>,
    message: &str,
    scheduled_at: DateTime<Utc>,
) -> Result<NotificationDelivery, AppError> {
    if message.trim().is_empty() {
        return Err(AppError::validation("A notification message is required."));
    }

    let delivery = NotificationDelivery {
        id: crate::id::new_uuid(),
        workspace_id: rule.workspace_id.clone(),
        delivery_kind: NotificationDeliveryKind::ReminderRule,
        reminder_rule_id: Some(rule.id.clone()),
        target: target.cloned(),
        scheduled_at,
        delivered_at: None,
        status: NotificationDeliveryStatus::Scheduled,
        channel: rule.channel,
        message: message.to_string(),
        error_message: None,
        created_at: time::now_rfc3339(),
        cycle_key: None,
    };
    notification_repo::insert_delivery(conn, &delivery)?;
    Ok(delivery)
}

/// Evaluate enabled reminder rules and persist at most one automatic delivery
/// per rule per UTC day. There is no recurrence interval in the accepted
/// schema, so a pending/delivered history item suppresses further automatic
/// scheduling that day instead of repeatedly scheduling on each poll.
pub fn evaluate_notification_rules(
    conn: &mut rusqlite::Connection,
    now: DateTime<Utc>,
) -> Result<Vec<NotificationDelivery>, AppError> {
    evaluate_notification_rules_with_locale(conn, now, None)
}

/// Evaluate reminder rules using the runtime-resolved locale for workspaces in
/// `system` mode. Persisted notification history is not translated later.
pub fn evaluate_notification_rules_with_locale(
    conn: &mut rusqlite::Connection,
    now: DateTime<Utc>,
    runtime_locale: Option<&str>,
) -> Result<Vec<NotificationDelivery>, AppError> {
    let tx = conn.transaction()?;
    let rules = notification_repo::list_enabled_rules(&tx, now)?;
    let mut scheduled = Vec::new();

    for rule in rules {
        if rule.has_delivery_today || rule.deliveries_today >= rule.max_reminders_per_day {
            continue;
        }
        let workspace_locale = SqliteWorkspaceRepo.get(&tx, &rule.workspace_id)?.locale;
        let locale = resolve_notification_locale(&workspace_locale, runtime_locale);

        if rule.reminder_type == ReminderType::Deadline {
            let (overdue_tasks, approaching_tasks) = deadline_task_counts(&tx, &rule, now)?;
            if overdue_tasks + approaching_tasks == 0 {
                continue;
            }
            scheduled.push(schedule_notification(
                &tx,
                &rule,
                None,
                &build_safe_message_for_locale(
                    &rule,
                    NotificationContext::Deadline {
                        overdue_tasks,
                        approaching_tasks,
                    },
                    locale,
                ),
                now,
            )?);
        } else if should_notify(&rule, now) {
            scheduled.push(schedule_notification(
                &tx,
                &rule,
                None,
                &build_safe_message_for_locale(&rule, NotificationContext::Generic, locale),
                now,
            )?);
        }
    }

    tx.commit()?;
    Ok(scheduled)
}

/// Fixed daily target slot for backup reminders (`09:00` UTC). This is a
/// system constant, not per-rule configuration.
pub const BACKUP_REMINDER_TARGET_TIME: &str = "09:00";
/// Fixed lock-screen-safe message for backup reminders. It is not
/// user-editable and never carries file names, paths, counts, or personal
/// content.
pub const BACKUP_REMINDER_SAFE_MESSAGE: &str = "A validated backup is due.";

fn backup_reminder_message(locale: &str) -> &'static str {
    match locale {
        "es-ES" => "Hay que crear una copia de seguridad validada.",
        "de-DE" => "Eine geprüfte Sicherung ist fällig.",
        "fr-FR" => "Une sauvegarde validée est due.",
        _ => BACKUP_REMINDER_SAFE_MESSAGE,
    }
}

/// The next `09:00` UTC target slot at or after `now`. Backup reminders always
/// fire on this fixed slot, which is outside the quiet interval
/// `[22:00, 07:00)` UTC, so deferring to the next 09:00 slot naturally skips
/// quiet hours: 23:00 defers to tomorrow 09:00 while 06:00 defers to today
/// 09:00.
fn next_backup_slot(now: DateTime<Utc>) -> DateTime<Utc> {
    let today_slot = now
        .date_naive()
        .and_hms_opt(9, 0, 0)
        .expect("09:00 is a valid time")
        .and_utc();
    if now <= today_slot {
        today_slot
    } else {
        today_slot + chrono::Duration::days(1)
    }
}

/// Whether `now` falls inside the fixed backup-reminder quiet interval
/// `[22:00, 07:00)` UTC (start-inclusive, end-exclusive, overnight).
fn is_in_quiet_hours(now: DateTime<Utc>) -> bool {
    let hour = now.hour();
    !(7..22).contains(&hour)
}

/// The validated-backup cycle anchor: its UTC date normalized to the `09:00`
/// target slot. The stored timestamp is parsed but its clock time is ignored;
/// the next cycle is computed from the backup's UTC date plus the interval.
fn backup_cycle_anchor(timestamp: &str) -> Result<DateTime<Utc>, AppError> {
    let backup_at = DateTime::parse_from_rfc3339(timestamp)
        .map_err(|_| AppError::database("The stored backup timestamp is invalid."))?
        .with_timezone(&Utc);
    Ok(backup_at
        .date_naive()
        .and_hms_opt(9, 0, 0)
        .expect("09:00 is a valid time")
        .and_utc())
}

/// Roll an anchor forward to the most recent due cycle slot at or before
/// `now`. This never backfills: with several missed cycles only the current
/// cycle's slot is returned, and the next evaluation advances from there.
fn current_cycle_slot(
    anchor: DateTime<Utc>,
    interval_days: i64,
    now: DateTime<Utc>,
) -> DateTime<Utc> {
    let elapsed_days = (now - anchor).num_days();
    let intervals_elapsed = elapsed_days / interval_days;
    anchor + chrono::Duration::days(intervals_elapsed * interval_days)
}

/// Compute the effective scheduled slot for a workspace's next backup-reminder
/// cycle, or `None` when the cycle is not due yet.
///
/// The cycle anchor is the most recent event among the last validated manual
/// backup and the last recorded backup delivery. When a validated backup is
/// newer than an existing delivery, it takes precedence; when the delivery is
/// newer, the cycle continues from its effective scheduled slot.
fn compute_backup_cycle_slot(
    conn: &rusqlite::Connection,
    workspace_id: &str,
    preference: &BackupReminderPreference,
    now: DateTime<Utc>,
) -> Result<Option<DateTime<Utc>>, AppError> {
    let interval_days = i64::from(preference.interval_days);

    let backup_anchor = preference
        .last_successful_backup_at
        .as_deref()
        .map(backup_cycle_anchor)
        .transpose()?;
    // Retention pruning and history clearing may remove terminal backup
    // delivery rows; the durable per-cycle key survives cleanup (Phase 8.7.4),
    // so the delivery-derived anchor falls back to it when the row is gone.
    let delivery_anchor = notification_repo::latest_backup_delivery(conn, workspace_id)?
        .map(|delivery| delivery.scheduled_at);
    let durable_anchor = notification_repo::latest_backup_cycle_key_slot(conn, workspace_id)?;
    let delivery_slot = match (delivery_anchor, durable_anchor) {
        (Some(row), Some(key)) => Some(row.max(key)),
        (Some(row), None) => Some(row),
        (None, Some(key)) => Some(key),
        (None, None) => None,
    };

    let anchor_slot = match (backup_anchor, delivery_slot) {
        (Some(backup), Some(delivery)) => Some(backup.max(delivery)),
        (Some(backup), None) => Some(backup),
        (None, Some(delivery)) => Some(delivery),
        (None, None) => None,
    };

    match anchor_slot {
        // Initial cycle: no successful backup and no delivery yet. The first
        // reminder is due at the next 09:00 UTC slot, which establishes the
        // initial cycle anchor.
        None => Ok(Some(next_backup_slot(now))),
        Some(anchor) => {
            let next_cycle = anchor + chrono::Duration::days(interval_days);
            if now < next_cycle {
                return Ok(None);
            }
            if is_in_quiet_hours(now) {
                // Defer to the next eligible 09:00 slot (23:00 => tomorrow,
                // 06:00 => today).
                Ok(Some(next_backup_slot(now)))
            } else {
                // The cycle is due now: persist its 09:00 slot and deliver
                // immediately rather than deferring to tomorrow.
                Ok(Some(current_cycle_slot(anchor, interval_days, now)))
            }
        }
    }
}

/// Evaluate the enabled backup-reminder preference for every workspace and
/// persist at most one due backup delivery per workspace per cycle.
///
/// Scheduling semantics (settled L3 contract, 2026-10-07):
/// - The target slot is `09:00` UTC; the quiet interval `[22:00, 07:00)` UTC
///   defers a due reminder to the next 09:00 slot.
/// - If no successful backup exists, the first due reminder is at the next
///   09:00 UTC slot and establishes the initial cycle anchor.
/// - A validated manual backup newer than an existing delivery takes
///   precedence as the cycle anchor; otherwise the cycle continues from the
///   most recent delivery's effective scheduled slot.
/// - An anchored due cycle first evaluated later the same day outside quiet
///   hours is persisted at that day's 09:00 slot and delivered immediately.
/// - Missed cycles are never replayed or backfilled; only the current due
///   cycle is scheduled.
/// - Disabling preserves delivery history and the cycle anchor; re-enabling
///   does not reset recorded cycles. Interval changes affect only the next
///   not-yet-recorded cycle.
/// - Every new delivery claims its durable `backup:<cycle_key>` row so
///   recorded cycles and their anchor survive history clearing and retention
///   pruning (Phase 8.7.4).
/// - Backup deliveries never consume reminder-rule daily caps and never carry
///   a rule, target, or content-derived message.
pub fn evaluate_backup_reminders(
    conn: &mut rusqlite::Connection,
    now: DateTime<Utc>,
) -> Result<Vec<NotificationDelivery>, AppError> {
    evaluate_backup_reminders_with_locale(conn, now, None)
}

/// Evaluate backup reminders using the same workspace/runtime locale selection
/// as reminder rules. Previously scheduled history remains unchanged.
pub fn evaluate_backup_reminders_with_locale(
    conn: &mut rusqlite::Connection,
    now: DateTime<Utc>,
    runtime_locale: Option<&str>,
) -> Result<Vec<NotificationDelivery>, AppError> {
    let tx = conn.transaction()?;
    let workspaces = SqliteWorkspaceRepo.list(&tx)?;
    let mut scheduled = Vec::new();

    for workspace in workspaces {
        let preference = crate::domain::settings::load_backup_reminder(&tx, &workspace.id)?;
        if !preference.enabled {
            continue;
        }

        let effective_slot = match compute_backup_cycle_slot(&tx, &workspace.id, &preference, now)?
        {
            None => continue,
            Some(slot) => slot,
        };

        let cycle_key = effective_slot.to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
        if notification_repo::backup_cycle_exists(&tx, &workspace.id, &cycle_key)? {
            continue;
        }

        // Record the durable cycle key atomically with the row so the cycle
        // stays deduplicated and reliably anchored even if history retention
        // or clearing later removes this terminal delivery row (Phase 8.7.4).
        notification_repo::claim_event_key(
            &tx,
            &workspace.id,
            &format!("backup:{cycle_key}"),
            now,
        )?;

        let delivery = NotificationDelivery {
            id: crate::id::new_uuid(),
            workspace_id: workspace.id.clone(),
            delivery_kind: NotificationDeliveryKind::Backup,
            reminder_rule_id: None,
            target: None,
            scheduled_at: effective_slot,
            delivered_at: None,
            status: NotificationDeliveryStatus::Scheduled,
            channel: NotificationChannel::Desktop,
            message: backup_reminder_message(resolve_notification_locale(
                &workspace.locale,
                runtime_locale,
            ))
            .to_string(),
            error_message: None,
            created_at: time::now_rfc3339(),
            cycle_key: Some(cycle_key),
        };
        notification_repo::insert_delivery(&tx, &delivery)?;
        scheduled.push(delivery);
    }

    tx.commit()?;
    Ok(scheduled)
}

/// Count derived overdue and approaching/blocked-risk tasks for a deadline
/// notification. It uses the same deadline intelligence as the dashboard, but
/// passes the independent notification lead window rather than the workspace's
/// display warning window. Only aggregate counts are used to build the message.
fn deadline_task_counts(
    conn: &rusqlite::Connection,
    rule: &ReminderRule,
    now: DateTime<Utc>,
) -> Result<(i64, i64), AppError> {
    let today = now.date_naive().to_string();
    let at_risk = crate::domain::deadline::at_risk_tasks(
        conn,
        &rule.workspace_id,
        &today,
        rule.deadline_notification_window_days,
    )?;

    let mut overdue = 0_i64;
    let mut approaching = 0_i64;
    for task in at_risk {
        match task.deadline_status {
            crate::domain::deadline::DeadlineStatus::Overdue => overdue += 1,
            crate::domain::deadline::DeadlineStatus::Approaching => approaching += 1,
            crate::domain::deadline::DeadlineStatus::BlockedRisk => {
                if task
                    .deadline_at
                    .as_deref()
                    .is_some_and(|date| date < today.as_str())
                {
                    overdue += 1;
                } else {
                    approaching += 1;
                }
            }
            crate::domain::deadline::DeadlineStatus::None
            | crate::domain::deadline::DeadlineStatus::Normal => {}
        }
    }
    Ok((overdue, approaching))
}

/// Dismiss a scheduled delivery (Phase 8.4). The row stays in history with
/// status `dismissed`; no task, plan, or session state is touched, and a
/// dismissed delivery is never re-scheduled automatically.
pub fn dismiss_delivery(
    conn: &rusqlite::Connection,
    id: &str,
) -> Result<NotificationDelivery, AppError> {
    let delivery = notification_repo::get_delivery(conn, id)?;
    if delivery.status != NotificationDeliveryStatus::Scheduled {
        return Err(AppError::validation(
            "Only a scheduled notification can be dismissed.",
        ));
    }
    notification_repo::mark_delivery_dismissed(conn, id)?;
    notification_repo::get_delivery(conn, id)
}

/// Snooze a scheduled delivery at an explicit user choice: the original row is
/// preserved as `snoozed` history and a new scheduled row is inserted for
/// `now + minutes`. The duration must be one of the rule's configured snooze
/// options, which keeps rescheduling bounded to deliberate actions and avoids
/// notification storms. Missed work is never silently copied or rescheduled:
/// only the notification delivery moves, never any task or plan.
pub fn snooze_delivery(
    conn: &rusqlite::Connection,
    id: &str,
    minutes: u32,
    now: DateTime<Utc>,
) -> Result<NotificationDelivery, AppError> {
    let delivery = notification_repo::get_delivery(conn, id)?;
    if delivery.status != NotificationDeliveryStatus::Scheduled {
        return Err(AppError::validation(
            "Only a scheduled notification can be snoozed.",
        ));
    }
    if delivery.delivery_kind != NotificationDeliveryKind::ReminderRule {
        return Err(AppError::validation(
            "This notification cannot be snoozed through reminder settings.",
        ));
    }
    let reminder_rule_id = delivery.reminder_rule_id.as_deref().ok_or_else(|| {
        AppError::database("The reminder notification is missing its rule reference.")
    })?;
    let options = notification_repo::snooze_options_for_rule(conn, reminder_rule_id)?;
    if !options.contains(&minutes) {
        return Err(AppError::validation(
            "The snooze duration must be one of the configured snooze options for this reminder.",
        ));
    }
    notification_repo::mark_delivery_snoozed(conn, id)?;

    let scheduled_at = now
        .checked_add_signed(chrono::Duration::minutes(i64::from(minutes)))
        .ok_or_else(|| AppError::validation("The snooze duration is out of range."))?;
    let rescheduled = NotificationDelivery {
        id: crate::id::new_uuid(),
        workspace_id: delivery.workspace_id,
        delivery_kind: delivery.delivery_kind,
        reminder_rule_id: delivery.reminder_rule_id,
        target: delivery.target,
        scheduled_at,
        delivered_at: None,
        status: NotificationDeliveryStatus::Scheduled,
        channel: delivery.channel,
        message: delivery.message,
        error_message: None,
        created_at: time::now_rfc3339(),
        cycle_key: delivery.cycle_key,
    };
    notification_repo::insert_delivery(conn, &rescheduled)?;
    Ok(rescheduled)
}

/// Parse a schema `HH:MM` time with the repository's canonical time parser.
pub(crate) fn parse_rule_time(value: Option<String>) -> Result<Option<NaiveTime>, AppError> {
    value
        .map(|value| {
            time::parse_time_of_day(&value)?;
            NaiveTime::parse_from_str(&value, "%H:%M")
                .map_err(|_| AppError::validation("A reminder time is invalid."))
        })
        .transpose()
}

#[cfg(test)]
mod tests {
    use chrono::TimeZone;

    use super::*;
    use crate::test_utils::test_conn;

    fn utc_time(hour: u32, minute: u32) -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 10, 5, hour, minute, 0)
            .single()
            .expect("valid UTC test datetime")
    }

    fn rule() -> ReminderRule {
        ReminderRule {
            id: "rule-1".to_string(),
            workspace_id: "w1".to_string(),
            reminder_type: ReminderType::Planning,
            enabled: true,
            recurrence_rule: None,
            preferred_time: Some(NaiveTime::from_hms_opt(0, 0, 0).unwrap()),
            time_window_start: None,
            time_window_end: None,
            quiet_hours_start: None,
            quiet_hours_end: None,
            deadline_notification_window_days: DEFAULT_DEADLINE_NOTIFICATION_WINDOW_DAYS,
            max_reminders_per_day: 5,
            snooze_minutes: vec![10, 30, 60],
            channel: NotificationChannel::Desktop,
            escalation: ReminderEscalation::None,
            privacy_safe_message: "A gentle reminder".to_string(),
            deliveries_today: 0,
            has_delivery_today: false,
        }
    }

    #[test]
    fn safe_messages_never_copy_sensitive_context() {
        let mut configured = rule();
        configured.privacy_safe_message = "A safe reminder".to_string();

        let message = build_safe_message(&configured, NotificationContext::Generic);

        assert_eq!(message, "A safe reminder");
        assert!(!message.contains("Secret task title"));
        assert!(!message.contains("private journal excerpt"));
        assert!(!message.contains("therapy content"));
    }

    #[test]
    fn seeded_defaults_are_localized_only_for_exact_type_and_seed_value() {
        let expected = [
            (
                "planning",
                "Time to plan your day",
                "Es hora de planificar tu día",
                "Zeit, deinen Tag zu planen",
                "Il est temps de planifier votre journée",
            ),
            (
                "focus",
                "Ready for a focus block?",
                "¿Listo para un bloque de concentración?",
                "Bereit für einen Fokusblock?",
                "Prêt pour un bloc de concentration ?",
            ),
            (
                "journaling",
                "A short journal entry helps reflection",
                "Una breve entrada de diario ayuda a reflexionar",
                "Ein kurzer Tagebucheintrag hilft beim Nachdenken",
                "Une courte entrée de journal aide à réfléchir",
            ),
            (
                "deadline",
                "You have tasks approaching their deadline",
                "Tienes tareas próximas a su fecha límite",
                "Bei einigen Aufgaben rückt die Frist näher",
                "Certaines tâches approchent de leur échéance",
            ),
            (
                "review",
                "Weekly review time",
                "Hora de la revisión semanal",
                "Zeit für den Wochenrückblick",
                "C'est l'heure du bilan hebdomadaire",
            ),
        ];
        for (kind, en, es, de, fr) in expected {
            let mut configured = rule();
            configured.reminder_type = kind.parse().expect("valid reminder type");
            configured.privacy_safe_message = en.to_string();
            for (locale, expected_message) in
                [("en-US", en), ("es-ES", es), ("de-DE", de), ("fr-FR", fr)]
            {
                assert_eq!(
                    build_safe_message_for_locale(
                        &configured,
                        NotificationContext::Generic,
                        locale
                    ),
                    expected_message,
                    "{kind} in {locale}"
                );
            }
        }

        let mut customized = rule();
        customized.privacy_safe_message = "  My lock-screen-safe wording.  ".to_string();
        assert_eq!(
            build_safe_message_for_locale(&customized, NotificationContext::Generic, "fr-FR"),
            "  My lock-screen-safe wording.  "
        );
        customized.privacy_safe_message = "Time to plan your day".to_string();
        customized.reminder_type = ReminderType::Review;
        assert_eq!(
            build_safe_message_for_locale(&customized, NotificationContext::Generic, "fr-FR"),
            "Time to plan your day",
            "a seed string on the wrong reminder type is user-configured"
        );
    }

    #[test]
    fn workspace_override_wins_and_runtime_locale_is_allowlisted_with_english_fallback() {
        assert_eq!(resolve_notification_locale("fr-FR", Some("de-DE")), "fr-FR");
        assert_eq!(
            resolve_notification_locale("system", Some("es-ES")),
            "es-ES"
        );
        assert_eq!(resolve_notification_locale("system", None), "en-US");
        assert_eq!(
            resolve_notification_locale("system", Some("pt-BR")),
            "en-US"
        );
        assert_eq!(
            resolve_notification_locale("system", Some("system")),
            "en-US"
        );
        assert_eq!(
            resolve_notification_locale("invalid", Some("fr-FR")),
            "en-US"
        );
        assert_eq!(
            backup_reminder_message("en-US"),
            "A validated backup is due."
        );
        assert_eq!(
            backup_reminder_message("es-ES"),
            "Hay que crear una copia de seguridad validada."
        );
        assert_eq!(
            backup_reminder_message("de-DE"),
            "Eine geprüfte Sicherung ist fällig."
        );
        assert_eq!(
            backup_reminder_message("fr-FR"),
            "Une sauvegarde validée est due."
        );
    }

    #[test]
    fn deadline_messages_are_localized_count_only_and_ignore_configured_text() {
        let mut configured = rule();
        configured.reminder_type = ReminderType::Deadline;
        configured.privacy_safe_message =
            "Private title, journal excerpt, therapy note, and custom wording".to_string();
        let context = NotificationContext::Deadline {
            overdue_tasks: 2,
            approaching_tasks: 1,
        };
        for (locale, expected) in [
            (
                "en-US",
                "2 tasks are overdue; 1 task is approaching its deadline",
            ),
            (
                "es-ES",
                "2 tareas están vencidas; 1 tarea se acerca a su fecha límite",
            ),
            (
                "de-DE",
                "2 Aufgaben sind überfällig; 1 Aufgabe nähert sich ihrer Frist",
            ),
            (
                "fr-FR",
                "2 tâches sont en retard; 1 tâche approche de son échéance",
            ),
        ] {
            let message = build_safe_message_for_locale(&configured, context, locale);
            assert_eq!(message, expected, "{locale}");
            assert!(!message.contains("Private title"));
            assert!(!message.contains("journal excerpt"));
            assert!(!message.contains("therapy note"));
            assert!(!message.contains("custom wording"));
        }
        assert_eq!(
            build_safe_message_for_locale(
                &configured,
                NotificationContext::Deadline {
                    overdue_tasks: -4,
                    approaching_tasks: 0,
                },
                "es-ES"
            ),
            ""
        );
    }

    #[test]
    fn deadline_message_contains_only_a_non_negative_count() {
        let mut configured = rule();
        configured.privacy_safe_message =
            "Secret task title and private journal excerpt and therapy content".to_string();

        let message = build_safe_message(
            &configured,
            NotificationContext::Deadline {
                overdue_tasks: 2,
                approaching_tasks: 3,
            },
        );
        assert_eq!(
            message,
            "2 tasks are overdue; 3 tasks are approaching their deadline"
        );
        assert!(!message.contains("Secret task title"));
        assert!(!message.contains("private journal excerpt"));
        assert!(!message.contains("therapy content"));

        assert_eq!(
            build_safe_message(
                &configured,
                NotificationContext::Deadline {
                    overdue_tasks: -7,
                    approaching_tasks: 0,
                },
            ),
            ""
        );
    }

    #[test]
    fn quiet_hours_block_normal_and_overnight_intervals() {
        let mut configured = rule();
        configured.quiet_hours_start = Some(NaiveTime::from_hms_opt(22, 0, 0).unwrap());
        configured.quiet_hours_end = Some(NaiveTime::from_hms_opt(7, 0, 0).unwrap());

        assert!(!should_notify(&configured, utc_time(23, 30)));
        assert!(!should_notify(&configured, utc_time(6, 59)));
        assert!(should_notify(&configured, utc_time(7, 0)));
        assert!(should_notify(&configured, utc_time(12, 0)));
    }

    #[test]
    fn max_reminders_per_day_is_enforced() {
        let mut configured = rule();
        configured.max_reminders_per_day = 2;
        configured.deliveries_today = 1;
        assert!(should_notify(&configured, utc_time(12, 0)));
        configured.deliveries_today = 2;
        assert!(!should_notify(&configured, utc_time(12, 0)));
        configured.max_reminders_per_day = 0;
        configured.deliveries_today = 0;
        assert!(!should_notify(&configured, utc_time(12, 0)));
    }

    #[test]
    fn unconfigured_and_unsupported_recurrence_rules_are_not_due() {
        let mut configured = rule();
        configured.preferred_time = None;
        assert!(!should_notify(&configured, utc_time(12, 0)));

        configured.preferred_time = Some(NaiveTime::from_hms_opt(9, 0, 0).unwrap());
        configured.recurrence_rule = Some("unsupported recurrence syntax".to_string());
        assert!(!should_notify(&configured, utc_time(12, 0)));
    }

    #[test]
    fn preferred_time_and_window_gate_eligibility() {
        let mut configured = rule();
        configured.preferred_time = Some(NaiveTime::from_hms_opt(9, 30, 0).unwrap());
        configured.time_window_start = Some(NaiveTime::from_hms_opt(9, 0, 0).unwrap());
        configured.time_window_end = Some(NaiveTime::from_hms_opt(11, 0, 0).unwrap());

        assert!(!should_notify(&configured, utc_time(9, 29)));
        assert!(should_notify(&configured, utc_time(9, 30)));
        assert!(should_notify(&configured, utc_time(10, 45)));
        assert!(!should_notify(&configured, utc_time(11, 0)));

        configured.time_window_start = Some(NaiveTime::from_hms_opt(22, 0, 0).unwrap());
        configured.time_window_end = Some(NaiveTime::from_hms_opt(2, 0, 0).unwrap());
        configured.preferred_time = None;
        assert!(should_notify(&configured, utc_time(23, 0)));
        assert!(should_notify(&configured, utc_time(1, 0)));
        assert!(!should_notify(&configured, utc_time(12, 0)));
    }

    #[test]
    fn evaluation_does_not_duplicate_deliveries_on_repeated_polls() {
        let mut conn = test_conn();
        conn.execute(
            "INSERT INTO reminder_rules (
                id, workspace_id, type, enabled, preferred_time, privacy_safe_message,
                created_at, updated_at
             ) VALUES ('rule-1', 'w1', 'planning', 1, '00:00', 'Plan gently',
                       '2026-10-05T00:00:00Z', '2026-10-05T00:00:00Z')",
            [],
        )
        .expect("insert reminder rule");
        let now = utc_time(10, 0);

        let first = evaluate_notification_rules(&mut conn, now).expect("first evaluation");
        let second = evaluate_notification_rules(&mut conn, now).expect("second evaluation");
        assert_eq!(first.len(), 1);
        assert!(second.is_empty());
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM notification_deliveries", [], |row| {
                row.get(0)
            })
            .expect("count deliveries");
        assert_eq!(count, 1);
    }

    #[test]
    fn evaluation_uses_manual_workspace_locale_instead_of_runtime_locale() {
        let mut conn = test_conn();
        conn.execute("UPDATE workspaces SET locale = 'fr-FR' WHERE id = 'w1'", [])
            .expect("set manual workspace locale");
        conn.execute(
            "INSERT INTO reminder_rules (
                id, workspace_id, type, enabled, preferred_time, privacy_safe_message,
                created_at, updated_at
             ) VALUES ('rule-1', 'w1', 'planning', 1, '00:00',
                       'Time to plan your day', '2026-10-05T00:00:00Z',
                       '2026-10-05T00:00:00Z')",
            [],
        )
        .expect("insert seeded default reminder");

        let deliveries =
            evaluate_notification_rules_with_locale(&mut conn, utc_time(10, 0), Some("de-DE"))
                .expect("evaluate localized reminder");
        assert_eq!(deliveries.len(), 1);
        assert_eq!(
            deliveries[0].message,
            "Il est temps de planifier votre journée"
        );
    }

    fn insert_deadline_rule(conn: &rusqlite::Connection, enabled: bool, window_days: u32) {
        conn.execute(
            "INSERT INTO reminder_rules (
                id, workspace_id, type, enabled, deadline_notification_window_days,
                privacy_safe_message, created_at, updated_at
             ) VALUES ('deadline-rule', 'w1', 'deadline', ?1, ?2, 'Safe default',
                       '2026-10-05T00:00:00Z', '2026-10-05T00:00:00Z')",
            rusqlite::params![enabled, window_days],
        )
        .expect("insert deadline rule");
    }

    fn insert_deadline_task(
        conn: &rusqlite::Connection,
        id: &str,
        status: &str,
        target_date: &str,
    ) {
        conn.execute(
            "INSERT INTO tasks (
                id, workspace_id, title, status, target_date, created_at, updated_at
             ) VALUES (?1, 'w1', ?2, ?3, ?4, '2026-10-01T00:00:00Z',
                       '2026-10-01T00:00:00Z')",
            rusqlite::params![id, format!("private task title {id}"), status, target_date],
        )
        .expect("insert deadline task");
    }

    fn evaluate_deadline(conn: &mut rusqlite::Connection) -> Vec<NotificationDelivery> {
        evaluate_notification_rules(conn, utc_time(10, 0)).expect("evaluate rules")
    }

    #[test]
    fn deadline_alert_reports_overdue_count_without_exposing_task_content_or_changing_status() {
        let mut conn = test_conn();
        insert_deadline_rule(&conn, true, 3);
        insert_deadline_task(&conn, "late-1", "planned", "2026-10-04");
        insert_deadline_task(&conn, "late-2", "in_progress", "2026-10-03");

        let deliveries = evaluate_deadline(&mut conn);

        assert_eq!(deliveries.len(), 1);
        assert_eq!(deliveries[0].scheduled_at, utc_time(10, 0));
        assert_eq!(deliveries[0].message, "2 tasks are overdue");
        assert!(!deliveries[0].message.contains("private task title"));
        let statuses: Vec<String> = conn
            .prepare("SELECT status FROM tasks ORDER BY id")
            .expect("prepare task status query")
            .query_map([], |row| row.get(0))
            .expect("query statuses")
            .collect::<Result<_, _>>()
            .expect("collect statuses");
        assert_eq!(statuses, vec!["planned", "in_progress"]);
    }

    #[test]
    fn deadline_alert_includes_approaching_tasks_at_one_and_seven_days() {
        let mut conn = test_conn();
        insert_deadline_rule(&conn, true, 7);
        insert_deadline_task(&conn, "one-day", "planned", "2026-10-06");
        insert_deadline_task(&conn, "seven-day", "blocked", "2026-10-12");

        let deliveries = evaluate_deadline(&mut conn);

        assert_eq!(deliveries.len(), 1);
        assert_eq!(
            deliveries[0].message,
            "2 tasks are approaching their deadline"
        );

        let mut one_day_conn = test_conn();
        insert_deadline_rule(&one_day_conn, true, 1);
        insert_deadline_task(&one_day_conn, "one-day", "planned", "2026-10-06");
        insert_deadline_task(&one_day_conn, "seven-day", "planned", "2026-10-12");
        one_day_conn
            .execute(
                "UPDATE workspaces SET settings_json = '{\"warning_window_days\":7}'
                 WHERE id = 'w1'",
                [],
            )
            .expect("set wider workspace display warning window");
        let one_day_deliveries = evaluate_deadline(&mut one_day_conn);
        assert_eq!(one_day_deliveries.len(), 1);
        assert_eq!(
            one_day_deliveries[0].message,
            "1 task is approaching its deadline"
        );
    }

    #[test]
    fn deadline_alert_counts_a_blocked_risk_task_as_approaching() {
        let mut conn = test_conn();
        insert_deadline_rule(&conn, true, 3);
        insert_deadline_task(&conn, "prerequisite", "planned", "2026-10-20");
        insert_deadline_task(&conn, "blocked-risk", "in_progress", "2026-10-06");
        conn.execute(
            "INSERT INTO task_dependencies (
                id, blocked_task_id, prerequisite_task_id, created_at
             ) VALUES ('dep-1', 'blocked-risk', 'prerequisite', '2026-10-01T00:00:00Z')",
            [],
        )
        .expect("insert incomplete prerequisite");

        let deliveries = evaluate_deadline(&mut conn);

        assert_eq!(deliveries.len(), 1);
        assert_eq!(deliveries[0].message, "1 task is approaching its deadline");
    }

    #[test]
    fn deadline_alert_is_not_created_outside_the_notification_window() {
        let mut conn = test_conn();
        insert_deadline_rule(&conn, true, 3);
        insert_deadline_task(&conn, "later", "planned", "2026-10-09");

        assert!(evaluate_deadline(&mut conn).is_empty());
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM notification_deliveries", [], |row| {
                row.get(0)
            })
            .expect("count deliveries");
        assert_eq!(count, 0);
    }

    #[test]
    fn disabled_deadline_rule_does_not_create_an_alert() {
        let mut conn = test_conn();
        insert_deadline_rule(&conn, false, 3);
        insert_deadline_task(&conn, "late", "planned", "2026-10-04");

        assert!(evaluate_deadline(&mut conn).is_empty());
    }

    #[test]
    fn deadline_alert_is_suppressed_for_the_rest_of_its_utc_calendar_day() {
        let mut conn = test_conn();
        insert_deadline_rule(&conn, true, 3);
        insert_deadline_task(&conn, "late", "planned", "2026-10-04");

        assert_eq!(evaluate_deadline(&mut conn).len(), 1);
        assert!(
            evaluate_deadline(&mut conn).is_empty(),
            "repeat poll suppressed"
        );
        // The guard is backed by delivery history, not transient process state,
        // so startup/poll evaluations cannot regenerate the same day's alert.
        assert!(
            evaluate_deadline(&mut conn).is_empty(),
            "later evaluation suppressed"
        );
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM notification_deliveries", [], |row| {
                row.get(0)
            })
            .expect("count deliveries");
        assert_eq!(count, 1);
    }

    #[test]
    fn missed_notification_does_not_change_task_status() {
        let mut conn = test_conn();
        conn.execute(
            "INSERT INTO tasks (id, workspace_id, title, status, created_at, updated_at)
             VALUES ('task-1', 'w1', 'Keep working', 'in_progress',
                     '2026-10-05T00:00:00Z', '2026-10-05T00:00:00Z')",
            [],
        )
        .expect("insert task");
        conn.execute(
            "INSERT INTO reminder_rules (
                id, workspace_id, type, enabled, preferred_time, privacy_safe_message,
                created_at, updated_at
             ) VALUES ('rule-1', 'w1', 'planning', 1, '00:00', 'Plan gently',
                       '2026-10-05T00:00:00Z', '2026-10-05T00:00:00Z')",
            [],
        )
        .expect("insert reminder rule");

        let scheduled = evaluate_notification_rules(&mut conn, utc_time(10, 0))
            .expect("evaluate notification rules");
        assert_eq!(scheduled.len(), 1);

        conn.execute(
            "UPDATE notification_deliveries
             SET status = 'failed', error_message = 'Missed while app was closed'
             WHERE id = ?1",
            [&scheduled[0].id],
        )
        .expect("mark notification missed");

        let status: String = conn
            .query_row("SELECT status FROM tasks WHERE id = 'task-1'", [], |row| {
                row.get(0)
            })
            .expect("read task status");
        assert_eq!(status, "in_progress");
    }

    fn valid_config() -> ReminderRuleConfig {
        ReminderRuleConfig {
            enabled: true,
            preferred_time: Some("09:00".to_string()),
            time_window_start: None,
            time_window_end: None,
            quiet_hours_start: Some("22:00".to_string()),
            quiet_hours_end: Some("07:00".to_string()),
            deadline_notification_window_days: DEFAULT_DEADLINE_NOTIFICATION_WINDOW_DAYS,
            max_reminders_per_day: 5,
            snooze_minutes: vec![10, 30, 60],
            channel: NotificationChannel::Desktop,
            escalation: ReminderEscalation::None,
            privacy_safe_message: "A gentle reminder".to_string(),
        }
    }

    #[test]
    fn config_validation_rejects_invalid_values() {
        let mut config = valid_config();
        config.preferred_time = Some("9:00".to_string());
        assert!(config.validate().is_err());

        let mut config = valid_config();
        config.quiet_hours_start = Some("22:00".to_string());
        config.quiet_hours_end = Some("22:00".to_string());
        assert!(config.validate().is_err());

        let mut config = valid_config();
        config.time_window_start = Some("11:00".to_string());
        config.time_window_end = Some("09:00".to_string());
        config.preferred_time = None;
        assert!(
            config.validate().is_ok(),
            "overnight windows remain valid configuration"
        );
        let mut config = valid_config();
        config.time_window_start = Some("09:00".to_string());
        config.time_window_end = Some("09:00".to_string());
        assert!(config.validate().is_err());

        let mut config = valid_config();
        config.deadline_notification_window_days = 0;
        assert!(config.validate().is_err());
        let mut config = valid_config();
        config.deadline_notification_window_days = 8;
        assert!(config.validate().is_err());
        let mut config = valid_config();
        config.deadline_notification_window_days = 1;
        assert!(config.validate().is_ok());
        config.deadline_notification_window_days = 7;
        assert!(config.validate().is_ok());

        let mut config = valid_config();
        config.max_reminders_per_day = MAX_REMINDERS_PER_DAY_LIMIT + 1;
        assert!(config.validate().is_err());
        let mut config = valid_config();
        config.max_reminders_per_day = 0;
        assert!(config.validate().is_ok(), "zero means never notify");

        let mut config = valid_config();
        config.snooze_minutes = vec![0];
        assert!(config.validate().is_err());
        let mut config = valid_config();
        config.snooze_minutes = vec![721];
        assert!(config.validate().is_err());
        let mut config = valid_config();
        config.snooze_minutes = vec![30, 30];
        assert!(config.validate().is_err());
        let mut config = valid_config();
        config.snooze_minutes = (1..=9).collect();
        assert!(config.validate().is_err());

        let mut config = valid_config();
        config.privacy_safe_message = "   ".to_string();
        assert!(config.validate().is_err());
        let mut config = valid_config();
        config.privacy_safe_message = "x".repeat(MAX_SAFE_MESSAGE_CHARS + 1);
        assert!(config.validate().is_err());

        assert!(valid_config().validate().is_ok());
    }

    #[test]
    fn unset_profile_times_are_valid_and_default_profile_uses_overnight_quiet_hours() {
        let mut config = valid_config();
        config.preferred_time = None;
        config.quiet_hours_start = None;
        config.quiet_hours_end = None;
        assert!(config.validate().is_ok());

        assert_eq!(DEFAULT_PREFERRED_NOTIFICATION_TIME, "09:00");
        assert_eq!(DEFAULT_QUIET_HOURS_START, "22:00");
        assert_eq!(DEFAULT_QUIET_HOURS_END, "07:00");
    }

    #[test]
    fn escalation_and_status_enums_reject_unknown_values() {
        assert_eq!(
            "gentle".parse::<ReminderEscalation>().expect("gentle"),
            ReminderEscalation::Gentle
        );
        assert!("loud".parse::<ReminderEscalation>().is_err());
        assert!("dismissed".parse::<NotificationDeliveryStatus>().is_ok());
        assert!("pending".parse::<NotificationDeliveryStatus>().is_err());
    }

    fn insert_rule_and_delivery(
        conn: &rusqlite::Connection,
        snooze_json: &str,
        delivery_status: &str,
    ) {
        conn.execute(
            "INSERT INTO reminder_rules (
                id, workspace_id, type, snooze_minutes_json, privacy_safe_message,
                created_at, updated_at
             ) VALUES ('rule-1', 'w1', 'focus', ?1, 'Ready for a focus block?',
                       '2026-10-05T00:00:00Z', '2026-10-05T00:00:00Z')",
            [snooze_json],
        )
        .expect("insert rule");
        conn.execute(
            "INSERT INTO notification_deliveries (
                id, workspace_id, delivery_kind, reminder_rule_id,
                target_entity_type, target_entity_id,
                scheduled_at, status, channel, safe_message, created_at
             ) VALUES ('delivery-1', 'w1', 'reminder_rule', 'rule-1', 'block', 'b1',
                       '2026-10-05T09:00:00Z', ?1, 'desktop',
                       'Ready for a focus block?', '2026-10-05T08:00:00Z')",
            [delivery_status],
        )
        .expect("insert delivery");
    }

    #[test]
    fn dismiss_marks_the_scheduled_delivery_and_keeps_history() {
        let conn = test_conn();
        insert_rule_and_delivery(&conn, "[10,30,60]", "scheduled");

        let dismissed = dismiss_delivery(&conn, "delivery-1").expect("dismiss");
        assert_eq!(dismissed.status, NotificationDeliveryStatus::Dismissed);
        assert_eq!(dismissed.message, "Ready for a focus block?");
        assert_eq!(dismissed.reminder_rule_id.as_deref(), Some("rule-1"));

        let status: String = conn
            .query_row(
                "SELECT status FROM notification_deliveries WHERE id = 'delivery-1'",
                [],
                |row| row.get(0),
            )
            .expect("read status");
        assert_eq!(status, "dismissed");
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM notification_deliveries", [], |row| {
                row.get(0)
            })
            .expect("count");
        assert_eq!(count, 1, "dismiss preserves the original row");
    }

    #[test]
    fn dismiss_rejects_unknown_or_non_scheduled_deliveries() {
        let conn = test_conn();
        insert_rule_and_delivery(&conn, "[10,30,60]", "delivered");

        assert!(dismiss_delivery(&conn, "delivery-1").is_err());
        assert!(dismiss_delivery(&conn, "missing-id").is_err());
    }

    #[test]
    fn snooze_preserves_history_and_reschedules_only_the_delivery() {
        let conn = test_conn();
        insert_rule_and_delivery(&conn, "[15,90]", "scheduled");
        let now = utc_time(10, 0);

        let rescheduled = snooze_delivery(&conn, "delivery-1", 15, now).expect("snooze");
        assert_eq!(rescheduled.status, NotificationDeliveryStatus::Scheduled);
        assert_eq!(rescheduled.scheduled_at, utc_time(10, 15));
        assert_eq!(rescheduled.message, "Ready for a focus block?");
        assert_eq!(rescheduled.reminder_rule_id.as_deref(), Some("rule-1"));
        assert_ne!(rescheduled.id, "delivery-1");
        let target = rescheduled.target.as_ref().expect("target preserved");
        assert_eq!(target.entity_type, NotificationTargetType::Block);
        assert_eq!(&target.entity_id, "b1");

        let (original_status, original_scheduled_at): (String, String) = conn
            .query_row(
                "SELECT status, scheduled_at FROM notification_deliveries WHERE id = 'delivery-1'",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .expect("read original row");
        assert_eq!(original_status, "snoozed");
        assert_eq!(
            original_scheduled_at, "2026-10-05T09:00:00Z",
            "history of the original schedule is preserved"
        );

        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM notification_deliveries", [], |row| {
                row.get(0)
            })
            .expect("count");
        assert_eq!(count, 2);
    }

    #[test]
    fn snooze_requires_a_configured_duration_and_a_scheduled_delivery() {
        let conn = test_conn();
        insert_rule_and_delivery(&conn, "[15,90]", "scheduled");
        let now = utc_time(10, 0);

        assert!(snooze_delivery(&conn, "delivery-1", 20, now).is_err());
        assert!(snooze_delivery(&conn, "missing-id", 15, now).is_err());

        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM notification_deliveries", [], |row| {
                row.get(0)
            })
            .expect("count");
        assert_eq!(count, 1, "rejected snoozes change nothing");
    }

    // --- Backup-reminder dispatcher tests ---

    fn utc_datetime(year: i32, month: u32, day: u32, hour: u32, minute: u32) -> DateTime<Utc> {
        Utc.with_ymd_and_hms(year, month, day, hour, minute, 0)
            .single()
            .expect("valid UTC test datetime")
    }

    fn set_backup_preference(conn: &rusqlite::Connection, enabled: bool, interval_days: u32) {
        crate::domain::settings::update_backup_reminder(conn, "w1", enabled, interval_days)
            .expect("set backup preference");
    }

    fn record_backup(conn: &rusqlite::Connection, timestamp: &str) {
        crate::domain::settings::record_successful_backup(conn, "w1", timestamp)
            .expect("record backup");
    }

    fn backup_deliveries(conn: &rusqlite::Connection) -> Vec<(String, String, String)> {
        conn.prepare("SELECT id, status, cycle_key FROM notification_deliveries WHERE delivery_kind = 'backup' ORDER BY scheduled_at")
            .expect("prepare backup delivery query")
            .query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))
            .expect("query backup deliveries")
            .collect::<Result<_, _>>()
            .expect("collect backup deliveries")
    }

    #[test]
    fn backup_reminder_first_cycle_is_due_and_scheduled_at_next_0900_slot() {
        let mut conn = test_conn();
        set_backup_preference(&conn, true, 7);
        // No successful backup and no deliveries: the first cycle is due now.
        let now = utc_datetime(2026, 10, 5, 10, 0);

        let scheduled = evaluate_backup_reminders(&mut conn, now).expect("evaluate");
        assert_eq!(scheduled.len(), 1);
        assert_eq!(scheduled[0].delivery_kind, NotificationDeliveryKind::Backup);
        assert_eq!(scheduled[0].message, "A validated backup is due.");
        assert_eq!(scheduled[0].reminder_rule_id, None);
        assert_eq!(scheduled[0].target, None);
        assert_eq!(scheduled[0].channel, NotificationChannel::Desktop);
        // Next 09:00 slot from 10:00 is the following day at 09:00.
        assert_eq!(scheduled[0].scheduled_at, utc_datetime(2026, 10, 6, 9, 0));
        assert_eq!(
            scheduled[0].cycle_key.as_deref(),
            Some("2026-10-06T09:00:00Z")
        );
    }

    #[test]
    fn backup_reminder_quiet_hours_defer_to_next_0900_slot() {
        let mut conn = test_conn();
        set_backup_preference(&conn, true, 7);
        // 23:00 UTC is inside the [22:00, 07:00) quiet interval.
        let now = utc_datetime(2026, 10, 5, 23, 30);

        let scheduled = evaluate_backup_reminders(&mut conn, now).expect("evaluate");
        assert_eq!(scheduled.len(), 1);
        // Deferred to the next 09:00 slot, never inside quiet hours.
        assert_eq!(scheduled[0].scheduled_at, utc_datetime(2026, 10, 6, 9, 0));
    }

    #[test]
    fn backup_reminder_inside_quiet_hours_before_0700_defers_to_same_day_0900() {
        let mut conn = test_conn();
        set_backup_preference(&conn, true, 7);
        // 06:00 UTC is inside the quiet interval but before the 09:00 slot.
        let now = utc_datetime(2026, 10, 5, 6, 0);

        let scheduled = evaluate_backup_reminders(&mut conn, now).expect("evaluate");
        assert_eq!(scheduled.len(), 1);
        assert_eq!(scheduled[0].scheduled_at, utc_datetime(2026, 10, 5, 9, 0));
    }

    #[test]
    fn backup_reminder_next_cycle_is_based_on_validated_backup_date_plus_interval() {
        let mut conn = test_conn();
        set_backup_preference(&conn, true, 7);
        record_backup(&conn, "2026-10-01T15:30:00Z");
        // Backup UTC date is 2026-10-01; next cycle is 2026-10-08T09:00.
        // Evaluate before the cycle is due: no delivery.
        let before = evaluate_backup_reminders(&mut conn, utc_datetime(2026, 10, 8, 8, 0))
            .expect("evaluate before due");
        assert!(before.is_empty());

        // Evaluate after the cycle is due, later the same day outside quiet
        // hours: persist today's 09:00 slot, do not defer to tomorrow.
        let scheduled = evaluate_backup_reminders(&mut conn, utc_datetime(2026, 10, 8, 10, 0))
            .expect("evaluate after due");
        assert_eq!(scheduled.len(), 1);
        assert_eq!(scheduled[0].scheduled_at, utc_datetime(2026, 10, 8, 9, 0));
        assert_eq!(
            scheduled[0].cycle_key.as_deref(),
            Some("2026-10-08T09:00:00Z")
        );
    }

    #[test]
    fn backup_cycle_anchor_and_dedupe_survive_history_retention_pruning() {
        let mut conn = test_conn();
        set_backup_preference(&conn, true, 30);

        // First cycle: scheduled at the next 09:00 slot, then delivered.
        let first = evaluate_backup_reminders(&mut conn, utc_datetime(2026, 10, 5, 10, 0))
            .expect("first cycle");
        assert_eq!(first.len(), 1);
        crate::repos::notification_repo::record_delivery_result(
            &conn,
            &first[0].id,
            Some(utc_datetime(2026, 10, 6, 9, 30)),
            utc_datetime(2026, 10, 6, 9, 30),
        )
        .expect("record the delivery");

        let cycle_key = first[0].cycle_key.clone().expect("cycle key");
        let durable_before: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM notification_delivery_dedupe
                 WHERE workspace_id = 'w1' AND event_key = ?1",
                [format!("backup:{cycle_key}")],
                |row| row.get(0),
            )
            .expect("count the claimed cycle key");
        assert_eq!(
            durable_before, 1,
            "every backup delivery claims its durable cycle key"
        );

        // More than seven days pass; the strict retention cutoff removes the
        // delivered terminal row, which was the only delivery anchor row.
        let prune_now = utc_datetime(2026, 11, 3, 10, 0);
        let pruned = crate::repos::notification_repo::prune_expired_history(&conn, prune_now)
            .expect("run retention cleanup");
        assert_eq!(pruned, 1, "the delivered backup row is retention-eligible");
        assert!(
            crate::repos::notification_repo::latest_backup_delivery(&conn, "w1")
                .expect("latest backup delivery")
                .is_none(),
            "the delivered backup row is gone"
        );

        // Re-evaluating while the recorded cycle is not due must not recreate
        // its delivery, and must not treat the pruned workspace as an initial
        // cycle due at the next 09:00 slot.
        let after_prune = evaluate_backup_reminders(&mut conn, utc_datetime(2026, 11, 3, 10, 0))
            .expect("evaluate after pruning");
        assert!(
            after_prune.is_empty(),
            "polling/reopening must not recreate a pruned cycle's delivery"
        );

        // Once the anchored next cycle is truly due, delivery proceeds
        // normally from the durable record, with its own new cycle key.
        let next = evaluate_backup_reminders(&mut conn, utc_datetime(2026, 11, 5, 10, 0))
            .expect("evaluate the next cycle");
        assert_eq!(next.len(), 1);
        assert_eq!(next[0].scheduled_at, utc_datetime(2026, 11, 5, 9, 0));
        assert_eq!(
            next[0].cycle_key.as_deref(),
            Some("2026-11-05T09:00:00Z"),
            "the due cycle uses its own key, not a duplicate of the pruned one"
        );
    }

    #[test]
    fn backup_reminder_next_cycle_is_interval_based_from_last_effective_slot() {
        let mut conn = test_conn();
        set_backup_preference(&conn, true, 7);
        // First cycle establishes the anchor at 2026-10-06T09:00.
        let first = evaluate_backup_reminders(&mut conn, utc_datetime(2026, 10, 5, 10, 0))
            .expect("first cycle");
        assert_eq!(first[0].scheduled_at, utc_datetime(2026, 10, 6, 9, 0));

        // Next cycle is 7 days after the effective slot: 2026-10-13T09:00.
        // Not due yet on 2026-10-12.
        let before = evaluate_backup_reminders(&mut conn, utc_datetime(2026, 10, 12, 10, 0))
            .expect("evaluate before next due");
        assert!(before.is_empty());

        // Due on 2026-10-13; first evaluated later the same day outside quiet
        // hours, so it persists today's 09:00 slot.
        let second = evaluate_backup_reminders(&mut conn, utc_datetime(2026, 10, 13, 10, 0))
            .expect("second cycle");
        assert_eq!(second.len(), 1);
        assert_eq!(second[0].scheduled_at, utc_datetime(2026, 10, 13, 9, 0));
        assert_eq!(second[0].cycle_key.as_deref(), Some("2026-10-13T09:00:00Z"));
    }

    #[test]
    fn backup_reminder_does_not_backfill_missed_cycles() {
        let mut conn = test_conn();
        set_backup_preference(&conn, true, 7);
        // Anchor at 2026-10-06T09:00. By 2026-10-27 several cycles were
        // missed; only the current due cycle is scheduled, at today's 09:00.
        let _first = evaluate_backup_reminders(&mut conn, utc_datetime(2026, 10, 5, 10, 0))
            .expect("first cycle");

        let scheduled = evaluate_backup_reminders(&mut conn, utc_datetime(2026, 10, 27, 10, 0))
            .expect("evaluate");
        assert_eq!(
            scheduled.len(),
            1,
            "only the current due cycle is scheduled"
        );
        assert_eq!(scheduled[0].scheduled_at, utc_datetime(2026, 10, 27, 9, 0));
        let deliveries = backup_deliveries(&conn);
        assert_eq!(deliveries.len(), 2, "no missed cycles are replayed");
    }

    #[test]
    fn backup_reminder_repeat_evaluation_does_not_duplicate_a_cycle() {
        let mut conn = test_conn();
        set_backup_preference(&conn, true, 7);
        let now = utc_datetime(2026, 10, 5, 10, 0);

        let first = evaluate_backup_reminders(&mut conn, now).expect("first");
        let second = evaluate_backup_reminders(&mut conn, now).expect("second");
        assert_eq!(first.len(), 1);
        assert!(
            second.is_empty(),
            "repeat evaluation is suppressed by cycle dedupe"
        );
        assert_eq!(backup_deliveries(&conn).len(), 1);
    }

    #[test]
    fn backup_reminder_cycle_dedupe_is_satisfied_by_any_delivery_status() {
        let mut conn = test_conn();
        set_backup_preference(&conn, true, 7);
        let now = utc_datetime(2026, 10, 5, 10, 0);
        let first = evaluate_backup_reminders(&mut conn, now).expect("first");
        assert_eq!(first.len(), 1);

        // Mark the delivery failed; the cycle is still satisfied.
        conn.execute(
            "UPDATE notification_deliveries SET status = 'failed' WHERE id = ?1",
            [&first[0].id],
        )
        .expect("mark failed");
        let after_failed =
            evaluate_backup_reminders(&mut conn, now).expect("evaluate after failed");
        assert!(
            after_failed.is_empty(),
            "a failed row satisfies the cycle dedupe"
        );

        // A dismissed row also satisfies the dedupe.
        conn.execute(
            "UPDATE notification_deliveries SET status = 'dismissed' WHERE id = ?1",
            [&first[0].id],
        )
        .expect("mark dismissed");
        let after_dismissed =
            evaluate_backup_reminders(&mut conn, now).expect("evaluate after dismissed");
        assert!(after_dismissed.is_empty());
    }

    #[test]
    fn backup_reminder_is_scoped_to_the_enabled_workspace() {
        let mut conn = test_conn();
        set_backup_preference(&conn, true, 7);
        // A second workspace with the preference explicitly disabled.
        conn.execute(
            "INSERT INTO workspaces (id, name, created_at, updated_at)
             VALUES ('w2', 'Other', '2026-10-05T00:00:00Z', '2026-10-05T00:00:00Z')",
            [],
        )
        .expect("insert second workspace");
        conn.execute(
            "UPDATE workspaces SET settings_json =
                 '{\"backup_reminder\":{\"enabled\":false,\"interval_days\":7}}'
             WHERE id = 'w2'",
            [],
        )
        .expect("disable second workspace backup preference");

        let scheduled = evaluate_backup_reminders(&mut conn, utc_datetime(2026, 10, 5, 10, 0))
            .expect("evaluate");
        assert_eq!(scheduled.len(), 1);
        assert_eq!(scheduled[0].workspace_id, "w1");
    }

    #[test]
    fn backup_reminder_disabled_preference_does_not_schedule() {
        let mut conn = test_conn();
        set_backup_preference(&conn, false, 7);
        let scheduled = evaluate_backup_reminders(&mut conn, utc_datetime(2026, 10, 5, 10, 0))
            .expect("evaluate");
        assert!(scheduled.is_empty());
    }

    #[test]
    fn backup_reminder_disable_and_reenable_preserves_history_and_anchor() {
        let mut conn = test_conn();
        set_backup_preference(&conn, true, 7);
        let first =
            evaluate_backup_reminders(&mut conn, utc_datetime(2026, 10, 5, 10, 0)).expect("first");
        assert_eq!(first[0].scheduled_at, utc_datetime(2026, 10, 6, 9, 0));

        // Disable: history is preserved and no new delivery is scheduled.
        set_backup_preference(&conn, false, 7);
        let while_disabled =
            evaluate_backup_reminders(&mut conn, utc_datetime(2026, 10, 13, 10, 0))
                .expect("evaluate while disabled");
        assert!(while_disabled.is_empty());
        assert_eq!(backup_deliveries(&conn).len(), 1, "history preserved");

        // Re-enable: the anchor is the last recorded slot (2026-10-06T09:00),
        // so the next cycle is 2026-10-13T09:00, not a reset.
        set_backup_preference(&conn, true, 7);
        let after_reenable =
            evaluate_backup_reminders(&mut conn, utc_datetime(2026, 10, 13, 10, 0))
                .expect("evaluate after re-enable");
        assert_eq!(after_reenable.len(), 1);
        assert_eq!(
            after_reenable[0].scheduled_at,
            utc_datetime(2026, 10, 13, 9, 0)
        );
    }

    #[test]
    fn backup_reminder_interval_change_affects_only_the_next_cycle() {
        let mut conn = test_conn();
        set_backup_preference(&conn, true, 7);
        let first =
            evaluate_backup_reminders(&mut conn, utc_datetime(2026, 10, 5, 10, 0)).expect("first");
        assert_eq!(first[0].scheduled_at, utc_datetime(2026, 10, 6, 9, 0));

        // Change the interval to 14 days; the next cycle uses the new interval
        // from the last recorded slot.
        set_backup_preference(&conn, true, 14);
        let second = evaluate_backup_reminders(&mut conn, utc_datetime(2026, 10, 20, 10, 0))
            .expect("second cycle");
        assert_eq!(second.len(), 1);
        assert_eq!(second[0].scheduled_at, utc_datetime(2026, 10, 20, 9, 0));
    }

    #[test]
    fn backup_reminder_newer_validated_backup_takes_precedence_over_delivery_anchor() {
        let mut conn = test_conn();
        set_backup_preference(&conn, true, 7);
        // First cycle establishes a delivery at 2026-10-01T09:00.
        let first = evaluate_backup_reminders(&mut conn, utc_datetime(2026, 10, 1, 8, 0))
            .expect("first cycle");
        assert_eq!(first[0].scheduled_at, utc_datetime(2026, 10, 1, 9, 0));

        // A validated backup on 2026-10-02 is newer than that delivery, so it
        // takes precedence as the cycle anchor: 10-02 UTC date + 7 days.
        record_backup(&conn, "2026-10-02T15:30:00Z");

        let before = evaluate_backup_reminders(&mut conn, utc_datetime(2026, 10, 9, 8, 0))
            .expect("evaluate before due");
        assert!(before.is_empty());

        let scheduled = evaluate_backup_reminders(&mut conn, utc_datetime(2026, 10, 9, 10, 0))
            .expect("evaluate after due");
        assert_eq!(scheduled.len(), 1);
        assert_eq!(scheduled[0].scheduled_at, utc_datetime(2026, 10, 9, 9, 0));
        assert_eq!(
            scheduled[0].cycle_key.as_deref(),
            Some("2026-10-09T09:00:00Z")
        );
    }

    #[test]
    fn backup_reminder_delivery_newer_than_backup_continues_from_effective_slot() {
        let mut conn = test_conn();
        set_backup_preference(&conn, true, 7);
        record_backup(&conn, "2026-10-01T15:30:00Z");
        // First cycle from the backup: 10-01 UTC date + 7 = 10-08T09:00.
        let first = evaluate_backup_reminders(&mut conn, utc_datetime(2026, 10, 8, 10, 0))
            .expect("first cycle");
        assert_eq!(first[0].scheduled_at, utc_datetime(2026, 10, 8, 9, 0));

        // The delivery (10-08) is now newer than the backup (10-01), so the
        // next cycle continues from its effective slot: 10-15T09:00.
        let before = evaluate_backup_reminders(&mut conn, utc_datetime(2026, 10, 15, 8, 0))
            .expect("evaluate before due");
        assert!(before.is_empty());

        let second = evaluate_backup_reminders(&mut conn, utc_datetime(2026, 10, 15, 10, 0))
            .expect("second cycle");
        assert_eq!(second.len(), 1);
        assert_eq!(second[0].scheduled_at, utc_datetime(2026, 10, 15, 9, 0));
        assert_eq!(second[0].cycle_key.as_deref(), Some("2026-10-15T09:00:00Z"));
    }

    #[test]
    fn backup_reminder_anchored_cycle_in_quiet_hours_defers_to_next_0900_slot() {
        let mut conn = test_conn();
        set_backup_preference(&conn, true, 7);
        record_backup(&conn, "2026-10-01T15:30:00Z");
        // Next cycle 10-08T09:00 is due. At 10-08T23:00 (quiet) it defers to
        // tomorrow 09:00 rather than delivering inside quiet hours.
        let scheduled = evaluate_backup_reminders(&mut conn, utc_datetime(2026, 10, 8, 23, 0))
            .expect("evaluate in quiet hours");
        assert_eq!(scheduled.len(), 1);
        assert_eq!(scheduled[0].scheduled_at, utc_datetime(2026, 10, 9, 9, 0));
        assert_eq!(
            scheduled[0].cycle_key.as_deref(),
            Some("2026-10-09T09:00:00Z")
        );
    }

    #[test]
    fn backup_reminder_overdue_cycle_in_early_quiet_hours_defers_to_same_day_0900() {
        let mut conn = test_conn();
        set_backup_preference(&conn, true, 7);
        record_backup(&conn, "2026-10-01T15:30:00Z");
        // Next cycle 10-08T09:00 was missed. At 10-09T06:00 (quiet, before
        // 09:00) the overdue cycle defers to today's 09:00, not tomorrow.
        let scheduled = evaluate_backup_reminders(&mut conn, utc_datetime(2026, 10, 9, 6, 0))
            .expect("evaluate overdue in early quiet hours");
        assert_eq!(scheduled.len(), 1);
        assert_eq!(scheduled[0].scheduled_at, utc_datetime(2026, 10, 9, 9, 0));
        assert_eq!(
            scheduled[0].cycle_key.as_deref(),
            Some("2026-10-09T09:00:00Z")
        );
    }

    #[test]
    fn backup_reminder_does_not_expose_content_or_change_task_status() {
        let mut conn = test_conn();
        set_backup_preference(&conn, true, 7);
        conn.execute(
            "INSERT INTO tasks (id, workspace_id, title, status, created_at, updated_at)
             VALUES ('task-1', 'w1', 'Private task', 'in_progress',
                     '2026-10-05T00:00:00Z', '2026-10-05T00:00:00Z')",
            [],
        )
        .expect("insert task");

        let scheduled = evaluate_backup_reminders(&mut conn, utc_datetime(2026, 10, 5, 10, 0))
            .expect("evaluate");
        assert_eq!(scheduled.len(), 1);
        assert_eq!(scheduled[0].message, "A validated backup is due.");
        assert!(!scheduled[0].message.contains("Private task"));
        assert_eq!(scheduled[0].target, None);

        let status: String = conn
            .query_row("SELECT status FROM tasks WHERE id = 'task-1'", [], |row| {
                row.get(0)
            })
            .expect("read task status");
        assert_eq!(status, "in_progress");
    }
}
