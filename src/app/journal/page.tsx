import { useEffect, useState } from "react";
import { Link } from "react-router";
import { useTranslation } from "react-i18next";
import PageContainer from "@/components/navigation/PageContainer";
import {
  archiveJournalEntries,
  listJournalEntries,
  setJournalEntriesExportEligible,
} from "@/lib/api/journalApi";
import { ensureDefaultWorkspace } from "@/lib/api/workspaceApi";
import { toDisplayMessage } from "@/lib/errors";
import { formatDate } from "@/lib/format";
import type { JournalEntry } from "@/lib/types";

function formatMonth(monthKey: string): string {
  const [year, month] = monthKey.split("-").map(Number);
  return formatDate(new Date(year, month - 1, 1), {
    month: "long",
    year: "numeric",
  });
}

/**
 * The occurred-on date-only value is an application-owned date, so it renders
 * through the shared Intl date formatter with a local-noon anchor (the
 * established 8.6.8/8.6.9 pattern) instead of the raw stored `YYYY-MM-DD`.
 * Only read-only labels change; stored values and date inputs keep the exact
 * ISO string. Authored titles/body are never passed through this.
 */
function formatEntryDate(occurredOn: string): string {
  return formatDate(new Date(`${occurredOn}T12:00:00`), { dateStyle: "medium" });
}

