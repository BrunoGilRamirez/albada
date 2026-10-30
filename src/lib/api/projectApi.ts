import { invoke } from "../tauri";
import type { Project } from "../types";

export interface CreateProjectInput {
  workspace_id: string;
  goal_id?: string | null;
  title: string;
  description_markdown?: string | null;
  why_it_matters_markdown?: string | null;
  definition_of_done_markdown?: string | null;
}

export interface UpdateProjectInput {
  title: string;
  description_markdown?: string | null;
  why_it_matters_markdown?: string | null;
  definition_of_done_markdown?: string | null;
  status: string;
  priority: string;
  target_date?: string | null;
}

export function createProject(input: CreateProjectInput): Promise<Project> {
  return invoke<Project>("create_project", { dto: input });
}

export function getProject(id: string): Promise<Project> {
  return invoke<Project>("get_project", { id });
}

export function listProjects(workspaceId: string): Promise<Project[]> {
  return invoke<Project[]>("list_projects", { workspaceId });
}

export function listProjectsForGoal(goalId: string): Promise<Project[]> {
  return invoke<Project[]>("list_projects_for_goal", { goalId });
}

export function updateProject(id: string, input: UpdateProjectInput): Promise<Project> {
  return invoke<Project>("update_project", { id, dto: input });
}

export function archiveProject(id: string): Promise<Project> {
  return invoke<Project>("archive_project", { id });
}

export function moveProject(id: string, goalId: string | null): Promise<Project> {
  return invoke<Project>("move_project", { id, goalId });
}
