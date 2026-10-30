import i18n from "i18next";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { render, screen, waitFor } from "@testing-library/react";
import { MemoryRouter, Route, Routes } from "react-router";
import ProjectsPage from "@/app/projects/page";
import { SUPPORTED_LOCALES, type AppLocale } from "@/lib/i18n/locale";

// 8.6.9 Projects list locale coverage: heading, new-project action, subtitle,
// empty state, and row status/progress/deadline badges render from the active
// locale; the user-authored project title stays verbatim. Synthetic fixtures
// only.

vi.mock("@/lib/api/workspaceApi", () => ({
  ensureDefaultWorkspace: vi.fn(),
}));

vi.mock("@/lib/api/projectApi", () => ({
  listProjects: vi.fn(),
}));

vi.mock("@/lib/api/progressApi", () => ({
  getProjectProgress: vi.fn(),
}));

vi.mock("@/lib/api/deadlineApi", () => ({
  getProjectDeadlineStatus: vi.fn(),
}));

import { ensureDefaultWorkspace } from "@/lib/api/workspaceApi";
import { listProjects } from "@/lib/api/projectApi";
import { getProjectProgress } from "@/lib/api/progressApi";
import { getProjectDeadlineStatus } from "@/lib/api/deadlineApi";

const projectFixture = {
  id: "p1",
  workspace_id: "w1",
  goal_id: null,
  title: "Build the garden shed",
  description_markdown: null,
  why_it_matters_markdown: null,
  definition_of_done_markdown: null,
  status: "planned",
  priority: "normal",
  target_date: "2026-11-15",
  created_at: "2026-09-08T10:00:00Z",
  updated_at: "2026-09-08T10:00:00Z",
  archived_at: null,
};

interface ProjectsCopy {
  title: string;
  newProject: string;
  subtitle: string;
  empty: string;
  statusPlanned: string;
  progressNotStarted: string;
  deadlineApproaching: string;
}

const COPY: Record<AppLocale, ProjectsCopy> = {
  "en-US": {
    title: "Projects",
    newProject: "New project",
    subtitle:
      "Bounded results you can divide into tasks. Progress comes from completed leaf tasks weighted by effort points.",
    empty: "No projects yet. Create one to turn a goal into actionable work.",
    statusPlanned: "Planned",
    progressNotStarted: "Not started",
    deadlineApproaching: "Approaching",
  },
  "es-ES": {
    title: "Proyectos",
    newProject: "Nuevo proyecto",
    subtitle:
      "Resultados acotados que puedes dividir en tareas. El progreso proviene de las tareas hoja completadas, ponderadas por puntos de esfuerzo.",
    empty: "Aún no hay proyectos. Crea uno para convertir un objetivo en trabajo accionable.",
    statusPlanned: "Planificado",
    progressNotStarted: "Sin comenzar",
    deadlineApproaching: "Próxima",
  },
  "de-DE": {
    title: "Projekte",
    newProject: "Neues Projekt",
    subtitle:
      "Abgegrenzte Ergebnisse, die Sie in Aufgaben unterteilen können. Der Fortschritt kommt von abgeschlossenen Endaufgaben, gewichtet nach Aufwandspunkten.",
    empty:
      "Noch keine Projekte. Erstellen Sie eines, um ein Ziel in umsetzbare Arbeit zu verwandeln.",
    statusPlanned: "Geplant",
    progressNotStarted: "Nicht begonnen",
    deadlineApproaching: "Bald fällig",
  },
  "fr-FR": {
    title: "Projets",
    newProject: "Nouveau projet",
    subtitle:
      "Des résultats délimités que vous pouvez diviser en tâches. La progression provient des tâches feuilles terminées, pondérées par points d'effort.",
    empty:
      "Aucun projet pour l'instant. Créez-en un pour transformer un objectif en travail réalisable.",
    statusPlanned: "Planifié",
    progressNotStarted: "Non démarré",
    deadlineApproaching: "Proche",
  },
};

function renderPage() {
  return render(
    <MemoryRouter initialEntries={["/projects"]}>
      <Routes>
        <Route path="/projects" element={<ProjectsPage />} />
      </Routes>
    </MemoryRouter>,
  );
}

beforeEach(() => {
  vi.mocked(ensureDefaultWorkspace).mockReset();
  vi.mocked(listProjects).mockReset();
  vi.mocked(getProjectProgress).mockReset();
  vi.mocked(getProjectDeadlineStatus).mockReset();
  vi.mocked(ensureDefaultWorkspace).mockResolvedValue({ id: "w1", name: "Default" } as never);
});

afterEach(async () => {
  await i18n.changeLanguage("en-US");
});

describe.each([...SUPPORTED_LOCALES])("Projects list in %s", (locale) => {
  it("renders the heading, action, subtitle, and empty state", async () => {
    await i18n.changeLanguage(locale);
    const e = COPY[locale];
    vi.mocked(listProjects).mockResolvedValue([]);

    renderPage();

    expect(await screen.findByRole("heading", { name: e.title })).toBeTruthy();
    expect(screen.getByRole("link", { name: e.newProject })).toBeTruthy();
    expect(screen.getByText(e.subtitle)).toBeTruthy();
    expect(screen.getByText(e.empty)).toBeTruthy();
  });

  it("renders localized row badges while keeping the authored title verbatim", async () => {
    await i18n.changeLanguage(locale);
    const e = COPY[locale];
    vi.mocked(listProjects).mockResolvedValue([projectFixture] as never);
    vi.mocked(getProjectProgress).mockResolvedValue({
      percentage: null,
      label: "not_started",
      completed_points: 0,
      total_points: 0,
      contributors: [],
    } as never);
    vi.mocked(getProjectDeadlineStatus).mockResolvedValue({
      status: "approaching",
      at_risk_tasks: [],
      warning_window_days: 7,
    } as never);

    renderPage();

    expect(await screen.findByText("Build the garden shed")).toBeTruthy();
    expect(await screen.findByText(e.statusPlanned)).toBeTruthy();
    // Progress/deadline state lands asynchronously after the list renders.
    expect(await screen.findByText(e.progressNotStarted)).toBeTruthy();
    expect(await screen.findByText(e.deadlineApproaching)).toBeTruthy();
  });
});

describe("Projects list runtime locale switching", () => {
  it("re-renders the page chrome in the new language without remounting", async () => {
    vi.mocked(listProjects).mockResolvedValue([]);
    renderPage();

    expect(await screen.findByRole("heading", { name: "Projects" })).toBeTruthy();

    await i18n.changeLanguage("es-ES");
    await waitFor(() => expect(screen.getByRole("heading", { name: "Proyectos" })).toBeTruthy());
    expect(screen.getByRole("link", { name: "Nuevo proyecto" })).toBeTruthy();
    expect(screen.queryByRole("heading", { name: "Projects" })).toBeNull();
  });
});
