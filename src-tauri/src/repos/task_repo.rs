use std::str::FromStr;

use rusqlite::{params, Connection, OptionalExtension};

use crate::domain::models::Task;
use crate::domain::types::{ClosureMode, EffortSize, Priority, TaskStatus};
use crate::domain::{task_closure, task_tree};
use crate::error::AppError;
use crate::id;
use crate::time;

use super::year_theme_repo::map_constraint_violation;

pub trait TaskRepo {
    fn create(
        &self,
        conn: &Connection,
        workspace_id: &str,
        project_id: &Option<String>,
        goal_id: &Option<String>,
        parent_task_id: &Option<String>,
        title: &str,
        description_markdown: &Option<String>,
        notes_markdown: &Option<String>,
        priority: Priority,
        effort_size: &Option<EffortSize>,
        target_date: &Option<String>,
        scheduled_date: &Option<String>,
    ) -> Result<Task, AppError>;
    fn create_child_task(
        &self,
        conn: &Connection,
        parent_id: &str,
        workspace_id: &str,
        title: &str,
        description_markdown: &Option<String>,
        notes_markdown: &Option<String>,
        priority: Priority,
        effort_size: &Option<EffortSize>,
        target_date: &Option<String>,
        scheduled_date: &Option<String>,
    ) -> Result<Task, AppError>;
    fn get(&self, conn: &Connection, id: &str) -> Result<Task, AppError>;
    fn list(&self, conn: &Connection, workspace_id: &str) -> Result<Vec<Task>, AppError>;
    fn list_for_project(&self, conn: &Connection, project_id: &str) -> Result<Vec<Task>, AppError>;
    fn list_for_goal(&self, conn: &Connection, goal_id: &str) -> Result<Vec<Task>, AppError>;
    fn list_standalone(&self, conn: &Connection, workspace_id: &str)
        -> Result<Vec<Task>, AppError>;
    fn list_archived(&self, conn: &Connection, workspace_id: &str) -> Result<Vec<Task>, AppError>;
    fn update(
        &self,
        conn: &Connection,
        id: &str,
        title: &str,
        description_markdown: &Option<String>,
        notes_markdown: &Option<String>,
        priority: Priority,
        effort_size: &Option<EffortSize>,
        target_date: &Option<String>,
        scheduled_date: &Option<String>,
    ) -> Result<Task, AppError>;
    fn move_task(
        &self,
        conn: &Connection,
        id: &str,
        new_project_id: &Option<String>,
        new_goal_id: &Option<String>,
        new_parent_task_id: &Option<String>,
    ) -> Result<Task, AppError>;
    /// Attach a leaf (childless root) task to a project or goal, updating its
    /// owner context without moving a subtree.
    fn attach_to_scope(
        &self,
        conn: &Connection,
        id: &str,
        new_project_id: &Option<String>,
        new_goal_id: &Option<String>,
    ) -> Result<Task, AppError>;
    fn archive_subtree(&self, conn: &Connection, id: &str) -> Result<(), AppError>;
    fn restore_subtree(&self, conn: &Connection, id: &str) -> Result<(), AppError>;
    fn restore_task_including_subtree(&self, conn: &Connection, id: &str) -> Result<(), AppError>;
    fn delete_subtree(&self, conn: &Connection, id: &str) -> Result<(), AppError>;
    fn archive(&self, conn: &Connection, id: &str) -> Result<Task, AppError>;
}

pub(crate) fn row_to_task(row: &rusqlite::Row<'_>) -> Result<Task, rusqlite::Error> {
    let status: String = row.get(9)?;
    let priority: String = row.get(10)?;
    let effort: Option<String> = row.get(11)?;
    let closure_mode: Option<String> = row.get(17)?;
    Ok(Task {
        id: row.get(0)?,
        workspace_id: row.get(1)?,
        project_id: row.get(2)?,
        goal_id: row.get(3)?,
        parent_task_id: row.get(4)?,
        title: row.get(5)?,
        description_markdown: row.get(6)?,
        notes_markdown: row.get(7)?,
        blocking_note_markdown: row.get(8)?,
        status: status.parse().map_err(|_| {
            rusqlite::Error::InvalidColumnType(9, "status".to_string(), rusqlite::types::Type::Text)
        })?,
        priority: priority.parse().map_err(|_| {
            rusqlite::Error::InvalidColumnType(
                10,
                "priority".to_string(),
                rusqlite::types::Type::Text,
            )
        })?,
        effort_size: effort
            .as_deref()
            .map(EffortSize::from_str)
            .transpose()
            .map_err(|_| {
                rusqlite::Error::InvalidColumnType(
                    11,
                    "effort_size".to_string(),
                    rusqlite::types::Type::Text,
                )
            })?,
        effort_points: row.get(12)?,
        target_date: row.get(13)?,
        scheduled_date: row.get(14)?,
        completed_at: row.get(15)?,
        cancelled_at: row.get(16)?,
        closure_mode: closure_mode
            .as_deref()
            .map(ClosureMode::from_str)
            .transpose()
            .map_err(|_| {
                rusqlite::Error::InvalidColumnType(
                    17,
                    "closure_mode".to_string(),
                    rusqlite::types::Type::Text,
                )
            })?,
        closure_reason_markdown: row.get(18)?,
        created_at: row.get(19)?,
        updated_at: row.get(20)?,
        archived_at: row.get(21)?,
    })
}

