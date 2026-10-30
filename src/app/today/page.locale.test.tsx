import i18n from "i18next";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { render, screen } from "@testing-library/react";
import { MemoryRouter, Route, Routes } from "react-router";
import TodayPage from "@/app/today/page";
import { SUPPORTED_LOCALES, type AppLocale } from "@/lib/i18n/locale";
import { getOrCreateDailyPlan, listDayTasks } from "@/lib/api/dailyPlanApi";
import { listTasks } from "@/lib/api/taskApi";
import { listBlocksForDate } from "@/lib/api/blockApi";
import { listFixedCommitments } from "@/lib/api/fixedCommitmentApi";
import { listAtRiskTasks } from "@/lib/api/deadlineApi";
import { ensureDefaultWorkspace } from "@/lib/api/workspaceApi";

// 8.6.8 Today locale coverage: every application-owned label on the dashboard
// and its panels renders from the active locale, application-owned dates go
// through the shared Intl formatter, and user-authored titles stay verbatim.
// Date/number expectations are computed with Intl in the test so they track
// the runtime ICU instead of being hardcoded.

vi.mock("@/lib/api/workspaceApi", () => ({
  ensureDefaultWorkspace: vi.fn(),
}));

vi.mock("@/lib/api/dailyPlanApi", () => ({
  getOrCreateDailyPlan: vi.fn(),
  listDayTasks: vi.fn(),
  updateDailyPlanStatus: vi.fn(),
  deselectTaskForDay: vi.fn(),
  replanTask: vi.fn(),
  reorderDayTasks: vi.fn(),
  getCapacitySuggestion: vi.fn(),
  selectTaskForDay: vi.fn(),
  updateDailyPlanIntention: vi.fn(),
  setDailyPlanPlanningBlock: vi.fn(),
}));

vi.mock("@/lib/api/taskApi", () => ({
  listTasks: vi.fn(),
  completeTask: vi.fn(),
  transitionTaskStatus: vi.fn(),
  whyIsBlocked: vi.fn(),
}));

vi.mock("@/lib/api/blockApi", () => ({
  listBlocksForDate: vi.fn(),
  createBlock: vi.fn(),
  updateBlock: vi.fn(),
  createBlockRecurrence: vi.fn(),
  updateBlockRecurrence: vi.fn(),
  getBlockRecurrence: vi.fn(),
  startBlock: vi.fn(),
  completeBlock: vi.fn(),
  skipBlock: vi.fn(),
  cancelBlock: vi.fn(),
  deactivateBlock: vi.fn(),
  deleteBlock: vi.fn(),
  reactivateBlock: vi.fn(),
  deactivateBlockRecurrence: vi.fn(),
  reactivateBlockRecurrence: vi.fn(),
  deleteBlockRecurrence: vi.fn(),
  listInactiveBlocks: vi.fn(),
  listInactiveBlockRecurrences: vi.fn(),
  listUpcomingBlocks: vi.fn(),
}));

vi.mock("@/lib/api/sessionApi", () => ({
  createSession: vi.fn(),
  startSession: vi.fn(),
  pauseSession: vi.fn(),
  finishSession: vi.fn(),
  skipSession: vi.fn(),
  listSessionsForTask: vi.fn(),
}));

vi.mock("@/lib/api/fixedCommitmentApi", () => ({
  listFixedCommitments: vi.fn(),
}));

vi.mock("@/lib/api/deadlineApi", () => ({
  listAtRiskTasks: vi.fn(),
}));

const FIX_DATE = "2026-09-08";

const plan = {
  id: "plan-1",
  workspace_id: "w1",
  date: FIX_DATE,
  status: "in_review",
};

const planTask = {
  id: "dpt-1",
  daily_plan_id: "plan-1",
  task_id: "t1",
  order_index: 0,
  selection_reason: "manual",
  planned_status: "selected",
};

const task = {
  id: "t1",
  workspace_id: "w1",
  title: "Write the changelog",
  status: "in_progress",
  priority: "high",
  target_date: "2026-09-10",
};

const block = {
  id: "b1",
  workspace_id: "w1",
  daily_plan_id: "plan-1",
  date: FIX_DATE,
  block_type: "focus",
  status: "planned",
  label: null,
  planned_start_at: null,
  planned_end_at: null,
  time_window_label: "morning",
  flexibility: "flexible",
  note_markdown: null,
  // The lifecycle state and the optional task reference are part of the
  // block contract since 8.7.8.
  lifecycle_state: "active",
  task_id: null,
};

