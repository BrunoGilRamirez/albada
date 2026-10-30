import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import BlockForm from "@/components/blocks/BlockForm";
import {
  createBlock,
  createBlockRecurrence,
  getBlockRecurrence,
  updateBlock,
  updateBlockRecurrence,
} from "@/lib/api/blockApi";
import { listTasks } from "@/lib/api/taskApi";
import type { Block, BlockRecurrence, BlockType, Task } from "@/lib/types";

vi.mock("@/lib/api/blockApi", () => ({
  createBlock: vi.fn(),
  updateBlock: vi.fn(),
  createBlockRecurrence: vi.fn(),
  updateBlockRecurrence: vi.fn(),
  getBlockRecurrence: vi.fn(),
}));

vi.mock("@/lib/api/taskApi", () => ({
  listTasks: vi.fn(),
}));

function makeBlock(overrides: Partial<Block> = {}): Block {
  return {
    id: "b1",
    workspace_id: "w1",
    daily_plan_id: null,
    date: "2026-10-19",
    block_type: "focus",
    status: "planned",
    label: "Morning writing",
    planned_start_at: "09:00",
    planned_end_at: "10:00",
    estimated_duration_seconds: 2700,
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
    lifecycle_state: "active",
    task_id: null,
    reactivated_from_block_id: null,
    ...overrides,
  };
}

function makeRule(overrides: Partial<BlockRecurrence> = {}): BlockRecurrence {
  return {
    id: "r1",
    workspace_id: "w1",
    version_id: "rv1",
    weekdays: [1, 3],
    local_start_time: "07:00",
    estimated_duration_seconds: 1800,
    block_type: "personal",
    label: "Morning walk",
    flexibility: "fixed",
    note_markdown: null,
    enabled: true,
    effective_from: "2026-09-01",
    effective_until: "2026-12-24",
    created_at: "2026-09-01T00:00:00Z",
    updated_at: "2026-09-01T00:00:00Z",
    lifecycle_state: "active",
    task_id: null,
    ...overrides,
  };
}

function makeTask(overrides: Partial<Task> = {}): Task {
  return {
    id: "t1",
    workspace_id: "w1",
    project_id: null,
    goal_id: null,
    parent_task_id: null,
    title: "Write the changelog",
    description_markdown: null,
    notes_markdown: null,
    blocking_note_markdown: null,
    status: "planned",
    priority: "normal",
    effort_size: null,
    effort_points: 3,
    target_date: null,
    scheduled_date: null,
    completed_at: null,
    cancelled_at: null,
    closure_mode: null,
    closure_reason_markdown: null,
    created_at: "2026-10-18T08:00:00Z",
    updated_at: "2026-10-18T08:00:00Z",
    archived_at: null,
    ...overrides,
  };
}

function renderForm(props: { editing?: Block | null; defaultType?: BlockType } = {}) {
  const handlers = {
    onPending: vi.fn(),
    onCommitted: vi.fn(),
    onRecurrenceCommitted: vi.fn(),
    onRollback: vi.fn(),
    onClose: vi.fn(),
  };
  render(<BlockForm workspaceId="w1" date="2026-10-19" editing={null} {...handlers} {...props} />);
  return handlers;
}

