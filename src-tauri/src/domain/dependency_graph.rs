use rusqlite::{params, Connection};
use serde::Serialize;

use crate::domain::types::TaskStatus;
use crate::error::AppError;

/// An incomplete prerequisite task that blocks another task.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PrerequisiteTask {
    pub dependency_id: String,
    pub task_id: String,
    pub title: String,
    pub status: String,
}

/// Detect a dependency cycle that would be introduced by adding an edge from
/// `blocked_task_id` to `prerequisite_task_id` (blocked depends on prerequisite).
///
/// Traversal follows `prerequisite_task_id` edges from the blocked task to its
/// prerequisites. A cycle exists when `blocked_task_id` is reachable from
/// `prerequisite_task_id`.
pub fn detect_dependency_cycle(
    conn: &Connection,
    blocked_task_id: &str,
    prerequisite_task_id: &str,
) -> Result<(), AppError> {
    if blocked_task_id == prerequisite_task_id {
        return Err(AppError::validation("A task cannot depend on itself."));
    }

    let mut visited = std::collections::HashSet::new();
    let mut stack = vec![prerequisite_task_id.to_string()];

    while let Some(current) = stack.pop() {
        if current == blocked_task_id {
            return Err(AppError::validation(
                "This dependency would create a cycle.",
            ));
        }
        if !visited.insert(current.clone()) {
            continue;
        }
        let prereqs = direct_prerequisites(conn, &current)?;
        for prereq in prereqs {
            stack.push(prereq);
        }
    }

    Ok(())
}

/// Direct prerequisite task ids of a task from active (non-removed) edges.
pub fn direct_prerequisites(conn: &Connection, task_id: &str) -> Result<Vec<String>, AppError> {
    let mut stmt = conn.prepare(
        "SELECT prerequisite_task_id FROM task_dependencies
         WHERE blocked_task_id = ?1 AND removed_at IS NULL",
    )?;
    let rows = stmt.query_map([task_id], |row| row.get::<_, String>(0))?;
    let mut ids = Vec::new();
    for row in rows {
        ids.push(row?);
    }
    Ok(ids)
}

/// Compute the incomplete prerequisites that currently block a task.
///
/// A prerequisite blocks when it is not `completed` and not `cancelled`
/// (per the eligible-scope rules: cancelled tasks do not block).
pub fn incomplete_prerequisites(
    conn: &Connection,
    task_id: &str,
) -> Result<Vec<PrerequisiteTask>, AppError> {
    let mut stmt = conn.prepare(
        "SELECT d.id, t.id, t.title, t.status
         FROM task_dependencies d
         JOIN tasks t ON t.id = d.prerequisite_task_id
         WHERE d.blocked_task_id = ?1
           AND d.removed_at IS NULL
           AND t.status NOT IN ('completed', 'cancelled')
           AND t.archived_at IS NULL
         ORDER BY t.title",
    )?;
    let rows = stmt.query_map([task_id], |row| {
        Ok(PrerequisiteTask {
            dependency_id: row.get(0)?,
            task_id: row.get(1)?,
            title: row.get(2)?,
            status: row.get(3)?,
        })
    })?;
    let mut result = Vec::new();
    for row in rows {
        result.push(row?);
    }
    Ok(result)
}

/// Derive whether a task is blocked by incomplete prerequisites.
/// Returns `Some(prerequisites)` when blocked, `None` otherwise.
pub fn is_blocked(
    conn: &Connection,
    task_id: &str,
) -> Result<Option<Vec<PrerequisiteTask>>, AppError> {
    let blockers = incomplete_prerequisites(conn, task_id)?;
    if blockers.is_empty() {
        Ok(None)
    } else {
        Ok(Some(blockers))
    }
}

/// List every active prerequisite relationship for a task, including already
/// completed or cancelled prerequisites. Used by the dependency editor so the
/// user can review and remove relationships.
pub fn all_active_prerequisites(
    conn: &Connection,
    task_id: &str,
) -> Result<Vec<PrerequisiteTask>, AppError> {
    let mut stmt = conn.prepare(
        "SELECT d.id, t.id, t.title, t.status
         FROM task_dependencies d
         JOIN tasks t ON t.id = d.prerequisite_task_id
         WHERE d.blocked_task_id = ?1
           AND d.removed_at IS NULL
           AND t.archived_at IS NULL
         ORDER BY t.title",
    )?;
    let rows = stmt.query_map([task_id], |row| {
        Ok(PrerequisiteTask {
            dependency_id: row.get(0)?,
            task_id: row.get(1)?,
            title: row.get(2)?,
            status: row.get(3)?,
        })
    })?;
    let mut result = Vec::new();
    for row in rows {
        result.push(row?);
    }
    Ok(result)
}

