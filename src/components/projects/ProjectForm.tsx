import { useState } from "react";
import { useTranslation } from "react-i18next";
import type { Priority, ProjectStatus } from "../../lib/types";
import { PRIORITIES, PROJECT_STATUSES } from "../../lib/types";
import MarkdownField, { useMarkdownEditorChrome } from "../markdown/MarkdownField";

export interface ProjectFormState {
  title: string;
  description_markdown: string;
  why_it_matters_markdown: string;
  definition_of_done_markdown: string;
  status: ProjectStatus;
  priority: Priority;
  target_date: string;
}

interface ProjectFormProps {
  initial?: Partial<ProjectFormState>;
  editableManagement?: boolean;
  onSubmit: (values: ProjectFormState) => void | Promise<void>;
  submitLabel?: string;
}

const EMPTY: ProjectFormState = {
  title: "",
  description_markdown: "",
  why_it_matters_markdown: "",
  definition_of_done_markdown: "",
  status: "inbox",
  priority: "none",
  target_date: "",
};

function ProjectForm({
  initial,
  editableManagement = false,
  onSubmit,
  submitLabel,
}: ProjectFormProps) {
  const { t } = useTranslation();
  const [values, setValues] = useState<ProjectFormState>({ ...EMPTY, ...initial });
  const [submitting, setSubmitting] = useState(false);

  const set = <K extends keyof ProjectFormState>(key: K, value: ProjectFormState[K]) => {
    setValues((prev) => ({ ...prev, [key]: value }));
  };

  const handleSubmit = async (event: React.FormEvent) => {
    event.preventDefault();
    if (!values.title.trim()) return;
    setSubmitting(true);
    try {
      await onSubmit(values);
    } finally {
      setSubmitting(false);
    }
  };

  const inputClass = "joi-field__input";
  const selectClass = "joi-field__select";
  const labelClass = "joi-field__label mb-1 block";

  // Shared Markdown preview chrome and editor toolbar chrome (8.6.9): this
  // screen group is localized, so its MarkdownField instances receive the
  // catalog copy; out-of-scope surfaces keep the components' English defaults
  // by not passing these.
  const previewLabel = t("common.markdown.preview");
  const previewEmptyText = t("common.markdown.empty");
  const previewRegionLabel = (label: string) => t("common.markdown.previewRegion", { label });
  const editorChrome = useMarkdownEditorChrome();
  const descriptionLabel = t("projects.form.description");
  const whyLabel = t("projects.form.why");
  const dodLabel = t("projects.form.dod");

  return (
    <form onSubmit={handleSubmit} className="flex flex-col gap-4">
      <div>
        <label htmlFor="project-title" className={labelClass}>
          {t("common.fields.title")}
        </label>
        <input
          id="project-title"
          className={inputClass}
          value={values.title}
          onChange={(e) => set("title", e.target.value)}
          required
        />
      </div>

      <MarkdownField
        id="project-description"
        label={descriptionLabel}
        value={values.description_markdown}
        onChange={(value) => set("description_markdown", value)}
        previewLabel={previewLabel}
        previewRegionLabel={previewRegionLabel(descriptionLabel)}
        previewEmptyText={previewEmptyText}
        editorChrome={editorChrome}
      />

      <MarkdownField
        id="project-why"
        label={whyLabel}
        value={values.why_it_matters_markdown}
        onChange={(value) => set("why_it_matters_markdown", value)}
        rows={3}
        previewLabel={previewLabel}
        previewRegionLabel={previewRegionLabel(whyLabel)}
        previewEmptyText={previewEmptyText}
        editorChrome={editorChrome}
      />

      <MarkdownField
        id="project-dod"
        label={dodLabel}
        value={values.definition_of_done_markdown}
        onChange={(value) => set("definition_of_done_markdown", value)}
        rows={3}
        previewLabel={previewLabel}
        previewRegionLabel={previewRegionLabel(dodLabel)}
        previewEmptyText={previewEmptyText}
        editorChrome={editorChrome}
      />

      {editableManagement ? (
        <div className="grid grid-cols-1 gap-4 sm:grid-cols-3">
          <div>
            <label htmlFor="project-status" className={labelClass}>
              {t("common.fields.status")}
            </label>
            <select
              id="project-status"
              className={selectClass}
              value={values.status}
              onChange={(e) => set("status", e.target.value as ProjectStatus)}
            >
              {PROJECT_STATUSES.map((status) => (
                <option key={status} value={status}>
                  {t(`common.labels.projectStatus.${status}`)}
                </option>
              ))}
            </select>
          </div>
          <div>
            <label htmlFor="project-priority" className={labelClass}>
              {t("common.fields.priority")}
            </label>
            <select
              id="project-priority"
              className={selectClass}
              value={values.priority}
              onChange={(e) => set("priority", e.target.value as Priority)}
            >
              {PRIORITIES.map((value) => (
                <option key={value} value={value}>
                  {t(`common.labels.priority.${value}`)}
                </option>
              ))}
            </select>
          </div>
          <div>
            <label htmlFor="project-target-date" className={labelClass}>
              {t("common.fields.targetDate")}
            </label>
            <input
              id="project-target-date"
              type="date"
              className={inputClass}
              value={values.target_date}
              onChange={(e) => set("target_date", e.target.value)}
            />
          </div>
        </div>
      ) : null}

      <button type="submit" disabled={submitting} className="joi-btn joi-btn--primary px-4 py-2">
        {submitLabel ?? t("projects.form.submit")}
      </button>
    </form>
  );
}

export default ProjectForm;
