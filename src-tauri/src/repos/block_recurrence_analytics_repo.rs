//! Workspace-scoped, read-only habit analytics for recurring block series.
//!
//! Implements the settled Phase 8.7.10 contract of
//! `docs/data-entities-modeling.md` section 8.9 as the Phase 8.7.12 aggregation
//! API: the requested local-date range is validated and its end is clamped to
//! the workspace-local today, expected dates are derived day by day from the
//! immutable recurrence versions, their enabled ISO weekdays, and the series'
//! active lifecycle intervals, and outcome counts are read from persisted
//! occurrence rows only.
//!
//! This module never writes. It inserts, updates, and deletes no row, never
//! materializes an occurrence, and never changes a block, recurrence, task,
//! notification, or task-progress value. Both counts and the expected set are
//! deterministic functions of persisted data and the echoed effective range, so
//! repeated queries over unchanged data return identical results.

use std::collections::{BTreeMap, BTreeSet};

use chrono::{Datelike, Days, NaiveDate, Utc};
use rusqlite::{params, Connection};

use crate::commands::dto::{
    RecurrenceAnalyticsCountsDto, RecurrenceAnalyticsCoverageDto,
    RecurrenceAnalyticsCoverageIntervalDto, RecurrenceAnalyticsRateDto,
    RecurrenceAnalyticsResponse, RecurrenceAnalyticsSeriesDto, RecurrenceAnalyticsWeekBucketDto,
};
use crate::error::AppError;
use crate::repos::block_recurrence_repo::{local_date_of, parse_timezone, workspace_timezone};
use crate::time;

/// A series' active lifecycle interval as analytics reads it. `until` is NULL
/// for an open (still active) interval.
#[derive(Debug, Clone, PartialEq, Eq)]
struct LifecycleInterval {
    from: NaiveDate,
    until: Option<NaiveDate>,
}

impl LifecycleInterval {
    fn contains(&self, date: NaiveDate) -> bool {
        date >= self.from && self.until.is_none_or(|until| date <= until)
    }
}

/// An immutable recurrence version as analytics selects it for one local date.
#[derive(Debug, Clone, PartialEq, Eq)]
struct VersionWindow {
    id: String,
    effective_from: NaiveDate,
    effective_until: Option<NaiveDate>,
    /// Enabled ISO weekdays, Monday = 1 … Sunday = 7.
    weekdays: BTreeSet<i64>,
    label: Option<String>,
}

impl VersionWindow {
    fn covers(&self, date: NaiveDate) -> bool {
        date >= self.effective_from && self.effective_until.is_none_or(|until| date <= until)
    }
}

/// The persisted occurrence outcomes of one series on one local date.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
struct DailyOutcomes {
    planned: i64,
    active: i64,
    completed: i64,
    skipped: i64,
    cancelled: i64,
}

impl DailyOutcomes {
    /// A local date is recorded when it has a row in one of these statuses.
    /// `planned` is recorded but not yet outcome'd; a missing row is never
    /// presented as completed or failed.
    fn is_recorded(&self) -> bool {
        self.planned + self.active + self.completed + self.skipped > 0
    }
}

/// Validate the requested local-date range and clamp its end to the
/// workspace-local `today`. Both bounds are inclusive; `from_date` is never
/// clamped, so a range that starts in the future keeps its echoed start and
/// yields no in-range dates. `today` is injected so the clamp is pure and
/// testable without a clock.
pub fn effective_range(
    from_date: &str,
    to_date: &str,
    today: &str,
) -> Result<(String, String), AppError> {
    let from = time::parse_date(from_date)?;
    let to = time::parse_date(to_date)?;
    let today = time::parse_date(today)?;
    if from > to {
        return Err(AppError::validation(
            "The analytics range start date cannot be after its end date.",
        ));
    }
    Ok((format_date(from), format_date(to.min(today))))
}

/// The Monday of the ISO week containing `date`, as `YYYY-MM-DD`.
pub fn week_start_monday(date: &str) -> Result<String, AppError> {
    Ok(format_date(monday_of(time::parse_date(date)?)))
}

/// Workspace-scoped, read-only analytics for every non-deleted recurring block
/// series in `[from_date, to_date]`, with the effective end clamped to the
/// workspace-local today.
pub fn analytics_for_workspace(
    conn: &Connection,
    workspace_id: &str,
    from_date: &str,
    to_date: &str,
) -> Result<RecurrenceAnalyticsResponse, AppError> {
    let timezone = parse_timezone(&workspace_timezone(conn, workspace_id)?)?;
    let today = local_date_of(Utc::now(), timezone);
    let (effective_from, effective_to) = effective_range(from_date, to_date, &today)?;
    let effective_from_date = time::parse_date(&effective_from)?;
    let effective_to_date = time::parse_date(&effective_to)?;

    let mut series = Vec::new();
    for (recurrence_id, current_version_id) in load_series(conn, workspace_id)? {
        let intervals = load_lifecycle_intervals(conn, workspace_id, &recurrence_id)?;
        let versions = load_version_windows(conn, &recurrence_id)?;
        let outcomes = load_daily_outcomes(
            conn,
            workspace_id,
            &recurrence_id,
            &effective_from,
            &effective_to,
        )?;
        series.push(aggregate_series(
            &recurrence_id,
            current_version_id.as_deref(),
            &versions,
            &intervals,
            &outcomes,
            effective_from_date,
            effective_to_date,
        ));
    }

    Ok(RecurrenceAnalyticsResponse {
        effective_from,
        effective_to,
        series,
    })
}

