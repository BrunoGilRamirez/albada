use rusqlite::{params, Connection, OptionalExtension};

use crate::domain::models::FixedCommitment;
use crate::domain::types::FixedCommitmentType;
use crate::domain::validation::validate_title;
use crate::error::AppError;
use crate::id;
use crate::time;

pub trait FixedCommitmentRepo {
    fn create(
        &self,
        conn: &Connection,
        workspace_id: &str,
        title: &str,
        description_markdown: &Option<String>,
        commitment_type: FixedCommitmentType,
        start_at: &Option<String>,
        end_at: &Option<String>,
        duration_minutes: &Option<i64>,
        recurrence_rule: &Option<String>,
        movable: bool,
        privacy_scope: &str,
    ) -> Result<FixedCommitment, AppError>;
    fn get(&self, conn: &Connection, id: &str) -> Result<FixedCommitment, AppError>;
    fn list(
        &self,
        conn: &Connection,
        workspace_id: &str,
        active_only: bool,
    ) -> Result<Vec<FixedCommitment>, AppError>;
    fn update(
        &self,
        conn: &Connection,
        id: &str,
        title: &str,
        description_markdown: &Option<String>,
        commitment_type: FixedCommitmentType,
        start_at: &Option<String>,
        end_at: &Option<String>,
        duration_minutes: &Option<i64>,
        recurrence_rule: &Option<String>,
        movable: bool,
    ) -> Result<FixedCommitment, AppError>;
    fn set_active(
        &self,
        conn: &Connection,
        id: &str,
        active: bool,
    ) -> Result<FixedCommitment, AppError>;
    fn delete(&self, conn: &Connection, id: &str) -> Result<(), AppError>;
}

const COMMITMENT_COLS: &str = "id, workspace_id, title, description_markdown, type, start_at, end_at, duration_minutes, recurrence_rule, movable, active, privacy_scope, created_at, updated_at";

fn row_to_commitment(row: &rusqlite::Row<'_>) -> Result<FixedCommitment, rusqlite::Error> {
    let commitment_type: String = row.get(4)?;
    Ok(FixedCommitment {
        id: row.get(0)?,
        workspace_id: row.get(1)?,
        title: row.get(2)?,
        description_markdown: row.get(3)?,
        commitment_type: commitment_type.parse().map_err(|_| {
            rusqlite::Error::InvalidColumnType(4, "type".to_string(), rusqlite::types::Type::Text)
        })?,
        start_at: row.get(5)?,
        end_at: row.get(6)?,
        duration_minutes: row.get(7)?,
        recurrence_rule: row.get(8)?,
        movable: row.get::<_, i64>(9)? != 0,
        active: row.get::<_, i64>(10)? != 0,
        privacy_scope: row.get(11)?,
        created_at: row.get(12)?,
        updated_at: row.get(13)?,
    })
}

pub struct SqliteFixedCommitmentRepo;

impl FixedCommitmentRepo for SqliteFixedCommitmentRepo {
    #[allow(clippy::too_many_arguments)]
    fn create(
        &self,
        conn: &Connection,
        workspace_id: &str,
        title: &str,
        description_markdown: &Option<String>,
        commitment_type: FixedCommitmentType,
        start_at: &Option<String>,
        end_at: &Option<String>,
        duration_minutes: &Option<i64>,
        recurrence_rule: &Option<String>,
        movable: bool,
        privacy_scope: &str,
    ) -> Result<FixedCommitment, AppError> {
        validate_title(title)?;
        if let (Some(start), Some(end)) = (start_at, end_at) {
            validate_commitment_window(start, end)?;
        }
        let id = id::new_uuid();
        let now = time::now_rfc3339();
        conn.execute(
            "INSERT INTO fixed_commitments (id, workspace_id, title, description_markdown, type, start_at, end_at, duration_minutes, recurrence_rule, movable, active, privacy_scope, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, 1, ?11, ?12, ?12)",
            params![
                id,
                workspace_id,
                title.trim(),
                description_markdown,
                commitment_type.to_string(),
                start_at,
                end_at,
                duration_minutes,
                recurrence_rule,
                i64::from(movable),
                privacy_scope,
                now
            ],
        )?;
        self.get(conn, &id)
    }

    fn get(&self, conn: &Connection, id: &str) -> Result<FixedCommitment, AppError> {
        conn.query_row(
            &format!("SELECT {COMMITMENT_COLS} FROM fixed_commitments WHERE id = ?1"),
            [id],
            row_to_commitment,
        )
        .optional()?
        .ok_or_else(|| AppError::not_found("The fixed commitment was not found."))
    }

    fn list(
        &self,
        conn: &Connection,
        workspace_id: &str,
        active_only: bool,
    ) -> Result<Vec<FixedCommitment>, AppError> {
        let sql = if active_only {
            format!(
                "SELECT {COMMITMENT_COLS} FROM fixed_commitments WHERE workspace_id = ?1 AND active = 1 ORDER BY start_at IS NULL, start_at, created_at"
            )
        } else {
            format!(
                "SELECT {COMMITMENT_COLS} FROM fixed_commitments WHERE workspace_id = ?1 ORDER BY start_at IS NULL, start_at, created_at"
            )
        };
        let mut stmt = conn.prepare(&sql)?;
        let rows = stmt.query_map([workspace_id], row_to_commitment)?;
        let mut result = Vec::new();
        for row in rows {
            result.push(row?);
        }
        Ok(result)
    }

