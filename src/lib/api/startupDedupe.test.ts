import { beforeEach, describe, expect, it, vi } from "vitest";

const { invokeMock, isTauriMock } = vi.hoisted(() => ({
  invokeMock: vi.fn(),
  isTauriMock: vi.fn(),
}));

vi.mock("@tauri-apps/api/core", () => ({
  invoke: invokeMock,
  isTauri: isTauriMock,
}));

import { getOrCreateDailyPlan, listDayTasks } from "@/lib/api/dailyPlanApi";
import { listAtRiskTasks } from "@/lib/api/deadlineApi";
import { listBlocksForDate } from "@/lib/api/blockApi";
import { listFixedCommitments } from "@/lib/api/fixedCommitmentApi";
import { listTasks } from "@/lib/api/taskApi";
import { ensureDefaultWorkspace } from "@/lib/api/workspaceApi";
import type { DailyPlan, DeadlineTaskInfo, Workspace } from "@/lib/types";

const workspace = { id: "w1", name: "Life" } as unknown as Workspace;
const plan = { id: "p1", status: "planned" } as unknown as DailyPlan;
const atRisk = [] as unknown as DeadlineTaskInfo[];

/**
 * Startup fan-out evidence: under StrictMode the Today effect runs twice and
 * NotificationBell mounts twice, so `ensureDefaultWorkspace` was called four
 * times and each of Today's follow-up commands twice. The typed boundary must
 * collapse each concurrent group into exactly one native command while
 * sequential refreshes keep their original semantics.
 *
 * `isTauri` is false here so the diagnostics channel (first-invoke marking,
 * error reporting) stays silent and the native call counts below reflect
 * business commands only; the diagnostics paths have their own tests in
 * `src/lib/tauri.test.ts`.
 */
