import { invoke } from "../tauri";
import type { TaskSession } from "../types";

export interface CreateSessionInput {
  task_id: string;
  block_id?: string | null;
  daily_plan_task_id?: string | null;
}

export interface FinishSessionInput {
  outcome_markdown?: string | null;
  next_step_markdown?: string | null;
}

export function createSession(input: CreateSessionInput): Promise<TaskSession> {
  return invoke<TaskSession>("create_session", { dto: input });
}

export function getSession(id: string): Promise<TaskSession> {
  return invoke<TaskSession>("get_session", { id });
}

export function startSession(id: string): Promise<TaskSession> {
  return invoke<TaskSession>("start_session_cmd", { id });
}

export function pauseSession(id: string): Promise<TaskSession> {
  return invoke<TaskSession>("pause_session_cmd", { id });
}

export function finishSession(id: string, input: FinishSessionInput): Promise<TaskSession> {
  return invoke<TaskSession>("finish_session_cmd", { id, dto: input });
}

export function skipSession(id: string): Promise<TaskSession> {
  return invoke<TaskSession>("skip_session_cmd", { id });
}

export function listSessionsForTask(taskId: string): Promise<TaskSession[]> {
  return invoke<TaskSession[]>("list_sessions_for_task", { taskId });
}

export function listSessionsForBlock(blockId: string): Promise<TaskSession[]> {
  return invoke<TaskSession[]>("list_sessions_for_block", { blockId });
}

export function listSessionsForDayTask(dailyPlanTaskId: string): Promise<TaskSession[]> {
  return invoke<TaskSession[]>("list_sessions_for_day_task", { dailyPlanTaskId });
}
