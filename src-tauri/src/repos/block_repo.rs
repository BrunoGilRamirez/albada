use chrono::{DateTime, Utc};
use rusqlite::{params, Connection, OptionalExtension};

use crate::commands::dto::ReactivateBlockResult;
use crate::domain::models::Block;
use crate::domain::types::{BlockLifecycleState, BlockType, Flexibility};
use crate::error::AppError;
use crate::id;
use crate::time;

pub trait BlockRepo {
    fn create(
        &self,
        conn: &Connection,
        workspace_id: &str,
        daily_plan_id: &Option<String>,
        date: &str,
        block_type: &str,
        label: &Option<String>,
        planned_start_at: &Option<String>,
        legacy_planned_end_at: &Option<String>,
        estimated_duration_seconds: Option<i64>,
        time_window_label: &Option<String>,
        flexibility: &str,
        note_markdown: &Option<String>,
        task_id: &Option<String>,
    ) -> Result<Block, AppError>;
    fn update(
        &self,
        conn: &Connection,
        id: &str,
        daily_plan_id: &Option<String>,
        date: &str,
        block_type: &str,
        label: &Option<String>,
        planned_start_at: &Option<String>,
        estimated_duration_seconds: Option<i64>,
        time_window_label: &Option<String>,
        flexibility: &str,
        note_markdown: &Option<String>,
        task_id: &Option<String>,
    ) -> Result<Block, AppError>;
    fn get(&self, conn: &Connection, id: &str) -> Result<Block, AppError>;
    fn list_for_date(
        &self,
        conn: &Connection,
        workspace_id: &str,
        date: &str,
    ) -> Result<Vec<Block>, AppError>;
    fn list_for_plan(&self, conn: &Connection, daily_plan_id: &str)
        -> Result<Vec<Block>, AppError>;
    fn list_between(
        &self,
        conn: &Connection,
        workspace_id: &str,
        from_date: &str,
        to_date: &str,
    ) -> Result<Vec<Block>, AppError>;
}

const BLOCK_COLS: &str = "id, workspace_id, daily_plan_id, date, type, status, label, planned_start_at, planned_end_at, estimated_duration_seconds, recurrence_id, recurrence_version_id, local_date, local_start_time, timezone_id, resolved_start_at, time_window_label, actual_start_at, actual_end_at, flexibility, note_markdown, created_at, updated_at, lifecycle_state, task_id, reactivated_from_block_id";

fn row_to_block(row: &rusqlite::Row<'_>) -> Result<Block, rusqlite::Error> {
    let block_type: String = row.get(4)?;
    let status: String = row.get(5)?;
    let flexibility: String = row.get(19)?;
    let lifecycle_state: String = row.get(23)?;
    Ok(Block {
        id: row.get(0)?,
        workspace_id: row.get(1)?,
        daily_plan_id: row.get(2)?,
        date: row.get(3)?,
        block_type: block_type.parse().map_err(|_| invalid_column(4, "type"))?,
        status: status.parse().map_err(|_| invalid_column(5, "status"))?,
        label: row.get(6)?,
        planned_start_at: row.get(7)?,
        planned_end_at: row.get(8)?,
        estimated_duration_seconds: row.get(9)?,
        recurrence_id: row.get(10)?,
        recurrence_version_id: row.get(11)?,
        local_date: row.get(12)?,
        local_start_time: row.get(13)?,
        timezone_id: row.get(14)?,
        resolved_start_at: row.get(15)?,
        time_window_label: row.get(16)?,
        actual_start_at: row.get(17)?,
        actual_end_at: row.get(18)?,
        flexibility: flexibility
            .parse()
            .map_err(|_| invalid_column(19, "flexibility"))?,
        note_markdown: row.get(20)?,
        created_at: row.get(21)?,
        updated_at: row.get(22)?,
        lifecycle_state: lifecycle_state
            .parse()
            .map_err(|_| invalid_column(23, "lifecycle_state"))?,
        task_id: row.get(24)?,
        reactivated_from_block_id: row.get(25)?,
    })
}

fn invalid_column(index: usize, name: &str) -> rusqlite::Error {
    rusqlite::Error::InvalidColumnType(index, name.to_string(), rusqlite::types::Type::Text)
}

pub struct SqliteBlockRepo;

impl BlockRepo for SqliteBlockRepo {
    fn create(
        &self,
        conn: &Connection,
        workspace_id: &str,
        daily_plan_id: &Option<String>,
        date: &str,
        block_type: &str,
        label: &Option<String>,
        planned_start_at: &Option<String>,
        legacy_planned_end_at: &Option<String>,
        estimated_duration_seconds: Option<i64>,
        time_window_label: &Option<String>,
        flexibility: &str,
        note_markdown: &Option<String>,
        task_id: &Option<String>,
    ) -> Result<Block, AppError> {
        let block_type = block_type.parse::<BlockType>()?;
        let flexibility = flexibility.parse::<Flexibility>()?;
        if legacy_planned_end_at.is_some() {
            return Err(AppError::validation(
                "planned_end_at is no longer editable; provide estimated_duration_seconds instead.",
            ));
        }
        time::parse_date(date)?;
        crate::domain::validation::validate_optional_block_start(planned_start_at)?;
        crate::domain::validation::validate_block_duration(estimated_duration_seconds)?;
        if let Some(task_id) = task_id {
            validate_task_reference(conn, workspace_id, task_id)?;
        }
        let id = id::new_uuid();
        let now = time::now_rfc3339();
        conn.execute(
            "INSERT INTO blocks (id, workspace_id, daily_plan_id, date, type, status, label,
                planned_start_at, estimated_duration_seconds, time_window_label, flexibility,
                note_markdown, created_at, updated_at, task_id)
             VALUES (?1, ?2, ?3, ?4, ?5, 'planned', ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?12, ?13)",
            params![
                id,
                workspace_id,
                daily_plan_id,
                date,
                block_type.to_string(),
                label,
                planned_start_at,
                estimated_duration_seconds,
                time_window_label,
                flexibility.to_string(),
                note_markdown,
                now,
                task_id
            ],
        )?;
        self.get(conn, &id)
    }

    fn update(
        &self,
        conn: &Connection,
        id: &str,
        daily_plan_id: &Option<String>,
        date: &str,
        block_type: &str,
        label: &Option<String>,
        planned_start_at: &Option<String>,
        estimated_duration_seconds: Option<i64>,
        time_window_label: &Option<String>,
        flexibility: &str,
        note_markdown: &Option<String>,
        task_id: &Option<String>,
    ) -> Result<Block, AppError> {
        let block_type = block_type.parse::<BlockType>()?;
        let flexibility = flexibility.parse::<Flexibility>()?;
        time::parse_date(date)?;
        crate::domain::validation::validate_optional_block_start(planned_start_at)?;
        crate::domain::validation::validate_block_duration(estimated_duration_seconds)?;
        validate_optional_task_reference_for_block(conn, id, task_id)?;
        let is_recurrence: bool = conn.query_row(
            "SELECT recurrence_id IS NOT NULL FROM blocks WHERE id = ?1",
            [id],
            |row| row.get(0),
        )?;
        if is_recurrence {
            return Err(AppError::validation(
                "Recurring occurrences must be changed through their recurrence.",
            ));
        }
        let now = time::now_rfc3339();
        terminate_pending_events_after(
            conn,
            |tx| {
                tx.execute(
                    "UPDATE blocks SET daily_plan_id = ?2, date = ?3, type = ?4, label = ?5,
                        planned_start_at = ?6, estimated_duration_seconds = ?7,
                        time_window_label = ?8, flexibility = ?9, note_markdown = ?10,
                        task_id = ?11, updated_at = ?12 WHERE id = ?1",
                    params![
                        id,
                        daily_plan_id,
                        date,
                        block_type.to_string(),
                        label,
                        planned_start_at,
                        estimated_duration_seconds,
                        time_window_label,
                        flexibility.to_string(),
                        note_markdown,
                        task_id,
                        now
                    ],
                )?;
                Ok(())
            },
            id,
            "The block changed before this notification was due.",
        )?;
        self.get(conn, id)
    }

    fn get(&self, conn: &Connection, id: &str) -> Result<Block, AppError> {
        conn.query_row(
            &format!("SELECT {BLOCK_COLS} FROM blocks WHERE id = ?1"),
            [id],
            row_to_block,
        )
        .optional()?
        .ok_or_else(|| AppError::not_found("The block was not found."))
    }

    fn list_for_date(
        &self,
        conn: &Connection,
        workspace_id: &str,
        date: &str,
    ) -> Result<Vec<Block>, AppError> {
        crate::repos::block_recurrence_repo::materialize_for_date(conn, workspace_id, date)?;
        let mut stmt = conn.prepare(&format!(
            "SELECT {BLOCK_COLS} FROM blocks
             WHERE workspace_id = ?1 AND date = ?2 AND lifecycle_state != 'deleted'
               AND NOT EXISTS (
                   SELECT 1 FROM block_recurrences r
                   WHERE r.id = blocks.recurrence_id AND r.lifecycle_state = 'deleted'
               )
             ORDER BY planned_start_at IS NULL, planned_start_at, created_at, id"
        ))?;
        let rows = stmt.query_map(params![workspace_id, date], row_to_block)?;
        let mut result = Vec::new();
        for row in rows {
            result.push(row?);
        }
        Ok(result)
    }

    fn list_for_plan(
        &self,
        conn: &Connection,
        daily_plan_id: &str,
    ) -> Result<Vec<Block>, AppError> {
        let mut stmt = conn.prepare(&format!(
            "SELECT {BLOCK_COLS} FROM blocks
             WHERE daily_plan_id = ?1 AND lifecycle_state != 'deleted'
               AND NOT EXISTS (
                   SELECT 1 FROM block_recurrences r
                   WHERE r.id = blocks.recurrence_id AND r.lifecycle_state = 'deleted'
               )
             ORDER BY planned_start_at IS NULL, planned_start_at, created_at"
        ))?;
        let rows = stmt.query_map([daily_plan_id], row_to_block)?;
        let mut result = Vec::new();
        for row in rows {
            result.push(row?);
        }
        Ok(result)
    }

    fn list_between(
        &self,
        conn: &Connection,
        workspace_id: &str,
        from_date: &str,
        to_date: &str,
    ) -> Result<Vec<Block>, AppError> {
        crate::time::parse_date(from_date)?;
        crate::time::parse_date(to_date)?;
        let mut stmt = conn.prepare(&format!(
            "SELECT {BLOCK_COLS} FROM blocks
             WHERE workspace_id = ?1 AND date BETWEEN ?2 AND ?3
               AND lifecycle_state != 'deleted'
               AND NOT EXISTS (
                   SELECT 1 FROM block_recurrences r
                   WHERE r.id = blocks.recurrence_id AND r.lifecycle_state = 'deleted'
               )
             ORDER BY date, planned_start_at IS NULL, planned_start_at, created_at"
        ))?;
        let rows = stmt.query_map(params![workspace_id, from_date, to_date], row_to_block)?;
        let mut result = Vec::new();
        for row in rows {
            result.push(row?);
        }
        Ok(result)
    }
}

