import { useEffect, useMemo, useState } from "react";
import { Link } from "react-router";
import { useTranslation } from "react-i18next";
import { listTherapyCollections, listTherapyRecords } from "@/lib/api/therapyApi";
import PageContainer from "@/components/navigation/PageContainer";
import {
  createExportJob,
  exportTemplateCapability,
  previewExportSelection,
  saveExportArtifactCopy,
} from "@/lib/api/exportApi";
import { requestExportCancel, runExportWithProgress, type JobProgressDto } from "@/lib/api/jobApi";
import { ensureDefaultWorkspace } from "@/lib/api/workspaceApi";
import { isStalePreviewError, toDisplayMessage } from "@/lib/errors";
import { formatDate } from "@/lib/format";
import type {
  ExportJob,
  ExportPreview as ExportPreviewData,
  ExportArtifact,
  TemplateCapability,
  TherapyRecord,
} from "@/lib/types";
import ExportPreview from "@/components/export/ExportPreview";
import ExportJobStatus from "@/components/export/ExportJobStatus";
import JobProgressModal, { type JobProgressState } from "@/components/jobs/JobProgressModal";

// Formats offered on this screen; the labels render from the matching
// `common.export.formats` catalog keys.
const EXPORT_FORMATS = ["markdown", "typst", "pdf"] as const;
type TherapyExportFormat = (typeof EXPORT_FORMATS)[number];

// Template identifiers are export-template content (8.6.7 exclusion): the
// option value stays verbatim; only the selector label chrome is translated.
const TEMPLATES = ["default"];

/**
 * The record occurred-on date-only value is an application-owned date: the
 * read-only label renders through the shared Intl date formatter with a
 * local-noon anchor (the established 8.6.8/8.6.9 pattern). Stored values stay
 * the exact ISO string; authored record titles render verbatim and never pass
 * through this.
 */
function formatRecordDate(occurredOn: string): string {
  return formatDate(new Date(`${occurredOn}T12:00:00`), { dateStyle: "medium" });
}

