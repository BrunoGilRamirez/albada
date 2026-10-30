import { Link } from "react-router";
import { useTranslation } from "react-i18next";
import { formatNumber } from "../../lib/format";
import type { ProgressDetails } from "../../lib/types";
import ProgressBar from "./ProgressBar";

interface ProgressDetailsModalProps {
  open: boolean;
  title: string;
  details: ProgressDetails | null;
  onClose: () => void;
}

function ProgressDetailsModal({ open, title, details, onClose }: ProgressDetailsModalProps) {
  const { t } = useTranslation();
  if (!open) return null;

  // `count` selects the CLDR plural form while `countDisplay` is the
  // locale-formatted visible value (the shared 8.6.7/8.6.8 pattern). Point
  // totals are derived application numbers and go through the Intl formatter.
  const leafTaskCount = details?.contributors.length ?? 0;
  const summary = details
    ? t("common.progress.pointsSummary", {
        count: leafTaskCount,
        completed: formatNumber(details.completed_points),
        total: formatNumber(details.total_points),
        countDisplay: formatNumber(leafTaskCount),
      })
    : null;

  return (
    <div
      role="dialog"
      aria-modal="true"
      aria-label={t("common.progress.detailsFor", { title })}
      className="joi-dialog__backdrop z-50"
      onClick={onClose}
    >
      <div
        className="joi-dialog__panel max-h-[80vh] overflow-y-auto p-6"
        onClick={(event) => event.stopPropagation()}
      >
        <div className="flex items-start justify-between gap-4">
          <div>
            <h2 className="joi-dialog__title">{title}</h2>
            <p className="joi-text--muted mt-1 text-sm">{t("common.progress.explainer")}</p>
          </div>
          <button
            type="button"
            onClick={onClose}
            aria-label={t("common.progress.close")}
            className="joi-btn joi-btn--ghost joi-btn--sm"
          >
            ✕
          </button>
        </div>

        {details ? (
          <>
            <div className="joi-border--default mt-4 rounded-lg border p-4">
              <ProgressBar percentage={details.percentage} label={details.label} size="md" />
              <p className="joi-text mt-2 text-sm">{summary}</p>
            </div>

            {details.contributors.length > 0 ? (
              <ul className="mt-4 flex flex-col gap-2">
                {details.contributors.map((contributor) => (
                  <li
                    key={contributor.task_id}
                    className="joi-border--default flex items-center gap-3 rounded-lg border px-3 py-2"
                  >
                    <Link
                      to={`/tasks/${contributor.task_id}`}
                      className="joi-text min-w-0 flex-1 truncate text-sm hover:underline"
                    >
                      {contributor.title}
                    </Link>
                    <span
                      className={`joi-badge shrink-0 ${
                        contributor.completed ? "joi-badge--success" : "joi-badge--neutral"
                      }`}
                    >
                      {contributor.completed
                        ? t("common.progress.done")
                        : t(`common.labels.taskStatus.${contributor.status}`)}
                    </span>
                    <span className="joi-text--muted shrink-0 text-xs tabular-nums">
                      {t("common.progress.pointsValue", {
                        points: formatNumber(contributor.effort_points),
                      })}
                    </span>
                  </li>
                ))}
              </ul>
            ) : (
              <p className="joi-empty__state mt-4">{t("common.progress.empty")}</p>
            )}
          </>
        ) : (
          <p className="joi-empty__loading mt-4">{t("common.progress.loading")}</p>
        )}
      </div>
    </div>
  );
}

export default ProgressDetailsModal;
