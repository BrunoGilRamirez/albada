import { useEffect, useState } from "react";
import { Link, useParams } from "react-router";
import { useTranslation } from "react-i18next";
import DeadlineStatusBadge from "@/components/deadlines/DeadlineStatusBadge";
import Breadcrumb, { type BreadcrumbItem } from "@/components/navigation/Breadcrumb";
import ProgressBar from "@/components/progress/ProgressBar";
import ProgressDetailsModal from "@/components/progress/ProgressDetailsModal";
import AddExistingTaskModal from "@/components/tasks/AddExistingTaskModal";
import TaskTree from "@/components/tasks/TaskTree";
import MarkdownPreview from "@/components/markdown/MarkdownPreview";
import PageContainer from "@/components/navigation/PageContainer";
import PageSkeleton from "@/components/navigation/PageSkeleton";
import { getProjectDeadlineStatus } from "@/lib/api/deadlineApi";
import { getGoal } from "@/lib/api/goalApi";
import { getProjectProgress, getTaskProgress } from "@/lib/api/progressApi";
import { getProject } from "@/lib/api/projectApi";
import { attachTask, listTasks, listTasksForProject } from "@/lib/api/taskApi";
import { toDisplayMessage } from "@/lib/errors";
import { attachCandidates, buildTaskForest } from "@/lib/taskTree";
import type { DeadlineStatusDto, ProgressDetails, Project, Task } from "@/lib/types";

interface GoalBreadcrumb {
  id: string;
  title: string;
}

