import { describe, expect, it, vi } from "vitest";
import { fireEvent, render, screen } from "@testing-library/react";
import JobProgressModal from "@/components/jobs/JobProgressModal";
import type { JobProgressState } from "@/components/jobs/JobProgressModal";

function runningState(): JobProgressState {
  return {
    phase: "Running…",
    percent: null,
    status: "running",
    terminal: false,
    error: null,
    recoveryLabel: null,
    onRecovery: vi.fn(),
    onCancelRequest: vi.fn(),
  };
}

describe("JobProgressModal", () => {
  it("announces progress through a live region", () => {
    render(<JobProgressModal state={runningState()} onClose={vi.fn()} />);

    expect(screen.getByRole("status")).toHaveTextContent("Running…");
    expect(screen.getByText(/navigate elsewhere/)).toBeInTheDocument();
  });

  it("requests cooperative cancellation when Cancel is clicked", () => {
    const state = runningState();
    const onClose = vi.fn();
    render(<JobProgressModal state={state} onClose={onClose} />);

    fireEvent.click(screen.getByRole("button", { name: "Cancel" }));

    expect(state.onCancelRequest).toHaveBeenCalledTimes(1);
    expect(onClose).toHaveBeenCalledTimes(1);
  });

  it("closes without requesting cancellation when Close is clicked", () => {
    const state = runningState();
    const onClose = vi.fn();
    render(<JobProgressModal state={state} onClose={onClose} />);

    fireEvent.click(screen.getByLabelText("Close progress dialog"));

    expect(state.onCancelRequest).not.toHaveBeenCalled();
    expect(onClose).toHaveBeenCalledTimes(1);
  });

  it("shows a recovery action after failure", () => {
    const state: JobProgressState = {
      ...runningState(),
      status: "failed",
      terminal: true,
      error: "The export failed, but the Markdown source was saved.",
      recoveryLabel: "View report",
    };
    render(<JobProgressModal state={state} onClose={vi.fn()} />);

    const recovery = screen.getByRole("button", { name: "View report" });
    expect(recovery).toBeInTheDocument();
    fireEvent.click(recovery);
    expect(state.onRecovery).toHaveBeenCalledTimes(1);
  });
});