describe("startup command deduplication at the typed boundary", () => {
  beforeEach(() => {
    invokeMock.mockReset();
    isTauriMock.mockReset();
    isTauriMock.mockReturnValue(false);
  });

  it("shares one ensure_default_workspace native call across four concurrent mounts", async () => {
    invokeMock.mockResolvedValue(workspace);

    const results = await Promise.all([
      ensureDefaultWorkspace(),
      ensureDefaultWorkspace(),
      ensureDefaultWorkspace(),
      ensureDefaultWorkspace(),
    ]);

    expect(invokeMock).toHaveBeenCalledTimes(1);
    expect(invokeMock).toHaveBeenCalledWith("ensure_default_workspace", undefined);
    results.forEach((result) => expect(result).toBe(workspace));
  });

  it("re-runs ensure_default_workspace for a later sequential call", async () => {
    invokeMock.mockResolvedValue(workspace);

    await ensureDefaultWorkspace();
    await ensureDefaultWorkspace();

    expect(invokeMock).toHaveBeenCalledTimes(2);
  });

  it("dedupes get_or_create_daily_plan per date and keeps distinct dates separate", async () => {
    invokeMock.mockResolvedValue(plan);

    await Promise.all([
      getOrCreateDailyPlan("2026-10-06"),
      getOrCreateDailyPlan("2026-10-06"),
      getOrCreateDailyPlan("2026-10-07"),
    ]);

    expect(invokeMock).toHaveBeenCalledTimes(2);
    expect(invokeMock).toHaveBeenCalledWith("get_or_create_daily_plan", { date: "2026-10-06" });
    expect(invokeMock).toHaveBeenCalledWith("get_or_create_daily_plan", { date: "2026-10-07" });
  });

  it("dedupes list_at_risk_tasks per workspace", async () => {
    invokeMock.mockResolvedValue(atRisk);

    await Promise.all([listAtRiskTasks("w1"), listAtRiskTasks("w1")]);

    expect(invokeMock).toHaveBeenCalledTimes(1);
    expect(invokeMock).toHaveBeenCalledWith("list_at_risk_tasks", { workspaceId: "w1" });
  });

  it("dedupes list_tasks per workspace and keeps distinct workspaces separate", async () => {
    invokeMock.mockResolvedValue([]);

    await Promise.all([listTasks("w1"), listTasks("w1"), listTasks("w2")]);

    expect(invokeMock).toHaveBeenCalledTimes(2);
    expect(invokeMock).toHaveBeenCalledWith("list_tasks", { workspaceId: "w1" });
    expect(invokeMock).toHaveBeenCalledWith("list_tasks", { workspaceId: "w2" });
  });

  it("dedupes list_blocks_for_date per workspace and date", async () => {
    invokeMock.mockResolvedValue([]);

    await Promise.all([
      listBlocksForDate("w1", "2026-10-06"),
      listBlocksForDate("w1", "2026-10-06"),
      listBlocksForDate("w1", "2026-10-07"),
    ]);

    expect(invokeMock).toHaveBeenCalledTimes(2);
  });

  it("dedupes list_fixed_commitments per workspace and flag without colliding flags", async () => {
    invokeMock.mockResolvedValue([]);

    await Promise.all([
      listFixedCommitments("w1", true),
      listFixedCommitments("w1", true),
      listFixedCommitments("w1", false),
    ]);

    expect(invokeMock).toHaveBeenCalledTimes(2);
    expect(invokeMock).toHaveBeenCalledWith("list_fixed_commitments", {
      workspaceId: "w1",
      activeOnly: true,
    });
    expect(invokeMock).toHaveBeenCalledWith("list_fixed_commitments", {
      workspaceId: "w1",
      activeOnly: false,
    });
  });

  it("dedupes list_day_tasks per plan and re-runs for sequential refreshes", async () => {
    invokeMock.mockResolvedValue([]);

    await Promise.all([listDayTasks("p1"), listDayTasks("p1")]);
    expect(invokeMock).toHaveBeenCalledTimes(1);

    await listDayTasks("p1");
    expect(invokeMock).toHaveBeenCalledTimes(2);
  });

  it("clears a failed in-flight entry so the next caller retries the command", async () => {
    invokeMock.mockRejectedValueOnce(new Error("db locked")).mockResolvedValueOnce(workspace);

    await expect(ensureDefaultWorkspace()).rejects.toThrow();
    await expect(ensureDefaultWorkspace()).resolves.toBe(workspace);

    expect(invokeMock).toHaveBeenCalledTimes(2);
  });

  it("shares the same rejection across concurrent callers of one key", async () => {
    const failure = new Error("db locked");
    invokeMock.mockRejectedValue(failure);

    const results = await Promise.allSettled([ensureDefaultWorkspace(), ensureDefaultWorkspace()]);

    expect(invokeMock).toHaveBeenCalledTimes(1);
    results.forEach((result) => {
      expect(result.status).toBe("rejected");
    });
  });

  it("collapses Today's StrictMode double-load into one native call per command", async () => {
    const commandResults: Record<string, unknown> = {
      ensure_default_workspace: workspace,
      get_or_create_daily_plan: plan,
      list_tasks: [],
      list_blocks_for_date: [],
      list_at_risk_tasks: atRisk,
      list_fixed_commitments: [],
      list_day_tasks: [],
    };
    invokeMock.mockImplementation((command: string) => Promise.resolve(commandResults[command]));

    // Mirrors Today's load(): ensure, then the five-way Promise.all, then the
    // plan rows. Two concurrent runs = the StrictMode mount/cleanup/remount.
    const todayLoad = async (date: string): Promise<void> => {
      const ws = await ensureDefaultWorkspace();
      const [dailyPlan] = await Promise.all([
        getOrCreateDailyPlan(date),
        listTasks(ws.id),
        listBlocksForDate(ws.id, date),
        listAtRiskTasks(ws.id),
        listFixedCommitments(ws.id, true),
      ]);
      await listDayTasks(dailyPlan.id);
    };

    await Promise.all([todayLoad("2026-10-06"), todayLoad("2026-10-06")]);

    const commands = invokeMock.mock.calls.map((call) => call[0] as string);
    expect(commands.sort()).toEqual([
      "ensure_default_workspace",
      "get_or_create_daily_plan",
      "list_at_risk_tasks",
      "list_blocks_for_date",
      "list_day_tasks",
      "list_fixed_commitments",
      "list_tasks",
    ]);
  });
});
