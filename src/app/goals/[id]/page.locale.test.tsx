import i18n from "i18next";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { MemoryRouter, Route, Routes } from "react-router";
import GoalDetailPage from "@/app/goals/[id]/page";
import { SUPPORTED_LOCALES, type AppLocale } from "@/lib/i18n/locale";

// 8.6.9 Goal detail locale coverage: action buttons, dialog titles and
// descriptions, section headings, and empty states come from the active
// locale. The goal title (authored) and the Markdown preview content are
// rendered verbatim in every locale. The modal's search/close/actions render
// from the shared catalog. Synthetic fixtures only.

vi.mock("@/lib/api/goalApi", () => ({
  getGoal: vi.fn(),
}));

vi.mock("@/lib/api/projectApi", () => ({
  listProjects: vi.fn(),
  listProjectsForGoal: vi.fn(),
  moveProject: vi.fn(),
}));

vi.mock("@/lib/api/taskApi", () => ({
  listTasks: vi.fn(),
  listTasksForGoal: vi.fn(),
  attachTask: vi.fn(),
}));

vi.mock("@/lib/api/progressApi", () => ({
  getGoalProgress: vi.fn(),
}));

vi.mock("@/lib/api/deadlineApi", () => ({
  getGoalDeadlineStatus: vi.fn(),
}));

import { getGoal } from "@/lib/api/goalApi";
import { listProjects, listProjectsForGoal, moveProject } from "@/lib/api/projectApi";
import { listTasks, listTasksForGoal, attachTask } from "@/lib/api/taskApi";
import { getGoalProgress } from "@/lib/api/progressApi";
import { getGoalDeadlineStatus } from "@/lib/api/deadlineApi";

const goal = {
  id: "g1",
  workspace_id: "w1",
  year_theme_id: null,
  title: "Learn testing",
  description_markdown: "# raw authored heading\nverbatim *source*",
  why_it_matters_markdown: null,
  success_definition_markdown: null,
  status: "active",
  priority: "high",
  target_date: "2026-12-01",
  created_at: "2026-09-08T10:00:00Z",
  updated_at: "2026-09-08T10:00:00Z",
  archived_at: null,
};

interface DetailCopy {
  description: string;
  linkedProjects: string;
  linkedProjectsEmpty: string;
  directTasks: string;
  directTasksEmpty: string;
  viewProgress: string;
  addProject: string;
  addExistingProject: string;
  addDirectTask: string;
  addExistingTask: string;
  edit: string;
  attachProjectDescription: string;
  searchProjects: string;
  searchProjectsPlaceholder: string;
  attachEmpty: string;
  close: string;
  add: string;
  projectPlanned: string;
  goalActive: string;
}