/// Aggregate one series over an already clamped effective range. Pure: it only
/// reads the loaded rows passed in, so its behavior is fully determined by its
/// arguments.
fn aggregate_series(
    series_id: &str,
    current_version_id: Option<&str>,
    versions: &[VersionWindow],
    intervals: &[LifecycleInterval],
    outcomes: &BTreeMap<String, DailyOutcomes>,
    effective_from: NaiveDate,
    effective_to: NaiveDate,
) -> RecurrenceAnalyticsSeriesDto {
    let coverage_start = intervals.iter().map(|interval| interval.from).min();
    let first_monday = monday_of(effective_from);
    let bucket_count = week_bucket_count(first_monday, monday_of(effective_to));
    let mut weekly: Vec<RecurrenceAnalyticsWeekBucketDto> = (0..bucket_count)
        .map(|index| RecurrenceAnalyticsWeekBucketDto {
            week_start: format_date(first_monday + Days::new(index as u64 * 7)),
            ..Default::default()
        })
        .collect();

    // Expected dates of `E` (a date covered by the series and inside the
    // requested range) whose occurrence row carries a cancelled status. Those
    // dates leave the completion-rate denominator.
    let mut cancelled_within_expectations = 0_i64;
    // Latest in-range date that is expected or observed; it selects the display
    // label's version.
    let mut latest_covered: Option<NaiveDate> = None;

    // A series without lifecycle intervals has no expected dates at all: every
    // count stays zero and the rate is hidden. Its rows are not reinterpreted
    // as activity, because nothing in the range is in coverage.
    if let Some(coverage_start) = coverage_start {
        let mut day = effective_from;
        while day <= effective_to {
            let observed = outcomes.get(&format_date(day)).copied();
            let expected = day >= coverage_start
                && intervals.iter().any(|interval| interval.contains(day))
                && version_covering(versions, day)
                    .is_some_and(|version| version.weekdays.contains(&iso_weekday(day)));
            let bucket = bucket_index(day, first_monday, bucket_count);

            if expected {
                let daily = observed.unwrap_or_default();
                if daily.cancelled > 0 {
                    cancelled_within_expectations += 1;
                }
                if let Some(index) = bucket {
                    weekly[index].scheduled += 1;
                    weekly[index].cancelled += daily.cancelled;
                    if daily.is_recorded() {
                        weekly[index].completed += daily.completed;
                        weekly[index].started += daily.active;
                        weekly[index].skipped += daily.skipped;
                    } else {
                        weekly[index].unrecorded += 1;
                    }
                }
            } else if let Some(daily) = observed.filter(|daily| daily.cancelled > 0) {
                // Reconciliation (contract section 8.9): the out-of-schedule
                // exclusion applies to the expected set, but a withdrawal is
                // recorded history and stays in the cancelled bucket of its
                // own local date. Every other outcome on an out-of-schedule
                // date is ignored entirely.
                if let Some(index) = bucket {
                    weekly[index].cancelled += daily.cancelled;
                }
            }

            if expected || observed.is_some() {
                latest_covered = Some(day);
            }

            match day.succ_opt() {
                Some(next) => day = next,
                None => break,
            }
        }
    }

    // Totals are the sum of the emitted buckets, so per-week counts add up to
    // the series totals for the same effective range by construction.
    let counts = sum_weekly(&weekly);
    let denominator = counts.scheduled - cancelled_within_expectations;
    let rate = (denominator > 0).then(|| RecurrenceAnalyticsRateDto {
        denominator,
        completed: counts.completed,
        percent: completion_percent(counts.completed, denominator),
    });

    // The label is the snapshot of the version covering the latest expected or
    // observed in-range date (latest `effective_from` precedence). Labels are
    // never rewritten onto older history, so a rename only affects later dates.
    let current_label = current_version_id
        .and_then(|current| versions.iter().find(|version| version.id == current))
        .and_then(|version| version.label.clone());
    let (display_label, display_label_is_current_fallback) =
        match latest_covered.and_then(|date| version_covering(versions, date)) {
            Some(version) => (version.label.clone(), false),
            None => (current_label, true),
        };

    RecurrenceAnalyticsSeriesDto {
        series_id: series_id.to_owned(),
        display_label,
        display_label_is_current_fallback,
        coverage: RecurrenceAnalyticsCoverageDto {
            start: coverage_start.map(format_date),
            intervals: intervals
                .iter()
                .map(|interval| RecurrenceAnalyticsCoverageIntervalDto {
                    from: format_date(interval.from),
                    until: interval.until.map(format_date),
                })
                .collect(),
        },
        counts,
        rate,
        weekly,
    }
}

/// The version covering `date`, preferring the greatest `effective_from` and,
/// on a tie, the greatest version id. Immutable version edits can leave
/// overlapping open windows, so the selection must be deterministic.
fn version_covering(versions: &[VersionWindow], date: NaiveDate) -> Option<&VersionWindow> {
    versions
        .iter()
        .filter(|version| version.covers(date))
        .max_by(|left, right| {
            (left.effective_from, left.id.as_str()).cmp(&(right.effective_from, right.id.as_str()))
        })
}

/// Monday of the ISO week containing `date` (ISO weeks start Monday).
fn monday_of(date: NaiveDate) -> NaiveDate {
    date - Days::new(u64::from(date.weekday().num_days_from_monday()))
}

/// Number of contiguous weekly buckets from `first_monday` through
/// `last_monday`, zero when the effective range is inverted.
fn week_bucket_count(first_monday: NaiveDate, last_monday: NaiveDate) -> usize {
    if last_monday < first_monday {
        return 0;
    }
    ((last_monday - first_monday).num_days() / 7 + 1) as usize
}

fn bucket_index(day: NaiveDate, first_monday: NaiveDate, bucket_count: usize) -> Option<usize> {
    let offset = (day - first_monday).num_days();
    if offset < 0 {
        return None;
    }
    let index = (offset / 7) as usize;
    (index < bucket_count).then_some(index)
}

fn sum_weekly(weekly: &[RecurrenceAnalyticsWeekBucketDto]) -> RecurrenceAnalyticsCountsDto {
    let mut counts = RecurrenceAnalyticsCountsDto::default();
    for bucket in weekly {
        counts.scheduled += bucket.scheduled;
        counts.completed += bucket.completed;
        counts.started += bucket.started;
        counts.skipped += bucket.skipped;
        counts.cancelled += bucket.cancelled;
        counts.unrecorded += bucket.unrecorded;
    }
    counts
}

