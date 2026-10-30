import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import {
  addJournalReference,
  listReferencesForEntry,
  removeJournalReference,
} from "../../lib/api/journalReferenceApi";
import { toDisplayMessage } from "../../lib/errors";
import type {
  JournalReferenceEntityType,
  JournalReferenceRelation,
  JournalTaskReference,
} from "../../lib/types";
import { JOURNAL_REFERENCE_ENTITY_TYPES, JOURNAL_REFERENCE_RELATIONS } from "../../lib/types";

interface JournalReferenceEditorProps {
  entryId: string;
  workspaceId: string;
}

function JournalReferenceEditor({ entryId }: JournalReferenceEditorProps) {
  const { t } = useTranslation();
  const [references, setReferences] = useState<JournalTaskReference[]>([]);
  const [entityType, setEntityType] = useState<JournalReferenceEntityType>("task");
  const [entityId, setEntityId] = useState("");
  const [relation, setRelation] = useState<JournalReferenceRelation>("mentions");
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);
  const [showForm, setShowForm] = useState(false);

  const reload = async () => {
    setError(null);
    try {
      const refs = await listReferencesForEntry(entryId);
      setReferences(refs);
    } catch (err) {
      setError(toDisplayMessage(err));
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    void reload();
  }, [entryId]);

  const handleAdd = async () => {
    if (!entityId.trim()) return;
    setError(null);
    try {
      await addJournalReference(entryId, entityType, entityId.trim(), relation);
      setEntityId("");
      await reload();
    } catch (err) {
      setError(toDisplayMessage(err));
    }
  };

  const handleRemove = async (referenceId: string) => {
    setError(null);
    try {
      await removeJournalReference(referenceId);
      await reload();
    } catch (err) {
      setError(toDisplayMessage(err));
    }
  };

  if (loading) {
    return <p className="text-sm joi-text--subtle">{t("journal.reference.loading")}</p>;
  }

  const inputClass = "joi-field__input";
  const selectClass = "joi-field__select";

  return (
    <div className="flex flex-col gap-3">
      <button
        type="button"
        onClick={() => setShowForm((open) => !open)}
        aria-expanded={showForm}
        aria-controls="journal-reference-form"
        className="joi-btn joi-btn--secondary self-start px-3 py-1.5"
      >
        {t(showForm ? "common.actions.cancel" : "common.actions.add")}
      </button>

      {showForm ? (
        <div id="journal-reference-form" className="flex flex-wrap gap-2">
          <select
            aria-label={t("journal.reference.entityTypeAria")}
            value={entityType}
            onChange={(e) => setEntityType(e.target.value as JournalReferenceEntityType)}
            className={selectClass}
          >
            {JOURNAL_REFERENCE_ENTITY_TYPES.map((value) => (
              <option key={value} value={value}>
                {t(`journal.reference.entityTypes.${value}`)}
              </option>
            ))}
          </select>
          <input
            aria-label={t("journal.reference.entityIdAria")}
            className={inputClass}
            value={entityId}
            onChange={(e) => setEntityId(e.target.value)}
            placeholder={t("journal.reference.entityIdPlaceholder")}
          />
          <select
            aria-label={t("journal.reference.relationAria")}
            value={relation}
            onChange={(e) => setRelation(e.target.value as JournalReferenceRelation)}
            className={selectClass}
          >
            {JOURNAL_REFERENCE_RELATIONS.map((value) => (
              <option key={value} value={value}>
                {t(`journal.reference.relations.${value}`)}
              </option>
            ))}
          </select>
          <button
            type="button"
            onClick={handleAdd}
            disabled={!entityId.trim()}
            className="joi-btn joi-btn--primary"
          >
            {t("common.actions.add")}
          </button>
        </div>
      ) : null}

      {error ? <p className="text-sm joi-text--danger">{error}</p> : null}

      <ul className="flex flex-col gap-1">
        {references.map((reference) => (
          <li
            key={reference.id}
            className="joi-surface--raised flex items-center justify-between rounded-md px-3 py-2 text-sm"
          >
            <div className="flex min-w-0 items-center gap-2">
              <span className="joi-badge joi-badge--neutral shrink-0">
                {t(`journal.reference.entityTypes.${reference.entity_type}`)}
              </span>
              <span className="truncate font-mono joi-text">{reference.entity_id}</span>
              <span className="shrink-0 text-xs joi-text--muted">
                {t(`journal.reference.relations.${reference.relation}`)}
              </span>
            </div>
            <button
              type="button"
              onClick={() => handleRemove(reference.id)}
              className="joi-text--danger text-xs font-medium hover:underline"
            >
              {t("common.actions.remove")}
            </button>
          </li>
        ))}
        {references.length === 0 ? (
          <li className="joi-empty__state">{t("journal.reference.empty")}</li>
        ) : null}
      </ul>
    </div>
  );
}

export default JournalReferenceEditor;
