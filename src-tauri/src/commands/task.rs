use std::str::FromStr;

use crate::commands::dto::{
    CloseParentManuallyDto, CreateChildTaskDto, CreateTaskDto, MoveTaskDto,
    TransitionTaskStatusDto, UpdateTaskDto,
};
use crate::domain::models::Task;
use crate::domain::task_closure;
use crate::domain::types::{ClosureMode, EffortSize, HistorySource, Priority, TaskStatus};
use crate::error::AppError;
use crate::repos::task_repo::{complete_task, tree_rows, SqliteTaskRepo, TaskRepo};
use crate::state::Db;
use serde::Serialize;
use tauri::State;

/// A task plus its descendants and blocking prerequisites, for tree views.
#[derive(Debug, Serialize)]
pub struct TaskTreeDto {
    pub task: Task,
    pub children: Vec<TaskTreeDto>,
    pub blocked_by: Vec<crate::domain::dependency_graph::PrerequisiteTask>,
}

fn build_tree(
    conn: &rusqlite::Connection,
    root: &Task,
    children_map: &std::collections::HashMap<String, Vec<Task>>,
) -> Result<TaskTreeDto, AppError> {
    let blocked_by = crate::domain::dependency_graph::incomplete_prerequisites(conn, &root.id)?;
    let children = children_map
        .get(&root.id)
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .map(|child| build_tree(conn, &child, children_map))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(TaskTreeDto {
        task: root.clone(),
        children,
        blocked_by,
    })
}

#[tauri::command]
pub fn create_task(db: State<'_, Db>, dto: CreateTaskDto) -> Result<Task, AppError> {
    log::debug!(
        "create_task payload: workspace_id={:?} project_id={:?} goal_id={:?} parent_task_id={:?} title={:?} priority={:?} effort_size={:?}",
        dto.workspace_id,
        dto.project_id,
        dto.goal_id,
        dto.parent_task_id,
        dto.title,
        dto.priority,
        dto.effort_size
    );
    let effort = dto
        .effort_size
        .as_deref()
        .map(EffortSize::from_str)
        .transpose()?;
    let priority: Priority = dto.priority.parse()?;
    crate::log_command!(
        "create_task",
        db.with_conn(|conn| {
            SqliteTaskRepo.create(
                conn,
                &dto.workspace_id,
                &dto.project_id,
                &dto.goal_id,
                &dto.parent_task_id,
                &dto.title,
                &dto.description_markdown,
                &dto.notes_markdown,
                priority,
                &effort,
                &dto.target_date,
                &dto.scheduled_date,
            )
        })
    )
}

#[tauri::command]
pub fn create_child_task(db: State<'_, Db>, dto: CreateChildTaskDto) -> Result<Task, AppError> {
    let effort = dto
        .effort_size
        .as_deref()
        .map(EffortSize::from_str)
        .transpose()?;
    let priority: Priority = dto.priority.parse()?;
    crate::log_command!(
        "create_child_task",
        db.with_conn(|conn| {
            let workspace_id = conn.query_row(
                "SELECT workspace_id FROM tasks WHERE id = ?1",
                [&dto.parent_id],
                |row| row.get::<_, String>(0),
            )?;
            SqliteTaskRepo.create_child_task(
                conn,
                &dto.parent_id,
                &workspace_id,
                &dto.title,
                &dto.description_markdown,
                &dto.notes_markdown,
                priority,
                &effort,
                &dto.target_date,
                &dto.scheduled_date,
            )
        })
    )
}

#[tauri::command]
pub fn get_task(db: State<'_, Db>, id: String) -> Result<Task, AppError> {
    db.with_conn(|conn| SqliteTaskRepo.get(conn, &id))
}

#[tauri::command]
pub fn get_task_with_tree(db: State<'_, Db>, id: String) -> Result<TaskTreeDto, AppError> {
    db.with_conn(|conn| {
        let root = SqliteTaskRepo.get(conn, &id)?;
        let rows = tree_rows(conn, &id)?;
        let mut children_map: std::collections::HashMap<String, Vec<Task>> =
            std::collections::HashMap::new();
        for task in rows {
            if let Some(parent_id) = &task.parent_task_id {
                children_map
                    .entry(parent_id.clone())
                    .or_default()
                    .push(task);
            }
        }
        build_tree(conn, &root, &children_map)
    })
}

#[tauri::command]
pub fn list_tasks(db: State<'_, Db>, workspace_id: String) -> Result<Vec<Task>, AppError> {
    db.with_conn(|conn| SqliteTaskRepo.list(conn, &workspace_id))
}

#[tauri::command]
pub fn list_tasks_for_project(
    db: State<'_, Db>,
    project_id: String,
) -> Result<Vec<Task>, AppError> {
    db.with_conn(|conn| SqliteTaskRepo.list_for_project(conn, &project_id))
}

#[tauri::command]
pub fn list_tasks_for_goal(db: State<'_, Db>, goal_id: String) -> Result<Vec<Task>, AppError> {
    db.with_conn(|conn| SqliteTaskRepo.list_for_goal(conn, &goal_id))
}

#[tauri::command]
pub fn list_standalone_tasks(
    db: State<'_, Db>,
    workspace_id: String,
) -> Result<Vec<Task>, AppError> {
    db.with_conn(|conn| SqliteTaskRepo.list_standalone(conn, &workspace_id))
}

#[tauri::command]
pub fn list_archived_tasks(db: State<'_, Db>, workspace_id: String) -> Result<Vec<Task>, AppError> {
    db.with_conn(|conn| SqliteTaskRepo.list_archived(conn, &workspace_id))
}