    #[allow(clippy::too_many_arguments)]
    fn update(
        &self,
        conn: &Connection,
        id: &str,
        title: &str,
        description_markdown: &Option<String>,
        commitment_type: FixedCommitmentType,
        start_at: &Option<String>,
        end_at: &Option<String>,
        duration_minutes: &Option<i64>,
        recurrence_rule: &Option<String>,
        movable: bool,
    ) -> Result<FixedCommitment, AppError> {
        validate_title(title)?;
        if let (Some(start), Some(end)) = (start_at, end_at) {
            validate_commitment_window(start, end)?;
        }
        let updated = conn.execute(
            "UPDATE fixed_commitments
             SET title = ?2, description_markdown = ?3, type = ?4, start_at = ?5, end_at = ?6,
                 duration_minutes = ?7, recurrence_rule = ?8, movable = ?9, updated_at = ?10
             WHERE id = ?1",
            params![
                id,
                title.trim(),
                description_markdown,
                commitment_type.to_string(),
                start_at,
                end_at,
                duration_minutes,
                recurrence_rule,
                i64::from(movable),
                time::now_rfc3339()
            ],
        )?;
        if updated == 0 {
            return Err(AppError::not_found("The fixed commitment was not found."));
        }
        self.get(conn, id)
    }

    fn set_active(
        &self,
        conn: &Connection,
        id: &str,
        active: bool,
    ) -> Result<FixedCommitment, AppError> {
        let updated = conn.execute(
            "UPDATE fixed_commitments SET active = ?2, updated_at = ?3 WHERE id = ?1",
            params![id, i64::from(active), time::now_rfc3339()],
        )?;
        if updated == 0 {
            return Err(AppError::not_found("The fixed commitment was not found."));
        }
        self.get(conn, id)
    }

    fn delete(&self, conn: &Connection, id: &str) -> Result<(), AppError> {
        let deleted = conn.execute("DELETE FROM fixed_commitments WHERE id = ?1", [id])?;
        if deleted == 0 {
            return Err(AppError::not_found("The fixed commitment was not found."));
        }
        Ok(())
    }
}

/// Validate that a commitment's time window is coherent. The stored values may
/// be datetimes or `HH:MM` local times; when both sides are `HH:MM` they are
/// compared directly, otherwise the raw strings are compared lexically as a
/// heuristic for ISO-style values.
fn validate_commitment_window(start: &str, end: &str) -> Result<(), AppError> {
    let start_time = crate::time::parse_time_of_day(start);
    let end_time = crate::time::parse_time_of_day(end);
    match (start_time, end_time) {
        (Ok(start_minutes), Ok(end_minutes)) => {
            if start_minutes >= end_minutes {
                return Err(AppError::validation(
                    "The commitment start must be before its end.",
                ));
            }
        }
        _ => {
            if start >= end {
                return Err(AppError::validation(
                    "The commitment start must be before its end.",
                ));
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::test_conn;

    #[test]
    fn create_and_list_commitments() {
        let conn = test_conn();
        let repo = SqliteFixedCommitmentRepo;
        let item = repo
            .create(
                &conn,
                "w1",
                "Morning workout",
                &None,
                FixedCommitmentType::Exercise,
                &Some("07:00".to_string()),
                &Some("08:00".to_string()),
                &Some(60),
                &None,
                false,
                "private",
            )
            .expect("create");
        assert!(item.active);
        assert!(!item.movable);
        let items = repo.list(&conn, "w1", true).expect("list");
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].id, item.id);
    }

    #[test]
    fn inactive_commitments_are_excluded_when_requested() {
        let conn = test_conn();
        let repo = SqliteFixedCommitmentRepo;
        let item = repo
            .create(
                &conn,
                "w1",
                "Appointment",
                &None,
                FixedCommitmentType::Appointment,
                &None,
                &None,
                &None,
                &None,
                true,
                "private",
            )
            .expect("create");
        repo.set_active(&conn, &item.id, false).expect("deactivate");
        assert!(repo.list(&conn, "w1", true).expect("active").is_empty());
        assert_eq!(repo.list(&conn, "w1", false).expect("all").len(), 1);
    }

    #[test]
    fn reversed_time_window_is_rejected() {
        let conn = test_conn();
        let repo = SqliteFixedCommitmentRepo;
        assert!(repo
            .create(
                &conn,
                "w1",
                "Bad window",
                &None,
                FixedCommitmentType::Other,
                &Some("10:00".to_string()),
                &Some("09:00".to_string()),
                &None,
                &None,
                false,
                "private",
            )
            .is_err());
    }

    #[test]
    fn empty_title_is_rejected() {
        let conn = test_conn();
        let repo = SqliteFixedCommitmentRepo;
        assert!(repo
            .create(
                &conn,
                "w1",
                "  ",
                &None,
                FixedCommitmentType::Other,
                &None,
                &None,
                &None,
                &None,
                false,
                "private",
            )
            .is_err());
    }
}
