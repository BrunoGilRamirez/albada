use rusqlite::{params, Connection};

use crate::domain::types::{ClosureMode, HistorySource, TaskStatus};
use crate::error::AppError;
use crate::time;

/// Fetch the status and children of a task.
fn task_status_and_children(conn: &Connection, task_id: &str) -> Result<(String, i64), AppError> {
    conn.query_row(
        "SELECT status, (SELECT COUNT(*) FROM tasks c WHERE c.parent_task_id = t.id AND c.archived_at IS NULL)
         FROM tasks t WHERE t.id = ?1",
        [task_id],
        |row| Ok((row.get(0)?, row.get(1)?)),
    )
    .map_err(Into::into)
}

/// Record a status-history entry for a task.
pub fn record_history(
    conn: &Connection,
    task_id: &str,
    from_status: Option<&str>,
    to_status: &str,
    source: HistorySource,
    reason: &Option<String>,
    metadata_json: Option<&str>,
) -> Result<(), AppError> {
    let id = crate::id::new_uuid();
    let occurred_at = time::now_rfc3339();
    conn.execute(
        "INSERT INTO task_status_history (id, task_id, from_status, to_status, reason_markdown, source, metadata_json, occurred_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        params![
            id,
            task_id,
            from_status,
            to_status,
            reason,
            source.to_string(),
            metadata_json,
            occurred_at
        ],
    )?;
    Ok(())
}

/// Check whether every eligible child of a task is completed.
/// Eligible children are non-archived tasks with a status other than
/// `cancelled` or `deferred`; those are excluded from the active scope.
fn all_eligible_children_completed(conn: &Connection, task_id: &str) -> Result<bool, AppError> {
    let pending: i64 = conn.query_row(
        "SELECT COUNT(*) FROM tasks
         WHERE parent_task_id = ?1
           AND archived_at IS NULL
           AND status NOT IN ('completed', 'cancelled', 'deferred')",
        [task_id],
        |row| row.get(0),
    )?;
    Ok(pending == 0)
}

/// Manually close a parent task with open children.
///
/// Rules:
/// - Reject when the task has no children.
/// - Reject when `closure_mode` is not a manual mode.
/// - Require non-empty `closure_reason_markdown`.
/// - Set status to `completed`, record history, and do not modify children.
pub fn close_parent_manually(
    conn: &Connection,
    task_id: &str,
    closure_mode: ClosureMode,
    reason: &str,
) -> Result<(), AppError> {
    if !closure_mode.is_manual() {
        return Err(AppError::validation(
            "Manual closure requires a manual closure mode.",
        ));
    }
    let reason = reason.trim();
    if reason.is_empty() {
        return Err(AppError::validation(
            "A closure reason is required when closing a parent with open children.",
        ));
    }

    let (status, children) = task_status_and_children(conn, task_id)?;
    if children == 0 {
        return Err(AppError::validation(
            "Manual closure is only valid for a parent task with children.",
        ));
    }
    if status == "completed" {
        return Err(AppError::validation("This parent is already completed."));
    }

    let now = time::now_rfc3339();
    conn.execute(
        "UPDATE tasks
         SET status = 'completed', completed_at = ?2, closure_mode = ?3,
             closure_reason_markdown = ?4, updated_at = ?2
         WHERE id = ?1",
        params![task_id, now, closure_mode.to_string(), reason],
    )?;

    record_history(
        conn,
        task_id,
        Some(&status),
        "completed",
        HistorySource::User,
        &Some(reason.to_string()),
        Some(&format!(
            r#"{{"closure_mode":"{}"}}"#,
            closure_mode.as_str()
        )),
    )?;

    Ok(())
}

