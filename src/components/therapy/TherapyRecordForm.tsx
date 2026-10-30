import { useState } from "react";
import { useTranslation } from "react-i18next";
import MarkdownField, { useMarkdownEditorChrome } from "../markdown/MarkdownField";
import type { TherapyRecordStatus, TherapyRecordType } from "../../lib/types";
import { THERAPY_RECORD_STATUSES, THERAPY_RECORD_TYPES } from "../../lib/types";

export interface TherapyRecordFormState {
  record_type: TherapyRecordType;
  title: string;
  occurred_on: string;
  content_markdown: string;
  status: TherapyRecordStatus;
}

interface TherapyRecordFormProps {
  initial?: Partial<TherapyRecordFormState>;
  isEdit?: boolean;
  onSubmit: (values: TherapyRecordFormState) => void | Promise<void>;
  submitLabel?: string;
  disabled?: boolean;
}

const EMPTY: TherapyRecordFormState = {
  record_type: "note",
  title: "",
  occurred_on: "",
  content_markdown: "",
  status: "open",
};

function TherapyRecordForm({
  initial,
  isEdit = false,
  onSubmit,
  submitLabel,
  disabled = false,
}: TherapyRecordFormProps) {
  const { t } = useTranslation();
  const [values, setValues] = useState<TherapyRecordFormState>({ ...EMPTY, ...initial });
  const [submitting, setSubmitting] = useState(false);

  const set = <K extends keyof TherapyRecordFormState>(
    key: K,
    value: TherapyRecordFormState[K],
  ) => {
    setValues((prev) => ({ ...prev, [key]: value }));
  };

  const handleSubmit = async (event: React.FormEvent) => {
    event.preventDefault();
    if (!values.title.trim()) {
      return;
    }
    setSubmitting(true);
    try {
      // 8.6.1: therapy Markdown is stored verbatim; the previous heading
      // normalization was removed so edit/save round-trips the exact source.
      await onSubmit(values);
    } finally {
      setSubmitting(false);
    }
  };

  const inputClass = "joi-field__input";
  const selectClass = "joi-field__select";
  const labelClass = "joi-field__label mb-1 block";

  // Shared Markdown preview chrome and editor toolbar chrome (8.6.10): this
  // screen group is localized, so its MarkdownField instance receives the
  // catalog copy; the component defaults remain untouched for other callers.
  const contentLabel = t("therapy.form.content");
  const editorChrome = useMarkdownEditorChrome();

  return (
    <form onSubmit={handleSubmit} className="flex flex-col gap-4">
      <div className="grid grid-cols-1 gap-4 sm:grid-cols-2">
        <div>
          <label htmlFor="record-type" className={labelClass}>
            {t("therapy.form.type")}
          </label>
          <select
            id="record-type"
            className={selectClass}
            value={values.record_type}
            onChange={(e) => set("record_type", e.target.value as TherapyRecordType)}
          >
            {THERAPY_RECORD_TYPES.map((value) => (
              <option key={value} value={value}>
                {t(`therapy.recordTypes.${value}`)}
              </option>
            ))}
          </select>
        </div>

        {isEdit ? (
          <div>
            <label htmlFor="record-status" className={labelClass}>
              {t("common.fields.status")}
            </label>
            <select
              id="record-status"
              className={selectClass}
              value={values.status}
              onChange={(e) => set("status", e.target.value as TherapyRecordStatus)}
            >
              {THERAPY_RECORD_STATUSES.map((value) => (
                <option key={value} value={value}>
                  {t(`therapy.recordStatuses.${value}`)}
                </option>
              ))}
            </select>
          </div>
        ) : null}
      </div>

      <div>
        <label htmlFor="record-title" className={labelClass}>
          {t("common.fields.title")}
        </label>
        <input
          id="record-title"
          className={inputClass}
          value={values.title}
          onChange={(e) => set("title", e.target.value)}
          required
        />
      </div>

      <div>
        <label htmlFor="record-occurred-on" className={labelClass}>
          {t("therapy.form.occurredOn")}
        </label>
        <input
          id="record-occurred-on"
          type="date"
          className={inputClass}
          value={values.occurred_on}
          onChange={(e) => set("occurred_on", e.target.value)}
        />
      </div>

      <MarkdownField
        id="record-content"
        label={contentLabel}
        value={values.content_markdown}
        onChange={(value) => set("content_markdown", value)}
        minHeight={300}
        maxHeight={600}
        previewLabel={t("common.markdown.preview")}
        previewRegionLabel={t("common.markdown.previewRegion", { label: contentLabel })}
        previewEmptyText={t("common.markdown.empty")}
        editorChrome={editorChrome}
      />

      <button
        type="submit"
        disabled={submitting || disabled}
        className="joi-btn joi-btn--primary px-4 py-2"
      >
        {submitLabel ?? t("therapy.form.saveSubmit")}
      </button>
    </form>
  );
}

export default TherapyRecordForm;
