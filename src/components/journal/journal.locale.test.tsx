import i18n from "i18next";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { act, fireEvent, render, screen, waitFor } from "@testing-library/react";
import JournalForm from "@/components/journal/JournalForm";
import { exportTemplateCatalog } from "@/lib/api/exportApi";
import { SUPPORTED_LOCALES, type AppLocale } from "@/lib/i18n/locale";

// 8.6.10 Journal locale coverage: the journal form renders every
// application-owned label, placeholder, helper, privacy/exportability copy,
// template-field chrome, and submit action from the active locale; blank-title
// validation keeps working in every language; and the shared Markdown preview
// and editor chrome localizes for this surface through the established opt-in
// props. Runtime language switches never translate authored prose, emotions,
// or template values. Synthetic fixtures only.

vi.mock("@/lib/api/exportApi", () => ({
  exportTemplateCatalog: vi.fn(),
}));

const cleanup = {
  remove_comments: false,
  remove_guide_blockquotes: false,
  remove_response_markers: false,
  remove_placeholders: false,
};

// Fixture includes the removed legacy body field and a non-dedicated select
// field, so the legacy field stays hidden and the ID-derived fallback remains
// exercised.
const chromeTemplate = {
  id: "foundation-journal",
  version: "1.0.1",
  display_name: "Synthetic journal",
  locale: "en",
  description: "Synthetic fixture template",
  min_app_version: "0.0.0",
  fields: [
    { id: "entry.free", field_type: "markdown", required: false },
    {
      id: "entry.focus",
      field_type: "single_select",
      required: false,
      options: ["Synthetic option"],
    },
  ],
  sections: [],
  cleanup,
};

interface JournalFormCopy {
  title: string;
  date: string;
  content: string;
  mood: string;
  moodPlaceholder: string;
  energy: string;
  energyPlaceholder: string;
  emotions: string;
  emotionsPlaceholder: string;
  emotionsHelp: string;
  template: string;
  templateFallbackOption: string;
  exportable: string;
  exportableHelp: string;
  saveSubmit: string;
  selectOption: string;
  previewLabel: string;
  previewEmpty: string;
  previewRegion: string;
  toolbarLabel: string;
  bold: string;
  italic: string;
  link: string;
  list: string;
  headingSelect: string;
  headingPlaceholder: string;
  headingOption2: string;
  headingHelp: string;
}

