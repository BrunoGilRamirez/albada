import { invoke } from "../tauri";
import type { ProgressDetails } from "../types";

export function getProjectProgress(projectId: string): Promise<ProgressDetails> {
  return invoke<ProgressDetails>("get_project_progress", { projectId });
}

export function getGoalProgress(goalId: string): Promise<ProgressDetails> {
  return invoke<ProgressDetails>("get_goal_progress", { goalId });
}

export function getTaskProgress(taskId: string): Promise<ProgressDetails> {
  return invoke<ProgressDetails>("get_task_progress", { taskId });
}
