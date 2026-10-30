import { describe, expect, it, vi, beforeEach } from "vitest";
import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import JournalForm from "@/components/journal/JournalForm";
import { exportTemplateCatalog } from "@/lib/api/exportApi";

vi.mock("@/lib/api/exportApi", () => ({
  exportTemplateCatalog: vi.fn(),
}));

const cleanup = {
  remove_comments: false,
  remove_guide_blockquotes: false,
  remove_response_markers: false,
  remove_placeholders: false,
};

// Synthetic template fixture with a legacy body field and the preview shape.
// All content is
// invented for tests only.
const moodTemplate = {
  id: "foundation-journal",
  version: "1.0.1",
  display_name: "Synthetic journal",
  locale: "en",
  description: "Synthetic fixture template",
  min_app_version: "0.0.0",
  fields: [
    { id: "entry.free", field_type: "markdown", required: false },
    { id: "entry.mood", field_type: "short_text", required: false },
  ],
  sections: [],
  cleanup,
};

const structuredTemplate = {
  ...moodTemplate,
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
};

function fillBasics() {
  fireEvent.change(screen.getByLabelText("Title"), { target: { value: "A quiet day" } });
  fireEvent.change(screen.getByLabelText("Date"), { target: { value: "2026-09-08" } });
}

describe("JournalForm live Markdown preview", () => {
  beforeEach(() => {
    vi.mocked(exportTemplateCatalog).mockReset();
    vi.mocked(exportTemplateCatalog).mockResolvedValue([]);
  });

  it("pairs the content field with a named preview region", () => {
    render(<JournalForm onSubmit={vi.fn()} />);

    const textarea = screen.getByLabelText("Content (Markdown)");
    const region = screen.getByRole("region", { name: "Content (Markdown) preview" });
    expect(textarea.tagName.toLowerCase()).toBe("textarea");
    expect(region.textContent).toMatch(/No content yet/);
  });

  it("updates the preview live while typing without changing the source", () => {
    render(<JournalForm onSubmit={vi.fn()} />);

    fireEvent.change(screen.getByLabelText("Content (Markdown)"), {
      target: { value: "# Raw heading\n\n**bold** prose" },
    });

    const textarea = screen.getByLabelText("Content (Markdown)") as HTMLTextAreaElement;
    expect(textarea.value).toBe("# Raw heading\n\n**bold** prose");
    const region = screen.getByRole("region", { name: "Content (Markdown) preview" });
    expect(region.textContent).toContain("Raw heading");
    expect(region.textContent).toContain("bold");
  });
});

