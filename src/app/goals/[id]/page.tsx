import { useEffect, useState } from "react";
import { Link, useParams } from "react-router";
import { useTranslation } from "react-i18next";
import DeadlineStatusBadge from "@/components/deadlines/DeadlineStatusBadge";
import Breadcrumb, { type BreadcrumbItem } from "@/components/navigation/Breadcrumb";
import ProgressBar from "@/components/progress/ProgressBar";
import ProgressDetailsModal from "@/components/progress/ProgressDetailsModal";
import AddExistingProjectModal from "@/components/projects/AddExistingProjectModal";
import AddExistingTaskModal from "@/components/tasks/AddExistingTaskModal";
import MarkdownPreview from "@/components/markdown/MarkdownPreview";
import PageContainer from "@/components/navigation/PageContainer";
import PageSkeleton from "@/components/navigation/PageSkeleton";
import { getGoalDeadlineStatus } from "@/lib/api/deadlineApi";
import { getGoal } from "@/lib/api/goalApi";
import { listProjects, listProjectsForGoal, moveProject } from "@/lib/api/projectApi";
import { getGoalProgress } from "@/lib/api/progressApi";
import { attachTask, listTasks, listTasksForGoal } from "@/lib/api/taskApi";
import { toDisplayMessage } from "@/lib/errors";
import { attachCandidates } from "@/lib/taskTree";
import type { DeadlineStatusDto, Goal, ProgressDetails, Project, Task } from "@/lib/types";

