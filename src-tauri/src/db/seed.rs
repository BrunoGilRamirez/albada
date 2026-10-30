//! Application-level seeding of safe default data (Phase 8.0).
//!
//! Seeding runs below the command boundary, from the established workspace
//! initialization path (`SqliteWorkspaceRepo::ensure_default`), because the
//! default workspace is created lazily and does not exist at migration time
//! while `reminder_rules.workspace_id` is a NOT NULL foreign key. There is no
//! second DB owner: this module only inserts rows through the single SQLite
//! connection owned by `Db`.

use rusqlite::{params, Connection};

use crate::error::AppError;
use crate::{id, time};

/// The five default reminder rules seeded per workspace: `(type,
/// privacy_safe_message)`. All other columns take the schema defaults
/// (`enabled = 1`, `max_reminders_per_day = 5`, snooze `[10,30,60]`,
/// `channel = 'desktop'`, `escalation = 'none'`). Every enabled default uses
/// the default profile (`09:00`, quiet hours `22:00`–`07:00`); recurrence and
/// time windows stay NULL until configured. Messages are lock-screen safe and
/// never reference task titles, journal excerpts, or therapy content.
pub const DEFAULT_REMINDER_RULES: [(&str, &str); 5] =
    crate::domain::notifications::DEFAULT_REMINDER_MESSAGES;

