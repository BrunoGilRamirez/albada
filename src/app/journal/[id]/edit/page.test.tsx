import { describe, expect, it, vi, beforeEach } from "vitest";
import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { MemoryRouter, Route, Routes } from "react-router";
import EditJournalEntryPage from "@/app/journal/[id]/edit/page";
import { getJournalEntry, updateJournalEntry } from "@/lib/api/journalApi";
import { exportTemplateCatalog } from "@/lib/api/exportApi";

vi.mock("@/lib/api/journalApi", () => ({
  getJournalEntry: vi.fn(),
  updateJournalEntry: vi.fn(),
}));

vi.mock("@/lib/api/exportApi", () => ({
  exportTemplateCatalog: vi.fn(),
}));

// Synthetic fixtures include the removed legacy field ID.
// No personal, journal, or therapy content is used anywhere in this file.
const structuredTemplate = {
  id: "foundation-journal",
  version: "1.0.1",
  display_name: "Synthetic journal",
  locale: "en",
  description: "Synthetic fixture template",
  min_app_version: "0.0.0",
  fields: [
    { id: "entry.free", field_type: "markdown", required: true },
    {
      id: "entry.emotions",
      field_type: "multi_select",
      required: false,
      options: ["calm", "joy"],
    },
    { id: "entry.mood", field_type: "single_select", required: false, options: ["low", "good"] },
    { id: "entry.energy", field_type: "bounded_scale", required: false },
  ],
  sections: [],
  cleanup: {
    remove_comments: false,
    remove_guide_blockquotes: false,
    remove_response_markers: false,
    remove_placeholders: false,
  },
};

// Legacy-unversioned fixture: the entity exposes an absent (masked) template
// pair while a hidden historical `entry.free` value remains available.
function legacyEntry() {
  return {
    id: "j1",
    workspace_id: "w1",
    occurred_on: "2026-09-08",
    title: "Legacy import",
    content_markdown: "## Notes\n\nfirst line\n\nsecond line",
    template_id: null,
    template_version: null,
    template_values: { "entry.free": "Stored answer" },
    mood_label: "good",
    energy_label: "7",
    emotions: ["calm", "gratitude"],
    export_eligible: false,
    source: "import",
    created_at: "2026-09-08T20:00:00Z",
    updated_at: "2026-09-08T20:00:00Z",
    archived_at: null,
  };
}

function renderPage() {
  return render(
    <MemoryRouter initialEntries={["/journal/j1/edit"]}>
      <Routes>
        <Route path="/journal/:id/edit" element={<EditJournalEntryPage />} />
        <Route path="/journal/:id" element={<p>Journal detail stub</p>} />
      </Routes>
    </MemoryRouter>,
  );
}

