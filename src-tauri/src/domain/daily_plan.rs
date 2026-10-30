use rusqlite::Connection;

use crate::domain::models::DailyPlanTask;
use crate::domain::types::{
    HistorySource, PlannedStatus, ReplanDecision, SelectionReason, TaskStatus,
};
use crate::error::AppError;
use crate::repos::daily_plan_repo::{
    get_plan_task, select_task, update_planned_status, DailyPlanRepo, SqliteDailyPlanRepo,
};
use crate::repos::task_repo::task_exists;
use crate::time;

/// Apply an explicit replanning decision to a missed or changed daily plan
/// task.
///
/// Replanning is always deliberate: nothing is silently copied or rescheduled,
/// and a task's `target_date` is never modified. Every decision records a note
/// in `decision_note_markdown` on the plan task and, where relevant, updates
/// `scheduled_date` or the task status with history.
pub fn replan_task(
    conn: &Connection,
    daily_plan_task_id: &str,
    decision: ReplanDecision,
    note: &Option<String>,
) -> Result<DailyPlanTask, AppError> {
    let row = get_plan_task(conn, daily_plan_task_id)?;
    let plan = SqliteDailyPlanRepo.get(conn, &row.daily_plan_id)?;

    let task_id = row.task_id.clone();
    if !task_exists(conn, &task_id)? {
        return Err(AppError::not_found("The task was not found."));
    }

    let task_status: String =
        conn.query_row("SELECT status FROM tasks WHERE id = ?1", [&task_id], |r| {
            r.get(0)
        })?;
    if task_status == "completed" {
        return Err(AppError::validation(
            "A completed task cannot be replanned. Reopen it explicitly first.",
        ));
    }

    let note_text = decision_note(decision, note.as_deref());

    match decision {
        ReplanDecision::MoveToNextDay => {
            let next_date = time::next_day(&plan.date)?;
            let next_plan =
                SqliteDailyPlanRepo.get_or_create(conn, &plan.workspace_id, &next_date)?;
            select_task(
                conn,
                &next_plan.id,
                &task_id,
                SelectionReason::Manual,
                None,
                Some(&plan.id),
            )?;
            conn.execute(
                "UPDATE tasks SET scheduled_date = ?2, updated_at = ?3 WHERE id = ?1",
                rusqlite::params![&task_id, next_date, time::now_rfc3339()],
            )?;
            update_planned_status(
                conn,
                daily_plan_task_id,
                PlannedStatus::Skipped,
                Some(&note_text),
            )
        }
        ReplanDecision::ReturnToPlanned => {
            transition_if_needed(conn, &task_id, TaskStatus::Planned, &note_text)?;
            update_planned_status(
                conn,
                daily_plan_task_id,
                PlannedStatus::Skipped,
                Some(&note_text),
            )
        }
        ReplanDecision::RemoveDate => {
            conn.execute(
                "UPDATE tasks SET scheduled_date = NULL, updated_at = ?2 WHERE id = ?1",
                rusqlite::params![&task_id, time::now_rfc3339()],
            )?;
            update_planned_status(
                conn,
                daily_plan_task_id,
                PlannedStatus::Skipped,
                Some(&note_text),
            )
        }
        ReplanDecision::Defer => {
            transition_if_needed(conn, &task_id, TaskStatus::Deferred, &note_text)?;
            update_planned_status(
                conn,
                daily_plan_task_id,
                PlannedStatus::Deferred,
                Some(&note_text),
            )
        }
        ReplanDecision::Divide => {
            // The UI owns child-task creation; this records the decision so the
            // user can decompose the work without losing the intent.
            update_planned_status(
                conn,
                daily_plan_task_id,
                row.planned_status,
                Some(&note_text),
            )
        }
    }
}

/// Transition a task to a status unless it already has that status.
fn transition_if_needed(
    conn: &Connection,
    task_id: &str,
    to: TaskStatus,
    reason: &str,
) -> Result<(), AppError> {
    let current: String =
        conn.query_row("SELECT status FROM tasks WHERE id = ?1", [task_id], |r| {
            r.get(0)
        })?;
    if current == to.to_string() {
        return Ok(());
    }
    crate::domain::task_closure::transition_status(
        conn,
        task_id,
        to,
        &Some(reason.to_string()),
        HistorySource::User,
    )?;
    Ok(())
}