/// Fetch the status of a task.
pub fn task_status(conn: &Connection, task_id: &str) -> Result<TaskStatus, AppError> {
    let status: String =
        conn.query_row("SELECT status FROM tasks WHERE id = ?1", [task_id], |row| {
            row.get(0)
        })?;
    status.parse()
}

/// Whether a task exists.
pub fn task_exists(conn: &Connection, task_id: &str) -> Result<bool, AppError> {
    let count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM tasks WHERE id = ?1",
        [task_id],
        |row| row.get(0),
    )?;
    Ok(count > 0)
}

/// Insert an active dependency edge. Rejects self-dependency and cycles.
pub fn insert_dependency(
    conn: &Connection,
    id: &str,
    blocked_task_id: &str,
    prerequisite_task_id: &str,
    note: &Option<String>,
) -> Result<(), AppError> {
    if !task_exists(conn, blocked_task_id)? {
        return Err(AppError::not_found("The blocked task was not found."));
    }
    if !task_exists(conn, prerequisite_task_id)? {
        return Err(AppError::not_found("The prerequisite task was not found."));
    }
    detect_dependency_cycle(conn, blocked_task_id, prerequisite_task_id)?;

    let now = crate::time::now_rfc3339();
    conn.execute(
        "INSERT INTO task_dependencies (id, blocked_task_id, prerequisite_task_id, type, note_markdown, created_at)
         VALUES (?1, ?2, ?3, 'blocks', ?4, ?5)
         ON CONFLICT(blocked_task_id, prerequisite_task_id) DO UPDATE SET
            removed_at = NULL",
        params![id, blocked_task_id, prerequisite_task_id, note, now],
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::test_conn;
    use crate::time;

    fn insert_task(conn: &Connection, id: &str, status: &str) {
        let now = time::now_rfc3339();
        conn.execute(
            "INSERT INTO tasks (id, workspace_id, title, status, created_at, updated_at)
             VALUES (?1, 'w1', ?2, ?3, ?4, ?4)",
            params![id, format!("task {id}"), status, now],
        )
        .expect("insert task");
    }

    fn insert_dependency_row(conn: &Connection, blocked: &str, prereq: &str) {
        let now = time::now_rfc3339();
        conn.execute(
            "INSERT INTO task_dependencies (id, blocked_task_id, prerequisite_task_id, created_at)
             VALUES (?1, ?2, ?3, ?4)",
            params![format!("d-{blocked}-{prereq}"), blocked, prereq, now],
        )
        .expect("insert dependency");
    }

    #[test]
    fn self_dependency_is_rejected() {
        let conn = test_conn();
        insert_task(&conn, "a", "planned");
        assert!(detect_dependency_cycle(&conn, "a", "a").is_err());
    }

    #[test]
    fn a_to_b_to_c_to_a_cycle_is_rejected() {
        let conn = test_conn();
        insert_task(&conn, "a", "planned");
        insert_task(&conn, "b", "planned");
        insert_task(&conn, "c", "planned");
        insert_dependency_row(&conn, "a", "b");
        insert_dependency_row(&conn, "b", "c");
        // c -> a would close the cycle.
        assert!(detect_dependency_cycle(&conn, "c", "a").is_err());
    }

    #[test]
    fn valid_chain_is_accepted() {
        let conn = test_conn();
        insert_task(&conn, "a", "planned");
        insert_task(&conn, "b", "planned");
        insert_task(&conn, "c", "planned");
        assert!(detect_dependency_cycle(&conn, "b", "a").is_ok());
        assert!(detect_dependency_cycle(&conn, "c", "b").is_ok());
    }

    #[test]
    fn blocked_derivation_updates_after_completion() {
        let conn = test_conn();
        insert_task(&conn, "a", "in_progress");
        insert_task(&conn, "b", "in_progress");
        insert_dependency_row(&conn, "b", "a");

        let blocked = is_blocked(&conn, "b").expect("derive blocked");
        assert!(blocked.is_some());
        assert_eq!(blocked.unwrap().len(), 1);

        // Complete the prerequisite.
        let now = time::now_rfc3339();
        conn.execute(
            "UPDATE tasks SET status = 'completed', completed_at = ?2, updated_at = ?2 WHERE id = 'a'",
            params![now, now],
        )
        .expect("complete prerequisite");

        let blocked = is_blocked(&conn, "b").expect("derive blocked after completion");
        assert!(blocked.is_none());
    }

    #[test]
    fn cancelled_prerequisite_does_not_block() {
        let conn = test_conn();
        insert_task(&conn, "a", "cancelled");
        insert_task(&conn, "b", "in_progress");
        insert_dependency_row(&conn, "b", "a");
        let blocked = is_blocked(&conn, "b").expect("derive blocked");
        assert!(blocked.is_none());
    }
}