function ProjectDetailPage() {
  const { t } = useTranslation();
  const { id } = useParams<{ id: string }>();
  const [project, setProject] = useState<Project | null>(null);
  const [tasks, setTasks] = useState<Task[]>([]);
  const [progress, setProgress] = useState<ProgressDetails | null>(null);
  const [progressByTask, setProgressByTask] = useState<Record<string, ProgressDetails>>({});
  const [deadline, setDeadline] = useState<DeadlineStatusDto | null>(null);
  // The goal crumb keeps the user-authored title (loaded data); the fallback
  // "Projects" root is application copy and is translated at render time so
  // it follows runtime language changes.
  const [goalCrumb, setGoalCrumb] = useState<GoalBreadcrumb | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [showProgressDetails, setShowProgressDetails] = useState(false);
  const [showAttachExisting, setShowAttachExisting] = useState(false);
  const [attachCandidatesTasks, setAttachCandidatesTasks] = useState<Task[]>([]);

  const reload = async (currentId: string) => {
    try {
      const [projectData, projectTasks, progressData, deadlineData] = await Promise.all([
        getProject(currentId),
        listTasksForProject(currentId),
        getProjectProgress(currentId),
        getProjectDeadlineStatus(currentId),
      ]);
      setProject(projectData);
      setTasks(projectTasks);
      setProgress(progressData);
      setDeadline(deadlineData);

      if (projectData.goal_id) {
        const goal = await getGoal(projectData.goal_id);
        setGoalCrumb({ id: goal.id, title: goal.title });
      } else {
        setGoalCrumb(null);
      }

      const parentIds = new Set(projectTasks.map((task) => task.parent_task_id));
      const parents = projectTasks.filter((task) => parentIds.has(task.id));
      const treeProgress: Record<string, ProgressDetails> = {};
      await Promise.all(
        parents.map(async (parent) => {
          try {
            treeProgress[parent.id] = await getTaskProgress(parent.id);
          } catch (err) {
            setError(toDisplayMessage(err));
          }
        }),
      );
      setProgressByTask(treeProgress);
    } catch (err) {
      setError(toDisplayMessage(err));
    }
  };

  useEffect(() => {
    if (!id) return;
    void reload(id);
  }, [id]);

  const openAttachExisting = async () => {
    if (!project) return;
    setShowAttachExisting(true);
    try {
      const allTasks = await listTasks(project.workspace_id);
      setAttachCandidatesTasks(
        attachCandidates(allTasks, (task) => task.project_id === project.id),
      );
    } catch (err) {
      setError(toDisplayMessage(err));
    }
  };

  const handleAttachTask = async (taskId: string) => {
    if (!id) return;
    try {
      await attachTask(taskId, id, null);
      setShowAttachExisting(false);
      await reload(id);
    } catch (err) {
      setError(toDisplayMessage(err));
    }
  };

  if (!project) {
    return (
      <section>
        <h1 className="text-2xl font-semibold">{t("projects.detailTitle")}</h1>
        {error ? <p className="joi-text--danger mt-2 text-sm">{error}</p> : <PageSkeleton />}
      </section>
    );
  }

  const breadcrumbs: BreadcrumbItem[] = goalCrumb
    ? [{ label: goalCrumb.title, to: `/goals/${goalCrumb.id}` }, { label: project.title }]
    : [{ label: t("projects.title"), to: "/projects" }, { label: project.title }];

  const roots = buildTaskForest(tasks);

  return (
    <PageContainer>
      <Breadcrumb items={breadcrumbs} />
      <div className="flex flex-wrap items-center gap-3">
        {/* Notification target marker (Phase 8.7.5): the intent bridge
            highlights and scrolls to this heading, unavailable targets fall
            through to the page's existing not-found handling. */}
        <h1 data-notification-target={project.id} className="text-2xl font-semibold">
          {project.title}
        </h1>
        {deadline ? (
          <DeadlineStatusBadge status={deadline.status} deadlineAt={project.target_date} />
        ) : null}
        <span className="joi-badge joi-badge--neutral">
          {t(`common.labels.projectStatus.${project.status}`)}
        </span>
      </div>

      {error ? <p className="joi-text--danger mt-2 text-sm">{error}</p> : null}

      <div className="mt-4 flex flex-wrap items-center gap-3">
        <ProgressBar percentage={progress?.percentage} label={progress?.label} />
        <button
          type="button"
          onClick={() => setShowProgressDetails(true)}
          className="joi-btn joi-btn--secondary px-3 py-1.5"
        >
          {t("common.actions.viewProgressDetails")}
        </button>
      </div>

      <div className="mt-4 flex flex-wrap gap-2">
        <Link
          to={`/projects/${project.id}/tasks/new`}
          className="joi-btn joi-btn--primary px-3 py-1.5"
        >
          {t("projects.addTask")}
        </Link>
        <button
          type="button"
          onClick={() => void openAttachExisting()}
          className="joi-btn joi-btn--secondary px-3 py-1.5"
        >
          {t("common.actions.addExistingTask")}
        </button>
        <Link
          to={`/projects/${project.id}/edit`}
          className="joi-btn joi-btn--secondary px-3 py-1.5"
        >
          {t("common.actions.edit")}
        </Link>
      </div>

      {project.description_markdown ? (
        <div className="mt-4">
          <h2 className="joi-text mb-1 text-sm font-medium">{t("common.labels.description")}</h2>
          <MarkdownPreview
            content={project.description_markdown}
            emptyText={t("common.markdown.empty")}
          />
        </div>
      ) : null}
      {project.why_it_matters_markdown ? (
        <div className="mt-4">
          <h2 className="joi-text mb-1 text-sm font-medium">{t("common.labels.whyItMatters")}</h2>
          <MarkdownPreview
            content={project.why_it_matters_markdown}
            emptyText={t("common.markdown.empty")}
          />
        </div>
      ) : null}
      {project.definition_of_done_markdown ? (
        <div className="mt-4">
          <h2 className="joi-text mb-1 text-sm font-medium">{t("projects.definitionOfDone")}</h2>
          <MarkdownPreview
            content={project.definition_of_done_markdown}
            emptyText={t("common.markdown.empty")}
          />
        </div>
      ) : null}

      <div className="mt-6">
        <h2 className="mb-2 text-lg font-medium">{t("common.labels.taskTree")}</h2>
        {roots.length === 0 ? (
          <p className="joi-empty__state">{t("projects.taskTreeEmpty")}</p>
        ) : (
          <ul className="flex flex-col">
            {roots.map((root) => (
              <TaskTree key={root.task.id} tree={root} progressByTask={progressByTask} />
            ))}
          </ul>
        )}
      </div>

      <AddExistingTaskModal
        open={showAttachExisting}
        title={t("common.actions.addExistingTask")}
        description={t("projects.attachTaskDescription")}
        tasks={attachCandidatesTasks}
        onAttach={handleAttachTask}
        onClose={() => setShowAttachExisting(false)}
      />

      <ProgressDetailsModal
        open={showProgressDetails}
        title={project.title}
        details={progress}
        onClose={() => setShowProgressDetails(false)}
      />
    </PageContainer>
  );
}

export default ProjectDetailPage;