const COPY: Record<AppLocale, JournalFormCopy> = {
  "en-US": {
    title: "Title",
    date: "Date",
    content: "Content (Markdown)",
    mood: "Mood",
    moodPlaceholder: "e.g. calm, tired",
    energy: "Energy",
    energyPlaceholder: "e.g. low, high",
    emotions: "Emotions",
    emotionsPlaceholder: "e.g. calm, grateful, tired",
    emotionsHelp: "Free text, any words you choose. Separate multiple emotions with commas.",
    template: "Journal template",
    templateFallbackOption: "Foundation journal (1.0.1)",
    exportable: "Exportable",
    exportableHelp:
      "Entries always stay local and private on this computer. Exportable only allows this entry to be included when you explicitly export it; it never makes the entry public. Disabling this prevents the entry from being exported.",
    saveSubmit: "Save entry",
    selectOption: "Select…",
    previewLabel: "Preview",
    previewEmpty: "No content yet.",
    previewRegion: "{{label}} preview",
    toolbarLabel: "Markdown formatting",
    bold: "Bold",
    italic: "Italic",
    link: "Link",
    list: "List",
    headingSelect: "Insert heading",
    headingPlaceholder: "Heading…",
    headingOption2: "Heading 2 (##)",
    headingHelp: "Heading help",
  },
  "es-ES": {
    title: "Título",
    date: "Fecha",
    content: "Contenido (Markdown)",
    mood: "Ánimo",
    moodPlaceholder: "p. ej.: tranquila, cansada",
    energy: "Energía",
    energyPlaceholder: "p. ej.: baja, alta",
    emotions: "Emociones",
    emotionsPlaceholder: "p. ej.: tranquila, agradecida, cansada",
    emotionsHelp: "Texto libre, las palabras que quieras. Separa varias emociones con comas.",
    template: "Plantilla de diario",
    templateFallbackOption: "Diario base (1.0.1)",
    exportable: "Exportable",
    exportableHelp:
      "Las entradas siempre permanecen locales y privadas en este equipo. «Exportable» solo permite incluir esta entrada cuando la exportas de forma explícita; nunca la hace pública. Desactivar esto impide que la entrada se exporte.",
    saveSubmit: "Guardar entrada",
    selectOption: "Seleccionar…",
    previewLabel: "Vista previa",
    previewEmpty: "Aún no hay contenido.",
    previewRegion: "Vista previa de {{label}}",
    toolbarLabel: "Formato de Markdown",
    bold: "Negrita",
    italic: "Cursiva",
    link: "Enlace",
    list: "Lista",
    headingSelect: "Insertar encabezado",
    headingPlaceholder: "Encabezado…",
    headingOption2: "Encabezado 2 (##)",
    headingHelp: "Ayuda sobre encabezados",
  },
  "de-DE": {
    title: "Titel",
    date: "Datum",
    content: "Inhalt (Markdown)",
    mood: "Stimmung",
    moodPlaceholder: "z. B. ruhig, müde",
    energy: "Energie",
    energyPlaceholder: "z. B. niedrig, hoch",
    emotions: "Emotionen",
    emotionsPlaceholder: "z. B. ruhig, dankbar, müde",
    emotionsHelp: "Freitext, beliebige Wörter. Trenne mehrere Emotionen mit Kommas.",
    template: "Tagebuchvorlage",
    templateFallbackOption: "Basis-Tagebuch (1.0.1)",
    exportable: "Exportierbar",
    exportableHelp:
      "Einträge bleiben immer lokal und privat auf diesem Computer. «Exportierbar» erlaubt nur, diesen Eintrag bei einem ausdrücklichen Export einzubeziehen; öffentlich wird er dadurch nie. Wenn du dies deaktivierst, kann der Eintrag nicht exportiert werden.",
    saveSubmit: "Eintrag speichern",
    selectOption: "Auswählen…",
    previewLabel: "Vorschau",
    previewEmpty: "Noch kein Inhalt.",
    previewRegion: "Vorschau von {{label}}",
    toolbarLabel: "Markdown-Formatierung",
    bold: "Fett",
    italic: "Kursiv",
    link: "Link",
    list: "Liste",
    headingSelect: "Überschrift einfügen",
    headingPlaceholder: "Überschrift…",
    headingOption2: "Überschrift 2 (##)",
    headingHelp: "Hilfe zu Überschriften",
  },
  "fr-FR": {
    title: "Titre",
    date: "Date",
    content: "Contenu (Markdown)",
    mood: "Humeur",
    moodPlaceholder: "p. ex. calme, fatiguée",
    energy: "Énergie",
    energyPlaceholder: "p. ex. basse, haute",
    emotions: "Émotions",
    emotionsPlaceholder: "p. ex. calme, reconnaissante, fatiguée",
    emotionsHelp:
      "Texte libre, les mots de votre choix. Séparez plusieurs émotions par des virgules.",
    template: "Modèle de journal",
    templateFallbackOption: "Journal de base (1.0.1)",
    exportable: "Exportable",
    exportableHelp:
      "Les entrées restent toujours locales et privées sur cet ordinateur. « Exportable » permet seulement d'inclure cette entrée lors d'un export explicite ; cela ne la rend jamais publique. Désactiver cette option empêche l'export de l'entrée.",
    saveSubmit: "Enregistrer l'entrée",
    selectOption: "Sélectionner…",
    previewLabel: "Aperçu",
    previewEmpty: "Aucun contenu pour le moment.",
    previewRegion: "Aperçu de {{label}}",
    toolbarLabel: "Mise en forme Markdown",
    bold: "Gras",
    italic: "Italique",
    link: "Lien",
    list: "Liste",
    headingSelect: "Insérer un titre",
    headingPlaceholder: "Titre…",
    headingOption2: "Titre 2 (##)",
    headingHelp: "Aide sur les titres",
  },
};

