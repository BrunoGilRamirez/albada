use rusqlite::{params, Connection, OptionalExtension};

use crate::domain::models::{DailyPlan, DailyPlanTask};
use crate::domain::types::{DailyPlanStatus, PlannedStatus, SelectionReason};
use crate::error::AppError;
use crate::id;
use crate::time;

use super::year_theme_repo::map_constraint_violation;

pub trait DailyPlanRepo {
    /// Return the plan for a workspace and date, creating it when missing.
    fn get_or_create(
        &self,
        conn: &Connection,
        workspace_id: &str,
        date: &str,
    ) -> Result<DailyPlan, AppError>;
    fn get(&self, conn: &Connection, id: &str) -> Result<DailyPlan, AppError>;
    fn get_by_date(
        &self,
        conn: &Connection,
        workspace_id: &str,
        date: &str,
    ) -> Result<Option<DailyPlan>, AppError>;
    fn list_between(
        &self,
        conn: &Connection,
        workspace_id: &str,
        from_date: &str,
        to_date: &str,
    ) -> Result<Vec<DailyPlan>, AppError>;
    fn update_status(
        &self,
        conn: &Connection,
        id: &str,
        status: DailyPlanStatus,
    ) -> Result<DailyPlan, AppError>;
    fn set_planning_block(
        &self,
        conn: &Connection,
        id: &str,
        block_id: Option<&str>,
    ) -> Result<DailyPlan, AppError>;
    fn set_intention(
        &self,
        conn: &Connection,
        id: &str,
        intention_markdown: &Option<String>,
        capacity_note_markdown: &Option<String>,
    ) -> Result<DailyPlan, AppError>;
    fn set_review(
        &self,
        conn: &Connection,
        id: &str,
        review_markdown: &Option<String>,
    ) -> Result<DailyPlan, AppError>;
}

const PLAN_COLS: &str = "id, workspace_id, date, status, intention_markdown, capacity_note_markdown, review_markdown, planning_block_id, created_at, updated_at";
const PLAN_TASK_COLS: &str = "id, daily_plan_id, task_id, order_index, selection_reason, planned_status, carryover_from_daily_plan_id, decision_note_markdown, created_at, updated_at";

pub(crate) fn row_to_daily_plan(row: &rusqlite::Row<'_>) -> Result<DailyPlan, rusqlite::Error> {
    let status: String = row.get(3)?;
    Ok(DailyPlan {
        id: row.get(0)?,
        workspace_id: row.get(1)?,
        date: row.get(2)?,
        status: status.parse().map_err(|_| invalid_column(3, "status"))?,
        intention_markdown: row.get(4)?,
        capacity_note_markdown: row.get(5)?,
        review_markdown: row.get(6)?,
        planning_block_id: row.get(7)?,
        created_at: row.get(8)?,
        updated_at: row.get(9)?,
    })
}

pub(crate) fn row_to_plan_task(row: &rusqlite::Row<'_>) -> Result<DailyPlanTask, rusqlite::Error> {
    let reason: String = row.get(4)?;
    let planned: String = row.get(5)?;
    Ok(DailyPlanTask {
        id: row.get(0)?,
        daily_plan_id: row.get(1)?,
        task_id: row.get(2)?,
        order_index: row.get(3)?,
        selection_reason: reason.parse().map_err(|_| invalid_column(4, "reason"))?,
        planned_status: planned.parse().map_err(|_| invalid_column(5, "planned"))?,
        carryover_from_daily_plan_id: row.get(6)?,
        decision_note_markdown: row.get(7)?,
        created_at: row.get(8)?,
        updated_at: row.get(9)?,
    })
}

fn invalid_column(index: usize, name: &str) -> rusqlite::Error {
    rusqlite::Error::InvalidColumnType(index, name.to_string(), rusqlite::types::Type::Text)
}

pub struct SqliteDailyPlanRepo;

