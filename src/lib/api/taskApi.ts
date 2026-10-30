import { dedupeInFlight, invoke } from "../tauri";
import type {
  PrerequisiteTask,
  StatusHistoryEntry,
  Task,
  TaskDependency,
  TaskTree,
} from "../types";

export interface CreateTaskInput {
  workspace_id: string;
  project_id?: string | null;
  goal_id?: string | null;
  parent_task_id?: string | null;
  title: string;
  description_markdown?: string | null;
  notes_markdown?: string | null;
  priority: string;
  effort_size?: string | null;
  target_date?: string | null;
  scheduled_date?: string | null;
}

export interface CreateChildTaskInput {
  parent_id: string;
  title: string;
  description_markdown?: string | null;
  notes_markdown?: string | null;
  priority: string;
  effort_size?: string | null;
  target_date?: string | null;
  scheduled_date?: string | null;
}

export interface UpdateTaskInput {
  title: string;
  description_markdown?: string | null;
  notes_markdown?: string | null;
  priority: string;
  effort_size?: string | null;
  target_date?: string | null;
  scheduled_date?: string | null;
}

export interface MoveTaskInput {
  project_id?: string | null;
  goal_id?: string | null;
  parent_task_id?: string | null;
}

export function createTask(input: CreateTaskInput): Promise<Task> {
  return invoke<Task>("create_task", { dto: input });
}

export function createChildTask(input: CreateChildTaskInput): Promise<Task> {
  return invoke<Task>("create_child_task", { dto: input });
}

export function getTask(id: string): Promise<Task> {
  return invoke<Task>("get_task", { id });
}

export function getTaskWithTree(id: string): Promise<TaskTree> {
  return invoke<TaskTree>("get_task_with_tree", { id });
}

export function listTasks(workspaceId: string): Promise<Task[]> {
  // Today's StrictMode double-load asks for the workspace task set twice at
  // once; concurrent same-workspace callers share one native command, while
  // sequential refreshes (after task edits) always re-run.
  return dedupeInFlight(`list_tasks:${workspaceId}`, () =>
    invoke<Task[]>("list_tasks", { workspaceId }),
  );
}

export function listTasksForProject(projectId: string): Promise<Task[]> {
  return invoke<Task[]>("list_tasks_for_project", { projectId });
}

export function listTasksForGoal(goalId: string): Promise<Task[]> {
  return invoke<Task[]>("list_tasks_for_goal", { goalId });
}

export function listStandaloneTasks(workspaceId: string): Promise<Task[]> {
  return invoke<Task[]>("list_standalone_tasks", { workspaceId });
}

export function listArchivedTasks(workspaceId: string): Promise<Task[]> {
  return invoke<Task[]>("list_archived_tasks", { workspaceId });
}

export function updateTask(id: string, input: UpdateTaskInput): Promise<Task> {
  return invoke<Task>("update_task", { id, dto: input });
}

export function moveTask(id: string, input: MoveTaskInput): Promise<Task> {
  return invoke<Task>("move_task", { id, dto: input });
}

export function attachTask(
  id: string,
  projectId: string | null,
  goalId: string | null,
): Promise<Task> {
  return invoke<Task>("attach_task", { id, projectId, goalId });
}

export function transitionTaskStatus(
  id: string,
  to_status: string,
  reason_markdown?: string | null,
): Promise<Task> {
  return invoke<Task>("transition_task_status", {
    id,
    dto: { to_status, reason_markdown },
  });
}

export function completeTask(id: string): Promise<Task> {
  return invoke<Task>("complete_task_cmd", { id });
}

export function closeParentManually(
  id: string,
  closure_mode: string,
  reason_markdown: string,
): Promise<Task> {
  return invoke<Task>("close_parent_manually", {
    id,
    dto: { closure_mode, reason_markdown },
  });
}

export function archiveTask(id: string): Promise<Task> {
  return invoke<Task>("archive_task", { id });
}

export function restoreTask(id: string): Promise<Task> {
  return invoke<Task>("restore_task", { id });
}

export function restoreArchivedTask(id: string): Promise<Task> {
  return invoke<Task>("restore_archived_task", { id });
}

export function deleteTask(id: string): Promise<void> {
  return invoke<void>("delete_task", { id });
}

export function listTaskHistory(taskId: string): Promise<StatusHistoryEntry[]> {
  return invoke<StatusHistoryEntry[]>("list_task_history", { taskId });
}

export function whyIsBlocked(taskId: string): Promise<PrerequisiteTask[]> {
  return invoke<PrerequisiteTask[]>("why_is_blocked", { taskId });
}

export interface AddDependencyInput {
  blocked_task_id: string;
  prerequisite_task_id: string;
  note_markdown?: string | null;
}

export function addDependency(input: AddDependencyInput): Promise<TaskDependency> {
  return invoke<TaskDependency>("add_dependency", { dto: input });
}

export function removeDependency(id: string, reason_markdown?: string | null): Promise<void> {
  return invoke<void>("remove_dependency", { id, dto: { reason_markdown } });
}

export function listBlockingTasks(taskId: string): Promise<PrerequisiteTask[]> {
  return invoke<PrerequisiteTask[]>("list_blocking_tasks", { taskId });
}

export function listAllPrerequisites(taskId: string): Promise<PrerequisiteTask[]> {
  return invoke<PrerequisiteTask[]>("list_all_prerequisites", { taskId });
}

export function listDependenciesForTask(taskId: string): Promise<TaskDependency[]> {
  return invoke<TaskDependency[]>("list_dependencies_for_task", { taskId });
}
