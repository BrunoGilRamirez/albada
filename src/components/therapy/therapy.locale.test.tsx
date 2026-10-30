import i18n from "i18next";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { act, fireEvent, render, screen, waitFor } from "@testing-library/react";
import TherapyPrivacyBanner from "@/components/therapy/TherapyPrivacyBanner";
import TherapyRecordForm from "@/components/therapy/TherapyRecordForm";
import TherapyReferenceEditor from "@/components/therapy/TherapyReferenceEditor";
import { listReferencesForRecord } from "@/lib/api/therapyReferenceApi";
import { listTasks } from "@/lib/api/taskApi";
import { SUPPORTED_LOCALES, type AppLocale } from "@/lib/i18n/locale";

// 8.6.10 Therapy component locale coverage: the record form, the task
// reference editor, and the privacy banner render every application-owned
// label, enum option, placeholder, accessible validation, and Markdown
// chrome from the active catalog, while user-authored titles, Markdown, and
// notes stay verbatim. Therapy copy lives only in the `therapy` namespace.
// Synthetic fixtures only.

vi.mock("@/lib/api/therapyReferenceApi", () => ({
  addTherapyReference: vi.fn(),
  listReferencesForRecord: vi.fn(),
  removeTherapyReference: vi.fn(),
}));

// The reference editor offers linkable tasks from a `<select>`, so the
// workspace task list is the selector's data source.
vi.mock("@/lib/api/taskApi", () => ({
  listTasks: vi.fn(),
}));

interface TherapyCopy {
  banner: string;
  type: string;
  noteType: string;
  exerciseType: string;
  sessionNoteType: string;
  status: string;
  openStatus: string;
  completedStatus: string;
  title: string;
  occurredOn: string;
  content: string;
  saveSubmit: string;
  previewLabel: string;
  previewEmpty: string;
  previewRegion: string;
  toolbarLabel: string;
  bold: string;
  headingSelect: string;
  headingHelp: string;
  linkedTasks: string;
  taskId: string;
  taskIdPlaceholder: string;
  relation: string;
  supportsRelation: string;
  relatedToRelation: string;
  noteLabel: string;
  addReference: string;
  referenceEmpty: string;
  needTaskId: string;
}

