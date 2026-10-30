import { useState } from "react";
import { useTranslation } from "react-i18next";
import type { GoalStatus, Priority } from "../../lib/types";
import { GOAL_STATUSES, PRIORITIES } from "../../lib/types";
import MarkdownField, { useMarkdownEditorChrome } from "../markdown/MarkdownField";

export interface GoalFormState {
  title: string;
  description_markdown: string;
  why_it_matters_markdown: string;
  success_definition_markdown: string;
  status: GoalStatus;
  priority: Priority;
  target_date: string;
}

interface GoalFormProps {
  initial?: Partial<GoalFormState>;
  editableManagement?: boolean;
  onSubmit: (values: GoalFormState) => void | Promise<void>;
  submitLabel?: string;
}

const EMPTY: GoalFormState = {
  title: "",
  description_markdown: "",
  why_it_matters_markdown: "",
  success_definition_markdown: "",
  status: "inbox",
  priority: "none",
  target_date: "",
};

function GoalForm({ initial, editableManagement = false, onSubmit, submitLabel }: GoalFormProps) {
  const { t } = useTranslation();
  const [values, setValues] = useState<GoalFormState>({ ...EMPTY, ...initial });
  const [submitting, setSubmitting] = useState(false);

  const set = <K extends keyof GoalFormState>(key: K, value: GoalFormState[K]) => {
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
  const descriptionLabel = t("goals.form.description");
  const whyLabel = t("goals.form.why");
  const successLabel = t("goals.form.success");

  return (
    <form onSubmit={handleSubmit} className="flex flex-col gap-4">
      <div>
        <label htmlFor="goal-title" className={labelClass}>
          {t("common.fields.title")}
        </label>
        <input
          id="goal-title"
          className={inputClass}
          value={values.title}
          onChange={(e) => set("title", e.target.value)}
          required
        />
      </div>

      <MarkdownField
        id="goal-description"
        label={descriptionLabel}
        value={values.description_markdown}
        onChange={(value) => set("description_markdown", value)}
        previewLabel={previewLabel}
        previewRegionLabel={previewRegionLabel(descriptionLabel)}
        previewEmptyText={previewEmptyText}
        editorChrome={editorChrome}
      />

      <MarkdownField
        id="goal-why"
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
        id="goal-success"
        label={successLabel}
        value={values.success_definition_markdown}
        onChange={(value) => set("success_definition_markdown", value)}
        rows={3}
        previewLabel={previewLabel}
        previewRegionLabel={previewRegionLabel(successLabel)}
        previewEmptyText={previewEmptyText}
        editorChrome={editorChrome}
      />

      {editableManagement ? (
        <div className="grid grid-cols-1 gap-4 sm:grid-cols-3">
          <div>
            <label htmlFor="goal-status" className={labelClass}>
              {t("common.fields.status")}
            </label>
            <select
              id="goal-status"
              className={selectClass}
              value={values.status}
              onChange={(e) => set("status", e.target.value as GoalStatus)}
            >
              {GOAL_STATUSES.map((status) => (
                <option key={status} value={status}>
                  {t(`common.labels.goalStatus.${status}`)}
                </option>
              ))}
            </select>
          </div>
          <div>
            <label htmlFor="goal-priority" className={labelClass}>
              {t("common.fields.priority")}
            </label>
            <select
              id="goal-priority"
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
            <label htmlFor="goal-target-date" className={labelClass}>
              {t("common.fields.targetDate")}
            </label>
            <input
              id="goal-target-date"
              type="date"
              className={inputClass}
              value={values.target_date}
              onChange={(e) => set("target_date", e.target.value)}
            />
          </div>
        </div>
      ) : null}

      <button type="submit" disabled={submitting} className="joi-btn joi-btn--primary px-4 py-2">
        {submitLabel ?? t("goals.form.submit")}
      </button>
    </form>
  );
}

export default GoalForm;
