use rusqlite::{Connection, OptionalExtension};
use serde::Serialize;

use crate::domain::models::Task;
use crate::domain::types::TaskStatus;
use crate::error::AppError;
use crate::repos::task_repo::{row_to_task, TaskRepo, SELECT_COLS};

/// The scope over which progress is derived.
///
/// Project and goal scopes aggregate their eligible leaves; the task scope is
/// either the task itself (when it has no children) or its descendant leaves.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProgressScope<'a> {
    Project(&'a str),
    Goal(&'a str),
    Task(&'a str),
}

impl ProgressScope<'_> {
    /// Short stable key used in logs: `project:<id>`, `goal:<id>`, `task:<id>`.
    pub fn log_key(&self) -> String {
        match self {
            ProgressScope::Project(id) => format!("project:{id}"),
            ProgressScope::Goal(id) => format!("goal:{id}"),
            ProgressScope::Task(id) => format!("task:{id}"),
        }
    }
}

/// A single eligible leaf task contributing to the numerator and denominator.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ProgressContributor {
    pub task_id: String,
    pub title: String,
    pub status: String,
    pub effort_points: i64,
    pub completed: bool,
}

/// Result of a progress calculation.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ProgressResult {
    /// Percentage when eligible leaves exist, otherwise `None`.
    pub percentage: Option<f64>,
    /// Display label when `percentage` is `None`: `no_tasks`, `not_started`,
    /// `archived`, `cancelled`, or `abandoned`. Empty when a percentage exists.
    pub label: String,
    pub completed_points: i64,
    pub total_points: i64,
    pub contributors: Vec<ProgressContributor>,
}

impl ProgressResult {
    fn empty(label: &str) -> Self {
        Self {
            percentage: None,
            label: label.to_string(),
            completed_points: 0,
            total_points: 0,
            contributors: Vec::new(),
        }
    }
}

const ELIGIBLE_STATUSES: &str = "('planned', 'in_progress', 'blocked', 'completed')";

/// Whether a task is a leaf, i.e. it has no non-archived children.
///
/// Errors when the task does not exist.
pub fn is_leaf_task(conn: &Connection, task_id: &str) -> Result<bool, AppError> {
    let leaf: i64 = conn.query_row(
        "SELECT EXISTS (
             SELECT 1 FROM tasks WHERE id = ?1
         ) AND NOT EXISTS (
             SELECT 1 FROM tasks c WHERE c.parent_task_id = ?1 AND c.archived_at IS NULL
         )",
        [task_id],
        |row| row.get(0),
    )?;
    if leaf == 0 {
        // Distinguish "task not found" from "has children".
        let exists: i64 = conn.query_row(
            "SELECT EXISTS (SELECT 1 FROM tasks WHERE id = ?1)",
            [task_id],
            |row| row.get(0),
        )?;
        if exists == 0 {
            return Err(AppError::not_found("The task was not found."));
        }
    }
    Ok(leaf != 0)
}

/// Collect the eligible leaf tasks for a scope.
///
/// A leaf is a task with no non-archived children. Eligible statuses are
/// `planned`, `in_progress`, `blocked`, and `completed`; cancelled, deferred,
/// and archived tasks are excluded from the active denominator.
pub fn eligible_leaves(
    conn: &Connection,
    scope: &ProgressScope<'_>,
) -> Result<Vec<Task>, AppError> {
    match scope {
        ProgressScope::Project(project_id) => {
            query_rows(conn, &project_leaves_sql(), &[project_id], scope)
        }
        ProgressScope::Goal(goal_id) => query_rows(conn, &goal_leaves_sql(), &[goal_id], scope),
        ProgressScope::Task(task_id) => {
            if is_leaf_task(conn, task_id)? {
                // The task itself is the only leaf when it has no children.
                query_rows(conn, &task_self_leaf_sql(), &[task_id], scope)
            } else {
                query_rows(conn, &task_descendant_leaves_sql(), &[task_id], scope)
            }
        }
    }
}