pub(crate) const SELECT_COLS: &str = "id, workspace_id, project_id, goal_id, parent_task_id, title, description_markdown, notes_markdown, blocking_note_markdown, status, priority, effort_size, effort_points, target_date, scheduled_date, completed_at, cancelled_at, closure_mode, closure_reason_markdown, created_at, updated_at, archived_at";

pub struct SqliteTaskRepo;

impl TaskRepo for SqliteTaskRepo {
    fn create(
        &self,
        conn: &Connection,
        workspace_id: &str,
        project_id: &Option<String>,
        goal_id: &Option<String>,
        parent_task_id: &Option<String>,
        title: &str,
        description_markdown: &Option<String>,
        notes_markdown: &Option<String>,
        priority: Priority,
        effort_size: &Option<EffortSize>,
        target_date: &Option<String>,
        scheduled_date: &Option<String>,
    ) -> Result<Task, AppError> {
        crate::domain::validation::validate_title(title)?;
        crate::domain::validation::validate_owner_context(project_id, goal_id)?;

        // Parent-context consistency: a child must inherit the parent's context.
        if let Some(parent_id) = parent_task_id {
            let (parent_project, parent_goal) = task_tree::task_context(conn, parent_id)?;
            let child_ctx = (project_id, goal_id);
            crate::domain::validation::validate_context_consistency(
                (&parent_project, &parent_goal),
                child_ctx,
            )?;
        }

        let id = id::new_uuid();
        let now = time::now_rfc3339();
        conn.execute(
            "INSERT INTO tasks (id, workspace_id, project_id, goal_id, parent_task_id, title, description_markdown, notes_markdown, status, priority, effort_size, target_date, scheduled_date, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, 'inbox', ?9, ?10, ?11, ?12, ?13, ?13)",
            params![id, workspace_id, project_id, goal_id, parent_task_id, title.trim(), description_markdown, notes_markdown, priority.to_string(), effort_size.map(|e| e.as_str()), target_date, scheduled_date, now],
        )
        .map_err(map_constraint_violation)?;

        // Adding an open child under an automatically completed parent must
        // reopen that parent (and any auto-completed ancestors).
        if parent_task_id.is_some() {
            crate::domain::task_closure::propagate_auto_reopen(conn, &id)?;
        }

        self.get(conn, &id)
    }

    fn create_child_task(
        &self,
        conn: &Connection,
        parent_id: &str,
        workspace_id: &str,
        title: &str,
        description_markdown: &Option<String>,
        notes_markdown: &Option<String>,
        priority: Priority,
        effort_size: &Option<EffortSize>,
        target_date: &Option<String>,
        scheduled_date: &Option<String>,
    ) -> Result<Task, AppError> {
        if !task_exists(conn, parent_id)? {
            return Err(AppError::not_found("The parent task was not found."));
        }
        let (parent_project, parent_goal) = task_tree::task_context(conn, parent_id)?;
        self.create(
            conn,
            workspace_id,
            &parent_project,
            &parent_goal,
            &Some(parent_id.to_string()),
            title,
            description_markdown,
            notes_markdown,
            priority,
            effort_size,
            target_date,
            scheduled_date,
        )
    }

    fn get(&self, conn: &Connection, id: &str) -> Result<Task, AppError> {
        conn.query_row(
            &format!("SELECT {SELECT_COLS} FROM tasks WHERE id = ?1"),
            [id],
            row_to_task,
        )
        .optional()?
        .ok_or_else(|| AppError::not_found("The task was not found."))
    }

    fn list(&self, conn: &Connection, workspace_id: &str) -> Result<Vec<Task>, AppError> {
        let mut stmt = conn.prepare(&format!(
            "SELECT {SELECT_COLS} FROM tasks WHERE workspace_id = ?1 AND archived_at IS NULL ORDER BY created_at"
        ))?;
        let rows = stmt.query_map([workspace_id], row_to_task)?;
        let mut result = Vec::new();
        for row in rows {
            result.push(row?);
        }
        Ok(result)
    }

    fn list_for_project(&self, conn: &Connection, project_id: &str) -> Result<Vec<Task>, AppError> {
        let mut stmt = conn.prepare(&format!(
            "SELECT {SELECT_COLS} FROM tasks WHERE project_id = ?1 AND archived_at IS NULL ORDER BY created_at"
        ))?;
        let rows = stmt.query_map([project_id], row_to_task)?;
        let mut result = Vec::new();
        for row in rows {
            result.push(row?);
        }
        Ok(result)
    }

    fn list_for_goal(&self, conn: &Connection, goal_id: &str) -> Result<Vec<Task>, AppError> {
        let mut stmt = conn.prepare(&format!(
            "SELECT {SELECT_COLS} FROM tasks WHERE goal_id = ?1 AND project_id IS NULL AND archived_at IS NULL ORDER BY created_at"
        ))?;
        let rows = stmt.query_map([goal_id], row_to_task)?;
        let mut result = Vec::new();
        for row in rows {
            result.push(row?);
        }
        Ok(result)
    }

    fn list_standalone(
        &self,
        conn: &Connection,
        workspace_id: &str,
    ) -> Result<Vec<Task>, AppError> {
        let mut stmt = conn.prepare(&format!(
            "SELECT {SELECT_COLS} FROM tasks
             WHERE workspace_id = ?1 AND project_id IS NULL
               AND archived_at IS NULL ORDER BY created_at"
        ))?;
        let rows = stmt.query_map([workspace_id], row_to_task)?;
        let mut result = Vec::new();
        for row in rows {
            result.push(row?);
        }
        Ok(result)
    }