const commitment = {
  id: "c1",
  workspace_id: "w1",
  title: "Team standup",
  commitment_type: "work",
  start_at: "09:00",
  end_at: "10:00",
};

const atRisk = {
  task_id: "t2",
  title: "Send the invoice",
  status: "planned",
  deadline_at: "2026-09-07T17:30:00Z",
  deadline_status: "overdue",
};

interface TodayCopy {
  title: string;
  notToday: string;
  planningRitual: string;
  planStatus: string;
  planStatusInReview: string;
  selectedTasks: string;
  blocks: string;
  deadlineWarnings: string;
  fixedCommitments: string;
  taskStatus: string;
  deadlinePrefix: string;
  blockFocus: string;
  blockMorning: string;
  startButton: string;
  workCommitment: string;
  overdueBadge: string;
}

const TODAY_COPY: Record<AppLocale, TodayCopy> = {
  "en-US": {
    title: "Today",
    notToday: "(not today)",
    planningRitual: "Planning ritual",
    planStatus: "Plan status",
    planStatusInReview: "In review",
    selectedTasks: "Selected tasks",
    blocks: "Blocks",
    deadlineWarnings: "Deadline warnings",
    fixedCommitments: "Fixed commitments",
    taskStatus: "In progress",
    deadlinePrefix: "deadline",
    blockFocus: "Focus",
    blockMorning: "Morning",
    startButton: "Start",
    workCommitment: "Work",
    overdueBadge: "Overdue",
  },
  "es-ES": {
    title: "Hoy",
    notToday: "(no es hoy)",
    planningRitual: "Ritual de planificación",
    planStatus: "Estado del plan",
    planStatusInReview: "En revisión",
    selectedTasks: "Tareas seleccionadas",
    blocks: "Bloques",
    deadlineWarnings: "Avisos de vencimiento",
    fixedCommitments: "Compromisos fijos",
    taskStatus: "En curso",
    deadlinePrefix: "fecha límite",
    blockFocus: "Enfoque",
    blockMorning: "Mañana",
    startButton: "Iniciar",
    workCommitment: "Trabajo",
    overdueBadge: "Vencida",
  },
  "de-DE": {
    title: "Heute",
    notToday: "(nicht heute)",
    planningRitual: "Planungsritual",
    planStatus: "Planungsstatus",
    planStatusInReview: "In Überprüfung",
    selectedTasks: "Ausgewählte Aufgaben",
    blocks: "Blöcke",
    deadlineWarnings: "Fristwarnungen",
    fixedCommitments: "Feste Termine",
    taskStatus: "In Bearbeitung",
    deadlinePrefix: "Frist",
    blockFocus: "Fokus",
    blockMorning: "Vormittag",
    startButton: "Starten",
    workCommitment: "Arbeit",
    overdueBadge: "Überfällig",
  },
  "fr-FR": {
    title: "Aujourd'hui",
    notToday: "(pas aujourd'hui)",
    planningRitual: "Rituel de planification",
    planStatus: "Statut du plan",
    planStatusInReview: "En revue",
    selectedTasks: "Tâches sélectionnées",
    blocks: "Blocs",
    deadlineWarnings: "Alertes d'échéance",
    fixedCommitments: "Engagements fixes",
    taskStatus: "En cours",
    deadlinePrefix: "échéance",
    blockFocus: "Focus",
    blockMorning: "Matin",
    startButton: "Démarrer",
    workCommitment: "Travail",
    overdueBadge: "En retard",
  },
};

function renderPage() {
  return render(
    <MemoryRouter initialEntries={[`/today?date=${FIX_DATE}`]}>
      <Routes>
        <Route path="/today" element={<TodayPage />} />
      </Routes>
    </MemoryRouter>,
  );
}

beforeEach(() => {
  vi.mocked(ensureDefaultWorkspace).mockReset();
  vi.mocked(getOrCreateDailyPlan).mockReset();
  vi.mocked(listDayTasks).mockReset();
  vi.mocked(listTasks).mockReset();
  vi.mocked(listBlocksForDate).mockReset();
  vi.mocked(listFixedCommitments).mockReset();
  vi.mocked(listAtRiskTasks).mockReset();

  vi.mocked(ensureDefaultWorkspace).mockResolvedValue({ id: "w1", name: "Default" } as never);
  vi.mocked(getOrCreateDailyPlan).mockResolvedValue(plan as never);
  vi.mocked(listDayTasks).mockResolvedValue([planTask] as never);
  vi.mocked(listTasks).mockResolvedValue([task] as never);
  vi.mocked(listBlocksForDate).mockResolvedValue([block] as never);
  vi.mocked(listFixedCommitments).mockResolvedValue([commitment] as never);
  vi.mocked(listAtRiskTasks).mockResolvedValue([atRisk] as never);
});

