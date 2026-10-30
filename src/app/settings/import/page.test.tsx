import { describe, expect, it, vi, beforeEach } from "vitest";
import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import ImportSettingsPage from "@/app/settings/import/page";
import {
  commitLegacyImport,
  getImportBatch,
  getImportBatchStatus,
  previewLegacyImport,
  startLegacyImport,
} from "@/lib/api/importApi";
import { ensureDefaultWorkspace } from "@/lib/api/workspaceApi";

vi.mock("@/lib/api/importApi", () => ({
  previewLegacyImport: vi.fn(),
  startLegacyImport: vi.fn(),
  commitLegacyImport: vi.fn(),
  getImportBatchStatus: vi.fn(),
  getImportBatch: vi.fn(),
}));

vi.mock("@/lib/api/workspaceApi", () => ({
  ensureDefaultWorkspace: vi.fn(),
}));

const report = {
  batch_id: "batch-1",
  source_root: "C:\\legacy\\proyecto-vida",
  status: "completed",
  started_at: "2026-09-06T10:00:00Z",
  completed_at: "2026-09-06T10:00:05Z",
  counts: [],
  warnings: [],
  review_items: [],
  preview_fingerprint: "fp-1",
};

const reviewReport = {
  ...report,
  status: "preview",
  review_items: [
    {
      id: "item-1",
      source_path: "journal/entries/2026-06-25-raw.md",
      proposed_entity_type: "journal_entry",
      title: "Raw entry",
      snippet: "safe snippet",
      confidence: "review",
      source_hash: "ab".repeat(32),
    },
  ],
};

describe("ImportSettingsPage", () => {
  beforeEach(() => {
    vi.mocked(previewLegacyImport).mockReset();
    vi.mocked(startLegacyImport).mockReset();
    vi.mocked(commitLegacyImport).mockReset();
    vi.mocked(getImportBatchStatus).mockReset();
    vi.mocked(getImportBatch).mockReset();
    vi.mocked(ensureDefaultWorkspace).mockReset();
    vi.mocked(previewLegacyImport).mockResolvedValue(report as never);
    vi.mocked(ensureDefaultWorkspace).mockResolvedValue({ id: "w1" } as never);
    vi.mocked(startLegacyImport).mockResolvedValue({
      batch_id: "batch-1",
      status: "running",
      started_at: report.started_at,
      completed_at: null,
    });
    vi.mocked(getImportBatchStatus).mockResolvedValue({
      batch_id: "batch-1",
      status: "completed",
      started_at: report.started_at,
      completed_at: report.completed_at,
    });
    vi.mocked(getImportBatch).mockResolvedValue(report as never);
  });

  it("defaults to scanning the selected repository root", () => {
    render(<ImportSettingsPage />);

    expect(screen.getByLabelText(/Import the full repository/)).toBeChecked();
    expect(screen.queryByLabelText(/Include only these paths/)).toBeNull();
  });

  it("passes trimmed non-empty include paths when not doing a full scan", async () => {
    render(<ImportSettingsPage />);

    fireEvent.click(screen.getByLabelText(/Import the full repository/));

    fireEvent.change(screen.getByLabelText(/Legacy repository root path/), {
      target: { value: "C:\\legacy\\proyecto-vida" },
    });
    fireEvent.change(screen.getByLabelText(/Include only these paths/), {
      target: {
        value: "  custom-journal  \n\ncustom-therapy\n",
      },
    });

    fireEvent.click(screen.getByRole("button", { name: /Preview import/ }));

    expect(previewLegacyImport).toHaveBeenCalledWith("C:\\legacy\\proyecto-vida", [
      "custom-journal",
      "custom-therapy",
    ]);
  });

  it("passes null include paths for a full repository scan", async () => {
    render(<ImportSettingsPage />);

    fireEvent.change(screen.getByLabelText(/Legacy repository root path/), {
      target: { value: "C:\\legacy\\proyecto-vida" },
    });
    expect(screen.queryByLabelText(/Include only these paths/)).toBeNull();

    fireEvent.click(screen.getByRole("button", { name: /Preview import/ }));

    expect(previewLegacyImport).toHaveBeenCalledWith("C:\\legacy\\proyecto-vida", null);
  });

  it("requires preview and per-item decisions before committing the import", async () => {
    vi.mocked(previewLegacyImport).mockResolvedValue(reviewReport as never);
    vi.mocked(commitLegacyImport).mockResolvedValue(report as never);
    render(<ImportSettingsPage />);

    fireEvent.change(screen.getByLabelText(/Legacy repository root path/), {
      target: { value: "C:\\legacy\\proyecto-vida" },
    });
    fireEvent.click(screen.getByRole("button", { name: /Preview import/ }));

    const commitButton = await screen.findByRole("button", {
      name: /Review decisions and commit/,
    });
    // Commit is blocked until every review item has an explicit decision.
    expect(commitButton).toBeDisabled();
    expect(commitLegacyImport).not.toHaveBeenCalled();

    fireEvent.change(screen.getByLabelText(/Decision for Raw entry/), {
      target: { value: "journal_entry" },
    });
    expect(commitButton).toBeEnabled();

    fireEvent.click(commitButton);
    await waitFor(() =>
      expect(commitLegacyImport).toHaveBeenCalledWith({
        workspace_id: "w1",
        source_root: "C:\\legacy\\proyecto-vida",
        include_paths: null,
        preview_fingerprint: "fp-1",
        decisions: [{ review_item_id: "item-1", entity_type: "journal_entry" }],
      }),
    );
    expect(getImportBatchStatus).toHaveBeenCalledWith("batch-1");
  });
});
