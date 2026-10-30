import { useTranslation } from "react-i18next";
import AnalyticsTrendTable from "@/components/analytics/AnalyticsTrendTable";
import { formatDate } from "@/lib/format";
import type { RecurrenceAnalyticsSeries } from "@/lib/types";

// Outcome counters rendered side by side in a fixed order. None of them is
// color-coded: "not recorded" is a missing observation, never a confirmed
// failure, so no counter may look like a verdict.
const COUNT_KEYS = [
  "scheduled",
  "completed",
  "started",
  "skipped",
  "cancelled",
  "unrecorded",
] as const;

interface AnalyticsSeriesCardProps {
  series: RecurrenceAnalyticsSeries;
  /** Navigates to the routine editor; the card itself never mutates data. */
  onReview: () => void;
}

/**
 * Application-owned local date rendered through the shared Intl formatter with
 * a local-noon anchor (the established date-only pattern). The API returns
 * plain `YYYY-MM-DD` local dates, never timestamps.
 */
function formatLocalDate(date: string): string {
  return formatDate(new Date(`${date}T12:00:00`), { dateStyle: "medium" });
}

/**
 * One recurring routine's read-only history: coverage, outcome counters, the
 * completion rate with its definition, and the weekly trend. The copy stays
 * neutral and never turns a missing record into a failure.
 */
function AnalyticsSeriesCard({ series, onReview }: AnalyticsSeriesCardProps) {
  const { t } = useTranslation();

  // Only closed intervals are shown as pauses; an open interval is the
  // routine's current active stretch.
  const pausedIntervals = series.coverage.intervals.filter(
    (interval): interval is { from: string; until: string } => interval.until !== null,
  );
  const title = series.display_label ?? t("analytics.unnamedRoutine");

  return (
    <article
      className="joi-border--default rounded-lg border p-4"
      data-testid={`analytics-series-${series.series_id}`}
    >
      <div className="flex flex-wrap items-start justify-between gap-3">
        <div>
          <h2 className="text-lg font-medium">{title}</h2>
          {series.display_label_is_current_fallback ? (
            <p className="joi-text--muted mt-1 text-xs">{t("analytics.outOfRangeLabelNote")}</p>
          ) : null}
        </div>
        <p className="joi-text mt-1 text-sm font-medium">
          {series.rate
            ? t("analytics.rate.display", {
                completed: series.rate.completed,
                scheduled: series.rate.denominator,
                percent: series.rate.percent,
              })
            : t("analytics.rate.hidden")}
        </p>
      </div>

      <div className="mt-2 flex flex-col gap-1">
        {series.coverage.start ? (
          <p className="joi-text--muted text-xs">
            {t("analytics.coverage.since", { date: formatLocalDate(series.coverage.start) })}
          </p>
        ) : (
          <p className="joi-text--muted text-xs">{t("analytics.coverage.unknown")}</p>
        )}
        {pausedIntervals.map((interval) => (
          <p key={`${interval.from}-${interval.until}`} className="joi-text--muted text-xs">
            {t("analytics.coverage.paused", {
              from: formatLocalDate(interval.from),
              until: formatLocalDate(interval.until),
            })}
          </p>
        ))}
      </div>

      <dl className="mt-3 grid grid-cols-2 gap-2 sm:grid-cols-3 lg:grid-cols-6">
        {COUNT_KEYS.map((key) => (
          <div key={key} className="joi-border--default rounded-md border p-2">
            <dt className="joi-text--muted text-xs">{t(`analytics.counts.${key}`)}</dt>
            <dd className="mt-1 text-lg font-medium">{series.counts[key]}</dd>
          </div>
        ))}
      </dl>

      <AnalyticsTrendTable weekly={series.weekly} />

      <p className="joi-text--muted mt-3 text-xs">{t("analytics.definition")}</p>
      <p className="joi-text--muted mt-1 text-xs">{t("analytics.unrecordedNote")}</p>

      <button type="button" onClick={onReview} className="joi-btn joi-btn--secondary mt-3">
        {t("analytics.reviewRoutine")}
      </button>
    </article>
  );
}

export default AnalyticsSeriesCard;