describe.each([...SUPPORTED_LOCALES])("JournalForm in %s", (locale) => {
  beforeEach(() => {
    vi.mocked(exportTemplateCatalog).mockReset();
    vi.mocked(exportTemplateCatalog).mockResolvedValue([]);
  });

  it("renders translated labels, placeholders, privacy copy, and the submit action", async () => {
    await i18n.changeLanguage(locale);
    const e = COPY[locale];

    render(<JournalForm onSubmit={vi.fn()} />);

    expect(screen.getByLabelText(e.title)).toBeTruthy();
    expect(screen.getByLabelText(e.date)).toBeTruthy();
    expect(screen.getByLabelText(e.content)).toBeTruthy();
    expect(screen.getByLabelText(e.mood)).toBeTruthy();
    expect(screen.getByLabelText(e.energy)).toBeTruthy();
    expect(screen.getByLabelText(e.emotions)).toBeTruthy();
    expect(screen.getByLabelText(e.template)).toBeTruthy();

    // Placeholders are translated UI chrome; they never enter stored values.
    const mood = screen.getByLabelText(e.mood) as HTMLInputElement;
    const energy = screen.getByLabelText(e.energy) as HTMLInputElement;
    const emotions = screen.getByLabelText(e.emotions) as HTMLInputElement;
    expect(mood.placeholder).toBe(e.moodPlaceholder);
    expect(energy.placeholder).toBe(e.energyPlaceholder);
    expect(emotions.placeholder).toBe(e.emotionsPlaceholder);

    // Validation helper and the privacy/exportability explanation render
    // translated, byte-for-byte from the catalog.
    expect(screen.getByText(e.emotionsHelp)).toBeTruthy();
    expect(screen.getByText(e.exportableHelp)).toBeTruthy();
    expect(screen.getByLabelText(e.exportable)).toBeTruthy();

    expect(screen.getByRole("button", { name: e.saveSubmit })).toBeTruthy();

    // The catalog-unavailable template fallback label is application-owned
    // chrome and translates; the option value stays pinned to the bundled
    // foundation-journal identity (8.6.10 follow-up).
    const fallback = screen.getByRole("option", { name: e.templateFallbackOption });
    expect((fallback as HTMLOptionElement).value).toBe("foundation-journal");
  });

  it("blocks submission when the title is blank (validation stays locale-neutral)", async () => {
    await i18n.changeLanguage(locale);
    const e = COPY[locale];
    const onSubmit = vi.fn();

    render(<JournalForm onSubmit={onSubmit} />);
    fireEvent.click(screen.getByRole("button", { name: e.saveSubmit }));

    expect(onSubmit).not.toHaveBeenCalled();
  });

  it("pairs the content field with the localized preview and editor chrome", async () => {
    await i18n.changeLanguage(locale);
    const e = COPY[locale];

    const { container } = render(<JournalForm onSubmit={vi.fn()} />);

    // Shared MarkdownField opt-in chrome (8.6.10).
    expect(screen.getByText(e.previewLabel)).toBeTruthy();
    expect(screen.getByText(e.previewEmpty)).toBeTruthy();
    expect(
      screen.getByRole("region", {
        name: e.previewRegion.replace("{{label}}", e.content),
      }),
    ).toBeTruthy();

    // Shared MarkdownEditor opt-in toolbar chrome.
    expect(container.querySelectorAll(`[aria-label="${e.toolbarLabel}"]`)).toHaveLength(1);
    expect(screen.getByRole("button", { name: e.bold })).toBeTruthy();
    expect(screen.getByRole("button", { name: e.italic })).toBeTruthy();
    expect(screen.getByRole("button", { name: e.link })).toBeTruthy();
    expect(screen.getByRole("button", { name: e.list })).toBeTruthy();
    expect(screen.getByRole("combobox", { name: e.headingSelect })).toBeTruthy();
    expect(screen.getByText(e.headingPlaceholder)).toBeTruthy();
    expect(screen.getByRole("option", { name: e.headingOption2 })).toBeTruthy();
    expect(screen.getByRole("button", { name: e.headingHelp })).toBeTruthy();
  });

  it("renders translated template-field chrome over verbatim manifest data", async () => {
    await i18n.changeLanguage(locale);
    const e = COPY[locale];
    vi.mocked(exportTemplateCatalog).mockResolvedValue([chromeTemplate] as never);

    render(<JournalForm onSubmit={vi.fn()} />);

    // The legacy body field stays hidden; the select placeholder is
    // application chrome and translates, while manifest data stays verbatim.
    expect(await screen.findByText("Synthetic journal (1.0.1)")).toBeTruthy();
    expect(screen.queryByLabelText(/^(free|frei|libre)$/i)).toBeNull();
    expect(screen.getByRole("option", { name: e.selectOption })).toBeTruthy();
    expect(screen.getByRole("option", { name: "Synthetic option" })).toBeTruthy();
    // Unknown manifest IDs keep the ID-derived fallback label (never a
    // translation of template content).
    expect(screen.getByLabelText("Focus")).toBeTruthy();
  });
});

