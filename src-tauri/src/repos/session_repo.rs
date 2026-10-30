use rusqlite::{params, Connection, OptionalExtension};

use crate::domain::models::TaskSession;
use crate::error::AppError;
use crate::id;
use crate::time;

pub trait SessionRepo {
    fn create(
        &self,
        conn: &Connection,
        task_id: &str,
        block_id: &Option<String>,
        daily_plan_task_id: &Option<String>,
    ) -> Result<TaskSession, AppError>;
    fn get(&self, conn: &Connection, id: &str) -> Result<TaskSession, AppError>;
    fn list_for_task(&self, conn: &Connection, task_id: &str)
        -> Result<Vec<TaskSession>, AppError>;
    fn list_for_daily_plan_task(
        &self,
        conn: &Connection,
        daily_plan_task_id: &str,
    ) -> Result<Vec<TaskSession>, AppError>;
    fn list_for_block(
        &self,
        conn: &Connection,
        block_id: &str,
    ) -> Result<Vec<TaskSession>, AppError>;
}

const SESSION_COLS: &str = "id, task_id, block_id, daily_plan_task_id, started_at, ended_at, status, outcome_markdown, next_step_markdown, created_at, updated_at";

fn row_to_session(row: &rusqlite::Row<'_>) -> Result<TaskSession, rusqlite::Error> {
    let status: String = row.get(6)?;
    Ok(TaskSession {
        id: row.get(0)?,
        task_id: row.get(1)?,
        block_id: row.get(2)?,
        daily_plan_task_id: row.get(3)?,
        started_at: row.get(4)?,
        ended_at: row.get(5)?,
        status: status.parse().map_err(|_| {
            rusqlite::Error::InvalidColumnType(6, "status".to_string(), rusqlite::types::Type::Text)
        })?,
        outcome_markdown: row.get(7)?,
        next_step_markdown: row.get(8)?,
        created_at: row.get(9)?,
        updated_at: row.get(10)?,
    })
}

pub struct SqliteSessionRepo;

impl SessionRepo for SqliteSessionRepo {
    fn create(
        &self,
        conn: &Connection,
        task_id: &str,
        block_id: &Option<String>,
        daily_plan_task_id: &Option<String>,
    ) -> Result<TaskSession, AppError> {
        let task_exists: i64 = conn.query_row(
            "SELECT EXISTS (SELECT 1 FROM tasks WHERE id = ?1)",
            [task_id],
            |row| row.get(0),
        )?;
        if task_exists == 0 {
            return Err(AppError::not_found("The task was not found."));
        }
        let id = id::new_uuid();
        let now = time::now_rfc3339();
        conn.execute(
            "INSERT INTO task_sessions (id, task_id, block_id, daily_plan_task_id, status, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, 'planned', ?5, ?5)",
            params![id, task_id, block_id, daily_plan_task_id, now],
        )?;
        self.get(conn, &id)
    }

    fn get(&self, conn: &Connection, id: &str) -> Result<TaskSession, AppError> {
        conn.query_row(
            &format!("SELECT {SESSION_COLS} FROM task_sessions WHERE id = ?1"),
            [id],
            row_to_session,
        )
        .optional()?
        .ok_or_else(|| AppError::not_found("The task session was not found."))
    }

    fn list_for_task(
        &self,
        conn: &Connection,
        task_id: &str,
    ) -> Result<Vec<TaskSession>, AppError> {
        let mut stmt = conn.prepare(&format!(
            "SELECT {SESSION_COLS} FROM task_sessions WHERE task_id = ?1 ORDER BY created_at"
        ))?;
        let rows = stmt.query_map([task_id], row_to_session)?;
        let mut result = Vec::new();
        for row in rows {
            result.push(row?);
        }
        Ok(result)
    }

    fn list_for_daily_plan_task(
        &self,
        conn: &Connection,
        daily_plan_task_id: &str,
    ) -> Result<Vec<TaskSession>, AppError> {
        let mut stmt = conn.prepare(&format!(
            "SELECT {SESSION_COLS} FROM task_sessions WHERE daily_plan_task_id = ?1 ORDER BY created_at"
        ))?;
        let rows = stmt.query_map([daily_plan_task_id], row_to_session)?;
        let mut result = Vec::new();
        for row in rows {
            result.push(row?);
        }
        Ok(result)
    }

    fn list_for_block(
        &self,
        conn: &Connection,
        block_id: &str,
    ) -> Result<Vec<TaskSession>, AppError> {
        let mut stmt = conn.prepare(&format!(
            "SELECT {SESSION_COLS} FROM task_sessions WHERE block_id = ?1 ORDER BY created_at"
        ))?;
        let rows = stmt.query_map([block_id], row_to_session)?;
        let mut result = Vec::new();
        for row in rows {
            result.push(row?);
        }
        Ok(result)
    }
}

fn current_status(conn: &Connection, id: &str) -> Result<String, AppError> {
    conn.query_row(
        "SELECT status FROM task_sessions WHERE id = ?1",
        [id],
        |row| row.get(0),
    )
    .map_err(Into::into)
}

/// Start a session, stamping `started_at`. Allowed from `planned` or `paused`.
pub fn start_session(conn: &Connection, id: &str) -> Result<TaskSession, AppError> {
    let status = current_status(conn, id)?;
    if status != "planned" && status != "paused" {
        return Err(AppError::validation(
            "Only a planned or paused session can be started.",
        ));
    }
    let now = time::now_rfc3339();
    conn.execute(
        "UPDATE task_sessions SET status = 'started', started_at = ?2, updated_at = ?2 WHERE id = ?1",
        params![id, now],
    )?;
    SqliteSessionRepo.get(conn, id)
}