afterEach(async () => {
  await i18n.changeLanguage("en-US");
});

describe.each([...SUPPORTED_LOCALES])("Today page in %s", (locale) => {
  it("renders all panel labels and Intl-formatted application dates", async () => {
    await i18n.changeLanguage(locale);
    const e = TODAY_COPY[locale];

    renderPage();

    expect(await screen.findByRole("heading", { name: e.title })).toBeTruthy();

    // Application-owned header date through the shared Intl pattern (static,
    // derived from the date query parameter).
    const headerDate = new Date(`${FIX_DATE}T12:00:00`).toLocaleDateString(locale, {
      weekday: "long",
      year: "numeric",
      month: "long",
      day: "numeric",
    });
    const headerText = `${headerDate} ${e.notToday}`;
    const headerLine = screen.getByText((content) => content === headerText);
    expect(headerLine).toBeTruthy();

    expect(screen.getByRole("button", { name: e.planningRitual })).toBeTruthy();

    // Section headings from the Today catalog (rendered eagerly).
    expect(screen.getByRole("heading", { name: e.selectedTasks })).toBeTruthy();
    expect(screen.getByRole("heading", { name: e.blocks })).toBeTruthy();
    expect(screen.getByRole("heading", { name: e.deadlineWarnings })).toBeTruthy();
    expect(screen.getByRole("heading", { name: e.fixedCommitments })).toBeTruthy();

    // Everything below appears once the async load has committed.
    expect(await screen.findByLabelText(e.planStatus)).toBeTruthy();
    expect(await screen.findByRole("option", { name: e.planStatusInReview })).toBeTruthy();

    // Task row: translated status + translated deadline prefix + Intl medium
    // date; the user-authored task title stays verbatim.
    const deadlineDate = new Date("2026-09-10T12:00:00").toLocaleDateString(locale, {
      dateStyle: "medium",
    });
    const taskLine = `${e.taskStatus} · ${e.deadlinePrefix} ${deadlineDate}`;
    expect(await screen.findByText(taskLine)).toBeTruthy();
    expect(await screen.findByText("Write the changelog")).toBeTruthy();

    // Block card labels come from the Today catalog; the localized Start
    // action comes from the shared action catalog.
    expect((await screen.findAllByText(e.blockFocus)).length).toBeGreaterThanOrEqual(2);
    expect(await screen.findByText(e.blockMorning)).toBeTruthy();
    expect(await screen.findByRole("button", { name: e.startButton })).toBeTruthy();

    // Commitment badge is the shared label; the user-entered title stays.
    expect(await screen.findByText(e.workCommitment)).toBeTruthy();
    expect(await screen.findByText("Team standup")).toBeTruthy();

    // Deadline warning: translated badge + Intl date-time for deadline_at.
    const deadlineAt = new Date("2026-09-07T17:30:00Z").toLocaleString(locale);
    expect(await screen.findByText(e.overdueBadge)).toBeTruthy();
    expect(await screen.findByText(deadlineAt)).toBeTruthy();
  });
});

describe("Today page runtime locale switching", () => {
  it("re-renders the dashboard in the new language without remounting", async () => {
    renderPage();
    expect(await screen.findByRole("heading", { name: "Today" })).toBeTruthy();

    await i18n.changeLanguage("de-DE");
    const ritualButton = await screen.findByRole("button", { name: "Planungsritual" });
    expect(ritualButton).toBeTruthy();
    expect(screen.getByRole("heading", { name: "Heute" })).toBeTruthy();
    // The header date reformats with the German Intl pattern.
    const headerDate = new Date(`${FIX_DATE}T12:00:00`).toLocaleDateString("de-DE", {
      weekday: "long",
      year: "numeric",
      month: "long",
      day: "numeric",
    });
    const headerText = `${headerDate} (nicht heute)`;
    const headerLine = screen.getByText((content) => content === headerText);
    expect(headerLine).toBeTruthy();
  });
});
