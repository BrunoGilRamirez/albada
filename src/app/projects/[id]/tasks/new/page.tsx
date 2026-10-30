import { useEffect, useState } from "react";
import { useNavigate, useParams } from "react-router";
import { useTranslation } from "react-i18next";
import TaskForm, { type TaskFormState } from "@/components/tasks/TaskForm";
import Breadcrumb from "@/components/navigation/Breadcrumb";
import PageContainer from "@/components/navigation/PageContainer";
import { createTask } from "@/lib/api/taskApi";
import { getProject } from "@/lib/api/projectApi";
import { toDisplayMessage } from "@/lib/errors";

function NewTaskPage() {
  const { t } = useTranslation();
  const { id: projectId } = useParams<{ id: string }>();
  const navigate = useNavigate();
  const [error, setError] = useState<string | null>(null);
  const [projectTitle, setProjectTitle] = useState<string | null>(null);
  const [workspaceId, setWorkspaceId] = useState<string | null>(null);

  useEffect(() => {
    if (!projectId) return;
    void getProject(projectId)
      .then((project) => {
        setProjectTitle(project.title);
        setWorkspaceId(project.workspace_id);
      })
      .catch((err) => setError(toDisplayMessage(err)));
  }, [projectId]);

  const handleSubmit = async (values: TaskFormState) => {
    if (!projectId || !workspaceId) {
      setError(t("tasks.create.projectLoadingContext"));
      return;
    }
    try {
      const task = await createTask({
        workspace_id: workspaceId,
        project_id: projectId,
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
    <PageContainer variant="reading">
      <Breadcrumb
        items={[
          { label: t("projects.title"), to: "/projects" },
          { label: projectTitle ?? t("projects.detailTitle"), to: `/projects/${projectId}` },
          { label: t("tasks.create.newTitle") },
        ]}
      />
      <h1 className="text-2xl font-semibold">
        {projectTitle
          ? t("tasks.create.newInProject", { project: projectTitle })
          : t("tasks.create.newTitle")}
      </h1>
      {error ? <p className="joi-text--danger mt-2 text-sm">{error}</p> : null}
      <div className="mt-4">
        <TaskForm
          onSubmit={handleSubmit}
          submitLabel={t("tasks.form.createSubmit")}
          disabled={!workspaceId}
        />
      </div>
    </PageContainer>
  );
}

export default NewTaskPage;
