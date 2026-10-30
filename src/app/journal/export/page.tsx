import { useEffect, useMemo, useState } from "react";
import { Link } from "react-router";
import { useTranslation } from "react-i18next";
import { listJournalEntries } from "@/lib/api/journalApi";
import PageContainer from "@/components/navigation/PageContainer";
import {
  createExportJob,
  deleteExportJob,
  exportTemplateCapability,
  listExportJobs,
  openExportArtifact,
  previewExportSelection,
  saveExportArtifactCopy,
} from "@/lib/api/exportApi";
import { requestExportCancel, runExportWithProgress, type JobProgressDto } from "@/lib/api/jobApi";
import { ensureDefaultWorkspace } from "@/lib/api/workspaceApi";
import { isStalePreviewError, toDisplayMessage } from "@/lib/errors";
import { formatDate } from "@/lib/format";
import type {
  ExportJob,
  ExportArtifact,
  ExportPreview as ExportPreviewData,
  JournalEntry,
  TemplateCapability,
} from "@/lib/types";
import ExportPreview from "@/components/export/ExportPreview";
import ExportJobStatus from "@/components/export/ExportJobStatus";
import ExportHistory from "@/components/export/ExportHistory";
import JobProgressModal, { type JobProgressState } from "@/components/jobs/JobProgressModal";

// Formats offered on this screen; the labels render from the matching
// `common.export.formats` catalog keys.
const EXPORT_FORMATS = ["markdown", "typst", "pdf"] as const;
type JournalExportFormat = (typeof EXPORT_FORMATS)[number];

// Template identifiers are export-template content (8.6.7 exclusion): the
// option value stays verbatim; only the selector label chrome is translated.
const TEMPLATES = ["foundation-journal"];

/**
 * The row's occurred-on date-only value is an application-owned date: the
 * read-only label renders through the shared Intl date formatter with a
 * local-noon anchor (the established 8.6.8/8.6.9 pattern). Stored/selected
 * values and the date-range inputs keep the exact ISO string; authored entry
 * titles render verbatim and never pass through this.
 */
function formatEntryDate(occurredOn: string): string {
  return formatDate(new Date(`${occurredOn}T12:00:00`), { dateStyle: "medium" });
}