const COPY: Record<AppLocale, TherapyCopy> = {
  "en-US": {
    banner:
      "Privacy mode: therapy records are kept separate from the executive dashboard and are not included in generic exports.",
    type: "Type",
    noteType: "Note",
    exerciseType: "Exercise",
    sessionNoteType: "Session note",
    status: "Status",
    openStatus: "Open",
    completedStatus: "Completed",
    title: "Title",
    occurredOn: "Occurred on",
    content: "Content (Markdown)",
    saveSubmit: "Save record",
    previewLabel: "Preview",
    previewEmpty: "No content yet.",
    previewRegion: "{{label}} preview",
    toolbarLabel: "Markdown formatting",
    bold: "Bold",
    headingSelect: "Insert heading",
    headingHelp: "Heading help",
    linkedTasks: "Linked tasks",
    taskId: "Task ID",
    taskIdPlaceholder: "task-uuid",
    relation: "Relation",
    supportsRelation: "Supports",
    relatedToRelation: "Related to",
    noteLabel: "Note (optional)",
    addReference: "Add reference",
    referenceEmpty: "No linked tasks yet.",
    needTaskId: "Please enter a task ID.",
  },
  "es-ES": {
    banner:
      "Modo de privacidad: los registros de terapia se mantienen separados del panel ejecutivo y no se incluyen en exportaciones genéricas.",
    type: "Tipo",
    noteType: "Nota",
    exerciseType: "Ejercicio",
    sessionNoteType: "Nota de sesión",
    status: "Estado",
    openStatus: "Abierto",
    completedStatus: "Completado",
    title: "Título",
    occurredOn: "Fecha del hecho",
    content: "Contenido (Markdown)",
    saveSubmit: "Guardar registro",
    previewLabel: "Vista previa",
    previewEmpty: "Aún no hay contenido.",
    previewRegion: "Vista previa de {{label}}",
    toolbarLabel: "Formato de Markdown",
    bold: "Negrita",
    headingSelect: "Insertar encabezado",
    headingHelp: "Ayuda sobre encabezados",
    linkedTasks: "Tareas vinculadas",
    taskId: "ID de tarea",
    taskIdPlaceholder: "task-uuid",
    relation: "Relación",
    supportsRelation: "Apoya",
    relatedToRelation: "Relacionado con",
    noteLabel: "Nota (opcional)",
    addReference: "Añadir referencia",
    referenceEmpty: "Aún no hay tareas vinculadas.",
    needTaskId: "Introduce un ID de tarea.",
  },
  "de-DE": {
    banner:
      "Privatsphäre-Modus: Therapienotizen bleiben vom Executive-Dashboard getrennt und werden nicht in allgemeine Exporte aufgenommen.",
    type: "Typ",
    noteType: "Notiz",
    exerciseType: "Übung",
    sessionNoteType: "Sitzungsnotiz",
    status: "Status",
    openStatus: "Offen",
    completedStatus: "Abgeschlossen",
    title: "Titel",
    occurredOn: "Datum des Geschehens",
    content: "Inhalt (Markdown)",
    saveSubmit: "Notiz speichern",
    previewLabel: "Vorschau",
    previewEmpty: "Noch kein Inhalt.",
    previewRegion: "Vorschau von {{label}}",
    toolbarLabel: "Markdown-Formatierung",
    bold: "Fett",
    headingSelect: "Überschrift einfügen",
    headingHelp: "Hilfe zu Überschriften",
    linkedTasks: "Verknüpfte Aufgaben",
    taskId: "Aufgaben-ID",
    taskIdPlaceholder: "task-uuid",
    relation: "Beziehung",
    supportsRelation: "Unterstützt",
    relatedToRelation: "Bezieht sich auf",
    noteLabel: "Notiz (optional)",
    addReference: "Referenz hinzufügen",
    referenceEmpty: "Noch keine verknüpften Aufgaben.",
    needTaskId: "Bitte gib eine Aufgaben-ID ein.",
  },
  "fr-FR": {
    banner:
      "Mode confidentialité : les dossiers de thérapie restent séparés du tableau de bord exécutif et ne sont pas inclus dans les exports génériques.",
    type: "Type",
    noteType: "Note",
    exerciseType: "Exercice",
    sessionNoteType: "Note de session",
    status: "Statut",
    openStatus: "Ouvert",
    completedStatus: "Terminé",
    title: "Titre",
    occurredOn: "Date des faits",
    content: "Contenu (Markdown)",
    saveSubmit: "Enregistrer le dossier",
    previewLabel: "Aperçu",
    previewEmpty: "Aucun contenu pour le moment.",
    previewRegion: "Aperçu de {{label}}",
    toolbarLabel: "Mise en forme Markdown",
    bold: "Gras",
    headingSelect: "Insérer un titre",
    headingHelp: "Aide sur les titres",
    linkedTasks: "Tâches liées",
    taskId: "ID de tâche",
    taskIdPlaceholder: "task-uuid",
    relation: "Relation",
    supportsRelation: "Soutient",
    relatedToRelation: "Lié à",
    noteLabel: "Note (facultatif)",
    addReference: "Ajouter la référence",
    referenceEmpty: "Aucune tâche liée pour le moment.",
    needTaskId: "Veuillez saisir un ID de tâche.",
  },
};

