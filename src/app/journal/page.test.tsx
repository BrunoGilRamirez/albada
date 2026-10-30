import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { MemoryRouter } from "react-router";
import JournalPage from "@/app/journal/page";
import {
  archiveJournalEntries,
  listJournalEntries,
  setJournalEntriesExportEligible,
} from "@/lib/api/journalApi";
import { ensureDefaultWorkspace } from "@/lib/api/workspaceApi";
import type { JournalEntry } from "@/lib/types";

vi.mock("@/lib/api/journalApi", () => ({
  listJournalEntries: vi.fn(),
  archiveJournalEntries: vi.fn(),
  setJournalEntriesExportEligible: vi.fn(),
}));

vi.mock("@/lib/api/workspaceApi", () => ({
  ensureDefaultWorkspace: vi.fn(),
}));

/**
 * Phase 8.6.5 Journal list and bulk actions (UI slice). Synthetic fixtures
 * only; no personal, journal, or therapy content is used anywhere in this
 * file. Archived rows never reach this list (the native list command excludes
 * them), so archived/not-current-list selection scoping is pinned by the
 * batch-success path: archived ids leave the list and the selection together,
 * and a list change prunes any selected id that is no longer listed.
 */
function syntheticEntry(id: string, title: string, exportEligible: boolean): JournalEntry {
  return {
    id,
    workspace_id: "w1",
    occurred_on: "2026-10-02",
    title,
    content_markdown: "Synthetic body.",
    template_id: null,
    template_version: null,
    template_values: {},
    mood_label: null,
    energy_label: null,
    emotions: [],
    export_eligible: exportEligible,
    source: "user",
    created_at: "2026-10-02T09:00:00Z",
    updated_at: "2026-10-02T09:00:00Z",
    archived_at: null,
  };
}

const alpha = (): JournalEntry => syntheticEntry("j1", "Synthetic alpha", true);
const beta = (): JournalEntry => syntheticEntry("j2", "Synthetic beta", false);

function renderPage() {
  return render(
    <MemoryRouter initialEntries={["/journal"]}>
      <JournalPage />
    </MemoryRouter>,
  );
}

