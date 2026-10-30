use chrono::{
    DateTime, Datelike, LocalResult, NaiveDate, NaiveDateTime, NaiveTime, SecondsFormat, TimeZone,
    Utc,
};
use chrono_tz::Tz;
use rusqlite::{params, Connection, OptionalExtension};

use crate::domain::models::{BlockRecurrence, ScheduledBlockRecurrence};
use crate::domain::types::{BlockLifecycleState, BlockType, Flexibility};
use crate::error::AppError;
use crate::{id, time};

#[derive(Debug)]
pub struct RecurrenceRule<'a> {
    pub weekdays: Option<&'a [i64]>,
    pub local_start_time: Option<&'a str>,
    pub estimated_duration_seconds: Option<i64>,
    pub task_id: Option<&'a str>,
    pub block_type: &'a str,
    pub label: &'a Option<String>,
    pub flexibility: &'a str,
    pub note_markdown: &'a Option<String>,
    pub enabled: bool,
    pub effective_from: &'a str,
    pub effective_until: &'a Option<String>,
}

struct ValidatedRecurrenceRule<'a> {
    weekdays: &'a [i64],
    local_start_time: &'a str,
    estimated_duration_seconds: i64,
    task_id: Option<&'a str>,
    block_type: BlockType,
    label: &'a Option<String>,
    flexibility: Flexibility,
    note_markdown: &'a Option<String>,
    enabled: bool,
    effective_from: &'a str,
    effective_until: &'a Option<String>,
}

pub fn create(
    conn: &Connection,
    workspace_id: &str,
    rule: RecurrenceRule<'_>,
) -> Result<BlockRecurrence, AppError> {
    let rule = validate_rule(&rule)?;
    let timezone_id = workspace_timezone(conn, workspace_id)?;
    let timezone = parse_timezone(&timezone_id)?;
    if let Some(task_id) = rule.task_id {
        crate::repos::block_repo::validate_task_reference(conn, workspace_id, task_id)?;
    }
    let recurrence_id = id::new_uuid();
    let now = time::now_rfc3339();
    conn.execute(
        "INSERT INTO block_recurrences (id, workspace_id, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?3)",
        params![recurrence_id, workspace_id, now],
    )?;
    let version_id = insert_version(conn, &recurrence_id, &rule)?;
    conn.execute(
        "UPDATE block_recurrences SET current_version_id = ?2 WHERE id = ?1",
        params![recurrence_id, version_id],
    )?;
    // A new series starts its active lifecycle interval on the workspace-local
    // date of its creation.
    insert_open_lifecycle_interval(
        conn,
        workspace_id,
        &recurrence_id,
        &local_date_of(Utc::now(), timezone),
    )?;
    get(conn, workspace_id, &recurrence_id)
}

pub fn update(
    conn: &Connection,
    workspace_id: &str,
    recurrence_id: &str,
    rule: RecurrenceRule<'_>,
) -> Result<BlockRecurrence, AppError> {
    let rule = validate_rule(&rule)?;
    let timezone_id = workspace_timezone(conn, workspace_id)?;
    let timezone = parse_timezone(&timezone_id)?;
    ensure_recurrence_timezone_policy_unambiguous(conn, workspace_id, recurrence_id, &timezone_id)?;
    get(conn, workspace_id, recurrence_id)?;
    if let Some(task_id) = rule.task_id {
        crate::repos::block_repo::validate_task_reference(conn, workspace_id, task_id)?;
    }
    let version_id = insert_version(conn, recurrence_id, &rule)?;
    let now = time::now_rfc3339();
    conn.execute(
        "UPDATE block_recurrences SET current_version_id = ?3, updated_at = ?4
         WHERE id = ?1 AND workspace_id = ?2",
        params![recurrence_id, workspace_id, version_id, now],
    )?;

    let new_weekdays: std::collections::HashSet<i64> = rule.weekdays.iter().copied().collect();
    let mut stmt = conn.prepare(
        "SELECT id, local_date, resolved_start_at FROM blocks
         WHERE workspace_id = ?1 AND recurrence_id = ?2 AND status = 'planned'",
    )?;
    let rows = stmt.query_map(params![workspace_id, recurrence_id], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, String>(2)?,
        ))
    })?;
    let mut future_rows = Vec::new();
    let now_utc = Utc::now();
    for row in rows {
        let (block_id, local_date, resolved_start_at) = row?;
        let resolved = DateTime::parse_from_rfc3339(&resolved_start_at).map_err(|_| {
            AppError::validation("A recurring block has an invalid resolved start snapshot.")
        })?;
        if resolved.with_timezone(&Utc) > now_utc {
            future_rows.push((block_id, local_date));
        }
    }
    drop(stmt);

    for (block_id, local_date) in future_rows {
        let date = time::parse_date(&local_date)?;
        let weekday = i64::from(date.weekday().number_from_monday());
        let within_range = date >= time::parse_date(rule.effective_from)?
            && rule
                .effective_until
                .as_deref()
                .map(time::parse_date)
                .transpose()?
                .is_none_or(|end| date <= end);
        if !rule.enabled || !within_range || !new_weekdays.contains(&weekday) {
            conn.execute(
                "UPDATE blocks SET status = 'cancelled', updated_at = ?2 WHERE id = ?1",
                params![block_id, now],
            )?;
            continue;
        }
        let resolved = resolve_local_start(&date, rule.local_start_time, timezone)?;
        conn.execute(
            "UPDATE blocks SET recurrence_version_id = ?2, type = ?3, label = ?4,
                planned_start_at = ?5, estimated_duration_seconds = ?6,
                flexibility = ?7, note_markdown = ?8, local_start_time = ?5,
                timezone_id = ?9, resolved_start_at = ?10, task_id = ?11,
                updated_at = ?12
             WHERE id = ?1",
            params![
                block_id,
                version_id,
                rule.block_type.to_string(),
                rule.label,
                rule.local_start_time,
                rule.estimated_duration_seconds,
                rule.flexibility.to_string(),
                rule.note_markdown,
                timezone_id,
                resolved,
                rule.task_id,
                now
            ],
        )?;
    }
    get(conn, workspace_id, recurrence_id)
}

pub fn get(
    conn: &Connection,
    workspace_id: &str,
    recurrence_id: &str,
) -> Result<BlockRecurrence, AppError> {
    let mut recurrence = conn
        .query_row(
            "SELECT r.id, r.workspace_id, v.id, v.local_start_time,
                v.estimated_duration_seconds, v.type, v.label, v.flexibility,
                v.note_markdown, v.enabled, v.effective_from, v.effective_until,
                r.created_at, r.updated_at, r.lifecycle_state, v.task_id
             FROM block_recurrences r
             JOIN block_recurrence_versions v ON v.id = r.current_version_id
             WHERE r.workspace_id = ?1 AND r.id = ?2",
            params![workspace_id, recurrence_id],
            |row| {
                let block_type: String = row.get(5)?;
                let flexibility: String = row.get(7)?;
                let lifecycle_state: String = row.get(14)?;
                Ok(BlockRecurrence {
                    id: row.get(0)?,
                    workspace_id: row.get(1)?,
                    version_id: row.get(2)?,
                    weekdays: Vec::new(),
                    local_start_time: row.get(3)?,
                    estimated_duration_seconds: row.get(4)?,
                    block_type: block_type.parse().map_err(|_| {
                        rusqlite::Error::InvalidColumnType(
                            5,
                            "type".into(),
                            rusqlite::types::Type::Text,
                        )
                    })?,
                    label: row.get(6)?,
                    flexibility: flexibility.parse().map_err(|_| {
                        rusqlite::Error::InvalidColumnType(
                            7,
                            "flexibility".into(),
                            rusqlite::types::Type::Text,
                        )
                    })?,
                    note_markdown: row.get(8)?,
                    enabled: row.get(9)?,
                    effective_from: row.get(10)?,
                    effective_until: row.get(11)?,
                    created_at: row.get(12)?,
                    updated_at: row.get(13)?,
                    lifecycle_state: lifecycle_state.parse().map_err(|_| {
                        rusqlite::Error::InvalidColumnType(
                            14,
                            "lifecycle_state".into(),
                            rusqlite::types::Type::Text,
                        )
                    })?,
                    task_id: row.get(15)?,
                })
            },
        )
        .optional()?
        .ok_or_else(|| AppError::not_found("The block recurrence was not found."))?;
    let mut stmt = conn.prepare(
        "SELECT weekday FROM block_recurrence_version_weekdays
         WHERE version_id = ?1 ORDER BY weekday",
    )?;
    recurrence.weekdays = stmt
        .query_map([&recurrence.version_id], |row| row.get(0))?
        .collect::<Result<Vec<i64>, _>>()?;
    Ok(recurrence)
}