const COPY: Record<AppLocale, DetailCopy> = {
  "en-US": {
    description: "Description",
    linkedProjects: "Linked projects",
    linkedProjectsEmpty: "No projects linked to this goal yet.",
    directTasks: "Direct tasks",
    directTasksEmpty: "No direct tasks yet. Small goals can use tasks without a project.",
    viewProgress: "View progress details",
    addProject: "Add project",
    addExistingProject: "Add existing project",
    addDirectTask: "Add direct task",
    addExistingTask: "Add existing task",
    edit: "Edit",
    attachProjectDescription: "Link a project that already exists so it contributes to this goal.",
    searchProjects: "Search projects",
    searchProjectsPlaceholder: "Search projects…",
    attachEmpty: "No eligible projects to link.",
    close: "Close",
    add: "Add",
    projectPlanned: "Planned",
    goalActive: "Active",
  },
  "es-ES": {
    description: "Descripción",
    linkedProjects: "Proyectos vinculados",
    linkedProjectsEmpty: "Aún no hay proyectos vinculados a este objetivo.",
    directTasks: "Tareas directas",
    directTasksEmpty:
      "Aún no hay tareas directas. Los objetivos pequeños pueden usar tareas sin proyecto.",
    viewProgress: "Ver detalles del progreso",
    addProject: "Añadir proyecto",
    addExistingProject: "Añadir proyecto existente",
    addDirectTask: "Añadir tarea directa",
    addExistingTask: "Añadir tarea existente",
    edit: "Editar",
    attachProjectDescription:
      "Vincula un proyecto que ya existe para que contribuya a este objetivo.",
    searchProjects: "Buscar proyectos",
    searchProjectsPlaceholder: "Buscar proyectos…",
    attachEmpty: "No hay proyectos elegibles para vincular.",
    close: "Cerrar",
    add: "Añadir",
    projectPlanned: "Planificado",
    goalActive: "Activo",
  },
  "de-DE": {
    description: "Beschreibung",
    linkedProjects: "Verknüpfte Projekte",
    linkedProjectsEmpty: "Noch keine Projekte mit diesem Ziel verknüpft.",
    directTasks: "Direkte Aufgaben",
    directTasksEmpty:
      "Noch keine direkten Aufgaben. Kleine Ziele können Aufgaben ohne Projekt nutzen.",
    viewProgress: "Fortschrittsdetails anzeigen",
    addProject: "Projekt hinzufügen",
    addExistingProject: "Vorhandenes Projekt hinzufügen",
    addDirectTask: "Direkte Aufgabe hinzufügen",
    addExistingTask: "Vorhandene Aufgabe hinzufügen",
    edit: "Bearbeiten",
    attachProjectDescription:
      "Verknüpfen Sie ein vorhandenes Projekt, damit es zu diesem Ziel beiträgt.",
    searchProjects: "Projekte suchen",
    searchProjectsPlaceholder: "Projekte suchen…",
    attachEmpty: "Keine für die Verknüpfung geeigneten Projekte.",
    close: "Schließen",
    add: "Hinzufügen",
    projectPlanned: "Geplant",
    goalActive: "Aktiv",
  },
  "fr-FR": {
    description: "Description",
    linkedProjects: "Projets liés",
    linkedProjectsEmpty: "Aucun projet lié à cet objectif pour l'instant.",
    directTasks: "Tâches directes",
    directTasksEmpty:
      "Aucune tâche directe pour l'instant. Les petits objectifs peuvent utiliser des tâches sans projet.",
    viewProgress: "Voir les détails de progression",
    addProject: "Ajouter un projet",
    addExistingProject: "Ajouter un projet existant",
    addDirectTask: "Ajouter une tâche directe",
    addExistingTask: "Ajouter une tâche existante",
    edit: "Modifier",
    attachProjectDescription: "Liez un projet existant pour qu'il contribue à cet objectif.",
    searchProjects: "Rechercher des projets",
    searchProjectsPlaceholder: "Rechercher des projets…",
    attachEmpty: "Aucun projet éligible à lier.",
    close: "Fermer",
    add: "Ajouter",
    projectPlanned: "Planifié",
    goalActive: "Actif",
  },
};

beforeEach(() => {
  vi.mocked(getGoal).mockReset();
  vi.mocked(listProjects).mockReset();
  vi.mocked(listProjectsForGoal).mockReset();
  vi.mocked(moveProject).mockReset();
  vi.mocked(listTasks).mockReset();
  vi.mocked(listTasksForGoal).mockReset();
  vi.mocked(attachTask).mockReset();
  vi.mocked(getGoalProgress).mockReset();
  vi.mocked(getGoalDeadlineStatus).mockReset();

  vi.mocked(getGoal).mockResolvedValue(goal as never);
  vi.mocked(listProjectsForGoal).mockResolvedValue([] as never);
  vi.mocked(listTasksForGoal).mockResolvedValue([] as never);
  vi.mocked(getGoalProgress).mockResolvedValue({
    percentage: 50,
    label: "active",
    completed_points: 5,
    total_points: 10,
    contributors: [],
  } as never);
  vi.mocked(getGoalDeadlineStatus).mockResolvedValue({
    status: "none",
    at_risk_tasks: [],
    warning_window_days: 14,
  } as never);
});

afterEach(async () => {
  await i18n.changeLanguage("en-US");
});

function renderDetail() {
  return render(
    <MemoryRouter initialEntries={["/goals/g1"]}>
      <Routes>
        <Route path="/goals/:id" element={<GoalDetailPage />} />
      </Routes>
    </MemoryRouter>,
  );
}

