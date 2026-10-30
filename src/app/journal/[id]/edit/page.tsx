import { useEffect, useState } from "react";
import { useNavigate, useParams } from "react-router";
import { useTranslation } from "react-i18next";
import JournalForm, { type JournalFormState } from "@/components/journal/JournalForm";
import Breadcrumb from "@/components/navigation/Breadcrumb";
import PageContainer from "@/components/navigation/PageContainer";
import PageSkeleton from "@/components/navigation/PageSkeleton";
import { getJournalEntry, updateJournalEntry } from "@/lib/api/journalApi";
import type { JournalTemplateValues } from "@/lib/types";
import { toDisplayMessage } from "@/lib/errors";

const NON_EDITABLE_TEMPLATE_FIELD_IDS = [
  "entry.free",
  "entry.emotions",
  "entry.mood",
  "entry.energy",
];

/** The structured values the form renders and edits as template fields. */
function editableTemplateValues(values: JournalTemplateValues): Record<string, string> {
  const editable: Record<string, string> = {};
  for (const [fieldId, value] of Object.entries(values)) {
    if (NON_EDITABLE_TEMPLATE_FIELD_IDS.includes(fieldId) || typeof value !== "string") continue;
    editable[fieldId] = value;
  }
  return editable;
}

/** Flat structural equality for the loaded and submitted template-value maps. */
function sameTemplateValues(a: Record<string, string>, b: Record<string, string>): boolean {
  const aKeys = Object.keys(a);
  if (aKeys.length !== Object.keys(b).length) return false;
  return aKeys.every((key) => Object.prototype.hasOwnProperty.call(b, key) && b[key] === a[key]);
}

function EditJournalEntryPage() {
  const { t } = useTranslation();
  const { id } = useParams<{ id: string }>();
  const navigate = useNavigate();
  const [initial, setInitial] = useState<Partial<JournalFormState> | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (!id) return;
    void getJournalEntry(id)
      .then((entry) => {
        setInitial({
          title: entry.title,
          occurred_on: entry.occurred_on,
          content_markdown: entry.content_markdown,
          template_id: entry.template_id ?? "",
          // Keep the recorded pair exactly as stored; a masked
          // legacy-unversioned entry stays an absent pair (no inference).
          template_version: entry.template_version ?? "",
          mood_label: entry.mood_label ?? "",
          energy_label: entry.energy_label ?? "",
          emotions: entry.emotions,
          export_eligible: entry.export_eligible,
          template_values: editableTemplateValues(entry.template_values),
        });
      })
      .catch((err) => setError(toDisplayMessage(err)));
  }, [id]);

  const handleSubmit = async (values: JournalFormState) => {
    if (!id) return;
    const storedValues = initial?.template_values ?? {};
    const valuesChanged = !sameTemplateValues(values.template_values, storedValues);
    try {
      const entry = await updateJournalEntry(id, {
        occurred_on: values.occurred_on,
        title: values.title,
        content_markdown: values.content_markdown,
        template_id: values.template_id || null,
        template_version: values.template_version || null,
        // Keep hidden legacy values untouched when the edited form has no
        // editable template-specific fields.
        template_values: valuesChanged ? values.template_values : null,
        mood_label: values.mood_label || null,
        energy_label: values.energy_label || null,
        emotions: values.emotions,
        export_eligible: values.export_eligible,
      });
      navigate(`/journal/${entry.id}`);
    } catch (err) {
      setError(toDisplayMessage(err));
    }
  };

  if (!initial) {
    return (
      <section>
        <h1 className="text-2xl font-semibold">{t("journal.editTitle")}</h1>
        {error ? (
          <p className="mt-2 text-sm joi-text--danger">{error}</p>
        ) : (
          <PageSkeleton variant="form" />
        )}
      </section>
    );
  }

  return (
    <PageContainer>
      <Breadcrumb
        items={[
          { label: t("journal.title"), to: "/journal" },
          { label: t("journal.detailTitle"), to: `/journal/${id}` },
          { label: t("journal.editTitle") },
        ]}
      />
      <h1 className="text-2xl font-semibold">{t("journal.editTitle")}</h1>
      {error ? <p className="mt-2 text-sm joi-text--danger">{error}</p> : null}
      <div className="mt-4">
        <JournalForm
          initial={initial}
          onSubmit={handleSubmit}
          submitLabel={t("common.actions.saveChanges")}
        />
      </div>
    </PageContainer>
  );
}

export default EditJournalEntryPage;
