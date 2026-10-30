import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import type { TemplateManifest } from "../../lib/types";
import { exportTemplateCatalog } from "../../lib/api/exportApi";
import MarkdownField, { useMarkdownEditorChrome } from "../markdown/MarkdownField";
import TemplateFieldRenderer from "./TemplateFieldRenderer";

const EDITOR_MIN_HEIGHT = 360;
const EDITOR_MAX_HEIGHT = 720;

const NON_EDITABLE_TEMPLATE_FIELD_IDS = [
  "entry.free",
  "entry.emotions",
  "entry.mood",
  "entry.energy",
];

/**
 * Parses the free-text Emotions field into the entity's ordered list:
 * split on commas, trim each token, discard empty tokens, preserve order.
 * No taxonomy is imposed; any user-entered value is kept verbatim.
 */
export function parseEmotionList(text: string): string[] {
  return text
    .split(",")
    .map((token) => token.trim())
    .filter((token) => token.length > 0);
}

export interface JournalFormState {
  title: string;
  occurred_on: string;
  content_markdown: string;
  template_id: string;
  // Exact manifest version paired with template_id. Template identity is a
  // consistent pair (8.6.3 contract): entries with no recorded version keep
  // this empty and are sent as an absent pair, never inferred.
  template_version: string;
  mood_label: string;
  energy_label: string;
  emotions: string[];
  export_eligible: boolean;
  template_values: Record<string, string>;
}

interface JournalFormProps {
  initial?: Partial<JournalFormState>;
  onSubmit: (values: JournalFormState) => void | Promise<void>;
  submitLabel?: string;
  disabled?: boolean;
}

const EMPTY: JournalFormState = {
  title: "",
  occurred_on: "",
  content_markdown: "",
  template_id: "foundation-journal",
  template_version: "1.0.1",
  mood_label: "",
  energy_label: "",
  emotions: [],
  export_eligible: true,
  template_values: {},
};