describe("BlockForm", () => {
  beforeEach(() => {
    vi.mocked(createBlock).mockReset();
    vi.mocked(updateBlock).mockReset();
    vi.mocked(createBlockRecurrence).mockReset();
    vi.mocked(updateBlockRecurrence).mockReset();
    vi.mocked(getBlockRecurrence).mockReset();
    vi.mocked(listTasks).mockReset();
  });

  afterEach(() => {
    cleanup();
  });

  it("saves an edited one-off block via updateBlock without planned_end_at", async () => {
    const editing = makeBlock({
      planned_start_at: "09:30",
      estimated_duration_seconds: 2700,
      time_window_label: "morning",
      note_markdown: "draft notes",
      flexibility: "fixed",
    });
    vi.mocked(updateBlock).mockResolvedValue(makeBlock({ label: "Renamed" }));
    const handlers = renderForm({ editing });

    expect(screen.getByLabelText("Label")).toHaveValue("Morning writing");
    expect(screen.getByLabelText("Planned start (optional)")).toHaveValue("09:30");
    expect(screen.getByLabelText("Estimated duration (minutes)")).toHaveValue(45);
    expect(screen.getByLabelText("Time window (optional)")).toHaveValue("morning");

    fireEvent.change(screen.getByLabelText("Label"), { target: { value: "Renamed" } });
    fireEvent.click(screen.getByRole("button", { name: "Save changes" }));

    await waitFor(() => expect(updateBlock).toHaveBeenCalledOnce());
    const args = vi.mocked(updateBlock).mock.calls[0]?.[1];
    expect(args).toBeDefined();
    expect(args).toEqual({
      daily_plan_id: null,
      date: "2026-10-19",
      block_type: "focus",
      label: "Renamed",
      planned_start_at: "09:30",
      estimated_duration_seconds: 2700,
      time_window_label: "morning",
      flexibility: "fixed",
      note_markdown: "draft notes",
      task_id: null,
    });
    expect(Object.keys(args as object)).not.toContain("planned_end_at");
    expect(handlers.onPending).toHaveBeenCalledOnce();
    expect(handlers.onCommitted).toHaveBeenCalledOnce();
  });

  it("submits the manual time window verbatim; no band is derived (B5)", async () => {
    vi.mocked(updateBlock).mockResolvedValue(makeBlock());
    const editing = makeBlock({
      planned_start_at: "07:00",
      estimated_duration_seconds: null,
      time_window_label: "evening",
    });
    renderForm({ editing });
    fireEvent.click(screen.getByRole("button", { name: "Save changes" }));
    await waitFor(() => expect(updateBlock).toHaveBeenCalledOnce());
    expect(vi.mocked(updateBlock).mock.calls[0]?.[1]).toMatchObject({
      planned_start_at: "07:00",
      time_window_label: "evening",
      estimated_duration_seconds: null,
    });
  });

  it("closes an edit form through Cancel without persisting", () => {
    const handlers = renderForm({ editing: makeBlock() });
    fireEvent.click(screen.getByRole("button", { name: "Cancel" }));
    expect(handlers.onClose).toHaveBeenCalledOnce();
    expect(updateBlock).not.toHaveBeenCalled();
    expect(createBlock).not.toHaveBeenCalled();
    expect(handlers.onPending).not.toHaveBeenCalled();
  });

  it("rejects a non-positive duration before calling the native command", async () => {
    vi.mocked(updateBlock).mockResolvedValue(makeBlock());
    renderForm({ editing: makeBlock() });
    fireEvent.change(screen.getByLabelText("Estimated duration (minutes)"), {
      target: { value: "0" },
    });
    fireEvent.click(screen.getByRole("button", { name: "Save changes" }));
    expect(await screen.findByRole("alert")).toHaveTextContent(
      "Enter a whole number of minutes greater than zero.",
    );
    expect(updateBlock).not.toHaveBeenCalled();
  });

  it("loads the recurrence rule, prefills ISO weekdays, saves the rule", async () => {
    const rule = makeRule();
    vi.mocked(getBlockRecurrence).mockResolvedValue(rule);
    vi.mocked(updateBlockRecurrence).mockResolvedValue({
      ...rule,
      version_id: "rv2",
      estimated_duration_seconds: 3600,
    });
    const editing = makeBlock({
      recurrence_id: "r1",
      recurrence_version_id: "rv1",
      block_type: "personal",
      label: "Morning walk",
      planned_start_at: "07:00",
      local_start_time: "07:00",
      estimated_duration_seconds: 1800,
      flexibility: "fixed",
    });
    const handlers = renderForm({ editing });

    expect(getBlockRecurrence).toHaveBeenCalledWith("w1", "r1");
    await screen.findByText(
      "You are editing the weekly rule. Changes apply to future occurrences; completed and skipped history is preserved.",
    );
    expect(screen.getByLabelText("Planned start (optional)")).toHaveValue("07:00");
    expect(screen.getByLabelText("Estimated duration (minutes)")).toHaveValue(30);
    expect(screen.getByRole("button", { name: "Monday" })).toHaveAttribute("aria-pressed", "true");
    expect(screen.getByRole("button", { name: "Wednesday" })).toHaveAttribute(
      "aria-pressed",
      "true",
    );
    expect(screen.getByRole("button", { name: "Tuesday" })).toHaveAttribute(
      "aria-pressed",
      "false",
    );

    fireEvent.change(screen.getByLabelText("Estimated duration (minutes)"), {
      target: { value: "60" },
    });
    fireEvent.click(screen.getByRole("button", { name: "Save changes" }));

    await waitFor(() => expect(updateBlockRecurrence).toHaveBeenCalledOnce());
    expect(updateBlockRecurrence).toHaveBeenCalledWith(
      "r1",
      expect.objectContaining({
        workspace_id: "w1",
        weekdays: [1, 3],
        local_start_time: "07:00",
        estimated_duration_seconds: 3600,
        block_type: "personal",
        label: "Morning walk",
        flexibility: "fixed",
        note_markdown: null,
        enabled: true,
        effective_from: "2026-09-01",
        effective_until: "2026-12-24",
      }),
    );
    // The occurrence row itself must not go through update_block: the native
    // command rejects recurrence materializations.
    expect(updateBlock).not.toHaveBeenCalled();
    expect(handlers.onRecurrenceCommitted).toHaveBeenCalledOnce();
  });

  it("requires an exact local start for an existing recurrence", async () => {
    vi.mocked(getBlockRecurrence).mockResolvedValue(makeRule());
    const handlers = renderForm({
      editing: makeBlock({ recurrence_id: "r1", recurrence_version_id: "rv1" }),
    });
    await screen.findByText(/You are editing the weekly rule/);
    fireEvent.change(screen.getByLabelText("Planned start (optional)"), {
      target: { value: "" },
    });
    fireEvent.click(screen.getByRole("button", { name: "Save changes" }));
    expect(await screen.findByRole("alert")).toHaveTextContent(
      "Recurring blocks need an exact start time.",
    );
    expect(updateBlockRecurrence).not.toHaveBeenCalled();
    expect(handlers.onPending).not.toHaveBeenCalled();
  });

  it("creates a weekly recurrence with ISO weekdays and duration", async () => {
    vi.mocked(createBlockRecurrence).mockResolvedValue(makeRule());
    const handlers = renderForm();
    fireEvent.click(screen.getByRole("checkbox", { name: "Repeats weekly" }));
    fireEvent.click(screen.getByRole("button", { name: "Friday" }));
    fireEvent.change(screen.getByLabelText("Label"), { target: { value: "Team sync" } });
    fireEvent.change(screen.getByLabelText("Planned start (optional)"), {
      target: { value: "08:00" },
    });
    fireEvent.change(screen.getByLabelText("Estimated duration (minutes)"), {
      target: { value: "30" },
    });
    fireEvent.click(screen.getByRole("button", { name: "Add block" }));

    await waitFor(() => expect(createBlockRecurrence).toHaveBeenCalledOnce());
    expect(createBlockRecurrence).toHaveBeenCalledWith({
      workspace_id: "w1",
      weekdays: [5],
      local_start_time: "08:00",
      estimated_duration_seconds: 1800,
      block_type: "focus",
      label: "Team sync",
      flexibility: "flexible",
      note_markdown: null,
      enabled: true,
      effective_from: "2026-10-19",
      effective_until: null,
      task_id: null,
    });
    expect(createBlock).not.toHaveBeenCalled();
    expect(handlers.onRecurrenceCommitted).toHaveBeenCalledOnce();
  });

  it("requires at least one weekday for weekly repetition", async () => {
    vi.mocked(createBlock).mockResolvedValue(makeBlock());
    renderForm();
    fireEvent.click(screen.getByRole("checkbox", { name: "Repeats weekly" }));
    fireEvent.change(screen.getByLabelText("Label"), { target: { value: "Stand-up" } });
    fireEvent.change(screen.getByLabelText("Planned start (optional)"), {
      target: { value: "09:00" },
    });
    fireEvent.change(screen.getByLabelText("Estimated duration (minutes)"), {
      target: { value: "15" },
    });
    fireEvent.click(screen.getByRole("button", { name: "Add block" }));
    expect(await screen.findByRole("alert")).toHaveTextContent("Select at least one weekday.");
    expect(createBlockRecurrence).not.toHaveBeenCalled();
    expect(createBlock).not.toHaveBeenCalled();
  });

  it("shows the translated error and rolls back when the native command rejects", async () => {
    vi.mocked(updateBlock).mockRejectedValue(
      new Error("Estimated block duration must be a positive number of seconds."),
    );
    const handlers = renderForm({ editing: makeBlock() });
    fireEvent.click(screen.getByRole("button", { name: "Save changes" }));
    await waitFor(() => expect(handlers.onRollback).toHaveBeenCalledOnce());
    expect(await screen.findByRole("alert")).toHaveTextContent(
      "Estimated block duration must be a positive number of seconds.",
    );
    // The provisional row was handed to the list before the rejection.
    expect(handlers.onPending).toHaveBeenCalledOnce();
  });

  it("reveals the task picker only when the checkbox is checked", async () => {
    vi.mocked(listTasks).mockResolvedValue([makeTask()]);
    renderForm();
    expect(screen.queryByLabelText("Task")).toBeNull();

    fireEvent.click(screen.getByRole("checkbox", { name: "Work on a task" }));
    expect(await screen.findByLabelText("Task")).toBeTruthy();
    await waitFor(() => expect(listTasks).toHaveBeenCalledWith("w1"));

    // The loaded rows are kept: collapsing and reopening never refetches.
    fireEvent.click(screen.getByRole("checkbox", { name: "Work on a task" }));
    fireEvent.click(screen.getByRole("checkbox", { name: "Work on a task" }));
    await waitFor(() => expect(screen.getByLabelText("Task")).toBeTruthy());
    expect(listTasks).toHaveBeenCalledOnce();
  });

  it("offers only tasks that can still be worked on", async () => {
    vi.mocked(listTasks).mockResolvedValue([
      makeTask({ id: "t-open", title: "Open task" }),
      makeTask({ id: "t-done", title: "Completed task", status: "completed" }),
      makeTask({ id: "t-cancel", title: "Cancelled task", status: "cancelled" }),
      makeTask({ id: "t-archived", title: "Archived task", archived_at: "2026-01-02T00:00:00Z" }),
      {
        ...makeTask({ id: "t-overdue", title: "Overdue task" }),
        target_date: "2020-01-01",
      } as Task,
      {
        ...makeTask({ id: "t-future", title: "Future deadline task" }),
        target_date: "2099-01-01",
      } as Task,
      {
        ...makeTask({ id: "t-deferred", title: "Deferred task", status: "deferred" }),
        target_date: "2020-01-01",
      } as Task,
    ]);
    renderForm();
    fireEvent.click(screen.getByRole("checkbox", { name: "Work on a task" }));

    const select = (await screen.findByLabelText("Task")) as HTMLSelectElement;
    // Deferred tasks derive a `none` deadline condition natively, so a past
    // `target_date` does not make them overdue.
    await waitFor(() => expect(select.options.length).toBe(4));
    const options = Array.from(select.options).map((option) => option.textContent);
    expect(options).toContain("Open task");
    expect(options).toContain("Future deadline task");
    expect(options).toContain("Deferred task");
    expect(options).not.toContain("Completed task");
    expect(options).not.toContain("Cancelled task");
    expect(options).not.toContain("Archived task");
    expect(options).not.toContain("Overdue task");
  });

  it("prefills the label with the localized work-on text", async () => {
    vi.mocked(listTasks).mockResolvedValue([
      makeTask({ id: "t-open", title: "Write the changelog" }),
    ]);
    renderForm();
    fireEvent.click(screen.getByRole("checkbox", { name: "Work on a task" }));
    const select = (await screen.findByLabelText("Task")) as HTMLSelectElement;
    await waitFor(() => expect(select.options.length).toBe(2));

    fireEvent.change(select, { target: { value: "t-open" } });
    // The title is never translated or rewritten; only the label is.
    expect(screen.getByLabelText("Label")).toHaveValue("Work on Write the changelog");
  });

  it("keeps a manually written label when the task selection changes", async () => {
    vi.mocked(listTasks).mockResolvedValue([
      makeTask({ id: "t-first", title: "First task" }),
      makeTask({ id: "t-second", title: "Second task" }),
    ]);
    renderForm();
    fireEvent.click(screen.getByRole("checkbox", { name: "Work on a task" }));
    const select = (await screen.findByLabelText("Task")) as HTMLSelectElement;
    await waitFor(() => expect(select.options.length).toBe(3));

    fireEvent.change(select, { target: { value: "t-first" } });
    expect(screen.getByLabelText("Label")).toHaveValue("Work on First task");

    fireEvent.change(screen.getByLabelText("Label"), { target: { value: "My own wording" } });
    fireEvent.change(select, { target: { value: "t-second" } });
    expect(screen.getByLabelText("Label")).toHaveValue("My own wording");
  });

  it("submits the selected task and clears the reference when unchecked", async () => {
    vi.mocked(listTasks).mockResolvedValue([makeTask({ id: "t-open", title: "Open task" })]);
    vi.mocked(createBlock).mockResolvedValue(makeBlock());
    renderForm();
    fireEvent.click(screen.getByRole("checkbox", { name: "Work on a task" }));
    const select = (await screen.findByLabelText("Task")) as HTMLSelectElement;
    await waitFor(() => expect(select.options.length).toBe(2));

    fireEvent.change(select, { target: { value: "t-open" } });
    fireEvent.click(screen.getByRole("button", { name: "Add block" }));
    await waitFor(() => expect(createBlock).toHaveBeenCalledOnce());
    expect(vi.mocked(createBlock).mock.calls[0]?.[0]).toMatchObject({
      task_id: "t-open",
      label: "Work on Open task",
    });

    // Unchecking removes the optional task reference again.
    fireEvent.click(screen.getByRole("checkbox", { name: "Work on a task" }));
    fireEvent.click(screen.getByRole("button", { name: "Add block" }));
    await waitFor(() => expect(createBlock).toHaveBeenCalledTimes(2));
    expect(vi.mocked(createBlock).mock.calls[1]?.[0]).toMatchObject({ task_id: null });
  });
});