/// List the series of a workspace in one lifecycle state, oldest first.
pub fn list_series(
    conn: &Connection,
    workspace_id: &str,
    lifecycle_state: BlockLifecycleState,
) -> Result<Vec<BlockRecurrence>, AppError> {
    let mut stmt = conn.prepare(
        "SELECT id FROM block_recurrences
         WHERE workspace_id = ?1 AND lifecycle_state = ?2
         ORDER BY created_at, id",
    )?;
    let ids = stmt
        .query_map(params![workspace_id, lifecycle_state.to_string()], |row| {
            row.get::<_, String>(0)
        })?
        .collect::<Result<Vec<_>, _>>()?;
    drop(stmt);
    let mut series = Vec::new();
    for id in ids {
        series.push(get(conn, workspace_id, &id)?);
    }
    Ok(series)
}

/// Active recurrence definitions with their next eligible local date. This
/// reads series rules without materializing future occurrence rows.
pub fn list_scheduled_series(
    conn: &Connection,
    workspace_id: &str,
) -> Result<Vec<ScheduledBlockRecurrence>, AppError> {
    let today = workspace_local_today(conn, workspace_id)?;
    let today_date = time::parse_date(&today)?;
    let mut scheduled = Vec::new();
    for recurrence in list_series(conn, workspace_id, BlockLifecycleState::Active)? {
        if !recurrence.enabled {
            continue;
        }
        let start = today_date.max(time::parse_date(&recurrence.effective_from)?);
        let until = recurrence
            .effective_until
            .as_deref()
            .map(time::parse_date)
            .transpose()?;
        let mut next_occurrence_date: Option<String> = None;
        for weekday in &recurrence.weekdays {
            let date = resolve_next_weekday_date(&start.format("%Y-%m-%d").to_string(), *weekday)?;
            if let Some(end) = until {
                if time::parse_date(&date)? > end {
                    continue;
                }
            }
            if next_occurrence_date
                .as_ref()
                .is_none_or(|current| date < *current)
            {
                next_occurrence_date = Some(date);
            }
        }
        if let Some(next_occurrence_date) = next_occurrence_date {
            scheduled.push(ScheduledBlockRecurrence {
                recurrence,
                next_occurrence_date,
            });
        }
    }
    scheduled.sort_by(|a, b| {
        a.next_occurrence_date
            .cmp(&b.next_occurrence_date)
            .then_with(|| {
                a.recurrence
                    .local_start_time
                    .cmp(&b.recurrence.local_start_time)
            })
            .then_with(|| a.recurrence.id.cmp(&b.recurrence.id))
    });
    Ok(scheduled)
}

/// Catch up missing occurrences for active routines on or before each
/// workspace's local today. Existing rows are authoritative; missed dates are
/// created as planned and never assigned an inferred outcome.
pub fn reconcile_active_occurrences(conn: &Connection) -> Result<(), AppError> {
    let workspaces = {
        let mut stmt = conn.prepare("SELECT id FROM workspaces ORDER BY id")?;
        let rows = stmt.query_map([], |row| row.get::<_, String>(0))?;
        rows.collect::<Result<Vec<_>, _>>()?
    };

    for workspace_id in workspaces {
        let today = workspace_local_today(conn, &workspace_id)?;
        let today_date = time::parse_date(&today)?;
        let ranges = {
            let mut stmt = conn.prepare(
                "SELECT i.active_from_local_date, MAX(b.local_date),
                    v.effective_from, v.effective_until
                 FROM block_recurrences r
                 JOIN block_recurrence_lifecycle_intervals i
                   ON i.workspace_id = r.workspace_id AND i.recurrence_id = r.id
                   AND i.active_until_local_date IS NULL
                 JOIN block_recurrence_versions v ON v.id = r.current_version_id
                 LEFT JOIN blocks b ON b.workspace_id = r.workspace_id
                   AND b.recurrence_id = r.id AND b.local_date <= ?2
                 WHERE r.workspace_id = ?1 AND r.lifecycle_state = 'active'
                   AND v.enabled = 1
                 GROUP BY r.id, i.active_from_local_date, v.effective_from,
                    v.effective_until",
            )?;
            let rows = stmt.query_map(params![workspace_id, today], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, Option<String>>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, Option<String>>(3)?,
                ))
            })?;
            rows.collect::<Result<Vec<_>, _>>()?
        };

        let mut earliest = None;
        let mut reconciliation_end = None;
        for (active_from, latest_occurrence, effective_from, effective_until) in ranges {
            let mut start = time::parse_date(&active_from)?.max(time::parse_date(&effective_from)?);
            if let Some(latest) = latest_occurrence {
                let next = time::parse_date(&latest)?
                    .succ_opt()
                    .ok_or_else(|| AppError::validation("The next recurrence date is invalid."))?;
                start = start.max(next);
            }
            let end = effective_until
                .as_deref()
                .map(time::parse_date)
                .transpose()?
                .unwrap_or(today_date)
                .min(today_date);
            if start > end {
                continue;
            }
            earliest = Some(earliest.map_or(start, |current: NaiveDate| current.min(start)));
            reconciliation_end =
                Some(reconciliation_end.map_or(end, |current: NaiveDate| current.max(end)));
        }

        let (Some(mut date), Some(end)) = (earliest, reconciliation_end) else {
            continue;
        };
        while date <= end {
            materialize_for_date_ignoring_schedule_errors(
                conn,
                &workspace_id,
                &date.format("%Y-%m-%d").to_string(),
            )?;
            let Some(next) = date.succ_opt() else {
                break;
            };
            date = next;
        }
    }
    Ok(())
}

/// Current lifecycle state of a series in a workspace.
fn series_lifecycle_state(
    conn: &Connection,
    workspace_id: &str,
    recurrence_id: &str,
) -> Result<BlockLifecycleState, AppError> {
    let lifecycle_state: Option<String> = conn
        .query_row(
            "SELECT lifecycle_state FROM block_recurrences
             WHERE workspace_id = ?1 AND id = ?2",
            params![workspace_id, recurrence_id],
            |row| row.get(0),
        )
        .optional()?;
    let lifecycle_state = lifecycle_state
        .ok_or_else(|| AppError::not_found("The block recurrence was not found."))?;
    lifecycle_state.parse()
}

/// Manually deactivate a series and withdraw its still-future planned
/// occurrences. Past and terminal occurrences keep their history.
pub fn deactivate_series(
    conn: &Connection,
    workspace_id: &str,
    recurrence_id: &str,
) -> Result<BlockRecurrence, AppError> {
    let lifecycle_state = series_lifecycle_state(conn, workspace_id, recurrence_id)?;
    if lifecycle_state != BlockLifecycleState::Active {
        return Err(AppError::validation(
            "Only an active block recurrence can be deactivated.",
        ));
    }
    let now = time::now_rfc3339();
    conn.execute(
        "UPDATE block_recurrences SET lifecycle_state = 'inactive', updated_at = ?3
         WHERE workspace_id = ?1 AND id = ?2",
        params![workspace_id, recurrence_id, now],
    )?;
    close_open_lifecycle_interval(
        conn,
        workspace_id,
        recurrence_id,
        &workspace_local_today(conn, workspace_id)?,
    )?;

    let mut stmt = conn.prepare(
        "SELECT id, resolved_start_at FROM blocks
         WHERE workspace_id = ?1 AND recurrence_id = ?2 AND status = 'planned'",
    )?;
    let rows = stmt.query_map(params![workspace_id, recurrence_id], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
    })?;
    let now_utc = Utc::now();
    let mut future_rows = Vec::new();
    for row in rows {
        let (block_id, resolved_start_at) = row?;
        let resolved = DateTime::parse_from_rfc3339(&resolved_start_at).map_err(|_| {
            AppError::validation("A recurring block has an invalid resolved start snapshot.")
        })?;
        if resolved.with_timezone(&Utc) > now_utc {
            future_rows.push(block_id);
        }
    }
    drop(stmt);

    for block_id in future_rows {
        conn.execute(
            "UPDATE blocks SET status = 'cancelled', lifecycle_state = 'inactive',
                updated_at = ?2 WHERE id = ?1",
            params![block_id, now],
        )?;
        crate::repos::notification_repo::fail_pending_block_event_deliveries(
            conn,
            &block_id,
            "The recurrence was deactivated before this notification was due.",
        )?;
    }
    get(conn, workspace_id, recurrence_id)
}