describe.each([...SUPPORTED_LOCALES])("Therapy components in %s", (locale) => {
  beforeEach(() => {
    localStorage.removeItem("albada-therapy-privacy-banner-dismissed");
    vi.mocked(listReferencesForRecord).mockReset();
    vi.mocked(listReferencesForRecord).mockResolvedValue([]);
    vi.mocked(listTasks).mockReset();
    vi.mocked(listTasks).mockResolvedValue([]);
  });

  it("shows the localized privacy banner", async () => {
    await i18n.changeLanguage(locale);

    render(<TherapyPrivacyBanner />);

    expect(screen.getByRole("note")).toHaveTextContent(COPY[locale].banner);
  });

  it("dismisses the privacy banner and remembers the choice", async () => {
    await i18n.changeLanguage(locale);

    const { rerender } = render(<TherapyPrivacyBanner />);
    fireEvent.click(screen.getByRole("button", { name: i18n.t("common.actions.close") }));

    expect(screen.queryByRole("note")).toBeNull();
    expect(localStorage.getItem("albada-therapy-privacy-banner-dismissed")).toBe("true");

    rerender(<TherapyPrivacyBanner />);
    expect(screen.queryByRole("note")).toBeNull();
  });

  it("renders localized record form labels, enum options, and validation", async () => {
    await i18n.changeLanguage(locale);
    const e = COPY[locale];
    const onSubmit = vi.fn();

    render(<TherapyRecordForm isEdit onSubmit={onSubmit} />);

    expect(screen.getByLabelText(e.title)).toBeTruthy();
    expect(screen.getByLabelText(e.type)).toBeTruthy();
    expect(screen.getByLabelText(e.occurredOn)).toBeTruthy();
    expect(screen.getByLabelText(e.content)).toBeTruthy();
    expect(screen.getByLabelText(e.status)).toBeTruthy();
    expect(screen.getByRole("button", { name: e.saveSubmit })).toBeTruthy();

    // Enum options render the localized therapy-domain labels, not raw values.
    expect(screen.getByRole("option", { name: e.noteType })).toBeTruthy();
    expect(screen.getByRole("option", { name: e.exerciseType })).toBeTruthy();
    expect(screen.getByRole("option", { name: e.sessionNoteType })).toBeTruthy();
    expect(screen.getByRole("option", { name: e.openStatus })).toBeTruthy();
    expect(screen.getByRole("option", { name: e.completedStatus })).toBeTruthy();

    // Blank-title validation stays locale-neutral: nothing submits.
    fireEvent.click(screen.getByRole("button", { name: e.saveSubmit }));
    expect(onSubmit).not.toHaveBeenCalled();
  });

  it("pairs the content field with the localized preview and editor chrome", async () => {
    await i18n.changeLanguage(locale);
    const e = COPY[locale];

    const { container } = render(<TherapyRecordForm onSubmit={vi.fn()} />);

    expect(screen.getByText(e.previewLabel)).toBeTruthy();
    expect(screen.getByText(e.previewEmpty)).toBeTruthy();
    expect(
      screen.getByRole("region", {
        name: e.previewRegion.replace("{{label}}", e.content),
      }),
    ).toBeTruthy();
    expect(container.querySelectorAll(`[aria-label="${e.toolbarLabel}"]`)).toHaveLength(1);
    expect(screen.getByRole("button", { name: e.bold })).toBeTruthy();
    expect(screen.getByRole("combobox", { name: e.headingSelect })).toBeTruthy();
    expect(screen.getByRole("button", { name: e.headingHelp })).toBeTruthy();
  });

  it("renders the localized reference editor with accessible validation", async () => {
    await i18n.changeLanguage(locale);
    const e = COPY[locale];

    render(<TherapyReferenceEditor recordId="tr1" workspaceId="w1" />);

    expect(await screen.findByText(e.linkedTasks)).toBeTruthy();
    expect(screen.getByText(e.referenceEmpty)).toBeTruthy();
    fireEvent.click(screen.getByRole("button", { name: e.addReference }));
    expect(screen.getByLabelText(e.taskId)).toBeTruthy();
    // The task field is a selector, not a free-text UUID box: the localized
    // placeholder is the first disabled option.
    expect(screen.getByRole("option", { name: e.taskIdPlaceholder })).toBeTruthy();
    expect(screen.getByLabelText(e.relation)).toBeTruthy();
    expect(screen.getByRole("option", { name: e.supportsRelation })).toBeTruthy();
    expect(screen.getByRole("option", { name: e.relatedToRelation })).toBeTruthy();
    expect(screen.getByLabelText(e.noteLabel)).toBeTruthy();
    expect(screen.getByRole("button", { name: e.addReference })).toBeTruthy();

    // The app-authored validation message localizes and never echoes any
    // record content.
    fireEvent.click(screen.getByRole("button", { name: e.addReference }));
    expect(await screen.findByText(e.needTaskId)).toBeTruthy();
  });
});

describe("TherapyRecordForm runtime locale switching", () => {
  it("re-renders the chrome and submits authored content byte-exact after switching", async () => {
    const onSubmit = vi.fn();
    const raw = "## Respirar en 4-7-8\n\n*synthetic exercise notes*";
    render(<TherapyRecordForm onSubmit={onSubmit} />);

    fireEvent.change(screen.getByLabelText("Title"), { target: { value: "Grounding plan" } });
    fireEvent.change(screen.getByLabelText("Content (Markdown)"), { target: { value: raw } });

    await act(async () => {
      await i18n.changeLanguage("de-DE");
    });
    await waitFor(() => expect(screen.getByLabelText("Titel")).toBeTruthy());
    expect(screen.getByRole("region", { name: "Vorschau von Inhalt (Markdown)" })).toBeTruthy();
    expect(screen.getByRole("button", { name: "Fett" })).toBeTruthy();
    expect(screen.queryByRole("button", { name: "Bold" })).toBeNull();

    const content = screen.getByLabelText("Inhalt (Markdown)") as HTMLTextAreaElement;
    expect(content.value).toBe(raw);

    fireEvent.click(screen.getByRole("button", { name: "Notiz speichern" }));
    await waitFor(() => expect(onSubmit).toHaveBeenCalledTimes(1));
    expect(onSubmit.mock.calls[0][0]).toEqual(
      expect.objectContaining({ title: "Grounding plan", content_markdown: raw }),
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