/// Convenience wrapper over `eligible_leaves` for a goal scope.
pub fn eligible_leaves_for_goal(conn: &Connection, goal_id: &str) -> Result<Vec<Task>, AppError> {
    eligible_leaves(conn, &ProgressScope::Goal(goal_id))
}

/// Map eligible leaf tasks to their contributor representation.
pub fn contributors_from(tasks: &[Task]) -> Vec<ProgressContributor> {
    tasks
        .iter()
        .map(|task| ProgressContributor {
            task_id: task.id.clone(),
            title: task.title.clone(),
            status: task.status.to_string(),
            effort_points: task.effort_points,
            completed: task.status == TaskStatus::Completed,
        })
        .collect()
}

/// Compute weighted progress for a scope.
///
/// Progress is `completed points / total points` across eligible leaves. Parent
/// tasks are never counted in addition to their children. When no eligible
/// leaves exist the result carries a display label instead of a percentage.
pub fn calculate_progress(
    conn: &Connection,
    scope: &ProgressScope<'_>,
) -> Result<ProgressResult, AppError> {
    let result = match scope {
        ProgressScope::Project(id) => project_progress(conn, id),
        ProgressScope::Goal(id) => goal_progress(conn, id),
        ProgressScope::Task(id) => task_progress(conn, id),
    };
    if let Ok(result) = &result {
        log::debug!(
            "progress: {} label={} percentage={:?} points={}/{} contributors={}",
            scope.log_key(),
            result.label,
            result.percentage,
            result.completed_points,
            result.total_points,
            result.contributors.len()
        );
    }
    result
}

/// The list of eligible leaves that make up a scope's numerator and denominator.
pub fn get_progress_contributors(
    conn: &Connection,
    scope: &ProgressScope<'_>,
) -> Result<Vec<ProgressContributor>, AppError> {
    let leaves = eligible_leaves(conn, scope)?;
    Ok(contributors_from(&leaves))
}

/// Leaf tasks directly owned by a project.
fn project_leaves_sql() -> String {
    format!(
        "SELECT {SELECT_COLS} FROM tasks
         WHERE project_id = ?1
           AND archived_at IS NULL
           AND status IN {ELIGIBLE_STATUSES}
           AND NOT EXISTS (
               SELECT 1 FROM tasks c WHERE c.parent_task_id = tasks.id AND c.archived_at IS NULL
           )
         ORDER BY created_at"
    )
}

/// Leaf tasks owned by a goal: direct goal tasks plus leaves under the goal's
/// projects. Direct tasks and project tasks cannot overlap because a task has
/// at most one owner context (project or goal), so nothing is double counted.
fn goal_leaves_sql() -> String {
    format!(
        "SELECT {SELECT_COLS} FROM tasks
         WHERE archived_at IS NULL
           AND status IN {ELIGIBLE_STATUSES}
           AND NOT EXISTS (
               SELECT 1 FROM tasks c WHERE c.parent_task_id = tasks.id AND c.archived_at IS NULL
           )
           AND (
               (goal_id = ?1 AND project_id IS NULL)
               OR project_id IN (SELECT id FROM projects WHERE goal_id = ?1 AND archived_at IS NULL)
           )
         ORDER BY created_at"
    )
}

/// A task with no children; the task itself is the leaf of its scope.
fn task_self_leaf_sql() -> String {
    format!(
        "SELECT {SELECT_COLS} FROM tasks
         WHERE id = ?1
           AND archived_at IS NULL
           AND status IN {ELIGIBLE_STATUSES}"
    )
}

