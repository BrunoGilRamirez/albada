import type { Task } from "./types";

/**
 * Task eligibility for the block task picker (8.7.9).
 *
 * A block may reference a task, but only a task that can still be worked on:
 * terminal tasks (completed/cancelled), archived tasks, and tasks whose
 * deadline already passed are never offered. The block never changes the
 * task's status or progress; this filter only decides what the user can
 * pick.
 *
 * Pure and local: no command calls, no persistence, no user data beyond the
 * rows the caller already loaded.
 */

/** Terminal task statuses: the work is already closed. */
const TERMINAL_TASK_STATUSES = new Set<string>(["completed", "cancelled"]);

/**
 * Calendar-local date (`YYYY-MM-DD`) of a timestamp, used when normalizing
 * deadline instants elsewhere. A plain date string is already a local
 * calendar date and stays verbatim.
 */
export function localCalendarDate(value: string): string {
  const dateOnly = /^(\d{4}-\d{2}-\d{2})$/.exec(value);
  if (dateOnly) {
    return dateOnly[1];
  }
  const parsed = new Date(value);
  if (Number.isNaN(parsed.getTime())) {
    return "";
  }
  const year = parsed.getFullYear();
  const month = String(parsed.getMonth() + 1).padStart(2, "0");
  const day = String(parsed.getDate()).padStart(2, "0");
  return `${year}-${month}-${day}`;
}

/** Today's local calendar date (`YYYY-MM-DD`). */
export function localTodayDate(now: Date = new Date()): string {
  const year = now.getFullYear();
  const month = String(now.getMonth() + 1).padStart(2, "0");
  const day = String(now.getDate()).padStart(2, "0");
  return `${year}-${month}-${day}`;
}

/**
 * True when the task may be attached to a new block.
 *
 * Overdue mirrors the native `deadline_status_for_task_model` derivation
 * (`src-tauri/src/domain/deadline.rs`): the deadline field is `target_date`,
 * and a `deferred` task derives a `none` deadline condition, so it is never
 * overdue. Only an open (`inbox | planned | in_progress | blocked`) task with
 * a past `target_date` is excluded as overdue; terminal and archived tasks
 * are excluded above.
 */
const OVERDUE_CAPABLE_STATUSES = new Set<string>(["inbox", "planned", "in_progress", "blocked"]);

export function isEligibleBlockTask(task: Task, today: string): boolean {
  if (TERMINAL_TASK_STATUSES.has(task.status)) {
    return false;
  }
  if (task.archived_at != null) {
    return false;
  }
  const targetDate = task.target_date;
  if (targetDate != null && OVERDUE_CAPABLE_STATUSES.has(task.status) && targetDate < today) {
    return false;
  }
  return true;
}

/** Eligible tasks in the loaded order, ready for the picker. */
export function eligibleBlockTasks(tasks: Task[], today: string): Task[] {
  return tasks.filter((task) => isEligibleBlockTask(task, today));
}
