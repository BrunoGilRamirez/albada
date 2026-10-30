import { invoke } from "../tauri";
import type { TherapyCollection, TherapyRecord } from "../types";

export interface CreateTherapyCollectionInput {
  workspace_id: string;
  name: string;
  privacy_scope?: string | null;
  encryption_status?: string | null;
}

export interface UpdateTherapyCollectionInput {
  name: string;
  privacy_scope?: string | null;
  encryption_status?: string | null;
}

export interface CreateTherapyRecordInput {
  therapy_collection_id: string;
  record_type: string;
  title: string;
  content_markdown: string;
  occurred_on?: string | null;
}

export interface UpdateTherapyRecordInput {
  record_type: string;
  title: string;
  content_markdown: string;
  occurred_on?: string | null;
  status: string;
}

export function createTherapyCollection(
  input: CreateTherapyCollectionInput,
): Promise<TherapyCollection> {
  return invoke<TherapyCollection>("create_therapy_collection", { dto: input });
}

export function listTherapyCollections(workspaceId: string): Promise<TherapyCollection[]> {
  return invoke<TherapyCollection[]>("list_therapy_collections", { workspaceId });
}

export function updateTherapyCollection(
  id: string,
  input: UpdateTherapyCollectionInput,
): Promise<TherapyCollection> {
  return invoke<TherapyCollection>("update_therapy_collection", { id, dto: input });
}

export function archiveTherapyCollection(id: string): Promise<TherapyCollection> {
  return invoke<TherapyCollection>("archive_therapy_collection", { id });
}

export function createTherapyRecord(input: CreateTherapyRecordInput): Promise<TherapyRecord> {
  return invoke<TherapyRecord>("create_therapy_record", { dto: input });
}

export function getTherapyRecord(id: string): Promise<TherapyRecord> {
  return invoke<TherapyRecord>("get_therapy_record", { id });
}

export function updateTherapyRecord(
  id: string,
  input: UpdateTherapyRecordInput,
): Promise<TherapyRecord> {
  return invoke<TherapyRecord>("update_therapy_record", { id, dto: input });
}

export function listTherapyRecords(collectionId: string): Promise<TherapyRecord[]> {
  // The Rust command argument `collection_id` crosses the IPC boundary as
  // camelCase (Tauri v2 default mapping); a `collection_id` key would arrive
  // as None and the command rejects with a validation error.
  return invoke<TherapyRecord[]>("list_therapy_records", { collectionId });
}

export function archiveTherapyRecord(id: string): Promise<TherapyRecord> {
  return invoke<TherapyRecord>("archive_therapy_record", { id });
}