/// Descendant leaves of a parent task.
fn task_descendant_leaves_sql() -> String {
    format!(
        "WITH RECURSIVE subtree(id) AS (
             SELECT id FROM tasks WHERE parent_task_id = ?1
             UNION ALL
             SELECT t.id FROM tasks t JOIN subtree s ON t.parent_task_id = s.id
         )
         SELECT {SELECT_COLS} FROM tasks
         WHERE id IN (SELECT id FROM subtree)
           AND archived_at IS NULL
           AND status IN {ELIGIBLE_STATUSES}
           AND NOT EXISTS (
               SELECT 1 FROM tasks c WHERE c.parent_task_id = tasks.id AND c.archived_at IS NULL
           )
         ORDER BY created_at"
    )
}

fn query_rows(
    conn: &Connection,
    sql: &str,
    params: &[&str],
    scope: &ProgressScope<'_>,
) -> Result<Vec<Task>, AppError> {
    let mut stmt = conn.prepare(sql)?;
    let rows = stmt.query_map(rusqlite::params_from_iter(params), row_to_task)?;
    let mut result = Vec::new();
    for row in rows {
        let task = row?;
        log::debug!(
            "progress: {} eligible leaf task={} status={} points={}",
            scope.log_key(),
            task.id,
            task.status,
            task.effort_points
        );
        result.push(task);
    }
    Ok(result)
}

fn finalize(leaves: Vec<Task>, scope: &ProgressScope<'_>) -> ProgressResult {
    if leaves.is_empty() {
        log::debug!(
            "progress: {} has no eligible leaves; display=not_started",
            scope.log_key()
        );
        return ProgressResult::empty("not_started");
    }
    let mut completed_points = 0i64;
    let mut total_points = 0i64;
    for task in &leaves {
        let points = if task.effort_points > 0 {
            task.effort_points
        } else {
            1
        };
        total_points += points;
        if task.status == TaskStatus::Completed {
            completed_points += points;
        }
    }
    let percentage = (completed_points as f64 / total_points as f64) * 100.0;
    let percentage = (percentage * 10.0).round() / 10.0;
    ProgressResult {
        percentage: Some(percentage),
        label: String::new(),
        completed_points,
        total_points,
        contributors: contributors_from(&leaves),
    }
}

fn project_progress(conn: &Connection, project_id: &str) -> Result<ProgressResult, AppError> {
    let status: String = conn
        .query_row(
            "SELECT status FROM projects WHERE id = ?1",
            [project_id],
            |row| row.get(0),
        )
        .optional()?
        .ok_or_else(|| AppError::not_found("The project was not found."))?;
    if status == "archived" {
        return Ok(ProgressResult::empty("archived"));
    }
    if status == "cancelled" {
        return Ok(ProgressResult::empty("cancelled"));
    }
    let task_count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM tasks WHERE project_id = ?1 AND archived_at IS NULL",
        [project_id],
        |row| row.get(0),
    )?;
    if task_count == 0 {
        return Ok(ProgressResult::empty("no_tasks"));
    }
    let scope = ProgressScope::Project(project_id);
    let leaves = eligible_leaves(conn, &scope)?;
    Ok(finalize(leaves, &scope))
}

fn goal_progress(conn: &Connection, goal_id: &str) -> Result<ProgressResult, AppError> {
    let status: String = conn
        .query_row("SELECT status FROM goals WHERE id = ?1", [goal_id], |row| {
            row.get(0)
        })
        .optional()?
        .ok_or_else(|| AppError::not_found("The goal was not found."))?;
    if status == "archived" {
        return Ok(ProgressResult::empty("archived"));
    }
    if status == "abandoned" {
        return Ok(ProgressResult::empty("abandoned"));
    }
    let task_count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM tasks
         WHERE archived_at IS NULL
           AND ((goal_id = ?1 AND project_id IS NULL)
                OR project_id IN (SELECT id FROM projects WHERE goal_id = ?1 AND archived_at IS NULL))",
        [goal_id],
        |row| row.get(0),
    )?;
    if task_count == 0 {
        return Ok(ProgressResult::empty("no_tasks"));
    }
    let scope = ProgressScope::Goal(goal_id);
    let leaves = eligible_leaves(conn, &scope)?;
    Ok(finalize(leaves, &scope))
}

