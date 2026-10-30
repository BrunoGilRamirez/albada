import { useTranslation } from "react-i18next";
import type { TemplateManifestField } from "../../lib/types";

interface TemplateFieldRendererProps {
  field: TemplateManifestField;
  value: string;
  onChange: (value: string) => void;
  // When false, the owning form's template pair is absent (a masked
  // legacy-unversioned edit): the rendered manifest comes from the catalog
  // fallback, so manifest `required` constraints are not enforced. Deliberate
  // template selection keeps the exact complete pair and enforces them.
  enforceRequired?: boolean;
  inputClass: string;
  selectClass: string;
  textareaClass: string;
  labelClass: string;
}

/**
 * Manifest field IDs whose label is application-owned UI chrome translatable
 * through the `journal.templateField` catalog keys. Unknown manifest field IDs
 * keep the ID-derived fallback label below: template identity, values, and
 * template-defined text are never translated or rewritten.
 */
function labelForField(id: string): string {
  const parts = id.split(".");
  const label = parts[parts.length - 1] ?? id;
  return label.replace(/_/g, " ").replace(/\b\w/g, (letter: string) => letter.toUpperCase());
}

/** Render a declarative template field without template-specific components. */
export default function TemplateFieldRenderer({
  field,
  value,
  onChange,
  enforceRequired = true,
  inputClass,
  selectClass,
  textareaClass,
  labelClass,
}: TemplateFieldRendererProps) {
  const { t } = useTranslation();
  if (
    field.id === "entry.free" ||
    field.id === "entry.mood" ||
    field.id === "entry.energy" ||
    field.id === "entry.emotions"
  ) {
    return null;
  }

  const label = labelForField(field.id);
  const selectPlaceholder = t("journal.templateField.select");
  const inputId = `journal-template-field-${field.id.replace(/\./g, "-")}`;

  if (field.field_type === "single_select") {
    return (
      <div>
        <label htmlFor={inputId} className={labelClass}>
          {label}
        </label>
        <select
          id={inputId}
          className={selectClass}
          value={value}
          onChange={(event) => onChange(event.target.value)}
          required={field.required && enforceRequired}
        >
          <option value="">{selectPlaceholder}</option>
          {(field.options ?? []).map((option) => (
            <option key={option} value={option}>
              {option}
            </option>
          ))}
        </select>
      </div>
    );
  }

  if (field.field_type === "multi_select") {
    const selected = new Set(
      value
        .split(",")
        .map((item) => item.trim())
        .filter(Boolean),
    );
    return (
      <fieldset>
        <legend className={labelClass}>{label}</legend>
        <div className="flex flex-wrap gap-3">
          {(field.options ?? []).map((option) => (
            <label key={option} className="flex items-center gap-1 text-sm">
              <input
                type="checkbox"
                className="joi-checkbox"
                checked={selected.has(option)}
                onChange={(event) => {
                  const next = new Set(selected);
                  if (event.target.checked) next.add(option);
                  else next.delete(option);
                  onChange(Array.from(next).join(", "));
                }}
              />
              {option}
            </label>
          ))}
        </div>
      </fieldset>
    );
  }

  if (field.field_type === "bounded_scale") {
    return (
      <div>
        <label htmlFor={inputId} className={labelClass}>
          {label}
        </label>
        <input
          id={inputId}
          type="number"
          min={1}
          max={10}
          className={inputClass}
          value={value}
          onChange={(event) => onChange(event.target.value)}
          required={field.required && enforceRequired}
        />
      </div>
    );
  }

  return (
    <div>
      <label htmlFor={inputId} className={labelClass}>
        {label}
      </label>
      <textarea
        id={inputId}
        className={`${textareaClass} min-h-20`}
        value={value}
        onChange={(event) => onChange(event.target.value)}
        required={field.required && enforceRequired}
      />
    </div>
  );
}