function TherapyExportPage() {
  const { t } = useTranslation();
  const [workspaceId, setWorkspaceId] = useState<string | null>(null);
  const [exportAll, setExportAll] = useState(false);
  const [records, setRecords] = useState<TherapyRecord[]>([]);
  const [selectedIds, setSelectedIds] = useState<Set<string>>(new Set());
  const [dateFrom, setDateFrom] = useState("");
  const [dateTo, setDateTo] = useState("");
  // The default job title is application-owned UI copy; the user can edit it.
  const [title, setTitle] = useState(t("therapy.export.defaultTitle"));
  const [exportType, setExportType] = useState<TherapyExportFormat>("markdown");
  const [template, setTemplate] = useState("default");
  const [preview, setPreview] = useState<ExportPreviewData | null>(null);
  const [job, setJob] = useState<ExportJob | null>(null);
  const [confirmed, setConfirmed] = useState(false);
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
    void listTherapyCollections(workspaceId)
      .then(async (collections) => {
        const all: TherapyRecord[] = [];
        for (const collection of collections) {
          const items = await listTherapyRecords(collection.id);
          all.push(...items);
        }
        setRecords(all);
      })
      .catch((err) => setError(toDisplayMessage(err)));
  }, [workspaceId]);

  const selection = useMemo(() => {
    if (exportAll) {
      return { export_all: true };
    }
    return {
      therapy_record_ids: selectedIds.size > 0 ? Array.from(selectedIds) : null,
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
      scope_type: "therapy",
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

  const toggleRecord = (id: string) => {
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
    if (!workspaceId || loading || !confirmed) return;
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
        scope_type: "therapy",
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

  const handleSaveCopyArtifact = async (artifact: ExportArtifact) => {
    try {
      const result = await saveExportArtifactCopy(workspaceId!, job!.id, artifact.id);
      if (!result.cancelled) {
        // Brief confirmation - user-cancelled dialog = silent no-op
      }
      // user-cancelled dialog = silent no-op (cancelled: true)
    } catch (err) {
      setError(toDisplayMessage(err));
    }
  };

  const inputClass = "joi-field__input disabled:opacity-50";
  const selectClass = "joi-field__select disabled:opacity-50";

  const formatLabel = (value: TherapyExportFormat) => t(`common.export.formats.${value}`);

  return (
    <PageContainer>
      <div className="flex items-center justify-between">
        <h1 className="text-2xl font-semibold">{t("therapy.export.title")}</h1>
        <Link to="/therapy" className="joi-btn joi-btn--secondary px-3 py-1.5">
          {t("common.actions.back")}
        </Link>
      </div>

      <p className="joi-text--muted mt-2 text-sm">{t("therapy.export.intro")}</p>

      {capability ? (
        <p className="joi-text--muted mt-2 text-xs" role="status">
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
        <p role="alert" className="joi-text--danger mt-2 text-sm">
          {error}
        </p>
      ) : null}

      {stalePreview ? (
        <div className="joi-alert--warning mt-2 flex items-center gap-3 rounded-md border p-3">
          <p className="joi-text--warning text-sm">{t("common.export.stalePreview")}</p>
          <button
            type="button"
            onClick={() => {
              setStalePreview(false);
              setError(null);
              setPreviewNonce((n) => n + 1);
            }}
            className="joi-btn joi-btn--secondary shrink-0 px-3 py-1.5"
          >
            {t("common.export.rePreview")}
          </button>
        </div>
      ) : null}

      <form onSubmit={handleSubmit} className="mt-4 flex flex-col gap-4">
        <div>
          <label className="joi-field__label mb-1 block">{t("common.fields.title")}</label>
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
            <label className="joi-field__label mb-1 block">{t("common.export.format")}</label>
            <select
              className={selectClass}
              value={exportType}
              onChange={(e) => setExportType(e.target.value as TherapyExportFormat)}
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
            <label className="joi-field__label mb-1 block">{t("common.export.template")}</label>
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
          <label htmlFor="export-all" className="joi-text text-sm font-medium">
            {t("common.export.exportAll")}
            <span className="joi-text--muted ml-2 text-xs font-normal">
              {t("therapy.export.exportAllNote")}
            </span>
          </label>
        </div>

        <div className="grid grid-cols-2 gap-4">
          <div>
            <label className="joi-field__label mb-1 block">{t("common.export.from")}</label>
            <input
              type="date"
              className={inputClass}
              value={dateFrom}
              onChange={(e) => setDateFrom(e.target.value)}
              disabled={loading || exportAll}
            />
          </div>
          <div>
            <label className="joi-field__label mb-1 block">{t("common.export.to")}</label>
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
          <label className="joi-field__label mb-1 block">{t("therapy.export.selected")}</label>
          {records.length === 0 ? (
            <p className="joi-empty__state">{t("therapy.export.empty")}</p>
          ) : (
            <ul className="joi-surface--base joi-border--default flex max-h-64 flex-col gap-1 overflow-y-auto rounded-lg border p-2">
              {records.map((record) => (
                <li key={record.id} className="flex items-center gap-2 text-sm">
                  <input
                    type="checkbox"
                    id={`record-${record.id}`}
                    className="joi-checkbox"
                    checked={selectedIds.has(record.id)}
                    onChange={() => toggleRecord(record.id)}
                    disabled={loading || exportAll}
                  />
                  <label htmlFor={`record-${record.id}`} className="joi-text flex-1">
                    {record.title}
                    <span className="joi-text--muted ml-2 text-xs">
                      {record.occurred_on
                        ? formatRecordDate(record.occurred_on)
                        : t("therapy.export.noDate")}
                    </span>
                  </label>
                </li>
              ))}
            </ul>
          )}
        </div>

        <ExportPreview preview={preview} />

        <label className="joi-alert joi-alert--warning flex flex-row items-start gap-2 rounded-md border p-3">
          <input
            type="checkbox"
            className="joi-checkbox"
            checked={confirmed}
            onChange={(e) => setConfirmed(e.target.checked)}
            disabled={loading}
          />
          <span className="text-sm">{t("therapy.export.confirmPrivacy")}</span>
        </label>

        <button
          type="submit"
          disabled={
            loading ||
            !confirmed ||
            (!exportAll && selectedIds.size === 0 && (!dateFrom || !dateTo))
          }
          className="joi-btn joi-btn--primary px-4 py-2"
        >
          {loading ? t("common.export.exporting") : t("common.actions.export")}
        </button>
      </form>

      {loading && !job ? (
        <p className="joi-empty__loading mt-4" role="status">
          {t("common.export.preparing")}
        </p>
      ) : null}

      {job ? (
        <div className="mt-4">
          <ExportJobStatus job={job} onSaveCopyArtifact={handleSaveCopyArtifact} />
        </div>
      ) : null}

      {progressOpen && progress ? (
        <div className="mt-4">
          <JobProgressModal state={progress} onClose={() => setProgressOpen(false)} />
        </div>
      ) : null}
    </PageContainer>
  );
}

export default TherapyExportPage;
