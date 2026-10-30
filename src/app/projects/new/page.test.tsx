import { describe, expect, it, vi, beforeEach } from "vitest";
import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { MemoryRouter, Route, Routes } from "react-router";
import NewProjectPage from "@/app/projects/new/page";
import { createProject } from "@/lib/api/projectApi";
import { ensureDefaultWorkspace } from "@/lib/api/workspaceApi";

vi.mock("@/lib/api/projectApi", () => ({
  createProject: vi.fn(),
}));

vi.mock("@/lib/api/workspaceApi", () => ({
  ensureDefaultWorkspace: vi.fn(),
}));

const workspace = { id: "w1", name: "Default" };

const createdProject = {
  id: "p1",
  workspace_id: "w1",
  goal_id: null,
  title: "Set up test harness",
  description_markdown: null,
  why_it_matters_markdown: null,
  definition_of_done_markdown: null,
  status: "inbox",
  priority: "none",
  target_date: null,
  created_at: "2026-09-08T10:00:00Z",
  updated_at: "2026-09-08T10:00:00Z",
  archived_at: null,
};

function renderPage() {
  return render(
    <MemoryRouter initialEntries={["/projects/new"]}>
      <Routes>
        <Route path="/projects/new" element={<NewProjectPage />} />
        <Route path="/projects/:id" element={<p>Project detail stub</p>} />
      </Routes>
    </MemoryRouter>,
  );
}

describe("NewProjectPage", () => {
  beforeEach(() => {
    vi.mocked(createProject).mockReset();
    vi.mocked(ensureDefaultWorkspace).mockReset();
    vi.mocked(ensureDefaultWorkspace).mockResolvedValue(workspace as never);
    vi.mocked(createProject).mockResolvedValue(createdProject as never);
  });

  it("creates a project from the form and routes to its detail page", async () => {
    renderPage();

    fireEvent.change(await screen.findByLabelText("Title"), {
      target: { value: "Set up test harness" },
    });
    fireEvent.click(screen.getByRole("button", { name: "Create project" }));

    await waitFor(() => expect(createProject).toHaveBeenCalledTimes(1));
    expect(createProject).toHaveBeenCalledWith(
      expect.objectContaining({
        workspace_id: "w1",
        goal_id: null,
        title: "Set up test harness",
        description_markdown: null,
        why_it_matters_markdown: null,
        definition_of_done_markdown: null,
      }),
    );
    expect(await screen.findByText("Project detail stub")).toBeTruthy();
  });

  it("keeps the goal link when arriving from a goal", async () => {
    render(
      <MemoryRouter initialEntries={["/projects/new?goalId=g1"]}>
        <Routes>
          <Route path="/projects/new" element={<NewProjectPage />} />
          <Route path="/projects/:id" element={<p>Project detail stub</p>} />
        </Routes>
      </MemoryRouter>,
    );

    expect(await screen.findByText(/linked to the goal/)).toBeTruthy();
    fireEvent.change(screen.getByLabelText("Title"), {
      target: { value: "Set up test harness" },
    });
    fireEvent.click(screen.getByRole("button", { name: "Create project" }));

    await waitFor(() => expect(createProject).toHaveBeenCalledTimes(1));
    expect(createProject).toHaveBeenCalledWith(expect.objectContaining({ goal_id: "g1" }));
  });
});
