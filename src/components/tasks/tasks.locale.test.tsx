import i18n from "i18next";
import { afterEach, describe, expect, it, vi } from "vitest";
import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import DependencyEditor from "@/components/tasks/DependencyEditor";
import PriorityEffortMatrix from "@/components/tasks/PriorityEffortMatrix";
import TaskForm from "@/components/tasks/TaskForm";
import { SUPPORTED_LOCALES, type AppLocale } from "@/lib/i18n/locale";

// 8.6.9 Tasks planning-forms locale coverage: the task form labels (notes,
// effort, scheduled date), the priority/effort enum options, and the whole
// priority-effort matrix chrome come from the active locale; blank-title
// validation keeps working everywhere; the dependency editor's selector,
// add/remove actions, completion badge, and empty state localize as well.
// Candidate task titles and authored Markdown stay verbatim. Synthetic
// fixtures only.

vi.mock("@/lib/api/taskApi", () => ({
  addDependency: vi.fn(),
  listAllPrerequisites: vi.fn(),
  listTasks: vi.fn(),
  removeDependency: vi.fn(),
}));

import { listAllPrerequisites, listTasks } from "@/lib/api/taskApi";

interface TaskFormCopy {
  title: string;
  description: string;
  notes: string;
  priority: string;
  effortSize: string;
  effortNotSet: string;
  targetDate: string;
  scheduledDate: string;
  submit: string;
  previewLabel: string;
  previewRegion: string;
  previewEmpty: string;
  toolbarLabel: string;
  bold: string;
  italic: string;
  link: string;
  list: string;
  headingSelect: string;
  headingPlaceholder: string;
  headingOption2: string;
  headingHelp: string;
  headingHelpTitle: string;
  matrixTitle: string;
  corner: string;
  priorityCritical: string;
  effortLarge: string;
  addressToday: string;
  pending: string;
}