impl DailyPlanRepo for SqliteDailyPlanRepo {
    fn get_or_create(
        &self,
        conn: &Connection,
        workspace_id: &str,
        date: &str,
    ) -> Result<DailyPlan, AppError> {
        time::parse_date(date)?;
        if let Some(existing) = self.get_by_date(conn, workspace_id, date)? {
            return Ok(existing);
        }
        let id = id::new_uuid();
        let now = time::now_rfc3339();
        conn.execute(
            "INSERT INTO daily_plans (id, workspace_id, date, status, created_at, updated_at)
             VALUES (?1, ?2, ?3, 'not_started', ?4, ?4)",
            params![id, workspace_id, date, now],
        )
        .map_err(map_constraint_violation)?;
        self.get(conn, &id)
    }

    fn get(&self, conn: &Connection, id: &str) -> Result<DailyPlan, AppError> {
        conn.query_row(
            &format!("SELECT {PLAN_COLS} FROM daily_plans WHERE id = ?1"),
            [id],
            row_to_daily_plan,
        )
        .optional()?
        .ok_or_else(|| AppError::not_found("The daily plan was not found."))
    }

    fn get_by_date(
        &self,
        conn: &Connection,
        workspace_id: &str,
        date: &str,
    ) -> Result<Option<DailyPlan>, AppError> {
        conn.query_row(
            &format!("SELECT {PLAN_COLS} FROM daily_plans WHERE workspace_id = ?1 AND date = ?2"),
            params![workspace_id, date],
            row_to_daily_plan,
        )
        .optional()
        .map_err(Into::into)
    }

    fn list_between(
        &self,
        conn: &Connection,
        workspace_id: &str,
        from_date: &str,
        to_date: &str,
    ) -> Result<Vec<DailyPlan>, AppError> {
        time::parse_date(from_date)?;
        time::parse_date(to_date)?;
        let mut stmt = conn.prepare(&format!(
            "SELECT {PLAN_COLS} FROM daily_plans WHERE workspace_id = ?1 AND date BETWEEN ?2 AND ?3 ORDER BY date"
        ))?;
        let rows = stmt.query_map(params![workspace_id, from_date, to_date], row_to_daily_plan)?;
        let mut result = Vec::new();
        for row in rows {
            result.push(row?);
        }
        Ok(result)
    }

    fn update_status(
        &self,
        conn: &Connection,
        id: &str,
        status: DailyPlanStatus,
    ) -> Result<DailyPlan, AppError> {
        let updated = conn.execute(
            "UPDATE daily_plans SET status = ?2, updated_at = ?3 WHERE id = ?1",
            params![id, status.to_string(), time::now_rfc3339()],
        )?;
        if updated == 0 {
            return Err(AppError::not_found("The daily plan was not found."));
        }
        self.get(conn, id)
    }

    fn set_planning_block(
        &self,
        conn: &Connection,
        id: &str,
        block_id: Option<&str>,
    ) -> Result<DailyPlan, AppError> {
        let updated = conn.execute(
            "UPDATE daily_plans SET planning_block_id = ?2, updated_at = ?3 WHERE id = ?1",
            params![id, block_id, time::now_rfc3339()],
        )?;
        if updated == 0 {
            return Err(AppError::not_found("The daily plan was not found."));
        }
        self.get(conn, id)
    }

    fn set_intention(
        &self,
        conn: &Connection,
        id: &str,
        intention_markdown: &Option<String>,
        capacity_note_markdown: &Option<String>,
    ) -> Result<DailyPlan, AppError> {
        let updated = conn.execute(
            "UPDATE daily_plans SET intention_markdown = ?2, capacity_note_markdown = ?3, updated_at = ?4 WHERE id = ?1",
            params![id, intention_markdown, capacity_note_markdown, time::now_rfc3339()],
        )?;
        if updated == 0 {
            return Err(AppError::not_found("The daily plan was not found."));
        }
        self.get(conn, id)
    }