/// Reactivate an inactive series. Occurrence generation resumes on demand.
pub fn reactivate_series(
    conn: &Connection,
    workspace_id: &str,
    recurrence_id: &str,
) -> Result<BlockRecurrence, AppError> {
    match series_lifecycle_state(conn, workspace_id, recurrence_id)? {
        BlockLifecycleState::Inactive => {}
        BlockLifecycleState::Active => {
            return Err(AppError::validation(
                "The block recurrence is already active.",
            ));
        }
        BlockLifecycleState::Deleted => {
            return Err(AppError::validation(
                "Deleted block recurrences cannot be reactivated.",
            ));
        }
    }
    let now = time::now_rfc3339();
    conn.execute(
        "UPDATE block_recurrences SET lifecycle_state = 'active', updated_at = ?3
         WHERE workspace_id = ?1 AND id = ?2",
        params![workspace_id, recurrence_id, now],
    )?;
    // Deactivation closed the previous interval, so the partial unique index
    // sees no open interval for this series.
    insert_open_lifecycle_interval(
        conn,
        workspace_id,
        recurrence_id,
        &workspace_local_today(conn, workspace_id)?,
    )?;
    get(conn, workspace_id, recurrence_id)
}

/// Soft-delete a series and withdraw only its still-future planned
/// occurrences. Historical and started work remains intact.
pub fn delete_series(
    conn: &Connection,
    workspace_id: &str,
    recurrence_id: &str,
) -> Result<BlockRecurrence, AppError> {
    series_lifecycle_state(conn, workspace_id, recurrence_id)?;
    let now = time::now_rfc3339();
    conn.execute(
        "UPDATE block_recurrences SET lifecycle_state = 'deleted', updated_at = ?3
         WHERE workspace_id = ?1 AND id = ?2",
        params![workspace_id, recurrence_id, now],
    )?;
    let mut stmt = conn.prepare(
        "SELECT id, resolved_start_at FROM blocks
         WHERE workspace_id = ?1 AND recurrence_id = ?2
           AND lifecycle_state = 'active' AND status = 'planned'",
    )?;
    let rows = stmt.query_map(params![workspace_id, recurrence_id], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
    })?;
    let now_utc = Utc::now();
    let mut future_ids = Vec::new();
    for row in rows {
        let (block_id, resolved_start_at) = row?;
        let resolved = DateTime::parse_from_rfc3339(&resolved_start_at).map_err(|_| {
            AppError::validation("A recurring block has an invalid resolved start snapshot.")
        })?;
        if resolved.with_timezone(&Utc) > now_utc {
            future_ids.push(block_id);
        }
    }
    drop(stmt);
    for block_id in future_ids {
        conn.execute(
            "UPDATE blocks SET status = 'cancelled', lifecycle_state = 'deleted',
                updated_at = ?2 WHERE id = ?1",
            params![block_id, now],
        )?;
        crate::repos::notification_repo::fail_pending_block_event_deliveries(
            conn,
            &block_id,
            "The recurrence was deleted before this notification was due.",
        )?;
    }
    close_open_lifecycle_interval(
        conn,
        workspace_id,
        recurrence_id,
        &workspace_local_today(conn, workspace_id)?,
    )?;
    get(conn, workspace_id, recurrence_id)
}

/// First local date on or after `today` whose ISO weekday (Monday=1 .. Sunday=7)
/// matches `weekday`.
pub fn resolve_next_weekday_date(today: &str, weekday: i64) -> Result<String, AppError> {
    if !(1..=7).contains(&weekday) {
        return Err(AppError::validation(
            "Select an ISO weekday from 1 through 7.",
        ));
    }
    let today = time::parse_date(today)?;
    for offset in 0..7 {
        let candidate = today
            .checked_add_days(chrono::Days::new(offset))
            .ok_or_else(|| AppError::validation("The next weekday date cannot be resolved."))?;
        if i64::from(candidate.weekday().number_from_monday()) == weekday {
            return Ok(candidate.format("%Y-%m-%d").to_string());
        }
    }
    Err(AppError::validation(
        "The next weekday date cannot be resolved.",
    ))
}

pub fn materialize_for_date(
    conn: &Connection,
    workspace_id: &str,
    date_text: &str,
) -> Result<(), AppError> {
    materialize_for_date_inner(conn, workspace_id, date_text, false)
}

/// Materialize a date for an upcoming page: one occurrence whose local start
/// cannot be resolved (for example a daylight-saving gap) is skipped while the
/// timezone-policy guard still propagates.
pub(crate) fn materialize_for_date_ignoring_schedule_errors(
    conn: &Connection,
    workspace_id: &str,
    date_text: &str,
) -> Result<(), AppError> {
    materialize_for_date_inner(conn, workspace_id, date_text, true)
}

fn materialize_for_date_inner(
    conn: &Connection,
    workspace_id: &str,
    date_text: &str,
    ignore_schedule_errors: bool,
) -> Result<(), AppError> {
    let date = time::parse_date(date_text)?;
    let timezone_id = workspace_timezone(conn, workspace_id)?;
    let timezone = parse_timezone(&timezone_id)?;
    ensure_date_timezone_policy_unambiguous(conn, workspace_id, date_text, &timezone_id)?;

    let mut stmt = conn.prepare(
        "SELECT r.id, v.id, v.local_start_time, v.estimated_duration_seconds,
            v.type, v.label, v.flexibility, v.note_markdown, v.enabled,
            v.effective_from, v.effective_until, v.task_id
         FROM block_recurrences r
         JOIN block_recurrence_versions v ON v.id = r.current_version_id
         WHERE r.workspace_id = ?1 AND r.lifecycle_state = 'active'",
    )?;
    let rows = stmt.query_map([workspace_id], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, String>(2)?,
            row.get::<_, i64>(3)?,
            row.get::<_, String>(4)?,
            row.get::<_, Option<String>>(5)?,
            row.get::<_, String>(6)?,
            row.get::<_, Option<String>>(7)?,
            row.get::<_, bool>(8)?,
            row.get::<_, String>(9)?,
            row.get::<_, Option<String>>(10)?,
            row.get::<_, Option<String>>(11)?,
        ))
    })?;
    let mut rules = Vec::new();
    for row in rows {
        rules.push(row?);
    }
    drop(stmt);

    let weekday = i64::from(date.weekday().number_from_monday());
    let mut occurrence_stmt = conn.prepare(
        "SELECT 1 FROM blocks WHERE workspace_id = ?1 AND recurrence_id = ?2
            AND local_date = ?3 LIMIT 1",
    )?;
    for (
        recurrence_id,
        version_id,
        local_start_time,
        duration_seconds,
        block_type,
        label,
        flexibility,
        note_markdown,
        enabled,
        effective_from,
        effective_until,
        task_id,
    ) in rules
    {
        if !enabled
            || date < time::parse_date(&effective_from)?
            || effective_until
                .as_deref()
                .map(time::parse_date)
                .transpose()?
                .is_some_and(|until| date > until)
        {
            continue;
        }
        let selected: bool = conn.query_row(
            "SELECT EXISTS(SELECT 1 FROM block_recurrence_version_weekdays
             WHERE version_id = ?1 AND weekday = ?2)",
            params![version_id, weekday],
            |row| row.get(0),
        )?;
        if !selected {
            continue;
        }
        let exists = occurrence_stmt
            .query_row(params![workspace_id, recurrence_id, date_text], |_| Ok(()))
            .optional()?
            .is_some();
        if exists {
            continue;
        }
        let block_type = block_type.parse::<BlockType>()?;
        let flexibility = flexibility.parse::<Flexibility>()?;
        let resolved = match resolve_local_start(&date, &local_start_time, timezone) {
            Ok(resolved) => resolved,
            Err(_) if ignore_schedule_errors => continue,
            Err(error) => return Err(error),
        };
        let block_id = id::new_uuid();
        let now = time::now_rfc3339();
        conn.execute(
            "INSERT INTO blocks (id, workspace_id, daily_plan_id, date, type, status,
                label, planned_start_at, estimated_duration_seconds, time_window_label,
                flexibility, note_markdown, created_at, updated_at, recurrence_id,
                recurrence_version_id, local_date, local_start_time, timezone_id,
                resolved_start_at, task_id)
             VALUES (?1, ?2, NULL, ?3, ?4, 'planned', ?5, ?6, ?7, NULL, ?8, ?9,
                ?10, ?10, ?11, ?12, ?3, ?6, ?13, ?14, ?15)",
            params![
                block_id,
                workspace_id,
                date_text,
                block_type.to_string(),
                label,
                local_start_time,
                duration_seconds,
                flexibility.to_string(),
                note_markdown,
                now,
                recurrence_id,
                version_id,
                timezone_id,
                resolved,
                task_id
            ],
        )?;
    }
    Ok(())
}

