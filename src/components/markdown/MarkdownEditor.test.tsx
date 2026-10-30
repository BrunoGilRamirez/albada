import { describe, expect, it, vi } from "vitest";
import { useState } from "react";
import { fireEvent, render, screen } from "@testing-library/react";
import MarkdownEditor, {
  HEADING_LEVELS,
  type MarkdownEditorChrome,
} from "@/components/markdown/MarkdownEditor";

// Focused coverage for the opt-in editor chrome (8.6.9 acceptance gap):
// callers that do not opt in must render the exact pre-opt-in strings
// (including the original Spanish-only heading tooltip), opted-in callers see
// their localized chrome, and the generated Markdown source is byte-identical
// either way because the inserted placeholders are content, not UI copy.
// Synthetic fixtures only.

const DEFAULT_HELP_TITLE =
  "No necesitas usar # para el título: el campo Title ya lo añade. Usa ## o niveles más profundos solo para subtítulos dentro del contenido.";

const FRENCH_CHROME: MarkdownEditorChrome = {
  toolbarLabel: "Mise en forme Markdown",
  bold: "Gras",
  italic: "Italique",
  link: "Lien",
  list: "Liste",
  headingSelect: "Insérer un titre",
  headingPlaceholder: "Titre…",
  headingOptions: HEADING_LEVELS.map((level) => `Titre ${level} (${"#".repeat(level)})`),
  headingHelp: "Aide sur les titres",
  headingHelpTitle: "Tu n'as pas besoin d'utiliser # pour le titre : le champ Titre l'ajoute déjà.",
};

function ControlledEditor({
  chrome,
  onCommit,
}: {
  chrome?: MarkdownEditorChrome;
  onCommit?: (value: string) => void;
}) {
  const [value, setValue] = useState("");
  return (
    <MarkdownEditor
      id="controlled-editor"
      label="Body"
      value={value}
      onChange={(next) => {
        setValue(next);
        onCommit?.(next);
      }}
      chrome={chrome}
    />
  );
}

describe("MarkdownEditor default chrome (opt-out surfaces)", () => {
  it("keeps the pre-opt-in toolbar strings byte-identical when no chrome is passed", () => {
    const { container } = render(<ControlledEditor />);

    expect(container.querySelectorAll('[aria-label="Markdown formatting"]')).toHaveLength(1);
    for (const name of ["Bold", "Italic", "Link", "List"]) {
      expect(screen.getByRole("button", { name })).toBeTruthy();
    }
    expect(screen.getByRole("combobox", { name: "Insert heading" })).toBeTruthy();
    expect(screen.getByText("Heading…")).toBeTruthy();
    for (const label of [
      "Heading 2 (##)",
      "Heading 3 (###)",
      "Heading 4 (####)",
      "Heading 5 (#####)",
      "Heading 6 (######)",
    ]) {
      expect(screen.getByRole("option", { name: label })).toBeTruthy();
    }
    const help = screen.getByRole("button", { name: "Heading help" });
    expect(help.getAttribute("title")).toBe(DEFAULT_HELP_TITLE);
  });
});

describe("MarkdownEditor localized chrome (opt-in surfaces)", () => {
  it("renders every provided chrome string and hides the English defaults", () => {
    const { container } = render(<ControlledEditor chrome={FRENCH_CHROME} />);

    expect(container.querySelectorAll('[aria-label="Mise en forme Markdown"]')).toHaveLength(1);
    expect(container.querySelectorAll('[aria-label="Markdown formatting"]')).toHaveLength(0);
    expect(screen.getByRole("button", { name: "Gras" })).toBeTruthy();
    expect(screen.getByRole("button", { name: "Italique" })).toBeTruthy();
    expect(screen.getByRole("button", { name: "Lien" })).toBeTruthy();
    expect(screen.getByRole("button", { name: "Liste" })).toBeTruthy();
    expect(screen.queryByRole("button", { name: "Bold" })).toBeNull();
    expect(screen.getByRole("combobox", { name: "Insérer un titre" })).toBeTruthy();
    expect(screen.getByText("Titre…")).toBeTruthy();
    expect(screen.getByRole("option", { name: "Titre 2 (##)" })).toBeTruthy();
    expect(screen.getByRole("option", { name: "Titre 6 (######)" })).toBeTruthy();
    expect(screen.queryByRole("option", { name: "Heading 2 (##)" })).toBeNull();
    const help = screen.getByRole("button", { name: "Aide sur les titres" });
    expect(help.getAttribute("title")).toBe(FRENCH_CHROME.headingHelpTitle);
  });

  it("keeps generated Markdown byte-identical regardless of the chrome language", async () => {
    const onCommit = vi.fn<(value: string) => void>();
    render(<ControlledEditor chrome={FRENCH_CHROME} onCommit={onCommit} />);
    const textarea = screen.getByLabelText("Body") as HTMLTextAreaElement;

    fireEvent.click(screen.getByRole("button", { name: "Gras" }));
    fireEvent.change(textarea, { target: { value: "" } });
    fireEvent.click(screen.getByRole("button", { name: "Italique" }));
    fireEvent.change(textarea, { target: { value: "" } });
    fireEvent.click(screen.getByRole("button", { name: "Liste" }));
    fireEvent.change(textarea, { target: { value: "" } });
    fireEvent.change(screen.getByRole("combobox", { name: "Insérer un titre" }), {
      target: { value: "2" },
    });
    fireEvent.change(textarea, { target: { value: "" } });
    fireEvent.click(screen.getByRole("button", { name: "Lien" }));

    // Let the toolbar's pending requestAnimationFrame selection-restore
    // callbacks run while the document is still attached.
    await new Promise((resolve) => requestAnimationFrame(() => resolve(undefined)));

    // Same English content placeholders the untranslated editor inserts:
    // localization changes the labels only, never the generated source.
    const committed = onCommit.mock.calls.map(([next]) => next);
    expect(committed).toContain("**bold text**");
    expect(committed).toContain("*italic text*");
    expect(committed).toContain("- list item");
    expect(committed).toContain("## Heading");
    expect(committed).toContain("[cursor here](link_here)");
  });
});

