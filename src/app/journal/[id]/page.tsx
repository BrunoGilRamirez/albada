import { useEffect, useState } from "react";
import { Link, useNavigate, useParams } from "react-router";
import { useTranslation } from "react-i18next";
import JournalReferenceEditor from "@/components/journal/JournalReferenceEditor";
import MarkdownPreview from "@/components/markdown/MarkdownPreview";
import Breadcrumb from "@/components/navigation/Breadcrumb";
import PageContainer from "@/components/navigation/PageContainer";
import PageSkeleton from "@/components/navigation/PageSkeleton";
import { archiveJournalEntry, getJournalEntry, listJournalEntries } from "@/lib/api/journalApi";
import { toDisplayMessage } from "@/lib/errors";
import { formatDate } from "@/lib/format";
import type { JournalEntry } from "@/lib/types";

/**
 * The occurred-on date-only value is an application-owned date: the read-only
 * label renders through the shared Intl date formatter with a local-noon
 * anchor (the established 8.6.8/8.6.9 pattern). Stored values and the date
 * inputs keep the exact ISO string; authored content never passes through it.
 */
function formatEntryDate(occurredOn: string): string {
  return formatDate(new Date(`${occurredOn}T12:00:00`), { dateStyle: "medium" });
}

function JournalDetailPage() {
  const { t } = useTranslation();
  const { id } = useParams<{ id: string }>();
  const navigate = useNavigate();
  const [entry, setEntry] = useState<JournalEntry | null>(null);
  const [entries, setEntries] = useState<JournalEntry[]>([]);
  const [error, setError] = useState<string | null>(null);

  const reload = async () => {
    if (!id) return;
    setError(null);
    try {
      const data = await getJournalEntry(id);
      setEntry(data);
      void listJournalEntries(data.workspace_id)
        .then(setEntries)
        .catch((err) => setError(toDisplayMessage(err)));
    } catch (err) {
      setError(toDisplayMessage(err));
    }
  };

  useEffect(() => {
    void reload();
  }, [id]);

  const handleArchive = async () => {
    if (!id) return;
    try {
      await archiveJournalEntry(id);
      navigate("/journal");
    } catch (err) {
      setError(toDisplayMessage(err));
    }
  };

  if (!entry) {
    return (
      <section>
        <h1 className="text-2xl font-semibold">{t("journal.detailTitle")}</h1>
        {error ? <p className="mt-2 text-sm joi-text--danger">{error}</p> : <PageSkeleton />}
      </section>
    );
  }

  // Entries are listed newest first, so the previous index is a newer entry
  // and the next index is an older one.
  const currentIndex = entries.findIndex((e) => e.id === entry.id);
  const newer = currentIndex > 0 ? entries[currentIndex - 1] : null;
  const older =
    currentIndex >= 0 && currentIndex < entries.length - 1 ? entries[currentIndex + 1] : null;

  const navLinkClass = "joi-btn joi-btn--secondary";

  return (
    <PageContainer>
      <Breadcrumb items={[{ label: t("journal.title"), to: "/journal" }, { label: entry.title }]} />
      <div className="flex flex-wrap items-center gap-3">
        {/* Authored title renders verbatim; only application-owned chrome localizes. */}
        <h1 className="text-2xl font-semibold">{entry.title}</h1>
        <span className="joi-badge joi-badge--neutral">
          {entry.export_eligible ? t("journal.exportable") : t("journal.private")}
        </span>
      </div>
      <p className="mt-1 text-sm joi-text--muted">{formatEntryDate(entry.occurred_on)}</p>
      {entry.mood_label || entry.energy_label ? (
        <p className="mt-1 text-sm joi-text--muted">
          {[entry.mood_label, entry.energy_label].filter(Boolean).join(" · ")}
        </p>
      ) : null}
      {error ? <p className="mt-2 text-sm joi-text--danger">{error}</p> : null}

      <div className="mt-4 flex flex-wrap gap-2">
        <Link to={`/journal/${entry.id}/edit`} className="joi-btn joi-btn--primary">
          {t("common.actions.edit")}
        </Link>
        <button type="button" onClick={handleArchive} className="joi-btn joi-btn--secondary">
          {t("common.actions.archive")}
        </button>
      </div>

      <div className="mt-4">
        <h2 className="mb-1 text-sm font-medium joi-text">{t("common.labels.content")}</h2>
        <MarkdownPreview content={entry.content_markdown} emptyText={t("common.markdown.empty")} />
      </div>

      <div className="mt-6">
        <h2 className="mb-2 text-lg font-medium">{t("journal.linkedItems")}</h2>
        <JournalReferenceEditor entryId={entry.id} workspaceId={entry.workspace_id} />
      </div>

      <div className="joi-border--default mt-6 flex items-center justify-between border-t pt-4">
        {older ? (
          <Link to={`/journal/${older.id}`} className={navLinkClass}>
            {t("journal.olderEntry")}
          </Link>
        ) : (
          <span />
        )}
        {newer ? (
          <Link to={`/journal/${newer.id}`} className={navLinkClass}>
            {t("journal.newerEntry")}
          </Link>
        ) : (
          <span />
        )}
      </div>
    </PageContainer>
  );
}

export default JournalDetailPage;