    fn set_review(
        &self,
        conn: &Connection,
        id: &str,
        review_markdown: &Option<String>,
    ) -> Result<DailyPlan, AppError> {
        let updated = conn.execute(
            "UPDATE daily_plans SET review_markdown = ?2, updated_at = ?3 WHERE id = ?1",
            params![id, review_markdown, time::now_rfc3339()],
        )?;
        if updated == 0 {
            return Err(AppError::not_found("The daily plan was not found."));
        }
        self.get(conn, id)
    }
}

/// Select a task for a daily plan.
///
/// The `UNIQUE(daily_plan_id, task_id)` constraint rejects a duplicate
/// selection for the same day. An `order_index` is assigned automatically when
/// not provided. Selecting a task never changes its `target_date` or
/// `scheduled_date`.
pub fn select_task(
    conn: &Connection,
    daily_plan_id: &str,
    task_id: &str,
    reason: SelectionReason,
    order_index: Option<i64>,
    carryover_from_daily_plan_id: Option<&str>,
) -> Result<DailyPlanTask, AppError> {
    let plan_exists: i64 = conn.query_row(
        "SELECT EXISTS (SELECT 1 FROM daily_plans WHERE id = ?1)",
        [daily_plan_id],
        |row| row.get(0),
    )?;
    if plan_exists == 0 {
        return Err(AppError::not_found("The daily plan was not found."));
    }
    let task_exists: i64 = conn.query_row(
        "SELECT EXISTS (SELECT 1 FROM tasks WHERE id = ?1)",
        [task_id],
        |row| row.get(0),
    )?;
    if task_exists == 0 {
        return Err(AppError::not_found("The task was not found."));
    }

    let order = match order_index {
        Some(index) => index,
        None => {
            let max: Option<i64> = conn
                .query_row(
                    "SELECT MAX(order_index) FROM daily_plan_tasks WHERE daily_plan_id = ?1",
                    [daily_plan_id],
                    |row| row.get(0),
                )
                .optional()?
                .flatten();
            max.unwrap_or(0) + 1
        }
    };

    let id = id::new_uuid();
    let now = time::now_rfc3339();
    conn.execute(
        "INSERT INTO daily_plan_tasks (id, daily_plan_id, task_id, order_index, selection_reason, planned_status, carryover_from_daily_plan_id, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, 'selected', ?6, ?7, ?7)",
        params![
            id,
            daily_plan_id,
            task_id,
            order,
            reason.to_string(),
            carryover_from_daily_plan_id,
            now
        ],
    )
    .map_err(map_constraint_violation)?;

    get_plan_task(conn, &id)
}

/// Fetch one daily-plan task row by id.
pub fn get_plan_task(
    conn: &Connection,
    daily_plan_task_id: &str,
) -> Result<DailyPlanTask, AppError> {
    conn.query_row(
        &format!("SELECT {PLAN_TASK_COLS} FROM daily_plan_tasks WHERE id = ?1"),
        [daily_plan_task_id],
        row_to_plan_task,
    )
    .optional()?
    .ok_or_else(|| AppError::not_found("The daily plan task was not found."))
}

/// Look up the plan-task row for a task within a plan, if any.
pub fn get_plan_task_by_task(
    conn: &Connection,
    daily_plan_id: &str,
    task_id: &str,
) -> Result<Option<DailyPlanTask>, AppError> {
    conn.query_row(
        &format!(
            "SELECT {PLAN_TASK_COLS} FROM daily_plan_tasks WHERE daily_plan_id = ?1 AND task_id = ?2"
        ),
        params![daily_plan_id, task_id],
        row_to_plan_task,
    )
    .optional()
    .map_err(Into::into)
}

