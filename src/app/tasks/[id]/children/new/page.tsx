import { useState } from "react";
import { useNavigate, useParams } from "react-router";
import { useTranslation } from "react-i18next";
import TaskForm, { type TaskFormState } from "@/components/tasks/TaskForm";
import Breadcrumb from "@/components/navigation/Breadcrumb";
import PageContainer from "@/components/navigation/PageContainer";
import { createChildTask } from "@/lib/api/taskApi";
import { toDisplayMessage } from "@/lib/errors";

function NewChildTaskPage() {
  const { t } = useTranslation();
  const { id: parentId } = useParams<{ id: string }>();
  const navigate = useNavigate();
  const [error, setError] = useState<string | null>(null);

  const handleSubmit = async (values: TaskFormState) => {
    if (!parentId) return;
    try {
      const task = await createChildTask({
        parent_id: parentId,
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
          { label: t("tasks.title"), to: "/tasks" },
          { label: t("tasks.detailTitle"), to: `/tasks/${parentId}` },
          { label: t("tasks.create.newChild") },
        ]}
      />
      <h1 className="text-2xl font-semibold">{t("tasks.create.newChild")}</h1>
      {error ? <p className="joi-text--danger mt-2 text-sm">{error}</p> : null}
      <div className="mt-4">
        <TaskForm onSubmit={handleSubmit} submitLabel={t("tasks.form.createChildSubmit")} />
      </div>
    </PageContainer>
  );
}

export default NewChildTaskPage;
