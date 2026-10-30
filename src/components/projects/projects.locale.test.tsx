import i18n from "i18next";
import { afterEach, describe, expect, it, vi } from "vitest";
import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import AddExistingProjectModal from "@/components/projects/AddExistingProjectModal";
import ProjectForm from "@/components/projects/ProjectForm";
import { SUPPORTED_LOCALES, type AppLocale } from "@/lib/i18n/locale";

// 8.6.9 Projects locale coverage: the project form labels, enum options,
// submit action, and blank-title validation are catalog-driven in all four
// locales; the add-existing-project dialog renders localized search, close,
// add/adding, empty-state, and status badge strings while candidate project
// titles (user-authored) stay verbatim. Runtime switching re-renders both
// surfaces without remounting. Synthetic fixtures only.

interface ProjectFormCopy {
  title: string;
  description: string;
  why: string;
  dod: string;
  status: string;
  priority: string;
  targetDate: string;
  submit: string;
  createSubmit: string;
  statusActive: string;
  statusCompleted: string;
  priorityNormal: string;
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
}

const FORM: Record<AppLocale, ProjectFormCopy> = {
  "en-US": {
    title: "Title",
    description: "Description (Markdown)",
    why: "Why it matters (Markdown)",
    dod: "Definition of done (Markdown)",
    status: "Status",
    priority: "Priority",
    targetDate: "Target date",
    submit: "Save project",
    createSubmit: "Create project",
    statusActive: "Active",
    statusCompleted: "Completed",
    priorityNormal: "Normal",
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
  },
  "es-ES": {
    title: "Título",
    description: "Descripción (Markdown)",
    why: "Por qué importa (Markdown)",
    dod: "Definición de terminado (Markdown)",
    status: "Estado",
    priority: "Prioridad",
    targetDate: "Fecha prevista",
    submit: "Guardar proyecto",
    createSubmit: "Crear proyecto",
    statusActive: "Activo",
    statusCompleted: "Completado",
    priorityNormal: "Normal",
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
  },
  "de-DE": {
    title: "Titel",
    description: "Beschreibung (Markdown)",
    why: "Warum es wichtig ist (Markdown)",
    dod: "Fertigstellungskriterien (Markdown)",
    status: "Status",
    priority: "Priorität",
    targetDate: "Zieldatum",
    submit: "Projekt speichern",
    createSubmit: "Projekt erstellen",
    statusActive: "Aktiv",
    statusCompleted: "Abgeschlossen",
    priorityNormal: "Normal",
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
  },
  "fr-FR": {
    title: "Titre",
    description: "Description (Markdown)",
    why: "Pourquoi c'est important (Markdown)",
    dod: "Critères d'achèvement (Markdown)",
    status: "Statut",
    priority: "Priorité",
    targetDate: "Date prévue",
    submit: "Enregistrer le projet",
    createSubmit: "Créer un projet",
    statusActive: "Actif",
    statusCompleted: "Terminé",
    priorityNormal: "Normale",
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
  },
};

interface ModalCopy {
  dialogTitle: string;
  searchLabel: string;
  searchPlaceholder: string;
  empty: string;
  close: string;
  add: string;
  adding: string;
  statusPlanned: string;
}

const MODAL: Record<AppLocale, ModalCopy> = {
  "en-US": {
    dialogTitle: "Attach target",
    searchLabel: "Search projects",
    searchPlaceholder: "Search projects…",
    empty: "No eligible projects to link.",
    close: "Close",
    add: "Add",
    adding: "Adding…",
    statusPlanned: "Planned",
  },
  "es-ES": {
    dialogTitle: "Attach target",
    searchLabel: "Buscar proyectos",
    searchPlaceholder: "Buscar proyectos…",
    empty: "No hay proyectos elegibles para vincular.",
    close: "Cerrar",
    add: "Añadir",
    adding: "Añadiendo…",
    statusPlanned: "Planificado",
  },
  "de-DE": {
    dialogTitle: "Attach target",
    searchLabel: "Projekte suchen",
    searchPlaceholder: "Projekte suchen…",
    empty: "Keine für die Verknüpfung geeigneten Projekte.",
    close: "Schließen",
    add: "Hinzufügen",
    adding: "Wird hinzugefügt…",
    statusPlanned: "Geplant",
  },
  "fr-FR": {
    dialogTitle: "Attach target",
    searchLabel: "Rechercher des projets",
    searchPlaceholder: "Rechercher des projets…",
    empty: "Aucun projet éligible à lier.",
    close: "Fermer",
    add: "Ajouter",
    adding: "Ajout…",
    statusPlanned: "Planifié",
  },
};

