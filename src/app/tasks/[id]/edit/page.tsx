import { useEffect, useState } from "react";
import { useNavigate, useParams } from "react-router";
import { useTranslation } from "react-i18next";
import TaskForm, { type TaskFormState } from "@/components/tasks/TaskForm";
import Breadcrumb from "@/components/navigation/Breadcrumb";
import PageContainer from "@/components/navigation/PageContainer";
import PageSkeleton from "@/components/navigation/PageSkeleton";
import { getTask, updateTask } from "@/lib/api/taskApi";
import { toDisplayMessage } from "@/lib/errors";

function EditTaskPage() {
  const { t } = useTranslation();
  const { id } = useParams<{ id: string }>();
  const navigate = useNavigate();
  const [initial, setInitial] = useState<Partial<TaskFormState> | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (!id) return;
    void getTask(id)
      .then((task) => {
        setInitial({
          title: task.title,
          description_markdown: task.description_markdown ?? "",
          notes_markdown: task.notes_markdown ?? "",
          priority: task.priority,
          effort_size: task.effort_size,
          target_date: task.target_date ?? "",
          scheduled_date: task.scheduled_date ?? "",
        });
      })
      .catch((err) => setError(toDisplayMessage(err)));
  }, [id]);

  const handleSubmit = async (values: TaskFormState) => {
    if (!id) return;
    try {
      const task = await updateTask(id, {
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

  if (!initial) {
    return (
      <section>
        <h1 className="text-2xl font-semibold">{t("tasks.editTitle")}</h1>
        {error ? (
          <p className="joi-text--danger mt-2 text-sm">{error}</p>
        ) : (
          <PageSkeleton variant="form" />
        )}
      </section>
    );
  }

  return (
    <PageContainer>
      <Breadcrumb
        items={[
          { label: t("tasks.title"), to: "/tasks" },
          { label: t("tasks.detailTitle"), to: `/tasks/${id}` },
          { label: t("tasks.editTitle") },
        ]}
      />
      <h1 className="text-2xl font-semibold">{t("tasks.editTitle")}</h1>
      {error ? <p className="joi-text--danger mt-2 text-sm">{error}</p> : null}
      <div className="mt-4">
        <TaskForm
          initial={initial}
          onSubmit={handleSubmit}
          submitLabel={t("common.actions.saveChanges")}
        />
      </div>
    </PageContainer>
  );
}

export default EditTaskPage;