function JournalForm({ initial, onSubmit, submitLabel, disabled = false }: JournalFormProps) {
  const { t } = useTranslation();
  const [values, setValues] = useState<JournalFormState>({ ...EMPTY, ...initial });
  // The Emotions control is a single free-text comma-separated field; the
  // canonical ordered list is derived from it on submit.
  const [emotionsText, setEmotionsText] = useState((initial?.emotions ?? []).join(", "));
  const [submitting, setSubmitting] = useState(false);
  const [templates, setTemplates] = useState<TemplateManifest[]>([]);

  useEffect(() => {
    void exportTemplateCatalog()
      .then((loaded) => {
        setTemplates(loaded);
        // Template-backed creates must send the exact template_id + manifest
        // template_version pair, so align the default selection with the
        // loaded manifest. This runs only for fresh forms: an entry's
        // recorded pair is kept exactly as stored and a missing version is
        // never inferred.
        if (initial?.template_id !== undefined) return;
        setValues((prev) => {
          const manifest = loaded.find((template) => template.id === prev.template_id);
          return manifest ? { ...prev, template_version: manifest.version } : prev;
        });
      })
      .catch(() => setTemplates([]));
  }, [initial?.template_id]);

  const activeTemplate =
    templates.find((template) => template.id === values.template_id) ?? templates[0];
  // Template identity is a consistent pair. While either half is missing (a
  // masked legacy-unversioned edit), the rendered manifest fields come only
  // from the catalog fallback, so their `required` constraints must not be
  // enforced. Free writing is always captured by content_markdown.
  const templatePairComplete = values.template_id !== "" && values.template_version !== "";

  const set = <K extends keyof JournalFormState>(key: K, value: JournalFormState[K]) => {
    setValues((prev) => ({ ...prev, [key]: value }));
  };

  const handleSubmit = async (event: React.FormEvent) => {
    event.preventDefault();
    if (!values.title.trim() || !values.occurred_on) {
      return;
    }
    setSubmitting(true);
    try {
      // 8.6.4 round-trip contract: `content_markdown` is submitted verbatim.
      // Heading normalization and `joi:*` marker materialization are removed;
      // emotions, mood, energy, export eligibility, and template values are
      // sent as structured fields separately from the Markdown prose.
      await onSubmit({ ...values, emotions: parseEmotionList(emotionsText) });
    } finally {
      setSubmitting(false);
    }
  };

  const inputClass = "joi-field__input";
  const selectClass = "joi-field__select";
  const textareaClass = "joi-field__textarea";
  const labelClass = "mb-1 block joi-field__label";

  // Shared Markdown preview chrome and editor toolbar chrome (8.6.10): this
  // screen group is localized, so its MarkdownField instance receives the
  // catalog copy; the component defaults remain untouched for other callers.
  const contentLabel = t("journal.form.content");
  const previewLabel = t("common.markdown.preview");
  const previewEmptyText = t("common.markdown.empty");
  const previewRegionLabel = t("common.markdown.previewRegion", { label: contentLabel });
  const editorChrome = useMarkdownEditorChrome();

  return (
    <form onSubmit={handleSubmit} className="flex flex-col gap-4">
      <div>
        <label htmlFor="journal-title" className={labelClass}>
          {t("common.fields.title")}
        </label>
        <input
          id="journal-title"
          className={inputClass}
          value={values.title}
          onChange={(e) => set("title", e.target.value)}
          required
        />
      </div>

      <div>
        <label htmlFor="journal-occurred-on" className={labelClass}>
          {t("journal.form.date")}
        </label>
        <input
          id="journal-occurred-on"
          type="date"
          className={inputClass}
          value={values.occurred_on}
          onChange={(e) => set("occurred_on", e.target.value)}
          required
        />
      </div>

      <MarkdownField
        id="journal-content"
        label={contentLabel}
        value={values.content_markdown}
        onChange={(value) => set("content_markdown", value)}
        minHeight={EDITOR_MIN_HEIGHT}
        maxHeight={EDITOR_MAX_HEIGHT}
        previewLabel={previewLabel}
        previewRegionLabel={previewRegionLabel}
        previewEmptyText={previewEmptyText}
        editorChrome={editorChrome}
      />

      <div className="grid grid-cols-1 gap-4 sm:grid-cols-2">
        <div>
          <label htmlFor="journal-mood" className={labelClass}>
            {t("journal.form.mood")}
          </label>
          <input
            id="journal-mood"
            className={inputClass}
            value={values.mood_label}
            onChange={(e) => set("mood_label", e.target.value)}
            placeholder={t("journal.form.moodPlaceholder")}
          />
        </div>
        <div>
          <label htmlFor="journal-energy" className={labelClass}>
            {t("journal.form.energy")}
          </label>
          <input
            id="journal-energy"
            className={inputClass}
            value={values.energy_label}
            onChange={(e) => set("energy_label", e.target.value)}
            placeholder={t("journal.form.energyPlaceholder")}
          />
        </div>
      </div>

      <div>
        <label htmlFor="journal-emotions" className={labelClass}>
          {t("journal.form.emotions")}
        </label>
        <input
          id="journal-emotions"
          className={inputClass}
          type="text"
          value={emotionsText}
          onChange={(e) => setEmotionsText(e.target.value)}
          placeholder={t("journal.form.emotionsPlaceholder")}
        />
        <p className="joi-field__help mt-1">{t("journal.form.emotionsHelp")}</p>
      </div>

      {activeTemplate?.fields
        .filter((field) => !NON_EDITABLE_TEMPLATE_FIELD_IDS.includes(field.id))
        .map((field) => (
          <TemplateFieldRenderer
            key={field.id}
            field={field}
            value={values.template_values[field.id] ?? ""}
            onChange={(value) =>
              set("template_values", {
                ...values.template_values,
                [field.id]: value,
              })
            }
            enforceRequired={templatePairComplete}
            inputClass={inputClass}
            selectClass={selectClass}
            textareaClass={textareaClass}
            labelClass={labelClass}
          />
        ))}

      <div>
        <label htmlFor="journal-template" className={labelClass}>
          {t("journal.form.template")}
        </label>
        <select
          id="journal-template"
          className={selectClass}
          value={values.template_id}
          onChange={(e) => {
            // Template identity is the exact manifest pair; switching the
            // template always updates both halves together.
            const id = e.target.value;
            const manifest = templates.find((template) => template.id === id);
            setValues((prev) => ({
              ...prev,
              template_id: id,
              template_version: manifest?.version ?? prev.template_version,
            }));
          }}
        >
          {templates.length > 0 ? (
            templates.map((template) => (
              <option key={`${template.id}@${template.version}`} value={template.id}>
                {template.display_name} ({template.version})
              </option>
            ))
          ) : (
            // Catalog-unavailable fallback label: application-owned chrome,
            // translated from the journal namespace. The loaded-manifest
            // branch above keeps template display names verbatim template
            // data, and this fallback's option value/version stay pinned to
            // the bundled foundation-journal identity (8.6.7 rule).
            <option value="foundation-journal">{t("journal.form.templateFallbackOption")}</option>
          )}
        </select>
      </div>

      <div>
        <div className="flex items-center gap-2">
          <input
            id="journal-exportable"
            type="checkbox"
            className="joi-checkbox h-4 w-4"
            checked={values.export_eligible}
            onChange={(e) => set("export_eligible", e.target.checked)}
          />
          <label htmlFor="journal-exportable" className="joi-field__label">
            {t("journal.exportable")}
          </label>
        </div>
        <p className="joi-field__help mt-1">{t("journal.form.exportableHelp")}</p>
      </div>

      <button type="submit" disabled={submitting || disabled} className="joi-btn joi-btn--primary">
        {submitLabel ?? t("journal.form.saveSubmit")}
      </button>
    </form>
  );
}

export default JournalForm;
