use rusqlite::{params, Connection, OptionalExtension};

use crate::domain::dependency_graph::{
    incomplete_prerequisites, insert_dependency, PrerequisiteTask,
};
use crate::domain::models::TaskDependency;
use crate::error::AppError;
use crate::id;
use crate::time;

pub trait DependencyRepo {
    fn add_dependency(
        &self,
        conn: &Connection,
        blocked_task_id: &str,
        prerequisite_task_id: &str,
        note: &Option<String>,
    ) -> Result<TaskDependency, AppError>;
    fn remove_dependency(
        &self,
        conn: &Connection,
        id: &str,
        reason: &Option<String>,
    ) -> Result<(), AppError>;
    fn list_for_task(
        &self,
        conn: &Connection,
        task_id: &str,
    ) -> Result<Vec<TaskDependency>, AppError>;
    fn list_dependents(
        &self,
        conn: &Connection,
        task_id: &str,
    ) -> Result<Vec<TaskDependency>, AppError>;
    fn list_blocking_tasks(
        &self,
        conn: &Connection,
        task_id: &str,
    ) -> Result<Vec<PrerequisiteTask>, AppError>;
}

fn row_to_dependency(row: &rusqlite::Row<'_>) -> Result<TaskDependency, rusqlite::Error> {
    Ok(TaskDependency {
        id: row.get(0)?,
        blocked_task_id: row.get(1)?,
        prerequisite_task_id: row.get(2)?,
        dependency_type: row.get(3)?,
        note_markdown: row.get(4)?,
        created_at: row.get(5)?,
        removed_at: row.get(6)?,
    })
}

const SELECT_COLS: &str =
    "id, blocked_task_id, prerequisite_task_id, type, note_markdown, created_at, removed_at";

pub struct SqliteDependencyRepo;

impl DependencyRepo for SqliteDependencyRepo {
    fn add_dependency(
        &self,
        conn: &Connection,
        blocked_task_id: &str,
        prerequisite_task_id: &str,
        note: &Option<String>,
    ) -> Result<TaskDependency, AppError> {
        let dep_id = id::new_uuid();
        insert_dependency(conn, &dep_id, blocked_task_id, prerequisite_task_id, note)?;

        // When the pair already exists (e.g. previously removed), the
        // ON CONFLICT path reuses the existing row id, so fetch by the
        // relationship pair instead of the freshly generated id.
        conn.query_row(
            &format!(
                "SELECT {SELECT_COLS} FROM task_dependencies
                 WHERE blocked_task_id = ?1 AND prerequisite_task_id = ?2"
            ),
            params![blocked_task_id, prerequisite_task_id],
            row_to_dependency,
        )
        .optional()?
        .ok_or_else(|| {
            AppError::unknown().with_detail(format!(
                "dependency lookup failed for blocked_task_id={blocked_task_id}, prerequisite_task_id={prerequisite_task_id}"
            ))
        })
    }

    fn remove_dependency(
        &self,
        conn: &Connection,
        id: &str,
        _reason: &Option<String>,
    ) -> Result<(), AppError> {
        let updated = conn.execute(
            "UPDATE task_dependencies SET removed_at = ?2 WHERE id = ?1 AND removed_at IS NULL",
            params![id, time::now_rfc3339()],
        )?;
        if updated == 0 {
            return Err(AppError::not_found("The dependency was not found."));
        }
        Ok(())
    }

    fn list_for_task(
        &self,
        conn: &Connection,
        task_id: &str,
    ) -> Result<Vec<TaskDependency>, AppError> {
        let mut stmt = conn.prepare(&format!(
            "SELECT {SELECT_COLS} FROM task_dependencies WHERE blocked_task_id = ?1 AND removed_at IS NULL ORDER BY created_at"
        ))?;
        let rows = stmt.query_map([task_id], row_to_dependency)?;
        let mut result = Vec::new();
        for row in rows {
            result.push(row?);
        }
        Ok(result)
    }

    fn list_dependents(
        &self,
        conn: &Connection,
        task_id: &str,
    ) -> Result<Vec<TaskDependency>, AppError> {
        let mut stmt = conn.prepare(&format!(
            "SELECT {SELECT_COLS} FROM task_dependencies WHERE prerequisite_task_id = ?1 AND removed_at IS NULL ORDER BY created_at"
        ))?;
        let rows = stmt.query_map([task_id], row_to_dependency)?;
        let mut result = Vec::new();
        for row in rows {
            result.push(row?);
        }
        Ok(result)
    }

