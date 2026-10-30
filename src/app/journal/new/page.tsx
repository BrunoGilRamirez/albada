import { useEffect, useState } from "react";
import { useNavigate } from "react-router";
import { useTranslation } from "react-i18next";
import JournalForm, { type JournalFormState } from "@/components/journal/JournalForm";
import Breadcrumb from "@/components/navigation/Breadcrumb";
import PageContainer from "@/components/navigation/PageContainer";
import { createJournalEntry } from "@/lib/api/journalApi";
import { ensureDefaultWorkspace } from "@/lib/api/workspaceApi";
import { toDisplayMessage } from "@/lib/errors";

function todayLocalISO(): string {
  const now = new Date();
  const year = now.getFullYear();
  const month = String(now.getMonth() + 1).padStart(2, "0");
  const day = String(now.getDate()).padStart(2, "0");
  return `${year}-${month}-${day}`;
}

function NewJournalEntryPage() {
  const { t } = useTranslation();
  const navigate = useNavigate();
  const [error, setError] = useState<string | null>(null);
  const [workspaceId, setWorkspaceId] = useState<string | null>(null);

  useEffect(() => {
    void ensureDefaultWorkspace()
      .then((workspace) => setWorkspaceId(workspace.id))
      .catch((err) => setError(toDisplayMessage(err)));
  }, []);

  const handleSubmit = async (values: JournalFormState) => {
    if (!workspaceId) return;
    try {
      const entry = await createJournalEntry({
        workspace_id: workspaceId,
        occurred_on: values.occurred_on,
        title: values.title,
        content_markdown: values.content_markdown,
        template_id: values.template_id || null,
        template_version: values.template_version || null,
        template_values: values.template_values,
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

  return (
    <PageContainer>
      <Breadcrumb
        items={[{ label: t("journal.title"), to: "/journal" }, { label: t("journal.newTitle") }]}
      />
      <h1 className="text-2xl font-semibold">{t("journal.newTitle")}</h1>
      {error ? <p className="mt-2 text-sm joi-text--danger">{error}</p> : null}
      <div className="mt-4">
        <JournalForm
          initial={{ occurred_on: todayLocalISO() }}
          onSubmit={handleSubmit}
          submitLabel={t("journal.form.createSubmit")}
        />
      </div>
    </PageContainer>
  );
}

export default NewJournalEntryPage;
