import { describe, expect, it, vi, beforeEach } from "vitest";
import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { MemoryRouter, Route, Routes } from "react-router";
import NewJournalEntryPage from "@/app/journal/new/page";
import { createJournalEntry } from "@/lib/api/journalApi";
import { exportTemplateCatalog } from "@/lib/api/exportApi";
import { ensureDefaultWorkspace } from "@/lib/api/workspaceApi";

vi.mock("@/lib/api/journalApi", () => ({
  createJournalEntry: vi.fn(),
}));

vi.mock("@/lib/api/exportApi", () => ({
  exportTemplateCatalog: vi.fn(),
}));

vi.mock("@/lib/api/workspaceApi", () => ({
  ensureDefaultWorkspace: vi.fn(),
}));

const workspace = { id: "w1", name: "Default" };

const createdEntry = {
  id: "j1",
  workspace_id: "w1",
  occurred_on: "2026-09-08",
  title: "A quiet day",
  content_markdown: "Wrote tests today.",
  template_id: "foundation-journal",
  template_version: "1.0.1",
  template_values: {},
  mood_label: null,
  energy_label: null,
  emotions: [],
  export_eligible: true,
  source: "user",
  created_at: "2026-09-08T20:00:00Z",
  updated_at: "2026-09-08T20:00:00Z",
  archived_at: null,
};

const catalogTemplate = {
  id: "foundation-journal",
  version: "1.1.0",
  display_name: "Synthetic journal",
  locale: "en",
  description: "Synthetic fixture template",
  min_app_version: "0.0.0",
  fields: [{ id: "entry.free", field_type: "textarea", required: false }],
  sections: [],
  cleanup: {
    remove_comments: false,
    remove_guide_blockquotes: false,
    remove_response_markers: false,
    remove_placeholders: false,
    remove_images: false,
    remove_empty_day_markers: false,
  },
};

// Synthetic fixture includes the removed legacy field to verify it is not
// rendered or submitted.
const bundledCreateTemplate = {
  ...catalogTemplate,
  version: "1.0.1",
  fields: [{ id: "entry.free", field_type: "markdown", required: true }],
};

function renderPage() {
  return render(
    <MemoryRouter initialEntries={["/journal/new"]}>
      <Routes>
        <Route path="/journal/new" element={<NewJournalEntryPage />} />
        <Route path="/journal/:id" element={<p>Journal detail stub</p>} />
      </Routes>
    </MemoryRouter>,
  );
}

