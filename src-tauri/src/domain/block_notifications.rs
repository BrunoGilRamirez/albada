//! Block start, boundary, and overrun notification scheduling (Phase 8.7.3).
//!
//! Block events are workspace-scoped notification deliveries with no reminder
//! rule: they use the `block` delivery kind, so they never consume or inherit
//! reminder-rule daily-cap accounting (blocker B4 stays open). Every event is
//! durably deduplicated through `notification_delivery_dedupe`, so poll
//! repeats, app restarts, history clearing, and pruning never replay a
//! claimed event. Suppressed (quiet-hours or disabled-rule) events still consume
//! their claim so they never fire later.
//!
//! Only today's existing blocks are read (`list_between` with `from == to`);
//! recurrences are never materialized here.

use chrono::{DateTime, Utc};

use crate::domain::models::Block;
use crate::domain::notifications::{self, NotificationDelivery};
use crate::domain::settings;
use crate::domain::types::BlockStatus;
use crate::error::AppError;
use crate::repos::block_recurrence_repo;
use crate::repos::block_repo::{BlockRepo, SqliteBlockRepo};
use crate::repos::notification_repo;
use crate::repos::workspace_repo::{SqliteWorkspaceRepo, WorkspaceRepo};

/// Evaluate block notifications for every workspace, without a runtime locale
/// override (workspaces in `system` mode fall back to English).
pub fn evaluate_block_notifications(
    conn: &mut rusqlite::Connection,
    now: DateTime<Utc>,
) -> Result<Vec<NotificationDelivery>, AppError> {
    evaluate_block_notifications_with_locale(conn, now, None)
}

/// Evaluate block notifications using the same workspace/runtime locale
/// selection as reminder rules. Previously scheduled history is unchanged.
pub fn evaluate_block_notifications_with_locale(
    conn: &mut rusqlite::Connection,
    now: DateTime<Utc>,
    runtime_locale: Option<&str>,
) -> Result<Vec<NotificationDelivery>, AppError> {
    let tx = conn.transaction()?;
    let workspaces = SqliteWorkspaceRepo.list(&tx)?;
    let mut scheduled = Vec::new();

    for workspace in workspaces {
        let timezone = block_recurrence_repo::parse_timezone(&workspace.timezone)?;
        let local_today = now
            .with_timezone(&timezone)
            .date_naive()
            .format("%Y-%m-%d")
            .to_string();

        // Read-only listing of today's existing blocks; never materialize.
        let blocks = BlockRepo::list_between(
            &SqliteBlockRepo,
            &tx,
            &workspace.id,
            &local_today,
            &local_today,
        )?;

        // Without a focus rule there is no quiet-hours or enabled gate to
        // apply, so the workspace is skipped.
        let Some(focus_rule) =
            notification_repo::focus_rule_for_workspace(&tx, &workspace.id, now)?
        else {
            continue;
        };

        let overrun_interval_minutes =
            settings::load_block_notification_preference(&tx, &workspace.id)?
                .overrun_interval_minutes;
        let locale = notifications::resolve_notification_locale(&workspace.locale, runtime_locale);

        for block in &blocks {
            if block.status == BlockStatus::Planned {
                let Some(start_instant) = resolve_block_start_instant(block, timezone) else {
                    continue;
                };
                if start_instant > now {
                    continue;
                }
                let event_key = format!("block:{}:start", block.id);
                if notifications::is_quiet_at(&focus_rule, start_instant) || !focus_rule.enabled {
                    // Record-only claim: a suppressed due start never replays.
                    notification_repo::claim_event_key(&tx, &workspace.id, &event_key, now)?;
                } else if notification_repo::claim_event_key(&tx, &workspace.id, &event_key, now)? {
                    let delivery = notification_repo::insert_block_event_delivery(
                        &tx,
                        &workspace.id,
                        &block.id,
                        start_instant,
                        start_message(locale),
                        now,
                    )?;
                    scheduled.push(delivery);
                }
                continue;
            }

            if block.status != BlockStatus::Active {
                // completed/skipped/cancelled: pending block events were
                // terminated by the block repository; nothing new is due.
                continue;
            }

            let Some(actual_start_at) = parse_instant(block.actual_start_at.as_deref()) else {
                continue;
            };
            let Some(duration_secs) = block.estimated_duration_seconds else {
                // Start event only; never boundary or overrun.
                continue;
            };

            let boundary = actual_start_at + chrono::Duration::seconds(duration_secs);

            if boundary <= now {
                let event_key = format!("block:{}:boundary", block.id);
                let conflict =
                    later_planned_block_conflict(&blocks, block, actual_start_at, timezone, now);
                if notifications::is_quiet_at(&focus_rule, boundary) || !focus_rule.enabled {
                    notification_repo::claim_event_key(&tx, &workspace.id, &event_key, now)?;
                } else if notification_repo::claim_event_key(&tx, &workspace.id, &event_key, now)? {
                    let mut message = boundary_message(locale).to_string();
                    if conflict {
                        message.push_str(conflict_suffix(locale));
                    }
                    let delivery = notification_repo::insert_block_event_delivery(
                        &tx,
                        &workspace.id,
                        &block.id,
                        boundary,
                        &message,
                        now,
                    )?;
                    scheduled.push(delivery);
                }
            }

            // Overrun forward scheduling. Only once the block has actually
            // reached its estimated duration does an overrun exist, and the
            // interval math never produces a slot in the past.
            if boundary > now {
                continue;
            }
            let elapsed_secs = (now - actual_start_at).num_seconds().max(0);
            let interval_secs = i64::from(overrun_interval_minutes) * 60;
            let first_overrun_number = duration_secs / interval_secs + 1;
            let interval_number = (elapsed_secs / interval_secs + 1).max(first_overrun_number);
            let slot = actual_start_at + chrono::Duration::seconds(interval_number * interval_secs);
            let slot_local_date = slot
                .with_timezone(&timezone)
                .date_naive()
                .format("%Y-%m-%d")
                .to_string();
            if slot > now
                && slot_local_date == local_today
                && focus_rule.enabled
                && !notifications::is_quiet_at(&focus_rule, slot)
            {
                let event_key = format!("block:{}:overrun:{}", block.id, interval_number);
                if notification_repo::claim_event_key(&tx, &workspace.id, &event_key, now)? {
                    let mut message = overrun_message(locale).to_string();
                    if later_planned_block_conflict(&blocks, block, actual_start_at, timezone, now)
                    {
                        message.push_str(conflict_suffix(locale));
                    }
                    let delivery = notification_repo::insert_block_event_delivery(
                        &tx,
                        &workspace.id,
                        &block.id,
                        slot,
                        &message,
                        now,
                    )?;
                    scheduled.push(delivery);
                }
            }
        }
    }

    tx.commit()?;
    Ok(scheduled)
}

