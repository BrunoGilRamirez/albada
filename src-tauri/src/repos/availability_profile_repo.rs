use rusqlite::{params, Connection, OptionalExtension};

use crate::domain::models::AvailabilityProfile;
use crate::error::AppError;
use crate::id;
use crate::time;

pub trait AvailabilityProfileRepo {
    fn create(
        &self,
        conn: &Connection,
        workspace_id: &str,
        weekday: i64,
        start_time: &str,
        end_time: &str,
        preferred_block_length_minutes: &Option<i64>,
        energy_expectation: &Option<String>,
        active_from: &Option<String>,
        active_until: &Option<String>,
        note_markdown: &Option<String>,
    ) -> Result<AvailabilityProfile, AppError>;
    fn get(&self, conn: &Connection, id: &str) -> Result<AvailabilityProfile, AppError>;
    fn list(
        &self,
        conn: &Connection,
        workspace_id: &str,
    ) -> Result<Vec<AvailabilityProfile>, AppError>;
    fn list_for_weekday(
        &self,
        conn: &Connection,
        workspace_id: &str,
        weekday: i64,
    ) -> Result<Vec<AvailabilityProfile>, AppError>;
    fn update(
        &self,
        conn: &Connection,
        id: &str,
        weekday: i64,
        start_time: &str,
        end_time: &str,
        preferred_block_length_minutes: &Option<i64>,
        energy_expectation: &Option<String>,
        active_from: &Option<String>,
        active_until: &Option<String>,
        note_markdown: &Option<String>,
    ) -> Result<AvailabilityProfile, AppError>;
    fn delete(&self, conn: &Connection, id: &str) -> Result<(), AppError>;
}

const PROFILE_COLS: &str = "id, workspace_id, weekday, start_time, end_time, preferred_block_length_minutes, energy_expectation, active_from, active_until, note_markdown, created_at, updated_at";

fn row_to_profile(row: &rusqlite::Row<'_>) -> Result<AvailabilityProfile, rusqlite::Error> {
    Ok(AvailabilityProfile {
        id: row.get(0)?,
        workspace_id: row.get(1)?,
        weekday: row.get(2)?,
        start_time: row.get(3)?,
        end_time: row.get(4)?,
        preferred_block_length_minutes: row.get(5)?,
        energy_expectation: row.get(6)?,
        active_from: row.get(7)?,
        active_until: row.get(8)?,
        note_markdown: row.get(9)?,
        created_at: row.get(10)?,
        updated_at: row.get(11)?,
    })
}

pub struct SqliteAvailabilityProfileRepo;

impl AvailabilityProfileRepo for SqliteAvailabilityProfileRepo {
    fn create(
        &self,
        conn: &Connection,
        workspace_id: &str,
        weekday: i64,
        start_time: &str,
        end_time: &str,
        preferred_block_length_minutes: &Option<i64>,
        energy_expectation: &Option<String>,
        active_from: &Option<String>,
        active_until: &Option<String>,
        note_markdown: &Option<String>,
    ) -> Result<AvailabilityProfile, AppError> {
        validate_profile(
            weekday,
            start_time,
            end_time,
            preferred_block_length_minutes,
        )?;
        if let Some(from) = active_from {
            time::parse_date(from)?;
        }
        if let Some(until) = active_until {
            time::parse_date(until)?;
        }
        if let (Some(from), Some(until)) = (active_from, active_until) {
            if from > until {
                return Err(AppError::validation(
                    "The availability active-from date must not be after active-until.",
                ));
            }
        }
        let id = id::new_uuid();
        let now = time::now_rfc3339();
        conn.execute(
            "INSERT INTO availability_profiles (id, workspace_id, weekday, start_time, end_time, preferred_block_length_minutes, energy_expectation, active_from, active_until, note_markdown, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?11)",
            params![
                id,
                workspace_id,
                weekday,
                start_time,
                end_time,
                preferred_block_length_minutes,
                energy_expectation,
                active_from,
                active_until,
                note_markdown,
                now
            ],
        )?;
        self.get(conn, &id)
    }

    fn get(&self, conn: &Connection, id: &str) -> Result<AvailabilityProfile, AppError> {
        conn.query_row(
            &format!("SELECT {PROFILE_COLS} FROM availability_profiles WHERE id = ?1"),
            [id],
            row_to_profile,
        )
        .optional()?
        .ok_or_else(|| AppError::not_found("The availability profile was not found."))
    }

    fn list(
        &self,
        conn: &Connection,
        workspace_id: &str,
    ) -> Result<Vec<AvailabilityProfile>, AppError> {
        let mut stmt = conn.prepare(&format!(
            "SELECT {PROFILE_COLS} FROM availability_profiles WHERE workspace_id = ?1 ORDER BY weekday, start_time"
        ))?;
        let rows = stmt.query_map([workspace_id], row_to_profile)?;
        let mut result = Vec::new();
        for row in rows {
            result.push(row?);
        }
        Ok(result)
    }