function JournalExportPage() {
  const { t } = useTranslation();
  const [workspaceId, setWorkspaceId] = useState<string | null>(null);
  const [exportAll, setExportAll] = useState(false);
  const [entries, setEntries] = useState<JournalEntry[]>([]);
  const [selectedIds, setSelectedIds] = useState<Set<string>>(new Set());
  const [dateFrom, setDateFrom] = useState("");
  const [dateTo, setDateTo] = useState("");
  // The default job title is application-owned UI copy; the user can edit it.
  const [title, setTitle] = useState(t("journal.export.defaultTitle"));
  // PDF is the primary Phase 7.10 journal deliverable. Markdown and Typst
  // remain available in the format selector, but the default must exercise
  // the complete Markdown -> Typst -> PDF pipeline.
  const [exportType, setExportType] = useState<JournalExportFormat>("pdf");
  const [template, setTemplate] = useState("foundation-journal");
  const [preview, setPreview] = useState<ExportPreviewData | null>(null);
  const [job, setJob] = useState<ExportJob | null>(null);
  const [exportHistory, setExportHistory] = useState<ExportJob[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [stalePreview, setStalePreview] = useState(false);
  const [previewNonce, setPreviewNonce] = useState(0);
  const [progress, setProgress] = useState<JobProgressState | null>(null);
  const [progressOpen, setProgressOpen] = useState(false);
  const [activeJobId, setActiveJobId] = useState<string | null>(null);
  const [capability, setCapability] = useState<TemplateCapability | null>(null);

  useEffect(() => {
    void exportTemplateCapability()
      .then(setCapability)
      .catch(() => setCapability(null));
  }, []);

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
    void listExportJobs(workspaceId)
      .then(setExportHistory)
      .catch((err) => setError(toDisplayMessage(err)));
  }, [workspaceId]);

  const handleOpenArtifact = async (artifact: ExportArtifact) => {
    try {
      await openExportArtifact(artifact.path);
    } catch (err) {
      setError(toDisplayMessage(err));
    }
  };

  const handleSaveCopyArtifact = async (artifact: ExportArtifact) => {
    try {
      const result = await saveExportArtifactCopy(workspaceId!, job!.id, artifact.id);
      if (!result.cancelled) {
        // Brief confirmation - no path displayed, just a toast-like message
        setError(null);
        // Could add a success state here, but keeping it simple per spec
      }
      // user-cancelled dialog = silent no-op (cancelled: true)
    } catch (err) {
      setError(toDisplayMessage(err));
    }
  };

  const handleDeleteJob = async (exportJob: ExportJob) => {
    if (!workspaceId) return;
    try {
      await deleteExportJob(workspaceId, exportJob.id);
      setExportHistory((previous) => previous.filter((job) => job.id !== exportJob.id));
      setJob((current) => (current?.id === exportJob.id ? null : current));
    } catch (err) {
      setError(toDisplayMessage(err));
    }
  };

  const selection = useMemo(() => {
    if (exportAll) {
      return { export_all: true };
    }
    return {
      journal_entry_ids: selectedIds.size > 0 ? Array.from(selectedIds) : null,
      date_range: dateFrom && dateTo ? { from: dateFrom, to: dateTo } : null,
    };
  }, [exportAll, selectedIds, dateFrom, dateTo]);

  useEffect(() => {
    if (!workspaceId) return;
    if (!exportAll && selectedIds.size === 0 && (!dateFrom || !dateTo)) {
      setPreview(null);
      return;
    }
    const controller = new AbortController();
    void previewExportSelection({
      workspace_id: workspaceId,
      export_type: exportType,
      scope_type: "journal",
      title,
      template_name: template,
      selection,
    })
      .then((result) => {
        if (!controller.signal.aborted) {
          setPreview(result);
          setStalePreview(false);
        }
      })
      .catch((err) => {
        if (!controller.signal.aborted) setError(toDisplayMessage(err));
      });
    return () => controller.abort();
  }, [workspaceId, exportType, title, template, selection, previewNonce]);

  const toggleEntry = (id: string) => {
    if (loading) return;
    setSelectedIds((prev) => {
      const next = new Set(prev);
      if (next.has(id)) next.delete(id);
      else next.add(id);
      return next;
    });
  };

  const toModalState = (state: JobProgressDto): JobProgressState => ({
    phase: state.phase ?? t("common.export.running"),
    percent: state.percent,
    status: state.status ?? "running",
    terminal: state.terminal,
    error: state.error,
    recoveryLabel: state.recoveryLabel,
    onRecovery: () => setProgressOpen(false),
    onCancelRequest: () => {
      if (activeJobId) {
        void requestExportCancel(activeJobId).catch((err: unknown) =>
          setError(toDisplayMessage(err)),
        );
      }
      setProgressOpen(false);
    },
  });

  const handleSubmit = async (event: React.FormEvent) => {
    event.preventDefault();
    if (!workspaceId || loading) return;
    if (!exportAll && selectedIds.size === 0 && (!dateFrom || !dateTo)) return;
    setLoading(true);
    setError(null);
    setStalePreview(false);
    setJob(null);
    setProgressOpen(true);
    try {
      const created = await createExportJob({
        workspace_id: workspaceId,
        export_type: exportType,
        scope_type: "journal",
        title,
        template_name: template,
        selection,
        preview_fingerprint: preview?.selection_fingerprint ?? null,
      });
      // Freeze the confirmed selection: inputs stay disabled until terminal.
      setActiveJobId(created.id);
      const ran = await runExportWithProgress(created.id, (state) =>
        setProgress(toModalState(state)),
      );
      setJob(ran);
      setExportHistory((previous) => [
        ran,
        ...previous.filter((previousJob) => previousJob.id !== ran.id),
      ]);
    } catch (err) {
      if (isStalePreviewError(err)) {
        setStalePreview(true);
        setError(t("common.export.stalePreview"));
      } else {
        setError(toDisplayMessage(err));
      }
    } finally {
      setLoading(false);
      setActiveJobId(null);
    }
  };

  const inputClass = "joi-field__input";
  const selectClass = "joi-field__select";

  const formatLabel = (value: JournalExportFormat) => t(`common.export.formats.${value}`);

  return (
    <PageContainer>
      <div className="flex items-center justify-between">
        <h1 className="text-2xl font-semibold">{t("journal.export.title")}</h1>
        <Link to="/journal" className="joi-btn joi-btn--secondary">
          {t("common.actions.back")}
        </Link>
      </div>

      {capability ? (
        <p className="mt-2 text-xs joi-text--muted" role="status">
          {t("common.export.template")} {capability.name} v{capability.version} ·{" "}
          {capability.valid ? t("common.export.valid") : t("common.export.invalid")} ·{" "}
          {capability.compiler_version}
          {capability.unavailable_resources.length > 0
            ? ` · ${t("common.export.unavailable", {
                resources: capability.unavailable_resources.join(", "),
              })}`
            : ""}
        </p>
      ) : null}

      {error ? (
        <p role="alert" className="mt-2 text-sm joi-text--danger">
          {error}
        </p>
      ) : null}

      {stalePreview ? (
        <div className="joi-alert joi-alert--warning mt-2 flex-row items-center gap-3">
          <p className="text-sm">{t("common.export.stalePreview")}</p>
          <button
            type="button"
            onClick={() => {
              setStalePreview(false);
              setError(null);
              setPreviewNonce((n) => n + 1);
            }}
            className="joi-btn joi-btn--secondary joi-btn--sm shrink-0"
          >
            {t("common.export.rePreview")}
          </button>
        </div>
      ) : null}

      <form onSubmit={handleSubmit} className="mt-4 flex flex-col gap-4">
        <div>
          <label className="mb-1 block joi-field__label">{t("common.fields.title")}</label>
          <input
            className={inputClass}
            value={title}
            onChange={(e) => setTitle(e.target.value)}
            required
            disabled={loading}
          />
        </div>

        <div className="grid grid-cols-2 gap-4">
          <div>
            <label className="mb-1 block joi-field__label">{t("common.export.format")}</label>
            <select
              className={selectClass}
              value={exportType}
              onChange={(e) => setExportType(e.target.value as JournalExportFormat)}
              disabled={loading}
            >
              {EXPORT_FORMATS.map((formatOption) => (
                <option key={formatOption} value={formatOption}>
                  {formatLabel(formatOption)}
                </option>
              ))}
            </select>
          </div>
          <div>
            <label className="mb-1 block joi-field__label">{t("common.export.template")}</label>
            <select
              className={selectClass}
              value={template}
              onChange={(e) => setTemplate(e.target.value)}
              disabled={loading}
            >
              {TEMPLATES.map((templateName) => (
                <option key={templateName} value={templateName}>
                  {templateName}
                </option>
              ))}
            </select>
          </div>
        </div>

        <div className="flex items-center gap-2">
          <input
            type="checkbox"
            id="export-all"
            className="joi-checkbox"
            checked={exportAll}
            onChange={(e) => setExportAll(e.target.checked)}
            disabled={loading}
          />
          <label htmlFor="export-all" className="joi-field__label">
            {t("common.export.exportAll")}
            <span className="ml-2 text-xs font-normal joi-text--muted">
              {t("journal.export.exportAllNote")}
            </span>
          </label>
        </div>

        <div className="grid grid-cols-2 gap-4">
          <div>
            <label className="mb-1 block joi-field__label">{t("common.export.from")}</label>
            <input
              type="date"
              className={inputClass}
              value={dateFrom}
              onChange={(e) => setDateFrom(e.target.value)}
              disabled={loading || exportAll}
            />
          </div>
          <div>
            <label className="mb-1 block joi-field__label">{t("common.export.to")}</label>
            <input
              type="date"
              className={inputClass}
              value={dateTo}
              onChange={(e) => setDateTo(e.target.value)}
              disabled={loading || exportAll}
            />
          </div>
        </div>

        <div>
          <label className="mb-1 block joi-field__label">{t("journal.export.selected")}</label>
          {entries.length === 0 ? (
            <p className="text-sm joi-text--muted">{t("journal.export.empty")}</p>
          ) : (
            <ul className="joi-border--default flex max-h-64 flex-col gap-1 overflow-y-auto rounded-lg border p-2">
              {entries.map((entry) => (
                <li key={entry.id} className="flex items-center gap-2 text-sm">
                  <input
                    type="checkbox"
                    id={`entry-${entry.id}`}
                    className="joi-checkbox"
                    checked={selectedIds.has(entry.id)}
                    onChange={() => toggleEntry(entry.id)}
                    disabled={loading || exportAll}
                  />
                  <label htmlFor={`entry-${entry.id}`} className="joi-text flex-1">
                    {entry.title}
                    <span className="ml-2 text-xs joi-text--muted">
                      {formatEntryDate(entry.occurred_on)}
                    </span>
                  </label>
                </li>
              ))}
            </ul>
          )}
        </div>

        <ExportPreview preview={preview} />

        <button
          type="submit"
          disabled={loading || (!exportAll && selectedIds.size === 0 && (!dateFrom || !dateTo))}
          className="joi-btn joi-btn--primary"
        >
          {loading
            ? t("common.export.exporting")
            : t("common.export.exportButton", { format: formatLabel(exportType) })}
        </button>
      </form>

      {loading && !job ? (
        <p className="mt-4 text-sm joi-text--muted" role="status">
          {t("common.export.preparing")}
        </p>
      ) : null}

      {job ? (
        <div className="mt-4">
          <ExportJobStatus
            job={job}
            onOpenArtifact={handleOpenArtifact}
            onSaveCopyArtifact={handleSaveCopyArtifact}
          />
        </div>
      ) : null}

      <ExportHistory
        jobs={exportHistory}
        onOpenArtifact={handleOpenArtifact}
        onSaveCopyArtifact={handleSaveCopyArtifact}
        onDeleteJob={handleDeleteJob}
      />

      {progressOpen && progress ? (
        <div className="mt-4">
          <JobProgressModal state={progress} onClose={() => setProgressOpen(false)} />
        </div>
      ) : null}
    </PageContainer>
  );
}

export default JournalExportPage;