/// List the selected tasks of a plan ordered by `order_index`.
pub fn list_plan_tasks(
    conn: &Connection,
    daily_plan_id: &str,
) -> Result<Vec<DailyPlanTask>, AppError> {
    let mut stmt = conn.prepare(&format!(
        "SELECT {PLAN_TASK_COLS} FROM daily_plan_tasks WHERE daily_plan_id = ?1 ORDER BY order_index"
    ))?;
    let rows = stmt.query_map([daily_plan_id], row_to_plan_task)?;
    let mut result = Vec::new();
    for row in rows {
        result.push(row?);
    }
    Ok(result)
}

/// Remove a task from a day. Associated sessions keep their `block_id` and
/// become orphaned with respect to the plan task (the reference is cleared).
pub fn deselect_task(conn: &Connection, daily_plan_task_id: &str) -> Result<(), AppError> {
    let deleted = conn.execute(
        "DELETE FROM daily_plan_tasks WHERE id = ?1",
        [daily_plan_task_id],
    )?;
    if deleted == 0 {
        return Err(AppError::not_found("The daily plan task was not found."));
    }
    Ok(())
}

/// Reorder the selected tasks of a plan. `ordered_ids` must contain exactly the
/// plan's current task ids; the order of the list becomes the new
/// `order_index`.
pub fn reorder_tasks(
    conn: &Connection,
    daily_plan_id: &str,
    ordered_ids: &[String],
) -> Result<(), AppError> {
    let current = list_plan_tasks(conn, daily_plan_id)?;
    let current_ids: std::collections::HashSet<String> =
        current.iter().map(|row| row.id.clone()).collect();
    let requested: std::collections::HashSet<&String> = ordered_ids.iter().collect();

    if current.len() != ordered_ids.len() || current_ids.iter().any(|id| !requested.contains(id)) {
        return Err(AppError::validation(
            "The reorder list must contain exactly the tasks selected for the day.",
        ));
    }

    let now = time::now_rfc3339();
    for (index, id) in ordered_ids.iter().enumerate() {
        conn.execute(
            "UPDATE daily_plan_tasks SET order_index = ?2, updated_at = ?3 WHERE id = ?1 AND daily_plan_id = ?4",
            params![id, index as i64, now, daily_plan_id],
        )?;
    }
    Ok(())
}

/// Update the planned status and decision note of a plan task.
pub fn update_planned_status(
    conn: &Connection,
    daily_plan_task_id: &str,
    status: PlannedStatus,
    decision_note_markdown: Option<&str>,
) -> Result<DailyPlanTask, AppError> {
    let updated = conn.execute(
        "UPDATE daily_plan_tasks SET planned_status = ?2, decision_note_markdown = ?3, updated_at = ?4 WHERE id = ?1",
        params![daily_plan_task_id, status.to_string(), decision_note_markdown, time::now_rfc3339()],
    )?;
    if updated == 0 {
        return Err(AppError::not_found("The daily plan task was not found."));
    }
    get_plan_task(conn, daily_plan_task_id)
}

#[cfg(test)]
mod tests {
    use super::*;
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
    fn get_or_create_is_idempotent() {
        let conn = test_conn();
        let repo = SqliteDailyPlanRepo;
        let first = repo
            .get_or_create(&conn, "w1", "2026-09-10")
            .expect("first");
        let second = repo
            .get_or_create(&conn, "w1", "2026-09-10")
            .expect("second");
        assert_eq!(first.id, second.id);
        assert_eq!(first.status, DailyPlanStatus::NotStarted);
    }

    #[test]
    fn duplicate_selection_for_day_is_rejected() {
        let conn = test_conn();
        let repo = SqliteDailyPlanRepo;
        let plan = repo.get_or_create(&conn, "w1", "2026-09-10").expect("plan");
        let task = create_task(&conn, "Task A");
        select_task(&conn, &plan.id, &task, SelectionReason::Manual, None, None)
            .expect("select once");
        assert!(
            select_task(&conn, &plan.id, &task, SelectionReason::Manual, None, None).is_err(),
            "task cannot be selected twice for the same day"
        );
    }

