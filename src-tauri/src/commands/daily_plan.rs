use crate::commands::dto::{
    ReplanTaskDto, SelectTaskForDayDto, UpdateDailyPlanDto, UpdateDailyPlanReviewDto,
};
use crate::domain::capacity::CapacitySuggestion;
use crate::domain::daily_plan;
use crate::domain::models::{DailyPlan, DailyPlanTask};
use crate::domain::types::{DailyPlanStatus, ReplanDecision, SelectionReason};
use crate::error::AppError;
use crate::repos::daily_plan_repo::{
    deselect_task, list_plan_tasks, reorder_tasks, select_task, DailyPlanRepo, SqliteDailyPlanRepo,
};
use crate::repos::workspace_repo::{SqliteWorkspaceRepo, WorkspaceRepo};
use crate::state::Db;
use tauri::State;

/// Return the daily plan for a date, creating it when it does not exist yet.
#[tauri::command]
pub fn get_or_create_daily_plan(db: State<'_, Db>, date: String) -> Result<DailyPlan, AppError> {
    crate::log_command!(
        "get_or_create_daily_plan",
        db.with_conn(|conn| {
            let workspace = SqliteWorkspaceRepo.ensure_default(conn)?;
            SqliteDailyPlanRepo.get_or_create(conn, &workspace.id, &date)
        })
    )
}

/// Return the daily plan for a date without creating it.
#[tauri::command]
pub fn get_daily_plan_by_date(
    db: State<'_, Db>,
    workspace_id: String,
    date: String,
) -> Result<Option<DailyPlan>, AppError> {
    db.with_conn(|conn| SqliteDailyPlanRepo.get_by_date(conn, &workspace_id, &date))
}

/// Daily plans across a date range, for the calendar view.
#[tauri::command]
pub fn list_daily_plans_between(
    db: State<'_, Db>,
    workspace_id: String,
    from_date: String,
    to_date: String,
) -> Result<Vec<DailyPlan>, AppError> {
    db.with_conn(|conn| SqliteDailyPlanRepo.list_between(conn, &workspace_id, &from_date, &to_date))
}

/// Update a daily plan's status (`not_started`, `planning`, `planned`,
/// `in_review`, `closed`).
#[tauri::command]
pub fn update_daily_plan_status(
    db: State<'_, Db>,
    daily_plan_id: String,
    status: String,
) -> Result<DailyPlan, AppError> {
    let status: DailyPlanStatus = status.parse()?;
    crate::log_command!(
        "update_daily_plan_status",
        db.with_conn(|conn| SqliteDailyPlanRepo.update_status(conn, &daily_plan_id, status))
    )
}

/// Save the intention and capacity notes of a plan.
#[tauri::command]
pub fn update_daily_plan_intention(
    db: State<'_, Db>,
    daily_plan_id: String,
    dto: UpdateDailyPlanDto,
) -> Result<DailyPlan, AppError> {
    db.with_conn(|conn| {
        SqliteDailyPlanRepo.set_intention(
            conn,
            &daily_plan_id,
            &dto.intention_markdown,
            &dto.capacity_note_markdown,
        )
    })
}

/// Save the end-of-day review note of a plan.
#[tauri::command]
pub fn update_daily_plan_review(
    db: State<'_, Db>,
    daily_plan_id: String,
    dto: UpdateDailyPlanReviewDto,
) -> Result<DailyPlan, AppError> {
    db.with_conn(|conn| SqliteDailyPlanRepo.set_review(conn, &daily_plan_id, &dto.review_markdown))
}

/// Record which planning block was used for a plan.
#[tauri::command]
pub fn set_daily_plan_planning_block(
    db: State<'_, Db>,
    daily_plan_id: String,
    block_id: Option<String>,
) -> Result<DailyPlan, AppError> {
    db.with_conn(|conn| {
        SqliteDailyPlanRepo.set_planning_block(conn, &daily_plan_id, block_id.as_deref())
    })
}

/// Select a task for a day without changing its deadline or scheduled date.
#[tauri::command]
pub fn select_task_for_day(
    db: State<'_, Db>,
    dto: SelectTaskForDayDto,
) -> Result<DailyPlanTask, AppError> {
    let reason: SelectionReason = dto.reason.parse()?;
    crate::log_command!(
        "select_task_for_day",
        db.with_conn(|conn| {
            select_task(conn, &dto.daily_plan_id, &dto.task_id, reason, None, None)
        })
    )
}

/// Remove a task from a day. Its sessions keep their block reference.
#[tauri::command]
pub fn deselect_task_for_day(
    db: State<'_, Db>,
    daily_plan_task_id: String,
) -> Result<(), AppError> {
    crate::log_command!(
        "deselect_task_for_day",
        db.with_conn(|conn| deselect_task(conn, &daily_plan_task_id))
    )
}

/// Reorder the selected tasks of a day.
#[tauri::command]
pub fn reorder_day_tasks(
    db: State<'_, Db>,
    daily_plan_id: String,
    ordered_ids: Vec<String>,
) -> Result<(), AppError> {
    crate::log_command!(
        "reorder_day_tasks",
        db.with_conn(|conn| reorder_tasks(conn, &daily_plan_id, &ordered_ids))
    )
}

/// The tasks selected for a day, ordered.
#[tauri::command]
pub fn list_day_tasks(
    db: State<'_, Db>,
    daily_plan_id: String,
) -> Result<Vec<DailyPlanTask>, AppError> {
    db.with_conn(|conn| list_plan_tasks(conn, &daily_plan_id))
}

/// Apply an explicit replanning decision to a missed or changed selection.
#[tauri::command]
pub fn replan_task(
    db: State<'_, Db>,
    daily_plan_task_id: String,
    dto: ReplanTaskDto,
) -> Result<DailyPlanTask, AppError> {
    let decision: ReplanDecision = dto.decision.parse()?;
    crate::log_command!(
        "replan_task",
        db.with_tx(|tx| daily_plan::replan_task(
            tx,
            &daily_plan_task_id,
            decision,
            &dto.note_markdown
        ))
    )
}

/// Suggested focus window for a date, derived from availability and commitments.
#[tauri::command]
pub fn get_capacity_suggestion(
    db: State<'_, Db>,
    workspace_id: String,
    date: String,
) -> Result<Option<CapacitySuggestion>, AppError> {
    db.with_conn(|conn| crate::domain::capacity::suggest_focus_window(conn, &workspace_id, &date))
}
