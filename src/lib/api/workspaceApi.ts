import { dedupeInFlight, invoke } from "../tauri";
import type { WorkspaceLocalePreference } from "../i18n/locale";
import type { BackupReminderPreference, Workspace } from "../types";

export interface CreateWorkspaceInput {
  name: string;
}

export interface UpdateWorkspaceInput {
  name: string;
}

export interface UpdateBackupReminderPreferenceInput {
  enabled: boolean;
  interval_days: number;
}

export function createWorkspace(input: CreateWorkspaceInput): Promise<Workspace> {
  return invoke<Workspace>("create_workspace", { dto: input });
}

export function getWorkspace(id: string): Promise<Workspace> {
  return invoke<Workspace>("get_workspace", { id });
}

export function listWorkspaces(): Promise<Workspace[]> {
  return invoke<Workspace[]>("list_workspaces");
}

export function updateWorkspace(id: string, input: UpdateWorkspaceInput): Promise<Workspace> {
  return invoke<Workspace>("update_workspace", { id, dto: input });
}

export function ensureDefaultWorkspace(): Promise<Workspace> {
  // Startup fan-out (StrictMode double-effects, Today + NotificationBell)
  // must resolve the default workspace through exactly one native command;
  // concurrent callers share the in-flight promise, later calls re-run.
  return dedupeInFlight("ensure_default_workspace", () =>
    invoke<Workspace>("ensure_default_workspace"),
  );
}

/**
 * Persist the workspace locale preference (`system` or a supported tag) and
 * return the updated workspace. Rust validates the exact value set and the
 * frontend applies the resulting locale separately via i18n.
 */
export function updateWorkspaceLocale(
  workspaceId: string,
  locale: WorkspaceLocalePreference,
): Promise<Workspace> {
  return invoke<Workspace>("update_workspace_locale", { workspaceId, locale });
}

export function getWarningWindow(workspaceId: string): Promise<number> {
  return invoke<number>("get_warning_window", { workspaceId });
}

export function updateWarningWindow(workspaceId: string, days: number): Promise<Workspace> {
  return invoke<Workspace>("update_warning_window", { workspaceId, days });
}

export function getBackupReminderPreference(
  workspaceId: string,
): Promise<BackupReminderPreference> {
  return invoke<BackupReminderPreference>("get_backup_reminder_preference", { workspaceId });
}

export function updateBackupReminderPreference(
  workspaceId: string,
  input: UpdateBackupReminderPreferenceInput,
): Promise<BackupReminderPreference> {
  return invoke<BackupReminderPreference>("update_backup_reminder_preference", {
    workspaceId,
    dto: input,
  });
}
