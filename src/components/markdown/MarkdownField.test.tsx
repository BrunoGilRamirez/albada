import i18n from "i18next";
import { afterEach, describe, expect, it, vi } from "vitest";
import { useState } from "react";
import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import MarkdownField, { useMarkdownEditorChrome } from "@/components/markdown/MarkdownField";
import type { MarkdownEditorChrome } from "@/components/markdown/MarkdownEditor";

interface ControlledFieldProps {
  label?: string;
  initial?: string;
  onCommit?: (value: string) => void;
  previewLabel?: string;
  previewRegionLabel?: string;
  previewEmptyText?: string;
  editorChrome?: MarkdownEditorChrome;
}

function ControlledField({
  label = "Description (Markdown)",
  initial = "",
  onCommit,
  previewLabel,
  previewRegionLabel,
  previewEmptyText,
  editorChrome,
}: ControlledFieldProps) {
  const [value, setValue] = useState(initial);
  return (
    <MarkdownField
      id="controlled-field"
      label={label}
      value={value}
      onChange={(next) => {
        setValue(next);
        onCommit?.(next);
      }}
      previewLabel={previewLabel}
      previewRegionLabel={previewRegionLabel}
      previewEmptyText={previewEmptyText}
      editorChrome={editorChrome}
    />
  );
}

const GERMAN_CHROME: MarkdownEditorChrome = {
  toolbarLabel: "Markdown-Formatierung",
  bold: "Fett",
  italic: "Kursiv",
  link: "Link",
  list: "Liste",
  headingSelect: "Überschrift einfügen",
  headingPlaceholder: "Überschrift…",
  headingOptions: [
    "Überschrift 2 (##)",
    "Überschrift 3 (###)",
    "Überschrift 4 (####)",
    "Überschrift 5 (#####)",
    "Überschrift 6 (######)",
  ],
  headingHelp: "Hilfe zu Überschriften",
  headingHelpTitle:
    "Für den Titel brauchst du kein #: Das Feld Titel fügt es bereits hinzu. Verwende ## oder tiefere Ebenen nur für Unterüberschriften innerhalb des Inhalts.",
};

