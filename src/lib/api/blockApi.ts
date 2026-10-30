import { dedupeInFlight, invoke } from "../tauri";
import type {
  Block,
  BlockRecurrence,
  RecurrenceAnalyticsResponse,
  ScheduledBlockRecurrence,
} from "../types";

export interface CreateBlockInput {
  workspace_id: string;
  daily_plan_id?: string | null;
  date: string;
  block_type: string;
  label?: string | null;
  planned_start_at?: string | null;
  estimated_duration_seconds?: number | null;
  /** @deprecated Legacy UI compatibility; the native command rejects non-null values. */
  planned_end_at?: string | null;
  time_window_label?: string | null;
  flexibility: string;
  note_markdown?: string | null;
  task_id?: string | null;
}

export interface UpdateBlockInput {
  daily_plan_id?: string | null;
  date: string;
  block_type: string;
  label?: string | null;
  planned_start_at?: string | null;
  estimated_duration_seconds?: number | null;
  time_window_label?: string | null;
  flexibility: string;
  note_markdown?: string | null;
  task_id?: string | null;
}

export interface BlockRecurrenceInput {
  workspace_id: string;
  weekdays: number[];
  local_start_time: string;
  estimated_duration_seconds: number;
  block_type: string;
  label?: string | null;
  flexibility: string;
  note_markdown?: string | null;
  enabled: boolean;
  effective_from: string;
  effective_until?: string | null;
  task_id?: string | null;
}

export function createBlock(input: CreateBlockInput): Promise<Block> {
  return invoke<Block>("create_block", { dto: input });
}

export function updateBlock(id: string, input: UpdateBlockInput): Promise<Block> {
  return invoke<Block>("update_block", { id, dto: input });
}

export function createBlockRecurrence(input: BlockRecurrenceInput): Promise<BlockRecurrence> {
  return invoke<BlockRecurrence>("create_block_recurrence", { dto: input });
}

export function updateBlockRecurrence(
  recurrenceId: string,
  input: BlockRecurrenceInput,
): Promise<BlockRecurrence> {
  return invoke<BlockRecurrence>("update_block_recurrence", {
    recurrenceId,
    dto: input,
  });
}

export function getBlockRecurrence(
  workspaceId: string,
  recurrenceId: string,
): Promise<BlockRecurrence> {
  return invoke<BlockRecurrence>("get_block_recurrence", { workspaceId, recurrenceId });
}

export function getBlock(id: string): Promise<Block> {
  return invoke<Block>("get_block", { id });
}

export function startBlock(id: string): Promise<Block> {
  return invoke<Block>("start_block_cmd", { id });
}

export function completeBlock(id: string): Promise<Block> {
  return invoke<Block>("complete_block_cmd", { id });
}

export function skipBlock(id: string): Promise<Block> {
  return invoke<Block>("skip_block_cmd", { id });
}

export function cancelBlock(id: string): Promise<Block> {
  return invoke<Block>("cancel_block_cmd", { id });
}

export function listBlocksForDate(workspaceId: string, date: string): Promise<Block[]> {
  // Duplicated only by Today's concurrent StrictMode loads; share one
  // in-flight command per workspace+date. Sequential edits stay fresh.
  return dedupeInFlight(`list_blocks_for_date:${workspaceId}:${date}`, () =>
    invoke<Block[]>("list_blocks_for_date", { workspaceId, date }),
  );
}

export function listBlocksForPlan(dailyPlanId: string): Promise<Block[]> {
  return invoke<Block[]>("list_blocks_for_plan", { dailyPlanId });
}

export function listBlocksBetween(
  workspaceId: string,
  fromDate: string,
  toDate: string,
): Promise<Block[]> {
  return invoke<Block[]>("list_blocks_between", { workspaceId, fromDate, toDate });
}

/** Deactivate an active one-time block (soft lifecycle change, no deletion). */
export function deactivateBlock(workspaceId: string, id: string): Promise<Block> {
  return invoke<Block>("deactivate_block_cmd", { workspaceId, id });
}

export interface ReactivateBlockInput {
  workspace_id: string;
  block_id: string;
  /** `"once"` creates one new occurrence; `"recurring"` creates a weekly series. */
  schedule: "once" | "recurring";
  /** ISO weekday 1..7 (Monday..Sunday); resolves to the next matching local date. */
  weekday: number;
  local_start_time?: string | null;
  estimated_duration_seconds?: number | null;
}

export interface ReactivateBlockResult {
  block: Block | null;
  recurrence: BlockRecurrence | null;
}

export function reactivateBlock(input: ReactivateBlockInput): Promise<ReactivateBlockResult> {
  return invoke<ReactivateBlockResult>("reactivate_block_cmd", { dto: input });
}

/** Soft-delete a one-time block (retains history; hidden from ordinary views). */
export function deleteBlock(workspaceId: string, id: string): Promise<Block> {
  return invoke<Block>("delete_block_cmd", { workspaceId, id });
}

export function deactivateBlockRecurrence(
  workspaceId: string,
  recurrenceId: string,
): Promise<BlockRecurrence> {
  return invoke<BlockRecurrence>("deactivate_block_recurrence_cmd", {
    workspaceId,
    recurrenceId,
  });
}

export function reactivateBlockRecurrence(
  workspaceId: string,
  recurrenceId: string,
): Promise<BlockRecurrence> {
  return invoke<BlockRecurrence>("reactivate_block_recurrence_cmd", {
    workspaceId,
    recurrenceId,
  });
}

export function deleteBlockRecurrence(
  workspaceId: string,
  recurrenceId: string,
): Promise<BlockRecurrence> {
  return invoke<BlockRecurrence>("delete_block_recurrence_cmd", {
    workspaceId,
    recurrenceId,
  });
}

/** One-time blocks with lifecycle `inactive` (terminal or manually deactivated). */
export function listInactiveBlocks(workspaceId: string): Promise<Block[]> {
  return invoke<Block[]>("list_inactive_blocks_cmd", { workspaceId });
}

/** Recurrence series with lifecycle `inactive` (generation suppressed). */
export function listInactiveBlockRecurrences(workspaceId: string): Promise<BlockRecurrence[]> {
  return invoke<BlockRecurrence[]>("list_inactive_block_recurrences_cmd", { workspaceId });
}

/** Active recurrence rules shown once, without materializing future dates. */
export function listScheduledBlockRecurrences(
  workspaceId: string,
): Promise<ScheduledBlockRecurrence[]> {
  return invoke<ScheduledBlockRecurrence[]>("list_scheduled_block_recurrences_cmd", {
    workspaceId,
  });
}

/**
 * Page through upcoming active scheduled occurrences in a bounded local-date
 * range. Each requested range materializes only its own dates; the UI pages
 * forward by dates, so there is no fixed lookahead cutoff.
 */
export function listUpcomingBlocks(
  workspaceId: string,
  fromDate: string,
  toDate: string,
  limit: number,
  offset: number,
): Promise<Block[]> {
  return invoke<Block[]>("list_upcoming_blocks_cmd", {
    workspaceId,
    fromDate,
    toDate,
    limit,
    offset,
  });
}

/**
 * Read-only habit analytics for recurring block series (8.7.12). The native
 * command clamps `toDate` to the workspace-local today and never materializes
 * an occurrence, so this is safe to call for any bounded range.
 */
export function getBlockRecurrenceAnalytics(
  workspaceId: string,
  fromDate: string,
  toDate: string,
): Promise<RecurrenceAnalyticsResponse> {
  return invoke<RecurrenceAnalyticsResponse>("get_block_recurrence_analytics_cmd", {
    workspaceId,
    fromDate,
    toDate,
  });
}