/// Validate a block is in a state that can leave `planned`.
fn ensure_not_finished(conn: &Connection, id: &str) -> Result<(), AppError> {
    let status: String =
        conn.query_row("SELECT status FROM blocks WHERE id = ?1", [id], |row| {
            row.get(0)
        })?;
    match status.as_str() {
        "completed" | "cancelled" => Err(AppError::validation(
            "The block is already completed or cancelled.",
        )),
        _ => Ok(()),
    }
}

/// Validate that an optional task reference exists in the given workspace.
///
/// A block/series may only reference a task owned by the same workspace; the
/// association is context only and never changes task state.
pub(crate) fn validate_task_reference(
    conn: &Connection,
    workspace_id: &str,
    task_id: &str,
) -> Result<(), AppError> {
    let task_workspace: Option<String> = conn
        .query_row(
            "SELECT workspace_id FROM tasks WHERE id = ?1",
            [task_id],
            |row| row.get(0),
        )
        .optional()?;
    if task_workspace.as_deref() != Some(workspace_id) {
        return Err(AppError::validation(
            "The referenced task was not found in this workspace.",
        ));
    }
    Ok(())
}

/// Validate a task reference supplied while updating an existing block.
fn validate_optional_task_reference_for_block(
    conn: &Connection,
    block_id: &str,
    task_id: &Option<String>,
) -> Result<(), AppError> {
    let Some(task_id) = task_id else {
        return Ok(());
    };
    let workspace_id: Option<String> = conn
        .query_row(
            "SELECT workspace_id FROM blocks WHERE id = ?1",
            [block_id],
            |row| row.get(0),
        )
        .optional()?;
    let workspace_id =
        workspace_id.ok_or_else(|| AppError::not_found("The block was not found."))?;
    validate_task_reference(conn, &workspace_id, task_id)
}

/// Keep a task reference only while the task still exists in the workspace.
/// Reactivation copies context but never fails because the task is gone.
fn existing_task_reference(
    conn: &Connection,
    workspace_id: &str,
    task_id: &Option<String>,
) -> Result<Option<String>, AppError> {
    let Some(task_id) = task_id else {
        return Ok(None);
    };
    Ok(validate_task_reference(conn, workspace_id, task_id)
        .ok()
        .map(|()| task_id.clone()))
}

/// Apply a block status/field change and terminate the block's still-pending
/// event deliveries atomically. When the caller already runs inside a
/// transaction both statements join it; otherwise a short-lived transaction
/// keeps the pair from being partially applied.
fn terminate_pending_events_after<F>(
    conn: &Connection,
    apply: F,
    block_id: &str,
    reason: &str,
) -> Result<(), AppError>
where
    F: FnOnce(&Connection) -> Result<(), AppError>,
{
    if conn.is_autocommit() {
        let tx = conn.unchecked_transaction()?;
        apply(&tx)?;
        crate::repos::notification_repo::fail_pending_block_event_deliveries(
            &tx, block_id, reason,
        )?;
        tx.commit()?;
    } else {
        apply(conn)?;
        crate::repos::notification_repo::fail_pending_block_event_deliveries(
            conn, block_id, reason,
        )?;
    }
    Ok(())
}

/// Start a block: moves it to `active` and stamps `actual_start_at`.
pub fn start_block(conn: &Connection, id: &str) -> Result<Block, AppError> {
    let status: String =
        conn.query_row("SELECT status FROM blocks WHERE id = ?1", [id], |row| {
            row.get(0)
        })?;
    if status == "active" {
        return Err(AppError::validation("The block is already active."));
    }
    ensure_not_finished(conn, id)?;
    let now = time::now_rfc3339();
    terminate_pending_events_after(
        conn,
        |tx| {
            tx.execute(
                "UPDATE blocks SET status = 'active', actual_start_at = ?2, updated_at = ?2 WHERE id = ?1",
                params![id, now],
            )?;
            Ok(())
        },
        id,
        "The block started before its reminder was delivered.",
    )?;
    SqliteBlockRepo.get(conn, id)
}

