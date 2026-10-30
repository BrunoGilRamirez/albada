use crate::commands::dto::{
    CreateBlockDto, CreateBlockRecurrenceDto, ReactivateBlockDto, ReactivateBlockResult,
    RecurrenceAnalyticsResponse, UpdateBlockDto, UpdateBlockRecurrenceDto,
};
use crate::domain::models::{Block, BlockRecurrence, ScheduledBlockRecurrence};
use crate::domain::types::BlockLifecycleState;
use crate::error::AppError;
use crate::repos::block_recurrence_analytics_repo;
use crate::repos::block_recurrence_repo::{self, RecurrenceRule};
use crate::repos::block_repo::{
    cancel_block, complete_block, deactivate_block, delete_block, list_inactive_blocks,
    list_upcoming_blocks, reactivate_block, skip_block, start_block, BlockRepo, SqliteBlockRepo,
};
use crate::state::Db;
use tauri::State;

/// Create a block. Exact times are optional; a block is a flexible intention
/// space.
#[tauri::command]
pub fn create_block(db: State<'_, Db>, dto: CreateBlockDto) -> Result<Block, AppError> {
    crate::log_command!(
        "create_block",
        db.with_tx(|conn| {
            SqliteBlockRepo.create(
                conn,
                &dto.workspace_id,
                &dto.daily_plan_id,
                &dto.date,
                &dto.block_type,
                &dto.label,
                &dto.planned_start_at,
                &dto.planned_end_at,
                dto.estimated_duration_seconds,
                &dto.time_window_label,
                &dto.flexibility,
                &dto.note_markdown,
                &dto.task_id,
            )
        })
    )
}

#[tauri::command]
pub fn update_block(db: State<'_, Db>, id: String, dto: UpdateBlockDto) -> Result<Block, AppError> {
    crate::log_command!(
        "update_block",
        db.with_tx(|conn| {
            SqliteBlockRepo.update(
                conn,
                &id,
                &dto.daily_plan_id,
                &dto.date,
                &dto.block_type,
                &dto.label,
                &dto.planned_start_at,
                dto.estimated_duration_seconds,
                &dto.time_window_label,
                &dto.flexibility,
                &dto.note_markdown,
                &dto.task_id,
            )
        })
    )
}

#[tauri::command]
pub fn create_block_recurrence(
    db: State<'_, Db>,
    dto: CreateBlockRecurrenceDto,
) -> Result<crate::domain::models::BlockRecurrence, AppError> {
    crate::log_command!(
        "create_block_recurrence",
        db.with_tx(|conn| {
            block_recurrence_repo::create(
                conn,
                &dto.workspace_id,
                RecurrenceRule {
                    weekdays: dto.weekdays.as_deref(),
                    local_start_time: dto.local_start_time.as_deref(),
                    estimated_duration_seconds: dto.estimated_duration_seconds,
                    task_id: dto.task_id.as_deref(),
                    block_type: &dto.block_type,
                    label: &dto.label,
                    flexibility: &dto.flexibility,
                    note_markdown: &dto.note_markdown,
                    enabled: dto.enabled,
                    effective_from: &dto.effective_from,
                    effective_until: &dto.effective_until,
                },
            )
        })
    )
}

#[tauri::command]
pub fn update_block_recurrence(
    db: State<'_, Db>,
    recurrence_id: String,
    dto: UpdateBlockRecurrenceDto,
) -> Result<crate::domain::models::BlockRecurrence, AppError> {
    crate::log_command!(
        "update_block_recurrence",
        db.with_tx(|conn| {
            block_recurrence_repo::update(
                conn,
                &dto.workspace_id,
                &recurrence_id,
                RecurrenceRule {
                    weekdays: dto.weekdays.as_deref(),
                    local_start_time: dto.local_start_time.as_deref(),
                    estimated_duration_seconds: dto.estimated_duration_seconds,
                    task_id: dto.task_id.as_deref(),
                    block_type: &dto.block_type,
                    label: &dto.label,
                    flexibility: &dto.flexibility,
                    note_markdown: &dto.note_markdown,
                    enabled: dto.enabled,
                    effective_from: &dto.effective_from,
                    effective_until: &dto.effective_until,
                },
            )
        })
    )
}

