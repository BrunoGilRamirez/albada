import { useEffect, useState } from "react";
import { Link } from "react-router";
import { useTranslation } from "react-i18next";
import StandaloneTaskList from "@/components/tasks/StandaloneTaskList";
import PageContainer from "@/components/navigation/PageContainer";
import { listAtRiskTasks } from "@/lib/api/deadlineApi";
import { ensureDefaultWorkspace } from "@/lib/api/workspaceApi";
import { listTasks } from "@/lib/api/taskApi";
import { listProjects } from "@/lib/api/projectApi";
import { listGoals } from "@/lib/api/goalApi";
import { toDisplayMessage } from "@/lib/errors";
import type { DeadlineTaskInfo, Goal, Project, Task } from "@/lib/types";

function TasksPage() {
  const { t } = useTranslation();
  const [tasks, setTasks] = useState<Task[]>([]);
  const [projects, setProjects] = useState<Project[]>([]);
  const [goals, setGoals] = useState<Goal[]>([]);
  const [deadlines, setDeadlines] = useState<Record<string, DeadlineTaskInfo>>({});
  const [error, setError] = useState<string | null>(null);
  const [workspaceId, setWorkspaceId] = useState<string | null>(null);

  useEffect(() => {
    void ensureDefaultWorkspace()
      .then((workspace) => setWorkspaceId(workspace.id))
      .catch((err) => setError(toDisplayMessage(err)));
  }, []);

  useEffect(() => {
    if (!workspaceId) return;
    void Promise.all([listTasks(workspaceId), listProjects(workspaceId), listGoals(workspaceId)])
      .then(([taskData, projectData, goalData]) => {
        setTasks(taskData);
        setProjects(projectData);
        setGoals(goalData);
      })
      .catch((err) => setError(toDisplayMessage(err)));
  }, [workspaceId]);

  useEffect(() => {
    if (!workspaceId) return;
    void listAtRiskTasks(workspaceId)
      .then((atRisk) => {
        const map: Record<string, DeadlineTaskInfo> = {};
        for (const info of atRisk) {
          map[info.task_id] = info;
        }
        setDeadlines(map);
      })
      .catch((err) => setError(toDisplayMessage(err)));
  }, [workspaceId]);

  return (
    <PageContainer>
      <div className="flex items-center justify-between">
        <h1 data-notification-target="deadline" className="text-2xl font-semibold">
          {t("tasks.listTitle")}
        </h1>
        <Link to="/tasks/new" className="joi-btn joi-btn--primary px-3 py-1.5">
          {t("tasks.new")}
        </Link>
      </div>
      {error ? <p className="joi-text--danger mt-2 text-sm">{error}</p> : null}
      <div className="mt-4">
        <StandaloneTaskList tasks={tasks} projects={projects} goals={goals} deadlines={deadlines} />
      </div>
    </PageContainer>
  );
}

export default TasksPage;