const FORM: Record<AppLocale, TaskFormCopy> = {
  "en-US": {
    title: "Title",
    description: "Description (Markdown)",
    notes: "Notes (Markdown)",
    priority: "Priority",
    effortSize: "Effort size",
    effortNotSet: "Not set",
    targetDate: "Target date",
    scheduledDate: "Scheduled date",
    submit: "Save task",
    previewLabel: "Preview",
    previewRegion: "{{label}} preview",
    previewEmpty: "No content yet.",
    toolbarLabel: "Markdown formatting",
    bold: "Bold",
    italic: "Italic",
    link: "Link",
    list: "List",
    headingSelect: "Insert heading",
    headingPlaceholder: "Heading…",
    headingOption2: "Heading 2 (##)",
    headingHelp: "Heading help",
    headingHelpTitle:
      "You don't need to use # for the title: the Title field already adds it. Use ## or deeper levels only for subheadings inside the content.",
    matrixTitle: "Priority and effort matrix",
    corner: "Effort / Priority",
    priorityCritical: "Critical",
    effortLarge: "Large",
    addressToday: "Address today",
    pending: "Pending",
  },
  "es-ES": {
    title: "Título",
    description: "Descripción (Markdown)",
    notes: "Notas (Markdown)",
    priority: "Prioridad",
    effortSize: "Tamaño de esfuerzo",
    effortNotSet: "Sin definir",
    targetDate: "Fecha prevista",
    scheduledDate: "Fecha programada",
    submit: "Guardar tarea",
    previewLabel: "Vista previa",
    previewRegion: "Vista previa de {{label}}",
    previewEmpty: "Aún no hay contenido.",
    toolbarLabel: "Formato de Markdown",
    bold: "Negrita",
    italic: "Cursiva",
    link: "Enlace",
    list: "Lista",
    headingSelect: "Insertar encabezado",
    headingPlaceholder: "Encabezado…",
    headingOption2: "Encabezado 2 (##)",
    headingHelp: "Ayuda sobre encabezados",
    headingHelpTitle:
      "No necesitas usar # para el título: el campo Título ya lo añade. Usa ## o niveles más profundos solo para subtítulos dentro del contenido.",
    matrixTitle: "Matriz de prioridad y esfuerzo",
    corner: "Esfuerzo / Prioridad",
    priorityCritical: "Crítica",
    effortLarge: "Grande",
    addressToday: "Atender hoy",
    pending: "Pendiente",
  },
  "de-DE": {
    title: "Titel",
    description: "Beschreibung (Markdown)",
    notes: "Notizen (Markdown)",
    priority: "Priorität",
    effortSize: "Aufwandsgröße",
    effortNotSet: "Nicht festgelegt",
    targetDate: "Zieldatum",
    scheduledDate: "Vorgesehener Tag",
    submit: "Aufgabe speichern",
    previewLabel: "Vorschau",
    previewRegion: "Vorschau von {{label}}",
    previewEmpty: "Noch kein Inhalt.",
    toolbarLabel: "Markdown-Formatierung",
    bold: "Fett",
    italic: "Kursiv",
    link: "Link",
    list: "Liste",
    headingSelect: "Überschrift einfügen",
    headingPlaceholder: "Überschrift…",
    headingOption2: "Überschrift 2 (##)",
    headingHelp: "Hilfe zu Überschriften",
    headingHelpTitle:
      "Für den Titel brauchst du kein #: Das Feld Titel fügt es bereits hinzu. Verwende ## oder tiefere Ebenen nur für Unterüberschriften innerhalb des Inhalts.",
    matrixTitle: "Prioritäten-Aufwands-Matrix",
    corner: "Aufwand / Priorität",
    priorityCritical: "Kritisch",
    effortLarge: "Groß",
    addressToday: "Heute erledigen",
    pending: "Ausstehend",
  },
  "fr-FR": {
    title: "Titre",
    description: "Description (Markdown)",
    notes: "Notes (Markdown)",
    priority: "Priorité",
    effortSize: "Taille d'effort",
    effortNotSet: "Non défini",
    targetDate: "Date prévue",
    scheduledDate: "Date programmée",
    submit: "Enregistrer la tâche",
    previewLabel: "Aperçu",
    previewRegion: "Aperçu de {{label}}",
    previewEmpty: "Aucun contenu pour le moment.",
    toolbarLabel: "Mise en forme Markdown",
    bold: "Gras",
    italic: "Italique",
    link: "Lien",
    list: "Liste",
    headingSelect: "Insérer un titre",
    headingPlaceholder: "Titre…",
    headingOption2: "Titre 2 (##)",
    headingHelp: "Aide sur les titres",
    headingHelpTitle:
      "Tu n'as pas besoin d'utiliser # pour le titre : le champ Titre l'ajoute déjà. Utilise ## ou des niveaux plus profonds uniquement pour les sous-titres à l'intérieur du contenu.",
    matrixTitle: "Matrice priorité-effort",
    corner: "Effort / Priorité",
    priorityCritical: "Critique",
    effortLarge: "Grande",
    addressToday: "Traiter aujourd'hui",
    pending: "En attente",
  },
};

interface DependencyCopy {
  addPrerequisite: string;
  selectPrerequisite: string;
  add: string;
  remove: string;
  noPrerequisites: string;
  completed: string;
  loading: string;
}

const DEPENDENCIES: Record<AppLocale, DependencyCopy> = {
  "en-US": {
    addPrerequisite: "Add prerequisite task",
    selectPrerequisite: "Select a prerequisite…",
    add: "Add",
    remove: "Remove",
    noPrerequisites: "No prerequisites defined.",
    completed: "Completed",
    loading: "Loading dependencies…",
  },
  "es-ES": {
    addPrerequisite: "Añadir tarea requisito previo",
    selectPrerequisite: "Selecciona un requisito previo…",
    add: "Añadir",
    remove: "Quitar",
    noPrerequisites: "No hay requisitos previos definidos.",
    completed: "Completada",
    loading: "Cargando dependencias…",
  },
  "de-DE": {
    addPrerequisite: "Aufgabe als Voraussetzung hinzufügen",
    selectPrerequisite: "Voraussetzung auswählen…",
    add: "Hinzufügen",
    remove: "Entfernen",
    noPrerequisites: "Keine Voraussetzungen definiert.",
    completed: "Abgeschlossen",
    loading: "Abhängigkeiten werden geladen…",
  },
  "fr-FR": {
    addPrerequisite: "Ajouter une tâche comme prérequis",
    selectPrerequisite: "Sélectionner un prérequis…",
    add: "Ajouter",
    remove: "Retirer",
    noPrerequisites: "Aucun prérequis défini.",
    completed: "Terminée",
    loading: "Chargement des dépendances…",
  },
};

