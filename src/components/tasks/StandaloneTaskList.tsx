import { useCallback, useEffect, useMemo, useState } from "react";
import { Link } from "react-router";
import { useTranslation } from "react-i18next";
import layoutColumnsIcon from "@/assets/layout-icons/layout-columns.svg";
import layoutListIcon from "@/assets/layout-icons/layout-list.svg";
import layoutTilesIcon from "@/assets/layout-icons/layout-tiles.svg";
import type { DeadlineTaskInfo, Goal, Project, Task } from "../../lib/types";
import { formatDate } from "../../lib/format";
import DeadlineStatusBadge from "../deadlines/DeadlineStatusBadge";
import TaskStatusBadge from "./TaskStatusBadge";
import TaskPriorityBadge from "./TaskPriorityBadge";
import TaskEffortBadge from "./TaskEffortBadge";

interface StandaloneTaskListProps {
  tasks: Task[];
  projects?: Project[];
  goals?: Goal[];
  deadlines?: Record<string, DeadlineTaskInfo>;
}

type TaskListLayout = "list" | "columns" | "tiles";
type TaskSort = "created" | "proximity" | "importance" | "difficulty";
type TaskFilter = "all" | "open" | "completed" | "overdue";
type TaskGrouping = "none" | "project" | "goal" | "nested";
type SortDirection = "asc" | "desc";

const LAYOUT_CLASS: Record<TaskListLayout, string> = {
  list: "grid grid-cols-1 gap-3",
  columns: "grid grid-cols-1 gap-3 md:grid-cols-2",
  tiles: "grid gap-3 [grid-template-columns:repeat(auto-fill,minmax(14rem,1fr))]",
};
const LAYOUT_ORDER: readonly TaskListLayout[] = ["list", "columns", "tiles"];
const LAYOUT_LABEL_KEY = {
  list: "tasks.layoutList",
  columns: "tasks.layoutColumns",
  tiles: "tasks.layoutTiles",
} as const;
const LAYOUT_ICON = { list: layoutListIcon, columns: layoutColumnsIcon, tiles: layoutTilesIcon };
const LAYOUT_STORAGE_KEY = "joi-projectlife-task-list-layout";
const PRIORITY_ORDER = { none: 0, low: 1, normal: 2, high: 3, critical: 4 } as const;

function isTaskListLayout(value: string | null): value is TaskListLayout {
  return value !== null && (LAYOUT_ORDER as readonly string[]).includes(value);
}

function readStoredLayout(): TaskListLayout {
  try {
    const stored = localStorage.getItem(LAYOUT_STORAGE_KEY);
    if (isTaskListLayout(stored)) return stored;
  } catch {
    // Storage is optional; the list still works for the current session.
  }
  return "list";
}

function formatApplicationDate(targetDate: string): string {
  return formatDate(new Date(`${targetDate}T12:00:00`), { dateStyle: "medium" });
}

function compareTasks(a: Task, b: Task, sort: TaskSort, direction: SortDirection): number {
  let result = 0;
  if (sort === "proximity") {
    if (a.target_date && b.target_date) {
      const order = a.target_date.localeCompare(b.target_date);
      if (order !== 0) result = order;
    } else if (a.target_date) result = -1;
    else if (b.target_date) result = 1;
  }
  if (result === 0 && sort === "importance") {
    const order = PRIORITY_ORDER[b.priority] - PRIORITY_ORDER[a.priority];
    if (order !== 0) result = order;
  }
  if (result === 0 && sort === "difficulty") {
    const order = b.effort_points - a.effort_points;
    if (order !== 0) result = order;
  }
  if (result === 0) result = a.created_at.localeCompare(b.created_at);
  return direction === "asc" ? result : -result;
}