const candidateProject = {
  id: "p1",
  workspace_id: "w1",
  goal_id: null,
  title: "Mi taller de cerámica",
  description_markdown: null,
  why_it_matters_markdown: null,
  definition_of_done_markdown: null,
  status: "planned",
  priority: "normal",
  target_date: null,
  created_at: "2026-09-08T10:00:00Z",
  updated_at: "2026-09-08T10:00:00Z",
  archived_at: null,
};

afterEach(async () => {
  await i18n.changeLanguage("en-US");
});

describe.each([...SUPPORTED_LOCALES])("ProjectForm in %s", (locale) => {
  it("renders translated field labels, enum options, and submit labels", async () => {
    await i18n.changeLanguage(locale);
    const e = FORM[locale];

    render(<ProjectForm editableManagement onSubmit={vi.fn()} />);

    expect(screen.getByLabelText(e.title)).toBeTruthy();
    expect(screen.getByLabelText(e.description)).toBeTruthy();
    expect(screen.getByLabelText(e.why)).toBeTruthy();
    expect(screen.getByLabelText(e.dod)).toBeTruthy();
    expect(screen.getByLabelText(e.status)).toBeTruthy();
    expect(screen.getByLabelText(e.priority)).toBeTruthy();
    expect(screen.getByLabelText(e.targetDate)).toBeTruthy();
    expect(screen.getByRole("button", { name: e.submit })).toBeTruthy();

    expect(screen.getByRole("option", { name: e.statusActive })).toBeTruthy();
    expect(screen.getByRole("option", { name: e.statusCompleted })).toBeTruthy();
    expect(screen.getByRole("option", { name: e.priorityNormal })).toBeTruthy();
  });

  it("renders the page-supplied create submit label", async () => {
    await i18n.changeLanguage(locale);
    const e = FORM[locale];

    render(<ProjectForm submitLabel={e.createSubmit} onSubmit={vi.fn()} />);
    expect(screen.getByRole("button", { name: e.createSubmit })).toBeTruthy();
  });

  it("blocks submission when the title is blank", async () => {
    await i18n.changeLanguage(locale);
    const e = FORM[locale];
    const onSubmit = vi.fn();

    render(<ProjectForm onSubmit={onSubmit} />);
    fireEvent.click(screen.getByRole("button", { name: e.submit }));
    expect(onSubmit).not.toHaveBeenCalled();
  });

  it("pairs each Markdown field with a localized named preview region", async () => {
    await i18n.changeLanguage(locale);
    const e = FORM[locale];

    render(<ProjectForm onSubmit={vi.fn()} />);

    expect(screen.getAllByText(e.previewLabel).length).toBe(3);
    expect(screen.getAllByText(e.previewEmpty).length).toBe(3);
    for (const label of [e.description, e.why, e.dod]) {
      expect(
        screen.getByRole("region", { name: e.previewRegion.replace("{{label}}", label) }),
      ).toBeTruthy();
    }
  });

  it("renders the localized Markdown editor chrome in all three field toolbars", async () => {
    await i18n.changeLanguage(locale);
    const e = FORM[locale];

    const { container } = render(<ProjectForm onSubmit={vi.fn()} />);

    // Every opt-in editor-chrome string (toolbar name, format buttons,
    // heading menu, help tooltip) comes from the active locale's catalog.
    expect(container.querySelectorAll(`[aria-label="${e.toolbarLabel}"]`)).toHaveLength(3);
    expect(screen.getAllByRole("button", { name: e.bold })).toHaveLength(3);
    expect(screen.getAllByRole("button", { name: e.italic })).toHaveLength(3);
    expect(screen.getAllByRole("button", { name: e.link })).toHaveLength(3);
    expect(screen.getAllByRole("button", { name: e.list })).toHaveLength(3);
    expect(screen.getAllByRole("combobox", { name: e.headingSelect })).toHaveLength(3);
    expect(screen.getAllByText(e.headingPlaceholder)).toHaveLength(3);
    expect(screen.getAllByRole("option", { name: e.headingOption2 })).toHaveLength(3);
    const help = screen.getAllByRole("button", { name: e.headingHelp });
    expect(help).toHaveLength(3);
    expect(help[0].getAttribute("title")).toBe(e.headingHelpTitle);
  });
});