/// Complete a block: moves it to `completed` and stamps `actual_end_at`.
pub fn complete_block(conn: &Connection, id: &str) -> Result<Block, AppError> {
    let status: String =
        conn.query_row("SELECT status FROM blocks WHERE id = ?1", [id], |row| {
            row.get(0)
        })?;
    if status != "active" {
        return Err(AppError::validation(
            "Only an active block can be completed. Start it first.",
        ));
    }
    let now = time::now_rfc3339();
    terminate_pending_events_after(
        conn,
        |tx| {
            tx.execute(
                "UPDATE blocks SET status = 'completed', lifecycle_state = 'inactive',
                    actual_end_at = ?2, updated_at = ?2 WHERE id = ?1",
                params![id, now],
            )?;
            Ok(())
        },
        id,
        "The block ended before this notification was due.",
    )?;
    SqliteBlockRepo.get(conn, id)
}

/// Skip a block without performing it.
pub fn skip_block(conn: &Connection, id: &str) -> Result<Block, AppError> {
    ensure_not_finished(conn, id)?;
    let now = time::now_rfc3339();
    terminate_pending_events_after(
        conn,
        |tx| {
            tx.execute(
                "UPDATE blocks SET status = 'skipped', lifecycle_state = 'inactive',
                    updated_at = ?2 WHERE id = ?1",
                params![id, now],
            )?;
            Ok(())
        },
        id,
        "The block ended before this notification was due.",
    )?;
    SqliteBlockRepo.get(conn, id)
}

/// Cancel a block.
pub fn cancel_block(conn: &Connection, id: &str) -> Result<Block, AppError> {
    ensure_not_finished(conn, id)?;
    let now = time::now_rfc3339();
    terminate_pending_events_after(
        conn,
        |tx| {
            tx.execute(
                "UPDATE blocks SET status = 'cancelled', lifecycle_state = 'inactive',
                    updated_at = ?2 WHERE id = ?1",
                params![id, now],
            )?;
            Ok(())
        },
        id,
        "The block ended before this notification was due.",
    )?;
    SqliteBlockRepo.get(conn, id)
}

/// Load the lifecycle state, recurrence linkage, and work status of a block in
/// a workspace.
fn block_lifecycle_row(
    conn: &Connection,
    workspace_id: &str,
    id: &str,
) -> Result<(BlockLifecycleState, Option<String>, String), AppError> {
    let row = conn
        .query_row(
            "SELECT lifecycle_state, recurrence_id, status FROM blocks
             WHERE id = ?1 AND workspace_id = ?2",
            params![id, workspace_id],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, Option<String>>(1)?,
                    row.get::<_, String>(2)?,
                ))
            },
        )
        .optional()?
        .ok_or_else(|| AppError::not_found("The block was not found."))?;
    Ok((row.0.parse()?, row.1, row.2))
}

/// Manually deactivate a one-off block. Inactive blocks stay visible in the
/// inactive collection; recurring occurrences belong to their series.
pub fn deactivate_block(
    conn: &Connection,
    workspace_id: &str,
    id: &str,
) -> Result<Block, AppError> {
    let (lifecycle_state, recurrence_id, status) = block_lifecycle_row(conn, workspace_id, id)?;
    if recurrence_id.is_some() {
        return Err(AppError::validation(
            "Recurring occurrences are managed through their series.",
        ));
    }
    if status == "active" {
        return Err(AppError::validation(
            "A started block must be completed, skipped, or cancelled before deactivation.",
        ));
    }
    if lifecycle_state != BlockLifecycleState::Active {
        return Err(AppError::validation(
            "Only an active block can be deactivated.",
        ));
    }
    let now = time::now_rfc3339();
    terminate_pending_events_after(
        conn,
        |tx| {
            tx.execute(
                "UPDATE blocks SET lifecycle_state = 'inactive', updated_at = ?2 WHERE id = ?1",
                params![id, now],
            )?;
            Ok(())
        },
        id,
        "The block was deactivated before this notification was due.",
    )?;
    SqliteBlockRepo.get(conn, id)
}

/// Soft-delete a one-off block. History, task reference, and delivery rows are
/// retained; the row is only hidden from ordinary views.
pub fn delete_block(conn: &Connection, workspace_id: &str, id: &str) -> Result<Block, AppError> {
    let (lifecycle_state, recurrence_id, status) = block_lifecycle_row(conn, workspace_id, id)?;
    if recurrence_id.is_some() {
        return Err(AppError::validation(
            "Recurring occurrences are managed through their series.",
        ));
    }
    if status == "active" {
        return Err(AppError::validation(
            "A started block must be completed, skipped, or cancelled before deletion.",
        ));
    }
    if lifecycle_state == BlockLifecycleState::Deleted {
        return Err(AppError::validation("The block is already deleted."));
    }
    let now = time::now_rfc3339();
    terminate_pending_events_after(
        conn,
        |tx| {
            tx.execute(
                "UPDATE blocks SET lifecycle_state = 'deleted', updated_at = ?2 WHERE id = ?1",
                params![id, now],
            )?;
            Ok(())
        },
        id,
        "The block was deleted before this notification was due.",
    )?;
    SqliteBlockRepo.get(conn, id)
}

/// Current local calendar date in the workspace timezone.
fn workspace_local_today(conn: &Connection, workspace_id: &str) -> Result<String, AppError> {
    let timezone_id = crate::repos::block_recurrence_repo::workspace_timezone(conn, workspace_id)?;
    let timezone = crate::repos::block_recurrence_repo::parse_timezone(&timezone_id)?;
    Ok(Utc::now()
        .with_timezone(&timezone)
        .date_naive()
        .format("%Y-%m-%d")
        .to_string())
}

/// Reactivate an inactive one-off block by creating a new occurrence linked to
/// its source. The historical source row is never modified.
///
/// `schedule` is `"once"` (one new block on the next matching weekday) or
/// `"recurring"` (a new weekly series starting on that weekday).
pub fn reactivate_block(
    conn: &Connection,
    workspace_id: &str,
    block_id: &str,
    schedule: &str,
    weekday: i64,
    local_start_time: &Option<String>,
    estimated_duration_seconds: Option<i64>,
) -> Result<ReactivateBlockResult, AppError> {
    let (lifecycle_state, recurrence_id, _) = block_lifecycle_row(conn, workspace_id, block_id)?;
    if recurrence_id.is_some() {
        return Err(AppError::validation(
            "Recurring occurrences are managed through their series.",
        ));
    }
    match lifecycle_state {
        BlockLifecycleState::Active => {
            return Err(AppError::validation(
                "Only an inactive block can be reactivated.",
            ));
        }
        BlockLifecycleState::Deleted => {
            return Err(AppError::validation(
                "Deleted blocks cannot be reactivated.",
            ));
        }
        BlockLifecycleState::Inactive => {}
    }
    if schedule != "once" && schedule != "recurring" {
        return Err(AppError::validation(
            "The reactivation schedule must be either one-time or recurring.",
        ));
    }
    let source = SqliteBlockRepo.get(conn, block_id)?;
    let today = workspace_local_today(conn, workspace_id)?;
    let date = crate::repos::block_recurrence_repo::resolve_next_weekday_date(&today, weekday)?;
    let task_id = existing_task_reference(conn, workspace_id, &source.task_id)?;
    let start_time = local_start_time
        .clone()
        .or_else(|| source.planned_start_at.clone());
    let duration = estimated_duration_seconds.or(source.estimated_duration_seconds);
    let block_type = source.block_type.to_string();
    let flexibility = source.flexibility.to_string();

    if schedule == "once" {
        let block = SqliteBlockRepo.create(
            conn,
            workspace_id,
            &None,
            &date,
            &block_type,
            &source.label,
            &start_time,
            &None,
            duration,
            &source.time_window_label,
            &flexibility,
            &source.note_markdown,
            &task_id,
        )?;
        conn.execute(
            "UPDATE blocks SET reactivated_from_block_id = ?2 WHERE id = ?1",
            params![block.id, block_id],
        )?;
        return Ok(ReactivateBlockResult {
            block: Some(SqliteBlockRepo.get(conn, &block.id)?),
            recurrence: None,
        });
    }

    let local_start_time = start_time.ok_or_else(|| {
        AppError::validation("A recurring reactivation requires a local start time.")
    })?;
    let duration = duration.ok_or_else(|| {
        AppError::validation("A recurring reactivation requires a positive estimated duration.")
    })?;
    let weekdays = [weekday];
    let no_effective_until: Option<String> = None;
    let recurrence = crate::repos::block_recurrence_repo::create(
        conn,
        workspace_id,
        crate::repos::block_recurrence_repo::RecurrenceRule {
            weekdays: Some(&weekdays),
            local_start_time: Some(&local_start_time),
            estimated_duration_seconds: Some(duration),
            task_id: task_id.as_deref(),
            block_type: &block_type,
            label: &source.label,
            flexibility: &flexibility,
            note_markdown: &source.note_markdown,
            enabled: true,
            effective_from: &date,
            effective_until: &no_effective_until,
        },
    )?;
    Ok(ReactivateBlockResult {
        block: None,
        recurrence: Some(recurrence),
    })
}

