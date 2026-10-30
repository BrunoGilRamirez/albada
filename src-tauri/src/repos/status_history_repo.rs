use rusqlite::Connection;

use crate::domain::models::StatusHistoryEntry;
use crate::domain::types::HistorySource;
use crate::error::AppError;

pub trait StatusHistoryRepo {
    fn record_transition(
        &self,
        conn: &Connection,
        task_id: &str,
        from_status: &Option<String>,
        to_status: &str,
        source: HistorySource,
        reason: &Option<String>,
    ) -> Result<(), AppError>;
    fn list_for_task(
        &self,
        conn: &Connection,
        task_id: &str,
    ) -> Result<Vec<StatusHistoryEntry>, AppError>;
}

fn row_to_history(row: &rusqlite::Row<'_>) -> Result<StatusHistoryEntry, rusqlite::Error> {
    Ok(StatusHistoryEntry {
        id: row.get(0)?,
        task_id: row.get(1)?,
        from_status: row.get(2)?,
        to_status: row.get(3)?,
        reason_markdown: row.get(4)?,
        source: row.get(5)?,
        metadata_json: row.get(6)?,
        occurred_at: row.get(7)?,
    })
}

const SELECT_COLS: &str =
    "id, task_id, from_status, to_status, reason_markdown, source, metadata_json, occurred_at";

pub struct SqliteStatusHistoryRepo;

impl StatusHistoryRepo for SqliteStatusHistoryRepo {
    fn record_transition(
        &self,
        conn: &Connection,
        task_id: &str,
        from_status: &Option<String>,
        to_status: &str,
        source: HistorySource,
        reason: &Option<String>,
    ) -> Result<(), AppError> {
        crate::domain::task_closure::record_history(
            conn,
            task_id,
            from_status.as_deref(),
            to_status,
            source,
            reason,
            None,
        )
    }

    fn list_for_task(
        &self,
        conn: &Connection,
        task_id: &str,
    ) -> Result<Vec<StatusHistoryEntry>, AppError> {
        let mut stmt = conn.prepare(&format!(
            "SELECT {SELECT_COLS} FROM task_status_history WHERE task_id = ?1 ORDER BY occurred_at"
        ))?;
        let rows = stmt.query_map([task_id], row_to_history)?;
        let mut result = Vec::new();
        for row in rows {
            result.push(row?);
        }
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::connection;
    use crate::db::migrations::run_migrations;
    use crate::repos::task_repo::{complete_task, SqliteTaskRepo, TaskRepo};
    use crate::repos::workspace_repo::{SqliteWorkspaceRepo, WorkspaceRepo};

    fn test_conn() -> (Connection, String) {
        let conn = connection::open_in_memory().expect("open in-memory connection");
        run_migrations(&conn).expect("run migrations");
        let ws_id = SqliteWorkspaceRepo
            .create(&conn, "Personal")
            .expect("ws")
            .id;
        (conn, ws_id)
    }

    #[test]
    fn history_records_completion() {
        let (conn, ws_id) = test_conn();
        let repo = SqliteTaskRepo;
        let task = repo
            .create(
                &conn,
                &ws_id,
                &None,
                &None,
                &None,
                "Task",
                &None,
                &None,
                crate::domain::types::Priority::Normal,
                &None,
                &None,
                &None,
            )
            .expect("create");
        crate::domain::task_closure::transition_status(
            &conn,
            &task.id,
            crate::domain::types::TaskStatus::InProgress,
            &None,
            crate::domain::types::HistorySource::User,
        )
        .expect("start task");
        complete_task(&conn, &task.id, &None).expect("complete");

        let history_repo = SqliteStatusHistoryRepo;
        let entries = history_repo
            .list_for_task(&conn, &task.id)
            .expect("list history");
        assert_eq!(entries.len(), 2);
        assert_eq!(entries.last().unwrap().to_status, "completed");
        assert_eq!(entries.last().unwrap().source, "user");
    }

    #[test]
    fn history_preserves_reopen() {
        let (conn, ws_id) = test_conn();
        let repo = SqliteTaskRepo;
        let task = repo
            .create(
                &conn,
                &ws_id,
                &None,
                &None,
                &None,
                "Task",
                &None,
                &None,
                crate::domain::types::Priority::Normal,
                &None,
                &None,
                &None,
            )
            .expect("create");

        let history_repo = SqliteStatusHistoryRepo;
        history_repo
            .record_transition(
                &conn,
                &task.id,
                &Some("inbox".into()),
                "planned",
                HistorySource::User,
                &None,
            )
            .expect("to planned");
        history_repo
            .record_transition(
                &conn,
                &task.id,
                &Some("planned".into()),
                "in_progress",
                HistorySource::User,
                &None,
            )
            .expect("to in_progress");
        history_repo
            .record_transition(
                &conn,
                &task.id,
                &Some("in_progress".into()),
                "completed",
                HistorySource::User,
                &None,
            )
            .expect("to completed");

        let entries = history_repo
            .list_for_task(&conn, &task.id)
            .expect("list history");
        assert_eq!(entries.len(), 3);
    }
}
