import i18n from "i18next";
import { afterEach, describe, expect, it, vi } from "vitest";
import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { MemoryRouter } from "react-router";
import GoalForm from "@/components/goals/GoalForm";
import ProgressDetailsModal from "@/components/progress/ProgressDetailsModal";
import { SUPPORTED_LOCALES, type AppLocale } from "@/lib/i18n/locale";

// 8.6.9 Goals locale coverage: the goal form renders every application-owned
// label, enum option, and submit action from the active locale, blank-title
// validation keeps working in every language, user-authored Markdown stays
// verbatim while switching languages at runtime, and the shared progress
// surface (ProgressDetailsModal, rendered by the Goals and Projects screens)
// localizes its pluralized effort summary. Synthetic fixtures only.
//
// The preview pane chrome (visible label, region name, empty placeholder)
// comes from the shared `common.markdown` keys the form passes into the
// reusable MarkdownField; these tests cover the localized chrome in all four
// locales.

interface GoalFormCopy {
  title: string;
  description: string;
  why: string;
  success: string;
  status: string;
  priority: string;
  targetDate: string;
  submitDefault: string;
  statusActive: string;
  statusAchieved: string;
  priorityCritical: string;
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
  progressExplainer: string;
  progressSummaryTwo: string;
  progressSummaryOne: string;
  progressClose: string;
  detailsFor: string;
  progressLoading: string;
  done: string;
  pointsValue: string;
}

const COPY: Record<AppLocale, GoalFormCopy> = {
  "en-US": {
    title: "Title",
    description: "Description (Markdown)",
    why: "Why it matters (Markdown)",
    success: "Success definition (Markdown)",
    status: "Status",
    priority: "Priority",
    targetDate: "Target date",
    submitDefault: "Save goal",
    statusActive: "Active",
    statusAchieved: "Achieved",
    priorityCritical: "Critical",
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
    progressExplainer: "Progress comes from eligible leaf tasks weighted by effort points.",
    progressSummaryTwo: "5 of 8 effort points completed across 2 leaf tasks.",
    progressSummaryOne: "5 of 8 effort points completed across 1 leaf task.",
    progressClose: "Close progress details",
    detailsFor: "Progress details for Synthetic goal",
    progressLoading: "Loading progress details…",
    done: "done",
    pointsValue: "5 pts",
  },
  "es-ES": {
    title: "Título",
    description: "Descripción (Markdown)",
    why: "Por qué importa (Markdown)",
    success: "Definición del éxito (Markdown)",
    status: "Estado",
    priority: "Prioridad",
    targetDate: "Fecha prevista",
    submitDefault: "Guardar objetivo",
    statusActive: "Activo",
    statusAchieved: "Alcanzado",
    priorityCritical: "Crítica",
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
    progressExplainer:
      "El progreso proviene de las tareas hoja elegibles, ponderadas por puntos de esfuerzo.",
    progressSummaryTwo: "5 de 8 puntos de esfuerzo completados en 2 tareas hoja.",
    progressSummaryOne: "5 de 8 puntos de esfuerzo completados en 1 tarea hoja.",
    progressClose: "Cerrar detalles del progreso",
    detailsFor: "Detalles del progreso de Synthetic goal",
    progressLoading: "Cargando detalles del progreso…",
    done: "hecha",
    pointsValue: "5 pts",
  },
  "de-DE": {
    title: "Titel",
    description: "Beschreibung (Markdown)",
    why: "Warum es wichtig ist (Markdown)",
    success: "Erfolgsdefinition (Markdown)",
    status: "Status",
    priority: "Priorität",
    targetDate: "Zieldatum",
    submitDefault: "Ziel speichern",
    statusActive: "Aktiv",
    statusAchieved: "Erreicht",
    priorityCritical: "Kritisch",
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
    progressExplainer:
      "Der Fortschritt kommt von berücksichtigten Endaufgaben, gewichtet nach Aufwandspunkten.",
    progressSummaryTwo: "5 von 8 Aufwandspunkten über 2 Endaufgaben abgeschlossen.",
    progressSummaryOne: "5 von 8 Aufwandspunkten über 1 Endaufgabe abgeschlossen.",
    progressClose: "Fortschrittsdetails schließen",
    detailsFor: "Fortschrittsdetails für Synthetic goal",
    progressLoading: "Fortschrittsdetails werden geladen…",
    done: "erledigt",
    pointsValue: "5 Pkt",
  },
  "fr-FR": {
    title: "Titre",
    description: "Description (Markdown)",
    why: "Pourquoi c'est important (Markdown)",
    success: "Définition du succès (Markdown)",
    status: "Statut",
    priority: "Priorité",
    targetDate: "Date prévue",
    submitDefault: "Enregistrer l'objectif",
    statusActive: "Actif",
    statusAchieved: "Atteint",
    priorityCritical: "Critique",
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
    progressExplainer:
      "La progression provient des tâches feuilles éligibles, pondérées par points d'effort.",
    progressSummaryTwo: "5 points d'effort sur 8 réalisés à travers 2 tâches feuilles.",
    progressSummaryOne: "5 points d'effort sur 8 réalisés à travers 1 tâche feuille.",
    progressClose: "Fermer les détails de progression",
    detailsFor: "Détails de progression pour Synthetic goal",
    progressLoading: "Chargement des détails de progression…",
    done: "faite",
    pointsValue: "5 pts",
  },
};

