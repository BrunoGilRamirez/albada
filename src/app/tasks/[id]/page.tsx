import { useEffect, useState } from "react";
import { Link, useNavigate, useParams } from "react-router";
import { useTranslation } from "react-i18next";
import Breadcrumb, { type BreadcrumbItem } from "@/components/navigation/Breadcrumb";
import DeadlineStatusBadge from "@/components/deadlines/DeadlineStatusBadge";
import ProgressBar from "@/components/progress/ProgressBar";
import BlockedReasonPanel from "@/components/tasks/BlockedReasonPanel";
import DependencyEditor from "@/components/tasks/DependencyEditor";
import TaskEffortBadge from "@/components/tasks/TaskEffortBadge";
import TaskPriorityBadge from "@/components/tasks/TaskPriorityBadge";
import TaskStatusBadge from "@/components/tasks/TaskStatusBadge";
import TaskTree from "@/components/tasks/TaskTree";
import MarkdownPreview from "@/components/markdown/MarkdownPreview";
import PageContainer from "@/components/navigation/PageContainer";
import PageSkeleton from "@/components/navigation/PageSkeleton";
import { getTaskDeadlineStatus } from "@/lib/api/deadlineApi";
import { getGoal } from "@/lib/api/goalApi";
import { getProject } from "@/lib/api/projectApi";
import { getTaskProgress } from "@/lib/api/progressApi";
import {
  archiveTask,
  completeTask,
  deleteTask,
  getTaskWithTree,
  listTasks,
  restoreTask,
  transitionTaskStatus,
} from "@/lib/api/taskApi";
import { toDisplayMessage } from "@/lib/errors";
import { ancestorChain } from "@/lib/taskTree";
import type {
  DeadlineStatusDto,
  ProgressDetails,
  Task,
  TaskTree as TaskTreeType,
} from "@/lib/types";
import { TASK_STATUSES } from "@/lib/types";

// The breadcrumb root can point at a user-authored project/goal title or at
// the application-owned "Tasks" list; only the latter is translated, so the
// root keeps its kind and its visible label is resolved at render time. This
// makes the root follow runtime language changes without reloading the page.
type BreadcrumbRoot = { kind: "project" | "goal"; label: string; to?: string } | { kind: "tasks" };