    fn list_archived(&self, conn: &Connection, workspace_id: &str) -> Result<Vec<Task>, AppError> {
        let mut stmt = conn.prepare(&format!(
            "SELECT {SELECT_COLS} FROM tasks
             WHERE workspace_id = ?1 AND archived_at IS NOT NULL
             ORDER BY updated_at DESC"
        ))?;
        let rows = stmt.query_map([workspace_id], row_to_task)?;
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
        title: &str,
        description_markdown: &Option<String>,
        notes_markdown: &Option<String>,
        priority: Priority,
        effort_size: &Option<EffortSize>,
        target_date: &Option<String>,
        scheduled_date: &Option<String>,
    ) -> Result<Task, AppError> {
        crate::domain::validation::validate_title(title)?;
        let updated = conn.execute(
            "UPDATE tasks
             SET title = ?2, description_markdown = ?3, notes_markdown = ?4, priority = ?5,
                 effort_size = ?6, target_date = ?7, scheduled_date = ?8, updated_at = ?9
             WHERE id = ?1",
            params![
                id,
                title.trim(),
                description_markdown,
                notes_markdown,
                priority.to_string(),
                effort_size.map(|e| e.as_str()),
                target_date,
                scheduled_date,
                time::now_rfc3339()
            ],
        )?;
        if updated == 0 {
            return Err(AppError::not_found("The task was not found."));
        }
        self.get(conn, id)
    }

    fn move_task(
        &self,
        conn: &Connection,
        id: &str,
        new_project_id: &Option<String>,
        new_goal_id: &Option<String>,
        new_parent_task_id: &Option<String>,
    ) -> Result<Task, AppError> {
        crate::domain::validation::validate_owner_context(new_project_id, new_goal_id)?;

        if let Some(parent_id) = new_parent_task_id {
            if parent_id == id {
                return Err(AppError::validation("A task cannot be its own parent."));
            }
            task_tree::detect_parent_cycle(conn, id, parent_id).map_err(|cycle| cycle.error())?;
            let (parent_project, parent_goal) = task_tree::task_context(conn, parent_id)?;
            if new_project_id != &parent_project || new_goal_id != &parent_goal {
                return Err(AppError::validation(
                    "A moved task must use its new parent's owner context.",
                ));
            }
        }
        // The requested context is written to the moved task. Its descendants
        // must already have that exact context, including when it is standalone.
        task_tree::validate_subtree_context(conn, id, new_project_id, new_goal_id)?;

        let updated = conn.execute(
            "UPDATE tasks
             SET project_id = ?2, goal_id = ?3, parent_task_id = ?4, updated_at = ?5
             WHERE id = ?1",
            params![
                id,
                new_project_id,
                new_goal_id,
                new_parent_task_id,
                time::now_rfc3339()
            ],
        )?;
        if updated == 0 {
            return Err(AppError::not_found("The task was not found."));
        }
        self.get(conn, id)
    }

    fn attach_to_scope(
        &self,
        conn: &Connection,
        id: &str,
        new_project_id: &Option<String>,
        new_goal_id: &Option<String>,
    ) -> Result<Task, AppError> {
        crate::domain::validation::validate_owner_context(new_project_id, new_goal_id)?;

        let task = self.get(conn, id)?;
        if task.parent_task_id.is_some() {
            return Err(AppError::validation(
                "Only a root task can be attached to a project or goal.",
            ));
        }
        let children: i64 = conn.query_row(
            "SELECT COUNT(*) FROM tasks WHERE parent_task_id = ?1 AND archived_at IS NULL",
            [id],
            |row| row.get(0),
        )?;
        if children > 0 {
            return Err(AppError::validation(
                "Only a task without children can be attached to a project or goal.",
            ));
        }

        if let Some(project_id) = new_project_id {
            let exists: i64 = conn.query_row(
                "SELECT COUNT(*) FROM projects WHERE id = ?1",
                [project_id],
                |row| row.get(0),
            )?;
            if exists == 0 {
                return Err(AppError::not_found("The project was not found."));
            }
        }
        if let Some(goal_id) = new_goal_id {
            let exists: i64 = conn.query_row(
                "SELECT COUNT(*) FROM goals WHERE id = ?1",
                [goal_id],
                |row| row.get(0),
            )?;
            if exists == 0 {
                return Err(AppError::not_found("The goal was not found."));
            }
        }

        let updated = conn.execute(
            "UPDATE tasks SET project_id = ?2, goal_id = ?3, updated_at = ?4 WHERE id = ?1",
            params![id, new_project_id, new_goal_id, time::now_rfc3339()],
        )?;
        if updated == 0 {
            return Err(AppError::not_found("The task was not found."));
        }
        log::info!(
            "task {id}: attached to scope project={:?} goal={:?}",
            new_project_id,
            new_goal_id
        );
        self.get(conn, id)
    }

