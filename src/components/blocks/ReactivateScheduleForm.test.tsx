import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import ReactivateScheduleForm from "@/components/blocks/ReactivateScheduleForm";
import { reactivateBlock } from "@/lib/api/blockApi";
import type { Block } from "@/lib/types";

vi.mock("@/lib/api/blockApi", () => ({
  reactivateBlock: vi.fn(),
}));

function makeBlock(overrides: Partial<Block> = {}): Block {
  return {
    id: "b1",
    workspace_id: "w1",
    daily_plan_id: null,
    date: "2026-10-19",
    block_type: "focus",
    status: "completed",
    label: "Morning writing",
    planned_start_at: null,
    planned_end_at: null,
    estimated_duration_seconds: null,
    recurrence_id: null,
    recurrence_version_id: null,
    local_date: null,
    local_start_time: null,
    timezone_id: null,
    resolved_start_at: null,
    time_window_label: null,
    actual_start_at: null,
    actual_end_at: null,
    flexibility: "flexible",
    note_markdown: null,
    created_at: "2026-10-18T08:00:00Z",
    updated_at: "2026-10-18T08:00:00Z",
    lifecycle_state: "inactive",
    task_id: null,
    reactivated_from_block_id: null,
    ...overrides,
  };
}

function renderForm(
  props: { block?: Block; now?: Date } = {},
  handlers: { onClose?: () => void; onReactivated?: () => void } = {},
) {
  const onClose = handlers.onClose ?? vi.fn();
  const onReactivated = handlers.onReactivated ?? vi.fn();
  render(
    <ReactivateScheduleForm
      workspaceId="w1"
      block={props.block ?? makeBlock()}
      now={props.now}
      onClose={onClose}
      onReactivated={onReactivated}
    />,
  );
  return { onClose, onReactivated };
}