/// Resolve a block's expiry start instant: the recurrence snapshot when
/// present, otherwise the one-off local `HH:MM` start resolved in `timezone`.
/// Missing, malformed, or DST-gap values yield `None` so the sweep skips that
/// row instead of failing.
fn resolve_expiry_start(
    date: &str,
    planned_start_at: &Option<String>,
    resolved_start_at: &Option<String>,
    timezone: chrono_tz::Tz,
) -> Option<DateTime<Utc>> {
    if let Some(resolved) = resolved_start_at {
        return DateTime::parse_from_rfc3339(resolved)
            .ok()
            .map(|instant| instant.with_timezone(&Utc));
    }
    let planned_start_at = planned_start_at.as_deref()?;
    let date = crate::time::parse_date(date).ok()?;
    let resolved =
        crate::repos::block_recurrence_repo::resolve_local_start(&date, planned_start_at, timezone)
            .ok()?;
    DateTime::parse_from_rfc3339(&resolved)
        .ok()
        .map(|instant| instant.with_timezone(&Utc))
}

/// Expire planned, timed blocks whose scheduled start plus estimated duration
/// has already passed without being started. Only block occurrences change;
/// recurrence series stay active and materialize their next weekday.
///
/// The sweep is idempotent and never touches completed, active, cancelled, or
/// inactive rows, nor rows without an estimated duration.
pub fn expire_elapsed_blocks(conn: &Connection, now: DateTime<Utc>) -> Result<usize, AppError> {
    let mut workspace_stmt = conn.prepare("SELECT id, timezone FROM workspaces")?;
    let workspaces = workspace_stmt
        .query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })?
        .collect::<Result<Vec<_>, _>>()?;
    drop(workspace_stmt);

    let now_text = now.to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
    let mut expired = 0usize;
    for (workspace_id, timezone_id) in workspaces {
        let timezone = crate::repos::block_recurrence_repo::parse_timezone(&timezone_id)?;
        let mut stmt = conn.prepare(
            "SELECT id, date, planned_start_at, resolved_start_at,
                estimated_duration_seconds, created_at, recurrence_id
             FROM blocks
             WHERE workspace_id = ?1 AND lifecycle_state = 'active'
               AND status = 'planned' AND estimated_duration_seconds IS NOT NULL",
        )?;
        let rows = stmt.query_map([&workspace_id], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, Option<String>>(2)?,
                row.get::<_, Option<String>>(3)?,
                row.get::<_, Option<i64>>(4)?,
                row.get::<_, String>(5)?,
                row.get::<_, Option<String>>(6)?,
            ))
        })?;
        let mut candidates = Vec::new();
        for row in rows {
            let (
                id,
                date,
                planned_start_at,
                resolved_start_at,
                duration,
                created_at,
                recurrence_id,
            ) = row?;
            let Some(duration) = duration else {
                continue;
            };
            let Some(start) =
                resolve_expiry_start(&date, &planned_start_at, &resolved_start_at, timezone)
            else {
                continue;
            };
            let due = start + chrono::Duration::seconds(duration);
            if recurrence_id.is_some()
                && DateTime::parse_from_rfc3339(&created_at)
                    .map_err(|_| {
                        AppError::validation("A recurring block has an invalid creation timestamp.")
                    })?
                    .with_timezone(&Utc)
                    > due
            {
                continue;
            }
            if now > due {
                candidates.push(id);
            }
        }
        drop(stmt);

        for id in candidates {
            let updated = conn.execute(
                "UPDATE blocks SET status = 'skipped', lifecycle_state = 'inactive',
                    updated_at = ?2
                 WHERE id = ?1 AND lifecycle_state = 'active' AND status = 'planned'",
                params![id, now_text],
            )?;
            if updated == 0 {
                continue;
            }
            crate::repos::notification_repo::fail_pending_block_event_deliveries(
                conn,
                &id,
                "The block expired before this notification was due.",
            )?;
            expired += 1;
        }
    }
    Ok(expired)
}

/// Inactive one-off blocks for the inactive collection, oldest first.
pub fn list_inactive_blocks(conn: &Connection, workspace_id: &str) -> Result<Vec<Block>, AppError> {
    let mut stmt = conn.prepare(&format!(
        "SELECT {BLOCK_COLS} FROM blocks
         WHERE workspace_id = ?1 AND lifecycle_state = 'inactive'
           AND recurrence_id IS NULL
         ORDER BY created_at, id"
    ))?;
    let rows = stmt.query_map([workspace_id], row_to_block)?;
    let mut result = Vec::new();
    for row in rows {
        result.push(row?);
    }
    Ok(result)
}

