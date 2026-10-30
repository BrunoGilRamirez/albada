use std::collections::HashSet;

use crate::domain::types::TaskStatus;
use crate::error::AppError;

/// Allowed transitions between task statuses.
///
/// Mirrors `docs/domain/task-lifecycle.md`:
/// - `inbox -> {planned, in_progress, blocked, completed, deferred, cancelled}`
///   (`inbox -> completed` lets a freshly captured trivial task be completed
///   directly; `inbox -> blocked` requires an explicit blocker, validated in the
///   command layer)
/// - `planned -> {inbox, in_progress, blocked, deferred, cancelled}`
/// - `in_progress -> {planned, blocked, completed, deferred, cancelled}`
/// - `blocked -> {planned, in_progress, deferred, cancelled}`
/// - `completed -> {in_progress, planned}` (reopen)
/// - `cancelled -> {inbox, planned}` (reopen)
/// - `deferred -> {inbox, planned}`
pub fn allowed_transitions() -> Vec<(TaskStatus, Vec<TaskStatus>)> {
    vec![
        (
            TaskStatus::Inbox,
            vec![
                TaskStatus::Planned,
                TaskStatus::InProgress,
                TaskStatus::Blocked,
                TaskStatus::Completed,
                TaskStatus::Deferred,
                TaskStatus::Cancelled,
            ],
        ),
        (
            TaskStatus::Planned,
            vec![
                TaskStatus::Inbox,
                TaskStatus::InProgress,
                TaskStatus::Blocked,
                TaskStatus::Deferred,
                TaskStatus::Cancelled,
            ],
        ),
        (
            TaskStatus::InProgress,
            vec![
                TaskStatus::Planned,
                TaskStatus::Blocked,
                TaskStatus::Completed,
                TaskStatus::Deferred,
                TaskStatus::Cancelled,
            ],
        ),
        (
            TaskStatus::Blocked,
            vec![
                TaskStatus::Planned,
                TaskStatus::InProgress,
                TaskStatus::Deferred,
                TaskStatus::Cancelled,
            ],
        ),
        (
            TaskStatus::Completed,
            vec![TaskStatus::InProgress, TaskStatus::Planned],
        ),
        (
            TaskStatus::Cancelled,
            vec![TaskStatus::Inbox, TaskStatus::Planned],
        ),
        (
            TaskStatus::Deferred,
            vec![TaskStatus::Inbox, TaskStatus::Planned],
        ),
    ]
}

/// Validate that a transition from `from` to `to` is allowed.
///
/// Returns an error describing the invalid pair when disallowed. Identical
/// from/to values are rejected (a no-op is not a transition).
pub fn validate_transition(from: TaskStatus, to: TaskStatus) -> Result<(), AppError> {
    if from == to {
        return Err(AppError::validation(format!(
            "Task is already '{from}'; no transition needed."
        )));
    }

    let table = allowed_transitions();
    let allowed: HashSet<TaskStatus> = table
        .iter()
        .find(|(src, _)| *src == from)
        .map(|(_, targets)| targets.iter().copied().collect())
        .unwrap_or_default();

    if allowed.contains(&to) {
        Ok(())
    } else {
        Err(AppError::validation(format!(
            "Invalid transition from '{from}' to '{to}'."
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normal_flow_is_allowed() {
        assert!(validate_transition(TaskStatus::Inbox, TaskStatus::Planned).is_ok());
        assert!(validate_transition(TaskStatus::Planned, TaskStatus::InProgress).is_ok());
        assert!(validate_transition(TaskStatus::InProgress, TaskStatus::Completed).is_ok());
    }

    #[test]
    fn blocked_transitions_are_allowed() {
        assert!(validate_transition(TaskStatus::Planned, TaskStatus::Blocked).is_ok());
        assert!(validate_transition(TaskStatus::InProgress, TaskStatus::Blocked).is_ok());
        assert!(validate_transition(TaskStatus::Blocked, TaskStatus::Planned).is_ok());
        assert!(validate_transition(TaskStatus::Blocked, TaskStatus::InProgress).is_ok());
    }

    #[test]
    fn reopen_transitions_are_allowed() {
        assert!(validate_transition(TaskStatus::Completed, TaskStatus::Planned).is_ok());
        assert!(validate_transition(TaskStatus::Completed, TaskStatus::InProgress).is_ok());
        assert!(validate_transition(TaskStatus::Cancelled, TaskStatus::Planned).is_ok());
        assert!(validate_transition(TaskStatus::Deferred, TaskStatus::Planned).is_ok());
    }

    #[test]
    fn inbox_to_completed_is_allowed() {
        assert!(validate_transition(TaskStatus::Inbox, TaskStatus::Completed).is_ok());
    }

    #[test]
    fn completed_to_cancelled_is_rejected() {
        assert!(validate_transition(TaskStatus::Completed, TaskStatus::Cancelled).is_err());
    }

    #[test]
    fn noop_transition_is_rejected() {
        assert!(validate_transition(TaskStatus::Planned, TaskStatus::Planned).is_err());
    }

    #[test]
    fn cancelled_to_in_progress_is_rejected() {
        assert!(validate_transition(TaskStatus::Cancelled, TaskStatus::InProgress).is_err());
    }
}
