import { describe, expect, it, vi, beforeEach } from "vitest";
import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { MemoryRouter, Route, Routes } from "react-router";
import NewIndependentTaskPage from "@/app/tasks/new/page";
import { createTask } from "@/lib/api/taskApi";
import { ensureDefaultWorkspace } from "@/lib/api/workspaceApi";

vi.mock("@/lib/api/taskApi", () => ({
  createTask: vi.fn(),
}));

vi.mock("@/lib/api/workspaceApi", () => ({
  ensureDefaultWorkspace: vi.fn(),
}));

const workspace = { id: "w1", name: "Default" };

const createdTask = {
  id: "t1",
  workspace_id: "w1",
  project_id: null,
  goal_id: null,
  parent_task_id: null,
  title: "Write the changelog",
  status: "inbox",
};

function renderPage(entry = "/tasks/new") {
  return render(
    <MemoryRouter initialEntries={[entry]}>
      <Routes>
        <Route path="/tasks/new" element={<NewIndependentTaskPage />} />
        <Route path="/tasks/:id" element={<p>Task detail stub</p>} />
      </Routes>
    </MemoryRouter>,
  );
}

describe("NewIndependentTaskPage", () => {
  beforeEach(() => {
    vi.mocked(createTask).mockReset();
    vi.mocked(ensureDefaultWorkspace).mockReset();
    vi.mocked(ensureDefaultWorkspace).mockResolvedValue(workspace as never);
    vi.mocked(createTask).mockResolvedValue(createdTask as never);
  });

  it("creates a standalone task with no project, goal, or parent", async () => {
    renderPage();

    expect(await screen.findByRole("heading", { name: "New task" })).toBeTruthy();

    fireEvent.change(screen.getByLabelText("Title"), {
      target: { value: "Write the changelog" },
    });
    fireEvent.click(screen.getByRole("button", { name: "Create task" }));

    await waitFor(() => expect(createTask).toHaveBeenCalledTimes(1));
    expect(createTask).toHaveBeenCalledWith(
      expect.objectContaining({
        workspace_id: "w1",
        project_id: null,
        goal_id: null,
        parent_task_id: null,
        title: "Write the changelog",
        priority: "normal",
        effort_size: null,
        target_date: null,
        scheduled_date: null,
      }),
    );
    expect(await screen.findByText("Task detail stub")).toBeTruthy();
  });

  it("links the task to the goal when arriving with goalId", async () => {
    renderPage("/tasks/new?goalId=g1");

    expect(await screen.findByRole("heading", { name: "New task" })).toBeTruthy();

    fireEvent.change(screen.getByLabelText("Title"), {
      target: { value: "Goal follow-up step" },
    });
    fireEvent.click(screen.getByRole("button", { name: "Create task" }));

    await waitFor(() => expect(createTask).toHaveBeenCalledTimes(1));
    expect(createTask).toHaveBeenCalledWith(
      expect.objectContaining({
        goal_id: "g1",
        project_id: null,
        parent_task_id: null,
      }),
    );
  });
});