describe("MarkdownField shared editor/preview pattern", () => {
  it("switches between split, editor-only, and preview-only modes", () => {
    render(<ControlledField />);
    const textarea = screen.getByLabelText("Description (Markdown)");
    const region = screen.getByRole("region", { name: "Description (Markdown) preview" });
    const split = screen.getByRole("button", { name: "Split" });
    const editor = screen.getByRole("button", { name: "Editor only" });
    const preview = screen.getByRole("button", { name: "Preview only" });

    expect(split).toHaveAttribute("aria-pressed", "true");
    fireEvent.click(editor);
    expect(textarea.parentElement?.parentElement).not.toHaveClass("hidden");
    expect(region.parentElement).toHaveClass("hidden");
    fireEvent.click(preview);
    expect(textarea.parentElement?.parentElement).toHaveClass("hidden");
    expect(region.parentElement).not.toHaveClass("hidden");
    fireEvent.click(split);
    expect(split).toHaveAttribute("aria-pressed", "true");
  });
  it("labels the textarea and names the preview region", () => {
    render(<ControlledField />);

    const textarea = screen.getByLabelText("Description (Markdown)");
    expect(textarea.tagName.toLowerCase()).toBe("textarea");
    expect(screen.getByRole("region", { name: "Description (Markdown) preview" })).toBeTruthy();
  });

  it("renders the empty-state placeholder in the preview region", () => {
    render(<ControlledField />);

    const region = screen.getByRole("region", { name: "Description (Markdown) preview" });
    expect(region.textContent).toMatch(/No content yet/);
  });

  it("updates the preview live as the user types", () => {
    render(<ControlledField />);

    fireEvent.change(screen.getByLabelText("Description (Markdown)"), {
      target: { value: "## Synthetic plan\n\n- **first step**" },
    });

    const region = screen.getByRole("region", { name: "Description (Markdown) preview" });
    expect(screen.getByRole("heading", { level: 2, name: "Synthetic plan" })).toBeTruthy();
    expect(region.textContent).toContain("first step");
  });

  it("keeps the Markdown source verbatim through the editor", () => {
    const onCommit = vi.fn();
    const source = "# raw heading\n\\# escape  <raw>  trailing   \n##second-nospace";
    render(<ControlledField onCommit={onCommit} />);

    fireEvent.change(screen.getByLabelText("Description (Markdown)"), {
      target: { value: source },
    });

    expect((screen.getByLabelText("Description (Markdown)") as HTMLTextAreaElement).value).toBe(
      source,
    );
    expect(onCommit).toHaveBeenCalledWith(source);
  });

  it("starts from a given source and keeps headings intact in the editor", () => {
    render(<ControlledField initial={"# Stored source\nbody"} />);

    expect((screen.getByLabelText("Description (Markdown)") as HTMLTextAreaElement).value).toBe(
      "# Stored source\nbody",
    );
    expect(
      screen.getByRole("region", { name: "Description (Markdown) preview" }).textContent,
    ).toContain("body");
  });

  it("does not render raw HTML in the preview", () => {
    render(<ControlledField initial={'<img src="x"><script>alert(1)</script>Safe text'} />);

    expect(document.querySelector("img")).toBeNull();
    expect(document.querySelector("script")).toBeNull();
    expect(
      screen.getByRole("region", { name: "Description (Markdown) preview" }).textContent,
    ).toContain("Safe text");
  });

  it("pairs the editor and preview in one responsive grid", () => {
    render(<ControlledField />);

    const region = screen.getByRole("region", { name: "Description (Markdown) preview" });
    const previewColumn = region.parentElement;
    const grid = previewColumn?.parentElement;
    expect(grid?.classList.contains("xl:grid-cols-2")).toBe(true);
    // The editor textarea lives in the same grid pair.
    expect(grid?.contains(screen.getByLabelText("Description (Markdown)"))).toBe(true);
  });

  it("keeps the preview at the editor height and within the same bounds", () => {
    render(<ControlledField />);

    const textarea = screen.getByLabelText("Description (Markdown)") as HTMLTextAreaElement;
    const preview = screen.getByRole("region", { name: "Description (Markdown) preview" });

    expect(preview.style.height).toBe(textarea.style.height);
    expect(preview.style.minHeight).toBe(textarea.style.minHeight);
    expect(preview.style.maxHeight).toBe(textarea.style.maxHeight);
  });

  it("keeps the English defaults for surfaces that pass no localized preview chrome", () => {
    render(<ControlledField />);

    expect(screen.getByText("Preview")).toBeTruthy();
    expect(screen.getByRole("region", { name: "Description (Markdown) preview" })).toBeTruthy();
    expect(screen.getByText("No content yet.")).toBeTruthy();
  });

  it("renders caller-provided localized preview chrome verbatim", () => {
    render(
      <ControlledField
        previewLabel="Vorschau"
        previewRegionLabel="Vorschau von Description (Markdown)"
        previewEmptyText="Noch kein Inhalt."
      />,
    );

    expect(screen.getByText("Vorschau")).toBeTruthy();
    expect(screen.getByText("Noch kein Inhalt.")).toBeTruthy();
    expect(
      screen.getByRole("region", { name: "Vorschau von Description (Markdown)" }),
    ).toBeTruthy();
    expect(screen.queryByText("Preview")).toBeNull();
    expect(screen.queryByRole("region", { name: "Description (Markdown) preview" })).toBeNull();
  });

  it("keeps the built-in editor toolbar chrome for surfaces that do not opt in", () => {
    const { container } = render(<ControlledField />);

    expect(container.querySelectorAll('[aria-label="Markdown formatting"]')).toHaveLength(1);
    for (const name of ["Bold", "Italic", "Link", "List"]) {
      expect(screen.getByRole("button", { name })).toBeTruthy();
    }
    expect(screen.getByRole("combobox", { name: "Insert heading" })).toBeTruthy();
    expect(screen.getByRole("option", { name: "Heading 2 (##)" })).toBeTruthy();
    const help = screen.getByRole("button", { name: "Heading help" });
    expect(help.getAttribute("title")).toBe(
      "No necesitas usar # para el título: el campo Title ya lo añade. Usa ## o niveles más profundos solo para subtítulos dentro del contenido.",
    );
  });

  it("renders the opted-in localized editor chrome through the field", () => {
    const { container } = render(<ControlledField editorChrome={GERMAN_CHROME} />);

    expect(container.querySelectorAll('[aria-label="Markdown-Formatierung"]')).toHaveLength(1);
    expect(container.querySelectorAll('[aria-label="Markdown formatting"]')).toHaveLength(0);
    expect(screen.getByRole("button", { name: "Fett" })).toBeTruthy();
    expect(screen.getByRole("button", { name: "Kursiv" })).toBeTruthy();
    expect(screen.getByRole("combobox", { name: "Überschrift einfügen" })).toBeTruthy();
    expect(screen.getByRole("option", { name: "Überschrift 2 (##)" })).toBeTruthy();
    const help = screen.getByRole("button", { name: "Hilfe zu Überschriften" });
    expect(help.getAttribute("title")).toBe(GERMAN_CHROME.headingHelpTitle);
  });
});

describe("useMarkdownEditorChrome", () => {
  function ChromeProbe() {
    const chrome = useMarkdownEditorChrome();
    return (
      <div>
        <span data-testid="toolbar">{chrome.toolbarLabel}</span>
        <span data-testid="bold">{chrome.bold}</span>
        <span data-testid="heading2">{chrome.headingOptions[0]}</span>
        <span data-testid="helpTitle">{chrome.headingHelpTitle}</span>
      </div>
    );
  }

  it("builds the catalog chrome in the active language and follows runtime switching", async () => {
    render(<ChromeProbe />);
    expect(screen.getByTestId("toolbar").textContent).toBe("Markdown formatting");
    expect(screen.getByTestId("bold").textContent).toBe("Bold");
    expect(screen.getByTestId("heading2").textContent).toBe("Heading 2 (##)");

    await i18n.changeLanguage("es-ES");
    await waitFor(() => expect(screen.getByTestId("bold").textContent).toBe("Negrita"));
    expect(screen.getByTestId("toolbar").textContent).toBe("Formato de Markdown");
    expect(screen.getByTestId("heading2").textContent).toBe("Encabezado 2 (##)");
    // The Spanish catalog tooltip is rendered verbatim.
    expect(screen.getByTestId("helpTitle").textContent).toBe(
      "No necesitas usar # para el título: el campo Título ya lo añade. Usa ## o niveles más profundos solo para subtítulos dentro del contenido.",
    );
  });

  afterEach(async () => {
    await i18n.changeLanguage("en-US");
  });
});
