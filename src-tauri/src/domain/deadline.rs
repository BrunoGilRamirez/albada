use chrono::NaiveDate;
use rusqlite::Connection;
use serde::Serialize;

use crate::domain::models::Task;
use crate::domain::progress::{self, ProgressScope};
use crate::domain::types::TaskStatus;
use crate::error::AppError;
use crate::repos::goal_repo::{GoalRepo, SqliteGoalRepo};
use crate::repos::project_repo::{ProjectRepo, SqliteProjectRepo};
use crate::repos::task_repo::{SqliteTaskRepo, TaskRepo};

/// Derived deadline condition for a task or an aggregated scope.
///
/// Severity ordering (used for aggregation and sorting) is intentional:
/// `overdue` is the worst, then `blocked_risk`, then `approaching`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DeadlineStatus {
    None,
    Normal,
    Approaching,
    BlockedRisk,
    Overdue,
}

impl DeadlineStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            DeadlineStatus::None => "none",
            DeadlineStatus::Normal => "normal",
            DeadlineStatus::Approaching => "approaching",
            DeadlineStatus::BlockedRisk => "blocked_risk",
            DeadlineStatus::Overdue => "overdue",
        }
    }
}

/// Deadline information for one task, used in risk lists and DTOs.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DeadlineTaskInfo {
    pub task_id: String,
    pub title: String,
    pub status: String,
    /// The task's deadline (`target_date` column), as `YYYY-MM-DD` or `None`.
    pub deadline_at: Option<String>,
    pub deadline_status: DeadlineStatus,
}

/// Aggregate deadline status for a project, goal, or single task.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DeadlineStatusDto {
    pub status: DeadlineStatus,
    /// Tasks that are approaching, at risk of blockage, or overdue. Sorted by
    /// severity (overdue first) then by deadline so the cause is understandable.
    pub at_risk_tasks: Vec<DeadlineTaskInfo>,
    pub warning_window_days: u32,
}

/// Derive the deadline condition for a single task.
///
/// Rules:
/// - `none`: no `deadline_at`, or the task is completed/cancelled/deferred.
/// - `overdue`: deadline before `now` and the task is still open.
/// - `approaching`: deadline within `warning_days` from `now` and not overdue.
/// - `blocked_risk`: the task has an incomplete prerequisite AND its deadline
///   is approaching or overdue.
/// - `normal`: deadline exists and is outside the warning window.
pub fn deadline_status_for_task(
    conn: &Connection,
    task_id: &str,
    now: &str,
    warning_days: u32,
) -> Result<DeadlineStatus, AppError> {
    let task = SqliteTaskRepo.get(conn, task_id)?;
    let status = deadline_status_for_task_model(conn, &task, now, warning_days)?;
    log::debug!(
        "deadline: task {task_id} now={now} warning_days={warning_days} status={}",
        status.as_str()
    );
    Ok(status)
}

fn deadline_status_for_task_model(
    conn: &Connection,
    task: &Task,
    now: &str,
    warning_days: u32,
) -> Result<DeadlineStatus, AppError> {
    let deadline = task.target_date.as_deref();
    if deadline.is_none()
        || matches!(
            task.status,
            TaskStatus::Completed | TaskStatus::Cancelled | TaskStatus::Deferred
        )
    {
        return Ok(DeadlineStatus::None);
    }
    let deadline = deadline.unwrap();

    let now_date = NaiveDate::parse_from_str(now, "%Y-%m-%d")
        .map_err(|_| AppError::validation(format!("Invalid reference date '{now}'.")))?;
    let deadline_date = NaiveDate::parse_from_str(deadline, "%Y-%m-%d")
        .map_err(|_| AppError::validation(format!("Invalid deadline date '{deadline}'.")))?;

    let is_overdue = deadline_date < now_date;
    let window_end = now_date
        .checked_add_days(chrono::Days::new(warning_days as u64))
        .unwrap_or(now_date);
    let is_approaching = !is_overdue && deadline_date <= window_end;

    let blocked_by_dependency =
        !crate::domain::dependency_graph::incomplete_prerequisites(conn, &task.id)?.is_empty();

    if blocked_by_dependency && (is_overdue || is_approaching) {
        return Ok(DeadlineStatus::BlockedRisk);
    }
    if is_overdue {
        return Ok(DeadlineStatus::Overdue);
    }
    if is_approaching {
        return Ok(DeadlineStatus::Approaching);
    }
    Ok(DeadlineStatus::Normal)
}