    fn list_for_weekday(
        &self,
        conn: &Connection,
        workspace_id: &str,
        weekday: i64,
    ) -> Result<Vec<AvailabilityProfile>, AppError> {
        let mut stmt = conn.prepare(&format!(
            "SELECT {PROFILE_COLS} FROM availability_profiles WHERE workspace_id = ?1 AND weekday = ?2 ORDER BY start_time"
        ))?;
        let rows = stmt.query_map(params![workspace_id, weekday], row_to_profile)?;
        let mut result = Vec::new();
        for row in rows {
            result.push(row?);
        }
        Ok(result)
    }

    fn update(
        &self,
        conn: &Connection,
        id: &str,
        weekday: i64,
        start_time: &str,
        end_time: &str,
        preferred_block_length_minutes: &Option<i64>,
        energy_expectation: &Option<String>,
        active_from: &Option<String>,
        active_until: &Option<String>,
        note_markdown: &Option<String>,
    ) -> Result<AvailabilityProfile, AppError> {
        validate_profile(
            weekday,
            start_time,
            end_time,
            preferred_block_length_minutes,
        )?;
        if let Some(from) = active_from {
            time::parse_date(from)?;
        }
        if let Some(until) = active_until {
            time::parse_date(until)?;
        }
        if let (Some(from), Some(until)) = (active_from, active_until) {
            if from > until {
                return Err(AppError::validation(
                    "The availability active-from date must not be after active-until.",
                ));
            }
        }
        let updated = conn.execute(
            "UPDATE availability_profiles
             SET weekday = ?2, start_time = ?3, end_time = ?4, preferred_block_length_minutes = ?5,
                 energy_expectation = ?6, active_from = ?7, active_until = ?8, note_markdown = ?9, updated_at = ?10
             WHERE id = ?1",
            params![
                id,
                weekday,
                start_time,
                end_time,
                preferred_block_length_minutes,
                energy_expectation,
                active_from,
                active_until,
                note_markdown,
                time::now_rfc3339()
            ],
        )?;
        if updated == 0 {
            return Err(AppError::not_found(
                "The availability profile was not found.",
            ));
        }
        self.get(conn, id)
    }

    fn delete(&self, conn: &Connection, id: &str) -> Result<(), AppError> {
        let deleted = conn.execute("DELETE FROM availability_profiles WHERE id = ?1", [id])?;
        if deleted == 0 {
            return Err(AppError::not_found(
                "The availability profile was not found.",
            ));
        }
        Ok(())
    }
}

fn validate_profile(
    weekday: i64,
    start_time: &str,
    end_time: &str,
    preferred_block_length_minutes: &Option<i64>,
) -> Result<(), AppError> {
    if !(0..=6).contains(&weekday) {
        return Err(AppError::validation(
            "The weekday must be between 0 (Sunday) and 6 (Saturday).",
        ));
    }
    crate::domain::validation::validate_time_window(start_time, end_time)?;
    if let Some(minutes) = preferred_block_length_minutes {
        if *minutes < 5 || *minutes > 600 {
            return Err(AppError::validation(
                "The preferred block length must be between 5 and 600 minutes.",
            ));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::test_conn;

    #[test]
    fn create_and_list_profiles() {
        let conn = test_conn();
        let repo = SqliteAvailabilityProfileRepo;
        let profile = repo
            .create(
                &conn,
                "w1",
                1,
                "09:00",
                "12:00",
                &Some(60),
                &Some("high".to_string()),
                &None,
                &None,
                &None,
            )
            .expect("create");
        assert_eq!(profile.weekday, 1);
        assert_eq!(repo.list(&conn, "w1").expect("list").len(), 1);
    }

    #[test]
    fn invalid_weekday_is_rejected() {
        let conn = test_conn();
        let repo = SqliteAvailabilityProfileRepo;
        assert!(repo
            .create(&conn, "w1", 7, "09:00", "10:00", &None, &None, &None, &None, &None)
            .is_err());
    }

    #[test]
    fn reversed_window_is_rejected() {
        let conn = test_conn();
        let repo = SqliteAvailabilityProfileRepo;
        assert!(repo
            .create(&conn, "w1", 1, "12:00", "09:00", &None, &None, &None, &None, &None)
            .is_err());
    }

    #[test]
    fn list_for_weekday_filters() {
        let conn = test_conn();
        let repo = SqliteAvailabilityProfileRepo;
        repo.create(
            &conn, "w1", 0, "09:00", "10:00", &None, &None, &None, &None, &None,
        )
        .expect("sunday");
        repo.create(
            &conn, "w1", 1, "09:00", "10:00", &None, &None, &None, &None, &None,
        )
        .expect("monday");
        let mondays = repo
            .list_for_weekday(&conn, "w1", 1)
            .expect("monday profiles");
        assert_eq!(mondays.len(), 1);
    }
}