    fn list_blocking_tasks(
        &self,
        conn: &Connection,
        task_id: &str,
    ) -> Result<Vec<PrerequisiteTask>, AppError> {
        incomplete_prerequisites(conn, task_id)
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
    fn add_and_remove_dependency() {
        let (conn, ws_id) = test_conn();
        let repo = SqliteTaskRepo;
        let a = repo
            .create(
                &conn,
                &ws_id,
                &None,
                &None,
                &None,
                "A",
                &None,
                &None,
                crate::domain::types::Priority::Normal,
                &None,
                &None,
                &None,
            )
            .expect("a");
        let b = repo
            .create(
                &conn,
                &ws_id,
                &None,
                &None,
                &None,
                "B",
                &None,
                &None,
                crate::domain::types::Priority::Normal,
                &None,
                &None,
                &None,
            )
            .expect("b");

        let dep_repo = SqliteDependencyRepo;
        let dep = dep_repo
            .add_dependency(&conn, &b.id, &a.id, &None)
            .expect("add");
        assert_eq!(dep.blocked_task_id, b.id);
        assert_eq!(dep.prerequisite_task_id, a.id);

        let blocked = dep_repo
            .list_blocking_tasks(&conn, &b.id)
            .expect("blocking");
        assert_eq!(blocked.len(), 1);

        dep_repo
            .remove_dependency(&conn, &dep.id, &None)
            .expect("remove");
        let blocked = dep_repo
            .list_blocking_tasks(&conn, &b.id)
            .expect("blocking after");
        assert!(blocked.is_empty());
    }

    #[test]
    fn re_add_after_remove_reactivates_same_row() {
        let (conn, ws_id) = test_conn();
        let repo = SqliteTaskRepo;
        let a = repo
            .create(
                &conn,
                &ws_id,
                &None,
                &None,
                &None,
                "A",
                &None,
                &None,
                crate::domain::types::Priority::Normal,
                &None,
                &None,
                &None,
            )
            .expect("a");
        let b = repo
            .create(
                &conn,
                &ws_id,
                &None,
                &None,
                &None,
                "B",
                &None,
                &None,
                crate::domain::types::Priority::Normal,
                &None,
                &None,
                &None,
            )
            .expect("b");

        let dep_repo = SqliteDependencyRepo;

        let first = dep_repo
            .add_dependency(&conn, &b.id, &a.id, &None)
            .expect("first add");
        let first_id = first.id.clone();

        dep_repo
            .remove_dependency(&conn, &first_id, &None)
            .expect("remove");

        // Re-adding the same pair must succeed and reactivate the same row id.
        let second = dep_repo
            .add_dependency(&conn, &b.id, &a.id, &None)
            .expect("re-add after remove");
        assert_eq!(second.id, first_id, "same dependency row reactivated");
        assert_eq!(second.removed_at, None);

        // It must still show as blocking.
        let blocked = dep_repo
            .list_blocking_tasks(&conn, &b.id)
            .expect("blocking");
        assert_eq!(blocked.len(), 1);
        assert_eq!(blocked[0].task_id, a.id);
    }

    #[test]
    fn self_dependency_is_rejected() {
        let (conn, ws_id) = test_conn();
        let repo = SqliteTaskRepo;
        let a = repo
            .create(
                &conn,
                &ws_id,
                &None,
                &None,
                &None,
                "A",
                &None,
                &None,
                crate::domain::types::Priority::Normal,
                &None,
                &None,
                &None,
            )
            .expect("a");
        let dep_repo = SqliteDependencyRepo;
        assert!(dep_repo.add_dependency(&conn, &a.id, &a.id, &None).is_err());
    }

    #[test]
    fn completing_prerequisite_unblocks() {
        let (conn, ws_id) = test_conn();
        let repo = SqliteTaskRepo;
        let a = repo
            .create(
                &conn,
                &ws_id,
                &None,
                &None,
                &None,
                "A",
                &None,
                &None,
                crate::domain::types::Priority::Normal,
                &None,
                &None,
                &None,
            )
            .expect("a");
        let b = repo
            .create(
                &conn,
                &ws_id,
                &None,
                &None,
                &None,
                "B",
                &None,
                &None,
                crate::domain::types::Priority::Normal,
                &None,
                &None,
                &None,
            )
            .expect("b");
        let dep_repo = SqliteDependencyRepo;
        dep_repo
            .add_dependency(&conn, &b.id, &a.id, &None)
            .expect("add");

        assert_eq!(
            dep_repo
                .list_blocking_tasks(&conn, &b.id)
                .expect("blocked")
                .len(),
            1
        );
        crate::domain::task_closure::transition_status(
            &conn,
            &a.id,
            crate::domain::types::TaskStatus::InProgress,
            &None,
            crate::domain::types::HistorySource::User,
        )
        .expect("start a");
        complete_task(&conn, &a.id, &None).expect("complete a");
        assert!(dep_repo
            .list_blocking_tasks(&conn, &b.id)
            .expect("unblocked")
            .is_empty());
    }
}
