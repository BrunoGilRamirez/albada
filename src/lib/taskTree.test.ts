import { describe, expect, it } from "vitest";
import { ancestorChain, attachCandidates, buildTaskForest, childTaskIds } from "@/lib/taskTree";
import type { Task } from "@/lib/types";

function makeTask(id: string, parent: string | null, project: string | null = null): Task {
  return {
    id,
    workspace_id: "w1",
    project_id: project,
    goal_id: null,
    parent_task_id: parent,
    title: `Task ${id}`,
    description_markdown: null,
    notes_markdown: null,
    blocking_note_markdown: null,
    status: "planned",
    priority: "normal",
    effort_size: null,
    effort_points: 1,
    target_date: null,
    scheduled_date: null,
    completed_at: null,
    cancelled_at: null,
    closure_mode: null,
    closure_reason_markdown: null,
    created_at: "2026-01-01T00:00:00Z",
    updated_at: "2026-01-01T00:00:00Z",
    archived_at: null,
  };
}

const tasks: Task[] = [
  makeTask("a", null, "p1"),
  makeTask("b", "a"),
  makeTask("c", "b"),
  makeTask("leaf", null),
];

describe("taskTree helpers", () => {
  it("buildTaskForest groups children under roots", () => {
    const forest = buildTaskForest(tasks);
    expect(forest).toHaveLength(2);
    expect(forest[0].children[0].children[0].task.id).toBe("c");
  });

  it("childTaskIds includes only tasks that have children", () => {
    const ids = childTaskIds(tasks);
    expect(ids.has("a")).toBe(true);
    expect(ids.has("b")).toBe(true);
    expect(ids.has("c")).toBe(false);
    expect(ids.has("leaf")).toBe(false);
  });

  it("attachCandidates only exposes childless root tasks", () => {
    const candidates = attachCandidates(tasks, (task) => task.project_id === "p1");
    expect(candidates.map((task) => task.id)).toEqual(["leaf"]);
  });

  it("ancestorChain returns top-down ancestors", () => {
    const chain = ancestorChain(tasks, "c");
    expect(chain.map((task) => task.id)).toEqual(["a", "b"]);
    expect(ancestorChain(tasks, "leaf")).toEqual([]);
  });
});
