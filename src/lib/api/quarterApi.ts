import { invoke } from "../tauri";
import type { Quarter, QuarterFocus } from "../types";

export interface CreateQuarterInput {
  year_theme_id: string;
  code: string;
  start_date: string;
  end_date: string;
}

export interface UpdateQuarterInput {
  start_date: string;
  end_date: string;
  status: string;
}

export interface CreateQuarterFocusInput {
  quarter_id: string;
  goal_id?: string | null;
  project_id?: string | null;
  priority: string;
  intention_markdown?: string | null;
  success_signal_markdown?: string | null;
}

export interface UpdateQuarterFocusInput {
  priority: string;
  intention_markdown?: string | null;
  success_signal_markdown?: string | null;
  status: string;
}

export function createQuarter(input: CreateQuarterInput): Promise<Quarter> {
  return invoke<Quarter>("create_quarter", { dto: input });
}

export function getQuarter(id: string): Promise<Quarter> {
  return invoke<Quarter>("get_quarter", { id });
}

export function listQuarters(yearThemeId: string): Promise<Quarter[]> {
  return invoke<Quarter[]>("list_quarters", { yearThemeId });
}

export function updateQuarter(id: string, input: UpdateQuarterInput): Promise<Quarter> {
  return invoke<Quarter>("update_quarter", { id, dto: input });
}

export function createQuarterFocus(input: CreateQuarterFocusInput): Promise<QuarterFocus> {
  return invoke<QuarterFocus>("create_quarter_focus", { dto: input });
}

export function listQuarterFocus(quarterId: string): Promise<QuarterFocus[]> {
  return invoke<QuarterFocus[]>("list_quarter_focus", { quarterId });
}

export function updateQuarterFocus(
  id: string,
  input: UpdateQuarterFocusInput,
): Promise<QuarterFocus> {
  return invoke<QuarterFocus>("update_quarter_focus", { id, dto: input });
}