fn validate_rule<'a>(rule: &RecurrenceRule<'a>) -> Result<ValidatedRecurrenceRule<'a>, AppError> {
    let weekdays = rule.weekdays.ok_or_else(|| {
        AppError::validation("Select one or more unique ISO weekdays from 1 through 7.")
    })?;
    let local_start_time = rule
        .local_start_time
        .ok_or_else(|| AppError::validation("A recurring block local start time is required."))?;
    let estimated_duration_seconds = rule.estimated_duration_seconds.ok_or_else(|| {
        AppError::validation("Recurring block duration must be a positive number of seconds.")
    })?;
    crate::domain::validation::validate_block_recurrence(
        weekdays,
        local_start_time,
        estimated_duration_seconds,
        rule.effective_from,
        rule.effective_until.as_deref(),
    )?;
    Ok(ValidatedRecurrenceRule {
        weekdays,
        local_start_time,
        estimated_duration_seconds,
        task_id: rule.task_id,
        block_type: rule.block_type.parse()?,
        label: rule.label,
        flexibility: rule.flexibility.parse()?,
        note_markdown: rule.note_markdown,
        enabled: rule.enabled,
        effective_from: rule.effective_from,
        effective_until: rule.effective_until,
    })
}

fn insert_version(
    conn: &Connection,
    recurrence_id: &str,
    rule: &ValidatedRecurrenceRule<'_>,
) -> Result<String, AppError> {
    let version_id = id::new_uuid();
    let now = time::now_rfc3339();
    conn.execute(
        "INSERT INTO block_recurrence_versions (id, recurrence_id, local_start_time,
            estimated_duration_seconds, type, label, flexibility, note_markdown,
            enabled, effective_from, effective_until, created_at, task_id)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
        params![
            version_id,
            recurrence_id,
            rule.local_start_time,
            rule.estimated_duration_seconds,
            rule.block_type.to_string(),
            rule.label,
            rule.flexibility.to_string(),
            rule.note_markdown,
            rule.enabled,
            rule.effective_from,
            rule.effective_until,
            now,
            rule.task_id
        ],
    )?;
    for weekday in rule.weekdays {
        conn.execute(
            "INSERT INTO block_recurrence_version_weekdays (version_id, weekday)
             VALUES (?1, ?2)",
            params![version_id, weekday],
        )?;
    }
    Ok(version_id)
}

/// Workspace-local calendar date (`YYYY-MM-DD`) of a UTC instant. The single
/// conversion used by recurrence lifecycle-interval writes.
pub(crate) fn local_date_of(instant: DateTime<Utc>, timezone: Tz) -> String {
    instant
        .with_timezone(&timezone)
        .date_naive()
        .format("%Y-%m-%d")
        .to_string()
}

/// Current local calendar date in the workspace timezone.
fn workspace_local_today(conn: &Connection, workspace_id: &str) -> Result<String, AppError> {
    let timezone_id = workspace_timezone(conn, workspace_id)?;
    let timezone = parse_timezone(&timezone_id)?;
    Ok(local_date_of(Utc::now(), timezone))
}

/// Open a new active lifecycle interval for a series (`active_until_local_date`
/// NULL). The partial unique index allows at most one open interval per series,
/// so this only runs for series creation and reactivation, which are the two
/// states in which the series has no open interval.
fn insert_open_lifecycle_interval(
    conn: &Connection,
    workspace_id: &str,
    recurrence_id: &str,
    local_date: &str,
) -> Result<(), AppError> {
    conn.execute(
        "INSERT INTO block_recurrence_lifecycle_intervals
         (id, workspace_id, recurrence_id, active_from_local_date,
          active_until_local_date, source, created_at)
         VALUES (?1, ?2, ?3, ?4, NULL, 'explicit', ?5)",
        params![
            id::new_uuid(),
            workspace_id,
            recurrence_id,
            local_date,
            time::now_rfc3339()
        ],
    )?;
    Ok(())
}

/// Close the open lifecycle interval of a series at `local_date`.
///
/// The normalized path closes the open interval. When a series has no open
/// interval (a legacy or otherwise anomalous state that the create/deactivate/
/// reactivate paths do not produce), a closed single-day interval for that date
/// is recorded instead of failing the lifecycle change. The stored end date is
/// never allowed to precede the interval start.
fn close_open_lifecycle_interval(
    conn: &Connection,
    workspace_id: &str,
    recurrence_id: &str,
    local_date: &str,
) -> Result<(), AppError> {
    let closed = conn.execute(
        "UPDATE block_recurrence_lifecycle_intervals
         SET active_until_local_date = MAX(active_from_local_date, ?3)
         WHERE workspace_id = ?1 AND recurrence_id = ?2
           AND active_until_local_date IS NULL",
        params![workspace_id, recurrence_id, local_date],
    )?;
    if closed == 0 {
        conn.execute(
            "INSERT INTO block_recurrence_lifecycle_intervals
             (id, workspace_id, recurrence_id, active_from_local_date,
              active_until_local_date, source, created_at)
             VALUES (?1, ?2, ?3, ?4, ?4, 'explicit', ?5)",
            params![
                id::new_uuid(),
                workspace_id,
                recurrence_id,
                local_date,
                time::now_rfc3339()
            ],
        )?;
    }
    Ok(())
}

pub(crate) fn workspace_timezone(
    conn: &Connection,
    workspace_id: &str,
) -> Result<String, AppError> {
    conn.query_row(
        "SELECT timezone FROM workspaces WHERE id = ?1",
        [workspace_id],
        |row| row.get(0),
    )
    .optional()?
    .ok_or_else(|| AppError::not_found("The workspace was not found."))
}

pub(crate) fn parse_timezone(timezone_id: &str) -> Result<Tz, AppError> {
    timezone_id
        .parse::<Tz>()
        .map_err(|_| AppError::validation("The workspace timezone is not a valid IANA timezone."))
}

fn ensure_recurrence_timezone_policy_unambiguous(
    conn: &Connection,
    workspace_id: &str,
    recurrence_id: &str,
    current_timezone: &str,
) -> Result<(), AppError> {
    let mut stmt = conn.prepare(
        "SELECT timezone_id, resolved_start_at FROM blocks
         WHERE workspace_id = ?1 AND recurrence_id = ?2 AND status = 'planned'",
    )?;
    let rows = stmt.query_map(params![workspace_id, recurrence_id], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
    })?;
    for row in rows {
        let (timezone_id, resolved_start_at) = row?;
        ensure_snapshot_timezone_unambiguous(&timezone_id, &resolved_start_at, current_timezone)?;
    }
    Ok(())
}

fn ensure_date_timezone_policy_unambiguous(
    conn: &Connection,
    workspace_id: &str,
    local_date: &str,
    current_timezone: &str,
) -> Result<(), AppError> {
    let mut stmt = conn.prepare(
        "SELECT timezone_id, resolved_start_at FROM blocks
         WHERE workspace_id = ?1 AND local_date = ?2
           AND recurrence_id IS NOT NULL AND status = 'planned'",
    )?;
    let rows = stmt.query_map(params![workspace_id, local_date], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
    })?;
    for row in rows {
        let (timezone_id, resolved_start_at) = row?;
        ensure_snapshot_timezone_unambiguous(&timezone_id, &resolved_start_at, current_timezone)?;
    }
    Ok(())
}