describe("MarkdownEditor keyboard editing", () => {
  it("continues unordered, ordered, and blockquote lines", () => {
    const onCommit = vi.fn<(value: string) => void>();
    render(<ControlledEditor onCommit={onCommit} />);
    const textarea = screen.getByLabelText("Body") as HTMLTextAreaElement;

    fireEvent.change(textarea, { target: { value: "* first" } });
    textarea.setSelectionRange(7, 7);
    fireEvent.keyDown(textarea, { key: "Enter" });
    expect(onCommit).toHaveBeenLastCalledWith("* first\n* ");

    fireEvent.change(textarea, { target: { value: "2) first" } });
    textarea.setSelectionRange(8, 8);
    fireEvent.keyDown(textarea, { key: "Enter" });
    expect(onCommit).toHaveBeenLastCalledWith("2) first\n3) ");

    fireEvent.change(textarea, { target: { value: "> quoted" } });
    textarea.setSelectionRange(8, 8);
    fireEvent.keyDown(textarea, { key: "Enter" });
    expect(onCommit).toHaveBeenLastCalledWith("> quoted\n> ");
  });

  it("indents selected lines, pairs symbols, wraps selections, and skips closers", () => {
    const onCommit = vi.fn<(value: string) => void>();
    render(<ControlledEditor onCommit={onCommit} />);
    const textarea = screen.getByLabelText("Body") as HTMLTextAreaElement;

    fireEvent.change(textarea, { target: { value: "one\ntwo" } });
    textarea.setSelectionRange(0, 7);
    fireEvent.keyDown(textarea, { key: "Tab" });
    expect(onCommit).toHaveBeenLastCalledWith("  one\n  two");

    fireEvent.change(textarea, { target: { value: "word" } });
    textarea.setSelectionRange(0, 4);
    fireEvent.keyDown(textarea, { key: "(" });
    expect(onCommit).toHaveBeenLastCalledWith("(word)");

    fireEvent.change(textarea, { target: { value: "" } });
    textarea.setSelectionRange(0, 0);
    fireEvent.keyDown(textarea, { key: "[" });
    expect(onCommit).toHaveBeenLastCalledWith("[]");
    textarea.setSelectionRange(1, 1);
    fireEvent.keyDown(textarea, { key: "]" });
    expect(onCommit).toHaveBeenLastCalledWith("[]");
  });

  it("keeps native spellcheck enabled and reveals the Windows dictionary action only for a selection", () => {
    render(<ControlledEditor />);
    const textarea = screen.getByLabelText("Body") as HTMLTextAreaElement;
    fireEvent.change(textarea, { target: { value: "text" } });
    textarea.setSelectionRange(0, 4);
    fireEvent.keyDown(textarea, { key: "b", ctrlKey: true });
    expect(textarea.value).toBe("**text**");

    expect(textarea.lang).toBe("en-US");
    expect(textarea.getAttribute("spellcheck")).toBe("true");
    expect(screen.queryByRole("button", { name: "Add to Windows dictionary" })).toBeNull();
    textarea.setSelectionRange(0, 8);
    fireEvent.select(textarea);
    expect(screen.getByRole("button", { name: "Add to Windows dictionary" })).toBeTruthy();
  });
});