function JournalPage() {
  const { t } = useTranslation();
  const [entries, setEntries] = useState<JournalEntry[]>([]);
  const [selectedIds, setSelectedIds] = useState<Set<string>>(new Set());
  const [error, setError] = useState<string | null>(null);
  const [feedback, setFeedback] = useState<string | null>(null);
  const [workspaceId, setWorkspaceId] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  useEffect(() => {
    void ensureDefaultWorkspace()
      .then((workspace) => setWorkspaceId(workspace.id))
      .catch((err) => setError(toDisplayMessage(err)));
  }, []);

  useEffect(() => {
    if (!workspaceId) return;
    void listJournalEntries(workspaceId)
      .then(setEntries)
      .catch((err) => setError(toDisplayMessage(err)));
  }, [workspaceId]);

  // The selection is scoped to the currently listed rows (the list command
  // already excludes archived entries). Whenever the list changes, any selected
  // id that is no longer listed — archived or otherwise absent — drops out,
  // so a batch action can never include archived/not-current-list rows.
  useEffect(() => {
    setSelectedIds((previous) => {
      const listed = new Set(entries.map((entry) => entry.id));
      const next = new Set([...previous].filter((id) => listed.has(id)));
      return next.size === previous.size ? previous : next;
    });
  }, [entries]);

  const toggleEntry = (id: string) => {
    setSelectedIds((previous) => {
      const next = new Set(previous);
      if (next.has(id)) next.delete(id);
      else next.add(id);
      return next;
    });
  };

  const selectAll = () => {
    setSelectedIds(new Set(entries.map((entry) => entry.id)));
  };

  const deselectAll = () => {
    setSelectedIds(new Set());
  };

  const handleArchiveSelected = async () => {
    if (!workspaceId || busy || selectedIds.size === 0) return;
    const ids = Array.from(selectedIds);
    // Count-only confirmation and feedback: never includes entry titles or body.
    const confirmed = window.confirm(t("journal.archiveConfirm", { count: ids.length }));
    if (!confirmed) return;
    setBusy(true);
    setError(null);
    setFeedback(null);
    try {
      const count = await archiveJournalEntries(workspaceId, ids);
      // The native batch is atomic: only now may local state change. Archived
      // rows leave the list (the list excludes archived entries) and the
      // selection is cleared with them.
      const idSet = new Set(ids);
      setEntries((previous) => previous.filter((entry) => !idSet.has(entry.id)));
      setSelectedIds(new Set());
      setFeedback(t("journal.archived", { count }));
    } catch (err) {
      // All-or-nothing: nothing changed in the database, so the list and the
      // selection stay exactly as they were. Surface one privacy-safe message.
      setError(toDisplayMessage(err));
    } finally {
      setBusy(false);
    }
  };

  const handleSetEligibility = async (exportEligible: boolean) => {
    if (!workspaceId || busy || selectedIds.size === 0) return;
    const ids = Array.from(selectedIds);
    setBusy(true);
    setError(null);
    setFeedback(null);
    try {
      const count = await setJournalEntriesExportEligible(workspaceId, ids, exportEligible);
      // Atomic native success: reflect it in the listed badges only now.
      const idSet = new Set(ids);
      setEntries((previous) =>
        previous.map((entry) =>
          idSet.has(entry.id) ? { ...entry, export_eligible: exportEligible } : entry,
        ),
      );
      setSelectedIds(new Set());
      setFeedback(
        exportEligible
          ? t("journal.markedExportable", { count })
          : t("journal.markedPrivate", { count }),
      );
    } catch (err) {
      setError(toDisplayMessage(err));
    } finally {
      setBusy(false);
    }
  };

  const months = new Map<string, JournalEntry[]>();
  for (const entry of entries) {
    const key = entry.occurred_on.slice(0, 7);
    const group = months.get(key) ?? [];
    group.push(entry);
    months.set(key, group);
  }

  const hasSelection = selectedIds.size > 0;
  const actionDisabled = busy || !hasSelection;

  return (
    <PageContainer>
      <div className="flex items-center justify-between">
        <h1 className="text-2xl font-semibold">{t("journal.title")}</h1>
        <div className="flex items-center gap-2">
          <Link to="/journal/export" className="joi-btn joi-btn--secondary">
            {t("common.actions.export")}
          </Link>
          <Link to="/journal/new" className="joi-btn joi-btn--primary">
            {t("journal.new")}
          </Link>
        </div>
      </div>
      {error ? (
        <p role="alert" className="mt-2 text-sm joi-text--danger">
          {error}
        </p>
      ) : null}
      {feedback ? (
        <p role="status" className="mt-2 text-sm joi-text--success">
          {feedback}
        </p>
      ) : null}
      {entries.length > 0 ? (
        <div className="mt-3 flex flex-wrap items-center gap-2">
          <span className="text-xs joi-text--muted" role="status">
            {hasSelection
              ? t("journal.selectedCount", { count: selectedIds.size })
              : t("journal.listedCount", { count: entries.length })}
          </span>
          <button
            type="button"
            onClick={selectAll}
            disabled={busy}
            className="joi-btn joi-btn--secondary joi-btn--sm"
          >
            {t("journal.selectAll")}
          </button>
          <button
            type="button"
            onClick={deselectAll}
            disabled={actionDisabled}
            className="joi-btn joi-btn--secondary joi-btn--sm"
          >
            {t("journal.deselectAll")}
          </button>
          <div className="flex items-center gap-2 sm:ml-auto">
            <button
              type="button"
              onClick={() => void handleSetEligibility(true)}
              disabled={actionDisabled}
              className="joi-btn joi-btn--secondary joi-btn--sm"
            >
              {t("journal.setExportable")}
            </button>
            <button
              type="button"
              onClick={() => void handleSetEligibility(false)}
              disabled={actionDisabled}
              className="joi-btn joi-btn--secondary joi-btn--sm"
            >
              {t("journal.setPrivate")}
            </button>
            <button
              type="button"
              onClick={() => void handleArchiveSelected()}
              disabled={actionDisabled}
              className="joi-btn joi-btn--danger joi-btn--sm"
            >
              {t("journal.archiveSelected")}
            </button>
          </div>
        </div>
      ) : null}
      <div className="mt-4 gap-x-10 xl:columns-2 3xl:columns-3">
        {[...months.entries()].map(([month, monthEntries]) => (
          <div key={month} className="break-inside-avoid">
            <h2 className="text-sm font-medium joi-text--muted">{formatMonth(month)}</h2>
            <ul className="mt-2 flex flex-col gap-1">
              {monthEntries.map((entry) => (
                <li key={entry.id} className="flex items-center gap-2">
                  {/* The checkbox is a sibling of the entry link, never nested
                      inside it, so both stay individually operable. The row
                      title is user-authored content and stays verbatim. */}
                  <input
                    type="checkbox"
                    className="joi-checkbox"
                    checked={selectedIds.has(entry.id)}
                    onChange={() => toggleEntry(entry.id)}
                    disabled={busy}
                    aria-label={t("journal.selectEntryAria", { title: entry.title })}
                  />
                  <Link
                    to={`/journal/${entry.id}`}
                    className="joi-interactive flex flex-1 items-center justify-between rounded-md px-2 py-1.5 text-sm"
                  >
                    <span className="truncate joi-text">{entry.title}</span>
                    <span className="flex shrink-0 items-center gap-2 text-xs joi-text--muted">
                      <span className="joi-badge joi-badge--neutral">
                        {entry.export_eligible ? t("journal.exportable") : t("journal.private")}
                      </span>
                      <span>{formatEntryDate(entry.occurred_on)}</span>
                    </span>
                  </Link>
                </li>
              ))}
            </ul>
          </div>
        ))}
        {entries.length === 0 ? <p className="joi-empty__state">{t("journal.empty")}</p> : null}
      </div>
    </PageContainer>
  );
}

export default JournalPage;
