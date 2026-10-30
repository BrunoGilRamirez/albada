import { useTranslation } from "react-i18next";
import { formatNumber } from "@/lib/format";
import type { ExportArtifact, ExportJob, ExportWarning } from "@/lib/types";

interface ExportJobStatusProps {
  job: ExportJob | null;
  onOpenArtifact?: (artifact: ExportArtifact) => void;
  onSaveCopyArtifact?: (artifact: ExportArtifact) => void;
}

// Presentation-only maps. Every status and severity keeps its own translated
// label in JSX (`common.exportJob.*`, `common.labels.jobStatus.*`,
// `common.labels.severity.*`), so the chip is never the only cue.
const STATUS_STYLES: Record<string, string> = {
  queued: "joi-badge--neutral",
  running: "joi-badge--warning",
  completed: "joi-badge--success",
  failed: "joi-badge--danger",
  cancelled: "joi-badge--neutral",
};

// Native job statuses with a localized label. Anything else falls back to the
// raw stored value so an unknown backend status is never keyed into the
// catalogs.
const KNOWN_STATUSES = ["queued", "running", "completed", "failed", "cancelled"] as const;
type KnownStatus = (typeof KNOWN_STATUSES)[number];

function isKnownStatus(value: string): value is KnownStatus {
  return (KNOWN_STATUSES as readonly string[]).includes(value);
}

const ARTIFACT_FORMAT_KEYS = ["md", "normalized_md", "typ", "pdf", "diagnostics"] as const;
type KnownArtifactFormat = (typeof ARTIFACT_FORMAT_KEYS)[number];

function isKnownArtifactFormat(value: string): value is KnownArtifactFormat {
  return (ARTIFACT_FORMAT_KEYS as readonly string[]).includes(value);
}

const WARNING_SEVERITY_STYLES: Record<string, string> = {
  info: "joi-badge--info",
  warning: "joi-badge--warning",
  error: "joi-badge--danger",
};

/** Compact, non-path representation of a checksum (first 8 chars). */
function shortChecksum(checksum: string | null): string | null {
  if (!checksum) return null;
  return checksum.length > 8 ? `${checksum.slice(0, 8)}…` : checksum;
}

function warningLocation(warning: ExportWarning): string {
  if (!warning.source) return "";
  const location = warning.line
    ? `:${warning.line}${warning.column ? `:${warning.column}` : ""}`
    : "";
  return `${warning.source}${location}`;
}