    #[test]
    fn selection_does_not_change_task_dates() {
        let conn = test_conn();
        let repo = SqliteDailyPlanRepo;
        let plan = repo.get_or_create(&conn, "w1", "2026-09-10").expect("plan");
        let task = SqliteTaskRepo
            .create(
                &conn,
                "w1",
                &None,
                &None,
                &None,
                "Deadline task",
                &None,
                &None,
                crate::domain::types::Priority::High,
                &None,
                &Some("2026-09-20".to_string()),
                &Some("2026-09-15".to_string()),
            )
            .expect("create");
        select_task(
            &conn,
            &plan.id,
            &task.id,
            SelectionReason::Deadline,
            None,
            None,
        )
        .expect("select");
        let after = SqliteTaskRepo.get(&conn, &task.id).expect("get task");
        assert_eq!(after.target_date.as_deref(), Some("2026-09-20"));
        assert_eq!(after.scheduled_date.as_deref(), Some("2026-09-15"));
    }

    #[test]
    fn order_index_is_assigned_incrementally() {
        let conn = test_conn();
        let repo = SqliteDailyPlanRepo;
        let plan = repo.get_or_create(&conn, "w1", "2026-09-10").expect("plan");
        let a = create_task(&conn, "A");
        let b = create_task(&conn, "B");
        let row_a = select_task(&conn, &plan.id, &a, SelectionReason::Manual, None, None)
            .expect("select a");
        let row_b = select_task(&conn, &plan.id, &b, SelectionReason::Manual, None, None)
            .expect("select b");
        assert!(row_a.order_index < row_b.order_index);
    }

    #[test]
    fn reorder_requires_exact_task_set() {
        let conn = test_conn();
        let repo = SqliteDailyPlanRepo;
        let plan = repo.get_or_create(&conn, "w1", "2026-09-10").expect("plan");
        let a = create_task(&conn, "A");
        let b = create_task(&conn, "B");
        let row_a = select_task(&conn, &plan.id, &a, SelectionReason::Manual, None, None)
            .expect("select a");
        let row_b = select_task(&conn, &plan.id, &b, SelectionReason::Manual, None, None)
            .expect("select b");

        reorder_tasks(&conn, &plan.id, &[row_b.id.clone(), row_a.id.clone()]).expect("reorder");

        let rows = list_plan_tasks(&conn, &plan.id).expect("list");
        assert_eq!(rows[0].id, row_b.id);
        assert_eq!(rows[1].id, row_a.id);

        assert!(
            reorder_tasks(&conn, &plan.id, std::slice::from_ref(&row_a.id)).is_err(),
            "missing a task is rejected"
        );
    }

    #[test]
    fn deselect_removes_selection() {
        let conn = test_conn();
        let repo = SqliteDailyPlanRepo;
        let plan = repo.get_or_create(&conn, "w1", "2026-09-10").expect("plan");
        let task = create_task(&conn, "Task");
        let row = select_task(&conn, &plan.id, &task, SelectionReason::Manual, None, None)
            .expect("select");
        deselect_task(&conn, &row.id).expect("deselect");
        assert!(list_plan_tasks(&conn, &plan.id).expect("list").is_empty());
    }

    #[test]
    fn list_between_returns_date_range() {
        let conn = test_conn();
        let repo = SqliteDailyPlanRepo;
        repo.get_or_create(&conn, "w1", "2026-09-09").expect("d1");
        repo.get_or_create(&conn, "w1", "2026-09-10").expect("d2");
        repo.get_or_create(&conn, "w1", "2026-09-11").expect("d3");
        let plans = repo
            .list_between(&conn, "w1", "2026-09-10", "2026-09-11")
            .expect("range");
        assert_eq!(plans.len(), 2);
        assert_eq!(plans[0].date, "2026-09-10");
        assert_eq!(plans[1].date, "2026-09-11");
    }
}
