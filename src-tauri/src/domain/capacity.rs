use rusqlite::Connection;
use serde::Serialize;

use crate::domain::models::FixedCommitment;
use crate::error::AppError;
use crate::repos::availability_profile_repo::{
    AvailabilityProfileRepo, SqliteAvailabilityProfileRepo,
};
use crate::repos::fixed_commitment_repo::{FixedCommitmentRepo, SqliteFixedCommitmentRepo};
use crate::time;

/// A suggested focus window derived from availability profiles and fixed
/// commitments. This is a suggestion only; the user can ignore it entirely.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CapacitySuggestion {
    /// Suggested start time as `HH:MM`.
    pub start_time: String,
    /// Suggested end time as `HH:MM`.
    pub end_time: String,
    pub preferred_block_length_minutes: Option<i64>,
    pub energy_expectation: Option<String>,
    pub profile_id: Option<String>,
}

/// Suggest a focus block window for a date.
///
/// The suggestion picks the first availability profile for the date's weekday
/// whose window does not overlap an active fixed commitment on that date. If no
/// profile fits, or there are no profiles, it returns `None`.
pub fn suggest_focus_window(
    conn: &Connection,
    workspace_id: &str,
    date: &str,
) -> Result<Option<CapacitySuggestion>, AppError> {
    let weekday = time::weekday_index(date)? as i64;

    let mut profiles =
        SqliteAvailabilityProfileRepo.list_for_weekday(conn, workspace_id, weekday)?;
    profiles.retain(|profile| {
        profile_active_on(
            profile.active_from.as_deref(),
            profile.active_until.as_deref(),
            date,
        )
    });
    profiles.sort_by(|a, b| a.start_time.cmp(&b.start_time));

    if profiles.is_empty() {
        log::debug!("capacity: no availability profiles for {date} (weekday {weekday})");
        return Ok(None);
    }

    let commitments = SqliteFixedCommitmentRepo.list(conn, workspace_id, true)?;

    for profile in &profiles {
        let start = time::parse_time_of_day(&profile.start_time)?;
        let end = time::parse_time_of_day(&profile.end_time)?;
        if commitments
            .iter()
            .any(|commitment| commitment_conflicts_on(commitment, date, start, end))
        {
            continue;
        }
        log::debug!(
            "capacity: suggested window {} {} on {date}",
            profile.start_time,
            profile.end_time
        );
        return Ok(Some(CapacitySuggestion {
            start_time: profile.start_time.clone(),
            end_time: profile.end_time.clone(),
            preferred_block_length_minutes: profile.preferred_block_length_minutes,
            energy_expectation: profile.energy_expectation.clone(),
            profile_id: Some(profile.id.clone()),
        }));
    }

    log::debug!("capacity: all weekday profiles conflict with commitments on {date}");
    Ok(None)
}

/// Whether a profile is active on a `YYYY-MM-DD` date.
fn profile_active_on(active_from: Option<&str>, active_until: Option<&str>, date: &str) -> bool {
    if let Some(from) = active_from {
        if from > date {
            return false;
        }
    }
    if let Some(until) = active_until {
        if until < date {
            return false;
        }
    }
    true
}

/// Whether a fixed commitment occupies any minute of the profile window on the
/// given date.
///
/// Commitments with `HH:MM` values are treated as applying on the date when
/// they are active; commitments carrying a `YYYY-MM-DD` prefix only conflict on
/// that specific date.
fn commitment_conflicts_on(
    commitment: &FixedCommitment,
    date: &str,
    start_minutes: i64,
    end_minutes: i64,
) -> bool {
    let Some(commitment_start) = commitment_window_minutes(commitment.start_at.as_deref(), date)
    else {
        return false;
    };
    let Some(commitment_end) = commitment_window_minutes(commitment.end_at.as_deref(), date) else {
        return false;
    };
    commitment_start < end_minutes && start_minutes < commitment_end
}

