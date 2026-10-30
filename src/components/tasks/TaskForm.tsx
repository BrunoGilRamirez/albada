import { useState } from "react";
import { useTranslation } from "react-i18next";
import type { EffortSize, Priority } from "../../lib/types";
import { EFFORT_SIZES, PRIORITIES } from "../../lib/types";
import MarkdownField, { useMarkdownEditorChrome } from "../markdown/MarkdownField";
import PriorityEffortMatrix from "./PriorityEffortMatrix";

export interface TaskFormState {
  title: string;
  description_markdown: string;
  notes_markdown: string;
  priority: Priority;
  effort_size: EffortSize | null;
  target_date: string;
  scheduled_date: string;
}

interface TaskFormProps {
  initial?: Partial<TaskFormState>;
  onSubmit: (values: TaskFormState) => void | Promise<void>;
  submitLabel?: string;
  disabled?: boolean;
}

const EMPTY: TaskFormState = {
  title: "",
  description_markdown: "",
  notes_markdown: "",
  priority: "normal",
  effort_size: null,
  target_date: "",
  scheduled_date: "",
};

function TaskForm({ initial, onSubmit, submitLabel, disabled = false }: TaskFormProps) {
  const { t } = useTranslation();
  const [values, setValues] = useState<TaskFormState>({ ...EMPTY, ...initial });
  const [submitting, setSubmitting] = useState(false);

  const set = <K extends keyof TaskFormState>(key: K, value: TaskFormState[K]) => {
    setValues((prev) => ({ ...prev, [key]: value }));
  };

  const handleMatrixSelect = (effort: EffortSize, priority: Priority) => {
    setValues((prev) => ({ ...prev, effort_size: effort, priority }));
  };

  const handleSubmit = async (event: React.FormEvent) => {
    event.preventDefault();
    if (!values.title.trim()) {
      return;
    }
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
  const descriptionLabel = t("tasks.form.description");
  const notesLabel = t("tasks.form.notes");

  return (
    <form onSubmit={handleSubmit} className="flex flex-col gap-4">
      <div>
        <label htmlFor="task-title" className={labelClass}>
          {t("common.fields.title")}
        </label>
        <input
          id="task-title"
          className={inputClass}
          value={values.title}
          onChange={(e) => set("title", e.target.value)}
          required
        />
      </div>

      <MarkdownField
        id="task-description"
        label={descriptionLabel}
        value={values.description_markdown}
        onChange={(value) => set("description_markdown", value)}
        previewLabel={previewLabel}
        previewRegionLabel={previewRegionLabel(descriptionLabel)}
        previewEmptyText={previewEmptyText}
        editorChrome={editorChrome}
      />

      <MarkdownField
        id="task-notes"
        label={notesLabel}
        value={values.notes_markdown}
        onChange={(value) => set("notes_markdown", value)}
        rows={4}
        previewLabel={previewLabel}
        previewRegionLabel={previewRegionLabel(notesLabel)}
        previewEmptyText={previewEmptyText}
        editorChrome={editorChrome}
      />

      <div className="grid grid-cols-1 gap-4 sm:grid-cols-3">
        <div>
          <label htmlFor="task-priority" className={labelClass}>
            {t("common.fields.priority")}
          </label>
          <select
            id="task-priority"
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
          <label htmlFor="task-effort" className={labelClass}>
            {t("tasks.effortSize")}
          </label>
          <select
            id="task-effort"
            className={selectClass}
            value={values.effort_size ?? ""}
            onChange={(e) =>
              set("effort_size", e.target.value ? (e.target.value as EffortSize) : null)
            }
          >
            <option value="">{t("tasks.effortNotSet")}</option>
            {EFFORT_SIZES.map((value) => (
              <option key={value} value={value}>
                {t(`common.labels.effortSize.${value}`)}
              </option>
            ))}
          </select>
        </div>
        <div>
          <label htmlFor="task-target-date" className={labelClass}>
            {t("common.fields.targetDate")}
          </label>
          <input
            id="task-target-date"
            type="date"
            className={inputClass}
            value={values.target_date}
            onChange={(e) => set("target_date", e.target.value)}
          />
        </div>
      </div>

      <div>
        <label htmlFor="task-scheduled-date" className={labelClass}>
          {t("tasks.scheduledDate")}
        </label>
        <input
          id="task-scheduled-date"
          type="date"
          className={inputClass}
          value={values.scheduled_date}
          onChange={(e) => set("scheduled_date", e.target.value)}
        />
      </div>

      <PriorityEffortMatrix
        effort={values.effort_size}
        priority={values.priority}
        onSelect={handleMatrixSelect}
      />

      <button
        type="submit"
        disabled={submitting || disabled}
        className="joi-btn joi-btn--primary px-4 py-2"
      >
        {submitLabel ?? t("tasks.form.submit")}
      </button>
    </form>
  );
}

export default TaskForm;
