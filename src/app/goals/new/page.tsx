import { useEffect, useState } from "react";
import { useNavigate } from "react-router";
import { useTranslation } from "react-i18next";
import GoalForm, { type GoalFormState } from "@/components/goals/GoalForm";
import Breadcrumb from "@/components/navigation/Breadcrumb";
import PageContainer from "@/components/navigation/PageContainer";
import { createGoal } from "@/lib/api/goalApi";
import { ensureDefaultWorkspace } from "@/lib/api/workspaceApi";
import { toDisplayMessage } from "@/lib/errors";

function NewGoalPage() {
  const { t } = useTranslation();
  const navigate = useNavigate();
  const [error, setError] = useState<string | null>(null);
  const [workspaceId, setWorkspaceId] = useState<string | null>(null);

  useEffect(() => {
    void ensureDefaultWorkspace()
      .then((workspace) => setWorkspaceId(workspace.id))
      .catch((err) => setError(toDisplayMessage(err)));
  }, []);

  const handleSubmit = async (values: GoalFormState) => {
    if (!workspaceId) return;
    try {
      const goal = await createGoal({
        workspace_id: workspaceId,
        year_theme_id: null,
        title: values.title,
        description_markdown: values.description_markdown || null,
        why_it_matters_markdown: values.why_it_matters_markdown || null,
        success_definition_markdown: values.success_definition_markdown || null,
      });
      navigate(`/goals/${goal.id}`);
    } catch (err) {
      setError(toDisplayMessage(err));
    }
  };

  return (
    <PageContainer>
      <Breadcrumb items={[{ label: t("goals.title"), to: "/goals" }, { label: t("goals.new") }]} />
      <h1 className="text-2xl font-semibold">{t("goals.new")}</h1>
      {error ? <p className="joi-text--danger mt-2 text-sm">{error}</p> : null}
      <div className="mt-4">
        <GoalForm onSubmit={handleSubmit} submitLabel={t("goals.form.createSubmit")} />
      </div>
    </PageContainer>
  );
}

export default NewGoalPage;
