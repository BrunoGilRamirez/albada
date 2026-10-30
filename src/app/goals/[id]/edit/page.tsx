import { useEffect, useState } from "react";
import { useNavigate, useParams } from "react-router";
import { useTranslation } from "react-i18next";
import GoalForm from "@/components/goals/GoalForm";
import Breadcrumb from "@/components/navigation/Breadcrumb";
import PageContainer from "@/components/navigation/PageContainer";
import PageSkeleton from "@/components/navigation/PageSkeleton";
import { getGoal, updateGoal } from "@/lib/api/goalApi";
import { toDisplayMessage } from "@/lib/errors";
import type { Goal } from "@/lib/types";

function EditGoalPage() {
  const { t } = useTranslation();
  const { id } = useParams<{ id: string }>();
  const navigate = useNavigate();
  const [goal, setGoal] = useState<Goal | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (!id) return;
    void getGoal(id)
      .then(setGoal)
      .catch((err) => setError(toDisplayMessage(err)));
  }, [id]);

  const handleSubmit = async (values: {
    title: string;
    description_markdown: string;
    why_it_matters_markdown: string;
    success_definition_markdown: string;
    status: string;
    priority: string;
    target_date: string;
  }) => {
    if (!id) return;
    try {
      const updated = await updateGoal(id, {
        title: values.title,
        description_markdown: values.description_markdown || null,
        why_it_matters_markdown: values.why_it_matters_markdown || null,
        success_definition_markdown: values.success_definition_markdown || null,
        status: values.status,
        priority: values.priority,
        target_date: values.target_date || null,
      });
      navigate(`/goals/${updated.id}`);
    } catch (err) {
      setError(toDisplayMessage(err));
    }
  };

  return (
    <PageContainer>
      <Breadcrumb
        items={[
          { label: t("goals.title"), to: "/goals" },
          { label: t("goals.detailTitle"), to: `/goals/${id}` },
          { label: t("goals.editTitle") },
        ]}
      />
      <h1 className="text-2xl font-semibold">{t("goals.editTitle")}</h1>
      {error ? <p className="joi-text--danger mt-2 text-sm">{error}</p> : null}
      {goal ? (
        <div className="mt-4">
          <GoalForm
            initial={{
              title: goal.title,
              description_markdown: goal.description_markdown ?? "",
              why_it_matters_markdown: goal.why_it_matters_markdown ?? "",
              success_definition_markdown: goal.success_definition_markdown ?? "",
              status: goal.status,
              priority: goal.priority,
              target_date: goal.target_date ?? "",
            }}
            editableManagement
            onSubmit={handleSubmit}
            submitLabel={t("common.actions.saveChanges")}
          />
        </div>
      ) : (
        <PageSkeleton variant="form" />
      )}
    </PageContainer>
  );
}

export default EditGoalPage;