describe("NewJournalEntryPage", () => {
  beforeEach(() => {
    vi.mocked(createJournalEntry).mockReset();
    vi.mocked(exportTemplateCatalog).mockReset();
    vi.mocked(ensureDefaultWorkspace).mockReset();

    vi.mocked(ensureDefaultWorkspace).mockResolvedValue(workspace as never);
    vi.mocked(exportTemplateCatalog).mockResolvedValue([]);
    vi.mocked(createJournalEntry).mockResolvedValue(createdEntry as never);
  });

  it("creates a journal entry with the typed Markdown content", async () => {
    renderPage();

    fireEvent.change(await screen.findByLabelText("Title"), {
      target: { value: "A quiet day" },
    });
    fireEvent.change(screen.getByLabelText("Date"), {
      target: { value: "2026-09-08" },
    });
    fireEvent.change(screen.getByLabelText("Content (Markdown)"), {
      target: { value: "Wrote tests today." },
    });
    fireEvent.click(screen.getByRole("button", { name: "Create entry" }));

    await waitFor(() => expect(createJournalEntry).toHaveBeenCalledTimes(1));
    expect(createJournalEntry).toHaveBeenCalledWith(
      expect.objectContaining({
        workspace_id: "w1",
        occurred_on: "2026-09-08",
        title: "A quiet day",
        content_markdown: "Wrote tests today.",
        template_id: "foundation-journal",
        template_version: "1.0.1",
        template_values: {},
        mood_label: null,
        energy_label: null,
        emotions: [],
        export_eligible: true,
      }),
    );
    expect(await screen.findByText("Journal detail stub")).toBeTruthy();
  });

  it("sends parsed comma emotions, verbatim prose, and default exportability", async () => {
    renderPage();

    fireEvent.change(await screen.findByLabelText("Title"), {
      target: { value: "A quiet day" },
    });
    fireEvent.change(screen.getByLabelText("Date"), {
      target: { value: "2026-09-08" },
    });
    fireEvent.change(screen.getByLabelText("Content (Markdown)"), {
      target: { value: "# Day log\n\nkept as authored" },
    });
    fireEvent.change(screen.getByLabelText("Emotions"), {
      target: { value: "calm, tired , ," },
    });
    fireEvent.change(screen.getByLabelText("Mood"), { target: { value: "good" } });
    fireEvent.click(screen.getByRole("button", { name: "Create entry" }));

    await waitFor(() => expect(createJournalEntry).toHaveBeenCalledTimes(1));
    expect(createJournalEntry).toHaveBeenCalledWith(
      expect.objectContaining({
        content_markdown: "# Day log\n\nkept as authored",
        emotions: ["calm", "tired"],
        mood_label: "good",
        export_eligible: true,
      }),
    );
  });

  it("sends export_eligible false when Exportable is turned off", async () => {
    renderPage();

    fireEvent.change(await screen.findByLabelText("Title"), {
      target: { value: "Private note" },
    });
    fireEvent.change(screen.getByLabelText("Date"), {
      target: { value: "2026-09-08" },
    });
    fireEvent.click(screen.getByLabelText("Exportable"));
    fireEvent.click(screen.getByRole("button", { name: "Create entry" }));

    await waitFor(() => expect(createJournalEntry).toHaveBeenCalledTimes(1));
    expect(createJournalEntry).toHaveBeenCalledWith(
      expect.objectContaining({ export_eligible: false }),
    );
  });

  it("pairs the template ID with the loaded manifest version on create", async () => {
    vi.mocked(exportTemplateCatalog).mockResolvedValue([catalogTemplate] as never);
    renderPage();

    await screen.findByText("Synthetic journal (1.1.0)");
    fireEvent.change(screen.getByLabelText("Title"), {
      target: { value: "A quiet day" },
    });
    fireEvent.change(screen.getByLabelText("Date"), {
      target: { value: "2026-09-08" },
    });
    fireEvent.change(screen.getByLabelText("Content (Markdown)"), {
      target: { value: "Wrote tests today." },
    });
    fireEvent.click(screen.getByRole("button", { name: "Create entry" }));

    await waitFor(() => expect(createJournalEntry).toHaveBeenCalledTimes(1));
    // 8.6.3 contract: a template-backed create sends the exact
    // template_id + manifest template_version pair, never the fallback alone.
    expect(createJournalEntry).toHaveBeenCalledWith(
      expect.objectContaining({
        template_id: "foundation-journal",
        template_version: "1.1.0",
      }),
    );
  });

  it("creates with free writing only in content_markdown", async () => {
    vi.mocked(exportTemplateCatalog).mockResolvedValue([bundledCreateTemplate] as never);
    renderPage();

    await screen.findByText("Synthetic journal (1.0.1)");
    fireEvent.change(screen.getByLabelText("Title"), {
      target: { value: "A structured day" },
    });
    fireEvent.change(screen.getByLabelText("Date"), {
      target: { value: "2026-09-08" },
    });
    fireEvent.change(screen.getByLabelText("Content (Markdown)"), {
      target: { value: "Today was fine.\n\n# More detail" },
    });
    fireEvent.click(screen.getByRole("button", { name: "Create entry" }));

    await waitFor(() => expect(createJournalEntry).toHaveBeenCalledTimes(1));
    expect(createJournalEntry).toHaveBeenCalledWith(
      expect.objectContaining({
        content_markdown: "Today was fine.\n\n# More detail",
        template_id: "foundation-journal",
        template_version: "1.0.1",
        template_values: {},
      }),
    );
    expect(screen.queryByLabelText("Free")).toBeNull();
  });

  it("does not submit when the title is blank", async () => {
    renderPage();

    fireEvent.change(await screen.findByLabelText("Date"), {
      target: { value: "2026-09-08" },
    });
    fireEvent.click(screen.getByRole("button", { name: "Create entry" }));

    expect(createJournalEntry).not.toHaveBeenCalled();
  });
});