/// Aggregate the worst deadline status across a project's eligible leaves.
pub fn deadline_status_for_project(
    conn: &Connection,
    project_id: &str,
    now: &str,
    warning_days: u32,
) -> Result<DeadlineStatusDto, AppError> {
    let project = SqliteProjectRepo.get(conn, project_id)?;
    let leaves = progress::eligible_leaves(conn, &ProgressScope::Project(project_id))?;
    let dto = aggregate(conn, &leaves, now, warning_days, project.workspace_id)?;
    log::debug!(
        "deadline: project {project_id} worst={} at_risk={}",
        dto.status.as_str(),
        dto.at_risk_tasks.len()
    );
    Ok(dto)
}

/// Aggregate the worst deadline status across a goal's direct tasks and project leaves.
pub fn deadline_status_for_goal(
    conn: &Connection,
    goal_id: &str,
    now: &str,
    warning_days: u32,
) -> Result<DeadlineStatusDto, AppError> {
    let goal = SqliteGoalRepo.get(conn, goal_id)?;
    let leaves = progress::eligible_leaves(conn, &ProgressScope::Goal(goal_id))?;
    let dto = aggregate(conn, &leaves, now, warning_days, goal.workspace_id)?;
    log::debug!(
        "deadline: goal {goal_id} worst={} at_risk={}",
        dto.status.as_str(),
        dto.at_risk_tasks.len()
    );
    Ok(dto)
}

/// Build a DTO from a set of eligible leaves, returning the worst status and
/// every task that is approaching, at risk of blockage, or overdue.
fn aggregate(
    conn: &Connection,
    leaves: &[Task],
    now: &str,
    warning_days: u32,
    workspace_id: String,
) -> Result<DeadlineStatusDto, AppError> {
    let mut worst = DeadlineStatus::None;
    let mut infos = Vec::new();
    for task in leaves {
        let status = deadline_status_for_task_model(conn, task, now, warning_days)?;
        if status > worst {
            worst = status;
        }
        if status >= DeadlineStatus::Approaching {
            infos.push(DeadlineTaskInfo {
                task_id: task.id.clone(),
                title: task.title.clone(),
                status: task.status.to_string(),
                deadline_at: task.target_date.clone(),
                deadline_status: status,
            });
        }
    }
    infos.sort_by(|a, b| {
        b.deadline_status
            .cmp(&a.deadline_status)
            .then_with(|| a.deadline_at.cmp(&b.deadline_at))
            .then_with(|| a.title.cmp(&b.title))
    });
    log::debug!(
        "deadline: aggregate workspace={workspace_id} leaves={} worst={:?}",
        leaves.len(),
        worst
    );
    Ok(DeadlineStatusDto {
        status: worst,
        at_risk_tasks: infos,
        warning_window_days: warning_days,
    })
}