/// Pause a session without ending it.
pub fn pause_session(conn: &Connection, id: &str) -> Result<TaskSession, AppError> {
    let status = current_status(conn, id)?;
    if status != "started" {
        return Err(AppError::validation(
            "Only an active session can be paused.",
        ));
    }
    conn.execute(
        "UPDATE task_sessions SET status = 'paused', updated_at = ?2 WHERE id = ?1",
        params![id, time::now_rfc3339()],
    )?;
    SqliteSessionRepo.get(conn, id)
}

/// Finish a session, stamping `ended_at` and recording the outcome and next
/// step. Finishing a session never completes the task.
pub fn finish_session(
    conn: &Connection,
    id: &str,
    outcome_markdown: &Option<String>,
    next_step_markdown: &Option<String>,
) -> Result<TaskSession, AppError> {
    let status = current_status(conn, id)?;
    if status != "started" && status != "paused" {
        return Err(AppError::validation(
            "Only a started or paused session can be finished.",
        ));
    }
    let now = time::now_rfc3339();
    conn.execute(
        "UPDATE task_sessions
         SET status = 'finished', ended_at = ?2, outcome_markdown = ?3, next_step_markdown = ?4, updated_at = ?2
         WHERE id = ?1",
        params![id, now, outcome_markdown, next_step_markdown],
    )?;
    SqliteSessionRepo.get(conn, id)
}

/// Skip a session. A skipped session is not a failure and does not affect the
/// task.
pub fn skip_session(conn: &Connection, id: &str) -> Result<TaskSession, AppError> {
    let status = current_status(conn, id)?;
    if status == "finished" || status == "skipped" {
        return Err(AppError::validation(
            "The session is already finished or skipped.",
        ));
    }
    conn.execute(
        "UPDATE task_sessions SET status = 'skipped', updated_at = ?2 WHERE id = ?1",
        params![id, time::now_rfc3339()],
    )?;
    SqliteSessionRepo.get(conn, id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::types::SessionStatus;
    use crate::repos::task_repo::{SqliteTaskRepo, TaskRepo};
    use crate::test_utils::test_conn;

    fn create_task(conn: &Connection, title: &str) -> String {
        SqliteTaskRepo
            .create(
                conn,
                "w1",
                &None,
                &None,
                &None,
                title,
                &None,
                &None,
                crate::domain::types::Priority::Normal,
                &None,
                &None,
                &None,
            )
            .expect("create task")
            .id
    }

    #[test]
    fn finishing_session_does_not_complete_task() {
        let conn = test_conn();
        let task_id = create_task(&conn, "Write report");
        let session = SqliteSessionRepo
            .create(&conn, &task_id, &None, &None)
            .expect("create session");
        assert_eq!(session.status, SessionStatus::Planned);

        start_session(&conn, &session.id).expect("start");
        finish_session(
            &conn,
            &session.id,
            &Some("Draft finished.".to_string()),
            &Some("Revise tomorrow.".to_string()),
        )
        .expect("finish");

        let after = SqliteTaskRepo.get(&conn, &task_id).expect("task");
        assert_eq!(
            after.status,
            crate::domain::types::TaskStatus::Inbox,
            "a session never completes a task"
        );
        assert_eq!(
            SqliteSessionRepo
                .get(&conn, &session.id)
                .expect("session")
                .status,
            SessionStatus::Finished
        );
    }

    #[test]
    fn session_lifecycle_allows_pause_and_resume() {
        let conn = test_conn();
        let task_id = create_task(&conn, "Deep work");
        let session = SqliteSessionRepo
            .create(&conn, &task_id, &None, &None)
            .expect("create");
        start_session(&conn, &session.id).expect("start");
        pause_session(&conn, &session.id).expect("pause");
        assert_eq!(
            SqliteSessionRepo
                .get(&conn, &session.id)
                .expect("session")
                .status,
            SessionStatus::Paused
        );
        start_session(&conn, &session.id).expect("resume");
        finish_session(&conn, &session.id, &None, &None).expect("finish");
        assert_eq!(
            SqliteSessionRepo
                .get(&conn, &session.id)
                .expect("session")
                .status,
            SessionStatus::Finished
        );
    }

    #[test]
    fn finishing_planned_session_is_rejected() {
        let conn = test_conn();
        let task_id = create_task(&conn, "Task");
        let session = SqliteSessionRepo
            .create(&conn, &task_id, &None, &None)
            .expect("create");
        assert!(finish_session(&conn, &session.id, &None, &None).is_err());
    }

    #[test]
    fn skipped_session_does_not_affect_task() {
        let conn = test_conn();
        let task_id = create_task(&conn, "Task");
        let session = SqliteSessionRepo
            .create(&conn, &task_id, &None, &None)
            .expect("create");
        skip_session(&conn, &session.id).expect("skip");
        assert_eq!(
            SqliteTaskRepo.get(&conn, &task_id).expect("task").status,
            crate::domain::types::TaskStatus::Inbox
        );
        assert_eq!(
            SqliteSessionRepo
                .get(&conn, &session.id)
                .expect("session")
                .status,
            SessionStatus::Skipped
        );
    }

    #[test]
    fn list_for_task_orders_by_creation() {
        let conn = test_conn();
        let task_id = create_task(&conn, "Task");
        let s1 = SqliteSessionRepo
            .create(&conn, &task_id, &None, &None)
            .expect("s1");
        let s2 = SqliteSessionRepo
            .create(&conn, &task_id, &None, &None)
            .expect("s2");
        let sessions = SqliteSessionRepo
            .list_for_task(&conn, &task_id)
            .expect("list");
        assert_eq!(sessions.len(), 2);
        assert_eq!(sessions[0].id, s1.id);
        assert_eq!(sessions[1].id, s2.id);
    }
}