describe("JournalPage bulk actions (8.6.5)", () => {
  beforeEach(() => {
    vi.mocked(ensureDefaultWorkspace).mockReset();
    vi.mocked(listJournalEntries).mockReset();
    vi.mocked(archiveJournalEntries).mockReset();
    vi.mocked(setJournalEntriesExportEligible).mockReset();

    vi.mocked(ensureDefaultWorkspace).mockResolvedValue({ id: "w1" } as never);
    vi.mocked(listJournalEntries).mockResolvedValue([alpha(), beta()]);
    vi.mocked(archiveJournalEntries).mockResolvedValue(2);
    vi.mocked(setJournalEntriesExportEligible).mockResolvedValue(2);
    vi.spyOn(window, "confirm").mockReturnValue(true);
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  it("toggles individual row selection and reflects it in the action controls", async () => {
    renderPage();

    const first = await screen.findByRole("checkbox", { name: "Select Synthetic alpha" });
    const second = screen.getByRole("checkbox", { name: "Select Synthetic beta" });
    expect(first).not.toBeChecked();
    expect(screen.getByRole("button", { name: "Archive selected" })).toBeDisabled();

    fireEvent.click(first);
    expect(first).toBeChecked();
    expect(screen.getByText("1 selected")).toBeTruthy();
    expect(screen.getByRole("button", { name: "Archive selected" })).toBeEnabled();
    expect(screen.getByRole("button", { name: "Set Exportable" })).toBeEnabled();
    expect(screen.getByRole("button", { name: "Set Private" })).toBeEnabled();

    fireEvent.click(first);
    expect(first).not.toBeChecked();
    expect(screen.getByRole("button", { name: "Archive selected" })).toBeDisabled();
    expect(second).not.toBeChecked();
  });

  it("selects and deselects every currently listed row", async () => {
    renderPage();

    await screen.findByRole("checkbox", { name: "Select Synthetic alpha" });
    fireEvent.click(screen.getByRole("button", { name: "Select all" }));

    expect(screen.getByRole("checkbox", { name: "Select Synthetic alpha" })).toBeChecked();
    expect(screen.getByRole("checkbox", { name: "Select Synthetic beta" })).toBeChecked();
    expect(screen.getByText("2 selected")).toBeTruthy();

    fireEvent.click(screen.getByRole("button", { name: "Deselect all" }));

    expect(screen.getByRole("checkbox", { name: "Select Synthetic alpha" })).not.toBeChecked();
    expect(screen.getByRole("checkbox", { name: "Select Synthetic beta" })).not.toBeChecked();
    expect(screen.getByRole("button", { name: "Archive selected" })).toBeDisabled();
  });

  it("skips archiving when the confirmation is cancelled", async () => {
    vi.mocked(window.confirm).mockReturnValue(false);
    renderPage();

    fireEvent.click(await screen.findByRole("button", { name: "Select all" }));
    fireEvent.click(screen.getByRole("button", { name: "Archive selected" }));

    expect(window.confirm).toHaveBeenCalledWith("Archive 2 selected journal entries?");
    expect(archiveJournalEntries).not.toHaveBeenCalled();
    // Cancelled confirmation: list and selection stay untouched.
    expect(screen.getByRole("checkbox", { name: "Select Synthetic alpha" })).toBeChecked();
    expect(screen.getByRole("checkbox", { name: "Select Synthetic beta" })).toBeChecked();
    expect(screen.getByText("Synthetic beta")).toBeTruthy();
  });

  it("archives the confirmed selection and updates the list only after success", async () => {
    renderPage();

    fireEvent.click(await screen.findByRole("button", { name: "Select all" }));
    fireEvent.click(screen.getByRole("button", { name: "Archive selected" }));

    await waitFor(() => expect(archiveJournalEntries).toHaveBeenCalledWith("w1", ["j1", "j2"]));
    expect(archiveJournalEntries).toHaveBeenCalledTimes(1);
    expect(await screen.findByText("Archived 2 journal entries.")).toBeTruthy();
    // Both rows left the list and the selection was cleared with them; the
    // empty state is shown because the native list excludes archived rows.
    expect(screen.queryByText("Synthetic alpha")).toBeNull();
    expect(screen.queryByText("Synthetic beta")).toBeNull();
    expect(
      screen.getByText("No journal entries yet. Write your first entry for today."),
    ).toBeTruthy();
    expect(screen.queryByText("2 selected")).toBeNull();
  });

  it("preserves list and selection when archive fails", async () => {
    vi.mocked(archiveJournalEntries).mockRejectedValue({
      code: "DATABASE",
      message: "journal batch archive failed",
    } as never);
    renderPage();

    fireEvent.click(await screen.findByRole("button", { name: "Select all" }));
    fireEvent.click(screen.getByRole("button", { name: "Archive selected" }));

    await waitFor(() => expect(archiveJournalEntries).toHaveBeenCalledTimes(1));
    expect((await screen.findByRole("alert")).textContent).toBe(
      "Your local data could not be saved. Please try again.",
    );
    // Atomic failure: nothing was partially applied; rows and selection stay.
    expect(screen.getByText("Synthetic alpha")).toBeTruthy();
    expect(screen.getByText("Synthetic beta")).toBeTruthy();
    expect(screen.getByRole("checkbox", { name: "Select Synthetic alpha" })).toBeChecked();
    expect(screen.getByRole("checkbox", { name: "Select Synthetic beta" })).toBeChecked();
    expect(screen.queryByText(/Archived/)).toBeNull();
  });

  it("marks the selected entries Exportable after success", async () => {
    renderPage();

    // Only the private beta row is selected, so the alpha badge must not move.
    fireEvent.click(await screen.findByRole("checkbox", { name: "Select Synthetic beta" }));
    vi.mocked(setJournalEntriesExportEligible).mockResolvedValue(1);
    fireEvent.click(screen.getByRole("button", { name: "Set Exportable" }));

    await waitFor(() =>
      expect(setJournalEntriesExportEligible).toHaveBeenCalledWith("w1", ["j2"], true),
    );
    expect(await screen.findByText("Marked 1 journal entry as Exportable.")).toBeTruthy();
    expect(screen.getAllByText("Exportable")).toHaveLength(2);
    expect(screen.queryByText("Private")).toBeNull();
    expect(screen.getByRole("checkbox", { name: "Select Synthetic beta" })).not.toBeChecked();
  });

  it("marks the selected entries Private and clears the selection after success", async () => {
    renderPage();

    fireEvent.click(await screen.findByRole("button", { name: "Select all" }));
    fireEvent.click(screen.getByRole("button", { name: "Set Private" }));

    await waitFor(() =>
      expect(setJournalEntriesExportEligible).toHaveBeenCalledWith("w1", ["j1", "j2"], false),
    );
    expect(await screen.findByText("Marked 2 journal entries as Private.")).toBeTruthy();
    // Both rows now read Private; the exportable badge is gone.
    expect(screen.getAllByText("Private")).toHaveLength(2);
    expect(screen.queryByText("Exportable")).toBeNull();
    expect(screen.getByRole("checkbox", { name: "Select Synthetic alpha" })).not.toBeChecked();
    expect(screen.getByRole("button", { name: "Set Private" })).toBeDisabled();
  });

  it("preserves badges and selection when the eligibility update fails", async () => {
    vi.mocked(setJournalEntriesExportEligible).mockRejectedValue({
      code: "VALIDATION",
      message: "entry id is not part of the active workspace",
    } as never);
    renderPage();

    fireEvent.click(await screen.findByRole("checkbox", { name: "Select Synthetic alpha" }));
    fireEvent.click(screen.getByRole("button", { name: "Set Private" }));

    await waitFor(() => expect(setJournalEntriesExportEligible).toHaveBeenCalledTimes(1));
    expect((await screen.findByRole("alert")).textContent).toBe(
      "Please check the information and try again.",
    );
    // No partial-success UI: the alpha row keeps its Exportable badge and the
    // selection stays for a retry.
    expect(screen.getByText("Synthetic alpha")).toBeTruthy();
    expect(screen.getAllByText("Exportable")).toHaveLength(1);
    expect(screen.getAllByText("Private")).toHaveLength(1);
    expect(screen.getByRole("checkbox", { name: "Select Synthetic alpha" })).toBeChecked();
    expect(screen.queryByText(/Marked/)).toBeNull();
  });
});
