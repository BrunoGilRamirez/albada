import { dedupeInFlight, invoke } from "../tauri";
import type { CapacitySuggestion, DailyPlan, DailyPlanTask, ReplanDecision } from "../types";

export interface SelectTaskForDayInput {
  daily_plan_id: string;
  task_id: string;
  reason: string;
}

export interface ReplanTaskInput {
  decision: ReplanDecision;
  note_markdown?: string | null;
}

export interface UpdateDailyPlanIntentionInput {
  intention_markdown?: string | null;
  capacity_note_markdown?: string | null;
}

export function getOrCreateDailyPlan(date: string): Promise<DailyPlan> {
  // The command returns the plan but no workspace id, so callers cannot
  // collapse the ensure+plan pair into one call; dedupe the plan call per
  // date instead. Concurrent same-date callers share one native command.
  return dedupeInFlight(`get_or_create_daily_plan:${date}`, () =>
    invoke<DailyPlan>("get_or_create_daily_plan", { date }),
  );
}

export function getDailyPlanByDate(workspaceId: string, date: string): Promise<DailyPlan | null> {
  return invoke<DailyPlan | null>("get_daily_plan_by_date", { workspaceId, date });
}

export function listDailyPlansBetween(
  workspaceId: string,
  fromDate: string,
  toDate: string,
): Promise<DailyPlan[]> {
  return invoke<DailyPlan[]>("list_daily_plans_between", {
    workspaceId,
    fromDate,
    toDate,
  });
}

export function updateDailyPlanStatus(dailyPlanId: string, status: string): Promise<DailyPlan> {
  return invoke<DailyPlan>("update_daily_plan_status", { dailyPlanId, status });
}

export function updateDailyPlanIntention(
  dailyPlanId: string,
  input: UpdateDailyPlanIntentionInput,
): Promise<DailyPlan> {
  return invoke<DailyPlan>("update_daily_plan_intention", { dailyPlanId, dto: input });
}

export function updateDailyPlanReview(
  dailyPlanId: string,
  reviewMarkdown: string | null,
): Promise<DailyPlan> {
  return invoke<DailyPlan>("update_daily_plan_review", {
    dailyPlanId,
    dto: { review_markdown: reviewMarkdown },
  });
}

export function setDailyPlanPlanningBlock(
  dailyPlanId: string,
  blockId: string | null,
): Promise<DailyPlan> {
  return invoke<DailyPlan>("set_daily_plan_planning_block", { dailyPlanId, blockId });
}

export function selectTaskForDay(input: SelectTaskForDayInput): Promise<DailyPlanTask> {
  return invoke<DailyPlanTask>("select_task_for_day", { dto: input });
}

export function deselectTaskForDay(dailyPlanTaskId: string): Promise<void> {
  return invoke<void>("deselect_task_for_day", { dailyPlanTaskId });
}

export function reorderDayTasks(dailyPlanId: string, orderedIds: string[]): Promise<void> {
  return invoke<void>("reorder_day_tasks", { dailyPlanId, orderedIds });
}

export function listDayTasks(dailyPlanId: string): Promise<DailyPlanTask[]> {
  // Both StrictMode copies of Today's load fetch rows for the same shared
  // plan concurrently; share one in-flight command per plan. Refreshes after
  // selection changes are sequential and always re-run.
  return dedupeInFlight(`list_day_tasks:${dailyPlanId}`, () =>
    invoke<DailyPlanTask[]>("list_day_tasks", { dailyPlanId }),
  );
}

export function replanTask(
  dailyPlanTaskId: string,
  decision: ReplanDecision,
  noteMarkdown?: string | null,
): Promise<DailyPlanTask> {
  return invoke<DailyPlanTask>("replan_task", {
    dailyPlanTaskId,
    dto: { decision, note_markdown: noteMarkdown },
  });
}

export function getCapacitySuggestion(
  workspaceId: string,
  date: string,
): Promise<CapacitySuggestion | null> {
  return invoke<CapacitySuggestion | null>("get_capacity_suggestion", {
    workspaceId,
    date,
  });
}
