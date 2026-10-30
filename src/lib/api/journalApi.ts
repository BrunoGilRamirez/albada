import { invoke } from "../tauri";
import type { JournalEntry, JournalTemplateValues } from "../types";

export interface CreateJournalEntryInput {
  workspace_id: string;
  occurred_on: string;
  title: string;
  content_markdown: string;
  template_id?: string | null;
  template_version?: string | null;
  template_values?: JournalTemplateValues | null;
  mood_label?: string | null;
  energy_label?: string | null;
  emotions?: string[] | null;
  export_eligible?: boolean | null;
  source?: string | null;
}

export interface UpdateJournalEntryInput {
  occurred_on: string;
  title: string;
  content_markdown: string;
  template_id?: string | null;
  template_version?: string | null;
  template_values?: JournalTemplateValues | null;
  mood_label?: string | null;
  energy_label?: string | null;
  emotions?: string[] | null;
  export_eligible?: boolean | null;
}

export function createJournalEntry(input: CreateJournalEntryInput): Promise<JournalEntry> {
  return invoke<JournalEntry>("create_journal_entry", { dto: input });
}

export function getJournalEntry(id: string): Promise<JournalEntry> {
  return invoke<JournalEntry>("get_journal_entry", { id });
}

export function updateJournalEntry(
  id: string,
  input: UpdateJournalEntryInput,
): Promise<JournalEntry> {
  return invoke<JournalEntry>("update_journal_entry", { id, dto: input });
}

export function listJournalEntries(
  workspaceId: string,
  occurredOnFrom?: string,
  occurredOnTo?: string,
): Promise<JournalEntry[]> {
  return invoke<JournalEntry[]>("list_journal_entries", {
    workspaceId,
    occurredOnFrom: occurredOnFrom ?? null,
    occurredOnTo: occurredOnTo ?? null,
  });
}

export function archiveJournalEntry(id: string): Promise<JournalEntry> {
  return invoke<JournalEntry>("archive_journal_entry", { id });
}

/**
 * Batch-archive journal entries (Phase 8.6.5 bulk actions). The native command
 * runs one all-or-nothing transaction: either every listed id is archived or
 * nothing changes and the rejection carries the failure. Returns the number of
 * archived entries (count only; never entry content). Tauri v2 maps the Rust
 * snake_case arguments to these camelCase payload keys by default.
 */
export function archiveJournalEntries(workspaceId: string, entryIds: string[]): Promise<number> {
  return invoke<number>("archive_journal_entries", { workspaceId, entryIds });
}

/**
 * Batch-set export eligibility for journal entries (Phase 8.6.5 bulk
 * actions). Same all-or-nothing transaction contract as
 * `archiveJournalEntries`; returns the number of updated entries (count
 * only). Setting eligibility never changes an entry's archived state and all
 * entries stay local/private.
 */
export function setJournalEntriesExportEligible(
  workspaceId: string,
  entryIds: string[],
  exportEligible: boolean,
): Promise<number> {
  return invoke<number>("set_journal_entries_export_eligible", {
    workspaceId,
    entryIds,
    exportEligible,
  });
}