#[tauri::command]
pub fn update_task(db: State<'_, Db>, id: String, dto: UpdateTaskDto) -> Result<Task, AppError> {
    let effort = dto
        .effort_size
        .as_deref()
        .map(EffortSize::from_str)
        .transpose()?;
    let priority: Priority = dto.priority.parse()?;
    db.with_conn(|conn| {
        SqliteTaskRepo.update(
            conn,
            &id,
            &dto.title,
            &dto.description_markdown,
            &dto.notes_markdown,
            priority,
            &effort,
            &dto.target_date,
            &dto.scheduled_date,
        )
    })
}

#[tauri::command]
pub fn move_task(db: State<'_, Db>, id: String, dto: MoveTaskDto) -> Result<Task, AppError> {
    db.with_tx(|tx| {
        SqliteTaskRepo.move_task(tx, &id, &dto.project_id, &dto.goal_id, &dto.parent_task_id)
    })
}

/// Attach an existing leaf (childless root) task to a project or goal without
/// moving its subtree.
#[tauri::command]
pub fn attach_task(
    db: State<'_, Db>,
    id: String,
    project_id: Option<String>,
    goal_id: Option<String>,
) -> Result<Task, AppError> {
    crate::log_command!(
        "attach_task",
        db.with_conn(|conn| SqliteTaskRepo.attach_to_scope(conn, &id, &project_id, &goal_id))
    )
}

#[tauri::command]
pub fn transition_task_status(
    db: State<'_, Db>,
    id: String,
    dto: TransitionTaskStatusDto,
) -> Result<Task, AppError> {
    let to: TaskStatus = dto.to_status.parse()?;
    crate::log_command!(
        "transition_task_status",
        db.with_tx(|tx| {
            if to == TaskStatus::Blocked {
                let blockers =
                    crate::domain::dependency_graph::incomplete_prerequisites(tx, &id)?;
                let has_note = dto
                    .reason_markdown
                    .as_ref()
                    .map(|note| !note.trim().is_empty())
                    .unwrap_or(false);
                if blockers.is_empty() && !has_note {
                    return Err(AppError::validation(
                        "A task can only be marked blocked when it has an active dependency or a blocking note.",
                    ));
                }
            }
            task_closure::transition_status(
                tx,
                &id,
                to,
                &dto.reason_markdown,
                HistorySource::User,
            )?;
            Ok(())
        })
    )?;
    db.with_conn(|conn| SqliteTaskRepo.get(conn, &id))
}

#[tauri::command]
pub fn complete_task_cmd(db: State<'_, Db>, id: String) -> Result<Task, AppError> {
    crate::log_command!(
        "complete_task",
        db.with_tx(|tx| {
            complete_task(tx, &id, &None)?;
            Ok(())
        })
    )?;
    db.with_conn(|conn| SqliteTaskRepo.get(conn, &id))
}

#[tauri::command]
pub fn close_parent_manually(
    db: State<'_, Db>,
    id: String,
    dto: CloseParentManuallyDto,
) -> Result<Task, AppError> {
    let mode: ClosureMode = dto.closure_mode.parse()?;
    crate::log_command!(
        "close_parent_manually",
        db.with_tx(|tx| {
            task_closure::close_parent_manually(tx, &id, mode, &dto.reason_markdown)?;
            Ok(())
        })
    )?;
    db.with_conn(|conn| SqliteTaskRepo.get(conn, &id))
}

#[tauri::command]
pub fn archive_task(db: State<'_, Db>, id: String) -> Result<Task, AppError> {
    crate::log_command!(
        "archive_task",
        db.with_tx(|tx| {
            SqliteTaskRepo.archive_subtree(tx, &id)?;
            Ok(())
        })
    )?;
    db.with_conn(|conn| SqliteTaskRepo.get(conn, &id))
}

#[tauri::command]
pub fn restore_task(db: State<'_, Db>, id: String) -> Result<Task, AppError> {
    crate::log_command!(
        "restore_task",
        db.with_tx(|tx| {
            SqliteTaskRepo.restore_subtree(tx, &id)?;
            Ok(())
        })
    )?;
    db.with_conn(|conn| SqliteTaskRepo.get(conn, &id))
}

#[tauri::command]
pub fn restore_archived_task(db: State<'_, Db>, id: String) -> Result<Task, AppError> {
    crate::log_command!(
        "restore_archived_task",
        db.with_tx(|tx| {
            SqliteTaskRepo.restore_task_including_subtree(tx, &id)?;
            Ok(())
        })
    )?;
    db.with_conn(|conn| SqliteTaskRepo.get(conn, &id))
}

#[tauri::command]
pub fn delete_task(db: State<'_, Db>, id: String) -> Result<(), AppError> {
    crate::log_command!(
        "delete_task",
        db.with_tx(|tx| {
            SqliteTaskRepo.delete_subtree(tx, &id)?;
            Ok(())
        })
    )
}

#[tauri::command]
pub fn list_task_history(
    db: State<'_, Db>,
    task_id: String,
) -> Result<Vec<crate::domain::models::StatusHistoryEntry>, AppError> {
    use crate::repos::status_history_repo::StatusHistoryRepo;
    db.with_conn(|conn| {
        crate::repos::status_history_repo::SqliteStatusHistoryRepo.list_for_task(conn, &task_id)
    })
}

#[tauri::command]
pub fn why_is_blocked(
    db: State<'_, Db>,
    task_id: String,
) -> Result<Vec<crate::domain::dependency_graph::PrerequisiteTask>, AppError> {
    db.with_conn(|conn| {
        let blockers = crate::domain::dependency_graph::incomplete_prerequisites(conn, &task_id)?;
        Ok(blockers)
    })
}