/// Parse an optional stored RFC3339 instant, treating a missing or malformed
/// value as absent so the poll skips the block instead of failing.
fn parse_instant(value: Option<&str>) -> Option<DateTime<Utc>> {
    let value = value?;
    DateTime::parse_from_rfc3339(value)
        .map(|timestamp| timestamp.with_timezone(&Utc))
        .ok()
}

/// Resolve a block's exact scheduled start instant in UTC. Recurring
/// occurrences use their materialized snapshot; one-off blocks resolve their
/// `HH:MM` local start in the workspace timezone. Blocks without a scheduled
/// start, a malformed snapshot, or a DST-gap start yield `None` so the poll
/// skips that block instead of failing.
fn resolve_block_start_instant(block: &Block, timezone: chrono_tz::Tz) -> Option<DateTime<Utc>> {
    if block.recurrence_id.is_some() {
        return parse_instant(block.resolved_start_at.as_deref());
    }
    let planned_start = block.planned_start_at.as_deref()?;
    let date = crate::time::parse_date(&block.date).ok()?;
    match block_recurrence_repo::resolve_local_start(&date, planned_start, timezone) {
        Ok(resolved) => DateTime::parse_from_rfc3339(&resolved)
            .map(|timestamp| timestamp.with_timezone(&Utc))
            .ok(),
        Err(_) => None,
    }
}

/// Settled conflict predicate (Phase 8.7.3): a conflict exists when a
/// different block in the same workspace and same local date, with status
/// `planned`, has an exact resolved start strictly later than the active
/// block's actual start, and `now` is at or past that later start. One-off and
/// recurring rows qualify equally; untimed planned blocks never qualify.
fn later_planned_block_conflict(
    blocks: &[Block],
    active: &Block,
    actual_start: DateTime<Utc>,
    timezone: chrono_tz::Tz,
    now: DateTime<Utc>,
) -> bool {
    blocks.iter().any(|candidate| {
        if candidate.id == active.id || candidate.status != BlockStatus::Planned {
            return false;
        }
        match resolve_block_start_instant(candidate, timezone) {
            Some(candidate_start) => candidate_start > actual_start && now >= candidate_start,
            None => false,
        }
    })
}

