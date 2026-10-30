import { beforeEach, describe, expect, it, vi } from "vitest";

const { invokeMock, isTauriMock } = vi.hoisted(() => ({
  invokeMock: vi.fn(),
  isTauriMock: vi.fn(),
}));

vi.mock("@tauri-apps/api/core", () => ({
  invoke: invokeMock,
  isTauri: isTauriMock,
}));

import {
  createTherapyRecord,
  listTherapyCollections,
  listTherapyRecords,
  updateTherapyCollection,
} from "@/lib/api/therapyApi";
import type { TherapyCollection, TherapyRecord } from "@/lib/types";

/**
 * IPC payload contract for the therapy typed boundary (Phase 8.6.0).
 *
 * Tauri v2 maps Rust snake_case command arguments to camelCase payload keys
 * by default. `list_therapy_records(db, collection_id: Option<String>)`
 * therefore expects `{ collectionId }`; sending `{ collection_id }` leaves
 * the Rust argument `None` and the command answers the validation error
 * "collection_id is required to list therapy records." The add-record flow
 * broke exactly that way: the record was created, but the refreshed
 * collection page could never list it. These tests pin the wire payload
 * after the fix so the mismatch cannot silently return.
 *
 * Object-graph DTO commands (`create_therapy_record`,
 * `update_therapy_collection`) keep their nested snake_case fields because
 * the Rust side deserializes them through serde DTOs with snake_case names.
 *
 * Synthetic fixtures only; `isTauri` is false so the diagnostics channel
 * (first-invoke marking, startup reports) stays silent.
 */
const syntheticCollection: TherapyCollection = {
  id: "c1",
  workspace_id: "w1",
  name: "Synthetic collection",
  privacy_scope: "therapy_private",
  encryption_status: null,
  created_at: "2026-10-13T09:00:00Z",
  updated_at: "2026-10-13T09:00:00Z",
  archived_at: null,
};

const syntheticRecord: TherapyRecord = {
  id: "tr1",
  therapy_collection_id: "c1",
  record_type: "note",
  title: "Synthetic note",
  content_markdown: "Synthetic markdown body.",
  occurred_on: null,
  status: "open",
  privacy_scope: "therapy_private",
  created_at: "2026-10-13T09:00:00Z",
  updated_at: "2026-10-13T09:00:00Z",
  archived_at: null,
};

describe("therapyApi command payloads at the typed Tauri boundary", () => {
  beforeEach(() => {
    invokeMock.mockReset();
    isTauriMock.mockReset();
    isTauriMock.mockReturnValue(false);
  });

  it("lists records for one collection with the camelCase collectionId key", async () => {
    invokeMock.mockResolvedValue([syntheticRecord]);

    const records = await listTherapyRecords("c1");

    expect(invokeMock).toHaveBeenCalledTimes(1);
    expect(invokeMock).toHaveBeenCalledWith("list_therapy_records", { collectionId: "c1" });
    expect(records).toEqual([syntheticRecord]);
  });

  it("creates a record through the snake_case dto object unchanged", async () => {
    invokeMock.mockResolvedValue(syntheticRecord);

    await createTherapyRecord({
      therapy_collection_id: "c1",
      record_type: "note",
      title: "Synthetic note",
      content_markdown: "Synthetic markdown body.",
      occurred_on: null,
    });

    expect(invokeMock).toHaveBeenCalledTimes(1);
    expect(invokeMock).toHaveBeenCalledWith("create_therapy_record", {
      dto: {
        therapy_collection_id: "c1",
        record_type: "note",
        title: "Synthetic note",
        content_markdown: "Synthetic markdown body.",
        occurred_on: null,
      },
    });
  });

  it("updates a collection with the id plus dto payload shape the command expects", async () => {
    invokeMock.mockResolvedValue(syntheticCollection);

    await updateTherapyCollection("c1", { name: "Renamed synthetic collection" });

    expect(invokeMock).toHaveBeenCalledTimes(1);
    expect(invokeMock).toHaveBeenCalledWith("update_therapy_collection", {
      id: "c1",
      dto: { name: "Renamed synthetic collection" },
    });
  });

  it("lists collections with the camelCase workspaceId key", async () => {
    invokeMock.mockResolvedValue([syntheticCollection]);

    await listTherapyCollections("w1");

    expect(invokeMock).toHaveBeenCalledTimes(1);
    expect(invokeMock).toHaveBeenCalledWith("list_therapy_collections", { workspaceId: "w1" });
  });
});