/// Seed the default reminder rules for `workspace_id`, idempotently.
///
/// Relies on the unique index `idx_reminder_rules_workspace_type`: re-running
/// after a rule already exists for a type is a no-op
/// (`ON CONFLICT ... DO NOTHING`), so user edits are never overwritten. This
/// also covers a workspace that exists before default-rule seeding runs.
pub fn seed_default_reminder_rules(conn: &Connection, workspace_id: &str) -> Result<(), AppError> {
    let now = time::now_rfc3339();
    for (rule_type, message) in DEFAULT_REMINDER_RULES {
        conn.execute(
            "INSERT INTO reminder_rules (
                 id, workspace_id, type, preferred_time, quiet_hours_start,
                 quiet_hours_end, privacy_safe_message, created_at, updated_at
             )
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?8)
             ON CONFLICT (workspace_id, type) DO NOTHING",
            params![
                id::new_uuid(),
                workspace_id,
                rule_type,
                crate::domain::notifications::DEFAULT_PREFERRED_NOTIFICATION_TIME,
                crate::domain::notifications::DEFAULT_QUIET_HOURS_START,
                crate::domain::notifications::DEFAULT_QUIET_HOURS_END,
                message,
                now,
            ],
        )?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils;

    fn rule_count(conn: &Connection, workspace_id: &str) -> i64 {
        conn.query_row(
            "SELECT COUNT(*) FROM reminder_rules WHERE workspace_id = ?1",
            [workspace_id],
            |row| row.get(0),
        )
        .expect("count reminder rules")
    }

    #[test]
    fn seed_creates_five_default_rules_with_safe_messages() {
        let conn = test_utils::test_conn();
        seed_default_reminder_rules(&conn, "w1").expect("seed defaults");

        assert_eq!(rule_count(&conn, "w1"), 5);
        for (rule_type, message) in DEFAULT_REMINDER_RULES {
            let stored: String = conn
                .query_row(
                    "SELECT privacy_safe_message FROM reminder_rules
                     WHERE workspace_id = 'w1' AND type = ?1",
                    [rule_type],
                    |row| row.get(0),
                )
                .unwrap_or_else(|_| panic!("expected seeded rule type '{rule_type}'"));
            assert_eq!(stored, message);

            let profile: (Option<String>, Option<String>, Option<String>) = conn
                .query_row(
                    "SELECT preferred_time, quiet_hours_start, quiet_hours_end
                     FROM reminder_rules WHERE workspace_id = 'w1' AND type = ?1",
                    [rule_type],
                    |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
                )
                .expect("read seeded notification profile");
            assert_eq!(
                profile,
                (
                    Some("09:00".into()),
                    Some("22:00".into()),
                    Some("07:00".into())
                ),
                "default rule '{rule_type}' uses the default notification profile"
            );
        }
    }

    #[test]
    fn seed_applies_schema_defaults() {
        let conn = test_utils::test_conn();
        seed_default_reminder_rules(&conn, "w1").expect("seed defaults");

        let (enabled, max_per_day, snooze_json, channel, escalation): (
            i64,
            i64,
            String,
            String,
            String,
        ) = conn
            .query_row(
                "SELECT enabled, max_reminders_per_day, snooze_minutes_json,
                        channel, escalation
                 FROM reminder_rules WHERE workspace_id = 'w1' AND type = 'planning'",
                [],
                |row| {
                    Ok((
                        row.get(0)?,
                        row.get(1)?,
                        row.get(2)?,
                        row.get(3)?,
                        row.get(4)?,
                    ))
                },
            )
            .expect("read seeded planning rule");
        assert_eq!(enabled, 1);
        assert_eq!(max_per_day, 5);
        assert_eq!(snooze_json, "[10,30,60]");
        assert_eq!(channel, "desktop");
        assert_eq!(escalation, "none");

        let profile: (Option<String>, Option<String>, Option<String>) = conn
            .query_row(
                "SELECT preferred_time, quiet_hours_start, quiet_hours_end
                 FROM reminder_rules WHERE workspace_id = 'w1' AND type = 'planning'",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .expect("read seeded notification profile");
        assert_eq!(
            profile,
            (
                Some("09:00".into()),
                Some("22:00".into()),
                Some("07:00".into())
            )
        );

        let deadline_window: i64 = conn
            .query_row(
                "SELECT deadline_notification_window_days FROM reminder_rules
                 WHERE workspace_id = 'w1' AND type = 'deadline'",
                [],
                |row| row.get(0),
            )
            .expect("read deadline notification default");
        assert_eq!(deadline_window, 3);
    }

    #[test]
    fn seed_is_idempotent_and_never_overwrites_user_edits() {
        let conn = test_utils::test_conn();
        seed_default_reminder_rules(&conn, "w1").expect("seed once");

        // Simulate a user editing one rule.
        conn.execute(
            "UPDATE reminder_rules
             SET privacy_safe_message = 'My own message', enabled = 0,
                 preferred_time = '08:45', quiet_hours_start = '23:00',
                 quiet_hours_end = '06:00'
             WHERE workspace_id = 'w1' AND type = 'focus'",
            [],
        )
        .expect("user edit");

        seed_default_reminder_rules(&conn, "w1").expect("seed again");
        seed_default_reminder_rules(&conn, "w1").expect("seed a third time");

        assert_eq!(rule_count(&conn, "w1"), 5, "no duplicate rules");
        let (message, enabled): (String, i64) = conn
            .query_row(
                "SELECT privacy_safe_message, enabled FROM reminder_rules
                 WHERE workspace_id = 'w1' AND type = 'focus'",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .expect("read edited focus rule");
        assert_eq!(message, "My own message", "user edit preserved");
        assert_eq!(enabled, 0, "user toggle preserved");
        let profile: (Option<String>, Option<String>, Option<String>) = conn
            .query_row(
                "SELECT preferred_time, quiet_hours_start, quiet_hours_end
                 FROM reminder_rules WHERE workspace_id = 'w1' AND type = 'focus'",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .expect("read user-configured focus profile");
        assert_eq!(
            profile,
            (
                Some("08:45".into()),
                Some("23:00".into()),
                Some("06:00".into())
            )
        );
    }

    #[test]
    fn workspace_deletion_cascades_to_rules_and_deliveries() {
        let conn = test_utils::test_conn();
        seed_default_reminder_rules(&conn, "w1").expect("seed defaults");

        let rule_id: String = conn
            .query_row(
                "SELECT id FROM reminder_rules WHERE workspace_id = 'w1' AND type = 'review'",
                [],
                |row| row.get(0),
            )
            .expect("read review rule id");
        let now = time::now_rfc3339();
        conn.execute(
            "INSERT INTO notification_deliveries (
                 id, workspace_id, delivery_kind, reminder_rule_id, scheduled_at,
                 status, channel, safe_message, created_at
             )
             VALUES ('d1', 'w1', 'reminder_rule', ?1, ?2, 'scheduled', 'desktop',
                     'Weekly review time', ?2)",
            params![rule_id, now],
        )
        .expect("insert delivery");
        conn.execute(
            "INSERT INTO notification_deliveries (
                 id, workspace_id, delivery_kind, scheduled_at, status, channel,
                 safe_message, created_at
             ) VALUES ('d2', 'w1', 'backup', ?1, 'scheduled', 'desktop',
                       'A generic safe reminder', ?1)",
            [&now],
        )
        .expect("insert workspace-scoped backup delivery");

        conn.execute("DELETE FROM workspaces WHERE id = 'w1'", [])
            .expect("delete workspace");

        let rules: i64 = conn
            .query_row("SELECT COUNT(*) FROM reminder_rules", [], |row| row.get(0))
            .expect("count rules");
        assert_eq!(rules, 0, "rules cascade-deleted with the workspace");
        let deliveries: i64 = conn
            .query_row("SELECT COUNT(*) FROM notification_deliveries", [], |row| {
                row.get(0)
            })
            .expect("count deliveries");
        assert_eq!(deliveries, 0, "deliveries cascade-deleted with their rule");
    }
}
