import { useTranslation } from "react-i18next";
import type { ExportArtifact, ExportJob } from "@/lib/types";
import { formatDateTime } from "@/lib/format";

interface ExportHistoryProps {
  jobs: ExportJob[];
  onOpenArtifact: (artifact: ExportArtifact) => void;
  onSaveCopyArtifact: (artifact: ExportArtifact) => void;
  onDeleteJob: (job: ExportJob) => void | Promise<void>;
}

const KNOWN_FORMAT_KEYS = ["md", "typ", "pdf"] as const;
type KnownFormatKey = (typeof KNOWN_FORMAT_KEYS)[number];

function isKnownFormatKey(value: string): value is KnownFormatKey {
  return (KNOWN_FORMAT_KEYS as readonly string[]).includes(value);
}

const KNOWN_STATUSES = ["queued", "running", "completed", "failed", "cancelled"] as const;
type KnownStatus = (typeof KNOWN_STATUSES)[number];

function isKnownStatus(value: string): value is KnownStatus {
  return (KNOWN_STATUSES as readonly string[]).includes(value);
}

/** Displays previous export jobs without exposing absolute local paths. */
export function ExportHistory({
  jobs,
  onOpenArtifact,
  onSaveCopyArtifact,
  onDeleteJob,
}: ExportHistoryProps) {
  const { t } = useTranslation();

  const formatLabel = (format: string): string =>
    isKnownFormatKey(format) ? t(`common.exportHistory.formats.${format}`) : format.toUpperCase();

  return (
    <section className="joi-border--default mt-8 rounded-lg border p-4">
      <div className="flex items-center justify-between gap-3">
        <div>
          <h2 className="text-base font-semibold">{t("common.exportHistory.title")}</h2>
          <p className="joi-text--muted mt-1 text-xs">{t("common.exportHistory.subtitle")}</p>
        </div>
        <span className="joi-text--muted text-xs">
          {t("common.exportHistory.count", { count: jobs.length })}
        </span>
      </div>

      {jobs.length === 0 ? (
        <p className="joi-text--muted mt-4 text-sm">{t("common.exportHistory.empty")}</p>
      ) : (
        <div className="mt-4 flex flex-col gap-3">
          {jobs.map((job) => (
            <article key={job.id} className="joi-border--default rounded-md border p-3">
              <div className="flex flex-wrap items-start justify-between gap-2">
                <div>
                  {/* The job title is application-owned export metadata supplied
                      by the caller; it is rendered verbatim. */}
                  <h3 className="text-sm font-medium">{job.title}</h3>
                  <p className="joi-text--muted mt-1 text-xs">
                    {formatDateTime(job.created_at)} ·{" "}
                    {isKnownStatus(job.status)
                      ? t(`common.labels.jobStatus.${job.status}`)
                      : job.status}
                  </p>
                </div>
                <div className="flex items-center gap-2">
                  <span className="joi-badge joi-badge--neutral">
                    {job.export_type.toUpperCase()}
                  </span>
                  <button
                    type="button"
                    disabled={job.status === "running"}
                    onClick={() => {
                      if (
                        window.confirm(
                          t("common.exportHistory.deleteConfirm", { title: job.title }),
                        )
                      ) {
                        void onDeleteJob(job);
                      }
                    }}
                    className="joi-btn joi-btn--danger rounded-md px-2.5 py-1 text-xs font-medium"
                  >
                    {t("common.actions.delete")}
                  </button>
                </div>
              </div>

              {job.error_message ? (
                <p className="joi-text--danger mt-2 text-xs">{job.error_message}</p>
              ) : null}

              {job.artifacts.length > 0 ? (
                <div className="mt-3 flex flex-wrap gap-2">
                  {job.artifacts.map((artifact) => (
                    <div className="flex gap-2" key={artifact.id}>
                      <button
                        type="button"
                        onClick={() => onOpenArtifact(artifact)}
                        className="joi-btn joi-btn--secondary rounded-md px-2.5 py-1 text-xs font-medium"
                      >
                        {t("common.exportHistory.openFormat", {
                          format: formatLabel(artifact.format),
                        })}
                      </button>
                      <button
                        type="button"
                        onClick={() => onSaveCopyArtifact(artifact)}
                        className="joi-btn joi-btn--secondary rounded-md px-2.5 py-1 text-xs font-medium"
                      >
                        {t("common.exportHistory.saveCopy")}
                      </button>
                    </div>
                  ))}
                </div>
              ) : (
                <p className="joi-text--muted mt-3 text-xs">
                  {t("common.exportHistory.noArtifact")}
                </p>
              )}
            </article>
          ))}
        </div>
      )}
    </section>
  );
}

export default ExportHistory;