fn start_message(locale: &str) -> &'static str {
    match locale {
        "es-ES" => "Un bloque planificado empieza ahora.",
        "de-DE" => "Ein geplanter Block beginnt jetzt.",
        "fr-FR" => "Un bloc planifié commence maintenant.",
        _ => "A planned block starts now.",
    }
}

fn boundary_message(locale: &str) -> &'static str {
    match locale {
        "es-ES" => "Un bloque de concentración alcanzó su duración estimada.",
        "de-DE" => "Ein Fokusblock hat seine geschätzte Dauer erreicht.",
        "fr-FR" => "Un bloc de concentration a atteint sa durée estimée.",
        _ => "A focus block reached its estimated duration.",
    }
}

fn overrun_message(locale: &str) -> &'static str {
    match locale {
        "es-ES" => "Un bloque de concentración sigue activo más allá de su estimación.",
        "de-DE" => "Ein Fokusblock ist weiterhin über die Schätzung hinaus aktiv.",
        "fr-FR" => "Un bloc de concentration est toujours actif au-delà de son estimation.",
        _ => "A focus block is still active beyond its estimate.",
    }
}

fn conflict_suffix(locale: &str) -> &'static str {
    match locale {
        "es-ES" => " Un bloque planificado posterior puede solaparse.",
        "de-DE" => " Ein später geplanter Block kann kollidieren.",
        "fr-FR" => " Un bloc planifié ultérieur peut entrer en conflit.",
        _ => " A later planned block may conflict.",
    }
}

#[cfg(test)]
mod tests {
    use chrono::TimeZone;
    use rusqlite::params;

    use super::*;
    use crate::domain::notifications::{
        NotificationChannel, NotificationDeliveryKind, NotificationDeliveryStatus,
        NotificationTarget, NotificationTargetType,
    };
    use crate::repos::block_repo;
    use crate::test_utils::test_conn;

    // Each test states nothing about personal content: fixtures use
    // synthetic workspaces, rules, and blocks only.

    const START_MESSAGE: &str = "A planned block starts now.";
    const BOUNDARY_MESSAGE: &str = "A focus block reached its estimated duration.";
    const OVERRUN_MESSAGE: &str = "A focus block is still active beyond its estimate.";
    const CONFLICT_SUFFIX: &str = " A later planned block may conflict.";
    const COMPLETION_REASON: &str = "The block ended before this notification was due.";

