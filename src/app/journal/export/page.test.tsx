import { describe, expect, it, vi, beforeEach } from "vitest";
import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { MemoryRouter } from "react-router";
import JournalExportPage from "@/app/journal/export/page";
import { listJournalEntries } from "@/lib/api/journalApi";
import {
  createExportJob,
  exportTemplateCapability,
  listExportJobs,
  openExportArtifact,
  previewExportSelection,
  runExportJob,
} from "@/lib/api/exportApi";
import { ensureDefaultWorkspace } from "@/lib/api/workspaceApi";

vi.mock("@/lib/api/journalApi", () => ({
  listJournalEntries: vi.fn(),
}));

vi.mock("@/lib/api/exportApi", () => ({
  createExportJob: vi.fn(),
  previewExportSelection: vi.fn(),
  runExportJob: vi.fn(),
  exportTemplateCapability: vi.fn(),
  listExportJobs: vi.fn(),
  openExportArtifact: vi.fn(),
}));

vi.mock("@/lib/api/workspaceApi", () => ({
  ensureDefaultWorkspace: vi.fn(),
}));

const workspace = { id: "w1", name: "Default" };
const entry = {
  id: "j1",
  workspace_id: "w1",
  occurred_on: "2026-09-04",
  title: "A quiet day",
  content_markdown: "Some private content.",
  template_id: null,
  template_version: null,
  template_values: {},
  mood_label: null,
  energy_label: null,
  emotions: [],
  export_eligible: false,
  source: "user",
  created_at: "2026-09-04T10:00:00Z",
  updated_at: "2026-09-04T10:00:00Z",
  archived_at: null,
};

const preview = {
  item_count: 1,
  items: [
    {
      id: "j1",
      entity_type: "journal_entry",
      title: "A quiet day",
      occurred_on: "2026-09-04",
      privacy_scope: "private",
      content_hash: "abc123",
    },
  ],
  resolved_selection: { journal_entry_ids: ["j1"], date_range: null },
  selection_fingerprint: "0123456789abcdef",
  has_therapy_private: false,
};

const completedJob = {
  id: "job-1",
  workspace_id: "w1",
  export_type: "markdown",
  scope_type: "journal",
  title: "Journal export",
  template_name: "default",
  status: "completed",
  output_path: null,
  generated_at: "2026-09-06T10:00:00Z",
  error_message: null,
  selection_fingerprint: "0123456789abcdef",
  warnings: [],
  created_at: "2026-09-06T10:00:00Z",
  updated_at: "2026-09-06T10:00:05Z",
  artifacts: [],
};

function renderPage() {
  return render(
    <MemoryRouter>
      <JournalExportPage />
    </MemoryRouter>,
  );
}

describe("JournalExportPage", () => {
  beforeEach(() => {
    vi.mocked(ensureDefaultWorkspace).mockReset();
    vi.mocked(listJournalEntries).mockReset();
    vi.mocked(previewExportSelection).mockReset();
    vi.mocked(createExportJob).mockReset();
    vi.mocked(runExportJob).mockReset();
    vi.mocked(exportTemplateCapability).mockReset();
    vi.mocked(listExportJobs).mockReset();
    vi.mocked(openExportArtifact).mockReset();
    vi.mocked(exportTemplateCapability).mockResolvedValue({
      name: "default",
      version: "1.0.0-mvp",
      valid: true,
      compiler_version: "typst 0.15.1 (embedded)",
      unavailable_resources: [],
    } as never);

    vi.mocked(ensureDefaultWorkspace).mockResolvedValue(workspace as never);
    vi.mocked(listJournalEntries).mockResolvedValue([entry as never]);
    vi.mocked(previewExportSelection).mockResolvedValue(preview as never);
    vi.mocked(createExportJob).mockResolvedValue(completedJob as never);
    vi.mocked(runExportJob).mockResolvedValue(completedJob as never);
    vi.mocked(listExportJobs).mockResolvedValue([]);
    vi.mocked(openExportArtifact).mockResolvedValue();
  });

  it("passes the preview fingerprint to createExportJob", async () => {
    renderPage();

    const checkbox = await screen.findByLabelText(/A quiet day/);
    fireEvent.click(checkbox);

    await screen.findByText("Selection fingerprint");

    fireEvent.click(screen.getByRole("button", { name: /Export/ }));

    await waitFor(() => expect(createExportJob).toHaveBeenCalledTimes(1));
    expect(createExportJob).toHaveBeenCalledWith(
      expect.objectContaining({ preview_fingerprint: "0123456789abcdef" }),
    );
  });

  it("prevents duplicate submission while a job is running", async () => {
    let resolveCreate: (value: never) => void = () => {};
    vi.mocked(createExportJob).mockImplementation(
      () =>
        new Promise((resolve) => {
          resolveCreate = resolve as (value: never) => void;
        }),
    );

    renderPage();

    const checkbox = await screen.findByLabelText(/A quiet day/);
    fireEvent.click(checkbox);
    await screen.findByText("Selection fingerprint");

    const submit = screen.getByRole("button", { name: /Export/ });
    fireEvent.click(submit);
    fireEvent.click(submit);

    expect(createExportJob).toHaveBeenCalledTimes(1);

    resolveCreate(completedJob as never);
    await waitFor(() => expect(runExportJob).toHaveBeenCalledTimes(1));
  });

  it("shows a clear re-preview path on a stale preview error", async () => {
    const staleError = new Error(
      '{"code":"VALIDATION","message":"The export selection changed since it was previewed; preview again."}',
    );
    vi.mocked(createExportJob).mockRejectedValue(staleError);

    renderPage();

    const checkbox = await screen.findByLabelText(/A quiet day/);
    fireEvent.click(checkbox);
    await screen.findByText("Selection fingerprint");

    fireEvent.click(screen.getByRole("button", { name: /Export/ }));

    expect(await screen.findByRole("button", { name: /Re-preview selection/ })).toBeTruthy();
    expect(screen.getByRole("alert")).toHaveTextContent(/changed since it was previewed/i);
  });

  it("export-all disables dates and previews without date_range or IDs", async () => {
    const { container } = renderPage();

    await screen.findByLabelText(/A quiet day/);
    // The "Exportar todo" literal was fixed by 8.6.10: the shared
    // `common.export.exportAll` catalog key now renders proper English copy on
    // the en-US surface.
    const exportAll = await screen.findByLabelText(/Export all/);
    fireEvent.click(exportAll);

    const dateInputs = container.querySelectorAll('input[type="date"]');
    expect(dateInputs).toHaveLength(2);
    dateInputs.forEach((input) => expect(input).toBeDisabled());

    await waitFor(() =>
      expect(previewExportSelection).toHaveBeenCalledWith(
        expect.objectContaining({
          selection: { export_all: true },
        }),
      ),
    );
  });
});
