use rusqlite::Connection;

use crate::error::AppError;

/// A task tree error that carries the IDs involved in a rejected operation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CycleDetected(pub Vec<String>);

impl CycleDetected {
    pub fn error(self) -> AppError {
        AppError::validation("The task hierarchy would form a cycle.")
            .with_detail(format!("ids={}", self.0.join(",")))
    }
}

/// Fetch the `parent_task_id` of a task, or `None` if it has no parent.
fn parent_of(conn: &Connection, task_id: &str) -> Result<Option<String>, AppError> {
    let result = conn.query_row(
        "SELECT parent_task_id FROM tasks WHERE id = ?1",
        [task_id],
        |row| row.get::<_, Option<String>>(0),
    );
    match result {
        Ok(parent) => Ok(parent),
        Err(rusqlite::Error::QueryReturnedNoRows) => {
            Err(AppError::not_found("The parent task was not found."))
        }
        Err(err) => Err(err.into()),
    }
}

/// Walk the parent chain starting at `start` collecting the visited task IDs.
/// Returns an error when the chain contains a cycle.
fn walk_parent_chain(conn: &Connection, start: &str) -> Result<Vec<String>, AppError> {
    let mut chain = Vec::new();
    let mut current = Some(start.to_string());
    let mut seen = std::collections::HashSet::new();

    while let Some(id) = current {
        if !seen.insert(id.clone()) {
            return Err(AppError::validation("The task hierarchy contains a cycle."));
        }
        chain.push(id.clone());
        current = parent_of(conn, &id)?;
    }

    Ok(chain)
}

/// Verify that assigning `new_parent_id` as the parent of `task_id` does not
/// create a cycle. Returns the IDs that would participate in the cycle on
/// failure.
pub fn detect_parent_cycle(
    conn: &Connection,
    task_id: &str,
    new_parent_id: &str,
) -> Result<(), CycleDetected> {
    if task_id == new_parent_id {
        return Err(CycleDetected(vec![task_id.to_string()]));
    }

    // Walking up from the prospective parent must not reach `task_id`.
    let chain = walk_parent_chain(conn, new_parent_id)
        .map_err(|_| CycleDetected(vec![task_id.to_string(), new_parent_id.to_string()]))?;

    if chain.contains(&task_id.to_string()) {
        Err(CycleDetected(chain))
    } else {
        Ok(())
    }
}

/// Fetch the `(project_id, goal_id)` context of a task.
pub fn task_context(
    conn: &Connection,
    task_id: &str,
) -> Result<(Option<String>, Option<String>), AppError> {
    conn.query_row(
        "SELECT project_id, goal_id FROM tasks WHERE id = ?1",
        [task_id],
        |row| Ok((row.get(0)?, row.get(1)?)),
    )
    .map_err(Into::into)
}

/// Collect every descendant id of `root` (excluding `root` itself).
pub fn descendant_ids(conn: &Connection, root: &str) -> Result<Vec<String>, AppError> {
    let mut stmt = conn.prepare(
        "WITH RECURSIVE subtree(id) AS (
            SELECT id FROM tasks WHERE parent_task_id = ?1
            UNION ALL
            SELECT t.id FROM tasks t JOIN subtree s ON t.parent_task_id = s.id
        )
        SELECT id FROM subtree",
    )?;
    let rows = stmt.query_map([root], |row| row.get::<_, String>(0))?;
    let mut ids = Vec::new();
    for row in rows {
        ids.push(row?);
    }
    Ok(ids)
}

/// Validate that every descendant of the task being moved already matches its
/// destination context. The moved task itself is excluded because `move_task`
/// writes its requested context as part of the operation.
pub fn validate_subtree_context(
    conn: &Connection,
    moved_task_id: &str,
    destination_project_id: &Option<String>,
    destination_goal_id: &Option<String>,
) -> Result<(), AppError> {
    for id in descendant_ids(conn, moved_task_id)? {
        let (project_id, goal_id) = task_context(conn, &id)?;
        crate::domain::validation::validate_context_consistency(
            (destination_project_id, destination_goal_id),
            (&project_id, &goal_id),
        )?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::test_conn;

    fn insert_task(
        conn: &Connection,
        id: &str,
        parent_id: Option<&str>,
        project_id: Option<&str>,
        goal_id: Option<&str>,
    ) {
        let now = "2026-01-01T00:00:00Z";
        conn.execute(
            "INSERT INTO tasks (id, workspace_id, project_id, goal_id, parent_task_id, title, created_at, updated_at)
             VALUES (?1, 'w1', ?2, ?3, ?4, ?5, ?6, ?6)",
            rusqlite::params![id, project_id, goal_id, parent_id, format!("task {id}"), now],
        )
        .expect("insert task");
    }

    #[test]
    fn self_parent_is_rejected() {
        let conn = test_conn();
        insert_task(&conn, "a", None, None, None);
        assert!(detect_parent_cycle(&conn, "a", "a").is_err());
    }

    #[test]
    fn grandchild_cycle_is_rejected() {
        let conn = test_conn();
        insert_task(&conn, "a", None, None, None);
        insert_task(&conn, "b", Some("a"), None, None);
        insert_task(&conn, "c", Some("b"), None, None);
        assert!(detect_parent_cycle(&conn, "a", "c").is_err());
    }

    #[test]
    fn valid_parent_is_accepted() {
        let conn = test_conn();
        insert_task(&conn, "a", None, None, None);
        insert_task(&conn, "b", None, None, None);
        assert!(detect_parent_cycle(&conn, "b", "a").is_ok());
    }

    #[test]
    fn descendant_ids_collects_subtree() {
        let conn = test_conn();
        insert_task(&conn, "a", None, None, None);
        insert_task(&conn, "b", Some("a"), None, None);
        insert_task(&conn, "c", Some("b"), None, None);
        let ids = descendant_ids(&conn, "a").expect("collect descendants");
        assert_eq!(ids, vec!["b", "c"]);
    }
}