describe("EditJournalEntryPage structured round-trip (8.6.4)", () => {
  beforeEach(() => {
    vi.mocked(getJournalEntry).mockReset();
    vi.mocked(updateJournalEntry).mockReset();
    vi.mocked(exportTemplateCatalog).mockReset();

    vi.mocked(getJournalEntry).mockResolvedValue(legacyEntry() as never);
    vi.mocked(exportTemplateCatalog).mockResolvedValue([]);
    vi.mocked(updateJournalEntry).mockResolvedValue(legacyEntry() as never);
  });

  it("loads stored structured metadata into the edit controls", async () => {
    vi.mocked(exportTemplateCatalog).mockResolvedValue([structuredTemplate] as never);
    renderPage();

    const emotions = (await screen.findByLabelText("Emotions")) as HTMLInputElement;
    expect(emotions.value).toBe("calm, gratitude");
    expect((screen.getByLabelText("Mood") as HTMLInputElement).value).toBe("good");
    expect((screen.getByLabelText("Energy") as HTMLInputElement).value).toBe("7");
    expect((screen.getByLabelText("Exportable") as HTMLInputElement).checked).toBe(false);
    expect((screen.getByLabelText("Date") as HTMLInputElement).value).toBe("2026-09-08");
    expect((screen.getByLabelText("Content (Markdown)") as HTMLTextAreaElement).value).toBe(
      "## Notes\n\nfirst line\n\nsecond line",
    );
    expect(screen.queryByLabelText("Free")).toBeNull();
    expect(screen.queryByLabelText(/retroactive/i)).toBeNull();
  });

  it("saves the full structured payload and preserves unchanged values", async () => {
    renderPage();

    await screen.findByLabelText("Emotions");
    fireEvent.click(screen.getByRole("button", { name: "Save changes" }));

    await waitFor(() => expect(updateJournalEntry).toHaveBeenCalledTimes(1));
    expect(updateJournalEntry).toHaveBeenCalledWith("j1", {
      occurred_on: "2026-09-08",
      title: "Legacy import",
      content_markdown: "## Notes\n\nfirst line\n\nsecond line",
      // Masked legacy pair stays absent; it is never inferred on edit.
      template_id: null,
      template_version: null,
      // Unchanged template values are sent as null so legacy hidden values
      // remain preserved by the native update.
      template_values: null,
      mood_label: "good",
      energy_label: "7",
      emotions: ["calm", "gratitude"],
      export_eligible: false,
    });
    expect(await screen.findByText("Journal detail stub")).toBeTruthy();
  });

  it("saves an unchanged absent-pair entry without manifest required", async () => {
    // A masked legacy-unversioned edit never infers a template identity.
    vi.mocked(exportTemplateCatalog).mockResolvedValue([structuredTemplate] as never);
    vi.mocked(getJournalEntry).mockResolvedValue({
      ...legacyEntry(),
      template_values: {},
    } as never);

    renderPage();

    await screen.findByLabelText("Emotions");
    expect(screen.queryByLabelText("Free")).toBeNull();
    fireEvent.click(screen.getByRole("button", { name: "Save changes" }));

    await waitFor(() => expect(updateJournalEntry).toHaveBeenCalledTimes(1));
    expect(updateJournalEntry).toHaveBeenCalledWith(
      "j1",
      expect.objectContaining({
        // The exact absent pair is kept; unchanged empty structured values
        // are sent as null so the native COALESCE preserves the stored map.
        template_id: null,
        template_version: null,
        template_values: null,
        content_markdown: "## Notes\n\nfirst line\n\nsecond line",
      }),
    );
    expect(await screen.findByText("Journal detail stub")).toBeTruthy();
  });

  it("sends edited emotions, eligibility, and verbatim prose", async () => {
    renderPage();

    await screen.findByLabelText("Emotions");
    fireEvent.change(screen.getByLabelText("Emotions"), {
      target: { value: "anxious , ,joy," },
    });
    fireEvent.click(screen.getByLabelText("Exportable"));
    fireEvent.change(screen.getByLabelText("Content (Markdown)"), {
      target: { value: "# Raw heading stays\n\nedited body" },
    });
    fireEvent.click(screen.getByRole("button", { name: "Save changes" }));

    await waitFor(() => expect(updateJournalEntry).toHaveBeenCalledTimes(1));
    expect(updateJournalEntry).toHaveBeenCalledWith(
      "j1",
      expect.objectContaining({
        content_markdown: "# Raw heading stays\n\nedited body",
        emotions: ["anxious", "joy"],
        export_eligible: true,
        template_values: null,
      }),
    );
  });

  it("keeps a hidden legacy field out of the form payload when editing prose", async () => {
    renderPage();

    const content = (await screen.findByLabelText("Content (Markdown)")) as HTMLTextAreaElement;
    expect(screen.queryByLabelText("Free")).toBeNull();
    fireEvent.change(content, { target: { value: "Edited prose only." } });
    fireEvent.click(screen.getByRole("button", { name: "Save changes" }));

    await waitFor(() => expect(updateJournalEntry).toHaveBeenCalledTimes(1));
    expect(updateJournalEntry).toHaveBeenCalledWith(
      "j1",
      expect.objectContaining({
        template_id: null,
        template_version: null,
        template_values: null,
        content_markdown: "Edited prose only.",
      }),
    );
    expect(await screen.findByText("Journal detail stub")).toBeTruthy();
  });

  it("retains a changed marker-bearing legacy body and reports the native guard", async () => {
    const markerSource = 'Authored\n<!-- joi:field id="entry.mood" -->good<!-- joi:end-field -->';
    vi.mocked(getJournalEntry).mockResolvedValue({
      ...legacyEntry(),
      content_markdown: markerSource,
    } as never);
    vi.mocked(updateJournalEntry).mockRejectedValue({
      code: "VALIDATION",
      message: "Journal content cannot contain generated metadata markers.",
    } as never);
    renderPage();

    const content = (await screen.findByLabelText("Content (Markdown)")) as HTMLTextAreaElement;
    expect(content.value).toBe(markerSource);

    fireEvent.change(content, { target: { value: `${markerSource}\n\nedited` } });
    fireEvent.click(screen.getByRole("button", { name: "Save changes" }));

    // The native no-marker guard rejects a changed ambiguous legacy body; the
    // page surfaces the error, never silently strips or rewrites the source,
    // and keeps the authored text in the editor.
    await waitFor(() =>
      expect(screen.getByText("Please check the information and try again.")).toBeTruthy(),
    );
    expect((screen.getByLabelText("Content (Markdown)") as HTMLTextAreaElement).value).toBe(
      `${markerSource}\n\nedited`,
    );
    expect(screen.queryByText("Journal detail stub")).toBeNull();
  });
});