function StandaloneTaskList({
  tasks,
  projects = [],
  goals = [],
  deadlines,
}: StandaloneTaskListProps) {
  const { t } = useTranslation();
  const [layout, setLayout] = useState<TaskListLayout>(readStoredLayout);
  const [sort, setSort] = useState<TaskSort>("created");
  const [sortDirection, setSortDirection] = useState<SortDirection>("desc");
  const [filter, setFilter] = useState<TaskFilter>("all");
  const [grouping, setGrouping] = useState<TaskGrouping>("none");
  const [collapsed, setCollapsed] = useState<Set<string>>(new Set());

  useEffect(() => {
    try {
      localStorage.setItem(LAYOUT_STORAGE_KEY, layout);
    } catch {
      // Storage is optional.
    }
  }, [layout]);

  const projectById = useMemo(
    () => new Map(projects.map((project) => [project.id, project])),
    [projects],
  );
  const goalById = useMemo(() => new Map(goals.map((goal) => [goal.id, goal])), [goals]);
  const scopeFor = useCallback(
    (task: Task) => {
      const project = task.project_id ? projectById.get(task.project_id) : undefined;
      const goal = task.goal_id
        ? goalById.get(task.goal_id)
        : project?.goal_id
          ? goalById.get(project.goal_id)
          : undefined;
      return { project, goal };
    },
    [goalById, projectById],
  );

  const visibleTasks = useMemo(() => {
    const filtered = tasks.filter((task) => {
      if (filter === "open") return task.status !== "completed" && task.status !== "cancelled";
      if (filter === "completed") return task.status === "completed";
      if (filter === "overdue") return deadlines?.[task.id]?.deadline_status === "overdue";
      return true;
    });
    return [...filtered].sort((a, b) => compareTasks(a, b, sort, sortDirection));
  }, [deadlines, filter, sort, sortDirection, tasks]);

  const grouped = useMemo(() => {
    if (grouping === "none" || grouping === "nested") return null;
    const groups = new Map<string, Task[]>();
    for (const task of visibleTasks) {
      const { project, goal } = scopeFor(task);
      const key = grouping === "project" ? (project?.id ?? "none") : (goal?.id ?? "none");
      groups.set(key, [...(groups.get(key) ?? []), task]);
    }
    return [...groups.entries()].map(([key, groupTasks]) => ({
      key: `${grouping}:${key}`,
      label:
        grouping === "project"
          ? (projectById.get(key)?.title ?? t("tasks.ungrouped"))
          : (goalById.get(key)?.title ?? t("tasks.ungrouped")),
      tasks: groupTasks,
    }));
  }, [grouping, goalById, projectById, scopeFor, t, visibleTasks]);

  const nestedGroups = useMemo(() => {
    if (grouping !== "nested") return null;
    const goalsWithProjects = new Map<string, Map<string, Task[]>>();
    for (const task of visibleTasks) {
      const { project, goal } = scopeFor(task);
      const goalKey = goal?.id ?? "none";
      const projectKey = project?.id ?? "none";
      const projectsForGoal = goalsWithProjects.get(goalKey) ?? new Map<string, Task[]>();
      projectsForGoal.set(projectKey, [...(projectsForGoal.get(projectKey) ?? []), task]);
      goalsWithProjects.set(goalKey, projectsForGoal);
    }
    return [...goalsWithProjects.entries()].map(([goalKey, projectsForGoal]) => ({
      key: `goal:${goalKey}`,
      label: goalById.get(goalKey)?.title ?? t("tasks.ungrouped"),
      projects: [...projectsForGoal.entries()].map(([projectKey, groupTasks]) => ({
        key: `project:${projectKey}`,
        label: projectById.get(projectKey)?.title ?? t("tasks.ungrouped"),
        tasks: groupTasks,
      })),
    }));
  }, [goalById, grouping, projectById, scopeFor, t, visibleTasks]);

  const toggleCollapsed = (key: string) => {
    setCollapsed((current) => {
      const next = new Set(current);
      if (next.has(key)) next.delete(key);
      else next.add(key);
      return next;
    });
  };

  const renderTask = (task: Task) => {
    const info = deadlines?.[task.id];
    const { project, goal } = scopeFor(task);
    return (
      <li
        key={task.id}
        className={`joi-surface--base joi-border--default rounded-lg border${layout === "tiles" ? " aspect-square" : ""}`}
        data-notification-target={task.id}
      >
        <div className="block h-full p-3 joi-interactive">
          <div className="flex h-full flex-col gap-2">
            <Link
              to={`/tasks/${task.id}`}
              className={`joi-text text-sm font-medium break-words hover:underline${layout === "tiles" ? " line-clamp-3" : " line-clamp-2"}`}
            >
              {task.title}
            </Link>
            <div
              className={`flex flex-wrap items-center gap-2${layout === "tiles" ? " mt-auto" : ""}`}
            >
              <TaskStatusBadge status={task.status} />
              <TaskPriorityBadge priority={task.priority} />
              {task.effort_size ? <TaskEffortBadge effort={task.effort_size} /> : null}
              {info ? (
                <DeadlineStatusBadge status={info.deadline_status} deadlineAt={info.deadline_at} />
              ) : null}
              {project ? (
                <Link
                  to={`/projects/${project.id}`}
                  className="joi-badge joi-badge--info hover:underline"
                >
                  {project.title}
                </Link>
              ) : null}
              {goal ? (
                <Link
                  to={`/goals/${goal.id}`}
                  className="joi-badge joi-badge--brand hover:underline"
                >
                  {goal.title}
                </Link>
              ) : null}
              {task.target_date ? (
                <span className="joi-text--subtle text-xs">
                  {formatApplicationDate(task.target_date)}
                </span>
              ) : null}
            </div>
          </div>
        </div>
      </li>
    );
  };

  const renderGrid = (groupTasks: Task[]) => (
    <ul className={LAYOUT_CLASS[layout]}>{groupTasks.map(renderTask)}</ul>
  );
  const renderGroup = (key: string, label: string, groupTasks: Task[]) => (
    <section key={key} className="mt-4 first:mt-0">
      <button
        type="button"
        className="joi-text flex w-full items-center gap-2 py-2 text-left text-sm font-semibold"
        aria-expanded={!collapsed.has(key)}
        onClick={() => toggleCollapsed(key)}
      >
        <span aria-hidden="true">{collapsed.has(key) ? "▸" : "▾"}</span>
        <span>{label}</span>
        <span className="joi-text--subtle text-xs">({groupTasks.length})</span>
      </button>
      {!collapsed.has(key) ? renderGrid(groupTasks) : null}
    </section>
  );

  if (tasks.length === 0) return <p className="joi-empty__state">{t("tasks.empty")}</p>;

  return (
    <>
      <div
        className="mb-4 flex flex-wrap items-center gap-2"
        role="group"
        aria-label={t("tasks.controlsLabel")}
      >
        <div className="flex" role="group" aria-label={t("tasks.layoutGroupLabel")}>
          <button
            type="button"
            onClick={() =>
              setLayout(LAYOUT_ORDER[(LAYOUT_ORDER.indexOf(layout) + 1) % LAYOUT_ORDER.length])
            }
            aria-label={t(LAYOUT_LABEL_KEY[layout])}
            title={t(LAYOUT_LABEL_KEY[layout])}
            className="joi-btn joi-btn--secondary joi-btn--sm"
          >
            <img src={LAYOUT_ICON[layout]} alt="" aria-hidden="true" className="joi-layout-icon" />
          </button>
        </div>
        <div className="ml-auto flex flex-wrap items-center gap-2">
          <label className="joi-field__label flex items-center gap-1">
            {t("tasks.sortLabel")}
            <select
              className="joi-field__select h-9 w-auto py-1.5"
              value={sort}
              onChange={(event) => setSort(event.target.value as TaskSort)}
            >
              <option value="created">{t("tasks.sortCreated")}</option>
              <option value="proximity">{t("tasks.sortProximity")}</option>
              <option value="importance">{t("tasks.sortImportance")}</option>
              <option value="difficulty">{t("tasks.sortDifficulty")}</option>
            </select>
            <button
              type="button"
              className="joi-btn joi-btn--secondary h-9 w-9 p-0"
              aria-label={t(
                sortDirection === "asc" ? "tasks.sortDescending" : "tasks.sortAscending",
              )}
              title={t(sortDirection === "asc" ? "tasks.sortDescending" : "tasks.sortAscending")}
              onClick={() =>
                setSortDirection((direction) => (direction === "asc" ? "desc" : "asc"))
              }
            >
              <span aria-hidden="true">{sortDirection === "asc" ? "↑" : "↓"}</span>
            </button>
          </label>
          <label className="joi-field__label flex items-center gap-1">
            {t("tasks.filterLabel")}
            <select
              className="joi-field__select h-9 w-auto py-1.5"
              value={filter}
              onChange={(event) => setFilter(event.target.value as TaskFilter)}
            >
              <option value="all">{t("tasks.filterAll")}</option>
              <option value="open">{t("tasks.filterOpen")}</option>
              <option value="completed">{t("tasks.filterCompleted")}</option>
              <option value="overdue">{t("tasks.filterOverdue")}</option>
            </select>
          </label>
          <label className="joi-field__label flex items-center gap-1">
            {t("tasks.groupLabel")}
            <select
              className="joi-field__select h-9 w-auto py-1.5"
              value={grouping}
              onChange={(event) => setGrouping(event.target.value as TaskGrouping)}
            >
              <option value="none">{t("tasks.groupNone")}</option>
              <option value="project">{t("tasks.groupProject")}</option>
              <option value="goal">{t("tasks.groupGoal")}</option>
              <option value="nested">{t("tasks.groupNested")}</option>
            </select>
          </label>
        </div>
      </div>
      {visibleTasks.length === 0 ? (
        <p className="joi-empty__state">{t("tasks.noMatchingTasks")}</p>
      ) : grouping === "nested" && nestedGroups ? (
        <div>
          {nestedGroups.map((goalGroup) => (
            <section key={goalGroup.key} className="mt-4 first:mt-0">
              <button
                type="button"
                className="joi-text flex w-full items-center gap-2 py-2 text-left text-sm font-semibold"
                aria-expanded={!collapsed.has(goalGroup.key)}
                onClick={() => toggleCollapsed(goalGroup.key)}
              >
                <span aria-hidden="true">{collapsed.has(goalGroup.key) ? "▸" : "▾"}</span>
                <span>{goalGroup.label}</span>
              </button>
              {!collapsed.has(goalGroup.key)
                ? goalGroup.projects.map((projectGroup) =>
                    renderGroup(
                      `${goalGroup.key}/${projectGroup.key}`,
                      projectGroup.label,
                      projectGroup.tasks,
                    ),
                  )
                : null}
            </section>
          ))}
        </div>
      ) : grouped ? (
        <div>{grouped.map((group) => renderGroup(group.key, group.label, group.tasks))}</div>
      ) : (
        renderGrid(visibleTasks)
      )}
    </>
  );
}

export default StandaloneTaskList;
