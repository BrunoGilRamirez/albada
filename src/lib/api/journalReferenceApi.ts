import { invoke } from "../tauri";
import type {
  JournalReferenceEntityType,
  JournalReferenceRelation,
  JournalTaskReference,
} from "../types";

export function addJournalReference(
  entryId: string,
  entityType: JournalReferenceEntityType,
  entityId: string,
  relation: JournalReferenceRelation,
): Promise<JournalTaskReference> {
  return invoke<JournalTaskReference>("add_journal_reference", {
    entryId,
    entityType,
    entityId,
    relation,
  });
}

export function removeJournalReference(id: string): Promise<void> {
  return invoke<void>("remove_journal_reference", { id });
}

export function listReferencesForEntry(entryId: string): Promise<JournalTaskReference[]> {
  return invoke<JournalTaskReference[]>("list_references_for_entry", { entryId });
}
