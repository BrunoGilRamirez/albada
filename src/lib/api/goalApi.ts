import { invoke } from "../tauri";
import type { Goal } from "../types";

export interface CreateGoalInput {
  workspace_id: string;
  year_theme_id?: string | null;
  title: string;
  description_markdown?: string | null;
  why_it_matters_markdown?: string | null;
  success_definition_markdown?: string | null;
}

export interface UpdateGoalInput {
  title: string;
  description_markdown?: string | null;
  why_it_matters_markdown?: string | null;
  success_definition_markdown?: string | null;
  status: string;
  priority: string;
  target_date?: string | null;
}

export function createGoal(input: CreateGoalInput): Promise<Goal> {
  return invoke<Goal>("create_goal", { dto: input });
}

export function getGoal(id: string): Promise<Goal> {
  return invoke<Goal>("get_goal", { id });
}

export function listGoals(workspaceId: string): Promise<Goal[]> {
  return invoke<Goal[]>("list_goals", { workspaceId });
}

export function updateGoal(id: string, input: UpdateGoalInput): Promise<Goal> {
  return invoke<Goal>("update_goal", { id, dto: input });
}

export function archiveGoal(id: string): Promise<Goal> {
  return invoke<Goal>("archive_goal", { id });
}
