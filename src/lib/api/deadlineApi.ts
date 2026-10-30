import { dedupeInFlight, invoke } from "../tauri";
import type { DeadlineStatusDto, DeadlineTaskInfo } from "../types";

export function getTaskDeadlineStatus(taskId: string): Promise<DeadlineStatusDto> {
  return invoke<DeadlineStatusDto>("get_task_deadline_status", { taskId });
}

export function getProjectDeadlineStatus(projectId: string): Promise<DeadlineStatusDto> {
  return invoke<DeadlineStatusDto>("get_project_deadline_status", { projectId });
}

export function getGoalDeadlineStatus(goalId: string): Promise<DeadlineStatusDto> {
  return invoke<DeadlineStatusDto>("get_goal_deadline_status", { goalId });
}

export function listAtRiskTasks(workspaceId: string): Promise<DeadlineTaskInfo[]> {
  // Startup may ask for the at-risk set from several mounts at once; share
  // one in-flight command per workspace. Sequential refreshes still re-run.
  return dedupeInFlight(`list_at_risk_tasks:${workspaceId}`, () =>
    invoke<DeadlineTaskInfo[]>("list_at_risk_tasks", { workspaceId }),
  );
}
