import { useEffect, useState } from "react";
import { useNavigate, useSearchParams } from "react-router";
import { useTranslation } from "react-i18next";
import TaskForm, { type TaskFormState } from "@/components/tasks/TaskForm";
import Breadcrumb from "@/components/navigation/Breadcrumb";
import PageContainer from "@/components/navigation/PageContainer";
import { ensureDefaultWorkspace } from "@/lib/api/workspaceApi";
import { createTask } from "@/lib/api/taskApi";
import { toDisplayMessage } from "@/lib/errors";

function NewIndependentTaskPage() {
  const { t } = useTranslation();
  const navigate = useNavigate();
  const [searchParams] = useSearchParams();
  const goalId = searchParams.get("goalId");
  const [error, setError] = useState<string | null>(null);
  const [workspaceId, setWorkspaceId] = useState<string | null>(null);

  useEffect(() => {
    void ensureDefaultWorkspace()
      .then((workspace) => setWorkspaceId(workspace.id))
      .catch((err) => setError(toDisplayMessage(err)));
  }, []);

  const handleSubmit = async (values: TaskFormState) => {
    if (!workspaceId) return;
    try {
      const task = await createTask({
        workspace_id: workspaceId,
        project_id: null,
        goal_id: goalId ?? null,
        parent_task_id: null,
        title: values.title,
        description_markdown: values.description_markdown || null,
        notes_markdown: values.notes_markdown || null,
        priority: values.priority,
        effort_size: values.effort_size,
        target_date: values.target_date || null,
        scheduled_date: values.scheduled_date || null,
      });
      navigate(`/tasks/${task.id}`);
    } catch (err) {
      setError(toDisplayMessage(err));
    }
  };

  return (
    <PageContainer>
      <Breadcrumb items={[{ label: t("tasks.title"), to: "/tasks" }, { label: t("tasks.new") }]} />
      <h1 className="text-2xl font-semibold">
        {goalId ? t("tasks.create.newTitle") : t("tasks.new")}
      </h1>
      {goalId ? (
        <p className="joi-text--muted mt-1 text-sm">{t("tasks.create.goalLinkNote")}</p>
      ) : null}
      {error ? <p className="joi-text--danger mt-2 text-sm">{error}</p> : null}
      <div className="mt-4">
        <TaskForm onSubmit={handleSubmit} submitLabel={t("tasks.form.createSubmit")} />
      </div>
    </PageContainer>
  );
}

export default NewIndependentTaskPage;
