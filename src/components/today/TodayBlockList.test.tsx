import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { useState } from "react";
import { cleanup, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import TodayBlockList from "@/components/today/TodayBlockList";
import {
  cancelBlock,
  completeBlock,
  createBlock,
  createBlockRecurrence,
  deactivateBlock,
  deactivateBlockRecurrence,
  deleteBlock,
  deleteBlockRecurrence,
  getBlockRecurrence,
  listBlocksForDate,
  listInactiveBlockRecurrences,
  listInactiveBlocks,
  listScheduledBlockRecurrences,
  listUpcomingBlocks,
  reactivateBlock,
  reactivateBlockRecurrence,
  skipBlock,
  startBlock,
  updateBlock,
  updateBlockRecurrence,
} from "@/lib/api/blockApi";
import type { Block, BlockRecurrence, ScheduledBlockRecurrence } from "@/lib/types";

vi.mock("@/lib/api/blockApi", () => ({
  createBlock: vi.fn(),
  updateBlock: vi.fn(),
  createBlockRecurrence: vi.fn(),
  updateBlockRecurrence: vi.fn(),
  getBlockRecurrence: vi.fn(),
  listBlocksForDate: vi.fn(),
  startBlock: vi.fn(),
  completeBlock: vi.fn(),
  skipBlock: vi.fn(),
  cancelBlock: vi.fn(),
  deactivateBlock: vi.fn(),
  deleteBlock: vi.fn(),
  reactivateBlock: vi.fn(),
  reactivateBlockRecurrence: vi.fn(),
  deactivateBlockRecurrence: vi.fn(),
  deleteBlockRecurrence: vi.fn(),
  listInactiveBlocks: vi.fn(),
  listInactiveBlockRecurrences: vi.fn(),
  listScheduledBlockRecurrences: vi.fn(),
  listUpcomingBlocks: vi.fn(),
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
    weekdays: [1],
    local_start_time: "07:00",
    estimated_duration_seconds: 1800,
    block_type: "personal",
    label: "Morning walk",
    flexibility: "fixed",
    note_markdown: null,
    enabled: true,
    effective_from: "2026-09-01",
    effective_until: null,
    created_at: "2026-09-01T00:00:00Z",
    updated_at: "2026-09-01T00:00:00Z",
    lifecycle_state: "active",
    task_id: null,
    ...overrides,
  };
}

function makeScheduledRule(
  overrides: Partial<ScheduledBlockRecurrence> = {},
): ScheduledBlockRecurrence {
  return {
    recurrence: makeRule(),
    next_occurrence_date: "2026-10-26",
    ...overrides,
  };
}

// Controlled-list harness mirroring the Today page: the list owns its rows
// through `onChanged` and re-renders from the committed state.
function Harness({ initial }: { initial: Block[] }) {
  const [blocks, setBlocks] = useState<Block[]>(initial);
  return (
    <TodayBlockList
      workspaceId="w1"
      date="2026-10-19"
      dailyPlanId={null}
      blocks={blocks}
      onChanged={setBlocks}
    />
  );
}

function cardTexts(): string[] {
  return screen.getAllByRole("article").map((card) => card.textContent ?? "");
}

describe("TodayBlockList", () => {
  beforeEach(() => {
    vi.mocked(createBlock).mockReset();
    vi.mocked(updateBlock).mockReset();
    vi.mocked(createBlockRecurrence).mockReset();
    vi.mocked(updateBlockRecurrence).mockReset();
    vi.mocked(getBlockRecurrence).mockReset();
    vi.mocked(listBlocksForDate).mockReset();
    vi.mocked(startBlock).mockReset();
    vi.mocked(completeBlock).mockReset();
    vi.mocked(skipBlock).mockReset();
    vi.mocked(cancelBlock).mockReset();
    vi.mocked(deactivateBlock).mockReset();
    vi.mocked(deleteBlock).mockReset();
    vi.mocked(reactivateBlock).mockReset();
    vi.mocked(deactivateBlockRecurrence).mockReset();
    vi.mocked(deleteBlockRecurrence).mockReset();
    vi.mocked(reactivateBlockRecurrence).mockReset();
    vi.mocked(listInactiveBlocks).mockReset();
    vi.mocked(listInactiveBlockRecurrences).mockReset();
    vi.mocked(listScheduledBlockRecurrences).mockReset();
    vi.mocked(listUpcomingBlocks).mockReset().mockResolvedValue([]);
  });

  afterEach(() => {
    cleanup();
  });

  it("sorts timed blocks first with untimed last and stable ties", () => {
    render(
      <Harness
        initial={[
          makeBlock({
            id: "b-late",
            label: "NineLate",
            planned_start_at: "09:00",
            created_at: "2026-01-02T00:00:00Z",
          }),
          makeBlock({
            id: "b-untimed-2",
            label: "UntimedLater",
            planned_start_at: null,
            created_at: "2026-01-03T00:00:00Z",
          }),
          makeBlock({
            id: "b-early",
            label: "SevenFifteen",
            planned_start_at: "07:15",
            created_at: "2026-01-01T00:00:00Z",
          }),
          makeBlock({
            id: "b-tie-a",
            label: "NineTieA",
            planned_start_at: "09:00",
            created_at: "2026-01-01T00:00:00Z",
          }),
          makeBlock({
            id: "b-tie-b",
            label: "NineTieB",
            planned_start_at: "09:00",
            created_at: "2026-01-01T00:00:00Z",
          }),
          makeBlock({
            id: "b-untimed-1",
            label: "UntimedFirst",
            planned_start_at: null,
            created_at: "2026-01-01T00:00:00Z",
          }),
        ]}
      />,
    );
    const texts = cardTexts();
    expect(texts).toHaveLength(6);
    expect(texts[0]).toContain("SevenFifteen");
    expect(texts[1]).toContain("NineTieA");
    expect(texts[2]).toContain("NineTieB");
    expect(texts[3]).toContain("NineLate");
    expect(texts[4]).toContain("UntimedFirst");
    expect(texts[5]).toContain("UntimedLater");
  });

  it("renders a created block immediately, then commits the response", async () => {
    let resolveCreate: ((block: Block) => void) | undefined;
    vi.mocked(createBlock).mockImplementation(
      () =>
        new Promise<Block>((resolve) => {
          resolveCreate = resolve;
        }),
    );
    render(<Harness initial={[]} />);

    fireEvent.click(screen.getByRole("button", { name: "+ Add block" }));
    fireEvent.change(screen.getByLabelText("Label"), { target: { value: "Draft deep work" } });
    fireEvent.click(screen.getByRole("button", { name: "Add block" }));

    // Optimistic: the provisional row is visible before the native command resolves.
    expect(screen.getAllByText("Draft deep work")).toHaveLength(1);

    resolveCreate?.(
      makeBlock({ id: "real-1", label: "Draft deep work", planned_start_at: "08:00" }),
    );
    await waitFor(() => expect(screen.queryByLabelText("Label")).toBeNull());

    // The native response is committed: exactly one row, provisional id gone.
    expect(screen.getAllByText("Draft deep work")).toHaveLength(1);
    expect(screen.getByRole("button", { name: "Start" })).toBeTruthy();
  });

  it("rolls the visible list back and shows error feedback when create rejects", async () => {
    vi.mocked(createBlock).mockRejectedValue(new Error("Database write failed."));
    render(<Harness initial={[]} />);

    fireEvent.click(screen.getByRole("button", { name: "+ Add block" }));
    fireEvent.change(screen.getByLabelText("Label"), { target: { value: "Doomed block" } });
    fireEvent.click(screen.getByRole("button", { name: "Add block" }));

    await waitFor(() =>
      expect(screen.getByRole("alert")).toHaveTextContent("Database write failed."),
    );
    expect(screen.queryByText("Doomed block")).toBeNull();
    // The list restored its prior state and the form stayed open for fixes.
    expect(screen.getByLabelText("Label")).toHaveValue("Doomed block");
  });

  it("shows an edited label immediately and keeps the committed row", async () => {
    let resolveUpdate: ((block: Block) => void) | undefined;
    vi.mocked(updateBlock).mockImplementation(
      () =>
        new Promise<Block>((resolve) => {
          resolveUpdate = resolve;
        }),
    );
    render(<Harness initial={[makeBlock({ id: "b1", label: "Morning writing" })]} />);

    fireEvent.click(screen.getByRole("button", { name: "Edit" }));
    fireEvent.change(screen.getByLabelText("Label"), { target: { value: "Renamed writing" } });
    fireEvent.click(screen.getByRole("button", { name: "Save changes" }));

    // Optimistic render before the response.
    expect(screen.getAllByText("Renamed writing")).toHaveLength(1);

    resolveUpdate?.(makeBlock({ id: "b1", label: "Renamed writing" }));
    await waitFor(() => expect(screen.queryByLabelText("Label")).toBeNull());
    expect(screen.getAllByText("Renamed writing")).toHaveLength(1);
  });

  it("rolls the visible list back when the edit rejects", async () => {
    vi.mocked(updateBlock).mockRejectedValue(new Error("Database write failed."));
    render(<Harness initial={[makeBlock({ id: "b1", label: "Morning writing" })]} />);

    fireEvent.click(screen.getByRole("button", { name: "Edit" }));
    fireEvent.change(screen.getByLabelText("Label"), { target: { value: "Broken rename" } });
    fireEvent.click(screen.getByRole("button", { name: "Save changes" }));

    await waitFor(() =>
      expect(screen.getByRole("alert")).toHaveTextContent("Database write failed."),
    );
    expect(screen.queryByText("Broken rename")).toBeNull();
    expect(screen.getByText("Morning writing")).toBeTruthy();
  });

  it("re-reads the day after a recurrence save to commit materialized occurrences", async () => {
    vi.mocked(getBlockRecurrence).mockResolvedValue(makeRule());
    vi.mocked(updateBlockRecurrence).mockResolvedValue(makeRule({ version_id: "rv2" }));
    vi.mocked(listBlocksForDate).mockResolvedValue([
      makeBlock({ id: "r-occ", label: "Walk fresh", recurrence_id: "r1" }),
    ]);
    render(
      <Harness
        initial={[
          makeBlock({
            id: "r-occ",
            label: "Walk",
            recurrence_id: "r1",
            planned_start_at: "07:00",
          }),
        ]}
      />,
    );

    expect(screen.getByText("Weekly")).toBeTruthy();
    fireEvent.click(screen.getByRole("button", { name: "Edit" }));
    await screen.findByText(/You are editing the weekly rule/);
    fireEvent.click(screen.getByRole("button", { name: "Save changes" }));

    await waitFor(() => expect(screen.getByText("Walk fresh")).toBeTruthy());
    expect(updateBlock).not.toHaveBeenCalled();
    expect(listBlocksForDate).toHaveBeenCalledWith("w1", "2026-10-19");
  });

  it("restores the original occurrence when the post-save recurrence refresh fails", async () => {
    vi.mocked(getBlockRecurrence).mockResolvedValue(makeRule());
    vi.mocked(updateBlockRecurrence).mockResolvedValue(makeRule({ version_id: "rv2" }));
    vi.mocked(listBlocksForDate).mockRejectedValue(new Error("Refresh failed."));
    render(
      <Harness
        initial={[
          makeBlock({
            id: "r-occ",
            label: "Walk",
            recurrence_id: "r1",
            planned_start_at: "07:00",
          }),
        ]}
      />,
    );

    fireEvent.click(screen.getByRole("button", { name: "Edit" }));
    await screen.findByText(/You are editing the weekly rule/);
    fireEvent.click(screen.getByRole("button", { name: "Save changes" }));

    await waitFor(() => expect(screen.getByRole("alert")).toHaveTextContent("Refresh failed."));
    // The write persisted and the notice says so, but the optimistic rule
    // values are not the authoritative occurrence: it reverts to the
    // committed original instead of lingering as a mismatched target.
    expect(screen.getByRole("alert")).toHaveTextContent("The recurrence was saved");
    expect(screen.queryByText("Morning walk")).toBeNull();
    expect(screen.getByText("Walk")).toBeTruthy();
    expect(screen.queryByLabelText("Label")).toBeNull();
    expect(updateBlock).not.toHaveBeenCalled();
  });

  it("drops the provisional row when the post-save recurrence create refresh fails", async () => {
    vi.mocked(createBlockRecurrence).mockResolvedValue(makeRule());
    vi.mocked(listBlocksForDate).mockRejectedValue(new Error("Refresh failed."));
    render(<Harness initial={[makeBlock({ id: "b-one", label: "Unrelated one-off" })]} />);

    fireEvent.click(screen.getByRole("button", { name: "+ Add block" }));
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

    // The provisional weekly row appears while the save/refresh cycle runs.
    expect(screen.getByText("Team sync")).toBeTruthy();

    await waitFor(() => expect(screen.getByRole("alert")).toHaveTextContent("Refresh failed."));
    // The rule persisted, but the provisional row carries a temp id and no
    // recurrence identity, so it is removed instead of exposing actions
    // against a non-persisted block; the unrelated committed row survives.
    expect(screen.queryByText("Team sync")).toBeNull();
    expect(screen.getByText("Unrelated one-off")).toBeTruthy();
    expect(screen.getByRole("alert")).toHaveTextContent("The recurrence was saved");
    expect(screen.queryByLabelText("Label")).toBeNull();
    expect(createBlockRecurrence).toHaveBeenCalledOnce();
    expect(createBlock).not.toHaveBeenCalled();
    expect(listBlocksForDate).toHaveBeenCalledWith("w1", "2026-10-19");
  });

  it("blocks concurrent card actions while a save is in flight and rolls back only the edited row", async () => {
    let rejectUpdate: ((err: Error) => void) | undefined;
    vi.mocked(updateBlock).mockImplementation(
      () =>
        new Promise<Block>((_resolve, reject) => {
          rejectUpdate = reject;
        }),
    );
    const b2 = makeBlock({ id: "b2", label: "Standup notes", planned_start_at: "10:00" });
    vi.mocked(startBlock).mockResolvedValue({ ...b2, status: "active" });
    render(<Harness initial={[makeBlock({ id: "b1", label: "Morning writing" }), b2]} />);

    fireEvent.click(
      within(screen.getAllByRole("article")[0]).getByRole("button", { name: "Edit" }),
    );
    fireEvent.change(screen.getByLabelText("Label"), { target: { value: "Renamed writing" } });
    fireEvent.click(screen.getByRole("button", { name: "Save changes" }));

    // The optimistic row renders immediately; other cards cannot fire a
    // competing status write while this save is pending.
    expect(screen.getByText("Renamed writing")).toBeTruthy();
    const otherStart = within(screen.getAllByRole("article")[1]).getByRole("button", {
      name: "Start",
    });
    expect(otherStart).toBeDisabled();
    fireEvent.click(otherStart);
    expect(startBlock).not.toHaveBeenCalled();

    rejectUpdate?.(new Error("Database write failed."));
    await waitFor(() =>
      expect(screen.getByRole("alert")).toHaveTextContent("Database write failed."),
    );
    // Scoped rollback: only the edited row reverts; the untouched row and
    // its now-re-enabled actions are unaffected.
    expect(screen.queryByText("Renamed writing")).toBeNull();
    expect(screen.getByText("Morning writing")).toBeTruthy();
    expect(screen.getByText("Standup notes")).toBeTruthy();
    expect(
      within(screen.getAllByRole("article")[1]).getByRole("button", { name: "Start" }),
    ).toBeEnabled();
  });

  it("hides the edit action on terminal blocks", () => {
    render(
      <Harness initial={[makeBlock({ id: "done", label: "Done block", status: "completed" })]} />,
    );
    expect(screen.queryByRole("button", { name: "Edit" })).toBeNull();
  });

  it("keeps the scheduled and inactive sections collapsed by default", () => {
    render(<Harness initial={[makeBlock({ id: "b1", label: "Morning writing" })]} />);
    expect(screen.getByRole("button", { name: "Scheduled blocks" })).toHaveAttribute(
      "aria-expanded",
      "false",
    );
    expect(screen.getByRole("button", { name: "Inactive blocks" })).toHaveAttribute(
      "aria-expanded",
      "false",
    );
    // Nothing is fetched before the user opens a section.
    expect(listScheduledBlockRecurrences).not.toHaveBeenCalled();
    expect(listInactiveBlocks).not.toHaveBeenCalled();
    expect(listInactiveBlockRecurrences).not.toHaveBeenCalled();
  });

  it("shows one card per scheduled routine without materializing future occurrences", async () => {
    vi.mocked(listScheduledBlockRecurrences).mockResolvedValue([
      makeScheduledRule(),
      makeScheduledRule({
        recurrence: makeRule({ id: "r2", label: "Evening reading", weekdays: [4] }),
        next_occurrence_date: "2026-10-22",
      }),
    ]);
    render(<Harness initial={[]} />);

    fireEvent.click(screen.getByRole("button", { name: "Scheduled blocks" }));
    await waitFor(() => expect(listScheduledBlockRecurrences).toHaveBeenCalledOnce());
    expect(listScheduledBlockRecurrences).toHaveBeenCalledWith("w1");
    expect(listUpcomingBlocks).toHaveBeenCalledWith("w1", "2026-10-20", "2027-01-17", 50, 0);
    expect(screen.getByText("Morning walk")).toBeTruthy();
    expect(screen.getByText("Evening reading")).toBeTruthy();
    expect(screen.getAllByText(/Next occurrence/)).toHaveLength(2);
    expect(screen.queryByRole("button", { name: "Start" })).toBeNull();
  });

  it("keeps future one-time blocks visible alongside recurrence definitions", async () => {
    vi.mocked(listScheduledBlockRecurrences).mockResolvedValue([]);
    vi.mocked(listUpcomingBlocks).mockResolvedValue([
      makeBlock({ id: "one-time", label: "One-time appointment", date: "2026-10-20" }),
    ]);
    render(<Harness initial={[]} />);

    fireEvent.click(screen.getByRole("button", { name: "Scheduled blocks" }));
    await screen.findByText("One-time appointment");

    expect(listUpcomingBlocks).toHaveBeenCalledWith("w1", "2026-10-20", "2027-01-17", 50, 0);
    expect(screen.getByRole("button", { name: "Start" })).toBeTruthy();
  });

  it("lists inactive one-time blocks and inactive series with their actions", async () => {
    vi.mocked(listInactiveBlocks).mockResolvedValue([
      makeBlock({
        id: "old-1",
        label: "Old focus",
        status: "completed",
        lifecycle_state: "inactive",
      }),
    ]);
    vi.mocked(listInactiveBlockRecurrences).mockResolvedValue([
      makeRule({ id: "r-old", label: "Old walk", lifecycle_state: "inactive" }),
    ]);
    render(<Harness initial={[]} />);

    fireEvent.click(screen.getByRole("button", { name: "Inactive blocks" }));
    await waitFor(() => expect(screen.getByText("Old focus")).toBeTruthy());
    expect(screen.getByText("Old walk")).toBeTruthy();
    // The series shows its exact start time, its duration, and its weekdays.
    expect(screen.getByText(/07:00/)).toBeTruthy();
    expect(screen.getByText(/30 min/)).toBeTruthy();
    expect(screen.getByText("Monday")).toBeTruthy();
    expect(screen.getAllByRole("button", { name: "Reactivate" })).toHaveLength(2);
    expect(screen.getAllByRole("button", { name: "Delete" })).toHaveLength(2);
  });

  it("confirms before deleting an inactive block and drops the deleted row", async () => {
    const confirmSpy = vi.spyOn(window, "confirm").mockReturnValue(false);
    vi.mocked(listInactiveBlocks).mockResolvedValue([
      makeBlock({
        id: "old-1",
        label: "Old focus",
        status: "completed",
        lifecycle_state: "inactive",
      }),
    ]);
    vi.mocked(listInactiveBlockRecurrences).mockResolvedValue([]);
    vi.mocked(deleteBlock).mockResolvedValue(
      makeBlock({ id: "old-1", label: "Old focus", lifecycle_state: "deleted" }),
    );
    render(<Harness initial={[]} />);

    fireEvent.click(screen.getByRole("button", { name: "Inactive blocks" }));
    await waitFor(() => expect(screen.getByText("Old focus")).toBeTruthy());

    // A declined confirmation performs no write at all.
    fireEvent.click(screen.getByRole("button", { name: "Delete" }));
    expect(deleteBlock).not.toHaveBeenCalled();
    expect(screen.getByText("Old focus")).toBeTruthy();

    confirmSpy.mockReturnValue(true);
    fireEvent.click(screen.getByRole("button", { name: "Delete" }));
    await waitFor(() => expect(deleteBlock).toHaveBeenCalledWith("w1", "old-1"));
    await waitFor(() => expect(screen.queryByText("Old focus")).toBeNull());
    confirmSpy.mockRestore();
  });

  it("removes a deactivated or deleted planned block from the day list", async () => {
    const confirmSpy = vi.spyOn(window, "confirm").mockReturnValue(true);
    vi.mocked(deactivateBlock).mockResolvedValue(
      makeBlock({ id: "b1", label: "Morning writing", lifecycle_state: "inactive" }),
    );
    vi.mocked(deleteBlock).mockResolvedValue(
      makeBlock({ id: "b2", label: "Second block", lifecycle_state: "deleted" }),
    );
    render(
      <Harness
        initial={[
          makeBlock({ id: "b1", label: "Morning writing" }),
          makeBlock({ id: "b2", label: "Second block", planned_start_at: "10:00" }),
        ]}
      />,
    );

    fireEvent.click(
      within(screen.getAllByRole("article")[0]).getByRole("button", { name: "Deactivate" }),
    );
    await waitFor(() => expect(screen.queryByText("Morning writing")).toBeNull());
    expect(deactivateBlock).toHaveBeenCalledWith("w1", "b1");

    fireEvent.click(
      within(screen.getAllByRole("article")[0]).getByRole("button", { name: "Delete" }),
    );
    await waitFor(() => expect(screen.queryByText("Second block")).toBeNull());
    expect(deleteBlock).toHaveBeenCalledWith("w1", "b2");
    confirmSpy.mockRestore();
  });

  it("removes a scheduled routine card when its series is deleted", async () => {
    const confirmSpy = vi.spyOn(window, "confirm").mockReturnValue(true);
    vi.mocked(listScheduledBlockRecurrences)
      .mockResolvedValueOnce([makeScheduledRule()])
      .mockResolvedValueOnce([]);
    vi.mocked(deleteBlockRecurrence).mockResolvedValue(makeRule({ lifecycle_state: "deleted" }));
    render(<Harness initial={[]} />);

    fireEvent.click(screen.getByRole("button", { name: "Scheduled blocks" }));
    await screen.findByText("Morning walk");
    fireEvent.click(screen.getByRole("button", { name: "Delete routine" }));

    await waitFor(() => expect(deleteBlockRecurrence).toHaveBeenCalledWith("w1", "r1"));
    expect(screen.queryByText("Morning walk")).toBeNull();
    expect(listUpcomingBlocks).toHaveBeenCalledWith("w1", "2026-10-20", "2027-01-17", 50, 0);
    confirmSpy.mockRestore();
  });

  it("re-reads the scheduled series after a weekly rule is saved", async () => {
    vi.mocked(listScheduledBlockRecurrences).mockResolvedValue([makeScheduledRule()]);
    vi.mocked(getBlockRecurrence).mockResolvedValue(makeRule());
    vi.mocked(updateBlockRecurrence).mockResolvedValue(makeRule({ version_id: "rv2" }));
    vi.mocked(listBlocksForDate).mockResolvedValue([]);
    render(<Harness initial={[]} />);

    fireEvent.click(screen.getByRole("button", { name: "Scheduled blocks" }));
    await waitFor(() => expect(listScheduledBlockRecurrences).toHaveBeenCalledOnce());

    fireEvent.click(screen.getByRole("button", { name: "Edit" }));
    await screen.findByText(/You are editing the weekly rule/);
    fireEvent.click(screen.getByRole("button", { name: "Save changes" }));

    await waitFor(() => expect(listScheduledBlockRecurrences).toHaveBeenCalledTimes(2));
    expect(updateBlockRecurrence).toHaveBeenCalledWith(
      "r1",
      expect.objectContaining({ workspace_id: "w1" }),
    );
  });

  it("reactivates an inactive block through the scheduling form", async () => {
    vi.mocked(listInactiveBlocks).mockResolvedValue([
      makeBlock({
        id: "old-1",
        label: "Old focus",
        status: "completed",
        lifecycle_state: "inactive",
      }),
    ]);
    vi.mocked(listInactiveBlockRecurrences).mockResolvedValue([]);
    vi.mocked(reactivateBlock).mockResolvedValue({
      block: makeBlock({ id: "new-1", label: "Old focus" }),
      recurrence: null,
    });
    render(<Harness initial={[]} />);

    fireEvent.click(screen.getByRole("button", { name: "Inactive blocks" }));
    await waitFor(() => expect(screen.getByText("Old focus")).toBeTruthy());

    fireEvent.click(screen.getByRole("button", { name: "Reactivate" }));
    expect(screen.getByRole("heading", { name: "Reactivate block" })).toBeTruthy();

    fireEvent.click(screen.getByRole("button", { name: "Reactivate block" }));
    await waitFor(() => expect(reactivateBlock).toHaveBeenCalledOnce());
    expect(reactivateBlock).toHaveBeenCalledWith({
      workspace_id: "w1",
      block_id: "old-1",
      schedule: "once",
      weekday: expect.any(Number),
    });
    // The form closes and the success notice is shown.
    await waitFor(() =>
      expect(screen.queryByRole("heading", { name: "Reactivate block" })).toBeNull(),
    );
    expect(screen.getByText("The block was reactivated.")).toBeTruthy();
  });

  it("reactivates an inactive series and points at the rule editor", async () => {
    vi.mocked(listInactiveBlocks).mockResolvedValue([]);
    vi.mocked(listInactiveBlockRecurrences).mockResolvedValue([
      makeRule({ id: "r-old", label: "Old walk", lifecycle_state: "inactive" }),
    ]);
    vi.mocked(reactivateBlockRecurrence).mockResolvedValue(
      makeRule({ id: "r-old", label: "Old walk", lifecycle_state: "active" }),
    );
    render(<Harness initial={[]} />);

    fireEvent.click(screen.getByRole("button", { name: "Inactive blocks" }));
    await waitFor(() => expect(screen.getByText("Old walk")).toBeTruthy());

    fireEvent.click(screen.getByRole("button", { name: "Reactivate" }));
    await waitFor(() => expect(reactivateBlockRecurrence).toHaveBeenCalledWith("w1", "r-old"));
    expect(await screen.findByText(/This routine is active again/)).toBeTruthy();
  });
});
