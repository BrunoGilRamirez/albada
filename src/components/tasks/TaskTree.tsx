import { useState } from "react";
import { Link } from "react-router";
import { useTranslation } from "react-i18next";
import type { ProgressDetails, TaskTree as TaskTreeType } from "../../lib/types";
import TaskStatusBadge from "./TaskStatusBadge";
import TaskPriorityBadge from "./TaskPriorityBadge";
import TaskEffortBadge from "./TaskEffortBadge";
import ProgressBar from "../progress/ProgressBar";

interface TaskTreeNodeProps {
  node: TaskTreeType;
  depth: number;
  progressByTask?: Record<string, ProgressDetails>;
}

function TaskTreeNode({ node, depth, progressByTask }: TaskTreeNodeProps) {
  const { t } = useTranslation();
  const [expanded, setExpanded] = useState(true);
  // Archived children stay visible in the parent's tree so the user can still
  // see and restore them; they only disappear from the general task lists.
  const hasChildren = node.children.length > 0;
  const isArchived = Boolean(node.task.archived_at);
  const progress = hasChildren ? progressByTask?.[node.task.id] : undefined;

  return (
    <li>
      <div
        className={`joi-tree__row rounded-md px-2 py-1.5 ${isArchived ? "opacity-60" : ""}`}
        style={{ paddingLeft: `${0.75 + depth * 1.25}rem` }}
      >
        {hasChildren ? (
          <button
            type="button"
            aria-label={expanded ? t("common.actions.collapse") : t("common.actions.expand")}
            onClick={() => setExpanded((value) => !value)}
            className="joi-text--subtle flex h-4 w-4 items-center justify-center"
          >
            {expanded ? "▾" : "▸"}
          </button>
        ) : (
          <span className="h-4 w-4" aria-hidden="true" />
        )}
        <Link
          to={`/tasks/${node.task.id}`}
          className="joi-text min-w-0 flex-1 truncate text-sm hover:underline"
        >
          {node.task.title}
        </Link>
        {isArchived ? (
          <span className="joi-badge joi-badge--neutral shrink-0">{t("tasks.archived")}</span>
        ) : null}
        {progress ? (
          <span className="shrink-0">
            <ProgressBar percentage={progress.percentage} label={progress.label} size="sm" />
          </span>
        ) : null}
        <TaskStatusBadge status={node.task.status} />
        <TaskPriorityBadge priority={node.task.priority} />
        <TaskEffortBadge effort={node.task.effort_size} />
        {node.blocked_by.length > 0 ? (
          <span className="joi-text--warning text-xs" title={t("tasks.blockedByPrerequisite")}>
            {t("common.labels.taskStatus.blocked").toLowerCase()}
          </span>
        ) : null}
      </div>
      {hasChildren && expanded ? (
        <ul className="joi-tree">
          {node.children.map((child) => (
            <TaskTreeNode
              key={child.task.id}
              node={child}
              depth={depth + 1}
              progressByTask={progressByTask}
            />
          ))}
        </ul>
      ) : null}
    </li>
  );
}

interface TaskTreeProps {
  tree: TaskTreeType;
  progressByTask?: Record<string, ProgressDetails>;
}

function TaskTree({ tree, progressByTask }: TaskTreeProps) {
  return (
    <ul className="joi-tree">
      <TaskTreeNode node={tree} depth={0} progressByTask={progressByTask} />
    </ul>
  );
}

export default TaskTree;