const details = (count: 1 | 2) => ({
  percentage: 62.5,
  label: "active",
  completed_points: 5,
  total_points: 8,
  contributors: Array.from({ length: count }, (_unused, index) => ({
    task_id: `leaf-${index}`,
    title: `Synthetic leaf ${index}`,
    status: "completed" as const,
    effort_points: 5,
    completed: true,
  })),
});

describe.each([...SUPPORTED_LOCALES])("GoalForm in %s", (locale) => {
  it("renders translated labels, enum options, and the default submit action", async () => {
    await i18n.changeLanguage(locale);
    const e = COPY[locale];

    render(<GoalForm editableManagement onSubmit={vi.fn()} />);

    expect(screen.getByLabelText(e.title)).toBeTruthy();
    expect(screen.getByLabelText(e.description)).toBeTruthy();
    expect(screen.getByLabelText(e.why)).toBeTruthy();
    expect(screen.getByLabelText(e.success)).toBeTruthy();
    expect(screen.getByLabelText(e.status)).toBeTruthy();
    expect(screen.getByLabelText(e.priority)).toBeTruthy();
    expect(screen.getByLabelText(e.targetDate)).toBeTruthy();
    expect(screen.getByRole("button", { name: e.submitDefault })).toBeTruthy();

    // Enum options render the localized domain labels, not raw stored values.
    expect(screen.getByRole("option", { name: e.statusActive })).toBeTruthy();
    expect(screen.getByRole("option", { name: e.statusAchieved })).toBeTruthy();
    expect(screen.getByRole("option", { name: e.priorityCritical })).toBeTruthy();
  });

  it("blocks submission when the title is blank (validation stays locale-neutral)", async () => {
    await i18n.changeLanguage(locale);
    const e = COPY[locale];
    const onSubmit = vi.fn();

    render(<GoalForm editableManagement onSubmit={onSubmit} />);
    fireEvent.click(screen.getByRole("button", { name: e.submitDefault }));

    expect(onSubmit).not.toHaveBeenCalled();
  });

  it("pairs each Markdown field with a localized named preview region", async () => {
    await i18n.changeLanguage(locale);
    const e = COPY[locale];

    render(<GoalForm onSubmit={vi.fn()} />);

    // The shared MarkdownField chrome (visible pane label, region name from
    // the {{label}} interpolation, empty placeholder) follows the catalog.
    expect(screen.getAllByText(e.previewLabel).length).toBe(3);
    expect(screen.getAllByText(e.previewEmpty).length).toBe(3);
    for (const label of [e.description, e.why, e.success]) {
      expect(
        screen.getByRole("region", { name: e.previewRegion.replace("{{label}}", label) }),
      ).toBeTruthy();
    }
  });

  it("renders the localized Markdown editor chrome in all three field toolbars", async () => {
    await i18n.changeLanguage(locale);
    const e = COPY[locale];

    const { container } = render(<GoalForm onSubmit={vi.fn()} />);

    // GoalForm renders three MarkdownFields, so every opt-in editor-chrome
    // string (toolbar name, format buttons, heading menu, help tooltip)
    // appears once per toolbar from the active locale's catalog.
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

  it("renders the localized progress details surface with a pluralized summary", async () => {
    await i18n.changeLanguage(locale);
    const e = COPY[locale];

    const { rerender } = render(
      <MemoryRouter>
        <ProgressDetailsModal open title="Synthetic goal" details={null} onClose={vi.fn()} />
      </MemoryRouter>,
    );
    expect(screen.getByText(e.progressLoading)).toBeTruthy();

    rerender(
      <MemoryRouter>
        <ProgressDetailsModal open title="Synthetic goal" details={details(2)} onClose={vi.fn()} />
      </MemoryRouter>,
    );
    expect(screen.getByRole("dialog", { name: e.detailsFor })).toBeTruthy();
    expect(screen.getByText(e.progressExplainer)).toBeTruthy();
    expect(screen.getByText(e.progressSummaryTwo)).toBeTruthy();
    expect(screen.getByRole("button", { name: e.progressClose })).toBeTruthy();
    // Contributor badges and the points unit also render from the active locale.
    expect(screen.getAllByText(e.done).length).toBe(2);
    expect(screen.getAllByText(e.pointsValue).length).toBe(2);

    rerender(
      <MemoryRouter>
        <ProgressDetailsModal open title="Synthetic goal" details={details(1)} onClose={vi.fn()} />
      </MemoryRouter>,
    );
    expect(screen.getByText(e.progressSummaryOne)).toBeTruthy();
  });
});

describe("GoalForm runtime locale switching", () => {
  it("re-renders the form labels in the new language and keeps authored Markdown verbatim", async () => {
    const onSubmit = vi.fn();
    const raw = "# raw direction\n*stays as typed*";
    render(<GoalForm onSubmit={onSubmit} />);

    fireEvent.change(screen.getByLabelText("Title"), { target: { value: "Make it real" } });
    fireEvent.change(screen.getByLabelText("Description (Markdown)"), { target: { value: raw } });

    await i18n.changeLanguage("fr-FR");
    await waitFor(() => expect(screen.getByLabelText("Titre")).toBeTruthy());
    expect(screen.queryByLabelText("Title")).toBeNull();

    // The shared Markdown preview chrome follows the language switch while the
    // authored source below stays byte-exact.
    expect(screen.getByRole("region", { name: "Aperçu de Description (Markdown)" })).toBeTruthy();
    expect(screen.queryByRole("region", { name: "Description (Markdown) preview" })).toBeNull();

    // The localized editor toolbar chrome follows the same runtime switch.
    expect(screen.getAllByRole("button", { name: "Gras" })).toHaveLength(3);
    expect(screen.getAllByRole("combobox", { name: "Insérer un titre" })).toHaveLength(3);
    expect(screen.queryByRole("button", { name: "Bold" })).toBeNull();
    expect(screen.queryByText("Heading…")).toBeNull();

    // Localization changed the chrome only: both user-authored values survive
    // the language switch untouched and submit byte-exact.
    const titleInput = screen.getByLabelText("Titre") as HTMLInputElement;
    const description = screen.getByLabelText("Description (Markdown)") as HTMLTextAreaElement;
    expect(titleInput.value).toBe("Make it real");
    expect(description.value).toBe(raw);

    fireEvent.click(screen.getByRole("button", { name: "Enregistrer l'objectif" }));
    await waitFor(() => expect(onSubmit).toHaveBeenCalledTimes(1));
    expect(onSubmit.mock.calls[0][0]).toEqual(
      expect.objectContaining({
        title: "Make it real",
        description_markdown: raw,
      }),
    );
  });

  it("localizes the progress details loading state", async () => {
    render(
      <MemoryRouter>
        <ProgressDetailsModal open title="Synthetic goal" details={null} onClose={vi.fn()} />
      </MemoryRouter>,
    );
    expect(screen.getByText("Loading progress details…")).toBeTruthy();

    await i18n.changeLanguage("de-DE");
    await waitFor(() =>
      expect(screen.getByText("Fortschrittsdetails werden geladen…")).toBeTruthy(),
    );
    expect(screen.queryByText("Loading progress details…")).toBeNull();
  });
});

afterEach(async () => {
  await i18n.changeLanguage("en-US");
});
