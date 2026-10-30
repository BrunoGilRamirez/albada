import { beforeEach, describe, expect, it, vi } from "vitest";

const { invokeMock, isTauriMock } = vi.hoisted(() => ({
  invokeMock: vi.fn(),
  isTauriMock: vi.fn(),
}));

vi.mock("@tauri-apps/api/core", () => ({
  invoke: invokeMock,
  isTauri: isTauriMock,
}));

import { archiveJournalEntries, setJournalEntriesExportEligible } from "@/lib/api/journalApi";

/**
 * IPC payload contract for the journal batch boundary (Phase 8.6.5 bulk
 * actions).
 *
 * Tauri v2 maps Rust snake_case command arguments to camelCase payload keys
 * by default, so `archive_journal_entries(db, workspace_id: String, entry_ids:
 * Vec<String>)` expects `{ workspaceId, entryIds }` and
 * `set_journal_entries_export_eligible(..., workspace_id, entry_ids,
 * export_eligible)` expects `{ workspaceId, entryIds, exportEligible }`.
 * A wrong key casing would leave the Rust arguments unset, exactly the
 * failure mode the therapy boundary hit in 8.6.0. Both commands run
 * all-or-nothing native transactions and resolve to a plain count (number);
 * these tests pin the wire payload and the count pass-through so the UI can
 * treat any rejection as "nothing changed".
 *
 * Synthetic fixtures only; `isTauri` is false so the diagnostics channel
 * (first-invoke marking, startup reports) stays silent.
 */
describe("journalApi batch command payloads at the typed Tauri boundary", () => {
  beforeEach(() => {
    invokeMock.mockReset();
    isTauriMock.mockReset();
    isTauriMock.mockReturnValue(false);
  });

  it("archives entries with the exact camelCase batch payload and count", async () => {
    invokeMock.mockResolvedValue(2);

    const count = await archiveJournalEntries("w1", ["j1", "j2"]);

    expect(invokeMock).toHaveBeenCalledTimes(1);
    expect(invokeMock).toHaveBeenCalledWith("archive_journal_entries", {
      workspaceId: "w1",
      entryIds: ["j1", "j2"],
    });
    expect(count).toBe(2);
  });

  it("passes the entry id list through unchanged (order and length preserved)", async () => {
    invokeMock.mockResolvedValue(3);
    const ids = ["j3", "j1", "j2"];

    await archiveJournalEntries("w1", ids);

    expect(invokeMock).toHaveBeenCalledWith("archive_journal_entries", {
      workspaceId: "w1",
      entryIds: ["j3", "j1", "j2"],
    });
  });

  it("sets export eligibility true with the exact camelCase payload", async () => {
    invokeMock.mockResolvedValue(1);

    const count = await setJournalEntriesExportEligible("w1", ["j1"], true);

    expect(invokeMock).toHaveBeenCalledTimes(1);
    expect(invokeMock).toHaveBeenCalledWith("set_journal_entries_export_eligible", {
      workspaceId: "w1",
      entryIds: ["j1"],
      exportEligible: true,
    });
    expect(count).toBe(1);
  });

  it("sets export eligibility false with the exact camelCase payload", async () => {
    invokeMock.mockResolvedValue(2);

    const count = await setJournalEntriesExportEligible("w1", ["j1", "j2"], false);

    expect(invokeMock).toHaveBeenCalledTimes(1);
    expect(invokeMock).toHaveBeenCalledWith("set_journal_entries_export_eligible", {
      workspaceId: "w1",
      entryIds: ["j1", "j2"],
      exportEligible: false,
    });
    expect(count).toBe(2);
  });
});