/// Upcoming active, planned one-time blocks for a bounded local date range.
/// Recurring rules are listed separately; this query never creates occurrence
/// rows as a side effect.
pub fn list_upcoming_blocks(
    conn: &Connection,
    workspace_id: &str,
    from_date: &str,
    to_date: &str,
    limit: i64,
    offset: i64,
) -> Result<Vec<Block>, AppError> {
    crate::time::parse_date(from_date)?;
    crate::time::parse_date(to_date)?;
    if !(1..=200).contains(&limit) {
        return Err(AppError::validation(
            "The upcoming block page limit must be between 1 and 200.",
        ));
    }
    if offset < 0 {
        return Err(AppError::validation(
            "The upcoming block page offset cannot be negative.",
        ));
    }
    let mut stmt = conn.prepare(&format!(
        "SELECT {BLOCK_COLS} FROM blocks
         WHERE workspace_id = ?1 AND lifecycle_state = 'active'
           AND recurrence_id IS NULL AND status = 'planned'
           AND date BETWEEN ?2 AND ?3
         ORDER BY date, planned_start_at IS NULL, planned_start_at, created_at, id
         LIMIT ?4 OFFSET ?5"
    ))?;
    let rows = stmt.query_map(
        params![workspace_id, from_date, to_date, limit, offset],
        row_to_block,
    )?;
    let mut result = Vec::new();
    for row in rows {
        result.push(row?);
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::types::BlockStatus;
    use crate::repos::block_recurrence_repo::{self, RecurrenceRule};
    use crate::repos::daily_plan_repo::{DailyPlanRepo, SqliteDailyPlanRepo};
    use crate::repos::notification_repo;
    use crate::test_utils::test_conn;

    static NO_TEXT: Option<String> = None;

    fn seed_workspace(conn: &Connection, id: &str) {
        conn.execute(
            "INSERT INTO workspaces (id, name, created_at, updated_at)
             VALUES (?1, 'Synthetic other workspace', '2026-01-01T00:00:00Z',
                     '2026-01-01T00:00:00Z')",
            [id],
        )
        .expect("seed synthetic workspace");
    }

    fn seed_task(conn: &Connection, workspace_id: &str, id: &str) {
        conn.execute(
            "INSERT INTO tasks (id, workspace_id, title, status, priority, created_at,
                updated_at)
             VALUES (?1, ?2, 'Synthetic task', 'planned', 'none', '2026-01-01T00:00:00Z',
                     '2026-01-01T00:00:00Z')",
            params![id, workspace_id],
        )
        .expect("seed synthetic task");
    }

    fn rule<'a>(weekdays: &'a [i64], start: &'a str, duration: i64) -> RecurrenceRule<'a> {
        RecurrenceRule {
            weekdays: Some(weekdays),
            local_start_time: Some(start),
            estimated_duration_seconds: Some(duration),
            task_id: None,
            block_type: "focus",
            label: &NO_TEXT,
            flexibility: "flexible",
            note_markdown: &NO_TEXT,
            enabled: true,
            effective_from: "2000-01-01",
            effective_until: &NO_TEXT,
        }
    }

    fn create_block(
        conn: &Connection,
        date: &str,
        block_type: &str,
        start: Option<&str>,
        duration: Option<i64>,
    ) -> Block {
        SqliteBlockRepo
            .create(
                conn,
                "w1",
                &None,
                date,
                block_type,
                &NO_TEXT,
                &start.map(str::to_string),
                &None,
                duration,
                &NO_TEXT,
                "flexible",
                &NO_TEXT,
                &NO_TEXT,
            )
            .expect("create synthetic block")
    }

    fn utc_instant(value: &str) -> DateTime<Utc> {
        DateTime::parse_from_rfc3339(value)
            .expect("valid test instant")
            .with_timezone(&Utc)
    }

    fn create_plan_block(conn: &Connection) -> (String, String) {
        let plan = SqliteDailyPlanRepo
            .get_or_create(conn, "w1", "2026-09-10")
            .expect("plan");
        let block = SqliteBlockRepo
            .create(
                conn,
                "w1",
                &Some(plan.id.clone()),
                "2026-09-10",
                "planning",
                &Some("Morning planning".to_string()),
                &None,
                &None,
                None,
                &Some("morning".to_string()),
                "flexible",
                &None,
                &None,
            )
            .expect("block");
        (plan.id, block.id)
    }

    #[test]
    fn block_can_exist_without_exact_times() {
        let conn = test_conn();
        let (plan_id, _) = create_plan_block(&conn);
        let blocks = SqliteBlockRepo
            .list_for_plan(&conn, &plan_id)
            .expect("list");
        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0].planned_start_at, None);
        assert_eq!(blocks[0].planned_end_at, None);
        assert_eq!(blocks[0].block_type, BlockType::Planning);
    }

    #[test]
    fn block_lifecycle_moves_planned_to_completed() {
        let conn = test_conn();
        let (_, block_id) = create_plan_block(&conn);
        assert_eq!(
            SqliteBlockRepo.get(&conn, &block_id).expect("get").status,
            BlockStatus::Planned
        );

        let started = start_block(&conn, &block_id).expect("start");
        assert_eq!(started.status, BlockStatus::Active);
        assert!(started.actual_start_at.is_some());

        let completed = complete_block(&conn, &block_id).expect("complete");
        assert_eq!(completed.status, BlockStatus::Completed);
        assert_eq!(
            completed.lifecycle_state,
            BlockLifecycleState::Inactive,
            "a completed one-off block leaves the active schedule"
        );
        assert!(completed.actual_end_at.is_some());
    }

    #[test]
    fn completing_planned_block_is_rejected() {
        let conn = test_conn();
        let (_, block_id) = create_plan_block(&conn);
        assert!(complete_block(&conn, &block_id).is_err());
    }

    #[test]
    fn skip_and_cancel_are_valid() {
        let conn = test_conn();
        let (_, block_id) = create_plan_block(&conn);
        let skipped = skip_block(&conn, &block_id).expect("skip");
        assert_eq!(skipped.status, BlockStatus::Skipped);
        assert_eq!(skipped.lifecycle_state, BlockLifecycleState::Inactive);

        let plan = SqliteDailyPlanRepo
            .get_or_create(&conn, "w1", "2026-09-11")
            .expect("plan");
        let other = SqliteBlockRepo
            .create(
                &conn,
                "w1",
                &Some(plan.id),
                "2026-09-11",
                "rest",
                &None,
                &None,
                &None,
                None,
                &None,
                "flexible",
                &None,
                &None,
            )
            .expect("block");
        let cancelled = cancel_block(&conn, &other.id).expect("cancel");
        assert_eq!(cancelled.status, BlockStatus::Cancelled);
        assert_eq!(cancelled.lifecycle_state, BlockLifecycleState::Inactive);
    }

    #[test]
    fn list_for_date_orders_blocks() {
        let conn = test_conn();
        SqliteBlockRepo
            .create(
                &conn,
                "w1",
                &None,
                "2026-09-10",
                "focus",
                &None,
                &Some("09:00".to_string()),
                &None,
                None,
                &None,
                "flexible",
                &None,
                &None,
            )
            .expect("first");
        SqliteBlockRepo
            .create(
                &conn,
                "w1",
                &None,
                "2026-09-10",
                "rest",
                &None,
                &Some("08:00".to_string()),
                &None,
                None,
                &None,
                "flexible",
                &None,
                &None,
            )
            .expect("second");
        let blocks = SqliteBlockRepo
            .list_for_date(&conn, "w1", "2026-09-10")
            .expect("list");
        assert_eq!(blocks.len(), 2);
        assert_eq!(blocks[0].block_type, BlockType::Rest);
    }

    #[test]
    fn deleting_a_series_hides_its_occurrences_but_preserves_the_rows() {
        let conn = test_conn();
        conn.execute("UPDATE workspaces SET timezone = 'UTC' WHERE id = 'w1'", [])
            .expect("set timezone");
        let recurrence = block_recurrence_repo::create(&conn, "w1", rule(&[1], "09:00", 1800))
            .expect("create recurrence");
        block_recurrence_repo::materialize_for_date(&conn, "w1", "2026-10-19")
            .expect("materialize occurrence");
        let occurrence = SqliteBlockRepo
            .list_for_date(&conn, "w1", "2026-10-19")
            .expect("list occurrence")
            .remove(0);
        let cancelled = cancel_block(&conn, &occurrence.id).expect("cancel occurrence");
        assert_eq!(cancelled.status, BlockStatus::Cancelled);

        block_recurrence_repo::delete_series(&conn, "w1", &recurrence.id)
            .expect("delete recurrence");

        assert!(SqliteBlockRepo
            .list_for_date(&conn, "w1", "2026-10-19")
            .expect("list date")
            .is_empty());
        assert!(SqliteBlockRepo
            .list_between(&conn, "w1", "2026-10-19", "2026-10-19")
            .expect("list date range")
            .is_empty());
        let preserved = SqliteBlockRepo
            .get(&conn, &occurrence.id)
            .expect("preserved occurrence");
        assert_eq!(preserved.status, BlockStatus::Cancelled);
        assert_eq!(preserved.lifecycle_state, BlockLifecycleState::Inactive);
    }

    #[test]
    fn updating_legacy_block_preserves_planned_end_and_status_history_fields() {
        let conn = test_conn();
        let block = SqliteBlockRepo
            .create(
                &conn,
                "w1",
                &None,
                "2026-09-10",
                "focus",
                &Some("Legacy".into()),
                &Some("09:00".into()),
                &None,
                None,
                &None,
                "flexible",
                &None,
                &None,
            )
            .expect("create block");
        conn.execute(
            "UPDATE blocks SET planned_end_at = '10:00', status = 'completed',
                actual_end_at = '2026-09-10T10:00:00Z' WHERE id = ?1",
            [&block.id],
        )
        .expect("seed legacy fields");

        let updated = SqliteBlockRepo
            .update(
                &conn,
                &block.id,
                &None,
                "2026-09-11",
                "review",
                &Some("Updated".into()),
                &Some("11:00".into()),
                Some(1800),
                &None,
                "approximate",
                &None,
                &None,
            )
            .expect("update block");

        assert_eq!(updated.planned_end_at.as_deref(), Some("10:00"));
        assert_eq!(updated.status, BlockStatus::Completed);
        assert_eq!(
            updated.actual_end_at.as_deref(),
            Some("2026-09-10T10:00:00Z")
        );
        assert_eq!(updated.estimated_duration_seconds, Some(1800));
    }

    #[test]
    fn new_blocks_start_active_without_task_or_reactivation_lineage() {
        let conn = test_conn();
        let block = create_block(&conn, "2026-09-10", "focus", Some("09:00"), Some(1800));
        assert_eq!(block.lifecycle_state, BlockLifecycleState::Active);
        assert_eq!(block.task_id, None);
        assert_eq!(block.reactivated_from_block_id, None);
    }

    #[test]
    fn completing_a_recurring_occurrence_inactivates_only_that_occurrence() {
        let conn = test_conn();
        let recurrence = block_recurrence_repo::create(&conn, "w1", rule(&[1], "09:30", 1800))
            .expect("create series");
        block_recurrence_repo::materialize_for_date(&conn, "w1", "2099-01-05")
            .expect("materialize first occurrence");
        let occurrence = SqliteBlockRepo
            .list_for_date(&conn, "w1", "2099-01-05")
            .expect("list occurrence")
            .remove(0);

        start_block(&conn, &occurrence.id).expect("start");
        let completed = complete_block(&conn, &occurrence.id).expect("complete");
        assert_eq!(completed.status, BlockStatus::Completed);
        assert_eq!(completed.lifecycle_state, BlockLifecycleState::Inactive);
        assert_eq!(
            block_recurrence_repo::get(&conn, "w1", &recurrence.id)
                .expect("series")
                .lifecycle_state,
            BlockLifecycleState::Active,
            "the series stays active"
        );

        block_recurrence_repo::materialize_for_date(&conn, "w1", "2099-01-12")
            .expect("materialize next weekday");
        let next = SqliteBlockRepo
            .list_for_date(&conn, "w1", "2099-01-12")
            .expect("list next occurrence");
        assert_eq!(next.len(), 1);
        assert_eq!(next[0].status, BlockStatus::Planned);
        assert_eq!(next[0].lifecycle_state, BlockLifecycleState::Active);
    }

    #[test]
    fn deactivating_an_occurrence_or_a_deleted_block_is_rejected() {
        let conn = test_conn();
        let block = create_block(&conn, "2026-09-10", "focus", Some("09:00"), Some(1800));
        let deactivated = deactivate_block(&conn, "w1", &block.id).expect("deactivate");
        assert_eq!(deactivated.lifecycle_state, BlockLifecycleState::Inactive);
        let error = deactivate_block(&conn, "w1", &block.id).expect_err("already inactive");
        assert!(error.message.contains("Only an active block"));

        let deleted = create_block(&conn, "2026-09-11", "focus", Some("09:00"), Some(1800));
        delete_block(&conn, "w1", &deleted.id).expect("delete");
        let error = deactivate_block(&conn, "w1", &deleted.id).expect_err("deleted block");
        assert!(error.message.contains("Only an active block"));

        block_recurrence_repo::create(&conn, "w1", rule(&[1], "09:30", 1800)).expect("series");
        block_recurrence_repo::materialize_for_date(&conn, "w1", "2099-01-05")
            .expect("materialize occurrence");
        let occurrence = SqliteBlockRepo
            .list_for_date(&conn, "w1", "2099-01-05")
            .expect("list occurrence")
            .remove(0);
        let error =
            deactivate_block(&conn, "w1", &occurrence.id).expect_err("series owns occurrences");
        assert!(error.message.contains("managed through their series"));
    }

    #[test]
    fn deactivating_or_deleting_a_started_block_is_rejected() {
        let conn = test_conn();
        let block = create_block(&conn, "2026-09-10", "focus", Some("09:00"), Some(1800));
        start_block(&conn, &block.id).expect("start");
        let error = deactivate_block(&conn, "w1", &block.id).expect_err("started block");
        assert!(error.message.contains("before deactivation"));
        let error = delete_block(&conn, "w1", &block.id).expect_err("started block");
        assert!(error.message.contains("before deletion"));
    }

    #[test]
    fn deleted_blocks_are_soft_deleted_and_hidden_from_schedule_queries() {
        let conn = test_conn();
        seed_task(&conn, "w1", "task-a");
        let block = SqliteBlockRepo
            .create(
                &conn,
                "w1",
                &None,
                "2099-01-05",
                "focus",
                &Some("Synthetic focus".into()),
                &Some("09:00".into()),
                &None,
                Some(1800),
                &NO_TEXT,
                "flexible",
                &NO_TEXT,
                &Some("task-a".into()),
            )
            .expect("create block");
        start_block(&conn, &block.id).expect("start");
        complete_block(&conn, &block.id).expect("complete");

        let deleted = delete_block(&conn, "w1", &block.id).expect("soft delete");
        assert_eq!(deleted.lifecycle_state, BlockLifecycleState::Deleted);
        assert_eq!(deleted.status, BlockStatus::Completed);
        assert_eq!(deleted.task_id.as_deref(), Some("task-a"));
        assert!(deleted.actual_start_at.is_some());
        assert!(deleted.actual_end_at.is_some());
        assert!(delete_block(&conn, "w1", &block.id).is_err());

        assert!(SqliteBlockRepo
            .list_for_date(&conn, "w1", "2099-01-05")
            .expect("date list")
            .is_empty());
        assert!(SqliteBlockRepo
            .list_between(&conn, "w1", "2099-01-01", "2099-01-31")
            .expect("range list")
            .is_empty());
        assert!(
            list_upcoming_blocks(&conn, "w1", "2099-01-05", "2099-01-05", 10, 0)
                .expect("upcoming")
                .is_empty()
        );
        assert!(list_inactive_blocks(&conn, "w1")
            .expect("inactive")
            .is_empty());
        let row_count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM blocks WHERE id = ?1",
                [&block.id],
                |row| row.get(0),
            )
            .expect("count rows");
        assert_eq!(row_count, 1, "soft deletion keeps exactly one row");
    }

    #[test]
    fn reactivating_an_active_or_deleted_block_is_rejected() {
        let conn = test_conn();
        let active = create_block(&conn, "2026-09-10", "focus", Some("09:00"), Some(1800));
        let error = reactivate_block(&conn, "w1", &active.id, "once", 1, &None, None)
            .expect_err("active source");
        assert!(error.message.contains("Only an inactive block"));

        let deleted = create_block(&conn, "2026-09-11", "focus", Some("09:00"), Some(1800));
        delete_block(&conn, "w1", &deleted.id).expect("delete");
        let error = reactivate_block(&conn, "w1", &deleted.id, "once", 1, &None, None)
            .expect_err("deleted source");
        assert!(error
            .message
            .contains("Deleted blocks cannot be reactivated"));
    }

    #[test]
    fn expire_elapsed_blocks_inactivates_an_overdue_one_off_block() {
        let conn = test_conn();
        let block = create_block(&conn, "2026-10-05", "focus", Some("09:00"), Some(600));
        notification_repo::insert_block_event_delivery(
            &conn,
            "w1",
            &block.id,
            utc_instant("2026-10-05T09:00:00Z"),
            "A planned block starts now.",
            utc_instant("2026-10-05T08:00:00Z"),
        )
        .expect("pending block delivery");

        let now = utc_instant("2026-10-05T10:00:00Z");
        assert_eq!(expire_elapsed_blocks(&conn, now).expect("sweep"), 1);
        let expired = SqliteBlockRepo
            .get(&conn, &block.id)
            .expect("expired block");
        assert_eq!(expired.status, BlockStatus::Skipped);
        assert_eq!(expired.lifecycle_state, BlockLifecycleState::Inactive);

        let (status, reason): (String, Option<String>) = conn
            .query_row(
                "SELECT status, error_message FROM notification_deliveries
                 WHERE target_entity_id = ?1",
                [&block.id],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .expect("read delivery");
        assert_eq!(status, "failed");
        assert_eq!(
            reason.as_deref(),
            Some("The block expired before this notification was due.")
        );
        assert_eq!(
            expire_elapsed_blocks(&conn, now).expect("idempotent sweep"),
            0
        );
    }

    #[test]
    fn expire_elapsed_blocks_skips_rows_without_duration_or_start() {
        let conn = test_conn();
        let no_duration = create_block(&conn, "2020-01-01", "focus", Some("09:00"), None);
        let no_start = create_block(&conn, "2020-01-01", "rest", None, Some(1800));

        let now = utc_instant("2026-10-05T10:00:00Z");
        assert_eq!(expire_elapsed_blocks(&conn, now).expect("sweep"), 0);
        for id in [&no_duration.id, &no_start.id] {
            let block = SqliteBlockRepo.get(&conn, id).expect("block");
            assert_eq!(block.status, BlockStatus::Planned);
            assert_eq!(block.lifecycle_state, BlockLifecycleState::Active);
        }
    }

    #[test]
    fn expire_elapsed_blocks_leaves_started_and_future_blocks_untouched() {
        let conn = test_conn();
        let started = create_block(&conn, "2026-10-05", "focus", Some("09:00"), Some(600));
        start_block(&conn, &started.id).expect("start");
        let future = create_block(&conn, "2099-01-05", "focus", Some("09:00"), Some(600));

        let now = utc_instant("2026-10-05T12:00:00Z");
        assert_eq!(expire_elapsed_blocks(&conn, now).expect("sweep"), 0);
        let started = SqliteBlockRepo
            .get(&conn, &started.id)
            .expect("started block");
        assert_eq!(started.status, BlockStatus::Active);
        assert_eq!(started.lifecycle_state, BlockLifecycleState::Active);
        let future = SqliteBlockRepo
            .get(&conn, &future.id)
            .expect("future block");
        assert_eq!(future.status, BlockStatus::Planned);
        assert_eq!(future.lifecycle_state, BlockLifecycleState::Active);
    }

    #[test]
    fn expire_elapsed_blocks_expires_a_recurring_occurrence_alone() {
        let conn = test_conn();
        let recurrence = block_recurrence_repo::create(&conn, "w1", rule(&[1], "09:00", 1800))
            .expect("create series");
        block_recurrence_repo::materialize_for_date(&conn, "w1", "2099-01-05")
            .expect("materialize occurrence");
        let occurrence = SqliteBlockRepo
            .list_for_date(&conn, "w1", "2099-01-05")
            .expect("list occurrence")
            .remove(0);

        let now = utc_instant("2099-01-05T10:00:00Z");
        assert_eq!(expire_elapsed_blocks(&conn, now).expect("sweep"), 1);
        let expired = SqliteBlockRepo.get(&conn, &occurrence.id).expect("expired");
        assert_eq!(expired.status, BlockStatus::Skipped);
        assert_eq!(expired.lifecycle_state, BlockLifecycleState::Inactive);
        assert_eq!(
            block_recurrence_repo::get(&conn, "w1", &recurrence.id)
                .expect("series")
                .lifecycle_state,
            BlockLifecycleState::Active
        );

        block_recurrence_repo::materialize_for_date(&conn, "w1", "2099-01-12")
            .expect("materialize next occurrence");
        let next = SqliteBlockRepo
            .list_for_date(&conn, "w1", "2099-01-12")
            .expect("next occurrence");
        assert_eq!(next.len(), 1);
        assert_eq!(next[0].status, BlockStatus::Planned);
        assert_eq!(next[0].lifecycle_state, BlockLifecycleState::Active);
    }

    #[test]
    fn late_materialized_recurring_occurrences_are_not_inferred_as_skipped() {
        let conn = test_conn();
        let recurrence = block_recurrence_repo::create(&conn, "w1", rule(&[1], "09:00", 1800))
            .expect("create series");
        block_recurrence_repo::materialize_for_date(&conn, "w1", "2020-01-06")
            .expect("materialize missed occurrence");
        let occurrence = SqliteBlockRepo
            .list_for_date(&conn, "w1", "2020-01-06")
            .expect("list occurrence")
            .remove(0);

        assert_eq!(expire_elapsed_blocks(&conn, Utc::now()).expect("sweep"), 0);
        let row = SqliteBlockRepo
            .get(&conn, &occurrence.id)
            .expect("late occurrence");
        assert_eq!(row.status, BlockStatus::Planned);
        assert_eq!(row.lifecycle_state, BlockLifecycleState::Active);
        assert_eq!(
            block_recurrence_repo::get(&conn, "w1", &recurrence.id)
                .expect("series")
                .lifecycle_state,
            BlockLifecycleState::Active
        );
    }

    #[test]
    fn reactivation_once_creates_a_linked_occurrence_and_preserves_the_source() {
        let conn = test_conn();
        seed_task(&conn, "w1", "task-a");
        let source = SqliteBlockRepo
            .create(
                &conn,
                "w1",
                &None,
                "2026-09-10",
                "focus",
                &Some("Synthetic focus".into()),
                &Some("09:00".into()),
                &None,
                Some(1800),
                &NO_TEXT,
                "flexible",
                &Some("Synthetic note".into()),
                &Some("task-a".into()),
            )
            .expect("source block");
        start_block(&conn, &source.id).expect("start");
        let completed = complete_block(&conn, &source.id).expect("complete");

        let result = reactivate_block(&conn, "w1", &source.id, "once", 1, &None, None)
            .expect("reactivate once");
        let reactivated = result.block.expect("new one-off occurrence");
        assert!(result.recurrence.is_none());
        assert_eq!(
            reactivated.reactivated_from_block_id.as_deref(),
            Some(source.id.as_str())
        );
        assert_eq!(reactivated.status, BlockStatus::Planned);
        assert_eq!(reactivated.lifecycle_state, BlockLifecycleState::Active);
        assert_eq!(reactivated.daily_plan_id, None);
        assert_eq!(reactivated.planned_start_at.as_deref(), Some("09:00"));
        assert_eq!(reactivated.estimated_duration_seconds, Some(1800));
        assert_eq!(reactivated.task_id.as_deref(), Some("task-a"));
        assert_eq!(reactivated.note_markdown.as_deref(), Some("Synthetic note"));
        assert_eq!(reactivated.block_type, BlockType::Focus);
        let today = Utc::now().format("%Y-%m-%d").to_string();
        let expected_date =
            block_recurrence_repo::resolve_next_weekday_date(&today, 1).expect("next Monday");
        assert_eq!(reactivated.date, expected_date);
        assert_ne!(reactivated.id, source.id);

        let unchanged = SqliteBlockRepo.get(&conn, &source.id).expect("source row");
        assert_eq!(unchanged.status, BlockStatus::Completed);
        assert_eq!(unchanged.lifecycle_state, BlockLifecycleState::Inactive);
        assert_eq!(unchanged.actual_start_at, completed.actual_start_at);
        assert_eq!(unchanged.actual_end_at, completed.actual_end_at);
        let rows: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM blocks WHERE workspace_id = 'w1'",
                [],
                |row| row.get(0),
            )
            .expect("count blocks");
        assert_eq!(rows, 2, "the source row is preserved");
    }

    #[test]
    fn reactivation_recurring_creates_a_series_on_the_selected_weekday() {
        let conn = test_conn();
        let source = create_block(&conn, "2026-09-10", "focus", Some("09:00"), Some(1800));
        deactivate_block(&conn, "w1", &source.id).expect("deactivate");

        let result = reactivate_block(
            &conn,
            "w1",
            &source.id,
            "recurring",
            3,
            &Some("10:15".into()),
            Some(3600),
        )
        .expect("reactivate recurring");
        let recurrence = result.recurrence.expect("new series");
        assert!(result.block.is_none());
        assert_eq!(recurrence.weekdays, vec![3]);
        assert_eq!(recurrence.local_start_time, "10:15");
        assert_eq!(recurrence.estimated_duration_seconds, 3600);
        assert_eq!(recurrence.lifecycle_state, BlockLifecycleState::Active);
        let today = Utc::now().format("%Y-%m-%d").to_string();
        let expected_date =
            block_recurrence_repo::resolve_next_weekday_date(&today, 3).expect("next Wednesday");
        assert_eq!(recurrence.effective_from, expected_date);

        block_recurrence_repo::materialize_for_date(&conn, "w1", &expected_date)
            .expect("materialize series");
        let occurrence = SqliteBlockRepo
            .list_for_date(&conn, "w1", &expected_date)
            .expect("occurrence");
        assert_eq!(occurrence.len(), 1);
        assert_eq!(
            occurrence[0].recurrence_id.as_deref(),
            Some(recurrence.id.as_str())
        );
    }

    #[test]
    fn reactivation_drops_a_missing_task_reference() {
        let conn = test_conn();
        seed_task(&conn, "w1", "task-a");
        let source = SqliteBlockRepo
            .create(
                &conn,
                "w1",
                &None,
                "2026-09-10",
                "focus",
                &NO_TEXT,
                &Some("09:00".into()),
                &None,
                Some(1800),
                &NO_TEXT,
                "flexible",
                &NO_TEXT,
                &Some("task-a".into()),
            )
            .expect("source block");
        start_block(&conn, &source.id).expect("start");
        complete_block(&conn, &source.id).expect("complete");

        // Simulate a task that no longer exists despite the retained reference.
        conn.pragma_update(None, "foreign_keys", "OFF")
            .expect("suspend the foreign-key guard");
        conn.execute("DELETE FROM tasks WHERE id = 'task-a'", [])
            .expect("remove synthetic task");
        conn.pragma_update(None, "foreign_keys", "ON")
            .expect("restore the foreign-key guard");

        let result = reactivate_block(&conn, "w1", &source.id, "once", 1, &None, None)
            .expect("reactivate without a task");
        assert_eq!(result.block.expect("new occurrence").task_id, None);
    }

    #[test]
    fn recurring_reactivation_requires_local_start_and_duration() {
        let conn = test_conn();
        let untimed = create_block(&conn, "2026-09-11", "focus", None, Some(1800));
        deactivate_block(&conn, "w1", &untimed.id).expect("deactivate untimed");
        let error = reactivate_block(&conn, "w1", &untimed.id, "recurring", 1, &None, Some(1800))
            .expect_err("local start is required");
        assert!(error.message.contains("requires a local start time"));

        let undured = create_block(&conn, "2026-09-12", "focus", Some("09:00"), None);
        deactivate_block(&conn, "w1", &undured.id).expect("deactivate undured");
        let error = reactivate_block(&conn, "w1", &undured.id, "recurring", 1, &None, None)
            .expect_err("duration is required");
        assert!(error
            .message
            .contains("requires a positive estimated duration"));

        // A one-off reactivation keeps the legacy untimed freedom.
        let once = reactivate_block(&conn, "w1", &untimed.id, "once", 1, &None, None)
            .expect("one-off may stay untimed");
        assert_eq!(once.block.expect("block").planned_start_at, None);
    }

    #[test]
    fn create_rejects_a_task_from_another_workspace() {
        let conn = test_conn();
        seed_workspace(&conn, "w2");
        seed_task(&conn, "w2", "task-b");
        let error = SqliteBlockRepo
            .create(
                &conn,
                "w1",
                &None,
                "2026-09-10",
                "focus",
                &NO_TEXT,
                &NO_TEXT,
                &None,
                None,
                &NO_TEXT,
                "flexible",
                &NO_TEXT,
                &Some("task-b".into()),
            )
            .expect_err("cross-workspace task reference");
        assert_eq!(
            error.message,
            "The referenced task was not found in this workspace."
        );
    }

    #[test]
    fn update_persists_a_valid_task_reference() {
        let conn = test_conn();
        seed_task(&conn, "w1", "task-a");
        let block = create_block(&conn, "2026-09-10", "focus", Some("09:00"), Some(1800));
        let updated = SqliteBlockRepo
            .update(
                &conn,
                &block.id,
                &None,
                "2026-09-10",
                "focus",
                &NO_TEXT,
                &Some("09:00".into()),
                Some(1800),
                &NO_TEXT,
                "flexible",
                &NO_TEXT,
                &Some("task-a".into()),
            )
            .expect("update with a task");
        assert_eq!(updated.task_id.as_deref(), Some("task-a"));

        let error = SqliteBlockRepo
            .update(
                &conn,
                &block.id,
                &None,
                "2026-09-10",
                "focus",
                &NO_TEXT,
                &Some("09:00".into()),
                Some(1800),
                &NO_TEXT,
                "flexible",
                &NO_TEXT,
                &Some("unknown-task".into()),
            )
            .expect_err("unknown task reference");
        assert_eq!(
            error.message,
            "The referenced task was not found in this workspace."
        );
    }

    #[test]
    fn upcoming_pages_include_only_one_time_blocks_without_materializing_recurrences() {
        let conn = test_conn();
        let early = create_block(&conn, "2099-01-05", "focus", Some("08:00"), Some(1800));
        let later = create_block(&conn, "2099-01-05", "rest", Some("12:00"), Some(1800));
        let inactive = create_block(&conn, "2099-01-05", "rest", Some("12:00"), Some(1800));
        deactivate_block(&conn, "w1", &inactive.id).expect("deactivate");
        let deleted = create_block(&conn, "2099-01-06", "rest", Some("12:00"), Some(1800));
        delete_block(&conn, "w1", &deleted.id).expect("delete");
        block_recurrence_repo::create(&conn, "w1", rule(&[1, 3], "09:30", 1800))
            .expect("create series");

        let first_page =
            list_upcoming_blocks(&conn, "w1", "2099-01-05", "2099-01-07", 2, 0).expect("page");
        assert_eq!(first_page.len(), 2);
        assert_eq!(first_page[0].id, early.id);
        assert_eq!(first_page[1].id, later.id);

        let second_page =
            list_upcoming_blocks(&conn, "w1", "2099-01-05", "2099-01-07", 2, 2).expect("page");
        assert!(second_page.is_empty());

        let recurrence_rows: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM blocks WHERE recurrence_id IS NOT NULL",
                [],
                |row| row.get(0),
            )
            .expect("count recurrence rows");
        assert_eq!(
            recurrence_rows, 0,
            "listing one-offs never materializes a routine"
        );

        assert!(list_upcoming_blocks(&conn, "w1", "2099-01-05", "2099-01-07", 0, 0).is_err());
        assert!(list_upcoming_blocks(&conn, "w1", "2099-01-05", "2099-01-07", 201, 0).is_err());
        assert!(list_upcoming_blocks(&conn, "w1", "2099-01-05", "2099-01-07", 10, -1).is_err());
        assert!(list_upcoming_blocks(&conn, "w1", "not-a-date", "2099-01-07", 10, 0).is_err());
    }
}