#[tauri::command]
pub fn get_block_recurrence(
    db: State<'_, Db>,
    workspace_id: String,
    recurrence_id: String,
) -> Result<crate::domain::models::BlockRecurrence, AppError> {
    db.with_conn(|conn| block_recurrence_repo::get(conn, &workspace_id, &recurrence_id))
}

#[tauri::command]
pub fn get_block(db: State<'_, Db>, id: String) -> Result<Block, AppError> {
    db.with_conn(|conn| SqliteBlockRepo.get(conn, &id))
}

#[tauri::command]
pub fn start_block_cmd(db: State<'_, Db>, id: String) -> Result<Block, AppError> {
    crate::log_command!("start_block", db.with_conn(|conn| start_block(conn, &id)))
}

#[tauri::command]
pub fn complete_block_cmd(db: State<'_, Db>, id: String) -> Result<Block, AppError> {
    crate::log_command!(
        "complete_block",
        db.with_conn(|conn| complete_block(conn, &id))
    )
}

#[tauri::command]
pub fn skip_block_cmd(db: State<'_, Db>, id: String) -> Result<Block, AppError> {
    crate::log_command!("skip_block", db.with_conn(|conn| skip_block(conn, &id)))
}

#[tauri::command]
pub fn cancel_block_cmd(db: State<'_, Db>, id: String) -> Result<Block, AppError> {
    crate::log_command!("cancel_block", db.with_conn(|conn| cancel_block(conn, &id)))
}

#[tauri::command]
pub fn list_blocks_for_date(
    db: State<'_, Db>,
    workspace_id: String,
    date: String,
) -> Result<Vec<Block>, AppError> {
    db.with_tx(|conn| SqliteBlockRepo.list_for_date(conn, &workspace_id, &date))
}

#[tauri::command]
pub fn list_blocks_for_plan(
    db: State<'_, Db>,
    daily_plan_id: String,
) -> Result<Vec<Block>, AppError> {
    db.with_conn(|conn| SqliteBlockRepo.list_for_plan(conn, &daily_plan_id))
}

#[tauri::command]
pub fn list_blocks_between(
    db: State<'_, Db>,
    workspace_id: String,
    from_date: String,
    to_date: String,
) -> Result<Vec<Block>, AppError> {
    db.with_conn(|conn| SqliteBlockRepo.list_between(conn, &workspace_id, &from_date, &to_date))
}

/// Move a one-off block out of the active schedule without deleting history.
#[tauri::command]
pub fn deactivate_block_cmd(
    db: State<'_, Db>,
    workspace_id: String,
    id: String,
) -> Result<Block, AppError> {
    crate::log_command!(
        "deactivate_block",
        db.with_tx(|conn| deactivate_block(conn, &workspace_id, &id))
    )
}

/// Reactivate an inactive one-off block as a new one-time or weekly occurrence.
#[tauri::command]
pub fn reactivate_block_cmd(
    db: State<'_, Db>,
    dto: ReactivateBlockDto,
) -> Result<ReactivateBlockResult, AppError> {
    crate::log_command!(
        "reactivate_block",
        db.with_tx(|conn| reactivate_block(
            conn,
            &dto.workspace_id,
            &dto.block_id,
            &dto.schedule,
            dto.weekday,
            &dto.local_start_time,
            dto.estimated_duration_seconds,
        ))
    )
}

/// Soft-delete a one-off block. History and references are retained.
#[tauri::command]
pub fn delete_block_cmd(
    db: State<'_, Db>,
    workspace_id: String,
    id: String,
) -> Result<Block, AppError> {
    crate::log_command!(
        "delete_block",
        db.with_tx(|conn| delete_block(conn, &workspace_id, &id))
    )
}