fn task_progress(conn: &Connection, task_id: &str) -> Result<ProgressResult, AppError> {
    let task = crate::repos::task_repo::SqliteTaskRepo.get(conn, task_id)?;
    if task.archived_at.is_some() {
        return Ok(ProgressResult::empty("archived"));
    }
    let scope = ProgressScope::Task(task_id);
    let leaves = eligible_leaves(conn, &scope)?;
    Ok(finalize(leaves, &scope))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::test_conn;
    use crate::time;

    fn insert_task(
        conn: &Connection,
        id: &str,
        title: &str,
        status: &str,
        parent: Option<&str>,
        project: Option<&str>,
        goal: Option<&str>,
        effort: Option<&str>,
        archived: bool,
    ) {
        let now = time::now_rfc3339();
        conn.execute(
            "INSERT INTO tasks (id, workspace_id, project_id, goal_id, parent_task_id, title, status, effort_size, archived_at, created_at, updated_at)
             VALUES (?1, 'w1', ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?9)",
            rusqlite::params![
                id,
                project,
                goal,
                parent,
                title,
                status,
                effort,
                if archived { Some(&now) } else { None },
                now
            ],
        )
        .expect("insert task");
    }

    fn insert_project(conn: &Connection, id: &str, status: &str) {
        let now = time::now_rfc3339();
        conn.execute(
            "INSERT INTO projects (id, workspace_id, title, status, created_at, updated_at)
             VALUES (?1, 'w1', ?2, ?3, ?4, ?4)",
            rusqlite::params![id, format!("project {id}"), status, now],
        )
        .expect("insert project");
    }

    fn insert_goal(conn: &Connection, id: &str, status: &str) {
        let now = time::now_rfc3339();
        conn.execute(
            "INSERT INTO goals (id, workspace_id, title, status, created_at, updated_at)
             VALUES (?1, 'w1', ?2, ?3, ?4, ?4)",
            rusqlite::params![id, format!("goal {id}"), status, now],
        )
        .expect("insert goal");
    }

    #[test]
    fn parent_with_children_is_not_a_leaf() {
        let conn = test_conn();
        insert_task(
            &conn,
            "p",
            "Parent",
            "planned",
            None,
            None,
            None,
            Some("medium"),
            false,
        );
        insert_task(
            &conn,
            "c1",
            "Child 1",
            "planned",
            Some("p"),
            None,
            None,
            Some("small"),
            false,
        );
        assert!(!is_leaf_task(&conn, "p").expect("parent not a leaf"));
        assert!(is_leaf_task(&conn, "c1").expect("child is a leaf"));
    }

    #[test]
    fn archived_child_is_excluded_from_leaf_detection() {
        let conn = test_conn();
        insert_task(
            &conn,
            "p",
            "Parent",
            "planned",
            None,
            None,
            None,
            Some("medium"),
            false,
        );
        insert_task(
            &conn,
            "c1",
            "Child 1",
            "completed",
            Some("p"),
            None,
            None,
            Some("small"),
            true,
        );
        assert!(
            is_leaf_task(&conn, "p").expect("parent with only archived children is a leaf"),
            "archived children are not counted as children"
        );
    }

    #[test]
    fn cancelled_task_is_excluded_from_denominator() {
        let conn = test_conn();
        insert_project(&conn, "prj", "active");
        insert_task(
            &conn,
            "done",
            "Done",
            "completed",
            None,
            Some("prj"),
            None,
            Some("large"),
            false,
        );
        insert_task(
            &conn,
            "cancel",
            "Cancelled",
            "cancelled",
            None,
            Some("prj"),
            None,
            Some("small"),
            false,
        );
        insert_task(
            &conn,
            "defer",
            "Deferred",
            "deferred",
            None,
            Some("prj"),
            None,
            Some("small"),
            false,
        );
        let result = calculate_progress(&conn, &ProgressScope::Project("prj")).expect("progress");
        assert_eq!(
            result.total_points, 5,
            "only the completed 5-point leaf counts"
        );
        assert_eq!(result.completed_points, 5);
        assert_eq!(result.percentage, Some(100.0));
    }

    #[test]
    fn simple_average_two_of_four_is_fifty_percent() {
        let conn = test_conn();
        insert_project(&conn, "prj", "active");
        insert_task(
            &conn,
            "t1",
            "T1",
            "completed",
            None,
            Some("prj"),
            None,
            Some("small"),
            false,
        );
        insert_task(
            &conn,
            "t2",
            "T2",
            "completed",
            None,
            Some("prj"),
            None,
            Some("small"),
            false,
        );
        insert_task(
            &conn,
            "t3",
            "T3",
            "planned",
            None,
            Some("prj"),
            None,
            Some("small"),
            false,
        );
        insert_task(
            &conn,
            "t4",
            "T4",
            "planned",
            None,
            Some("prj"),
            None,
            Some("small"),
            false,
        );
        let result = calculate_progress(&conn, &ProgressScope::Project("prj")).expect("progress");
        assert_eq!(result.total_points, 8);
        assert_eq!(result.completed_points, 4);
        assert_eq!(result.percentage, Some(50.0));
    }

    #[test]
    fn weighted_progress_uses_effort_points() {
        let conn = test_conn();
        insert_project(&conn, "prj", "active");
        insert_task(
            &conn,
            "done",
            "Done",
            "completed",
            None,
            Some("prj"),
            None,
            Some("very_large"),
            false,
        );
        insert_task(
            &conn,
            "open",
            "Open",
            "planned",
            None,
            Some("prj"),
            None,
            Some("trivial"),
            false,
        );
        let result = calculate_progress(&conn, &ProgressScope::Project("prj")).expect("progress");
        assert_eq!(result.total_points, 9);
        assert_eq!(result.completed_points, 8);
        assert!(
            (result.percentage.unwrap() - 88.9).abs() < 0.01,
            "got {}",
            result.percentage.unwrap()
        );
    }

    #[test]
    fn parent_is_not_double_counted() {
        let conn = test_conn();
        insert_project(&conn, "prj", "active");
        insert_task(
            &conn,
            "p",
            "Parent",
            "in_progress",
            None,
            Some("prj"),
            None,
            Some("large"),
            false,
        );
        insert_task(
            &conn,
            "c1",
            "Child 1",
            "completed",
            Some("p"),
            Some("prj"),
            None,
            Some("medium"),
            false,
        );
        insert_task(
            &conn,
            "c2",
            "Child 2",
            "planned",
            Some("p"),
            Some("prj"),
            None,
            Some("small"),
            false,
        );
        let result = calculate_progress(&conn, &ProgressScope::Project("prj")).expect("progress");
        assert_eq!(
            result.total_points, 5,
            "parent's 5 points are not added to its children"
        );
        assert_eq!(result.completed_points, 3);
        assert!((result.percentage.unwrap() - 60.0).abs() < 0.01);
    }

    #[test]
    fn goal_direct_task_and_project_task_not_double_counted() {
        let conn = test_conn();
        insert_goal(&conn, "goal", "active");
        insert_project(&conn, "prj", "active");
        conn.execute("UPDATE projects SET goal_id = 'goal' WHERE id = 'prj'", [])
            .expect("link project to goal");
        // A direct goal task.
        insert_task(
            &conn,
            "direct",
            "Direct",
            "completed",
            None,
            None,
            Some("goal"),
            Some("medium"),
            false,
        );
        // A task under the goal's project.
        insert_task(
            &conn,
            "projected",
            "In project",
            "planned",
            None,
            Some("prj"),
            None,
            Some("medium"),
            false,
        );
        let result = calculate_progress(&conn, &ProgressScope::Goal("goal")).expect("progress");
        assert_eq!(result.contributors.len(), 2, "both leaves contribute");
        assert_eq!(result.total_points, 6);
        assert_eq!(result.completed_points, 3);
        assert_eq!(result.percentage, Some(50.0));
    }

    #[test]
    fn manually_closed_parent_with_open_children_reflects_reality() {
        let conn = test_conn();
        insert_project(&conn, "prj", "active");
        insert_task(
            &conn,
            "p",
            "Parent",
            "completed",
            None,
            Some("prj"),
            None,
            Some("large"),
            false,
        );
        conn.execute(
            "UPDATE tasks SET closure_mode = 'manual_accepted_partial' WHERE id = 'p'",
            [],
        )
        .expect("manual closure");
        insert_task(
            &conn,
            "c1",
            "Inventory",
            "completed",
            Some("p"),
            Some("prj"),
            None,
            Some("small"),
            false,
        );
        insert_task(
            &conn,
            "c2",
            "Remaining",
            "planned",
            Some("p"),
            Some("prj"),
            None,
            Some("medium"),
            false,
        );
        let result = calculate_progress(&conn, &ProgressScope::Task("p")).expect("progress");
        assert_eq!(result.total_points, 5);
        assert_eq!(result.completed_points, 2);
        assert_eq!(
            result.percentage,
            Some(40.0),
            "manual parent closure does not complete open children"
        );
    }

    #[test]
    fn empty_project_shows_no_tasks() {
        let conn = test_conn();
        insert_project(&conn, "prj", "active");
        let result = calculate_progress(&conn, &ProgressScope::Project("prj")).expect("progress");
        assert_eq!(result.percentage, None);
        assert_eq!(result.label, "no_tasks");
    }

    #[test]
    fn archived_project_shows_archived() {
        let conn = test_conn();
        insert_project(&conn, "prj", "archived");
        let result = calculate_progress(&conn, &ProgressScope::Project("prj")).expect("progress");
        assert_eq!(result.label, "archived");
        assert_eq!(result.percentage, None);
    }

    #[test]
    fn leaf_with_no_effort_defaults_to_one_point() {
        let conn = test_conn();
        insert_project(&conn, "prj", "active");
        insert_task(
            &conn,
            "t1",
            "No effort",
            "completed",
            None,
            Some("prj"),
            None,
            None,
            false,
        );
        let result = calculate_progress(&conn, &ProgressScope::Project("prj")).expect("progress");
        assert_eq!(result.total_points, 1);
        assert_eq!(result.completed_points, 1);
    }

    #[test]
    fn rest_block_does_not_affect_progress() {
        let conn = test_conn();
        insert_project(&conn, "prj", "active");
        insert_task(
            &conn,
            "open",
            "Open",
            "planned",
            None,
            Some("prj"),
            None,
            Some("medium"),
            false,
        );
        let before = calculate_progress(&conn, &ProgressScope::Project("prj")).expect("before");
        assert_eq!(before.percentage, Some(0.0));

        // A rest block and a session for a task exist as time information and
        // must not change the weighted progress.
        let now = crate::time::now_rfc3339();
        conn.execute(
            "INSERT INTO blocks (id, workspace_id, date, type, status, flexibility, created_at, updated_at)
             VALUES ('rest-1', 'w1', '2026-09-10', 'rest', 'completed', 'flexible', ?1, ?1)",
            [&now],
        )
        .expect("insert rest block");
        conn.execute(
            "INSERT INTO task_sessions (id, task_id, block_id, status, created_at, updated_at)
             VALUES ('session-1', 'open', 'rest-1', 'finished', ?1, ?1)",
            [&now],
        )
        .expect("insert session");

        let after = calculate_progress(&conn, &ProgressScope::Project("prj")).expect("after");
        assert_eq!(after.total_points, before.total_points);
        assert_eq!(after.completed_points, before.completed_points);
        assert_eq!(after.percentage, before.percentage);
        assert_eq!(after.percentage, Some(0.0));
    }
}
