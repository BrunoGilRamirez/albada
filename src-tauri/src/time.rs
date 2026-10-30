use chrono::{Datelike, NaiveDate, Timelike, Utc};

use crate::error::AppError;

/// Current UTC time as an RFC 3339 string for storage in SQLite TEXT columns.
pub fn now_rfc3339() -> String {
    Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true)
}

/// The current date as `YYYY-MM-DD` (UTC) for deadline derivation and tests.
pub fn today_utc() -> String {
    Utc::now().format("%Y-%m-%d").to_string()
}

/// Parse a `YYYY-MM-DD` date string.
pub fn parse_date(date: &str) -> Result<NaiveDate, AppError> {
    NaiveDate::parse_from_str(date, "%Y-%m-%d")
        .map_err(|_| AppError::validation(format!("Invalid date '{date}'.")))
}

/// The local calendar day after `date`.
pub fn next_day(date: &str) -> Result<String, AppError> {
    let parsed = parse_date(date)?;
    Ok(parsed
        .checked_add_days(chrono::Days::new(1))
        .ok_or_else(|| {
            AppError::validation(format!("Cannot compute the next day after '{date}'."))
        })?
        .format("%Y-%m-%d")
        .to_string())
}

/// Day-of-week index for a date, using 0 = Sunday through 6 = Saturday to match
/// the `availability_profiles.weekday` CHECK constraint.
pub fn weekday_index(date: &str) -> Result<u32, AppError> {
    Ok(parse_date(date)?.weekday().num_days_from_sunday())
}

/// Parse a `HH:MM` local time string, returning minutes since midnight. The
/// value must be exactly `HH:MM` with a colon separator.
pub fn parse_time_of_day(time: &str) -> Result<i64, AppError> {
    if time.len() != 5 || time.as_bytes().get(2).copied() != Some(b':') {
        return Err(AppError::validation(format!("Invalid time '{time}'.")));
    }
    let parsed = chrono::NaiveTime::parse_from_str(time, "%H:%M")
        .map_err(|_| AppError::validation(format!("Invalid time '{time}'.")))?;
    Ok((parsed.hour() as i64) * 60 + parsed.minute() as i64)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn now_rfc3339_parses() {
        let value = now_rfc3339();
        assert!(
            chrono::DateTime::parse_from_rfc3339(&value).is_ok(),
            "expected valid RFC3339, got {value}"
        );
    }

    #[test]
    fn next_day_advances_into_next_month() {
        assert_eq!(next_day("2026-09-30").expect("next day"), "2026-10-01");
        assert_eq!(next_day("2026-12-31").expect("next day"), "2027-01-01");
    }

    #[test]
    fn weekday_index_uses_sunday_start() {
        // 2026-09-06 is a Sunday.
        assert_eq!(weekday_index("2026-09-06").expect("sunday"), 0);
        // 2026-09-07 is a Monday.
        assert_eq!(weekday_index("2026-09-07").expect("monday"), 1);
        // 2026-09-12 is a Saturday.
        assert_eq!(weekday_index("2026-09-12").expect("saturday"), 6);
    }

    #[test]
    fn time_of_day_parses() {
        assert_eq!(parse_time_of_day("09:30").expect("time"), 570);
        assert!(parse_time_of_day("9:30").is_err());
    }
}
