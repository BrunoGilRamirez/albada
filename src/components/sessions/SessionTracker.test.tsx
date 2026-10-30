import { describe, expect, it, vi, beforeEach } from "vitest";
import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import SessionTracker from "@/components/sessions/SessionTracker";

vi.mock("@/lib/api/sessionApi", () => ({
  listSessionsForTask: vi.fn(),
  createSession: vi.fn(),
  startSession: vi.fn(),
  pauseSession: vi.fn(),
  finishSession: vi.fn(),
  skipSession: vi.fn(),
}));

import { createSession, listSessionsForTask } from "@/lib/api/sessionApi";

const startedSession = {
  id: "session-1",
  task_id: "task-1",
  block_id: null,
  daily_plan_task_id: null,
  started_at: new Date(Date.now() - 65_000).toISOString(),
  ended_at: null,
  status: "started",
  outcome_markdown: null,
  next_step_markdown: null,
  created_at: "2026-09-04T10:00:00Z",
  updated_at: "2026-09-04T10:00:00Z",
};

describe("SessionTracker", () => {
  beforeEach(() => {
    vi.mocked(listSessionsForTask).mockReset();
    vi.mocked(createSession).mockReset();
  });

  it("shows a start button when there is no active session", () => {
    vi.mocked(listSessionsForTask).mockResolvedValue([]);
    render(<SessionTracker taskId="task-1" />);
    expect(screen.getByText("Start session")).toBeTruthy();
  });

  it("shows pause, finish, and skip buttons for an active session", async () => {
    vi.mocked(listSessionsForTask).mockResolvedValue([startedSession as never]);
    render(<SessionTracker taskId="task-1" />);
    expect(await screen.findByText("Pause")).toBeTruthy();
    expect(screen.getByText("Finish")).toBeTruthy();
    expect(screen.getByText("Skip")).toBeTruthy();
  });

  it("shows an elapsed timer for a started session", async () => {
    vi.mocked(listSessionsForTask).mockResolvedValue([startedSession as never]);
    render(<SessionTracker taskId="task-1" />);
    // 65 seconds elapsed renders as "01:05".
    expect(await screen.findByText("01:05")).toBeTruthy();
  });

  it("starts a focus session when Start session is clicked", async () => {
    // First load finds nothing active; the refresh after creating the session
    // returns the started one.
    vi.mocked(listSessionsForTask)
      .mockResolvedValueOnce([])
      .mockResolvedValueOnce([startedSession] as never);
    vi.mocked(createSession).mockResolvedValue(startedSession as never);

    render(<SessionTracker taskId="task-1" />);

    fireEvent.click(await screen.findByRole("button", { name: "Start session" }));

    await waitFor(() =>
      expect(createSession).toHaveBeenCalledWith({
        task_id: "task-1",
        block_id: null,
        daily_plan_task_id: null,
      }),
    );
    // The tracker switches to the active-session controls.
    expect(await screen.findByRole("button", { name: "Pause" })).toBeTruthy();
    expect(screen.getByRole("button", { name: "Finish" })).toBeTruthy();
  });
});
