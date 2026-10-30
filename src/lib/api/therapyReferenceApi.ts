import { invoke } from "../tauri";
import type { TherapyTaskReference } from "../types";

// The Rust command arguments `record_id`, `task_id`, and `note_markdown`
// cross the IPC boundary as camelCase (Tauri v2 default mapping); a
// snake_case key would arrive as None and the command rejects with a
// validation error.
export function addTherapyReference(
  recordId: string,
  taskId: string,
  relation: string,
  noteMarkdown?: string | null,
): Promise<TherapyTaskReference> {
  return invoke<TherapyTaskReference>("add_therapy_reference", {
    recordId,
    taskId,
    relation,
    noteMarkdown,
  });
}

export function removeTherapyReference(id: string): Promise<void> {
  return invoke<void>("remove_therapy_reference", { id });
}

export function listReferencesForRecord(recordId: string): Promise<TherapyTaskReference[]> {
  return invoke<TherapyTaskReference[]>("list_references_for_record", { recordId });
}
