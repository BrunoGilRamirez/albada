import { describe, expect, it, vi, beforeEach } from "vitest";
import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { MemoryRouter, Route, Routes } from "react-router";
import NewChildTaskPage from "@/app/tasks/[id]/children/new/page";
import { createChildTask } from "@/lib/api/taskApi";

vi.mock("@/lib/api/taskApi", () => ({
  createChildTask: vi.fn(),
}));

const createdChild = {
  id: "t2",
  workspace_id: "w1",
  project_id: null,
  goal_id: null,
  parent_task_id: "parent-1",
  title: "Collect the numbers",
  status: "inbox",
};

function renderPage() {
  return render(
    <MemoryRouter initialEntries={["/tasks/parent-1/children/new"]}>
      <Routes>
        <Route path="/tasks/:id/children/new" element={<NewChildTaskPage />} />
        <Route path="/tasks/:taskId" element={<p>Task detail stub</p>} />
      </Routes>
    </MemoryRouter>,
  );
}

describe("NewChildTaskPage", () => {
  beforeEach(() => {
    vi.mocked(createChildTask).mockReset();
    vi.mocked(createChildTask).mockResolvedValue(createdChild as never);
  });

  it("creates a child task under the parent from the route", async () => {
    renderPage();

    expect(await screen.findByRole("heading", { name: "New child task" })).toBeTruthy();

    fireEvent.change(screen.getByLabelText("Title"), {
      target: { value: "Collect the numbers" },
    });
    fireEvent.click(screen.getByRole("button", { name: "Create child task" }));

    await waitFor(() => expect(createChildTask).toHaveBeenCalledTimes(1));
    expect(createChildTask).toHaveBeenCalledWith(
      expect.objectContaining({
        parent_id: "parent-1",
        title: "Collect the numbers",
        priority: "normal",
        effort_size: null,
      }),
    );
    expect(await screen.findByText("Task detail stub")).toBeTruthy();
  });

  it("shows the API error when the child cannot be created", async () => {
    vi.mocked(createChildTask).mockRejectedValue(new Error("Parent not found"));
    renderPage();

    fireEvent.change(await screen.findByLabelText("Title"), {
      target: { value: "Collect the numbers" },
    });
    fireEvent.click(screen.getByRole("button", { name: "Create child task" }));

    expect(await screen.findByText("Parent not found")).toBeTruthy();
    expect(screen.queryByText("Task detail stub")).toBeNull();
  });
});
