import { invoke } from "../tauri";
import type { AvailabilityProfile } from "../types";

export interface CreateAvailabilityProfileInput {
  workspace_id: string;
  weekday: number;
  start_time: string;
  end_time: string;
  preferred_block_length_minutes?: number | null;
  energy_expectation?: string | null;
  active_from?: string | null;
  active_until?: string | null;
  note_markdown?: string | null;
}

export interface UpdateAvailabilityProfileInput {
  weekday: number;
  start_time: string;
  end_time: string;
  preferred_block_length_minutes?: number | null;
  energy_expectation?: string | null;
  active_from?: string | null;
  active_until?: string | null;
  note_markdown?: string | null;
}

export function createAvailabilityProfile(
  input: CreateAvailabilityProfileInput,
): Promise<AvailabilityProfile> {
  return invoke<AvailabilityProfile>("create_availability_profile", { dto: input });
}

export function getAvailabilityProfile(id: string): Promise<AvailabilityProfile> {
  return invoke<AvailabilityProfile>("get_availability_profile", { id });
}

export function listAvailabilityProfiles(workspaceId: string): Promise<AvailabilityProfile[]> {
  return invoke<AvailabilityProfile[]>("list_availability_profiles", { workspaceId });
}

export function updateAvailabilityProfile(
  id: string,
  input: UpdateAvailabilityProfileInput,
): Promise<AvailabilityProfile> {
  return invoke<AvailabilityProfile>("update_availability_profile", { id, dto: input });
}

export function deleteAvailabilityProfile(id: string): Promise<void> {
  return invoke<void>("delete_availability_profile", { id });
}
