import { dedupeInFlight, invoke } from "../tauri";
import type { FixedCommitment } from "../types";

export interface CreateFixedCommitmentInput {
  workspace_id: string;
  title: string;
  description_markdown?: string | null;
  commitment_type: string;
  start_at?: string | null;
  end_at?: string | null;
  duration_minutes?: number | null;
  recurrence_rule?: string | null;
  movable: boolean;
  privacy_scope: string;
}

export interface UpdateFixedCommitmentInput {
  title: string;
  description_markdown?: string | null;
  commitment_type: string;
  start_at?: string | null;
  end_at?: string | null;
  duration_minutes?: number | null;
  recurrence_rule?: string | null;
  movable: boolean;
}

export function createFixedCommitment(input: CreateFixedCommitmentInput): Promise<FixedCommitment> {
  return invoke<FixedCommitment>("create_fixed_commitment", { dto: input });
}

export function getFixedCommitment(id: string): Promise<FixedCommitment> {
  return invoke<FixedCommitment>("get_fixed_commitment", { id });
}

export function listFixedCommitments(
  workspaceId: string,
  activeOnly = true,
): Promise<FixedCommitment[]> {
  // Duplicated only by Today's concurrent StrictMode loads; share one
  // in-flight command per workspace+flag. The flag is part of the key so
  // different queries never collide.
  return dedupeInFlight(`list_fixed_commitments:${workspaceId}:${String(activeOnly)}`, () =>
    invoke<FixedCommitment[]>("list_fixed_commitments", {
      workspaceId,
      activeOnly,
    }),
  );
}

export function updateFixedCommitment(
  id: string,
  input: UpdateFixedCommitmentInput,
): Promise<FixedCommitment> {
  return invoke<FixedCommitment>("update_fixed_commitment", { id, dto: input });
}

export function setFixedCommitmentActive(id: string, active: boolean): Promise<FixedCommitment> {
  return invoke<FixedCommitment>("set_fixed_commitment_active", { id, active });
}

export function deleteFixedCommitment(id: string): Promise<void> {
  return invoke<void>("delete_fixed_commitment", { id });
}