describe("JournalForm structured round-trip (8.6.4)", () => {
  beforeEach(() => {
    vi.mocked(exportTemplateCatalog).mockReset();
    vi.mocked(exportTemplateCatalog).mockResolvedValue([]);
  });

  it("submits authored prose verbatim with no normalization or joi markers", async () => {
    const onSubmit = vi.fn();
    vi.mocked(exportTemplateCatalog).mockResolvedValue([moodTemplate] as never);
    render(<JournalForm onSubmit={onSubmit} />);

    await screen.findByText("Synthetic journal (1.0.1)");
    fillBasics();
    fireEvent.change(screen.getByLabelText("Mood"), { target: { value: "calm" } });
    fireEvent.change(screen.getByLabelText("Content (Markdown)"), {
      target: { value: "# Heading\nbody\n\n- point one\n- point two" },
    });
    fireEvent.click(screen.getByRole("button", { name: "Save entry" }));

    await waitFor(() => expect(onSubmit).toHaveBeenCalledTimes(1));
    const submitted = onSubmit.mock.calls[0][0];
    // Byte-exact source: no heading demotion, no appended marker blocks,
    // and mood stays a structured field instead of prose materialization.
    expect(submitted.content_markdown).toBe("# Heading\nbody\n\n- point one\n- point two");
    expect(submitted.content_markdown).not.toContain("joi:");
    expect(submitted.mood_label).toBe("calm");
    expect(submitted.template_values).toEqual({});
  });

  it("parses the comma-separated emotions field preserving order and trimming", async () => {
    const onSubmit = vi.fn();
    render(<JournalForm onSubmit={onSubmit} />);

    fillBasics();
    fireEvent.change(screen.getByLabelText("Emotions"), {
      target: { value: "joy, anxiety , ,gratitude," },
    });
    fireEvent.click(screen.getByRole("button", { name: "Save entry" }));

    await waitFor(() => expect(onSubmit).toHaveBeenCalledTimes(1));
    expect(onSubmit.mock.calls[0][0].emotions).toEqual(["joy", "anxiety", "gratitude"]);
  });

  it("replaces the manifest emotions checkboxes with one free-text field", async () => {
    vi.mocked(exportTemplateCatalog).mockResolvedValue([structuredTemplate] as never);
    render(<JournalForm onSubmit={vi.fn()} />);

    await screen.findByText("Synthetic journal (1.0.1)");
    const emotions = screen.getByLabelText("Emotions") as HTMLInputElement;
    expect(emotions.tagName.toLowerCase()).toBe("input");
    expect(emotions.type).toBe("text");
    // No taxonomy checkboxes from the manifest multi_select definition.
    expect(screen.queryByRole("checkbox", { name: "joy" })).toBeNull();
    expect(screen.queryByRole("checkbox", { name: "calm" })).toBeNull();
    expect(screen.queryByLabelText("Free")).toBeNull();
    expect(screen.queryByLabelText("Libre")).toBeNull();
  });

  it("uses content_markdown as the only free-writing field", async () => {
    const onSubmit = vi.fn();
    vi.mocked(exportTemplateCatalog).mockResolvedValue([structuredTemplate] as never);
    render(<JournalForm onSubmit={onSubmit} />);

    await screen.findByText("Synthetic journal (1.0.1)");
    fillBasics();
    fireEvent.change(screen.getByLabelText("Content (Markdown)"), {
      target: { value: "Today was fine.\n\n# More detail" },
    });
    fireEvent.change(screen.getByLabelText("Emotions"), { target: { value: "grateful" } });
    fireEvent.click(screen.getByRole("button", { name: "Save entry" }));

    await waitFor(() => expect(onSubmit).toHaveBeenCalledTimes(1));
    const submitted = onSubmit.mock.calls[0][0];
    expect(submitted.template_values).toEqual({});
    expect(submitted.content_markdown).toBe("Today was fine.\n\n# More detail");
    expect(submitted.content_markdown).not.toContain("joi:");
    expect(submitted.emotions).toEqual(["grateful"]);
  });

  it("defaults Exportable to enabled with visible helper text", () => {
    render(<JournalForm onSubmit={vi.fn()} />);

    const toggle = screen.getByLabelText("Exportable") as HTMLInputElement;
    expect(toggle.checked).toBe(true);
    expect(screen.getByText(/stay local and private/i)).toBeTruthy();
    expect(screen.getByText(/never makes the entry public/i)).toBeTruthy();
    expect(screen.getByText(/disabling this prevents the entry from being exported/i)).toBeTruthy();
  });

  it("sends export_eligible false when the Exportable toggle is disabled", async () => {
    const onSubmit = vi.fn();
    render(<JournalForm onSubmit={onSubmit} />);

    fillBasics();
    fireEvent.click(screen.getByLabelText("Exportable"));
    fireEvent.click(screen.getByRole("button", { name: "Save entry" }));

    await waitFor(() => expect(onSubmit).toHaveBeenCalledTimes(1));
    expect(onSubmit.mock.calls[0][0].export_eligible).toBe(false);
  });

  it("loads stored structured metadata into the edit controls", async () => {
    vi.mocked(exportTemplateCatalog).mockResolvedValue([structuredTemplate] as never);
    render(
      <JournalForm
        onSubmit={vi.fn()}
        initial={{
          title: "Earlier entry",
          occurred_on: "2026-09-01",
          content_markdown: "Stored prose.",
          mood_label: "good",
          energy_label: "7",
          emotions: ["calm", "gratitude"],
          export_eligible: false,
          template_values: { "entry.free": "Stored free answer" },
        }}
      />,
    );

    expect((screen.getByLabelText("Emotions") as HTMLInputElement).value).toBe("calm, gratitude");
    expect((screen.getByLabelText("Mood") as HTMLInputElement).value).toBe("good");
    expect((screen.getByLabelText("Energy") as HTMLInputElement).value).toBe("7");
    expect((screen.getByLabelText("Exportable") as HTMLInputElement).checked).toBe(false);
    expect((screen.getByLabelText("Content (Markdown)") as HTMLTextAreaElement).value).toBe(
      "Stored prose.",
    );
    expect(screen.queryByLabelText("Free")).toBeNull();
    expect(screen.queryByLabelText("Libre")).toBeNull();
  });

  it("never infers a template version for an absent pair when editing", async () => {
    const onSubmit = vi.fn();
    vi.mocked(exportTemplateCatalog).mockResolvedValue([moodTemplate] as never);
    render(
      <JournalForm
        onSubmit={onSubmit}
        initial={{
          title: "Legacy entry",
          occurred_on: "2026-09-01",
          content_markdown: "Stored prose.",
          template_id: "foundation-journal",
          template_version: "",
        }}
      />,
    );

    await screen.findByText("Synthetic journal (1.0.1)");
    fireEvent.click(screen.getByRole("button", { name: "Save entry" }));

    await waitFor(() => expect(onSubmit).toHaveBeenCalledTimes(1));
    const submitted = onSubmit.mock.calls[0][0];
    // The recorded pair stays exactly as stored: the absent version is not
    // backfilled from the loaded manifest on edit.
    expect(submitted.template_id).toBe("foundation-journal");
    expect(submitted.template_version).toBe("");
  });

  it("does not render a Retroactive control", () => {
    render(<JournalForm onSubmit={vi.fn()} />);

    expect(screen.queryByLabelText(/retroactive/i)).toBeNull();
    expect(screen.queryByRole("checkbox", { name: /retroactive/i })).toBeNull();
  });
});