/// Deactivate a recurrence series and withdraw its future occurrences.
#[tauri::command]
pub fn deactivate_block_recurrence_cmd(
    db: State<'_, Db>,
    workspace_id: String,
    recurrence_id: String,
) -> Result<BlockRecurrence, AppError> {
    crate::log_command!(
        "deactivate_block_recurrence",
        db.with_tx(|conn| block_recurrence_repo::deactivate_series(
            conn,
            &workspace_id,
            &recurrence_id
        ))
    )
}

/// Reactivate a recurrence series without rewriting occurrence history.
#[tauri::command]
pub fn reactivate_block_recurrence_cmd(
    db: State<'_, Db>,
    workspace_id: String,
    recurrence_id: String,
) -> Result<BlockRecurrence, AppError> {
    crate::log_command!(
        "reactivate_block_recurrence",
        db.with_tx(|conn| block_recurrence_repo::reactivate_series(
            conn,
            &workspace_id,
            &recurrence_id
        ))
    )
}

/// Soft-delete a recurrence series and mark its occurrences deleted.
#[tauri::command]
pub fn delete_block_recurrence_cmd(
    db: State<'_, Db>,
    workspace_id: String,
    recurrence_id: String,
) -> Result<BlockRecurrence, AppError> {
    crate::log_command!(
        "delete_block_recurrence",
        db.with_tx(|conn| block_recurrence_repo::delete_series(
            conn,
            &workspace_id,
            &recurrence_id
        ))
    )
}

/// Inactive one-off blocks retained for the collapsed inactive collection.
#[tauri::command]
pub fn list_inactive_blocks_cmd(
    db: State<'_, Db>,
    workspace_id: String,
) -> Result<Vec<Block>, AppError> {
    db.with_conn(|conn| list_inactive_blocks(conn, &workspace_id))
}

/// Inactive recurrence series retained for the inactive collection.
#[tauri::command]
pub fn list_inactive_block_recurrences_cmd(
    db: State<'_, Db>,
    workspace_id: String,
) -> Result<Vec<BlockRecurrence>, AppError> {
    db.with_conn(|conn| {
        block_recurrence_repo::list_series(conn, &workspace_id, BlockLifecycleState::Inactive)
    })
}

/// Active recurrence definitions shown once in the scheduled-blocks section.
#[tauri::command]
pub fn list_scheduled_block_recurrences_cmd(
    db: State<'_, Db>,
    workspace_id: String,
) -> Result<Vec<ScheduledBlockRecurrence>, AppError> {
    db.with_conn(|conn| block_recurrence_repo::list_scheduled_series(conn, &workspace_id))
}

/// Active upcoming planned occurrences materialized only for the requested
/// bounded date range.
#[tauri::command]
pub fn list_upcoming_blocks_cmd(
    db: State<'_, Db>,
    workspace_id: String,
    from_date: String,
    to_date: String,
    limit: i64,
    offset: i64,
) -> Result<Vec<Block>, AppError> {
    db.with_tx(|conn| {
        list_upcoming_blocks(conn, &workspace_id, &from_date, &to_date, limit, offset)
    })
}

/// Workspace-scoped, read-only habit analytics for recurring block series over
/// a validated local-date range whose end is clamped to the workspace-local
/// today. The aggregation creates no occurrence, changes no block, recurrence,
/// task, notification, or task progress, and is bounded to the effective range
/// and the requested workspace.
#[tauri::command]
pub fn get_block_recurrence_analytics_cmd(
    db: State<'_, Db>,
    workspace_id: String,
    from_date: String,
    to_date: String,
) -> Result<RecurrenceAnalyticsResponse, AppError> {
    crate::log_command!(
        "get_block_recurrence_analytics",
        db.with_conn(
            |conn| block_recurrence_analytics_repo::analytics_for_workspace(
                conn,
                &workspace_id,
                &from_date,
                &to_date
            )
        )
    )
}