export function ExportJobStatus({ job, onOpenArtifact, onSaveCopyArtifact }: ExportJobStatusProps) {
  const { t } = useTranslation();
  if (!job) return null;

  const statusLabel = isKnownStatus(job.status)
    ? t(`common.labels.jobStatus.${job.status}`)
    : job.status;
  const errorCount = job.warnings.filter((warning) => warning.severity === "error").length;
  const warningCount = job.warnings.filter((warning) => warning.severity === "warning").length;
  const infoCount = job.warnings.filter((warning) => warning.severity === "info").length;
  const diagnosticSummary = job.warnings
    .map((warning) => {
      const location = warning.source
        ? `${warning.source}${warning.line ? `:${warning.line}${warning.column ? `:${warning.column}` : ""}` : ""}`
        : "";
      return [warning.severity, warning.phase, warning.category, location, warning.code]
        .filter(Boolean)
        .join(" · ");
    })
    .join("\n");
  const groupedWarnings = Object.entries(
    job.warnings.reduce<Record<string, ExportWarning[]>>((groups, warning) => {
      const group = warning.phase ?? "general";
      (groups[group] ??= []).push(warning);
      return groups;
    }, {}),
  );

  /**
   * Human-readable byte size. The unit travels with the value inside the key
   * so each locale can order value and unit correctly; the value itself is
   * locale-formatted through the shared Intl number formatter.
   */
  const sizeLabel = (bytes: number | null): string => {
    if (bytes === null || bytes === undefined) return t("common.exportJob.unknownSize");
    if (bytes < 1024) return t("common.exportJob.bytes", { size: formatNumber(bytes) });
    const digits = { minimumFractionDigits: 1, maximumFractionDigits: 1 } as const;
    if (bytes < 1024 * 1024) {
      return t("common.exportJob.kilobytes", { size: formatNumber(bytes / 1024, digits) });
    }
    return t("common.exportJob.megabytes", { size: formatNumber(bytes / (1024 * 1024), digits) });
  };

  return (
    <div
      className="joi-border--default rounded-lg border p-4"
      role="status"
      aria-live="polite"
      aria-label={t("common.exportJob.ariaLabel", { status: statusLabel })}
    >
      <div className="flex items-center justify-between">
        <h3 className="text-sm font-medium">{t("common.exportJob.title")}</h3>
        <span className={`joi-badge ${STATUS_STYLES[job.status] ?? STATUS_STYLES.queued}`}>
          {statusLabel}
        </span>
      </div>

      {job.error_message ? (
        <div className="joi-alert--error mt-3 rounded-md p-2 text-xs">
          <p className="font-medium">{t("common.exportJob.failedSourceSaved")}</p>
          {/* Native failure text is application-owned diagnostics, not copy. */}
          <p className="mt-1">{job.error_message}</p>
        </div>
      ) : null}

      {job.warnings.length > 0 ? (
        <div className="mt-3">
          <h4 className="joi-text--muted text-xs font-medium">
            {t("common.exportJob.diagnostics", { count: job.warnings.length })}{" "}
            {t("common.exportJob.diagnosticCounts", {
              errors: formatNumber(errorCount),
              warnings: formatNumber(warningCount),
              info: formatNumber(infoCount),
            })}
          </h4>
          {job.warnings.length > 0 && typeof navigator !== "undefined" ? (
            <button
              type="button"
              className="joi-interactive joi-text mt-1 text-xs font-medium underline"
              onClick={() => void navigator.clipboard?.writeText(diagnosticSummary)}
            >
              {t("common.exportJob.copyDiagnostics")}
            </button>
          ) : null}
          <ul className="mt-1 flex flex-col gap-2">
            {groupedWarnings.map(([phase, warnings]) => (
              <li key={phase}>
                {/* Phase is native diagnostic data and stays verbatim. */}
                <h5 className="joi-text--muted text-[11px] font-semibold uppercase">{phase}</h5>
                <ul className="mt-1 flex flex-col gap-1">
                  {warnings.map((warning, index) => (
                    <li
                      key={`${warning.code}-${index}`}
                      className="joi-text--muted flex items-start gap-2 text-xs"
                    >
                      <span
                        className={`shrink-0 rounded px-1.5 py-0.5 font-medium uppercase ${WARNING_SEVERITY_STYLES[warning.severity] ?? WARNING_SEVERITY_STYLES.info}`}
                      >
                        {t(`common.labels.severity.${warning.severity}`)}
                      </span>
                      <span>
                        {warning.message}
                        {warning.category || warningLocation(warning) ? (
                          <span className="joi-text--muted ml-2">
                            {[warning.category, warningLocation(warning)]
                              .filter(Boolean)
                              .join(" · ")}
                          </span>
                        ) : null}
                      </span>
                    </li>
                  ))}
                </ul>
              </li>
            ))}
          </ul>
        </div>
      ) : null}

      {job.artifacts.length > 0 ? (
        <div className="mt-3">
          <h4 className="joi-text--muted text-xs font-medium">
            {t("common.exportJob.localArtifacts", { count: job.artifacts.length })}
          </h4>
          <ul className="mt-1 flex flex-col gap-1">
            {job.artifacts.map((artifact) => {
              const checksum = shortChecksum(artifact.checksum);
              const size = sizeLabel(artifact.size_bytes);
              return (
                <li
                  key={artifact.id}
                  className="joi-text--muted flex items-center justify-between text-xs"
                >
                  <span className="joi-badge--neutral rounded px-1.5 py-0.5 font-medium uppercase">
                    {t(
                      isKnownArtifactFormat(artifact.format)
                        ? `common.exportJob.formats.${artifact.format}`
                        : "common.exportJob.formats.unknown",
                    )}
                  </span>
                  <span className="flex items-center gap-2">
                    {onOpenArtifact ? (
                      <button
                        type="button"
                        onClick={() => onOpenArtifact(artifact)}
                        className="joi-interactive joi-text font-medium underline"
                      >
                        {t("common.exportJob.open")}
                      </button>
                    ) : null}
                    {onSaveCopyArtifact ? (
                      <button
                        type="button"
                        onClick={() => onSaveCopyArtifact(artifact)}
                        className="joi-interactive joi-surface--raised rounded px-1 py-0.5 text-xs font-medium"
                      >
                        {t("common.exportHistory.saveCopy")}
                      </button>
                    ) : null}
                    <span>{size}</span>
                    {checksum ? (
                      <code className="joi-badge--neutral rounded px-1.5 py-0.5 font-mono">
                        {checksum}
                      </code>
                    ) : null}
                  </span>
                </li>
              );
            })}
          </ul>
        </div>
      ) : null}
    </div>
  );
}

export default ExportJobStatus;