/// Rounded completion percentage. Only called with a positive denominator.
fn completion_percent(completed: i64, denominator: i64) -> i64 {
    (100.0 * completed as f64 / denominator as f64).round() as i64
}

fn format_date(date: NaiveDate) -> String {
    date.format("%Y-%m-%d").to_string()
}

/// ISO weekday, Monday = 1 … Sunday = 7.
fn iso_weekday(date: NaiveDate) -> i64 {
    i64::from(date.weekday().number_from_monday())
}

/// Non-deleted series of a workspace (`active` and `inactive`), ordered by
/// series id so repeated calls return a stable order.
fn load_series(
    conn: &Connection,
    workspace_id: &str,
) -> Result<Vec<(String, Option<String>)>, AppError> {
    let mut stmt = conn.prepare(
        "SELECT id, current_version_id FROM block_recurrences
         WHERE workspace_id = ?1 AND lifecycle_state != 'deleted'
         ORDER BY id",
    )?;
    let rows = stmt.query_map([workspace_id], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, Option<String>>(1)?))
    })?;
    Ok(rows.collect::<Result<Vec<_>, _>>()?)
}

/// Active lifecycle intervals of one series, oldest first. The full list is
/// returned for the coverage statement and its paused ranges.
fn load_lifecycle_intervals(
    conn: &Connection,
    workspace_id: &str,
    recurrence_id: &str,
) -> Result<Vec<LifecycleInterval>, AppError> {
    let mut stmt = conn.prepare(
        "SELECT active_from_local_date, active_until_local_date
         FROM block_recurrence_lifecycle_intervals
         WHERE workspace_id = ?1 AND recurrence_id = ?2
         ORDER BY active_from_local_date, active_until_local_date IS NULL, id",
    )?;
    let rows = stmt.query_map(params![workspace_id, recurrence_id], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, Option<String>>(1)?))
    })?;
    let mut intervals = Vec::new();
    for row in rows {
        let (from, until) = row?;
        intervals.push(LifecycleInterval {
            from: time::parse_date(&from)?,
            until: until.as_deref().map(time::parse_date).transpose()?,
        });
    }
    Ok(intervals)
}

/// Immutable versions of one series with their enabled weekdays, earliest
/// `effective_from` first.
fn load_version_windows(
    conn: &Connection,
    recurrence_id: &str,
) -> Result<Vec<VersionWindow>, AppError> {
    let mut stmt = conn.prepare(
        "SELECT v.id, v.effective_from, v.effective_until, v.label, w.weekday
         FROM block_recurrence_versions v
         LEFT JOIN block_recurrence_version_weekdays w ON w.version_id = v.id
         WHERE v.recurrence_id = ?1
         ORDER BY v.effective_from, v.id, w.weekday",
    )?;
    let rows = stmt.query_map([recurrence_id], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, Option<String>>(2)?,
            row.get::<_, Option<String>>(3)?,
            row.get::<_, Option<i64>>(4)?,
        ))
    })?;

    let mut versions: Vec<VersionWindow> = Vec::new();
    for row in rows {
        let (id, effective_from, effective_until, label, weekday) = row?;
        let index = if versions.last().is_some_and(|last| last.id == id) {
            versions.len() - 1
        } else {
            versions.push(VersionWindow {
                id,
                effective_from: time::parse_date(&effective_from)?,
                effective_until: effective_until
                    .as_deref()
                    .map(time::parse_date)
                    .transpose()?,
                weekdays: BTreeSet::new(),
                label,
            });
            versions.len() - 1
        };
        if let Some(weekday) = weekday {
            versions[index].weekdays.insert(weekday);
        }
    }
    Ok(versions)
}