    fn utc_time(hour: u32, minute: u32) -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 10, 5, hour, minute, 0)
            .single()
            .expect("valid UTC test datetime")
    }

    fn next_morning(hour: u32, minute: u32) -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 10, 6, hour, minute, 0)
            .single()
            .expect("valid UTC test datetime")
    }

    fn set_utc_timezone(conn: &rusqlite::Connection) {
        conn.execute("UPDATE workspaces SET timezone = 'UTC' WHERE id = 'w1'", [])
            .expect("set workspace timezone to UTC");
    }

    /// Seed the workspace focus rule. `quiet_hours` is an optional `(start,
    /// end)` pair of `HH:MM` strings; `None` stores NULLs so no time of day is
    /// quiet.
    fn seed_focus_rule(
        conn: &rusqlite::Connection,
        enabled: bool,
        quiet_hours: Option<(&str, &str)>,
    ) {
        let (quiet_start, quiet_end) = quiet_hours
            .map(|(start, end)| (Some(start.to_string()), Some(end.to_string())))
            .unwrap_or((None, None));
        conn.execute(
            "INSERT INTO reminder_rules (
                id, workspace_id, type, enabled, quiet_hours_start, quiet_hours_end,
                privacy_safe_message, created_at, updated_at
             ) VALUES ('focus-rule', 'w1', 'focus', ?1, ?2, ?3,
                       'Ready for a focus block?', '2026-10-05T00:00:00Z',
                       '2026-10-05T00:00:00Z')",
            params![enabled, quiet_start, quiet_end],
        )
        .expect("seed focus rule");
    }

    fn seed_one_off_block(
        conn: &rusqlite::Connection,
        id: &str,
        status: BlockStatus,
        planned_start_at: Option<&str>,
        estimated_duration_seconds: Option<i64>,
        actual_start_at: Option<&str>,
        label: Option<&str>,
    ) {
        conn.execute(
            "INSERT INTO blocks (
                id, workspace_id, date, type, status, label, planned_start_at,
                estimated_duration_seconds, actual_start_at, flexibility,
                created_at, updated_at
             ) VALUES (?1, 'w1', '2026-10-05', 'focus', ?2, ?3, ?4, ?5, ?6,
                       'flexible', '2026-10-05T00:00:00Z', '2026-10-05T00:00:00Z')",
            params![
                id,
                status.as_str(),
                label,
                planned_start_at,
                estimated_duration_seconds,
                actual_start_at
            ],
        )
        .expect("seed one-off block");
    }

    fn seed_active_block(conn: &rusqlite::Connection, id: &str, actual_start_at: &str) {
        seed_one_off_block(
            conn,
            id,
            BlockStatus::Active,
            Some("09:00"),
            Some(600),
            Some(actual_start_at),
            None,
        );
    }

    fn count_block_deliveries(conn: &rusqlite::Connection, message: &str) -> i64 {
        conn.query_row(
            "SELECT COUNT(*) FROM notification_deliveries
             WHERE delivery_kind = 'block' AND safe_message = ?1",
            [message],
            |row| row.get(0),
        )
        .expect("count block deliveries")
    }

    /// The (scheduled_at, message) rows of block overrun deliveries for a
    /// block, oldest slot first.
    fn overrun_rows(conn: &rusqlite::Connection, block_id: &str) -> Vec<(DateTime<Utc>, String)> {
        conn.prepare(
            "SELECT scheduled_at, safe_message FROM notification_deliveries
             WHERE delivery_kind = 'block' AND target_entity_id = ?1
               AND safe_message IN (?2, ?3)
             ORDER BY scheduled_at",
        )
        .expect("prepare overrun query")
        .query_map(
            params![
                block_id,
                OVERRUN_MESSAGE,
                format!("{OVERRUN_MESSAGE}{CONFLICT_SUFFIX}")
            ],
            |row| {
                let scheduled_at: String = row.get(0)?;
                let message: String = row.get(1)?;
                Ok((scheduled_at, message))
            },
        )
        .expect("query overrun rows")
        .map(|row| row.expect("read overrun row"))
        .map(|(scheduled_at, message)| {
            (
                DateTime::parse_from_rfc3339(&scheduled_at)
                    .expect("valid stored timestamp")
                    .with_timezone(&Utc),
                message,
            )
        })
        .collect()
    }

    #[test]
    fn planned_block_start_reminder_does_not_start_the_block() {
        let mut conn = test_conn();
        set_utc_timezone(&conn);
        seed_focus_rule(&conn, true, None);
        seed_one_off_block(
            &conn,
            "b1",
            BlockStatus::Planned,
            Some("09:00"),
            Some(600),
            None,
            Some("Synthetic focus block"),
        );

        let deliveries = evaluate_block_notifications(&mut conn, utc_time(9, 30))
            .expect("evaluate block notifications");

        assert_eq!(deliveries.len(), 1, "exactly one start delivery");
        let delivery = &deliveries[0];
        assert_eq!(delivery.message, START_MESSAGE);
        assert_eq!(delivery.scheduled_at, utc_time(9, 0));
        assert_eq!(delivery.delivery_kind, NotificationDeliveryKind::Block);
        assert_eq!(delivery.status, NotificationDeliveryStatus::Scheduled);
        assert_eq!(delivery.channel, NotificationChannel::Desktop);
        assert_eq!(delivery.reminder_rule_id, None);
        assert_eq!(delivery.cycle_key, None);
        assert_eq!(delivery.error_message, None);
        assert_eq!(
            delivery.target,
            Some(NotificationTarget {
                entity_type: NotificationTargetType::Block,
                entity_id: "b1".to_string(),
            })
        );

        let status: String = conn
            .query_row("SELECT status FROM blocks WHERE id = 'b1'", [], |row| {
                row.get(0)
            })
            .expect("read block status");
        assert_eq!(status, "planned", "evaluation never starts the block");
    }

    #[test]
    fn repeat_evaluation_and_restart_do_not_duplicate_start_event() {
        let mut conn = test_conn();
        set_utc_timezone(&conn);
        seed_focus_rule(&conn, true, None);
        seed_one_off_block(
            &conn,
            "b1",
            BlockStatus::Planned,
            Some("09:00"),
            Some(600),
            None,
            None,
        );

        let first =
            evaluate_block_notifications(&mut conn, utc_time(9, 30)).expect("first evaluation");
        assert_eq!(first.len(), 1);

        let second = evaluate_block_notifications(&mut conn, utc_time(9, 30))
            .expect("second evaluation at the same now");
        assert!(second.is_empty(), "no duplicate on repeat poll");

        // Durable dedupe survives visible history clearing: no new row.
        conn.execute("DELETE FROM notification_deliveries", [])
            .expect("clear delivery history");
        let after_clear = evaluate_block_notifications(&mut conn, utc_time(9, 30))
            .expect("evaluation after history clearing");
        assert!(after_clear.is_empty(), "claimed event never replays");
        assert_eq!(count_block_deliveries(&conn, START_MESSAGE), 0);
        assert_eq!(
            count_block_deliveries(&conn, BOUNDARY_MESSAGE),
            0,
            "no boundary for a planned block"
        );
    }

    #[test]
    fn quiet_hours_suppress_and_consume_the_start_event_without_replay() {
        let mut conn = test_conn();
        set_utc_timezone(&conn);
        seed_focus_rule(&conn, true, Some(("22:00", "07:00")));
        seed_one_off_block(
            &conn,
            "b1",
            BlockStatus::Planned,
            Some("23:00"),
            Some(600),
            None,
            None,
        );

        let quiet_poll = evaluate_block_notifications(&mut conn, utc_time(23, 30))
            .expect("poll during quiet hours");
        assert!(quiet_poll.is_empty(), "quiet hours suppress the start");
        let claimed: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM notification_delivery_dedupe
                 WHERE workspace_id = 'w1' AND event_key = 'block:b1:start'",
                [],
                |row| row.get(0),
            )
            .expect("count consumed start claim");
        assert_eq!(claimed, 1, "suppressed start consumed its claim");

        let after_quiet = evaluate_block_notifications(&mut conn, next_morning(7, 30))
            .expect("poll after quiet hours end");
        assert!(after_quiet.is_empty(), "suppressed start never replays");
        assert_eq!(count_block_deliveries(&conn, START_MESSAGE), 0);
    }

    #[test]
    fn disabled_focus_rule_suppresses_and_consumes_due_start_events() {
        let mut conn = test_conn();
        set_utc_timezone(&conn);
        seed_focus_rule(&conn, false, None);
        seed_one_off_block(
            &conn,
            "b1",
            BlockStatus::Planned,
            Some("09:00"),
            Some(600),
            None,
            None,
        );

        let disabled_poll = evaluate_block_notifications(&mut conn, utc_time(9, 30))
            .expect("poll with disabled focus rule");
        assert!(disabled_poll.is_empty(), "disabled rule suppresses start");

        // Enabling the rule later must not fire the already-consumed start.
        conn.execute(
            "UPDATE reminder_rules SET enabled = 1 WHERE id = 'focus-rule'",
            [],
        )
        .expect("enable focus rule");
        let enabled_poll = evaluate_block_notifications(&mut conn, utc_time(9, 30))
            .expect("poll after enabling the rule");
        assert!(enabled_poll.is_empty(), "consumed start never replays");
        assert_eq!(count_block_deliveries(&conn, START_MESSAGE), 0);
    }

    #[test]
    fn boundary_uses_persisted_actual_start_and_is_created_once() {
        let mut conn = test_conn();
        set_utc_timezone(&conn);
        seed_focus_rule(&conn, true, None);
        seed_active_block(&conn, "b1", "2026-10-05T09:00:00Z");

        let before = evaluate_block_notifications(&mut conn, utc_time(9, 9))
            .expect("poll before the boundary");
        assert!(before.is_empty(), "boundary not due before 09:10");

        let at_boundary =
            evaluate_block_notifications(&mut conn, utc_time(9, 10)).expect("poll at the boundary");
        let boundaries = at_boundary
            .iter()
            .filter(|delivery| delivery.message == BOUNDARY_MESSAGE)
            .collect::<Vec<_>>();
        assert_eq!(boundaries.len(), 1, "one boundary at 09:10");
        assert_eq!(boundaries[0].scheduled_at, utc_time(9, 10));

        let after = evaluate_block_notifications(&mut conn, utc_time(9, 11))
            .expect("poll after the boundary");
        assert!(
            after
                .iter()
                .all(|delivery| delivery.message != BOUNDARY_MESSAGE),
            "boundary is never recreated"
        );
        assert_eq!(count_block_deliveries(&conn, BOUNDARY_MESSAGE), 1);
    }

    #[test]
    fn missing_duration_still_gets_start_but_never_boundary_or_overrun() {
        let mut conn = test_conn();
        set_utc_timezone(&conn);
        seed_focus_rule(&conn, true, None);
        seed_one_off_block(
            &conn,
            "b1",
            BlockStatus::Planned,
            Some("11:00"),
            None,
            None,
            None,
        );

        let starts = evaluate_block_notifications(&mut conn, utc_time(11, 30))
            .expect("poll for the unestimated planned block");
        assert_eq!(starts.len(), 1, "start event only");
        assert_eq!(starts[0].message, START_MESSAGE);

        seed_one_off_block(
            &conn,
            "b2",
            BlockStatus::Active,
            Some("11:00"),
            None,
            Some("2026-10-05T11:00:00Z"),
            None,
        );
        let active_poll = evaluate_block_notifications(&mut conn, utc_time(11, 30))
            .expect("poll for the unestimated active block");
        assert!(
            active_poll.is_empty(),
            "no boundary or overrun without a duration"
        );
        let much_later = evaluate_block_notifications(&mut conn, utc_time(12, 0))
            .expect("later poll for the unestimated active block");
        assert!(much_later.is_empty(), "never a boundary or overrun");
        assert_eq!(count_block_deliveries(&conn, BOUNDARY_MESSAGE), 0);
        assert_eq!(count_block_deliveries(&conn, OVERRUN_MESSAGE), 0);
    }

    #[test]
    fn overrun_events_follow_the_configured_interval() {
        let mut conn = test_conn();
        set_utc_timezone(&conn);
        seed_focus_rule(&conn, true, None);
        seed_active_block(&conn, "b1", "2026-10-05T09:00:00Z");
        assert_eq!(
            settings::load_block_notification_preference(&conn, "w1")
                .expect("load default interval")
                .overrun_interval_minutes,
            10,
            "default overrun interval is 10 minutes"
        );

        // No overrun before the boundary: poll at 09:05 creates none.
        let early = evaluate_block_notifications(&mut conn, utc_time(9, 5))
            .expect("poll before the boundary");
        assert!(early.is_empty());

        // 09:10: boundary at 09:10 plus one pending overrun (#2) at 09:20.
        let at_boundary =
            evaluate_block_notifications(&mut conn, utc_time(9, 10)).expect("poll at the boundary");
        assert_eq!(at_boundary.len(), 2, "boundary plus pending overrun");
        assert!(at_boundary
            .iter()
            .any(|delivery| delivery.message == BOUNDARY_MESSAGE
                && delivery.scheduled_at == utc_time(9, 10)));
        assert!(at_boundary
            .iter()
            .any(|delivery| delivery.message == OVERRUN_MESSAGE
                && delivery.scheduled_at == utc_time(9, 20)));

        // 09:20: overrun #2 is now due and overrun #3 becomes pending at 09:30.
        let second = evaluate_block_notifications(&mut conn, utc_time(9, 20)).expect("second poll");
        assert_eq!(second.len(), 1, "one new pending overrun");
        assert_eq!(second[0].message, OVERRUN_MESSAGE);
        assert_eq!(second[0].scheduled_at, utc_time(9, 30));
        assert_eq!(
            overrun_rows(&conn, "b1"),
            vec![
                (utc_time(9, 20), OVERRUN_MESSAGE.to_string()),
                (utc_time(9, 30), OVERRUN_MESSAGE.to_string()),
            ],
            "the overrun set created by 09:20 is exactly {{2, 3}}"
        );

        // 09:30: overrun #3 is now due; the overrun set due by 09:30 stays
        // exactly {2, 3} while the next pending slot moves to 09:40.
        let third = evaluate_block_notifications(&mut conn, utc_time(9, 30)).expect("third poll");
        assert_eq!(third.len(), 1);
        assert_eq!(third[0].scheduled_at, utc_time(9, 40));
        let due_overruns = overrun_rows(&conn, "b1")
            .into_iter()
            .filter(|(slot, _)| *slot <= utc_time(9, 30))
            .map(|(slot, _)| slot)
            .collect::<Vec<_>>();
        assert_eq!(
            due_overruns,
            vec![utc_time(9, 20), utc_time(9, 30)],
            "the overrun set due by 09:30 is exactly {{2, 3}}"
        );

        // Repeated polls at the same now never duplicate a row.
        let repeat = evaluate_block_notifications(&mut conn, utc_time(9, 30))
            .expect("repeat poll at the same now");
        assert!(repeat.is_empty());
        assert_eq!(overrun_rows(&conn, "b1").len(), 3, "no duplicates");
    }

    #[test]
    fn overrun_interval_validation_and_default() {
        let conn = test_conn();
        assert_eq!(
            settings::load_block_notification_preference(&conn, "w1")
                .expect("load default preference")
                .overrun_interval_minutes,
            10,
            "default is 10 minutes"
        );
        assert!(
            settings::update_block_notification_preference(&conn, "w1", 0).is_err(),
            "zero is rejected"
        );
        for minutes in [1, 45] {
            let saved = settings::update_block_notification_preference(&conn, "w1", minutes)
                .expect("persist a positive interval");
            assert_eq!(saved.overrun_interval_minutes, minutes);
            assert_eq!(
                settings::load_block_notification_preference(&conn, "w1")
                    .expect("reload the preference")
                    .overrun_interval_minutes,
                minutes,
            );
        }

        // A stored invalid zero falls back to the default instead of failing.
        conn.execute(
            "UPDATE workspaces SET settings_json =
                 '{\"block_notifications\":{\"overrun_interval_minutes\":0}}'
             WHERE id = 'w1'",
            [],
        )
        .expect("seed an invalid stored interval");
        assert_eq!(
            settings::load_block_notification_preference(&conn, "w1")
                .expect("fall back to the default")
                .overrun_interval_minutes,
            10,
        );
    }

    #[test]
    fn conflict_uses_the_settled_predicate_and_stays_lock_screen_safe() {
        let active_with_label = "Synthetic active block";
        let later_with_label = "Synthetic later block";

        // A later planned block whose start has already begun conflicts.
        let mut conn = test_conn();
        set_utc_timezone(&conn);
        seed_focus_rule(&conn, true, None);
        seed_one_off_block(
            &conn,
            "active",
            BlockStatus::Active,
            Some("09:00"),
            Some(600),
            Some("2026-10-05T09:00:00Z"),
            Some(active_with_label),
        );
        seed_one_off_block(
            &conn,
            "later",
            BlockStatus::Planned,
            Some("09:30"),
            Some(600),
            None,
            Some(later_with_label),
        );
        let deliveries = evaluate_block_notifications(&mut conn, utc_time(10, 0))
            .expect("poll for the boundary");
        let boundary = deliveries
            .iter()
            .find(|delivery| delivery.message.starts_with(BOUNDARY_MESSAGE))
            .expect("boundary delivery");
        assert_eq!(
            boundary.message,
            format!("{BOUNDARY_MESSAGE}{CONFLICT_SUFFIX}"),
            "a started later planned block conflicts"
        );
        assert!(
            !boundary.message.contains(active_with_label)
                && !boundary.message.contains(later_with_label),
            "the message stays lock-screen safe and never carries block labels"
        );

        // A later planned block that has not started yet yields no suffix.
        let mut conn = test_conn();
        set_utc_timezone(&conn);
        seed_focus_rule(&conn, true, None);
        seed_one_off_block(
            &conn,
            "active",
            BlockStatus::Active,
            Some("09:00"),
            Some(600),
            Some("2026-10-05T09:00:00Z"),
            None,
        );
        seed_one_off_block(
            &conn,
            "later",
            BlockStatus::Planned,
            Some("10:30"),
            Some(600),
            None,
            None,
        );
        let deliveries = evaluate_block_notifications(&mut conn, utc_time(10, 0))
            .expect("poll for the boundary");
        let boundary = deliveries
            .iter()
            .find(|delivery| delivery.message.starts_with(BOUNDARY_MESSAGE))
            .expect("boundary delivery");
        assert_eq!(
            boundary.message, BOUNDARY_MESSAGE,
            "no suffix while the later block has not started"
        );

        // A later completed block yields no suffix.
        let mut conn = test_conn();
        set_utc_timezone(&conn);
        seed_focus_rule(&conn, true, None);
        seed_one_off_block(
            &conn,
            "active",
            BlockStatus::Active,
            Some("09:00"),
            Some(600),
            Some("2026-10-05T09:00:00Z"),
            None,
        );
        seed_one_off_block(
            &conn,
            "later",
            BlockStatus::Completed,
            Some("09:30"),
            Some(600),
            Some("2026-10-05T09:30:00Z"),
            None,
        );
        let deliveries = evaluate_block_notifications(&mut conn, utc_time(10, 0))
            .expect("poll for the boundary");
        let boundary = deliveries
            .iter()
            .find(|delivery| delivery.message.starts_with(BOUNDARY_MESSAGE))
            .expect("boundary delivery");
        assert_eq!(
            boundary.message, BOUNDARY_MESSAGE,
            "no suffix for a later completed block"
        );
    }

    #[test]
    fn completion_terminates_pending_block_events_and_creates_none_afterwards() {
        let mut conn = test_conn();
        set_utc_timezone(&conn);
        seed_focus_rule(&conn, true, None);
        seed_active_block(&conn, "b1", "2026-10-05T09:00:00Z");

        let deliveries = evaluate_block_notifications(&mut conn, utc_time(9, 10))
            .expect("poll creating the boundary and a pending overrun");
        assert_eq!(deliveries.len(), 2, "boundary plus pending overrun #2");

        let completed = block_repo::complete_block(&conn, "b1").expect("complete the block");
        assert_eq!(completed.status, BlockStatus::Completed);

        let terminated: (String, Option<String>) = conn
            .query_row(
                "SELECT status, error_message FROM notification_deliveries
                 WHERE target_entity_id = 'b1' AND scheduled_at = '2026-10-05T09:20:00Z'",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .expect("read the terminated overrun row");
        assert_eq!(
            terminated,
            ("failed".to_string(), Some(COMPLETION_REASON.to_string())),
            "the pending overrun is failed with the fixed technical reason"
        );

        // A later evaluation creates no boundary/overrun for the completed block.
        let later = evaluate_block_notifications(&mut conn, utc_time(10, 0))
            .expect("poll after completion");
        assert!(later.is_empty());
        assert_eq!(count_block_deliveries(&conn, BOUNDARY_MESSAGE), 1);
        assert_eq!(count_block_deliveries(&conn, OVERRUN_MESSAGE), 1);
    }

    #[test]
    fn block_events_are_workspace_scoped() {
        let mut conn = test_conn();
        set_utc_timezone(&conn);
        seed_focus_rule(&conn, true, None);

        // A second workspace with its own focus rule and its own due block.
        conn.execute(
            "INSERT INTO workspaces (id, name, created_at, updated_at)
             VALUES ('w2', 'Synthetic other', '2026-10-05T00:00:00Z',
                     '2026-10-05T00:00:00Z')",
            [],
        )
        .expect("seed second workspace");
        conn.execute(
            "INSERT INTO reminder_rules (
                id, workspace_id, type, enabled, privacy_safe_message,
                created_at, updated_at
             ) VALUES ('focus-rule-w2', 'w2', 'focus', 1,
                       'Ready for a focus block?', '2026-10-05T00:00:00Z',
                       '2026-10-05T00:00:00Z')",
            [],
        )
        .expect("seed second focus rule");
        conn.execute("UPDATE workspaces SET timezone = 'UTC' WHERE id = 'w2'", [])
            .expect("set second workspace timezone");
        conn.execute(
            "INSERT INTO blocks (
                id, workspace_id, date, type, status, label, planned_start_at,
                estimated_duration_seconds, actual_start_at, flexibility,
                created_at, updated_at
             ) VALUES ('b2', 'w2', '2026-10-05', 'focus', 'planned', NULL,
                       '09:00', 600, NULL, 'flexible',
                       '2026-10-05T00:00:00Z', '2026-10-05T00:00:00Z')",
            [],
        )
        .expect("seed second workspace block");
        seed_one_off_block(
            &conn,
            "b1",
            BlockStatus::Planned,
            Some("09:00"),
            Some(600),
            None,
            None,
        );

        let deliveries = evaluate_block_notifications(&mut conn, utc_time(9, 30))
            .expect("evaluate both workspaces");

        assert_eq!(
            deliveries.len(),
            2,
            "each workspace produces its own delivery"
        );
        let w1 = deliveries
            .iter()
            .find(|delivery| delivery.workspace_id == "w1")
            .expect("w1 delivery");
        assert_eq!(
            w1.target.as_ref().map(|target| target.entity_id.as_str()),
            Some("b1")
        );
        let w2 = deliveries
            .iter()
            .find(|delivery| delivery.workspace_id == "w2")
            .expect("w2 delivery");
        assert_eq!(
            w2.target.as_ref().map(|target| target.entity_id.as_str()),
            Some("b2")
        );
    }
}