/// Every open task in the workspace that is approaching, at risk of blockage,
/// or overdue, sorted by severity then deadline. Powers the dashboard widget.
pub fn at_risk_tasks(
    conn: &Connection,
    workspace_id: &str,
    now: &str,
    warning_days: u32,
) -> Result<Vec<DeadlineTaskInfo>, AppError> {
    let tasks = SqliteTaskRepo.list(conn, workspace_id)?;
    let mut infos = Vec::new();
    for task in tasks {
        let status = deadline_status_for_task_model(conn, &task, now, warning_days)?;
        if status >= DeadlineStatus::Approaching {
            infos.push(DeadlineTaskInfo {
                task_id: task.id,
                title: task.title,
                status: task.status.to_string(),
                deadline_at: task.target_date,
                deadline_status: status,
            });
        }
    }
    infos.sort_by(|a, b| {
        b.deadline_status
            .cmp(&a.deadline_status)
            .then_with(|| a.deadline_at.cmp(&b.deadline_at))
            .then_with(|| a.title.cmp(&b.title))
    });
    log::debug!(
        "deadline: at_risk_tasks workspace={workspace_id} count={}",
        infos.len()
    );
    Ok(infos)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::test_conn;
    use crate::time;

    fn insert_task(
        conn: &Connection,
        id: &str,
        status: &str,
        parent: Option<&str>,
        project: Option<&str>,
        goal: Option<&str>,
        target_date: Option<&str>,
    ) {
        let now = time::now_rfc3339();
        conn.execute(
            "INSERT INTO tasks (id, workspace_id, project_id, goal_id, parent_task_id, title, status, target_date, created_at, updated_at)
             VALUES (?1, 'w1', ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?8)",
            rusqlite::params![
                id,
                project,
                goal,
                parent,
                format!("task {id}"),
                status,
                target_date,
                now
            ],
        )
        .expect("insert task");
    }

    fn insert_project(conn: &Connection, id: &str) {
        let now = time::now_rfc3339();
        conn.execute(
            "INSERT INTO projects (id, workspace_id, title, created_at, updated_at)
             VALUES (?1, 'w1', ?2, ?3, ?3)",
            rusqlite::params![id, format!("project {id}"), now],
        )
        .expect("insert project");
    }

    fn insert_dependency(conn: &Connection, blocked: &str, prereq: &str) {
        let now = time::now_rfc3339();
        conn.execute(
            "INSERT INTO task_dependencies (id, blocked_task_id, prerequisite_task_id, created_at)
             VALUES (?1, ?2, ?3, ?4)",
            rusqlite::params![format!("d-{blocked}-{prereq}"), blocked, prereq, now],
        )
        .expect("insert dependency");
    }

    #[test]
    fn task_with_no_deadline_is_none() {
        let conn = test_conn();
        insert_task(&conn, "t", "planned", None, None, None, None);
        assert_eq!(
            deadline_status_for_task(&conn, "t", "2026-09-04", 3).expect("status"),
            DeadlineStatus::None
        );
    }

    #[test]
    fn one_day_away_is_approaching() {
        let conn = test_conn();
        insert_task(&conn, "t", "planned", None, None, None, Some("2026-09-05"));
        assert_eq!(
            deadline_status_for_task(&conn, "t", "2026-09-04", 3).expect("status"),
            DeadlineStatus::Approaching
        );
    }

    #[test]
    fn one_day_past_is_overdue() {
        let conn = test_conn();
        insert_task(
            &conn,
            "t",
            "in_progress",
            None,
            None,
            None,
            Some("2026-09-03"),
        );
        assert_eq!(
            deadline_status_for_task(&conn, "t", "2026-09-04", 3).expect("status"),
            DeadlineStatus::Overdue
        );
    }

    #[test]
    fn outside_window_is_normal() {
        let conn = test_conn();
        insert_task(&conn, "t", "planned", None, None, None, Some("2026-09-20"));
        assert_eq!(
            deadline_status_for_task(&conn, "t", "2026-09-04", 3).expect("status"),
            DeadlineStatus::Normal
        );
    }

    #[test]
    fn completed_task_is_not_overdue() {
        let conn = test_conn();
        insert_task(
            &conn,
            "t",
            "completed",
            None,
            None,
            None,
            Some("2026-08-01"),
        );
        assert_eq!(
            deadline_status_for_task(&conn, "t", "2026-09-04", 3).expect("status"),
            DeadlineStatus::None
        );
    }

    #[test]
    fn blocked_task_with_approaching_deadline_is_blocked_risk() {
        let conn = test_conn();
        insert_task(&conn, "a", "in_progress", None, None, None, None);
        insert_task(
            &conn,
            "b",
            "in_progress",
            None,
            None,
            None,
            Some("2026-09-05"),
        );
        insert_dependency(&conn, "b", "a");
        assert_eq!(
            deadline_status_for_task(&conn, "b", "2026-09-04", 3).expect("status"),
            DeadlineStatus::BlockedRisk
        );
    }

    #[test]
    fn blocked_task_outside_window_is_normal() {
        let conn = test_conn();
        insert_task(&conn, "a", "in_progress", None, None, None, None);
        insert_task(
            &conn,
            "b",
            "in_progress",
            None,
            None,
            None,
            Some("2026-09-20"),
        );
        insert_dependency(&conn, "b", "a");
        assert_eq!(
            deadline_status_for_task(&conn, "b", "2026-09-04", 3).expect("status"),
            DeadlineStatus::Normal
        );
    }

    #[test]
    fn project_aggregates_worst_status() {
        let conn = test_conn();
        insert_project(&conn, "prj");
        insert_task(
            &conn,
            "far",
            "planned",
            None,
            Some("prj"),
            None,
            Some("2026-09-30"),
        );
        insert_task(
            &conn,
            "near",
            "planned",
            None,
            Some("prj"),
            None,
            Some("2026-09-05"),
        );
        insert_task(
            &conn,
            "late",
            "in_progress",
            None,
            Some("prj"),
            None,
            Some("2026-09-01"),
        );
        let dto = deadline_status_for_project(&conn, "prj", "2026-09-04", 3).expect("dto");
        assert_eq!(dto.status, DeadlineStatus::Overdue);
        assert!(dto.at_risk_tasks.iter().any(|t| t.task_id == "late"));
    }

    #[test]
    fn at_risk_tasks_are_sorted_by_severity() {
        let conn = test_conn();
        insert_task(
            &conn,
            "near",
            "planned",
            None,
            None,
            None,
            Some("2026-09-05"),
        );
        insert_task(
            &conn,
            "late",
            "in_progress",
            None,
            None,
            None,
            Some("2026-09-01"),
        );
        let infos = at_risk_tasks(&conn, "w1", "2026-09-04", 3).expect("at risk");
        assert_eq!(infos.len(), 2);
        assert_eq!(infos[0].task_id, "late", "overdue first");
        assert_eq!(infos[1].task_id, "near");
    }
}
