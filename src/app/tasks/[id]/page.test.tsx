import { describe, expect, it, vi, beforeEach } from "vitest";
import { fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { MemoryRouter, Route, Routes } from "react-router";
import TaskDetailPage from "@/app/tasks/[id]/page";
import { completeTask, getTaskWithTree, listAllPrerequisites, listTasks } from "@/lib/api/taskApi";
import { getTaskDeadlineStatus } from "@/lib/api/deadlineApi";

vi.mock("@/lib/api/taskApi", () => ({
  getTaskWithTree: vi.fn(),
  listTasks: vi.fn(),
  completeTask: vi.fn(),
  transitionTaskStatus: vi.fn(),
  archiveTask: vi.fn(),
  restoreTask: vi.fn(),
  deleteTask: vi.fn(),
  addDependency: vi.fn(),
  removeDependency: vi.fn(),
  listAllPrerequisites: vi.fn(),
  whyIsBlocked: vi.fn(),
}));

vi.mock("@/lib/api/deadlineApi", () => ({
  getTaskDeadlineStatus: vi.fn(),
}));

vi.mock("@/lib/api/progressApi", () => ({
  getTaskProgress: vi.fn(),
}));

vi.mock("@/lib/api/goalApi", () => ({
  getGoal: vi.fn(),
}));

vi.mock("@/lib/api/projectApi", () => ({
  getProject: vi.fn(),
}));

const baseTask = {
  id: "t1",
  workspace_id: "w1",
  project_id: null,
  goal_id: null,
  parent_task_id: null,
  title: "Write the changelog",
  description_markdown: null,
  notes_markdown: null,
  blocking_note_markdown: null,
  priority: "high",
  effort_size: "medium",
  effort_points: 3,
  target_date: null,
  scheduled_date: null,
  closure_mode: null,
  closure_reason_markdown: null,
  created_at: "2026-09-08T10:00:00Z",
  updated_at: "2026-09-08T10:00:00Z",
  archived_at: null,
};

// Both fixtures share one shape; only `status`/`completed_at` differ, so
// `completed_at` must stay `string | null` for `treeFor` to accept either.
type FixtureTask = typeof baseTask & { status: string; completed_at: string | null };

const inProgressTask: FixtureTask = { ...baseTask, status: "in_progress", completed_at: null };
const completedTaskFixture: FixtureTask = {
  ...baseTask,
  status: "completed",
  completed_at: "2026-09-08T17:00:00Z",
};

function treeFor(task: FixtureTask) {
  return { task, children: [], blocked_by: [] };
}

function renderPage() {
  return render(
    <MemoryRouter initialEntries={["/tasks/t1"]}>
      <Routes>
        <Route path="/tasks/:id" element={<TaskDetailPage />} />
      </Routes>
    </MemoryRouter>,
  );
}

describe("TaskDetailPage completion flow", () => {
  beforeEach(() => {
    vi.mocked(getTaskWithTree).mockReset();
    vi.mocked(listTasks).mockReset();
    vi.mocked(completeTask).mockReset();
    vi.mocked(listAllPrerequisites).mockReset();
    vi.mocked(getTaskDeadlineStatus).mockReset();

    vi.mocked(listTasks).mockResolvedValue([inProgressTask] as never);
    vi.mocked(listAllPrerequisites).mockResolvedValue([]);
    vi.mocked(getTaskDeadlineStatus).mockResolvedValue({
      status: "normal",
      at_risk_tasks: [],
      warning_window_days: 3,
    } as never);
    vi.mocked(completeTask).mockResolvedValue(completedTaskFixture as never);
    // First load shows the task in progress; the reload after completing
    // shows it completed.
    vi.mocked(getTaskWithTree)
      .mockResolvedValueOnce(treeFor(inProgressTask) as never)
      .mockResolvedValueOnce(treeFor(completedTaskFixture) as never);
  });

  it("completes the task and reflects the completed state after reload", async () => {
    renderPage();

    expect(await screen.findByRole("heading", { name: "Write the changelog" })).toBeTruthy();

    fireEvent.click(screen.getByRole("button", { name: "Complete" }));

    await waitFor(() => expect(completeTask).toHaveBeenCalledWith("t1"));
    // The reload after completing switches the action row to Reopen.
    expect(await screen.findByRole("button", { name: "Reopen task" })).toBeTruthy();
    // The status badge appears both in the header row and in the task tree,
    // so scope the assertion to the header row next to the title.
    const headerRow = screen.getByRole("heading", { name: "Write the changelog" })
      .parentElement as HTMLElement;
    expect(within(headerRow).getByText("Completed")).toBeTruthy();
  });
});