const prerequisite = {
  dependency_id: "d1",
  task_id: "t9",
  title: "Comprar la tierra",
  status: "completed",
};

afterEach(async () => {
  await i18n.changeLanguage("en-US");
});

describe.each([...SUPPORTED_LOCALES])("TaskForm and matrix in %s", (locale) => {
  it("renders translated labels, enum options, and matrix chrome", async () => {
    await i18n.changeLanguage(locale);
    const e = FORM[locale];

    render(<TaskForm onSubmit={vi.fn()} />);

    expect(screen.getByLabelText(e.title)).toBeTruthy();
    expect(screen.getByLabelText(e.description)).toBeTruthy();
    expect(screen.getByLabelText(e.notes)).toBeTruthy();
    expect(screen.getByLabelText(e.priority)).toBeTruthy();
    expect(screen.getByLabelText(e.effortSize)).toBeTruthy();
    expect(screen.getByLabelText(e.targetDate)).toBeTruthy();
    expect(screen.getByLabelText(e.scheduledDate)).toBeTruthy();
    expect(screen.getByRole("button", { name: e.submit })).toBeTruthy();

    expect(screen.getByText(e.matrixTitle)).toBeTruthy();
    expect(screen.getByRole("columnheader", { name: e.corner })).toBeTruthy();
    expect(screen.getByRole("option", { name: e.effortNotSet })).toBeTruthy();
    expect(screen.getByRole("option", { name: e.priorityCritical })).toBeTruthy();

    // Matrix rows/columns/cells are catalog-driven, not raw enum strings.
    // "effortLarge" appears as both the select option and the matrix row
    // label, so assert the pair instead of a single unique match.
    expect(screen.getAllByText(e.effortLarge).length).toBeGreaterThanOrEqual(2);
    expect(screen.getByRole("columnheader", { name: e.priorityCritical })).toBeTruthy();
  });

  it("keeps matrix selection semantics while labels are localized", async () => {
    await i18n.changeLanguage(locale);
    const e = FORM[locale];
    const onSelect = vi.fn();

    render(<PriorityEffortMatrix effort={null} priority="none" onSelect={onSelect} />);
    fireEvent.click(screen.getByRole("button", { name: e.addressToday }));
    expect(onSelect).toHaveBeenCalledWith("trivial", "critical");
  });

  it("blocks submission when the title is blank", async () => {
    await i18n.changeLanguage(locale);
    const e = FORM[locale];
    const onSubmit = vi.fn();

    render(<TaskForm onSubmit={onSubmit} />);
    fireEvent.click(screen.getByRole("button", { name: e.submit }));
    expect(onSubmit).not.toHaveBeenCalled();
  });

  it("pairs both Markdown fields with localized named preview regions", async () => {
    await i18n.changeLanguage(locale);
    const e = FORM[locale];

    render(<TaskForm onSubmit={vi.fn()} />);

    expect(screen.getAllByText(e.previewLabel).length).toBe(2);
    expect(screen.getAllByText(e.previewEmpty).length).toBe(2);
    for (const label of [e.description, e.notes]) {
      expect(
        screen.getByRole("region", { name: e.previewRegion.replace("{{label}}", label) }),
      ).toBeTruthy();
    }
  });

  it("renders the localized Markdown editor chrome in both field toolbars", async () => {
    await i18n.changeLanguage(locale);
    const e = FORM[locale];

    const { container } = render(<TaskForm onSubmit={vi.fn()} />);

    // Every opt-in editor-chrome string (toolbar name, format buttons,
    // heading menu, help tooltip) comes from the active locale's catalog.
    expect(container.querySelectorAll(`[aria-label="${e.toolbarLabel}"]`)).toHaveLength(2);
    expect(screen.getAllByRole("button", { name: e.bold })).toHaveLength(2);
    expect(screen.getAllByRole("button", { name: e.italic })).toHaveLength(2);
    expect(screen.getAllByRole("button", { name: e.link })).toHaveLength(2);
    expect(screen.getAllByRole("button", { name: e.list })).toHaveLength(2);
    expect(screen.getAllByRole("combobox", { name: e.headingSelect })).toHaveLength(2);
    expect(screen.getAllByText(e.headingPlaceholder)).toHaveLength(2);
    expect(screen.getAllByRole("option", { name: e.headingOption2 })).toHaveLength(2);
    const help = screen.getAllByRole("button", { name: e.headingHelp });
    expect(help).toHaveLength(2);
    expect(help[0].getAttribute("title")).toBe(e.headingHelpTitle);
  });
});