describe("ReactivateScheduleForm", () => {
  beforeEach(() => {
    vi.mocked(reactivateBlock).mockReset();
  });

  afterEach(() => {
    cleanup();
  });

  it("defaults the weekday to the active local day", () => {
    // Wednesday 2026-10-21, 09:00 local time.
    renderForm({ now: new Date(2026, 9, 21, 9, 0, 0) });
    expect(screen.getByRole("button", { name: "Wednesday" })).toHaveAttribute(
      "aria-pressed",
      "true",
    );
    expect(screen.getByRole("button", { name: "Thursday" })).toHaveAttribute(
      "aria-pressed",
      "false",
    );
  });

  it("defaults the weekday to the next day after 23:00 local time", () => {
    // Wednesday 2026-10-21, 23:05 local time: the day is already over.
    renderForm({ now: new Date(2026, 9, 21, 23, 5, 0) });
    expect(screen.getByRole("button", { name: "Thursday" })).toHaveAttribute(
      "aria-pressed",
      "true",
    );
    expect(screen.getByRole("button", { name: "Wednesday" })).toHaveAttribute(
      "aria-pressed",
      "false",
    );
  });

  it("requires an explicit once or recurring choice", async () => {
    vi.mocked(reactivateBlock).mockResolvedValue({ block: null, recurrence: null });
    renderForm({ now: new Date(2026, 9, 21, 9, 0, 0) });

    const once = screen.getByRole("radio", { name: "Once" }) as HTMLInputElement;
    const recurring = screen.getByRole("radio", { name: "Weekly" }) as HTMLInputElement;
    expect(once.checked).toBe(true);
    expect(recurring.checked).toBe(false);

    fireEvent.click(recurring);
    expect(recurring.checked).toBe(true);
    expect(once.checked).toBe(false);
    // The recurring choice reveals the exact start and duration inputs.
    expect(screen.getByLabelText("Start time (HH:MM)")).toBeTruthy();
    expect(screen.getByLabelText("Duration (minutes)")).toBeTruthy();
  });

  it("rejects a recurring reactivation without an exact start time", async () => {
    renderForm({ now: new Date(2026, 9, 21, 9, 0, 0) });
    fireEvent.click(screen.getByRole("radio", { name: "Weekly" }));
    fireEvent.change(screen.getByLabelText("Duration (minutes)"), { target: { value: "30" } });
    fireEvent.click(screen.getByRole("button", { name: "Reactivate block" }));

    expect(await screen.findByRole("alert")).toHaveTextContent(
      "Recurring blocks need an exact start time.",
    );
    expect(reactivateBlock).not.toHaveBeenCalled();
  });

  it("rejects a non-positive or fractional duration", async () => {
    renderForm({ now: new Date(2026, 9, 21, 9, 0, 0) });
    fireEvent.click(screen.getByRole("radio", { name: "Weekly" }));
    fireEvent.change(screen.getByLabelText("Start time (HH:MM)"), { target: { value: "07:30" } });

    fireEvent.change(screen.getByLabelText("Duration (minutes)"), { target: { value: "0" } });
    fireEvent.click(screen.getByRole("button", { name: "Reactivate block" }));
    expect(await screen.findByRole("alert")).toHaveTextContent(
      "Enter a whole number of minutes greater than zero.",
    );

    fireEvent.change(screen.getByLabelText("Duration (minutes)"), { target: { value: "2.5" } });
    fireEvent.click(screen.getByRole("button", { name: "Reactivate block" }));
    expect(await screen.findByRole("alert")).toHaveTextContent(
      "Enter a whole number of minutes greater than zero.",
    );
    expect(reactivateBlock).not.toHaveBeenCalled();
  });

  it("prefills the source time and duration and reactivates once", async () => {
    vi.mocked(reactivateBlock).mockResolvedValue({
      block: makeBlock({ id: "new-1" }),
      recurrence: null,
    });
    const { onClose, onReactivated } = renderForm(
      {
        block: makeBlock({ planned_start_at: "09:00", estimated_duration_seconds: 2700 }),
        now: new Date(2026, 9, 21, 9, 0, 0),
      },
      {},
    );

    expect(screen.getByLabelText("Start time (HH:MM)")).toHaveValue("09:00");
    expect(screen.getByLabelText("Duration (minutes)")).toHaveValue(45);

    fireEvent.click(screen.getByRole("button", { name: "Reactivate block" }));
    await waitFor(() => expect(reactivateBlock).toHaveBeenCalledOnce());
    expect(reactivateBlock).toHaveBeenCalledWith({
      workspace_id: "w1",
      block_id: "b1",
      schedule: "once",
      weekday: 3,
    });
    // A successful reactivation closes the form and refreshes the owner.
    expect(onReactivated).toHaveBeenCalledOnce();
    expect(onClose).toHaveBeenCalledOnce();
  });

  it("reactivates as a weekly series with the chosen start and duration", async () => {
    vi.mocked(reactivateBlock).mockResolvedValue({ block: null, recurrence: null });
    renderForm({ now: new Date(2026, 9, 21, 9, 0, 0) });

    fireEvent.click(screen.getByRole("radio", { name: "Weekly" }));
    fireEvent.click(screen.getByRole("button", { name: "Friday" }));
    fireEvent.change(screen.getByLabelText("Start time (HH:MM)"), { target: { value: "07:30" } });
    fireEvent.change(screen.getByLabelText("Duration (minutes)"), { target: { value: "45" } });
    fireEvent.click(screen.getByRole("button", { name: "Reactivate block" }));

    await waitFor(() => expect(reactivateBlock).toHaveBeenCalledOnce());
    expect(reactivateBlock).toHaveBeenCalledWith({
      workspace_id: "w1",
      block_id: "b1",
      schedule: "recurring",
      weekday: 5,
      local_start_time: "07:30",
      estimated_duration_seconds: 2700,
    });
  });

  it("shows the native failure and keeps the form open", async () => {
    vi.mocked(reactivateBlock).mockRejectedValue(new Error("Reactivation failed."));
    const { onClose } = renderForm({ now: new Date(2026, 9, 21, 9, 0, 0) });

    fireEvent.click(screen.getByRole("button", { name: "Reactivate block" }));
    expect(await screen.findByRole("alert")).toHaveTextContent("Reactivation failed.");
    expect(onClose).not.toHaveBeenCalled();
  });
});