/// Extract a minute-of-day window edge from a stored value, returning `None`
/// when the value does not apply on `date` or cannot be parsed.
fn commitment_window_minutes(value: Option<&str>, date: &str) -> Option<i64> {
    let value = value?;
    // A full datetime value starts with a `YYYY-MM-DD` prefix and only applies
    // when that prefix matches the requested date.
    let looks_like_date = value.len() >= 10 && value.as_bytes().get(4).copied() == Some(b'-');
    if looks_like_date {
        if !value.starts_with(date) {
            return None;
        }
        let time_part = value
            .find('T')
            .or_else(|| value.find(' '))
            .map(|idx| &value[idx + 1..])
            .unwrap_or("");
        let hhmm: String = time_part.chars().take(5).collect();
        return time::parse_time_of_day(&hhmm).ok();
    }
    time::parse_time_of_day(value).ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::repos::availability_profile_repo::AvailabilityProfileRepo;
    use crate::repos::fixed_commitment_repo::FixedCommitmentRepo;
    use crate::test_utils::test_conn;

    #[test]
    fn suggests_first_profile_without_commitment() {
        let conn = test_conn();
        SqliteAvailabilityProfileRepo
            .create(
                &conn, "w1", 0, "09:00", "10:00", &None, &None, &None, &None, &None,
            )
            .expect("profile a");
        SqliteAvailabilityProfileRepo
            .create(
                &conn, "w1", 0, "11:00", "12:00", &None, &None, &None, &None, &None,
            )
            .expect("profile b");

        // 2026-09-06 is a Sunday (weekday 0).
        let suggestion = suggest_focus_window(&conn, "w1", "2026-09-06")
            .expect("suggestion")
            .expect("some");
        assert_eq!(suggestion.start_time, "09:00");
    }

    #[test]
    fn commitment_blocks_conflicting_profile() {
        let conn = test_conn();
        SqliteAvailabilityProfileRepo
            .create(
                &conn, "w1", 0, "09:00", "10:00", &None, &None, &None, &None, &None,
            )
            .expect("profile a");
        SqliteAvailabilityProfileRepo
            .create(
                &conn, "w1", 0, "11:00", "12:00", &None, &None, &None, &None, &None,
            )
            .expect("profile b");
        SqliteFixedCommitmentRepo
            .create(
                &conn,
                "w1",
                "Work call",
                &None,
                crate::domain::types::FixedCommitmentType::Work,
                &Some("09:30".to_string()),
                &Some("10:30".to_string()),
                &None,
                &None,
                false,
                "private",
            )
            .expect("commitment");

        let suggestion = suggest_focus_window(&conn, "w1", "2026-09-06")
            .expect("suggestion")
            .expect("some");
        assert_eq!(suggestion.start_time, "11:00");
    }

    #[test]
    fn all_profiles_blocked_returns_none() {
        let conn = test_conn();
        SqliteAvailabilityProfileRepo
            .create(
                &conn, "w1", 0, "09:00", "10:00", &None, &None, &None, &None, &None,
            )
            .expect("profile");
        SqliteFixedCommitmentRepo
            .create(
                &conn,
                "w1",
                "Whole window",
                &None,
                crate::domain::types::FixedCommitmentType::Work,
                &Some("08:00".to_string()),
                &Some("11:00".to_string()),
                &None,
                &None,
                false,
                "private",
            )
            .expect("commitment");

        assert_eq!(
            suggest_focus_window(&conn, "w1", "2026-09-06").expect("result"),
            None
        );
    }

    #[test]
    fn datestamped_commitment_only_conflicts_on_its_date() {
        let conn = test_conn();
        SqliteAvailabilityProfileRepo
            .create(
                &conn, "w1", 0, "09:00", "10:00", &None, &None, &None, &None, &None,
            )
            .expect("profile");
        SqliteFixedCommitmentRepo
            .create(
                &conn,
                "w1",
                "Dentist",
                &None,
                crate::domain::types::FixedCommitmentType::Appointment,
                &Some("2026-09-06T09:30:00Z".to_string()),
                &Some("2026-09-06T10:30:00Z".to_string()),
                &None,
                &None,
                false,
                "private",
            )
            .expect("commitment");

        // On the appointment day the window is blocked; on the next Sunday it is not.
        assert_eq!(
            suggest_focus_window(&conn, "w1", "2026-09-06").expect("result"),
            None
        );
        assert!(suggest_focus_window(&conn, "w1", "2026-09-13")
            .expect("result")
            .is_some());
    }

    #[test]
    fn active_date_range_filters_profiles() {
        let conn = test_conn();
        SqliteAvailabilityProfileRepo
            .create(
                &conn,
                "w1",
                0,
                "09:00",
                "10:00",
                &None,
                &None,
                &Some("2026-09-01".to_string()),
                &Some("2026-09-30".to_string()),
                &None,
            )
            .expect("in-range profile");
        SqliteAvailabilityProfileRepo
            .create(
                &conn,
                "w1",
                0,
                "11:00",
                "12:00",
                &None,
                &None,
                &Some("2026-10-01".to_string()),
                &Some("2026-10-31".to_string()),
                &None,
            )
            .expect("out-of-range profile");

        let suggestion = suggest_focus_window(&conn, "w1", "2026-09-06")
            .expect("suggestion")
            .expect("some");
        assert_eq!(suggestion.start_time, "09:00");
    }
}