function GoalDetailPage() {
  const { t } = useTranslation();
  const { id } = useParams<{ id: string }>();
  const [goal, setGoal] = useState<Goal | null>(null);
  const [projects, setProjects] = useState<Project[]>([]);
  const [directTasks, setDirectTasks] = useState<Task[]>([]);
  const [progress, setProgress] = useState<ProgressDetails | null>(null);
  const [deadline, setDeadline] = useState<DeadlineStatusDto | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [showProgressDetails, setShowProgressDetails] = useState(false);
  const [showAttachTask, setShowAttachTask] = useState(false);
  const [attachTaskCandidates, setAttachTaskCandidates] = useState<Task[]>([]);
  const [showAttachProject, setShowAttachProject] = useState(false);
  const [attachProjectCandidates, setAttachProjectCandidates] = useState<Project[]>([]);

  const reload = async (currentId: string) => {
    try {
      const [goalData, goalProjects, goalTasks, progressData, deadlineData] = await Promise.all([
        getGoal(currentId),
        listProjectsForGoal(currentId),
        listTasksForGoal(currentId),
        getGoalProgress(currentId),
        getGoalDeadlineStatus(currentId),
      ]);
      setGoal(goalData);
      setProjects(goalProjects);
      setDirectTasks(goalTasks);
      setProgress(progressData);
      setDeadline(deadlineData);
    } catch (err) {
      setError(toDisplayMessage(err));
    }
  };

  useEffect(() => {
    if (!id) return;
    void reload(id);
  }, [id]);

  const openAttachTask = async () => {
    if (!goal) return;
    setShowAttachTask(true);
    try {
      const allTasks = await listTasks(goal.workspace_id);
      const linkedProjectIds = new Set(projects.map((project) => project.id));
      setAttachTaskCandidates(
        attachCandidates(
          allTasks,
          (task) =>
            task.goal_id === goal.id ||
            (task.project_id !== null && linkedProjectIds.has(task.project_id)),
        ),
      );
    } catch (err) {
      setError(toDisplayMessage(err));
    }
  };

  const handleAttachTask = async (taskId: string) => {
    if (!id) return;
    try {
      await attachTask(taskId, null, id);
      setShowAttachTask(false);
      await reload(id);
    } catch (err) {
      setError(toDisplayMessage(err));
    }
  };

  const openAttachProject = async () => {
    if (!goal) return;
    setShowAttachProject(true);
    try {
      const allProjects = await listProjects(goal.workspace_id);
      setAttachProjectCandidates(allProjects.filter((project) => project.goal_id !== goal.id));
    } catch (err) {
      setError(toDisplayMessage(err));
    }
  };

  const handleAttachProject = async (projectId: string) => {
    if (!id) return;
    try {
      await moveProject(projectId, id);
      setShowAttachProject(false);
      await reload(id);
    } catch (err) {
      setError(toDisplayMessage(err));
    }
  };

  if (!goal) {
    return (
      <section>
        <h1 className="text-2xl font-semibold">{t("goals.detailTitle")}</h1>
        {error ? <p className="joi-text--danger mt-2 text-sm">{error}</p> : <PageSkeleton />}
      </section>
    );
  }

  const breadcrumbs: BreadcrumbItem[] = [
    { label: t("goals.title"), to: "/goals" },
    { label: goal.title },
  ];

  return (
    <PageContainer>
      <Breadcrumb items={breadcrumbs} />
      <div className="flex flex-wrap items-center gap-3">
        {/* Notification target marker (Phase 8.7.5): the intent bridge
            highlights and scrolls to this heading. */}
        <h1 data-notification-target={goal.id} className="text-2xl font-semibold">
          {goal.title}
        </h1>
        {deadline ? (
          <DeadlineStatusBadge status={deadline.status} deadlineAt={goal.target_date} />
        ) : null}
        <span className="joi-badge joi-badge--neutral">
          {t(`common.labels.goalStatus.${goal.status}`)}
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
          to={`/projects/new?goalId=${goal.id}`}
          className="joi-btn joi-btn--primary px-3 py-1.5"
        >
          {t("goals.addProject")}
        </Link>
        <button
          type="button"
          onClick={() => void openAttachProject()}
          className="joi-btn joi-btn--secondary px-3 py-1.5"
        >
          {t("common.actions.addExistingProject")}
        </button>
        <Link
          to={`/tasks/new?goalId=${goal.id}`}
          className="joi-btn joi-btn--secondary px-3 py-1.5"
        >
          {t("goals.addDirectTask")}
        </Link>
        <button
          type="button"
          onClick={() => void openAttachTask()}
          className="joi-btn joi-btn--secondary px-3 py-1.5"
        >
          {t("common.actions.addExistingTask")}
        </button>
        <Link to={`/goals/${goal.id}/edit`} className="joi-btn joi-btn--secondary px-3 py-1.5">
          {t("common.actions.edit")}
        </Link>
      </div>

      {goal.description_markdown ? (
        <div className="mt-4">
          <h2 className="joi-text mb-1 text-sm font-medium">{t("common.labels.description")}</h2>
          <MarkdownPreview
            content={goal.description_markdown}
            emptyText={t("common.markdown.empty")}
          />
        </div>
      ) : null}
      {goal.why_it_matters_markdown ? (
        <div className="mt-4">
          <h2 className="joi-text mb-1 text-sm font-medium">{t("common.labels.whyItMatters")}</h2>
          <MarkdownPreview
            content={goal.why_it_matters_markdown}
            emptyText={t("common.markdown.empty")}
          />
        </div>
      ) : null}
      {goal.success_definition_markdown ? (
        <div className="mt-4">
          <h2 className="joi-text mb-1 text-sm font-medium">{t("goals.successDefinition")}</h2>
          <MarkdownPreview
            content={goal.success_definition_markdown}
            emptyText={t("common.markdown.empty")}
          />
        </div>
      ) : null}

      <div className="mt-6 grid grid-cols-1 gap-6 sm:grid-cols-2">
        <div>
          <h2 className="mb-2 text-lg font-medium">{t("goals.linkedProjects")}</h2>
          {projects.length === 0 ? (
            <p className="joi-empty__state">{t("goals.linkedProjectsEmpty")}</p>
          ) : (
            <ul className="flex flex-col gap-2">
              {projects.map((project) => (
                <li key={project.id}>
                  <Link
                    to={`/projects/${project.id}`}
                    className="joi-surface--base joi-border--default block truncate rounded-lg border px-3 py-2 text-sm hover:underline"
                  >
                    {project.title}
                  </Link>
                </li>
              ))}
            </ul>
          )}
        </div>
        <div>
          <h2 className="mb-2 text-lg font-medium">{t("goals.directTasks")}</h2>
          {directTasks.length === 0 ? (
            <p className="joi-empty__state">{t("goals.directTasksEmpty")}</p>
          ) : (
            <ul className="flex flex-col gap-2">
              {directTasks.map((task) => (
                <li key={task.id}>
                  <Link
                    to={`/tasks/${task.id}`}
                    className="joi-surface--base joi-border--default block truncate rounded-lg border px-3 py-2 text-sm hover:underline"
                  >
                    {task.title}
                  </Link>
                </li>
              ))}
            </ul>
          )}
        </div>
      </div>

      <AddExistingTaskModal
        open={showAttachTask}
        title={t("common.actions.addExistingTask")}
        description={t("goals.attachTaskDescription")}
        tasks={attachTaskCandidates}
        onAttach={handleAttachTask}
        onClose={() => setShowAttachTask(false)}
      />

      <AddExistingProjectModal
        open={showAttachProject}
        title={t("common.actions.addExistingProject")}
        description={t("goals.attachProjectDescription")}
        projects={attachProjectCandidates}
        onAttach={handleAttachProject}
        onClose={() => setShowAttachProject(false)}
      />

      <ProgressDetailsModal
        open={showProgressDetails}
        title={goal.title}
        details={progress}
        onClose={() => setShowProgressDetails(false)}
      />
    </PageContainer>
  );
}

export default GoalDetailPage;