/// Attempt automatic parent completion.
///
/// Called after a child transitions to `completed`. If every eligible child is
/// completed and the parent is not already manually closed, the parent is
/// completed with `closure_mode: automatic` and its history recorded.
/// Returns `true` when the parent was completed by this call.
pub fn try_auto_complete_parent(conn: &Connection, parent_id: &str) -> Result<bool, AppError> {
    let (status, children) = task_status_and_children(conn, parent_id)?;
    if children == 0 {
        return Ok(false);
    }
    if status == "completed" {
        return Ok(false);
    }
    if !all_eligible_children_completed(conn, parent_id)? {
        return Ok(false);
    }

    let now = time::now_rfc3339();
    conn.execute(
        "UPDATE tasks
         SET status = 'completed', completed_at = ?2, closure_mode = 'automatic',
             updated_at = ?2
         WHERE id = ?1",
        params![parent_id, now],
    )?;

    record_history(
        conn,
        parent_id,
        Some(&status),
        "completed",
        HistorySource::System,
        &None,
        Some(r#"{"closure_mode":"automatic"}"#),
    )?;

    Ok(true)
}

/// After a child completes, propagate automatic completion up the ancestry
/// chain while possible.
pub fn propagate_auto_completion(conn: &Connection, child_id: &str) -> Result<(), AppError> {
    let mut current = conn.query_row(
        "SELECT parent_task_id FROM tasks WHERE id = ?1",
        [child_id],
        |row| row.get::<_, Option<String>>(0),
    )?;

    while let Some(parent_id) = current {
        if !try_auto_complete_parent(conn, &parent_id)? {
            break;
        }
        current = conn.query_row(
            "SELECT parent_task_id FROM tasks WHERE id = ?1",
            [&parent_id],
            |row| row.get::<_, Option<String>>(0),
        )?;
    }

    Ok(())
}

/// Reopen an automatically completed parent when it gains open children again.
///
/// Only parents closed with `closure_mode: automatic` are reverted; manually
/// closed parents keep their explicit closure and are never auto-reopened.
/// Returns `true` when the parent was reopened.
pub fn reopen_auto_completed_parent_if_needed(
    conn: &Connection,
    parent_id: &str,
) -> Result<bool, AppError> {
    let (status, closure_mode) = conn.query_row(
        "SELECT status, closure_mode FROM tasks WHERE id = ?1",
        [parent_id],
        |row| Ok((row.get::<_, String>(0)?, row.get::<_, Option<String>>(1)?)),
    )?;

    if status != "completed" || closure_mode.as_deref() != Some("automatic") {
        return Ok(false);
    }

    let pending: i64 = conn.query_row(
        "SELECT COUNT(*) FROM tasks
         WHERE parent_task_id = ?1
           AND archived_at IS NULL
           AND status NOT IN ('completed', 'cancelled', 'deferred')",
        [parent_id],
        |row| row.get(0),
    )?;
    if pending == 0 {
        return Ok(false);
    }

    let now = time::now_rfc3339();
    conn.execute(
        "UPDATE tasks
         SET status = 'in_progress', completed_at = NULL, closure_mode = NULL,
             closure_reason_markdown = NULL, updated_at = ?1
         WHERE id = ?2",
        params![now, parent_id],
    )?;
    record_history(
        conn,
        parent_id,
        Some("completed"),
        "in_progress",
        HistorySource::System,
        &None,
        Some(r#"{"reopen":"new_open_children"}"#),
    )?;

    Ok(true)
}

/// Walk ancestors from a child and reopen any automatically completed parent
/// that now has open children again (e.g. after adding a child or reopening one).
pub fn propagate_auto_reopen(conn: &Connection, child_id: &str) -> Result<(), AppError> {
    let mut current = conn.query_row(
        "SELECT parent_task_id FROM tasks WHERE id = ?1",
        [child_id],
        |row| row.get::<_, Option<String>>(0),
    )?;

    while let Some(parent_id) = current {
        if !reopen_auto_completed_parent_if_needed(conn, &parent_id)? {
            break;
        }
        current = conn.query_row(
            "SELECT parent_task_id FROM tasks WHERE id = ?1",
            [&parent_id],
            |row| row.get::<_, Option<String>>(0),
        )?;
    }

    Ok(())
}

/// Fetch the current status of a task as a string.
pub fn current_status(conn: &Connection, task_id: &str) -> Result<String, AppError> {
    conn.query_row("SELECT status FROM tasks WHERE id = ?1", [task_id], |row| {
        row.get(0)
    })
    .map_err(Into::into)
}

/// Perform a validated status transition, recording history.
pub fn transition_status(
    conn: &Connection,
    task_id: &str,
    to: TaskStatus,
    reason: &Option<String>,
    source: HistorySource,
) -> Result<String, AppError> {
    let from_status = current_status(conn, task_id)?;
    let from: TaskStatus = from_status.parse()?;

    crate::domain::status::validate_transition(from, to)?;

    let now = time::now_rfc3339();
    let to_str = to.to_string();

    match to {
        TaskStatus::Completed => {
            conn.execute(
                "UPDATE tasks SET status = ?2, completed_at = ?3, updated_at = ?3 WHERE id = ?1",
                params![task_id, to_str, now],
            )?;
        }
        TaskStatus::Cancelled => {
            conn.execute(
                "UPDATE tasks SET status = ?2, cancelled_at = ?3, updated_at = ?3 WHERE id = ?1",
                params![task_id, to_str, now],
            )?;
        }
        TaskStatus::Blocked => {
            // The blocking note is stored when provided; a dependency-only block
            // records an empty note.
            let note = reason.clone().unwrap_or_default();
            conn.execute(
                "UPDATE tasks SET status = ?2, blocking_note_markdown = ?3, updated_at = ?4 WHERE id = ?1",
                params![task_id, to_str, note, now],
            )?;
        }
        _ => {
            conn.execute(
                "UPDATE tasks SET status = ?2, updated_at = ?3 WHERE id = ?1",
                params![task_id, to_str, now],
            )?;
        }
    }

    record_history(
        conn,
        task_id,
        Some(&from_status),
        &to_str,
        source,
        reason,
        None,
    )?;

    // When a task moves away from completed back into an open state, its
    // automatically completed ancestors must be reopened too.
    if from == TaskStatus::Completed && to != TaskStatus::Completed {
        propagate_auto_reopen(conn, task_id)?;
    }

    Ok(from_status)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::test_conn;
    use crate::time;

    fn insert_task(conn: &Connection, id: &str, status: &str, parent: Option<&str>) {
        let now = time::now_rfc3339();
        conn.execute(
            "INSERT INTO tasks (id, workspace_id, title, status, parent_task_id, created_at, updated_at)
             VALUES (?1, 'w1', ?2, ?3, ?4, ?5, ?5)",
            params![id, format!("task {id}"), status, parent, now],
        )
        .expect("insert task");
    }

    fn status_of(conn: &Connection, id: &str) -> String {
        conn.query_row("SELECT status FROM tasks WHERE id = ?1", [id], |row| {
            row.get::<_, String>(0)
        })
        .expect("read status")
    }

    fn completed_at_of(conn: &Connection, id: &str) -> Option<String> {
        conn.query_row(
            "SELECT completed_at FROM tasks WHERE id = ?1",
            [id],
            |row| row.get::<_, Option<String>>(0),
        )
        .expect("read completed_at")
    }

    #[test]
    fn manual_closure_requires_reason() {
        let conn = test_conn();
        insert_task(&conn, "p", "planned", None);
        insert_task(&conn, "c", "in_progress", Some("p"));

        let err = close_parent_manually(&conn, "p", ClosureMode::ManualOther, "  ")
            .expect_err("reason is required");
        assert_eq!(err.code, crate::error::ErrorCode::Validation);
    }

    #[test]
    fn manual_closure_rejects_no_children() {
        let conn = test_conn();
        insert_task(&conn, "leaf", "planned", None);
        let err = close_parent_manually(&conn, "leaf", ClosureMode::ManualOther, "because")
            .expect_err("leaf has no children");
        assert_eq!(err.code, crate::error::ErrorCode::Validation);
    }

    #[test]
    fn manual_closure_completes_parent_only() {
        let conn = test_conn();
        insert_task(&conn, "p", "in_progress", None);
        insert_task(&conn, "c", "in_progress", Some("p"));

        close_parent_manually(
            &conn,
            "p",
            ClosureMode::ManualAcceptedPartial,
            "Partial result accepted.",
        )
        .expect("close manually");

        assert_eq!(status_of(&conn, "p"), "completed");
        assert_eq!(status_of(&conn, "c"), "in_progress");

        let history: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM task_status_history WHERE task_id = 'p'",
                [],
                |row| row.get(0),
            )
            .expect("count history");
        assert_eq!(history, 1);
    }

    #[test]
    fn automatic_parent_completion_when_all_children_done() {
        let conn = test_conn();
        insert_task(&conn, "p", "in_progress", None);
        insert_task(&conn, "c1", "in_progress", Some("p"));
        insert_task(&conn, "c2", "in_progress", Some("p"));

        let done = try_auto_complete_parent(&conn, "p").expect("check parent");
        assert!(!done, "children not complete yet");

        transition_status(
            &conn,
            "c1",
            TaskStatus::Completed,
            &None,
            HistorySource::User,
        )
        .expect("complete c1");
        let done = try_auto_complete_parent(&conn, "p").expect("check parent");
        assert!(!done, "c2 still open");

        transition_status(
            &conn,
            "c2",
            TaskStatus::Completed,
            &None,
            HistorySource::User,
        )
        .expect("complete c2");
        let done = try_auto_complete_parent(&conn, "p").expect("check parent");
        assert!(done, "all children complete");
        assert_eq!(status_of(&conn, "p"), "completed");

        let mode: String = conn
            .query_row("SELECT closure_mode FROM tasks WHERE id = 'p'", [], |row| {
                row.get(0)
            })
            .expect("read closure mode");
        assert_eq!(mode, "automatic");
    }

    #[test]
    fn cancelled_child_does_not_block_auto_completion() {
        let conn = test_conn();
        insert_task(&conn, "p", "in_progress", None);
        insert_task(&conn, "c1", "cancelled", Some("p"));
        insert_task(&conn, "c2", "in_progress", Some("p"));

        transition_status(
            &conn,
            "c2",
            TaskStatus::Completed,
            &None,
            HistorySource::User,
        )
        .expect("complete c2");

        let done = try_auto_complete_parent(&conn, "p").expect("check parent");
        assert!(done, "cancelled child excluded from scope");
        assert_eq!(status_of(&conn, "p"), "completed");
    }

    #[test]
    fn adding_open_child_reopens_auto_completed_parent() {
        let conn = test_conn();
        insert_task(&conn, "p", "in_progress", None);
        insert_task(&conn, "c1", "in_progress", Some("p"));

        transition_status(
            &conn,
            "c1",
            TaskStatus::Completed,
            &None,
            HistorySource::User,
        )
        .expect("complete c1");
        assert!(
            try_auto_complete_parent(&conn, "p").expect("auto-complete parent"),
            "parent auto-completes when the only child is done"
        );
        assert_eq!(status_of(&conn, "p"), "completed");

        // A new open child arrives; the auto-completed parent must reopen.
        insert_task(&conn, "c2", "in_progress", Some("p"));
        propagate_auto_reopen(&conn, "c2").expect("reopen ancestors");

        assert_eq!(status_of(&conn, "p"), "in_progress");

        let mode: Option<String> = conn
            .query_row("SELECT closure_mode FROM tasks WHERE id = 'p'", [], |row| {
                row.get(0)
            })
            .expect("read closure mode");
        assert_eq!(mode, None, "automatic closure cleared on reopen");
    }

    #[test]
    fn manually_closed_parent_is_not_auto_reopened() {
        let conn = test_conn();
        insert_task(&conn, "p", "in_progress", None);
        insert_task(&conn, "c1", "in_progress", Some("p"));

        close_parent_manually(
            &conn,
            "p",
            ClosureMode::ManualAcceptedPartial,
            "Good enough.",
        )
        .expect("manual closure");
        assert_eq!(status_of(&conn, "p"), "completed");

        // New open child must NOT reopen a manually closed parent.
        insert_task(&conn, "c2", "in_progress", Some("p"));
        propagate_auto_reopen(&conn, "c2").expect("attempt reopen");
        assert_eq!(status_of(&conn, "p"), "completed");
    }

    #[test]
    fn completed_to_cancelled_is_rejected() {
        let conn = test_conn();
        insert_task(&conn, "t", "completed", None);
        let err = transition_status(
            &conn,
            "t",
            TaskStatus::Cancelled,
            &None,
            HistorySource::User,
        )
        .expect_err("completed -> cancelled is invalid");
        assert_eq!(err.code, crate::error::ErrorCode::Validation);
    }

    #[test]
    fn inbox_to_completed_is_allowed() {
        let conn = test_conn();
        insert_task(&conn, "t", "inbox", None);
        transition_status(
            &conn,
            "t",
            TaskStatus::Completed,
            &None,
            HistorySource::User,
        )
        .expect("inbox -> completed is allowed");
        assert_eq!(status_of(&conn, "t"), "completed");
        assert!(completed_at_of(&conn, "t").is_some());
    }
}