    fn archive_subtree(&self, conn: &Connection, id: &str) -> Result<(), AppError> {
        // Only completed descendants are archived; incomplete tasks keep their
        // state and remain visible. The parent stays visible too.
        let now = time::now_rfc3339();
        conn.execute(
            "WITH RECURSIVE subtree(id) AS (
                SELECT id FROM tasks WHERE parent_task_id = ?1
                UNION ALL
                SELECT t.id FROM tasks t JOIN subtree s ON t.parent_task_id = s.id
             )
             UPDATE tasks SET archived_at = ?2, updated_at = ?2
             WHERE id IN (SELECT id FROM subtree) AND status = 'completed'",
            params![id, now],
        )?;
        Ok(())
    }

    fn restore_subtree(&self, conn: &Connection, id: &str) -> Result<(), AppError> {
        conn.execute(
            "WITH RECURSIVE subtree(id) AS (
                SELECT id FROM tasks WHERE parent_task_id = ?1
                UNION ALL
                SELECT t.id FROM tasks t JOIN subtree s ON t.parent_task_id = s.id
             )
             UPDATE tasks SET archived_at = NULL, updated_at = ?2 WHERE id IN (SELECT id FROM subtree)",
            params![id, time::now_rfc3339()],
        )?;
        Ok(())
    }

    fn restore_task_including_subtree(&self, conn: &Connection, id: &str) -> Result<(), AppError> {
        conn.execute(
            "WITH RECURSIVE subtree(id) AS (
                SELECT id FROM tasks WHERE id = ?1
                UNION ALL
                SELECT t.id FROM tasks t JOIN subtree s ON t.parent_task_id = s.id
             )
             UPDATE tasks SET archived_at = NULL, updated_at = ?2 WHERE id IN (SELECT id FROM subtree)",
            params![id, time::now_rfc3339()],
        )?;
        Ok(())
    }

    fn delete_subtree(&self, conn: &Connection, id: &str) -> Result<(), AppError> {
        // Deleting a parent cascades to children via the foreign key, and
        // related dependency/history/link rows cascade from the task id.
        conn.execute("DELETE FROM tasks WHERE id = ?1", [id])?;
        Ok(())
    }

    fn archive(&self, conn: &Connection, id: &str) -> Result<Task, AppError> {
        let status: String =
            conn.query_row("SELECT status FROM tasks WHERE id = ?1", [id], |row| {
                row.get(0)
            })?;
        if status != "completed" {
            return Err(AppError::validation(
                "Only completed tasks can be archived. Complete the task first.",
            ));
        }
        let updated = conn.execute(
            "UPDATE tasks SET archived_at = ?2, updated_at = ?2 WHERE id = ?1",
            params![id, time::now_rfc3339()],
        )?;
        if updated == 0 {
            return Err(AppError::not_found("The task was not found."));
        }
        self.get(conn, id)
    }
}

/// Whether a task exists in the database.
pub fn task_exists(conn: &Connection, task_id: &str) -> Result<bool, AppError> {
    let count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM tasks WHERE id = ?1",
        [task_id],
        |row| row.get(0),
    )?;
    Ok(count > 0)
}

/// Fetch a task tree rooted at `root_id` as a flat list ordered by creation.
/// Consumers reconstruct the hierarchy in memory.
pub fn tree_rows(conn: &Connection, root_id: &str) -> Result<Vec<Task>, AppError> {
    let mut stmt = conn.prepare(&format!(
        "WITH RECURSIVE subtree(id) AS (
            SELECT id FROM tasks WHERE id = ?1
            UNION ALL
            SELECT t.id FROM tasks t JOIN subtree s ON t.parent_task_id = s.id
         )
         SELECT {SELECT_COLS} FROM tasks WHERE id IN (SELECT id FROM subtree) ORDER BY created_at"
    ))?;
    let rows = stmt.query_map([root_id], row_to_task)?;
    let mut result = Vec::new();
    for row in rows {
        result.push(row?);
    }
    Ok(result)
}

/// Children of a task, ordered by creation.
pub fn children_of(conn: &Connection, parent_id: &str) -> Result<Vec<Task>, AppError> {
    let mut stmt = conn.prepare(&format!(
        "SELECT {SELECT_COLS} FROM tasks WHERE parent_task_id = ?1 AND archived_at IS NULL ORDER BY created_at"
    ))?;
    let rows = stmt.query_map([parent_id], row_to_task)?;
    let mut result = Vec::new();
    for row in rows {
        result.push(row?);
    }
    Ok(result)
}

