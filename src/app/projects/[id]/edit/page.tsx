import { useEffect, useState } from "react";
import { useNavigate, useParams } from "react-router";
import { useTranslation } from "react-i18next";
import ProjectForm from "@/components/projects/ProjectForm";
import Breadcrumb from "@/components/navigation/Breadcrumb";
import PageContainer from "@/components/navigation/PageContainer";
import PageSkeleton from "@/components/navigation/PageSkeleton";
import { getProject, updateProject } from "@/lib/api/projectApi";
import { toDisplayMessage } from "@/lib/errors";
import type { Project } from "@/lib/types";

function EditProjectPage() {
  const { t } = useTranslation();
  const { id } = useParams<{ id: string }>();
  const navigate = useNavigate();
  const [project, setProject] = useState<Project | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (!id) return;
    void getProject(id)
      .then(setProject)
      .catch((err) => setError(toDisplayMessage(err)));
  }, [id]);

  const handleSubmit = async (values: {
    title: string;
    description_markdown: string;
    why_it_matters_markdown: string;
    definition_of_done_markdown: string;
    status: string;
    priority: string;
    target_date: string;
  }) => {
    if (!id) return;
    try {
      const updated = await updateProject(id, {
        title: values.title,
        description_markdown: values.description_markdown || null,
        why_it_matters_markdown: values.why_it_matters_markdown || null,
        definition_of_done_markdown: values.definition_of_done_markdown || null,
        status: values.status,
        priority: values.priority,
        target_date: values.target_date || null,
      });
      navigate(`/projects/${updated.id}`);
    } catch (err) {
      setError(toDisplayMessage(err));
    }
  };

  return (
    <PageContainer>
      <Breadcrumb
        items={[
          { label: t("projects.title"), to: "/projects" },
          { label: t("projects.detailTitle"), to: `/projects/${id}` },
          { label: t("projects.editTitle") },
        ]}
      />
      <h1 className="text-2xl font-semibold">{t("projects.editTitle")}</h1>
      {error ? <p className="joi-text--danger mt-2 text-sm">{error}</p> : null}
      {project ? (
        <div className="mt-4">
          <ProjectForm
            initial={{
              title: project.title,
              description_markdown: project.description_markdown ?? "",
              why_it_matters_markdown: project.why_it_matters_markdown ?? "",
              definition_of_done_markdown: project.definition_of_done_markdown ?? "",
              status: project.status,
              priority: project.priority,
              target_date: project.target_date ?? "",
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

export default EditProjectPage;