/// Build a human-readable decision note that records which replan decision was
/// made and any user-provided note.
fn decision_note(decision: ReplanDecision, note: Option<&str>) -> String {
    let label = match decision {
        ReplanDecision::MoveToNextDay => "Moved to next day.",
        ReplanDecision::ReturnToPlanned => "Returned to planned.",
        ReplanDecision::RemoveDate => "Removed date.",
        ReplanDecision::Defer => "Deferred.",
        ReplanDecision::Divide => "Divided into children.",
    };
    match note {
        Some(note) if !note.trim().is_empty() => format!("{label} {note}"),
        _ => label.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::types::Priority;
    use crate::repos::daily_plan_repo::{list_plan_tasks, DailyPlanRepo, SqliteDailyPlanRepo};
    use crate::repos::task_repo::{SqliteTaskRepo, TaskRepo};
    use crate::test_utils::test_conn;

    fn create_task(conn: &Connection, title: &str) -> crate::domain::models::Task {
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
                Priority::Normal,
                &None,
                &None,
                &Some("2026-09-10".to_string()),
            )
            .expect("create task")
    }

    fn select_for_today(conn: &Connection, task_id: &str) -> String {
        let plan = SqliteDailyPlanRepo
            .get_or_create(conn, "w1", "2026-09-10")
            .expect("plan");
        let row = select_task(conn, &plan.id, task_id, SelectionReason::Manual, None, None)
            .expect("select");
        row.id
    }

    #[test]
    fn move_to_next_day_opens_next_plan_and_sets_scheduled_date() {
        let conn = test_conn();
        let task = create_task(&conn, "Carry me");
        let row_id = select_for_today(&conn, &task.id);

        replan_task(&conn, &row_id, ReplanDecision::MoveToNextDay, &None).expect("replan");

        let next_plan = SqliteDailyPlanRepo
            .get_by_date(&conn, "w1", "2026-09-11")
            .expect("next plan exists")
            .expect("next plan");
        let today_plan = SqliteDailyPlanRepo
            .get_by_date(&conn, "w1", "2026-09-10")
            .expect("today plan exists")
            .expect("today plan");
        let tasks = list_plan_tasks(&conn, &next_plan.id).expect("next tasks");
        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks[0].task_id, task.id);
        assert_eq!(
            tasks[0].carryover_from_daily_plan_id.as_deref(),
            Some(today_plan.id.as_str()),
            "the carryover points to the original plan"
        );

        let after = SqliteTaskRepo.get(&conn, &task.id).expect("task");
        assert_eq!(after.scheduled_date.as_deref(), Some("2026-09-11"));
        assert_eq!(
            after.target_date, None,
            "the deadline never changes during replanning"
        );

        let today_tasks = list_plan_tasks(&conn, &today_plan.id).expect("today tasks");
        assert_eq!(today_tasks[0].planned_status, PlannedStatus::Skipped);
    }

    #[test]
    fn return_to_planned_keeps_row_and_sets_task_status() {
        let conn = test_conn();
        let task = create_task(&conn, "Put back");
        crate::domain::task_closure::transition_status(
            &conn,
            &task.id,
            TaskStatus::InProgress,
            &None,
            HistorySource::User,
        )
        .expect("start task");
        let row_id = select_for_today(&conn, &task.id);

        replan_task(
            &conn,
            &row_id,
            ReplanDecision::ReturnToPlanned,
            &Some("Not today.".to_string()),
        )
        .expect("replan");

        let after = SqliteTaskRepo.get(&conn, &task.id).expect("task");
        assert_eq!(after.status, TaskStatus::Planned);
        let row = get_plan_task(&conn, &row_id).expect("row");
        assert_eq!(row.planned_status, PlannedStatus::Skipped);
        assert!(row
            .decision_note_markdown
            .as_deref()
            .unwrap()
            .contains("Not today."));
    }

    #[test]
    fn remove_date_clears_scheduled_date() {
        let conn = test_conn();
        let task = create_task(&conn, "No date");
        let row_id = select_for_today(&conn, &task.id);
        assert_eq!(task.scheduled_date.as_deref(), Some("2026-09-10"));

        replan_task(&conn, &row_id, ReplanDecision::RemoveDate, &None).expect("replan");

        let after = SqliteTaskRepo.get(&conn, &task.id).expect("task");
        assert_eq!(after.scheduled_date, None);
        let row = get_plan_task(&conn, &row_id).expect("row");
        assert_eq!(row.planned_status, PlannedStatus::Skipped);
    }

    #[test]
    fn defer_sets_task_status_and_marks_row() {
        let conn = test_conn();
        let task = create_task(&conn, "Defer me");
        let row_id = select_for_today(&conn, &task.id);

        replan_task(&conn, &row_id, ReplanDecision::Defer, &None).expect("replan");

        let after = SqliteTaskRepo.get(&conn, &task.id).expect("task");
        assert_eq!(after.status, TaskStatus::Deferred);
        let row = get_plan_task(&conn, &row_id).expect("row");
        assert_eq!(row.planned_status, PlannedStatus::Deferred);
    }

    #[test]
    fn completed_task_cannot_be_replanned() {
        let conn = test_conn();
        let task = create_task(&conn, "Done");
        crate::domain::task_closure::transition_status(
            &conn,
            &task.id,
            TaskStatus::Completed,
            &None,
            HistorySource::User,
        )
        .expect("complete");
        let row_id = select_for_today(&conn, &task.id);
        assert!(
            replan_task(&conn, &row_id, ReplanDecision::Defer, &None).is_err(),
            "replanning a completed task is rejected"
        );
    }

    #[test]
    fn divide_records_note_without_changing_status() {
        let conn = test_conn();
        let task = create_task(&conn, "Split me");
        let row_id = select_for_today(&conn, &task.id);

        replan_task(
            &conn,
            &row_id,
            ReplanDecision::Divide,
            &Some("Split into three parts.".to_string()),
        )
        .expect("replan");

        let row = get_plan_task(&conn, &row_id).expect("row");
        assert_eq!(row.planned_status, PlannedStatus::Selected);
        assert!(row
            .decision_note_markdown
            .as_deref()
            .unwrap()
            .contains("Split into three parts."));
    }
}