/// Complete a task explicitly, recording history and propagating auto
/// completion to ancestors. Uses the provided connection (caller controls tx).
pub fn complete_task(
    conn: &Connection,
    task_id: &str,
    reason: &Option<String>,
) -> Result<Task, AppError> {
    let from = task_closure::transition_status(
        conn,
        task_id,
        TaskStatus::Completed,
        reason,
        crate::domain::types::HistorySource::User,
    )?;
    if let Some(parent_id) = conn.query_row(
        "SELECT parent_task_id FROM tasks WHERE id = ?1",
        [task_id],
        |row| row.get::<_, Option<String>>(0),
    )? {
        let _ = from;
        task_closure::propagate_auto_completion(conn, task_id)?;
        let _ = parent_id;
    }
    let repo = SqliteTaskRepo;
    repo.get(conn, task_id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::connection;
    use crate::db::migrations::run_migrations;
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

    fn create_task(
        conn: &Connection,
        workspace_id: &str,
        project_id: Option<&str>,
        goal_id: Option<&str>,
        parent_task_id: Option<&str>,
        title: &str,
    ) -> Task {
        SqliteTaskRepo
            .create(
                conn,
                workspace_id,
                &project_id.map(str::to_string),
                &goal_id.map(str::to_string),
                &parent_task_id.map(str::to_string),
                title,
                &None,
                &None,
                Priority::Normal,
                &None,
                &None,
                &None,
            )
            .expect("create task")
    }

    fn task_structure(
        repo: &SqliteTaskRepo,
        conn: &Connection,
        ids: &[&String],
    ) -> Vec<(Option<String>, Option<String>, Option<String>)> {
        ids.iter()
            .map(|id| {
                let task = repo.get(conn, id).expect("get task");
                (task.parent_task_id, task.project_id, task.goal_id)
            })
            .collect()
    }

    #[test]
    fn standalone_task_is_valid() {
        let (conn, ws_id) = test_conn();
        let repo = SqliteTaskRepo;
        let task = repo
            .create(
                &conn,
                &ws_id,
                &None,
                &None,
                &None,
                "Go shopping",
                &None,
                &None,
                Priority::Normal,
                &None,
                &None,
                &None,
            )
            .expect("create");
        assert_eq!(task.project_id, None);
        assert_eq!(task.goal_id, None);
        assert_eq!(task.status, TaskStatus::Inbox);
    }

    #[test]
    fn self_parent_is_rejected() {
        let (conn, ws_id) = test_conn();
        let repo = SqliteTaskRepo;
        let task = repo
            .create(
                &conn,
                &ws_id,
                &None,
                &None,
                &None,
                "Root",
                &None,
                &None,
                Priority::Normal,
                &None,
                &None,
                &None,
            )
            .expect("create");
        assert!(repo
            .move_task(&conn, &task.id, &None, &None, &Some(task.id.clone()))
            .is_err());
    }

    #[test]
    fn grandchild_cycle_is_rejected() {
        let (conn, ws_id) = test_conn();
        let repo = SqliteTaskRepo;
        for (id, title) in [("proj-a", "A"), ("proj-b", "B")] {
            conn.execute(
                "INSERT INTO projects (id, workspace_id, title, created_at, updated_at) VALUES (?1, ?2, ?3, '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z')",
                rusqlite::params![id, ws_id, title],
            )
            .expect("insert project");
        }
        let a = create_task(&conn, &ws_id, Some("proj-a"), None, None, "A");
        let b = create_task(&conn, &ws_id, Some("proj-a"), None, Some(&a.id), "B");
        let c = create_task(&conn, &ws_id, Some("proj-a"), None, Some(&b.id), "C");
        let before = task_structure(&repo, &conn, &[&a.id, &b.id, &c.id]);

        assert!(repo
            .move_task(
                &conn,
                &a.id,
                &Some("proj-a".to_string()),
                &None,
                &Some(c.id.clone())
            )
            .is_err());
        assert_eq!(
            task_structure(&repo, &conn, &[&a.id, &b.id, &c.id]),
            before,
            "rejected cycle moves must not change parent or owner context"
        );
    }

    #[test]
    fn move_task_rejects_a_subtree_context_mismatch_without_mutation() {
        let (conn, ws_id) = test_conn();
        let repo = SqliteTaskRepo;
        for (id, title) in [("goal-a", "A"), ("goal-b", "B")] {
            conn.execute(
                "INSERT INTO goals (id, workspace_id, title, created_at, updated_at) VALUES (?1, ?2, ?3, '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z')",
                rusqlite::params![id, ws_id, title],
            )
            .expect("insert goal");
        }
        let source = create_task(&conn, &ws_id, None, Some("goal-a"), None, "Source");
        let child = create_task(
            &conn,
            &ws_id,
            None,
            Some("goal-a"),
            Some(&source.id),
            "Child",
        );
        let target = create_task(&conn, &ws_id, None, Some("goal-b"), None, "Target");
        let ids = [&source.id, &child.id, &target.id];
        let before = task_structure(&repo, &conn, &ids);

        assert!(repo
            .move_task(
                &conn,
                &source.id,
                &None,
                &Some("goal-b".to_string()),
                &Some(target.id.clone())
            )
            .is_err());
        assert_eq!(
            task_structure(&repo, &conn, &ids),
            before,
            "rejected context moves must preserve every subtree parent and owner"
        );
    }

    #[test]
    fn move_task_rejects_requested_context_that_differs_from_parent() {
        let (conn, ws_id) = test_conn();
        let repo = SqliteTaskRepo;
        for (id, title) in [("proj-a", "A"), ("proj-b", "B")] {
            conn.execute(
                "INSERT INTO projects (id, workspace_id, title, created_at, updated_at) VALUES (?1, ?2, ?3, '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z')",
                rusqlite::params![id, ws_id, title],
            )
            .expect("insert project");
        }
        let source = create_task(&conn, &ws_id, Some("proj-a"), None, None, "Source");
        let child = create_task(
            &conn,
            &ws_id,
            Some("proj-a"),
            None,
            Some(&source.id),
            "Child",
        );
        let target = create_task(&conn, &ws_id, Some("proj-a"), None, None, "Target");
        let ids = [&source.id, &child.id, &target.id];
        let before = task_structure(&repo, &conn, &ids);

        assert!(repo
            .move_task(
                &conn,
                &source.id,
                &Some("proj-b".to_string()),
                &None,
                &Some(target.id.clone())
            )
            .is_err());
        assert_eq!(
            task_structure(&repo, &conn, &ids),
            before,
            "a rejected destination-context disagreement must not mutate the tree"
        );
    }

    #[test]
    fn move_task_rejects_project_context_for_a_standalone_parent() {
        let (conn, ws_id) = test_conn();
        let repo = SqliteTaskRepo;
        conn.execute(
            "INSERT INTO projects (id, workspace_id, title, created_at, updated_at) VALUES ('proj-a', ?1, 'A', '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z')",
            [&ws_id],
        )
        .expect("insert project");
        let source = create_task(&conn, &ws_id, Some("proj-a"), None, None, "Source");
        let target = create_task(&conn, &ws_id, None, None, None, "Standalone target");
        let ids = [&source.id, &target.id];
        let before = task_structure(&repo, &conn, &ids);

        assert!(repo
            .move_task(
                &conn,
                &source.id,
                &Some("proj-a".to_string()),
                &None,
                &Some(target.id.clone())
            )
            .is_err());
        assert_eq!(
            task_structure(&repo, &conn, &ids),
            before,
            "standalone parents require the exact standalone owner context"
        );
    }

    #[test]
    fn move_task_rejects_scoped_descendants_under_a_standalone_parent() {
        let (conn, ws_id) = test_conn();
        let repo = SqliteTaskRepo;
        conn.execute(
            "INSERT INTO projects (id, workspace_id, title, created_at, updated_at) VALUES ('proj-a', ?1, 'A', '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z')",
            [&ws_id],
        )
        .expect("insert project");
        let source = create_task(&conn, &ws_id, Some("proj-a"), None, None, "Source");
        let child = create_task(
            &conn,
            &ws_id,
            Some("proj-a"),
            None,
            Some(&source.id),
            "Child",
        );
        let target = create_task(&conn, &ws_id, None, None, None, "Standalone target");
        let ids = [&source.id, &child.id, &target.id];
        let before = task_structure(&repo, &conn, &ids);

        assert!(repo
            .move_task(&conn, &source.id, &None, &None, &Some(target.id.clone()))
            .is_err());
        assert_eq!(
            task_structure(&repo, &conn, &ids),
            before,
            "descendants must match a standalone destination exactly"
        );
    }

    #[test]
    fn move_task_moves_a_tree_to_a_valid_parent() {
        let (conn, ws_id) = test_conn();
        let repo = SqliteTaskRepo;
        conn.execute(
            "INSERT INTO projects (id, workspace_id, title, created_at, updated_at) VALUES ('proj-a', ?1, 'A', '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z')",
            [&ws_id],
        )
        .expect("insert project");
        let source = create_task(&conn, &ws_id, Some("proj-a"), None, None, "Source");
        let child = create_task(
            &conn,
            &ws_id,
            Some("proj-a"),
            None,
            Some(&source.id),
            "Child",
        );
        let target = create_task(&conn, &ws_id, Some("proj-a"), None, None, "Target");

        let moved = repo
            .move_task(
                &conn,
                &source.id,
                &Some("proj-a".to_string()),
                &None,
                &Some(target.id.clone()),
            )
            .expect("move tree");

        assert_eq!(moved.parent_task_id.as_deref(), Some(target.id.as_str()));
        assert_eq!(moved.project_id.as_deref(), Some("proj-a"));
        assert_eq!(moved.goal_id, None);
        let moved_child = repo.get(&conn, &child.id).expect("get child");
        assert_eq!(
            moved_child.parent_task_id.as_deref(),
            Some(source.id.as_str())
        );
        assert_eq!(moved_child.project_id.as_deref(), Some("proj-a"));
        assert_eq!(moved_child.goal_id, None);
    }

    #[test]
    fn move_task_moves_a_standalone_tree_to_a_standalone_parent() {
        let (conn, ws_id) = test_conn();
        let repo = SqliteTaskRepo;
        let source = create_task(&conn, &ws_id, None, None, None, "Source");
        let child = create_task(&conn, &ws_id, None, None, Some(&source.id), "Child");
        let target = create_task(&conn, &ws_id, None, None, None, "Target");

        let moved = repo
            .move_task(&conn, &source.id, &None, &None, &Some(target.id.clone()))
            .expect("move standalone tree");

        assert_eq!(moved.parent_task_id.as_deref(), Some(target.id.as_str()));
        assert_eq!(moved.project_id, None);
        assert_eq!(moved.goal_id, None);
        let moved_child = repo.get(&conn, &child.id).expect("get child");
        assert_eq!(
            moved_child.parent_task_id.as_deref(),
            Some(source.id.as_str())
        );
        assert_eq!(moved_child.project_id, None);
        assert_eq!(moved_child.goal_id, None);
    }

    #[test]
    fn context_mismatch_is_rejected() {
        let (conn, ws_id) = test_conn();
        let repo = SqliteTaskRepo;
        let project_a = "proj-a".to_string();
        conn.execute(
            "INSERT INTO projects (id, workspace_id, title, created_at, updated_at) VALUES ('proj-a', ?1, 'A', '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z')",
            [&ws_id],
        ).expect("insert project a");
        let project_b = "proj-b".to_string();
        conn.execute(
            "INSERT INTO projects (id, workspace_id, title, created_at, updated_at) VALUES ('proj-b', ?1, 'B', '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z')",
            [&ws_id],
        ).expect("insert project b");

        let parent = repo
            .create(
                &conn,
                &ws_id,
                &Some(project_a),
                &None,
                &None,
                "Parent",
                &None,
                &None,
                Priority::Normal,
                &None,
                &None,
                &None,
            )
            .expect("parent");
        // Child in project B under parent in project A must be rejected.
        assert!(repo
            .create(
                &conn,
                &ws_id,
                &Some(project_b),
                &None,
                &Some(parent.id.clone()),
                "Child",
                &None,
                &None,
                Priority::Normal,
                &None,
                &None,
                &None
            )
            .is_err());
    }

    #[test]
    fn standalone_parent_rejects_scoped_children_but_accepts_standalone_children() {
        let (conn, ws_id) = test_conn();
        let repo = SqliteTaskRepo;
        conn.execute(
            "INSERT INTO projects (id, workspace_id, title, created_at, updated_at) VALUES ('proj-a', ?1, 'A', '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z')",
            [&ws_id],
        )
        .expect("insert project");
        conn.execute(
            "INSERT INTO goals (id, workspace_id, title, created_at, updated_at) VALUES ('goal-a', ?1, 'A', '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z')",
            [&ws_id],
        )
        .expect("insert goal");
        let parent = create_task(&conn, &ws_id, None, None, None, "Standalone parent");

        for (project_id, goal_id, title) in [
            (Some("proj-a"), None, "Project child"),
            (None, Some("goal-a"), "Goal child"),
        ] {
            assert!(repo
                .create(
                    &conn,
                    &ws_id,
                    &project_id.map(str::to_string),
                    &goal_id.map(str::to_string),
                    &Some(parent.id.clone()),
                    title,
                    &None,
                    &None,
                    Priority::Normal,
                    &None,
                    &None,
                    &None,
                )
                .is_err());
        }
        assert!(children_of(&conn, &parent.id)
            .expect("list children after rejected creates")
            .is_empty());

        let child = create_task(
            &conn,
            &ws_id,
            None,
            None,
            Some(&parent.id),
            "Standalone child",
        );
        assert_eq!(child.parent_task_id.as_deref(), Some(parent.id.as_str()));
        assert_eq!(child.project_id, None);
        assert_eq!(child.goal_id, None);
    }

    #[test]
    fn valid_three_level_tree_is_accepted() {
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
                Priority::Normal,
                &Some(EffortSize::Large),
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
                &Some(a.id.clone()),
                "B",
                &None,
                &None,
                Priority::Normal,
                &Some(EffortSize::Medium),
                &None,
                &None,
            )
            .expect("b");
        repo.create(
            &conn,
            &ws_id,
            &None,
            &None,
            &Some(b.id.clone()),
            "C",
            &None,
            &None,
            Priority::Normal,
            &Some(EffortSize::Trivial),
            &None,
            &None,
        )
        .expect("c");

        let tree = tree_rows(&conn, &a.id).expect("tree");
        assert_eq!(tree.len(), 3);
    }

    #[test]
    fn attach_standalone_task_to_project() {
        let (conn, ws_id) = test_conn();
        conn.execute(
            "INSERT INTO projects (id, workspace_id, title, created_at, updated_at) VALUES ('proj-a', ?1, 'A', '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z')",
            [&ws_id],
        )
        .expect("insert project");
        let repo = SqliteTaskRepo;
        let task = repo
            .create(
                &conn,
                &ws_id,
                &None,
                &None,
                &None,
                "Go shopping",
                &None,
                &None,
                Priority::Normal,
                &None,
                &None,
                &None,
            )
            .expect("create");
        let attached = repo
            .attach_to_scope(&conn, &task.id, &Some("proj-a".to_string()), &None)
            .expect("attach");
        assert_eq!(attached.project_id.as_deref(), Some("proj-a"));
        assert_eq!(attached.goal_id, None);
    }

    #[test]
    fn attach_standalone_task_to_goal() {
        let (conn, ws_id) = test_conn();
        conn.execute(
            "INSERT INTO goals (id, workspace_id, title, created_at, updated_at) VALUES ('goal-a', ?1, 'G', '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z')",
            [&ws_id],
        )
        .expect("insert goal");
        let repo = SqliteTaskRepo;
        let task = repo
            .create(
                &conn,
                &ws_id,
                &None,
                &None,
                &None,
                "Small step",
                &None,
                &None,
                Priority::Normal,
                &None,
                &None,
                &None,
            )
            .expect("create");
        let attached = repo
            .attach_to_scope(&conn, &task.id, &None, &Some("goal-a".to_string()))
            .expect("attach");
        assert_eq!(attached.goal_id.as_deref(), Some("goal-a"));
        assert_eq!(attached.project_id, None);

        let standalone = repo
            .list_standalone(&conn, &ws_id)
            .expect("list standalone");
        assert!(
            standalone.iter().any(|listed| listed.id == task.id),
            "goal-linked tasks without a project remain in the task list"
        );
    }

    #[test]
    fn attach_parent_with_children_is_rejected() {
        let (conn, ws_id) = test_conn();
        conn.execute(
            "INSERT INTO projects (id, workspace_id, title, created_at, updated_at) VALUES ('proj-a', ?1, 'A', '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z')",
            [&ws_id],
        )
        .expect("insert project");
        let repo = SqliteTaskRepo;
        let parent = repo
            .create(
                &conn,
                &ws_id,
                &None,
                &None,
                &None,
                "Parent",
                &None,
                &None,
                Priority::Normal,
                &None,
                &None,
                &None,
            )
            .expect("parent");
        repo.create(
            &conn,
            &ws_id,
            &None,
            &None,
            &Some(parent.id.clone()),
            "Child",
            &None,
            &None,
            Priority::Normal,
            &None,
            &None,
            &None,
        )
        .expect("child");
        assert!(repo
            .attach_to_scope(&conn, &parent.id, &Some("proj-a".to_string()), &None)
            .is_err());
    }

    #[test]
    fn attach_child_task_is_rejected() {
        let (conn, ws_id) = test_conn();
        conn.execute(
            "INSERT INTO projects (id, workspace_id, title, created_at, updated_at) VALUES ('proj-a', ?1, 'A', '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z')",
            [&ws_id],
        )
        .expect("insert project");
        let repo = SqliteTaskRepo;
        let parent = repo
            .create(
                &conn,
                &ws_id,
                &None,
                &None,
                &None,
                "Parent",
                &None,
                &None,
                Priority::Normal,
                &None,
                &None,
                &None,
            )
            .expect("parent");
        let child = repo
            .create(
                &conn,
                &ws_id,
                &None,
                &None,
                &Some(parent.id.clone()),
                "Child",
                &None,
                &None,
                Priority::Normal,
                &None,
                &None,
                &None,
            )
            .expect("child");
        assert!(repo
            .attach_to_scope(&conn, &child.id, &Some("proj-a".to_string()), &None)
            .is_err());
    }

    #[test]
    fn task_lifecycle_completes_and_auto_completes_parent() {
        let (conn, ws_id) = test_conn();
        let repo = SqliteTaskRepo;
        let parent = repo
            .create(
                &conn,
                &ws_id,
                &None,
                &None,
                &None,
                "Parent",
                &None,
                &None,
                Priority::Normal,
                &None,
                &None,
                &None,
            )
            .expect("parent");
        let child1 = repo
            .create(
                &conn,
                &ws_id,
                &None,
                &None,
                &Some(parent.id.clone()),
                "Child 1",
                &None,
                &None,
                Priority::Normal,
                &None,
                &None,
                &None,
            )
            .expect("child1");
        let child2 = repo
            .create(
                &conn,
                &ws_id,
                &None,
                &None,
                &Some(parent.id.clone()),
                "Child 2",
                &None,
                &None,
                Priority::Normal,
                &None,
                &None,
                &None,
            )
            .expect("child2");

        for child in [&child1, &child2] {
            crate::domain::task_closure::transition_status(
                &conn,
                &child.id,
                TaskStatus::InProgress,
                &None,
                crate::domain::types::HistorySource::User,
            )
            .expect("start child");
        }

        complete_task(&conn, &child1.id, &None).expect("complete child1");
        assert_eq!(
            repo.get(&conn, &parent.id).expect("parent").status,
            TaskStatus::Inbox,
            "parent not complete while a child is open"
        );

        complete_task(&conn, &child2.id, &None).expect("complete child2");
        let parent_after = repo.get(&conn, &parent.id).expect("parent after");
        assert_eq!(parent_after.status, TaskStatus::Completed);
        assert_eq!(parent_after.closure_mode, Some(ClosureMode::Automatic));
    }

    #[test]
    fn archive_subtree_archives_descendants() {
        let (conn, ws_id) = test_conn();
        let repo = SqliteTaskRepo;
        let parent = repo
            .create(
                &conn,
                &ws_id,
                &None,
                &None,
                &None,
                "Parent",
                &None,
                &None,
                Priority::Normal,
                &None,
                &None,
                &None,
            )
            .expect("parent");
        let child = repo
            .create(
                &conn,
                &ws_id,
                &None,
                &None,
                &Some(parent.id.clone()),
                "Child",
                &None,
                &None,
                Priority::Normal,
                &None,
                &None,
                &None,
            )
            .expect("child");
        // Only completed tasks can be archived, so complete the child first.
        crate::domain::task_closure::transition_status(
            &conn,
            &child.id,
            TaskStatus::InProgress,
            &None,
            crate::domain::types::HistorySource::User,
        )
        .expect("start child");
        complete_task(&conn, &child.id, &None).expect("complete child");

        repo.archive_subtree(&conn, &parent.id)
            .expect("archive subtree");
        assert!(
            repo.get(&conn, &parent.id)
                .expect("parent")
                .archived_at
                .is_none(),
            "parent stays visible when archiving its subtree"
        );
        assert!(repo
            .get(&conn, &child.id)
            .expect("child")
            .archived_at
            .is_some());
    }

    #[test]
    fn archive_subtree_skips_incomplete_descendants() {
        let (conn, ws_id) = test_conn();
        let repo = SqliteTaskRepo;
        let parent = repo
            .create(
                &conn,
                &ws_id,
                &None,
                &None,
                &None,
                "Parent",
                &None,
                &None,
                Priority::Normal,
                &None,
                &None,
                &None,
            )
            .expect("parent");
        let done = repo
            .create(
                &conn,
                &ws_id,
                &None,
                &None,
                &Some(parent.id.clone()),
                "Done child",
                &None,
                &None,
                Priority::Normal,
                &None,
                &None,
                &None,
            )
            .expect("done child");
        let open = repo
            .create(
                &conn,
                &ws_id,
                &None,
                &None,
                &Some(parent.id.clone()),
                "Open child",
                &None,
                &None,
                Priority::Normal,
                &None,
                &None,
                &None,
            )
            .expect("open child");

        crate::domain::task_closure::transition_status(
            &conn,
            &done.id,
            TaskStatus::InProgress,
            &None,
            crate::domain::types::HistorySource::User,
        )
        .expect("start done child");
        complete_task(&conn, &done.id, &None).expect("complete done child");

        repo.archive_subtree(&conn, &parent.id)
            .expect("archive subtree");

        // The completed child is archived; the incomplete child is not.
        assert!(repo
            .get(&conn, &done.id)
            .expect("done")
            .archived_at
            .is_some());
        assert!(repo
            .get(&conn, &open.id)
            .expect("open")
            .archived_at
            .is_none());
    }
}
