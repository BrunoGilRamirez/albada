import { describe, expect, it, vi, beforeEach } from "vitest";
import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { MemoryRouter, Route, Routes } from "react-router";
import NewGoalPage from "@/app/goals/new/page";
import { createGoal } from "@/lib/api/goalApi";
import { ensureDefaultWorkspace } from "@/lib/api/workspaceApi";

vi.mock("@/lib/api/goalApi", () => ({
  createGoal: vi.fn(),
}));

vi.mock("@/lib/api/workspaceApi", () => ({
  ensureDefaultWorkspace: vi.fn(),
}));

const workspace = { id: "w1", name: "Default" };

const createdGoal = {
  id: "g1",
  workspace_id: "w1",
  year_theme_id: null,
  title: "Learn testing",
  description_markdown: null,
  why_it_matters_markdown: null,
  success_definition_markdown: null,
  status: "inbox",
  priority: "none",
  target_date: null,
  created_at: "2026-09-08T10:00:00Z",
  updated_at: "2026-09-08T10:00:00Z",
  archived_at: null,
};

function renderPage() {
  return render(
    <MemoryRouter initialEntries={["/goals/new"]}>
      <Routes>
        <Route path="/goals/new" element={<NewGoalPage />} />
        <Route path="/goals/:id" element={<p>Goal detail stub</p>} />
      </Routes>
    </MemoryRouter>,
  );
}

describe("NewGoalPage", () => {
  beforeEach(() => {
    vi.mocked(createGoal).mockReset();
    vi.mocked(ensureDefaultWorkspace).mockReset();
    vi.mocked(ensureDefaultWorkspace).mockResolvedValue(workspace as never);
    vi.mocked(createGoal).mockResolvedValue(createdGoal as never);
  });

  it("creates a goal from the form and routes to its detail page", async () => {
    renderPage();

    fireEvent.change(await screen.findByLabelText("Title"), {
      target: { value: "Learn testing" },
    });
    fireEvent.click(screen.getByRole("button", { name: "Create goal" }));

    await waitFor(() => expect(createGoal).toHaveBeenCalledTimes(1));
    expect(createGoal).toHaveBeenCalledWith(
      expect.objectContaining({
        workspace_id: "w1",
        year_theme_id: null,
        title: "Learn testing",
        description_markdown: null,
        why_it_matters_markdown: null,
        success_definition_markdown: null,
      }),
    );
    expect(await screen.findByText("Goal detail stub")).toBeTruthy();
  });

  it("surfaces a failed create without navigating away", async () => {
    vi.mocked(createGoal).mockRejectedValue(new Error("boom"));
    renderPage();

    fireEvent.change(await screen.findByLabelText("Title"), {
      target: { value: "Learn testing" },
    });
    fireEvent.click(screen.getByRole("button", { name: "Create goal" }));

    expect(await screen.findByText("boom")).toBeTruthy();
    expect(screen.queryByText("Goal detail stub")).toBeNull();
  });
});
