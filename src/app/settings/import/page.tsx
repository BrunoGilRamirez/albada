import { useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import ImportReportView, {
  type ReviewDecisionValue,
} from "../../../components/import/ImportReportView";
import JobProgressModal, { type JobProgressState } from "../../../components/jobs/JobProgressModal";
import {
  commitLegacyImport,
  getImportBatch,
  getImportBatchStatus,
  previewLegacyImport,
  startLegacyImport,
} from "../../../lib/api/importApi";
import { startImportPoll } from "../../../lib/api/jobApi";
import { ensureDefaultWorkspace } from "../../../lib/api/workspaceApi";
import PageContainer from "../../../components/navigation/PageContainer";
import { toDisplayMessage } from "../../../lib/errors";
import type {
  ImportBatchStatusDto,
  ImportReportDto,
  ImportReviewItemDto,
  ReviewDecisionDto,
} from "../../../lib/types";

const POLL_INTERVAL_MS = 1000;

const TERMINAL_STATUSES = new Set(["completed", "failed", "cancelled"]);

// Batch statuses with a localized label under `common.labels.jobStatus.*`; an
// unknown backend status keeps its raw value instead of being keyed into the
// catalogs.
const KNOWN_STATUSES = ["queued", "running", "completed", "failed", "cancelled"] as const;
type KnownStatus = (typeof KNOWN_STATUSES)[number];

function isKnownStatus(value: string): value is KnownStatus {
  return (KNOWN_STATUSES as readonly string[]).includes(value);
}

function ImportSettingsPage() {
  const { t } = useTranslation();
  const [sourceRoot, setSourceRoot] = useState("");
  const [fullScan, setFullScan] = useState(true);
  const [includePaths, setIncludePaths] = useState("");
  const [preview, setPreview] = useState<ImportReportDto | null>(null);
  const [running, setRunning] = useState(false);
  const [status, setStatus] = useState<ImportBatchStatusDto | null>(null);
  const [report, setReport] = useState<ImportReportDto | null>(null);
  const [previewing, setPreviewing] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [decisions, setDecisions] = useState<Record<string, ReviewDecisionValue>>({});
  const [committing, setCommitting] = useState(false);
  const [progress, setProgress] = useState<JobProgressState | null>(null);
  const [progressOpen, setProgressOpen] = useState(false);

  // Track the active batch ID so polling can be cancelled on unmount or when a
  // new run starts. Polling is non-blocking: navigation stays usable.
  const activeBatchId = useRef<string | null>(null);
  const stopPoll = useRef<(() => void) | null>(null);

  useEffect(() => {
    return () => {
      activeBatchId.current = null;
      stopPoll.current?.();
      stopPoll.current = null;
    };
  }, []);

  const getIncludePaths = () =>
    fullScan
      ? null
      : includePaths
          .split("\n")
          .map((line) => line.trim())
          .filter((line) => line.length > 0);

  const handlePreview = async (event: React.FormEvent) => {
    event.preventDefault();
    setError(null);
    setReport(null);
    setPreview(null);
    setStatus(null);
    setDecisions({});
    setProgress(null);
    setProgressOpen(false);
    const path = sourceRoot.trim();
    if (!path) {
      setError(t("settings.import.needPath"));
      return;
    }
    setPreviewing(true);
    try {
      setPreview(await previewLegacyImport(path, getIncludePaths()));
    } catch (err) {
      setError(toDisplayMessage(err));
    } finally {
      setPreviewing(false);
    }
  };

  const pollBatch = async (batchId: string) => {
    while (activeBatchId.current === batchId) {
      let current: ImportBatchStatusDto;
      try {
        current = await getImportBatchStatus(batchId);
      } catch (err) {
        setError(toDisplayMessage(err));
        setRunning(false);
        return;
      }

      setStatus(current);

      if (TERMINAL_STATUSES.has(current.status)) {
        if (current.status === "completed") {
          try {
            setReport(await getImportBatch(batchId));
          } catch (err) {
            setError(toDisplayMessage(err));
          }
        } else {
          setError(t("settings.import.notComplete"));
        }
        setRunning(false);
        return;
      }

      await new Promise((resolve) => setTimeout(resolve, POLL_INTERVAL_MS));
    }
  };

  const handleDecisionChange = (item: ImportReviewItemDto, value: ReviewDecisionValue) => {
    setDecisions((prev) => ({ ...prev, [item.id]: value }));
  };

  const allDecided =
    preview !== null &&
    preview.review_items.length > 0 &&
    preview.review_items.every((item) => decisions[item.id] !== undefined);

  const handleCommit = async () => {
    const path = sourceRoot.trim();
    if (!preview || running || committing || !allDecided) return;
    setError(null);
    setReport(null);
    setCommitting(true);
    try {
      const workspace = await ensureDefaultWorkspace();
      const decisionList: ReviewDecisionDto[] = preview.review_items.map((item) => ({
        review_item_id: item.id,
        entity_type: decisions[item.id],
      }));
      const committed = await commitLegacyImport({
        workspace_id: workspace.id,
        source_root: path,
        include_paths: getIncludePaths(),
        preview_fingerprint: preview.preview_fingerprint,
        decisions: decisionList,
      });
      activeBatchId.current = committed.batch_id;
      setStatus({
        batch_id: committed.batch_id,
        status: committed.status,
        started_at: committed.started_at,
        completed_at: committed.completed_at,
      });
      setRunning(true);
      setProgressOpen(true);
      // Non-blocking poll; navigation stays usable and unmount stops polling.
      stopPoll.current?.();
      stopPoll.current = startImportPoll(committed.batch_id, (state) => {
        setProgress({
          // The progress phase is chrome shown by the shared job modal, so it
          // follows the active locale; the native error text is not translated.
          phase: state.phase ?? t("settings.import.importing"),
          percent: state.percent,
          status: state.status ?? "running",
          terminal: state.terminal,
          error: state.error,
          recoveryLabel: state.recoveryLabel,
          onRecovery: () => setProgressOpen(false),
          onCancelRequest: () => setProgressOpen(false),
        });
        if (state.terminal) {
          setRunning(false);
          if (state.status === "completed") {
            getImportBatch(committed.batch_id)
              .then(setReport)
              .catch((err: unknown) => setError(toDisplayMessage(err)));
          }
        }
      });
    } catch (err) {
      const message = toDisplayMessage(err);
      setError(
        /stale|fingerprint|changed|decision|incomplete/i.test(message)
          ? t("settings.import.staleHint", { message })
          : message,
      );
    } finally {
      setCommitting(false);
    }
  };

  const handleRun = async () => {
    const path = sourceRoot.trim();
    if (!path || running) return;
    setError(null);
    setReport(null);
    setRunning(true);
    setStatus(null);
    try {
      const started = await startLegacyImport(path, getIncludePaths());
      activeBatchId.current = started.batch_id;
      setStatus(started);
      setPreview(null);
      // Poll in the background; do not block navigation or the command call.
      void pollBatch(started.batch_id);
    } catch (err) {
      setError(toDisplayMessage(err));
      setRunning(false);
    }
  };

  return (
    <PageContainer>
      <h1 className="text-2xl font-semibold">{t("settings.import.title")}</h1>
      <p className="joi-text--muted mt-2">{t("settings.import.subtitle")}</p>

      {error ? (
        <p role="alert" className="joi-text--danger mt-4 text-sm">
          {error}
        </p>
      ) : null}

      <form
        onSubmit={handlePreview}
        className="joi-surface--base joi-border--default mt-6 flex flex-col gap-4 rounded-lg border p-4"
      >
        <div>
          <label htmlFor="import-source-root" className="joi-field__label mb-1 block">
            {t("settings.import.sourceRoot")}
          </label>
          <input
            id="import-source-root"
            type="text"
            value={sourceRoot}
            onChange={(e) => setSourceRoot(e.target.value)}
            // Locale-neutral path sample: it shows the expected shape of a
            // Windows path and is never a translatable string.
            placeholder="C:\path\to\proyecto-vida"
            className="joi-field__input"
          />
        </div>

        <div className="flex items-center gap-2">
          <input
            id="import-full-scan"
            type="checkbox"
            checked={fullScan}
            onChange={(e) => setFullScan(e.target.checked)}
            className="joi-checkbox h-4 w-4 rounded"
          />
          <label htmlFor="import-full-scan" className="joi-field__label">
            {t("settings.import.fullScan")}
          </label>
        </div>

        {!fullScan ? (
          <div>
            <label htmlFor="import-include-paths" className="joi-field__label mb-1 block">
              {t("settings.import.includePaths")}
            </label>
            <textarea
              id="import-include-paths"
              value={includePaths}
              onChange={(e) => setIncludePaths(e.target.value)}
              rows={4}
              className="joi-field__textarea"
            />
            <p className="joi-field__help mt-1">{t("settings.import.includePathsHelp")}</p>
          </div>
        ) : null}

        <button
          type="submit"
          disabled={running || previewing}
          className="joi-btn joi-btn--primary rounded-lg px-4 py-2"
        >
          {previewing ? t("settings.import.previewing") : t("settings.import.previewSubmit")}
        </button>
      </form>

      {previewing ? (
        <p role="status" className="joi-text--muted mt-4 text-sm">
          {t("settings.import.scanning")}
        </p>
      ) : null}

      {preview ? (
        <div className="joi-alert--warning mt-6 rounded-lg border p-4">
          <p className="joi-text--muted mb-3 text-xs">
            {t("import.fingerprintLabel")}{" "}
            <span className="font-mono">{preview.preview_fingerprint}</span>
          </p>
          <ImportReportView
            report={preview}
            decisions={decisions}
            onDecisionChange={handleDecisionChange}
          />
          <button
            type="button"
            onClick={() => void handleCommit()}
            disabled={running || committing || !allDecided}
            className="joi-btn joi-btn--primary mt-4 rounded-lg px-4 py-2"
          >
            {committing ? t("settings.import.committing") : t("settings.import.commitSubmit")}
          </button>
          {preview.review_items.length > 0 && !allDecided ? (
            <p className="joi-text--muted mt-2 text-xs">{t("settings.import.needDecisions")}</p>
          ) : null}
          <button
            type="button"
            onClick={() => void handleRun()}
            disabled={running}
            className="joi-btn joi-btn--secondary mt-4 ml-2 rounded-lg border px-4 py-2"
          >
            {running ? t("settings.import.importing") : t("settings.import.runSubmit")}
          </button>
        </div>
      ) : null}

      {running || status ? (
        <div
          role="status"
          aria-live="polite"
          className="joi-surface--base joi-border--default mt-6 rounded-lg border p-4"
        >
          <p className="joi-text text-sm font-medium">
            {t("settings.import.statusLine", {
              status: status
                ? isKnownStatus(status.status)
                  ? t(`common.labels.jobStatus.${status.status}`)
                  : status.status
                : t("settings.import.statusStarting"),
            })}
          </p>
          {running ? (
            <p className="joi-text--muted mt-1 text-sm">{t("settings.import.statusRunning")}</p>
          ) : null}
        </div>
      ) : null}

      {report ? (
        <div className="mt-6">
          <ImportReportView report={report} />
        </div>
      ) : null}

      {progressOpen && progress ? (
        <div className="mt-6">
          <JobProgressModal state={progress} onClose={() => setProgressOpen(false)} />
        </div>
      ) : null}
    </PageContainer>
  );
}

export default ImportSettingsPage;
