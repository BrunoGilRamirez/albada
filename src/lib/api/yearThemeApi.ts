import { invoke } from "../tauri";
import type { YearTheme } from "../types";

export interface CreateYearThemeInput {
  workspace_id: string;
  year: number;
  name: string;
  description_markdown?: string | null;
}

export interface UpdateYearThemeInput {
  name: string;
  description_markdown?: string | null;
}

export function createYearTheme(input: CreateYearThemeInput): Promise<YearTheme> {
  return invoke<YearTheme>("create_year_theme", { dto: input });
}

export function getYearTheme(id: string): Promise<YearTheme> {
  return invoke<YearTheme>("get_year_theme", { id });
}

export function listYearThemes(workspaceId: string): Promise<YearTheme[]> {
  return invoke<YearTheme[]>("list_year_themes", { workspaceId });
}

export function updateYearTheme(id: string, input: UpdateYearThemeInput): Promise<YearTheme> {
  return invoke<YearTheme>("update_year_theme", { id, dto: input });
}

export function archiveYearTheme(id: string): Promise<YearTheme> {
  return invoke<YearTheme>("archive_year_theme", { id });
}
