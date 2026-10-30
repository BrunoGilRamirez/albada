import i18n from "i18next";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { render, screen, waitFor } from "@testing-library/react";
import { MemoryRouter, Route, Routes } from "react-router";
import TasksPage from "@/app/tasks/page";
import { SUPPORTED_LOCALES, type AppLocale } from "@/lib/i18n/locale";

// 8.6.9 Tasks list locale coverage: the independent-tasks heading, the new
// action, the localized status/priority/effort badges, the shared deadline
// badge, and the Intl-formatted application-owned target date all follow the
// active locale; user-authored task titles stay verbatim. Synthetic fixtures
// only. Date expectations are computed with Intl in the test so they track
// the runtime ICU instead of being hardcoded.

vi.mock("@/lib/api/workspaceApi", () => ({
  ensureDefaultWorkspace: vi.fn(),
}));

vi.mock("@/lib/api/taskApi", () => ({
  listTasks: vi.fn(),
}));

vi.mock("@/lib/api/projectApi", () => ({
  listProjects: vi.fn(),
}));

vi.mock("@/lib/api/goalApi", () => ({
  listGoals: vi.fn(),
}));

vi.mock("@/lib/api/deadlineApi", () => ({
  listAtRiskTasks: vi.fn(),
}));

import { ensureDefaultWorkspace } from "@/lib/api/workspaceApi";
import { listTasks } from "@/lib/api/taskApi";
import { listProjects } from "@/lib/api/projectApi";
import { listGoals } from "@/lib/api/goalApi";
import { listAtRiskTasks } from "@/lib/api/deadlineApi";

const taskFixture = {
  id: "t1",
  workspace_id: "w1",
  project_id: null,
  goal_id: null,
  parent_task_id: null,
  title: "Water the ferns",
  description_markdown: null,
  notes_markdown: null,
  blocking_note_markdown: null,
  status: "planned",
  priority: "high",
  effort_size: "medium",
  effort_points: 3,
  target_date: "2026-10-20",
  scheduled_date: null,
  closure_mode: null,
  closure_reason_markdown: null,
  created_at: "2026-09-08T10:00:00Z",
  updated_at: "2026-09-08T10:00:00Z",
  archived_at: null,
};

const atRiskFixture = {
  task_id: "t1",
  title: "Water the ferns",
  status: "planned",
  deadline_at: "2026-10-20",
  deadline_status: "overdue",
};

interface TasksCopy {
  listTitle: string;
  newTask: string;
  statusPlanned: string;
  priorityHigh: string;
  effortMedium: string;
  pointsSuffix: string;
  overdue: string;
}

const COPY: Record<AppLocale, TasksCopy> = {
  "en-US": {
    listTitle: "Tasks",
    newTask: "New task",
    statusPlanned: "Planned",
    priorityHigh: "High",
    effortMedium: "Medium",
    pointsSuffix: "3p",
    overdue: "Overdue",
  },
  "es-ES": {
    listTitle: "Tareas",
    newTask: "Nueva tarea",
    statusPlanned: "Planificada",
    priorityHigh: "Alta",
    effortMedium: "Mediana",
    pointsSuffix: "3 p",
    overdue: "Vencida",
  },
  "de-DE": {
    listTitle: "Aufgaben",
    newTask: "Neue Aufgabe",
    statusPlanned: "Geplant",
    priorityHigh: "Hoch",
    effortMedium: "Mittel",
    pointsSuffix: "3 P",
    overdue: "Überfällig",
  },
  "fr-FR": {
    listTitle: "Tâches",
    newTask: "Nouvelle tâche",
    statusPlanned: "Planifiée",
    priorityHigh: "Haute",
    effortMedium: "Moyenne",
    pointsSuffix: "3 p",
    overdue: "En retard",
  },
};

function renderPage() {
  return render(
    <MemoryRouter initialEntries={["/tasks"]}>
      <Routes>
        <Route path="/tasks" element={<TasksPage />} />
      </Routes>
    </MemoryRouter>,
  );
}

beforeEach(() => {
  vi.mocked(ensureDefaultWorkspace).mockReset();
  vi.mocked(listTasks).mockReset();
  vi.mocked(listProjects).mockReset();
  vi.mocked(listGoals).mockReset();
  vi.mocked(listAtRiskTasks).mockReset();
  vi.mocked(ensureDefaultWorkspace).mockResolvedValue({ id: "w1", name: "Default" } as never);
  vi.mocked(listProjects).mockResolvedValue([]);
  vi.mocked(listGoals).mockResolvedValue([]);
  vi.mocked(listAtRiskTasks).mockResolvedValue([]);
});

afterEach(async () => {
  await i18n.changeLanguage("en-US");
});

describe.each([...SUPPORTED_LOCALES])("Tasks list in %s", (locale) => {
  it("renders localized badges and Intl-formatted dates with verbatim titles", async () => {
    await i18n.changeLanguage(locale);
    const e = COPY[locale];
    vi.mocked(listTasks).mockResolvedValue([taskFixture] as never);
    vi.mocked(listAtRiskTasks).mockResolvedValue([atRiskFixture] as never);

    renderPage();

    expect(await screen.findByRole("heading", { name: e.listTitle })).toBeTruthy();
    expect(screen.getByRole("link", { name: e.newTask })).toBeTruthy();

    // Authored title stays verbatim; every application-owned badge localizes.
    expect(await screen.findByText("Water the ferns")).toBeTruthy();
    expect(screen.getByText(e.statusPlanned)).toBeTruthy();
    expect(screen.getByText(e.priorityHigh)).toBeTruthy();
    expect(screen.getByText(e.effortMedium)).toBeTruthy();
    expect(screen.getByText(e.pointsSuffix)).toBeTruthy();
    // The at-risk deadline map lands asynchronously after the list renders.
    expect((await screen.findAllByText(e.overdue)).length).toBeGreaterThan(0);

    // The row's application-owned target date renders through the shared Intl
    // formatter (dateStyle medium) instead of the raw stored value.
    const expectedDate = new Date("2026-10-20T12:00:00").toLocaleDateString(locale, {
      dateStyle: "medium",
    });
    expect(await screen.findByText(expectedDate)).toBeTruthy();
  });

  it("shows the localized empty state when no standalone tasks exist", async () => {
    await i18n.changeLanguage(locale);
    vi.mocked(listTasks).mockResolvedValue([]);

    renderPage();

    const e = i18n.t("tasks.empty");
    expect(await screen.findByText(e)).toBeTruthy();
  });
});

describe("Tasks list runtime locale switching", () => {
  it("re-renders the list chrome, badges, and dates on language change without remounting", async () => {
    vi.mocked(listTasks).mockResolvedValue([taskFixture] as never);
    vi.mocked(listAtRiskTasks).mockResolvedValue([atRiskFixture] as never);
    renderPage();

    expect(await screen.findByRole("heading", { name: "Tasks" })).toBeTruthy();
    expect(await screen.findByText("Planned")).toBeTruthy();

    await i18n.changeLanguage("de-DE");
    await waitFor(() => expect(screen.getByRole("heading", { name: "Aufgaben" })).toBeTruthy());
    expect(screen.getByText("Geplant")).toBeTruthy();
    expect(screen.getAllByText("Überfällig").length).toBeGreaterThan(0);
    const expectedDate = new Date("2026-10-20T12:00:00").toLocaleDateString("de-DE", {
      dateStyle: "medium",
    });
    expect(screen.getByText(expectedDate)).toBeTruthy();
    expect(screen.queryByText("Planned")).toBeNull();
  });
});