fn ensure_snapshot_timezone_unambiguous(
    snapshot_timezone: &str,
    resolved_start_at: &str,
    current_timezone: &str,
) -> Result<(), AppError> {
    let now = Utc::now();
    let resolved = DateTime::parse_from_rfc3339(resolved_start_at).map_err(|_| {
        AppError::validation("A recurring block has an invalid resolved start snapshot.")
    })?;
    if resolved.with_timezone(&Utc) > now && snapshot_timezone != current_timezone {
        return Err(AppError::validation(
            "The workspace timezone changed while future recurring occurrences are materialized. The recurrence timezone policy (B2) must be resolved before continuing.",
        ));
    }
    Ok(())
}

pub(crate) fn resolve_local_start(
    date: &NaiveDate,
    local_time: &str,
    timezone: Tz,
) -> Result<String, AppError> {
    let local_time = NaiveTime::parse_from_str(local_time, "%H:%M")
        .map_err(|_| AppError::validation("The recurrence local start time is invalid."))?;
    let local = NaiveDateTime::new(*date, local_time);
    let resolved = match timezone.from_local_datetime(&local) {
        LocalResult::Single(value) => value,
        LocalResult::Ambiguous(first, second) => {
            if first.with_timezone(&Utc) <= second.with_timezone(&Utc) {
                first
            } else {
                second
            }
        }
        LocalResult::None => {
            return Err(AppError::validation(
                "This recurrence date falls in a daylight-saving gap and cannot be scheduled.",
            ));
        }
    };
    Ok(resolved
        .with_timezone(&Utc)
        .to_rfc3339_opts(SecondsFormat::Secs, true))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::types::BlockStatus;
    use crate::repos::block_repo::{BlockRepo, SqliteBlockRepo};
    use crate::repos::notification_repo;
    use crate::test_utils::test_conn;

    static NO_TEXT: Option<String> = None;

    fn seed_task(conn: &Connection, id: &str) {
        conn.execute(
            "INSERT INTO tasks (id, workspace_id, title, status, priority, created_at,
                updated_at)
             VALUES (?1, 'w1', 'Synthetic task', 'planned', 'none', '2026-01-01T00:00:00Z',
                     '2026-01-01T00:00:00Z')",
            [id],
        )
        .expect("seed synthetic task");
    }

    fn rule<'a>(
        weekdays: &'a [i64],
        start: &'a str,
        duration: i64,
        enabled: bool,
    ) -> RecurrenceRule<'a> {
        RecurrenceRule {
            weekdays: Some(weekdays),
            local_start_time: Some(start),
            estimated_duration_seconds: Some(duration),
            task_id: None,
            block_type: "focus",
            label: &NO_TEXT,
            flexibility: "flexible",
            note_markdown: &NO_TEXT,
            enabled,
            effective_from: "2000-01-01",
            effective_until: &NO_TEXT,
        }
    }

    #[test]
    fn weekday_selection_materializes_only_iso_selected_days_in_workspace() {
        let conn = test_conn();
        conn.execute("UPDATE workspaces SET timezone = 'UTC' WHERE id = 'w1'", [])
            .expect("set timezone");
        conn.execute(
            "INSERT INTO workspaces (id, name, created_at, updated_at)
             VALUES ('w2', 'Other synthetic workspace', 'created', 'updated')",
            [],
        )
        .expect("seed second workspace");
        create(&conn, "w1", rule(&[1, 3], "09:30", 1800, true)).expect("create recurrence");
        materialize_for_date(&conn, "w1", "2026-10-19").expect("Monday retrieval");
        materialize_for_date(&conn, "w1", "2026-10-20").expect("Tuesday retrieval");
        materialize_for_date(&conn, "w1", "2026-10-21").expect("Wednesday retrieval");
        assert_eq!(
            SqliteBlockRepo
                .list_for_date(&conn, "w1", "2026-10-19")
                .unwrap()
                .len(),
            1
        );
        assert!(SqliteBlockRepo
            .list_for_date(&conn, "w2", "2026-10-19")
            .unwrap()
            .is_empty());
        assert!(SqliteBlockRepo
            .list_for_date(&conn, "w1", "2026-10-20")
            .unwrap()
            .is_empty());
        let wednesday = SqliteBlockRepo
            .list_for_date(&conn, "w1", "2026-10-21")
            .unwrap();
        assert_eq!(wednesday.len(), 1);
        assert_eq!(wednesday[0].local_start_time.as_deref(), Some("09:30"));
        assert_eq!(wednesday[0].status, BlockStatus::Planned);
    }

    #[test]
    fn recurrence_edits_update_future_unstarted_and_cancel_removed_weekdays() {
        let conn = test_conn();
        conn.execute("UPDATE workspaces SET timezone = 'UTC' WHERE id = 'w1'", [])
            .expect("set timezone");
        let recurrence =
            create(&conn, "w1", rule(&[1, 2], "09:30", 1800, true)).expect("create recurrence");
        materialize_for_date(&conn, "w1", "2099-01-05").expect("materialize removed day");
        materialize_for_date(&conn, "w1", "2099-01-06").expect("materialize kept day");
        let updated = update(
            &conn,
            "w1",
            &recurrence.id,
            rule(&[2, 3], "10:45", 2400, true),
        )
        .expect("edit recurrence");
        assert_eq!(updated.weekdays, vec![2, 3]);
        let monday = SqliteBlockRepo
            .list_for_date(&conn, "w1", "2099-01-05")
            .unwrap();
        assert_eq!(monday[0].status, BlockStatus::Cancelled);
        let tuesday = SqliteBlockRepo
            .list_for_date(&conn, "w1", "2099-01-06")
            .unwrap();
        assert_eq!(tuesday[0].local_start_time.as_deref(), Some("10:45"));
        assert_eq!(tuesday[0].estimated_duration_seconds, Some(2400));
        assert_ne!(
            tuesday[0].recurrence_version_id.as_deref(),
            Some(recurrence.version_id.as_str())
        );
    }

    #[test]
    fn recurrence_edits_preserve_completed_skipped_and_past_unstarted_rows() {
        let conn = test_conn();
        conn.execute("UPDATE workspaces SET timezone = 'UTC' WHERE id = 'w1'", [])
            .expect("set timezone");
        let recurrence =
            create(&conn, "w1", rule(&[1], "09:30", 1800, true)).expect("create recurrence");
        materialize_for_date(&conn, "w1", "2020-01-06").expect("materialize past row");
        materialize_for_date(&conn, "w1", "2099-01-05").expect("materialize completed row");
        materialize_for_date(&conn, "w1", "2099-01-12").expect("materialize skipped row");
        conn.execute(
            "UPDATE blocks SET status = 'completed' WHERE local_date = '2099-01-05'",
            [],
        )
        .expect("set completed status");
        conn.execute(
            "UPDATE blocks SET status = 'skipped' WHERE local_date = '2099-01-12'",
            [],
        )
        .expect("set skipped status");
        let old_past = SqliteBlockRepo
            .list_for_date(&conn, "w1", "2020-01-06")
            .unwrap()[0]
            .recurrence_version_id
            .clone();
        let old_completed = SqliteBlockRepo
            .list_for_date(&conn, "w1", "2099-01-05")
            .unwrap()[0]
            .recurrence_version_id
            .clone();
        update(&conn, "w1", &recurrence.id, rule(&[2], "11:00", 3600, true))
            .expect("edit recurrence");
        for (date, status, old_version) in [
            ("2020-01-06", BlockStatus::Planned, old_past),
            ("2099-01-05", BlockStatus::Completed, old_completed),
            (
                "2099-01-12",
                BlockStatus::Skipped,
                SqliteBlockRepo
                    .list_for_date(&conn, "w1", "2099-01-12")
                    .unwrap()[0]
                    .recurrence_version_id
                    .clone(),
            ),
        ] {
            let block = SqliteBlockRepo
                .list_for_date(&conn, "w1", date)
                .unwrap()
                .remove(0);
            assert_eq!(block.status, status);
            assert_eq!(block.recurrence_version_id, old_version);
        }
    }

    #[test]
    fn timezone_change_guard_is_scoped_to_affected_recurrence_and_date() {
        let conn = test_conn();
        let recurrence_a =
            create(&conn, "w1", rule(&[1], "09:30", 1800, true)).expect("create recurrence");
        let recurrence_b =
            create(&conn, "w1", rule(&[2], "11:00", 1800, true)).expect("create other recurrence");
        materialize_for_date(&conn, "w1", "2099-01-05").expect("materialize future row");
        conn.execute(
            "UPDATE workspaces SET timezone = 'America/New_York' WHERE id = 'w1'",
            [],
        )
        .expect("change workspace timezone");
        materialize_for_date(&conn, "w1", "2099-01-12")
            .expect("other date in same recurrence uses current timezone");
        materialize_for_date(&conn, "w1", "2099-01-06")
            .expect("another recurrence uses current timezone");
        let unrelated = SqliteBlockRepo
            .list_for_date(&conn, "w1", "2099-01-06")
            .expect("retrieve unrelated recurrence date");
        assert_eq!(unrelated.len(), 1);
        assert_eq!(
            unrelated[0].timezone_id.as_deref(),
            Some("America/New_York")
        );

        let error = materialize_for_date(&conn, "w1", "2099-01-05")
            .expect_err("retrieving mismatched future occurrence must fail closed");
        assert!(error.message.contains("B2"));
        let updated_b = update(
            &conn,
            "w1",
            &recurrence_b.id,
            rule(&[2], "11:30", 1800, true),
        )
        .expect("unrelated recurrence update is not blocked");
        assert_ne!(updated_b.version_id, recurrence_b.version_id);

        let error = update(
            &conn,
            "w1",
            &recurrence_a.id,
            rule(&[1], "10:00", 1800, true),
        )
        .expect_err("edit must not guess B2");
        assert!(error.message.contains("B2"));
        assert_eq!(
            get(&conn, "w1", &recurrence_a.id).unwrap().version_id,
            recurrence_a.version_id,
            "failed update must not insert or select a new version"
        );
    }

    #[test]
    fn deleting_workspace_cascades_recurrence_versions_and_occurrences() {
        let conn = test_conn();
        let recurrence =
            create(&conn, "w1", rule(&[1], "09:30", 1800, true)).expect("create recurrence");
        materialize_for_date(&conn, "w1", "2099-01-05").expect("materialize occurrence");
        conn.execute("DELETE FROM workspaces WHERE id = 'w1'", [])
            .expect("delete workspace");
        let remaining: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM block_recurrences WHERE id = ?1",
                [&recurrence.id],
                |row| row.get(0),
            )
            .expect("count recurrence");
        assert_eq!(remaining, 0);
    }

    #[test]
    fn recurrence_versions_and_selected_weekdays_are_immutable() {
        let conn = test_conn();
        let recurrence =
            create(&conn, "w1", rule(&[1, 3], "09:30", 1800, true)).expect("create recurrence");
        assert!(conn
            .execute(
                "UPDATE block_recurrence_versions SET local_start_time = '10:00' WHERE id = ?1",
                [&recurrence.version_id],
            )
            .is_err());
        assert!(conn
            .execute(
                "DELETE FROM block_recurrence_versions WHERE id = ?1",
                [&recurrence.version_id],
            )
            .is_err());
        assert!(conn
            .execute(
                "UPDATE block_recurrence_version_weekdays SET weekday = 2
                 WHERE version_id = ?1 AND weekday = 1",
                [&recurrence.version_id],
            )
            .is_err());
        assert!(conn
            .execute(
                "DELETE FROM block_recurrence_version_weekdays
                 WHERE version_id = ?1 AND weekday = 1",
                [&recurrence.version_id],
            )
            .is_err());
    }

    #[test]
    fn missing_recurrence_fields_return_domain_validation_errors() {
        let conn = test_conn();
        let weekdays = [1];
        let start = "09:30";
        let no_duration = RecurrenceRule {
            weekdays: Some(&weekdays),
            local_start_time: Some(start),
            estimated_duration_seconds: None,
            task_id: None,
            block_type: "focus",
            label: &NO_TEXT,
            flexibility: "flexible",
            note_markdown: &NO_TEXT,
            enabled: true,
            effective_from: "2026-01-01",
            effective_until: &NO_TEXT,
        };
        let error = create(&conn, "w1", no_duration).expect_err("duration is required");
        assert!(error.message.contains("duration must be a positive"));

        let no_start = RecurrenceRule {
            weekdays: Some(&weekdays),
            local_start_time: None,
            estimated_duration_seconds: Some(60),
            task_id: None,
            block_type: "focus",
            label: &NO_TEXT,
            flexibility: "flexible",
            note_markdown: &NO_TEXT,
            enabled: true,
            effective_from: "2026-01-01",
            effective_until: &NO_TEXT,
        };
        let error = create(&conn, "w1", no_start).expect_err("local start is required");
        assert!(error.message.contains("local start time is required"));

        let no_weekdays = RecurrenceRule {
            weekdays: None,
            local_start_time: Some(start),
            estimated_duration_seconds: Some(60),
            task_id: None,
            block_type: "focus",
            label: &NO_TEXT,
            flexibility: "flexible",
            note_markdown: &NO_TEXT,
            enabled: true,
            effective_from: "2026-01-01",
            effective_until: &NO_TEXT,
        };
        let error = create(&conn, "w1", no_weekdays).expect_err("weekdays are required");
        assert!(error.message.contains("ISO weekdays"));
    }

    #[test]
    fn recurrence_local_time_resolves_ambiguous_time_earlier_and_rejects_gap() {
        let ny: Tz = "America/New_York".parse().unwrap();
        let earlier = resolve_local_start(&time::parse_date("2026-11-01").unwrap(), "01:30", ny)
            .expect("resolve repeated local time");
        assert_eq!(earlier, "2026-11-01T05:30:00Z");
        let error = resolve_local_start(&time::parse_date("2026-03-08").unwrap(), "02:30", ny)
            .expect_err("reject nonexistent local time");
        assert!(error.message.contains("daylight-saving gap"));
    }

    #[test]
    fn deactivating_a_series_withdraws_future_occurrences_and_preserves_history() {
        let conn = test_conn();
        conn.execute("UPDATE workspaces SET timezone = 'UTC' WHERE id = 'w1'", [])
            .expect("set timezone");
        let recurrence =
            create(&conn, "w1", rule(&[1], "09:30", 1800, true)).expect("create recurrence");
        materialize_for_date(&conn, "w1", "2020-01-06").expect("materialize past occurrence");
        materialize_for_date(&conn, "w1", "2099-01-05").expect("materialize future occurrence");
        let future = SqliteBlockRepo
            .list_for_date(&conn, "w1", "2099-01-05")
            .expect("future occurrence")
            .remove(0);
        notification_repo::insert_block_event_delivery(
            &conn,
            "w1",
            &future.id,
            Utc::now(),
            "A planned block starts now.",
            Utc::now(),
        )
        .expect("pending block delivery");

        let deactivated = deactivate_series(&conn, "w1", &recurrence.id).expect("deactivate");
        assert_eq!(deactivated.lifecycle_state, BlockLifecycleState::Inactive);
        assert!(deactivate_series(&conn, "w1", &recurrence.id).is_err());

        let past = SqliteBlockRepo
            .list_for_date(&conn, "w1", "2020-01-06")
            .expect("past occurrence")
            .remove(0);
        assert_eq!(past.status, BlockStatus::Planned);
        assert_eq!(past.lifecycle_state, BlockLifecycleState::Active);
        let withdrawn = SqliteBlockRepo
            .get(&conn, &future.id)
            .expect("withdrawn occurrence");
        assert_eq!(withdrawn.status, BlockStatus::Cancelled);
        assert_eq!(withdrawn.lifecycle_state, BlockLifecycleState::Inactive);
        let delivery: (String, Option<String>) = conn
            .query_row(
                "SELECT status, error_message FROM notification_deliveries
                 WHERE target_entity_id = ?1",
                [&future.id],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .expect("read delivery");
        assert_eq!(delivery.0, "failed");

        materialize_for_date(&conn, "w1", "2099-01-12").expect("materialize while inactive");
        assert!(SqliteBlockRepo
            .list_for_date(&conn, "w1", "2099-01-12")
            .expect("inactive retrieval")
            .is_empty());

        let reactivated = reactivate_series(&conn, "w1", &recurrence.id).expect("reactivate");
        assert_eq!(reactivated.lifecycle_state, BlockLifecycleState::Active);
        materialize_for_date(&conn, "w1", "2099-01-12").expect("materialize after reactivation");
        let restored = SqliteBlockRepo
            .list_for_date(&conn, "w1", "2099-01-12")
            .expect("restored occurrence");
        assert_eq!(restored.len(), 1);
        assert_eq!(restored[0].status, BlockStatus::Planned);
        assert_eq!(restored[0].lifecycle_state, BlockLifecycleState::Active);
        assert!(reactivate_series(&conn, "w1", &recurrence.id).is_err());
    }

    #[test]
    fn deleting_a_series_withdraws_future_occurrences_and_preserves_history() {
        let conn = test_conn();
        conn.execute("UPDATE workspaces SET timezone = 'UTC' WHERE id = 'w1'", [])
            .expect("set timezone");
        let recurrence =
            create(&conn, "w1", rule(&[1], "09:30", 1800, true)).expect("create recurrence");
        materialize_for_date(&conn, "w1", "2020-01-06").expect("materialize history");
        let historical = SqliteBlockRepo
            .list_for_date(&conn, "w1", "2020-01-06")
            .expect("historical occurrence")
            .remove(0);
        materialize_for_date(&conn, "w1", "2099-01-05").expect("materialize occurrence");
        let occurrence = SqliteBlockRepo
            .list_for_date(&conn, "w1", "2099-01-05")
            .expect("occurrence")
            .remove(0);

        let deleted = delete_series(&conn, "w1", &recurrence.id).expect("delete series");
        assert_eq!(deleted.lifecycle_state, BlockLifecycleState::Deleted);
        let withdrawn = SqliteBlockRepo
            .get(&conn, &occurrence.id)
            .expect("future occurrence");
        assert_eq!(withdrawn.status, BlockStatus::Cancelled);
        assert_eq!(withdrawn.lifecycle_state, BlockLifecycleState::Deleted);
        let kept = SqliteBlockRepo
            .get(&conn, &historical.id)
            .expect("historical occurrence");
        assert_eq!(kept.status, BlockStatus::Planned);
        assert_eq!(kept.lifecycle_state, BlockLifecycleState::Active);
        assert!(SqliteBlockRepo
            .list_for_date(&conn, "w1", "2099-01-05")
            .expect("hidden occurrence")
            .is_empty());
        assert!(list_series(&conn, "w1", BlockLifecycleState::Active)
            .expect("active series")
            .is_empty());
        assert_eq!(
            list_series(&conn, "w1", BlockLifecycleState::Deleted)
                .expect("deleted series")
                .len(),
            1
        );
        assert!(deactivate_series(&conn, "w1", &recurrence.id).is_err());
        assert!(reactivate_series(&conn, "w1", &recurrence.id).is_err());
    }

    /// Lifecycle intervals of one series, oldest first (a closed interval
    /// precedes an open one that starts on the same local date).
    fn interval_rows(
        conn: &Connection,
        recurrence_id: &str,
    ) -> Vec<(String, Option<String>, String)> {
        let mut stmt = conn
            .prepare(
                "SELECT active_from_local_date, active_until_local_date, source
                 FROM block_recurrence_lifecycle_intervals
                 WHERE recurrence_id = ?1
                 ORDER BY active_from_local_date, active_until_local_date IS NULL",
            )
            .expect("prepare lifecycle interval query");
        stmt.query_map([recurrence_id], |row| {
            Ok((row.get(0)?, row.get(1)?, row.get(2)?))
        })
        .expect("query lifecycle intervals")
        .collect::<Result<_, _>>()
        .expect("collect lifecycle intervals")
    }

    fn insert_interval(
        conn: &Connection,
        recurrence_id: &str,
        from: &str,
        until: Option<&str>,
    ) -> rusqlite::Result<usize> {
        conn.execute(
            "INSERT INTO block_recurrence_lifecycle_intervals
             (id, workspace_id, recurrence_id, active_from_local_date,
              active_until_local_date, source, created_at)
             VALUES (?1, 'w1', ?2, ?3, ?4, 'explicit', ?5)",
            params![
                id::new_uuid(),
                recurrence_id,
                from,
                until,
                time::now_rfc3339()
            ],
        )
    }

    #[test]
    fn series_lifecycle_changes_record_explicit_lifecycle_intervals() {
        let conn = test_conn();
        conn.execute("UPDATE workspaces SET timezone = 'UTC' WHERE id = 'w1'", [])
            .expect("set timezone");
        let today = time::today_utc();
        let recurrence =
            create(&conn, "w1", rule(&[1], "09:30", 1800, true)).expect("create recurrence");
        assert_eq!(
            interval_rows(&conn, &recurrence.id),
            vec![(today.clone(), None, "explicit".to_owned())],
            "a new series opens one explicit interval at the workspace-local today"
        );

        deactivate_series(&conn, "w1", &recurrence.id).expect("deactivate");
        assert_eq!(
            interval_rows(&conn, &recurrence.id),
            vec![(today.clone(), Some(today.clone()), "explicit".to_owned())]
        );

        reactivate_series(&conn, "w1", &recurrence.id).expect("reactivate");
        assert_eq!(
            interval_rows(&conn, &recurrence.id),
            vec![
                (today.clone(), Some(today.clone()), "explicit".to_owned()),
                (today.clone(), None, "explicit".to_owned()),
            ],
            "reactivation opens a second interval while the first stays closed"
        );

        delete_series(&conn, "w1", &recurrence.id).expect("delete series");
        assert_eq!(
            interval_rows(&conn, &recurrence.id),
            vec![
                (today.clone(), Some(today.clone()), "explicit".to_owned()),
                (today.clone(), Some(today.clone()), "explicit".to_owned()),
            ],
            "soft deletion closes the open interval"
        );
        assert_eq!(
            conn.query_row(
                "SELECT COUNT(*) FROM block_recurrence_lifecycle_intervals
                 WHERE active_until_local_date IS NULL",
                [],
                |row| row.get::<_, i64>(0),
            )
            .expect("count open intervals"),
            0
        );
    }

    #[test]
    fn lifecycle_intervals_of_two_workspaces_stay_separate() {
        let conn = test_conn();
        conn.execute("UPDATE workspaces SET timezone = 'UTC' WHERE id = 'w1'", [])
            .expect("set timezone");
        let now = time::now_rfc3339();
        conn.execute(
            "INSERT INTO workspaces (id, name, timezone, created_at, updated_at)
             VALUES ('w2', 'Other synthetic workspace', 'UTC', ?1, ?1)",
            [&now],
        )
        .expect("seed second workspace");
        let today = time::today_utc();

        // Identical dates in two workspaces neither overlap nor conflict: the
        // interval rules are scoped to one series.
        let first = create(&conn, "w1", rule(&[1], "09:30", 1800, true)).expect("first recurrence");
        let second =
            create(&conn, "w2", rule(&[1], "09:30", 1800, true)).expect("second recurrence");
        assert_eq!(interval_rows(&conn, &first.id).len(), 1);
        assert_eq!(interval_rows(&conn, &second.id).len(), 1);

        deactivate_series(&conn, "w1", &first.id).expect("deactivate first workspace series");
        assert_eq!(
            interval_rows(&conn, &second.id),
            vec![(today, None, "explicit".to_owned())],
            "the other workspace's series keeps its open interval"
        );
        let open_by_workspace: Vec<(String, i64)> = {
            let mut stmt = conn
                .prepare(
                    "SELECT workspace_id, COUNT(*) FROM block_recurrence_lifecycle_intervals
                     WHERE active_until_local_date IS NULL
                     GROUP BY workspace_id ORDER BY workspace_id",
                )
                .expect("prepare workspace isolation query");
            stmt.query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
                .expect("query workspace isolation")
                .collect::<Result<_, _>>()
                .expect("collect workspace isolation")
        };
        assert_eq!(open_by_workspace, vec![("w2".to_owned(), 1)]);
    }

    #[test]
    fn lifecycle_interval_writes_preserve_occurrence_outcomes_and_history() {
        let conn = test_conn();
        conn.execute("UPDATE workspaces SET timezone = 'UTC' WHERE id = 'w1'", [])
            .expect("set timezone");
        let recurrence =
            create(&conn, "w1", rule(&[1], "09:30", 1800, true)).expect("create recurrence");
        materialize_for_date(&conn, "w1", "2020-01-06").expect("materialize past occurrence");
        materialize_for_date(&conn, "w1", "2099-01-05").expect("materialize recorded occurrence");
        conn.execute(
            "UPDATE blocks SET status = 'completed' WHERE local_date = '2099-01-05'",
            [],
        )
        .expect("complete the occurrence");

        deactivate_series(&conn, "w1", &recurrence.id).expect("deactivate");
        reactivate_series(&conn, "w1", &recurrence.id).expect("reactivate");

        // A recorded outcome is never rewritten, inferred, or dropped by the
        // interval bookkeeping, and a past planned row still counts as planned.
        let recorded = SqliteBlockRepo
            .list_for_date(&conn, "w1", "2099-01-05")
            .expect("recorded occurrence")
            .remove(0);
        assert_eq!(recorded.status, BlockStatus::Completed);
        assert_eq!(recorded.lifecycle_state, BlockLifecycleState::Active);
        let past = SqliteBlockRepo
            .list_for_date(&conn, "w1", "2020-01-06")
            .expect("past occurrence")
            .remove(0);
        assert_eq!(past.status, BlockStatus::Planned);
        assert_eq!(interval_rows(&conn, &recurrence.id).len(), 2);
        let occurrences: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM blocks WHERE recurrence_id = ?1",
                [&recurrence.id],
                |row| row.get(0),
            )
            .expect("count occurrences");
        assert_eq!(occurrences, 2, "interval writes create no occurrence rows");
    }

    #[test]
    fn out_of_order_or_duplicate_open_interval_writes_are_rejected() {
        let conn = test_conn();
        conn.execute("UPDATE workspaces SET timezone = 'UTC' WHERE id = 'w1'", [])
            .expect("set timezone");
        let recurrence =
            create(&conn, "w1", rule(&[1], "09:30", 1800, true)).expect("create recurrence");
        assert!(insert_interval(&conn, &recurrence.id, "2000-01-01", Some("2000-01-31")).is_err());
        assert!(insert_interval(&conn, &recurrence.id, "2099-01-01", None).is_err());
        assert_eq!(interval_rows(&conn, &recurrence.id).len(), 1);
    }

    #[test]
    fn occurrences_copy_the_version_task_id_and_keep_their_snapshot() {
        let conn = test_conn();
        conn.execute("UPDATE workspaces SET timezone = 'UTC' WHERE id = 'w1'", [])
            .expect("set timezone");
        seed_task(&conn, "task-a");
        seed_task(&conn, "task-b");
        let monday = [1];
        let mut first_rule = rule(&monday, "09:30", 1800, true);
        first_rule.task_id = Some("task-a");
        let recurrence = create(&conn, "w1", first_rule).expect("create recurrence with task");
        assert_eq!(recurrence.task_id.as_deref(), Some("task-a"));
        materialize_for_date(&conn, "w1", "2020-01-06").expect("past occurrence");
        materialize_for_date(&conn, "w1", "2099-01-05").expect("future occurrence");
        let past = SqliteBlockRepo
            .list_for_date(&conn, "w1", "2020-01-06")
            .expect("past occurrence")
            .remove(0);
        assert_eq!(past.task_id.as_deref(), Some("task-a"));

        let mut second_rule = rule(&monday, "10:00", 1800, true);
        second_rule.task_id = Some("task-b");
        update(&conn, "w1", &recurrence.id, second_rule).expect("edit recurrence");
        let kept = SqliteBlockRepo
            .list_for_date(&conn, "w1", "2020-01-06")
            .expect("kept past occurrence")
            .remove(0);
        assert_eq!(kept.task_id.as_deref(), Some("task-a"));
        let adopted = SqliteBlockRepo
            .list_for_date(&conn, "w1", "2099-01-05")
            .expect("future occurrence")
            .remove(0);
        assert_eq!(adopted.task_id.as_deref(), Some("task-b"));
    }

    #[test]
    fn resolve_next_weekday_date_returns_today_a_later_day_and_next_week() {
        // 2026-10-21 is a Wednesday (ISO weekday 3).
        assert_eq!(
            resolve_next_weekday_date("2026-10-21", 3).expect("same weekday"),
            "2026-10-21"
        );
        assert_eq!(
            resolve_next_weekday_date("2026-10-21", 5).expect("later weekday"),
            "2026-10-23"
        );
        assert_eq!(
            resolve_next_weekday_date("2026-10-21", 1).expect("wraps to next week"),
            "2026-10-26"
        );
        assert_eq!(
            resolve_next_weekday_date("2026-10-21", 2).expect("wraps to next week"),
            "2026-10-27"
        );
    }

    #[test]
    fn scheduled_series_are_listed_once_without_materializing_future_rows() {
        let conn = test_conn();
        conn.execute("UPDATE workspaces SET timezone = 'UTC' WHERE id = 'w1'", [])
            .expect("set timezone");
        let recurrence =
            create(&conn, "w1", rule(&[1, 3, 5], "09:30", 1800, true)).expect("create recurrence");

        let scheduled = list_scheduled_series(&conn, "w1").expect("list scheduled series");

        assert_eq!(scheduled.len(), 1);
        assert_eq!(scheduled[0].recurrence.id, recurrence.id);
        assert!(scheduled[0].next_occurrence_date >= workspace_local_today(&conn, "w1").unwrap());
        let row_count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM blocks WHERE recurrence_id = ?1",
                [&recurrence.id],
                |row| row.get(0),
            )
            .expect("count materialized occurrences");
        assert_eq!(row_count, 0);
    }

    #[test]
    fn startup_reconciliation_catches_up_only_through_today_and_is_idempotent() {
        let conn = test_conn();
        conn.execute("UPDATE workspaces SET timezone = 'UTC' WHERE id = 'w1'", [])
            .expect("set timezone");
        let recurrence = create(
            &conn,
            "w1",
            rule(&[1, 2, 3, 4, 5, 6, 7], "09:30", 1800, true),
        )
        .expect("create recurrence");
        let today = time::parse_date(&workspace_local_today(&conn, "w1").unwrap())
            .expect("parse local today");
        let active_from = today
            .checked_sub_days(chrono::Days::new(4))
            .expect("four days ago")
            .format("%Y-%m-%d")
            .to_string();
        conn.execute(
            "UPDATE block_recurrence_lifecycle_intervals
             SET active_from_local_date = ?2 WHERE recurrence_id = ?1",
            params![recurrence.id, active_from],
        )
        .expect("simulate missed startup days");

        reconcile_active_occurrences(&conn).expect("reconcile missed dates");
        reconcile_active_occurrences(&conn).expect("reconcile idempotently");

        let rows: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM blocks WHERE recurrence_id = ?1",
                [&recurrence.id],
                |row| row.get(0),
            )
            .expect("count reconciled occurrences");
        assert_eq!(rows, 5);
        let future_rows: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM blocks WHERE recurrence_id = ?1 AND local_date > ?2",
                params![recurrence.id, today.format("%Y-%m-%d").to_string()],
                |row| row.get(0),
            )
            .expect("count future occurrences");
        assert_eq!(future_rows, 0);
        let non_planned: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM blocks WHERE recurrence_id = ?1 AND status != 'planned'",
                [&recurrence.id],
                |row| row.get(0),
            )
            .expect("count inferred outcomes");
        assert_eq!(non_planned, 0);
    }

    #[test]
    fn invalid_weekday_is_rejected() {
        for weekday in [0, 8, -1] {
            let error =
                resolve_next_weekday_date("2026-10-21", weekday).expect_err("invalid ISO weekday");
            assert_eq!(error.message, "Select an ISO weekday from 1 through 7.");
        }
        assert!(resolve_next_weekday_date("not-a-date", 1).is_err());
    }
}
