import { describe, expect, it, vi, beforeEach } from "vitest";
import { render, screen } from "@testing-library/react";
import { MemoryRouter, Route, Routes } from "react-router";
import TodayPage from "@/app/today/page";
import { getOrCreateDailyPlan, listDayTasks } from "@/lib/api/dailyPlanApi";
import { listTasks } from "@/lib/api/taskApi";
import { listBlocksForDate } from "@/lib/api/blockApi";
import { listFixedCommitments } from "@/lib/api/fixedCommitmentApi";
import { listAtRiskTasks } from "@/lib/api/deadlineApi";
import { ensureDefaultWorkspace } from "@/lib/api/workspaceApi";
import {
  archiveTherapyCollection,
  archiveTherapyRecord,
  createTherapyCollection,
  createTherapyRecord,
  getTherapyRecord,
  listTherapyCollections,
  listTherapyRecords,
  updateTherapyCollection,
  updateTherapyRecord,
} from "@/lib/api/therapyApi";

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

// The privacy contract: the executive dashboard must never reach therapy
// commands, so every therapy API function is a spy asserted to stay quiet.
vi.mock("@/lib/api/therapyApi", () => ({
  createTherapyCollection: vi.fn(),
  listTherapyCollections: vi.fn(),
  updateTherapyCollection: vi.fn(),
  archiveTherapyCollection: vi.fn(),
  createTherapyRecord: vi.fn(),
  getTherapyRecord: vi.fn(),
  updateTherapyRecord: vi.fn(),
  listTherapyRecords: vi.fn(),
  archiveTherapyRecord: vi.fn(),
}));

const workspace = { id: "w1", name: "Default" };

const plan = {
  id: "plan-1",
  workspace_id: "w1",
  date: "2026-09-08",
  status: "planned",
  intention_markdown: null,
  capacity_note_markdown: null,
  review_markdown: null,
  planning_block_id: null,
  created_at: "2026-09-08T08:00:00Z",
  updated_at: "2026-09-08T08:00:00Z",
};

const planTask = {
  id: "dpt-1",
  daily_plan_id: "plan-1",
  task_id: "t1",
  order_index: 0,
  selection_reason: "manual",
  planned_status: "selected",
  carryover_from_daily_plan_id: null,
  decision_note_markdown: null,
  created_at: "2026-09-08T08:05:00Z",
  updated_at: "2026-09-08T08:05:00Z",
};

const task = {
  id: "t1",
  workspace_id: "w1",
  project_id: null,
  goal_id: null,
  parent_task_id: null,
  title: "Write the changelog",
  description_markdown: null,
  notes_markdown: null,
  blocking_note_markdown: null,
  status: "in_progress",
  priority: "high",
  effort_size: "medium",
  effort_points: 3,
  target_date: null,
  scheduled_date: "2026-09-08",
  completed_at: null,
  cancelled_at: null,
  closure_mode: null,
  closure_reason_markdown: null,
  created_at: "2026-09-08T10:00:00Z",
  updated_at: "2026-09-08T10:00:00Z",
  archived_at: null,
};

function renderPage() {
  return render(
    <MemoryRouter initialEntries={["/today"]}>
      <Routes>
        <Route path="/today" element={<TodayPage />} />
      </Routes>
    </MemoryRouter>,
  );
}

describe("TodayPage (executive dashboard)", () => {
  beforeEach(() => {
    vi.mocked(ensureDefaultWorkspace).mockReset();
    vi.mocked(getOrCreateDailyPlan).mockReset();
    vi.mocked(listDayTasks).mockReset();
    vi.mocked(listTasks).mockReset();
    vi.mocked(listBlocksForDate).mockReset();
    vi.mocked(listFixedCommitments).mockReset();
    vi.mocked(listAtRiskTasks).mockReset();

    vi.mocked(ensureDefaultWorkspace).mockResolvedValue(workspace as never);
    vi.mocked(getOrCreateDailyPlan).mockResolvedValue(plan as never);
    vi.mocked(listDayTasks).mockResolvedValue([planTask] as never);
    vi.mocked(listTasks).mockResolvedValue([task] as never);
    vi.mocked(listBlocksForDate).mockResolvedValue([]);
    vi.mocked(listFixedCommitments).mockResolvedValue([]);
    vi.mocked(listAtRiskTasks).mockResolvedValue([]);
  });

  it("shows execution data without calling or rendering therapy content", async () => {
    renderPage();

    expect(await screen.findByRole("heading", { name: "Today" })).toBeTruthy();
    expect(await screen.findByText("Write the changelog")).toBeTruthy();

    // The dashboard loads only execution/time data; therapy commands are never
    // invoked from this route.
    expect(ensureDefaultWorkspace).toHaveBeenCalledTimes(1);
    for (const therapyApiCall of [
      createTherapyCollection,
      listTherapyCollections,
      updateTherapyCollection,
      archiveTherapyCollection,
      createTherapyRecord,
      getTherapyRecord,
      updateTherapyRecord,
      listTherapyRecords,
      archiveTherapyRecord,
    ]) {
      expect(therapyApiCall).not.toHaveBeenCalled();
    }
    expect(screen.queryByText(/therapy/i)).toBeNull();
  });
});