/// Persisted occurrence rows of one series inside the effective range, grouped
/// by local date and status. Deleted occurrences are excluded; a row outside
/// the requested range is never read.
fn load_daily_outcomes(
    conn: &Connection,
    workspace_id: &str,
    recurrence_id: &str,
    from_date: &str,
    to_date: &str,
) -> Result<BTreeMap<String, DailyOutcomes>, AppError> {
    let mut stmt = conn.prepare(
        "SELECT local_date, status, COUNT(*) FROM blocks
         WHERE workspace_id = ?1 AND recurrence_id = ?2
           AND lifecycle_state != 'deleted'
           AND local_date IS NOT NULL AND local_date >= ?3 AND local_date <= ?4
         GROUP BY local_date, status",
    )?;
    let rows = stmt.query_map(
        params![workspace_id, recurrence_id, from_date, to_date],
        |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, i64>(2)?,
            ))
        },
    )?;

    let mut outcomes: BTreeMap<String, DailyOutcomes> = BTreeMap::new();
    for row in rows {
        let (local_date, status, count) = row?;
        let daily = outcomes.entry(local_date).or_default();
        match status.as_str() {
            "planned" => daily.planned += count,
            "active" => daily.active += count,
            "completed" => daily.completed += count,
            "skipped" => daily.skipped += count,
            "cancelled" => daily.cancelled += count,
            _ => {}
        }
    }
    Ok(outcomes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::repos::block_recurrence_repo::{self, RecurrenceRule};
    use crate::test_utils::test_conn;

    static NO_TEXT: Option<String> = None;

    const UTC_WORKSPACE: &str = "UTC";

    fn use_utc_workspace(conn: &Connection) {
        conn.execute("UPDATE workspaces SET timezone = 'UTC' WHERE id = 'w1'", [])
            .expect("set workspace timezone");
    }

    fn rule<'a>(
        weekdays: &'a [i64],
        effective_from: &'a str,
        label: &'a Option<String>,
    ) -> RecurrenceRule<'a> {
        RecurrenceRule {
            weekdays: Some(weekdays),
            local_start_time: Some("09:30"),
            estimated_duration_seconds: Some(1800),
            task_id: None,
            block_type: "focus",
            label,
            flexibility: "flexible",
            note_markdown: &NO_TEXT,
            enabled: true,
            effective_from,
            effective_until: &NO_TEXT,
        }
    }

    /// Pin the series' open lifecycle interval to a fixed synthetic start date,
    /// so expectations never depend on the machine's current date.
    fn set_open_interval(conn: &Connection, workspace_id: &str, recurrence_id: &str, from: &str) {
        let changed = conn
            .execute(
                "UPDATE block_recurrence_lifecycle_intervals
                 SET active_from_local_date = ?3
                 WHERE workspace_id = ?1 AND recurrence_id = ?2
                   AND active_until_local_date IS NULL",
                params![workspace_id, recurrence_id, from],
            )
            .expect("pin open lifecycle interval");
        assert_eq!(changed, 1, "the series has exactly one open interval");
    }

    /// Close the series' open lifecycle interval at a fixed synthetic end date.
    fn set_closed_interval(
        conn: &Connection,
        workspace_id: &str,
        recurrence_id: &str,
        from: &str,
        until: &str,
    ) {
        let changed = conn
            .execute(
                "UPDATE block_recurrence_lifecycle_intervals
                 SET active_from_local_date = ?3, active_until_local_date = ?4
                 WHERE workspace_id = ?1 AND recurrence_id = ?2
                   AND active_until_local_date IS NULL",
                params![workspace_id, recurrence_id, from, until],
            )
            .expect("pin closed lifecycle interval");
        assert_eq!(changed, 1, "the series has exactly one open interval");
    }

    fn set_lifecycle(conn: &Connection, recurrence_id: &str, state: &str) {
        conn.execute(
            "UPDATE block_recurrences SET lifecycle_state = ?2 WHERE id = ?1",
            params![recurrence_id, state],
        )
        .expect("set series lifecycle state");
    }

    fn materialize(conn: &Connection, workspace_id: &str, date: &str) {
        block_recurrence_repo::materialize_for_date(conn, workspace_id, date)
            .expect("materialize synthetic occurrence");
    }

    fn set_status(conn: &Connection, recurrence_id: &str, local_date: &str, status: &str) {
        let changed = conn
            .execute(
                "UPDATE blocks SET status = ?3 WHERE recurrence_id = ?1 AND local_date = ?2",
                params![recurrence_id, local_date, status],
            )
            .expect("set synthetic occurrence status");
        assert_eq!(changed, 1, "expected one occurrence row on {local_date}");
    }

    fn counts(
        scheduled: i64,
        completed: i64,
        started: i64,
        skipped: i64,
        cancelled: i64,
        unrecorded: i64,
    ) -> RecurrenceAnalyticsCountsDto {
        RecurrenceAnalyticsCountsDto {
            scheduled,
            completed,
            started,
            skipped,
            cancelled,
            unrecorded,
        }
    }

    fn analytics(conn: &Connection, from: &str, to: &str) -> RecurrenceAnalyticsResponse {
        analytics_for_workspace(conn, "w1", from, to).expect("analytics query")
    }

    fn entry<'a>(
        response: &'a RecurrenceAnalyticsResponse,
        series_id: &str,
    ) -> &'a RecurrenceAnalyticsSeriesDto {
        response
            .series
            .iter()
            .find(|series| series.series_id == series_id)
            .expect("series present in response")
    }

    fn sum_buckets(series: &RecurrenceAnalyticsSeriesDto) -> RecurrenceAnalyticsCountsDto {
        sum_weekly(&series.weekly)
    }

    fn workspace_local_today(conn: &Connection, workspace_id: &str) -> String {
        let timezone = parse_timezone(&workspace_timezone(conn, workspace_id).unwrap()).unwrap();
        local_date_of(Utc::now(), timezone)
    }

    fn table_counts(conn: &Connection) -> (i64, i64, i64, i64, i64) {
        let read = |table: &str| -> i64 {
            conn.query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |row| {
                row.get(0)
            })
            .expect("count rows")
        };
        (
            read("blocks"),
            read("block_recurrences"),
            read("block_recurrence_versions"),
            read("block_recurrence_version_weekdays"),
            read("block_recurrence_lifecycle_intervals"),
        )
    }

    fn other_workspace(conn: &Connection, workspace_id: &str) {
        let now = time::now_rfc3339();
        conn.execute(
            "INSERT INTO workspaces (id, name, timezone, created_at, updated_at)
             VALUES (?1, 'Other synthetic workspace', ?2, ?3, ?3)",
            params![workspace_id, UTC_WORKSPACE, now],
        )
        .expect("seed second workspace");
    }

    #[test]
    fn cancelled_expected_dates_leave_the_denominator_and_a_zero_denominator_hides_the_rate() {
        let conn = test_conn();
        use_utc_workspace(&conn);

        // Monday series: one completed, one cancelled, one expected date with
        // no row at all.
        let monday = block_recurrence_repo::create(&conn, "w1", rule(&[1], "2000-01-01", &NO_TEXT))
            .expect("create monday series");
        set_open_interval(&conn, "w1", &monday.id, "2020-01-01");
        materialize(&conn, "w1", "2020-01-06");
        materialize(&conn, "w1", "2020-01-13");
        set_status(&conn, &monday.id, "2020-01-06", "completed");
        set_status(&conn, &monday.id, "2020-01-13", "cancelled");

        // Tuesday series whose every expected date is withdrawn: the
        // denominator is zero, so the rate must stay hidden.
        let tuesday =
            block_recurrence_repo::create(&conn, "w1", rule(&[2], "2000-01-01", &NO_TEXT))
                .expect("create tuesday series");
        set_open_interval(&conn, "w1", &tuesday.id, "2020-01-01");
        materialize(&conn, "w1", "2020-01-07");
        materialize(&conn, "w1", "2020-01-14");
        set_status(&conn, &tuesday.id, "2020-01-07", "cancelled");
        set_status(&conn, &tuesday.id, "2020-01-14", "cancelled");

        let response = analytics(&conn, "2020-01-06", "2020-01-20");
        assert_eq!(response.effective_from, "2020-01-06");
        assert_eq!(response.effective_to, "2020-01-20");
        assert_eq!(response.series.len(), 2);
        assert!(
            response
                .series
                .windows(2)
                .all(|pair| pair[0].series_id < pair[1].series_id),
            "series are ordered by series id ascending"
        );

        let monday_entry = entry(&response, &monday.id);
        assert_eq!(monday_entry.counts, counts(3, 1, 0, 0, 1, 2));
        assert_eq!(
            monday_entry.rate,
            Some(RecurrenceAnalyticsRateDto {
                denominator: 2,
                completed: 1,
                percent: 50,
            }),
            "the withdrawn expected date leaves the denominator"
        );

        let tuesday_entry = entry(&response, &tuesday.id);
        assert_eq!(tuesday_entry.counts, counts(2, 0, 0, 0, 2, 2));
        assert_eq!(
            tuesday_entry.rate, None,
            "a zero denominator hides the rate"
        );
    }

    #[test]
    fn version_edits_keep_one_series_id_and_use_the_latest_covering_version_label() {
        let conn = test_conn();
        use_utc_workspace(&conn);

        let first_label = Some("Morning walk".to_string());
        let series =
            block_recurrence_repo::create(&conn, "w1", rule(&[1], "2000-01-01", &first_label))
                .expect("create series");
        set_open_interval(&conn, "w1", &series.id, "2020-01-01");
        materialize(&conn, "w1", "2020-01-06");
        set_status(&conn, &series.id, "2020-01-06", "completed");

        let second_label = Some("Morning walk and midweek reset".to_string());
        let edited = block_recurrence_repo::update(
            &conn,
            "w1",
            &series.id,
            rule(&[1, 3], "2020-01-13", &second_label),
        )
        .expect("edit series");
        assert_eq!(edited.id, series.id, "an edit never splits series history");

        let response = analytics(&conn, "2020-01-06", "2020-01-15");
        assert_eq!(
            response.series.len(),
            1,
            "one series, not one row per version"
        );
        let current = entry(&response, &series.id);
        assert_eq!(current.series_id, edited.id);
        assert_eq!(current.display_label.as_deref(), second_label.as_deref());
        assert!(
            !current.display_label_is_current_fallback,
            "the in-range label comes from a covering version"
        );
        assert_eq!(
            current.counts,
            counts(3, 1, 0, 0, 0, 2),
            "2020-01-06 is expected through v1 and 2020-01-13/2020-01-15 through v2, \
             while the v1-only Wednesday 2020-01-08 is not expected"
        );

        let early = analytics(&conn, "2019-01-01", "2019-12-31");
        assert_eq!(early.series.len(), 1);
        let out_of_range = entry(&early, &series.id);
        assert_eq!(out_of_range.series_id, series.id);
        assert_eq!(out_of_range.counts, counts(0, 0, 0, 0, 0, 0));
        assert_eq!(out_of_range.rate, None);
        assert!(
            out_of_range.display_label_is_current_fallback,
            "a range with no in-coverage date falls back to the current version"
        );
        assert_eq!(
            out_of_range.display_label.as_deref(),
            second_label.as_deref()
        );
        assert_eq!(out_of_range.coverage.start.as_deref(), Some("2020-01-01"));
        assert_eq!(
            out_of_range.coverage.intervals,
            vec![RecurrenceAnalyticsCoverageIntervalDto {
                from: "2020-01-01".to_string(),
                until: None,
            }]
        );
    }

    #[test]
    fn only_weekdays_enabled_in_the_covering_version_are_expected() {
        let conn = test_conn();
        use_utc_workspace(&conn);

        let series =
            block_recurrence_repo::create(&conn, "w1", rule(&[1, 3], "2000-01-01", &NO_TEXT))
                .expect("create series");
        set_open_interval(&conn, "w1", &series.id, "2020-01-01");

        let both = analytics(&conn, "2020-01-06", "2020-01-12");
        assert_eq!(
            entry(&both, &series.id).counts,
            counts(2, 0, 0, 0, 0, 2),
            "Monday and Wednesday are expected; the other five days are not"
        );

        materialize(&conn, "w1", "2020-01-06");
        materialize(&conn, "w1", "2020-01-08");
        block_recurrence_repo::update(&conn, "w1", &series.id, rule(&[1], "2020-01-07", &NO_TEXT))
            .expect("edit weekdays");

        let monday_only = analytics(&conn, "2020-01-06", "2020-01-12");
        assert_eq!(
            entry(&monday_only, &series.id).counts,
            counts(1, 0, 0, 0, 0, 0),
            "Wednesday is disabled from 2020-01-07 on, so it stops being expected"
        );
    }

    #[test]
    fn expected_date_without_a_row_is_unrecorded_and_pre_coverage_dates_are_excluded() {
        let conn = test_conn();
        use_utc_workspace(&conn);

        let series = block_recurrence_repo::create(&conn, "w1", rule(&[1], "2000-01-01", &NO_TEXT))
            .expect("create series");
        set_open_interval(&conn, "w1", &series.id, "2020-01-13");

        // 2020-01-06 predates coverage: a completed row there creates no
        // expectation and is not reinterpreted as an in-schedule outcome.
        materialize(&conn, "w1", "2020-01-06");
        set_status(&conn, &series.id, "2020-01-06", "completed");
        // 2020-01-13 is materialized and still pending: recorded, not outcome'd.
        materialize(&conn, "w1", "2020-01-13");

        let response = analytics(&conn, "2020-01-06", "2020-01-20");
        let current = entry(&response, &series.id);
        assert_eq!(current.coverage.start.as_deref(), Some("2020-01-13"));
        assert_eq!(
            current.counts,
            counts(2, 0, 0, 0, 0, 1),
            "2020-01-20 has no row; the pending 2020-01-13 row is recorded"
        );
        assert_eq!(
            current.rate,
            Some(RecurrenceAnalyticsRateDto {
                denominator: 2,
                completed: 0,
                percent: 0,
            })
        );
        assert_eq!(
            current.counts.unrecorded, 1,
            "a missing outcome is never presented as completed or failed"
        );
    }

    #[test]
    fn zero_length_and_empty_ranges_are_valid_responses() {
        let conn = test_conn();
        use_utc_workspace(&conn);

        let series = block_recurrence_repo::create(&conn, "w1", rule(&[1], "2000-01-01", &NO_TEXT))
            .expect("create series");
        set_open_interval(&conn, "w1", &series.id, "2020-01-01");

        // 2020-01-07 is a Tuesday: the range holds no eligible date at all.
        let empty = analytics(&conn, "2020-01-07", "2020-01-07");
        assert_eq!(empty.effective_from, "2020-01-07");
        assert_eq!(empty.effective_to, "2020-01-07");
        let empty_entry = entry(&empty, &series.id);
        assert_eq!(empty_entry.counts, counts(0, 0, 0, 0, 0, 0));
        assert_eq!(empty_entry.rate, None);
        assert_eq!(
            empty_entry.weekly.len(),
            1,
            "the week is emitted with zeros"
        );
        assert_eq!(
            empty_entry.weekly[0],
            RecurrenceAnalyticsWeekBucketDto {
                week_start: "2020-01-06".to_string(),
                ..Default::default()
            }
        );
        assert!(empty_entry.display_label_is_current_fallback);

        // A zero-length range that does hold an expected date.
        let single = analytics(&conn, "2020-01-06", "2020-01-06");
        assert_eq!(entry(&single, &series.id).counts, counts(1, 0, 0, 0, 0, 1));
        assert_eq!(
            entry(&single, &series.id).rate,
            Some(RecurrenceAnalyticsRateDto {
                denominator: 1,
                completed: 0,
                percent: 0,
            })
        );
    }

    #[test]
    fn effective_range_validates_rejects_and_clamps_to_an_injected_today() {
        assert_eq!(
            effective_range("2020-01-01", "2099-12-31", "2026-10-22").expect("clamped range"),
            ("2020-01-01".to_string(), "2026-10-22".to_string())
        );
        assert_eq!(
            effective_range("2020-01-01", "2020-02-01", "2026-10-22").expect("past range"),
            ("2020-01-01".to_string(), "2020-02-01".to_string()),
            "an end before today is kept"
        );
        assert_eq!(
            effective_range("2026-10-22", "2026-10-22", "2026-10-22").expect("today"),
            ("2026-10-22".to_string(), "2026-10-22".to_string())
        );
        assert_eq!(
            effective_range("2099-01-01", "2099-12-31", "2026-10-22").expect("future range"),
            ("2099-01-01".to_string(), "2026-10-22".to_string()),
            "the start is never clamped"
        );
        assert!(effective_range("2020-02-01", "2020-01-01", "2026-10-22").is_err());
        assert!(effective_range("not-a-date", "2020-01-01", "2026-10-22").is_err());
        assert!(effective_range("2020-01-01", "2020-13-01", "2026-10-22").is_err());
    }

    #[test]
    fn a_future_end_clamps_to_the_workspace_local_today_and_reads_no_future_row() {
        let conn = test_conn();
        use_utc_workspace(&conn);

        let series = block_recurrence_repo::create(&conn, "w1", rule(&[1], "2000-01-01", &NO_TEXT))
            .expect("create series");
        set_open_interval(&conn, "w1", &series.id, "2020-01-01");
        // A far-future occurrence stays materialized but is outside the
        // effective range, so it is never read.
        materialize(&conn, "w1", "2099-01-05");

        let before = workspace_local_today(&conn, "w1");
        let response = analytics(&conn, "2020-01-01", "2099-12-31");
        let after = workspace_local_today(&conn, "w1");

        assert_eq!(response.effective_from, "2020-01-01");
        assert!(
            response.effective_to == before || response.effective_to == after,
            "the echoed end is the workspace-local today, not the requested end"
        );
        assert!(response.effective_to.as_str() < "2099-12-31");
        let current = entry(&response, &series.id);
        assert!(current.counts.scheduled > 0);
        assert_eq!(current.counts.completed, 0);
        assert_eq!(current.counts.cancelled, 0);
        assert_eq!(
            current.counts.unrecorded, current.counts.scheduled,
            "no in-range occurrence was materialized"
        );
    }

    #[test]
    fn inactive_series_use_their_intervals_and_deleted_series_are_excluded() {
        let conn = test_conn();
        use_utc_workspace(&conn);

        // Inactive with a closed interval: expectations stop at the close.
        let closed = block_recurrence_repo::create(&conn, "w1", rule(&[1], "2000-01-01", &NO_TEXT))
            .expect("create closed series");
        set_closed_interval(&conn, "w1", &closed.id, "2020-01-06", "2020-01-12");
        set_lifecycle(&conn, &closed.id, "inactive");

        // Inactive with an open interval: expectations continue.
        let open = block_recurrence_repo::create(&conn, "w1", rule(&[4], "2000-01-01", &NO_TEXT))
            .expect("create open series");
        set_open_interval(&conn, "w1", &open.id, "2020-01-01");
        set_lifecycle(&conn, &open.id, "inactive");

        // Deleted: fully excluded.
        let deleted =
            block_recurrence_repo::create(&conn, "w1", rule(&[5], "2000-01-01", &NO_TEXT))
                .expect("create deleted series");
        set_open_interval(&conn, "w1", &deleted.id, "2020-01-01");
        block_recurrence_repo::delete_series(&conn, "w1", &deleted.id).expect("delete series");

        let response = analytics(&conn, "2020-01-06", "2020-01-20");
        assert_eq!(
            response.series.len(),
            2,
            "the deleted series is not reported"
        );
        assert!(response
            .series
            .iter()
            .all(|series| series.series_id != deleted.id));

        let closed_entry = entry(&response, &closed.id);
        assert_eq!(
            closed_entry.counts,
            counts(1, 0, 0, 0, 0, 1),
            "only 2020-01-06 is inside the closed interval"
        );
        assert_eq!(closed_entry.coverage.start.as_deref(), Some("2020-01-06"));
        assert_eq!(
            closed_entry.coverage.intervals,
            vec![RecurrenceAnalyticsCoverageIntervalDto {
                from: "2020-01-06".to_string(),
                until: Some("2020-01-12".to_string()),
            }]
        );

        let open_entry = entry(&response, &open.id);
        assert_eq!(
            open_entry.counts,
            counts(2, 0, 0, 0, 0, 2),
            "2020-01-09 and 2020-01-16 are the Thursdays inside the open interval"
        );
        assert_eq!(
            open_entry.coverage.intervals,
            vec![RecurrenceAnalyticsCoverageIntervalDto {
                from: "2020-01-01".to_string(),
                until: None,
            }]
        );
    }

    #[test]
    fn out_of_schedule_cancelled_rows_stay_in_the_cancelled_bucket() {
        let conn = test_conn();
        use_utc_workspace(&conn);

        let series =
            block_recurrence_repo::create(&conn, "w1", rule(&[1, 3], "2000-01-01", &NO_TEXT))
                .expect("create series");
        set_open_interval(&conn, "w1", &series.id, "2020-01-01");
        materialize(&conn, "w1", "2020-01-06");
        materialize(&conn, "w1", "2020-01-08");
        set_status(&conn, &series.id, "2020-01-06", "completed");
        set_status(&conn, &series.id, "2020-01-08", "cancelled");

        // The edit removes Wednesday from 2020-01-07 on, so the withdrawal row
        // on 2020-01-08 is out of schedule while staying recorded history.
        block_recurrence_repo::update(&conn, "w1", &series.id, rule(&[1], "2020-01-07", &NO_TEXT))
            .expect("edit weekdays");

        let response = analytics(&conn, "2020-01-06", "2020-01-08");
        let current = entry(&response, &series.id);
        assert_eq!(
            current.counts,
            counts(1, 1, 0, 0, 1, 0),
            "the out-of-schedule withdrawal is still counted in the cancelled bucket"
        );
        assert_eq!(
            current.rate,
            Some(RecurrenceAnalyticsRateDto {
                denominator: 1,
                completed: 1,
                percent: 100,
            }),
            "the out-of-schedule withdrawal never enters the denominator"
        );
    }

    #[test]
    fn analytics_never_cross_workspaces() {
        let conn = test_conn();
        use_utc_workspace(&conn);
        other_workspace(&conn, "w2");

        let first = block_recurrence_repo::create(&conn, "w1", rule(&[1], "2000-01-01", &NO_TEXT))
            .expect("create first series");
        let second = block_recurrence_repo::create(&conn, "w2", rule(&[1], "2000-01-01", &NO_TEXT))
            .expect("create second series");
        set_open_interval(&conn, "w1", &first.id, "2020-01-01");
        set_open_interval(&conn, "w2", &second.id, "2020-01-01");
        materialize(&conn, "w1", "2020-01-06");
        materialize(&conn, "w2", "2020-01-06");
        set_status(&conn, &first.id, "2020-01-06", "skipped");
        set_status(&conn, &second.id, "2020-01-06", "completed");

        let first_response =
            analytics_for_workspace(&conn, "w1", "2020-01-06", "2020-01-06").expect("w1 analytics");
        assert_eq!(first_response.series.len(), 1);
        assert_eq!(first_response.series[0].series_id, first.id);
        assert_eq!(first_response.series[0].counts, counts(1, 0, 0, 1, 0, 0));

        let second_response =
            analytics_for_workspace(&conn, "w2", "2020-01-06", "2020-01-06").expect("w2 analytics");
        assert_eq!(second_response.series.len(), 1);
        assert_eq!(second_response.series[0].series_id, second.id);
        assert_eq!(second_response.series[0].counts, counts(1, 1, 0, 0, 0, 0));
    }

    #[test]
    fn weekly_buckets_are_contiguous_additive_and_stable() {
        let conn = test_conn();
        use_utc_workspace(&conn);

        let series =
            block_recurrence_repo::create(&conn, "w1", rule(&[1, 4], "2000-01-01", &NO_TEXT))
                .expect("create series");
        set_open_interval(&conn, "w1", &series.id, "2020-01-01");
        for (date, status) in [
            ("2020-01-06", "completed"),
            ("2020-01-09", "skipped"),
            ("2020-01-13", "cancelled"),
            ("2020-01-16", "active"),
            ("2020-01-20", "planned"),
        ] {
            materialize(&conn, "w1", date);
            set_status(&conn, &series.id, date, status);
        }

        let response = analytics(&conn, "2020-01-06", "2020-02-02");
        let current = entry(&response, &series.id);
        assert_eq!(
            current
                .weekly
                .iter()
                .map(|bucket| bucket.week_start.as_str())
                .collect::<Vec<_>>(),
            vec!["2020-01-06", "2020-01-13", "2020-01-20", "2020-01-27"],
        );
        assert_eq!(
            current.weekly,
            vec![
                RecurrenceAnalyticsWeekBucketDto {
                    week_start: "2020-01-06".to_string(),
                    scheduled: 2,
                    completed: 1,
                    skipped: 1,
                    ..Default::default()
                },
                RecurrenceAnalyticsWeekBucketDto {
                    week_start: "2020-01-13".to_string(),
                    scheduled: 2,
                    started: 1,
                    cancelled: 1,
                    unrecorded: 1,
                    ..Default::default()
                },
                RecurrenceAnalyticsWeekBucketDto {
                    week_start: "2020-01-20".to_string(),
                    scheduled: 2,
                    unrecorded: 1,
                    ..Default::default()
                },
                RecurrenceAnalyticsWeekBucketDto {
                    week_start: "2020-01-27".to_string(),
                    scheduled: 2,
                    unrecorded: 2,
                    ..Default::default()
                },
            ]
        );
        assert_eq!(current.counts, counts(8, 1, 1, 1, 1, 4));
        assert_eq!(
            sum_buckets(current),
            current.counts,
            "per-week counts add up to the series totals"
        );

        let repeated = analytics(&conn, "2020-01-06", "2020-02-02");
        assert_eq!(
            serde_json::to_string(&response).expect("serialize first response"),
            serde_json::to_string(&repeated).expect("serialize repeated response"),
            "repeated queries over unchanged data are byte-identical"
        );
    }

    #[test]
    fn expiry_sweep_flip_moves_planned_to_skipped_inside_the_denominator() {
        let conn = test_conn();
        use_utc_workspace(&conn);

        let series = block_recurrence_repo::create(&conn, "w1", rule(&[1], "2000-01-01", &NO_TEXT))
            .expect("create series");
        set_open_interval(&conn, "w1", &series.id, "2020-01-01");
        materialize(&conn, "w1", "2020-01-06");
        materialize(&conn, "w1", "2020-01-13");
        set_status(&conn, &series.id, "2020-01-06", "completed");

        let pending = analytics(&conn, "2020-01-06", "2020-01-20");
        let pending_entry = entry(&pending, &series.id);
        assert_eq!(pending_entry.counts, counts(3, 1, 0, 0, 0, 1));
        assert_eq!(
            pending_entry.rate,
            Some(RecurrenceAnalyticsRateDto {
                denominator: 3,
                completed: 1,
                percent: 33,
            })
        );

        // The elapsed-window expiry sweep persists the same skipped status a
        // manual skip uses; analytics never re-derives expiry.
        set_status(&conn, &series.id, "2020-01-13", "skipped");

        let expired = analytics(&conn, "2020-01-06", "2020-01-20");
        let expired_entry = entry(&expired, &series.id);
        assert_eq!(expired_entry.counts, counts(3, 1, 0, 1, 0, 1));
        assert_eq!(
            expired_entry.rate, pending_entry.rate,
            "the planned-to-skipped flip stays inside the denominator"
        );
        assert_eq!(
            expired_entry.counts.unrecorded,
            pending_entry.counts.unrecorded
        );
    }

    #[test]
    fn a_series_without_lifecycle_intervals_reports_no_expectations() {
        let conn = test_conn();
        use_utc_workspace(&conn);

        let series = block_recurrence_repo::create(&conn, "w1", rule(&[1], "2000-01-01", &NO_TEXT))
            .expect("create series");
        set_open_interval(&conn, "w1", &series.id, "2020-01-01");
        materialize(&conn, "w1", "2020-01-06");
        set_status(&conn, &series.id, "2020-01-06", "completed");
        conn.execute(
            "DELETE FROM block_recurrence_lifecycle_intervals WHERE recurrence_id = ?1",
            [&series.id],
        )
        .expect("remove lifecycle intervals");

        let response = analytics(&conn, "2020-01-06", "2020-01-20");
        let current = entry(&response, &series.id);
        assert_eq!(current.coverage.start, None);
        assert!(current.coverage.intervals.is_empty());
        assert_eq!(current.counts, counts(0, 0, 0, 0, 0, 0));
        assert_eq!(current.rate, None);
        assert!(current.display_label_is_current_fallback);
        assert_eq!(current.weekly.len(), 3);
        assert!(current.weekly.iter().all(|bucket| *bucket
            == RecurrenceAnalyticsWeekBucketDto {
                week_start: bucket.week_start.clone(),
                ..Default::default()
            }));
    }

    #[test]
    fn analytics_writes_nothing_and_creates_no_occurrence_rows() {
        let conn = test_conn();
        use_utc_workspace(&conn);

        let series =
            block_recurrence_repo::create(&conn, "w1", rule(&[1, 5], "2000-01-01", &NO_TEXT))
                .expect("create series");
        set_open_interval(&conn, "w1", &series.id, "2020-01-01");

        let before = table_counts(&conn);
        let response = analytics(&conn, "2020-01-01", "2020-12-31");
        let current = entry(&response, &series.id);
        assert!(
            current.counts.scheduled > 0,
            "the range has scheduled dates but no materialized rows"
        );
        assert_eq!(
            table_counts(&conn),
            before,
            "the analytics query is read-only"
        );
        assert_eq!(
            current.counts.unrecorded, current.counts.scheduled,
            "expected dates without rows are unrecorded, not completed or failed"
        );
    }

    #[test]
    fn week_start_and_version_selection_helpers_are_deterministic() {
        assert_eq!(
            week_start_monday("2020-01-09").expect("thursday week start"),
            "2020-01-06"
        );
        assert_eq!(
            week_start_monday("2020-01-06").expect("monday week start"),
            "2020-01-06"
        );
        assert!(week_start_monday("2020-13-01").is_err());

        // Overlapping open windows: the greatest `effective_from` wins, and a
        // tie is broken by the greater version id.
        let versions = vec![
            VersionWindow {
                id: "version-b".to_string(),
                effective_from: time::parse_date("2020-01-13").unwrap(),
                effective_until: None,
                weekdays: BTreeSet::from([1]),
                label: Some("Later".to_string()),
            },
            VersionWindow {
                id: "version-a".to_string(),
                effective_from: time::parse_date("2020-01-13").unwrap(),
                effective_until: None,
                weekdays: BTreeSet::from([1]),
                label: Some("Tied".to_string()),
            },
        ];
        let selected = version_covering(&versions, time::parse_date("2020-02-03").unwrap())
            .expect("covering version");
        assert_eq!(selected.label.as_deref(), Some("Later"));
        assert_eq!(
            version_covering(&versions, time::parse_date("2019-12-30").unwrap()),
            None
        );
    }
}