function TaskDetailPage() {
  const { t } = useTranslation();
  const { id } = useParams<{ id: string }>();
  const navigate = useNavigate();
  const [tree, setTree] = useState<TaskTreeType | null>(null);
  const [task, setTask] = useState<Task | null>(null);
  const [deadline, setDeadline] = useState<DeadlineStatusDto | null>(null);
  const [progress, setProgress] = useState<ProgressDetails | null>(null);
  const [rootCrumb, setRootCrumb] = useState<BreadcrumbRoot | null>(null);
  const [ancestorCrumb, setAncestorCrumb] = useState<BreadcrumbItem[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [targetStatus, setTargetStatus] = useState("");
  const [blockingNote, setBlockingNote] = useState("");
  const [confirmingDelete, setConfirmingDelete] = useState(false);

  const loadBreadcrumbs = async (currentTask: Task) => {
    try {
      const allTasks = await listTasks(currentTask.workspace_id);
      const ancestors = ancestorChain(allTasks, currentTask.id);
      let root: BreadcrumbRoot;
      if (currentTask.project_id) {
        const project = await getProject(currentTask.project_id);
        root = { kind: "project", label: project.title, to: `/projects/${project.id}` };
      } else if (currentTask.goal_id) {
        const goal = await getGoal(currentTask.goal_id);
        root = { kind: "goal", label: goal.title, to: `/goals/${goal.id}` };
      } else {
        root = { kind: "tasks" };
      }
      setRootCrumb(root);
      setAncestorCrumb(
        ancestors.map((ancestor) => ({
          label: ancestor.title,
          to: `/tasks/${ancestor.id}`,
        })),
      );
    } catch (err) {
      // Breadcrumbs are a convenience; a failure must not hide the task.
      setError(toDisplayMessage(err));
    }
  };

  const reload = async () => {
    if (!id) return;
    setError(null);
    setConfirmingDelete(false);
    setTargetStatus("");
    try {
      const data = await getTaskWithTree(id);
      setTree(data);
      setTask(data.task);
      void loadBreadcrumbs(data.task);
      void getTaskDeadlineStatus(id)
        .then(setDeadline)
        .catch((err) => setError(toDisplayMessage(err)));
      if (data.children.length > 0) {
        void getTaskProgress(id)
          .then(setProgress)
          .catch((err) => setError(toDisplayMessage(err)));
      } else {
        setProgress(null);
      }
    } catch (err) {
      setError(toDisplayMessage(err));
    }
  };

  useEffect(() => {
    void reload();
  }, [id]);

  const performTransition = async (toStatus: string) => {
    if (!id) return;
    const reason = toStatus === "blocked" && blockingNote.trim() ? blockingNote : undefined;
    try {
      await transitionTaskStatus(id, toStatus, reason);
      setTargetStatus("");
      setBlockingNote("");
      await reload();
    } catch (err) {
      setError(toDisplayMessage(err));
    }
  };

  const handleComplete = async () => {
    if (!id) return;
    try {
      await completeTask(id);
      await reload();
    } catch (err) {
      setError(toDisplayMessage(err));
    }
  };

  const handleReopen = async () => {
    if (!id) return;
    try {
      // Reopen to `in_progress` so the task can be completed again. The
      // lifecycle rules do not allow `planned -> completed`, so reopening to
      // `planned` would leave the task unable to be completed directly.
      await transitionTaskStatus(id, "in_progress");
      await reload();
    } catch (err) {
      setError(toDisplayMessage(err));
    }
  };

  const handleToggleArchive = async () => {
    if (!id || !tree) return;
    try {
      if (subtreeHasArchived(tree)) {
        await restoreTask(id);
      } else {
        await archiveTask(id);
      }
      await reload();
    } catch (err) {
      setError(toDisplayMessage(err));
    }
  };

  const handleDelete = async () => {
    if (!id) return;
    if (!confirmingDelete) {
      setConfirmingDelete(true);
      return;
    }
    try {
      await deleteTask(id);
      navigate("/tasks");
    } catch (err) {
      setError(toDisplayMessage(err));
    }
  };

  // The archive toggle hides only the completed children, so the button state
  // depends on whether any descendant is archived, not the task itself.
  const subtreeHasArchived = (node: TaskTreeType): boolean =>
    node.children.some((child) => Boolean(child.task.archived_at) || subtreeHasArchived(child));

  if (!task || !tree) {
    return (
      <section>
        <h1 className="text-2xl font-semibold">{t("tasks.detailTitle")}</h1>
        {error ? <p className="joi-text--danger mt-2 text-sm">{error}</p> : <PageSkeleton />}
      </section>
    );
  }

  const breadcrumbs: BreadcrumbItem[] = rootCrumb
    ? [
        rootCrumb.kind === "tasks"
          ? { label: t("tasks.title"), to: "/tasks" }
          : { label: rootCrumb.label, to: rootCrumb.to },
        ...ancestorCrumb,
        { label: task.title },
      ]
    : [];

  return (
    <PageContainer>
      <Breadcrumb items={breadcrumbs} />
      <div className="flex flex-wrap items-center gap-3">
        <h1 data-notification-target={task.id} className="text-2xl font-semibold">
          {task.title}
        </h1>
        <TaskStatusBadge status={task.status} />
        <TaskPriorityBadge priority={task.priority} />
        <TaskEffortBadge effort={task.effort_size} />
        {deadline ? (
          <DeadlineStatusBadge status={deadline.status} deadlineAt={task.target_date} />
        ) : null}
        {subtreeHasArchived(tree) ? (
          <span className="joi-badge joi-badge--neutral">{t("tasks.childrenArchived")}</span>
        ) : null}
      </div>

      {tree.children.length > 0 ? (
        <div className="mt-3">
          <ProgressBar percentage={progress?.percentage} label={progress?.label} size="sm" />
        </div>
      ) : null}

      {error ? <p className="joi-text--danger mt-2 text-sm">{error}</p> : null}

      <div className="mt-4 flex flex-wrap gap-2">
        {task.status === "completed" ? (
          <button
            type="button"
            onClick={handleReopen}
            className="joi-btn joi-btn--secondary px-3 py-1.5"
          >
            {t("tasks.reopen")}
          </button>
        ) : task.status === "in_progress" || task.status === "inbox" ? (
          <button
            type="button"
            onClick={handleComplete}
            className="joi-btn joi-btn--primary px-3 py-1.5"
          >
            {t("common.actions.complete")}
          </button>
        ) : null}
        {task.status !== "completed" ? (
          <select
            aria-label={t("tasks.changeStatus")}
            value={targetStatus}
            onChange={(e) => setTargetStatus(e.target.value)}
            className="joi-field__select w-auto px-2 py-1.5"
          >
            <option value="">{t("tasks.changeStatusPlaceholder")}</option>
            {TASK_STATUSES.filter(
              (status) =>
                status !== task.status && status !== "completed" && status !== "cancelled",
            ).map((status) => (
              <option key={status} value={status}>
                {t(`common.labels.taskStatus.${status}`)}
              </option>
            ))}
          </select>
        ) : null}
        {targetStatus === "blocked" && task.status !== "blocked" ? (
          <input
            aria-label={t("tasks.blockingNote")}
            type="text"
            placeholder={t("tasks.blockingNotePlaceholder")}
            value={blockingNote}
            onChange={(e) => setBlockingNote(e.target.value)}
            className="joi-field__input w-64 px-2 py-1.5"
          />
        ) : null}
        <button
          type="button"
          onClick={() => void performTransition(targetStatus)}
          disabled={!targetStatus}
          className="joi-btn joi-btn--primary px-3 py-1.5"
        >
          {t("common.actions.apply")}
        </button>
        <Link to={`/tasks/${task.id}/edit`} className="joi-btn joi-btn--primary px-3 py-1.5">
          {t("common.actions.edit")}
        </Link>
        {tree.children.length > 0 && !subtreeHasArchived(tree) ? (
          <button
            type="button"
            onClick={handleToggleArchive}
            title={t("tasks.archiveSubtreeHint")}
            className="joi-btn joi-btn--secondary px-3 py-1.5"
          >
            {t("tasks.archiveSubtree")}
          </button>
        ) : tree.children.length > 0 ? (
          <button
            type="button"
            onClick={handleToggleArchive}
            className="joi-btn joi-btn--secondary px-3 py-1.5"
          >
            {t("tasks.restoreSubtree")}
          </button>
        ) : null}
        <button
          type="button"
          onClick={handleDelete}
          className={`joi-btn px-3 py-1.5 ${confirmingDelete ? "joi-btn--danger" : "joi-btn--secondary"}`}
        >
          {confirmingDelete ? t("tasks.confirmDelete") : t("tasks.delete")}
        </button>
      </div>

      {task.status === "blocked" || tree.blocked_by.length > 0 ? (
        <div className="mt-4">
          <BlockedReasonPanel taskId={task.id} />
          {task.blocking_note_markdown ? (
            <div className="joi-alert joi-alert--warning mt-2">
              <h2 className="mb-1 text-sm font-medium">{t("tasks.blockingNote")}</h2>
              <MarkdownPreview
                content={task.blocking_note_markdown}
                emptyText={t("common.markdown.empty")}
              />
            </div>
          ) : null}
        </div>
      ) : null}

      {task.description_markdown ? (
        <div className="mt-4">
          <h2 className="joi-text mb-1 text-sm font-medium">{t("common.labels.description")}</h2>
          <MarkdownPreview
            content={task.description_markdown}
            emptyText={t("common.markdown.empty")}
          />
        </div>
      ) : null}

      {task.closure_reason_markdown ? (
        <div className="joi-surface--raised joi-border--default mt-4 rounded-lg border p-3">
          <h2 className="joi-text mb-1 text-sm font-medium">{t("tasks.closureReason")}</h2>
          <MarkdownPreview
            content={task.closure_reason_markdown}
            emptyText={t("common.markdown.empty")}
          />
        </div>
      ) : null}

      <div className="mt-6">
        <h2 className="mb-2 text-lg font-medium">{t("common.labels.taskTree")}</h2>
        <TaskTree tree={tree} />
      </div>

      <div className="mt-6 grid grid-cols-1 gap-6 sm:grid-cols-2">
        <div>
          <h2 className="mb-2 text-lg font-medium">{t("tasks.prerequisites")}</h2>
          <DependencyEditor workspaceId={task.workspace_id} taskId={task.id} />
        </div>
        <div>
          <h2 className="mb-2 text-lg font-medium">{t("tasks.createChild")}</h2>
          <Link
            to={`/tasks/${task.id}/children/new`}
            className="joi-btn joi-btn--secondary px-3 py-1.5"
          >
            {t("tasks.addChild")}
          </Link>
        </div>
      </div>
    </PageContainer>
  );
}

export default TaskDetailPage;