describe.each([...SUPPORTED_LOCALES])("Goal detail in %s", (locale) => {
  it("renders localized headings, actions, and empty states with verbatim content", async () => {
    await i18n.changeLanguage(locale);
    const e = COPY[locale];

    renderDetail();

    // Authored goal title and Markdown stay verbatim.
    expect(await screen.findByRole("heading", { name: "Learn testing" })).toBeTruthy();
    expect(
      await screen.findByRole("heading", { level: 1, name: "raw authored heading" }),
    ).toBeTruthy();
    expect(screen.getByRole("heading", { name: e.description })).toBeTruthy();
    expect(screen.getByRole("heading", { name: e.linkedProjects })).toBeTruthy();
    expect(screen.getByRole("heading", { name: e.directTasks })).toBeTruthy();
    expect(screen.getByText(e.linkedProjectsEmpty)).toBeTruthy();
    expect(screen.getByText(e.directTasksEmpty)).toBeTruthy();
    expect(screen.getByText(e.goalActive)).toBeTruthy();

    // Action row.
    expect(screen.getByRole("button", { name: e.viewProgress })).toBeTruthy();
    expect(screen.getByRole("link", { name: e.addProject })).toBeTruthy();
    expect(screen.getByRole("button", { name: e.addExistingProject })).toBeTruthy();
    expect(screen.getByRole("link", { name: e.addDirectTask })).toBeTruthy();
    expect(screen.getByRole("button", { name: e.addExistingTask })).toBeTruthy();
    expect(screen.getByRole("link", { name: e.edit })).toBeTruthy();
  });

  it("opens the add-existing-project dialog with localized chrome", async () => {
    await i18n.changeLanguage(locale);
    const e = COPY[locale];
    vi.mocked(listProjects).mockResolvedValue([] as never);

    renderDetail();
    fireEvent.click(await screen.findByRole("button", { name: e.addExistingProject }));

    const dialog = await screen.findByRole("dialog", { name: e.addExistingProject });
    expect(dialog).toBeTruthy();
    expect(screen.getByText(e.attachProjectDescription)).toBeTruthy();
    expect(screen.getByRole("searchbox", { name: e.searchProjects })).toBeTruthy();
    expect(screen.getByPlaceholderText(e.searchProjectsPlaceholder)).toBeTruthy();
    expect(screen.getByText(e.attachEmpty)).toBeTruthy();
    expect(screen.getByRole("button", { name: e.close })).toBeTruthy();
  });
});

describe("Goal detail runtime locale switching", () => {
  it("re-renders the chrome and the open dialog in the new language without remounting", async () => {
    vi.mocked(listProjects).mockResolvedValue([
      {
        id: "p2",
        workspace_id: "w1",
        goal_id: null,
        title: "Candidate project",
        status: "planned",
        priority: "normal",
        target_date: null,
        created_at: "2026-09-08T10:00:00Z",
        updated_at: "2026-09-08T10:00:00Z",
        archived_at: null,
      },
    ] as never);

    renderDetail();
    fireEvent.click(await screen.findByRole("button", { name: "Add existing project" }));
    expect(await screen.findByRole("dialog", { name: "Add existing project" })).toBeTruthy();
    // Candidates arrive asynchronously; the Add button and localized status
    // badge render once the mocked workspace list has settled.
    expect(await screen.findByRole("button", { name: "Add" })).toBeTruthy();
    expect(await screen.findByText("Planned")).toBeTruthy();

    await i18n.changeLanguage("es-ES");
    await waitFor(() =>
      expect(screen.getByRole("dialog", { name: "Añadir proyecto existente" })).toBeTruthy(),
    );
    expect(
      await screen.findByText(
        "Vincula un proyecto que ya existe para que contribuya a este objetivo.",
      ),
    ).toBeTruthy();
    expect(screen.getByPlaceholderText("Buscar proyectos…")).toBeTruthy();
    expect(await screen.findByRole("button", { name: "Añadir" })).toBeTruthy();
    expect(await screen.findByText("Planificado")).toBeTruthy();
    // Candidate titles are user-authored: they stay verbatim after switching.
    expect(screen.getByText("Candidate project")).toBeTruthy();
  });
});