describe.each([...SUPPORTED_LOCALES])("AddExistingProjectModal in %s", (locale) => {
  it("localizes search, empty state, and close chrome", async () => {
    await i18n.changeLanguage(locale);
    const e = MODAL[locale];

    render(
      <AddExistingProjectModal
        open
        title={e.dialogTitle}
        projects={[]}
        onAttach={vi.fn()}
        onClose={vi.fn()}
      />,
    );

    expect(screen.getByRole("dialog", { name: e.dialogTitle })).toBeTruthy();
    expect(screen.getByRole("searchbox", { name: e.searchLabel })).toBeTruthy();
    expect(screen.getByPlaceholderText(e.searchPlaceholder)).toBeTruthy();
    expect(screen.getByText(e.empty)).toBeTruthy();
    expect(screen.getByRole("button", { name: e.close })).toBeTruthy();
  });

  it("localizes the add action and status badge while the authored title stays verbatim", async () => {
    await i18n.changeLanguage(locale);
    const e = MODAL[locale];

    render(
      <AddExistingProjectModal
        open
        title={e.dialogTitle}
        projects={[candidateProject] as never}
        onAttach={vi.fn()}
        onClose={vi.fn()}
      />,
    );

    expect(await screen.findByText("Mi taller de cerámica")).toBeTruthy();
    expect(screen.getByText(e.statusPlanned)).toBeTruthy();
    expect(screen.getByRole("button", { name: e.add })).toBeTruthy();
  });
});

describe("Projects runtime locale switching", () => {
  it("re-renders the form labels in the new language and submits Markdown verbatim", async () => {
    const onSubmit = vi.fn();
    const raw = "### estado bruto\n- [ ] sin traducir";
    render(<ProjectForm editableManagement onSubmit={onSubmit} />);

    fireEvent.change(screen.getByLabelText("Title"), { target: { value: "Raw stays" } });
    fireEvent.change(screen.getByLabelText("Definition of done (Markdown)"), {
      target: { value: raw },
    });

    await i18n.changeLanguage("es-ES");
    await waitFor(() =>
      expect(screen.getByLabelText("Definición de terminado (Markdown)")).toBeTruthy(),
    );

    // The shared preview chrome follows the switch; the authored source does
    // not (asserted below).
    expect(
      screen.getByRole("region", { name: "Vista previa de Definición de terminado (Markdown)" }),
    ).toBeTruthy();

    // The localized editor toolbar chrome follows the same runtime switch.
    expect(screen.getAllByRole("button", { name: "Negrita" })).toHaveLength(3);
    expect(screen.getAllByRole("combobox", { name: "Insertar encabezado" })).toHaveLength(3);
    expect(screen.queryByRole("button", { name: "Bold" })).toBeNull();
    expect(screen.queryByText("Heading…")).toBeNull();

    const dod = screen.getByLabelText("Definición de terminado (Markdown)") as HTMLTextAreaElement;
    expect(dod.value).toBe(raw);
    expect(screen.getByRole("button", { name: "Guardar proyecto" })).toBeTruthy();
    expect(screen.queryByRole("button", { name: "Save project" })).toBeNull();

    fireEvent.click(screen.getByRole("button", { name: "Guardar proyecto" }));
    await waitFor(() => expect(onSubmit).toHaveBeenCalledTimes(1));
    expect(onSubmit.mock.calls[0][0]).toEqual(
      expect.objectContaining({
        title: "Raw stays",
        definition_of_done_markdown: raw,
      }),
    );
  });
});
