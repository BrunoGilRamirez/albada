import { useEffect, useState } from "react";
import { useNavigate, useSearchParams } from "react-router";
import { useTranslation } from "react-i18next";
import ProjectForm, { type ProjectFormState } from "@/components/projects/ProjectForm";
import Breadcrumb from "@/components/navigation/Breadcrumb";
import PageContainer from "@/components/navigation/PageContainer";
import { createProject } from "@/lib/api/projectApi";
import { ensureDefaultWorkspace } from "@/lib/api/workspaceApi";
import { toDisplayMessage } from "@/lib/errors";

function NewProjectPage() {
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

  const handleSubmit = async (values: ProjectFormState) => {
    if (!workspaceId) return;
    try {
      const project = await createProject({
        workspace_id: workspaceId,
        goal_id: goalId ?? null,
        title: values.title,
        description_markdown: values.description_markdown || null,
        why_it_matters_markdown: values.why_it_matters_markdown || null,
        definition_of_done_markdown: values.definition_of_done_markdown || null,
      });
      navigate(`/projects/${project.id}`);
    } catch (err) {
      setError(toDisplayMessage(err));
    }
  };

  return (
    <PageContainer>
      <Breadcrumb
        items={[{ label: t("projects.title"), to: "/projects" }, { label: t("projects.new") }]}
      />
      <h1 className="text-2xl font-semibold">{t("projects.new")}</h1>
      {goalId ? <p className="joi-page__subtitle mt-1">{t("projects.linkedToGoalNote")}</p> : null}
      {error ? <p className="joi-text--danger mt-2 text-sm">{error}</p> : null}
      <div className="mt-4">
        <ProjectForm onSubmit={handleSubmit} submitLabel={t("projects.form.createSubmit")} />
      </div>
    </PageContainer>
  );
}

export default NewProjectPage;
