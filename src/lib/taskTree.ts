import type { Task, TaskTree } from "./types";

/**
 * Reconstruct task trees from a flat task list (children grouped under their
 * parent). Dependency blockers are not populated here; the task detail view
 * provides them.
 */
export function buildTaskForest(tasks: Task[]): TaskTree[] {
  const byParent = new Map<string | null, Task[]>();
  for (const task of tasks) {
    const bucket = byParent.get(task.parent_task_id) ?? [];
    bucket.push(task);
    byParent.set(task.parent_task_id, bucket);
  }
  const build = (task: Task): TaskTree => ({
    task,
    children: (byParent.get(task.id) ?? []).map(build),
    blocked_by: [],
  });
  return (byParent.get(null) ?? []).map(build);
}

/** Ids of every task that has at least one child. */
export function childTaskIds(tasks: Task[]): Set<string> {
  return new Set(
    tasks.map((task) => task.parent_task_id).filter((id): id is string => Boolean(id)),
  );
}

/**
 * Candidates for attaching an existing task to a project/goal: non-archived
 * leaf root tasks (no parent, no children) that pass the `exclude` predicate
 * (e.g. already in the target scope). Only leaves can be attached because the
 * domain does not move whole subtrees.
 */
export function attachCandidates(tasks: Task[], exclude: (task: Task) => boolean): Task[] {
  const withChildren = childTaskIds(tasks);
  return tasks.filter(
    (task) =>
      task.archived_at === null &&
      task.parent_task_id === null &&
      !withChildren.has(task.id) &&
      !exclude(task),
  );
}

/**
 * Ancestors of a task, top-down (closest to the root first). The task itself
 * is not included. Returns an empty array when the task is a root.
 */
export function ancestorChain(tasks: Task[], taskId: string): Task[] {
  const byId = new Map(tasks.map((task) => [task.id, task]));
  const chain: Task[] = [];
  const seen = new Set<string>();
  let current = byId.get(taskId);
  while (current?.parent_task_id && !seen.has(current.id)) {
    seen.add(current.id);
    const parent = byId.get(current.parent_task_id);
    if (!parent) break;
    chain.unshift(parent);
    current = parent;
  }
  return chain;
}