describe.each([...SUPPORTED_LOCALES])("DependencyEditor in %s", (locale) => {
  it("localizes the selector, actions, completion badge, and empty state", async () => {
    await i18n.changeLanguage(locale);
    const e = DEPENDENCIES[locale];
    vi.mocked(listAllPrerequisites).mockReset();
    vi.mocked(listTasks).mockReset();
    vi.mocked(listAllPrerequisites).mockResolvedValue([prerequisite] as never);
    vi.mocked(listTasks).mockResolvedValue([] as never);

    const first = render(<DependencyEditor workspaceId="w1" taskId="t1" />);
    // The loading label appears before the mocked promises settle.
    expect(first.getByText(e.loading)).toBeTruthy();

    // The settled list then renders the localized chrome.
    expect(await screen.findByRole("combobox", { name: e.addPrerequisite })).toBeTruthy();
    expect(screen.getByText(e.completed)).toBeTruthy();
    expect(screen.getByRole("button", { name: e.remove })).toBeTruthy();
    expect(screen.getByRole("button", { name: e.add })).toBeTruthy();
    // Authored prerequisite titles stay verbatim.
    expect(screen.getByText("Comprar la tierra")).toBeTruthy();

    vi.mocked(listAllPrerequisites).mockResolvedValue([] as never);
    render(<DependencyEditor workspaceId="w1" taskId="t2" />);
    expect(await screen.findByText(e.noPrerequisites)).toBeTruthy();
  });
});

describe("Tasks runtime locale switching", () => {
  it("re-renders the form in the new language and submits authored Markdown verbatim", async () => {
    const onSubmit = vi.fn();
    const raw = "1. lista cruda\n\\# escape stays";
    render(<TaskForm onSubmit={onSubmit} />);

    fireEvent.change(screen.getByLabelText("Title"), { target: { value: "Divide the report" } });
    fireEvent.change(screen.getByLabelText("Notes (Markdown)"), { target: { value: raw } });

    await i18n.changeLanguage("de-DE");
    await waitFor(() => expect(screen.getByLabelText("Notizen (Markdown)")).toBeTruthy());

    // The shared preview chrome follows the switch; the authored source does
    // not (asserted below).
    expect(screen.getByRole("region", { name: "Vorschau von Notizen (Markdown)" })).toBeTruthy();

    // The localized editor toolbar chrome follows the same runtime switch.
    expect(screen.getAllByRole("button", { name: "Fett" })).toHaveLength(2);
    expect(screen.getAllByRole("combobox", { name: "Überschrift einfügen" })).toHaveLength(2);
    expect(screen.queryByRole("button", { name: "Bold" })).toBeNull();
    expect(screen.queryByText("Heading…")).toBeNull();

    const notes = screen.getByLabelText("Notizen (Markdown)") as HTMLTextAreaElement;
    expect(notes.value).toBe(raw);
    expect(screen.getByRole("button", { name: "Aufgabe speichern" })).toBeTruthy();

    fireEvent.click(screen.getByRole("button", { name: "Aufgabe speichern" }));
    await waitFor(() => expect(onSubmit).toHaveBeenCalledTimes(1));
    expect(onSubmit.mock.calls[0][0]).toEqual(
      expect.objectContaining({ title: "Divide the report", notes_markdown: raw }),
    );
  });
});