describe("JournalForm runtime locale switching", () => {
  beforeEach(() => {
    vi.mocked(exportTemplateCatalog).mockReset();
    vi.mocked(exportTemplateCatalog).mockResolvedValue([]);
  });

  it("re-renders chrome in the new language and keeps authored values byte-exact", async () => {
    const onSubmit = vi.fn();
    const raw = "# Diario sin tocar\n\n*prose stays verbatim*";
    render(<JournalForm onSubmit={onSubmit} />);

    fireEvent.change(screen.getByLabelText("Title"), { target: { value: "A quiet day" } });
    fireEvent.change(screen.getByLabelText("Date"), { target: { value: "2026-09-08" } });
    fireEvent.change(screen.getByLabelText("Content (Markdown)"), { target: { value: raw } });
    fireEvent.change(screen.getByLabelText("Emotions"), { target: { value: "calma, nostalgia" } });
    fireEvent.change(screen.getByLabelText("Mood"), { target: { value: "tranquila" } });

    await act(async () => {
      await i18n.changeLanguage("es-ES");
    });
    await waitFor(() => expect(screen.getByLabelText("Título")).toBeTruthy());
    expect(screen.queryByLabelText("Title")).toBeNull();

    // The preview region chrome follows the switch; the authored body is not
    // re-rendered or transformed by the locale change.
    expect(
      screen.getByRole("region", { name: "Vista previa de Contenido (Markdown)" }),
    ).toBeTruthy();
    expect(screen.getByRole("button", { name: "Guardar entrada" })).toBeTruthy();

    const content = screen.getByLabelText("Contenido (Markdown)") as HTMLTextAreaElement;
    const emotions = screen.getByLabelText("Emociones") as HTMLInputElement;
    const mood = screen.getByLabelText("Ánimo") as HTMLInputElement;
    expect(content.value).toBe(raw);
    expect(emotions.value).toBe("calma, nostalgia");
    expect(mood.value).toBe("tranquila");

    fireEvent.click(screen.getByRole("button", { name: "Guardar entrada" }));
    await waitFor(() => expect(onSubmit).toHaveBeenCalledTimes(1));
    expect(onSubmit.mock.calls[0][0]).toEqual(
      expect.objectContaining({
        title: "A quiet day",
        occurred_on: "2026-09-08",
        content_markdown: raw,
        emotions: ["calma", "nostalgia"],
        mood_label: "tranquila",
      }),
    );
  });
});

afterEach(async () => {
  // The tree may still be mounted here (afterEach runs before RTL's
  // auto-cleanup), so reset the language inside act.
  await act(async () => {
    await i18n.changeLanguage("en-US");
  });
});
