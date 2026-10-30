import { useTranslation } from "react-i18next";
import { formatDate } from "@/lib/format";
import type { RecurrenceAnalyticsWeekBucket } from "@/lib/types";

// The weekly trend reuses the same outcome counters as the totals, in the
// same fixed order, so a reader can compare a row with the card totals.
const COUNT_KEYS = [
  "scheduled",
  "completed",
  "started",
  "skipped",
  "cancelled",
  "unrecorded",
] as const;

interface AnalyticsTrendTableProps {
  weekly: RecurrenceAnalyticsWeekBucket[];
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
 * Weekly trend rendered as a real table: the caption names the table, the
 * column headers name the outcome counters, and the week cell is a row header
 * so a screen reader announces the week with every value. Bucket values come
 * straight from the API and are never recomputed here; a week without
 * activity still renders its row.
 */
function AnalyticsTrendTable({ weekly }: AnalyticsTrendTableProps) {
  const { t } = useTranslation();

  return (
    <table className="mt-4 w-full border-collapse text-sm">
      <caption className="joi-text--muted pb-2 text-left text-xs">
        {t("analytics.trend.title")}
      </caption>
      <thead>
        <tr>
          <th
            scope="col"
            className="joi-border--default border-b px-2 py-1 text-left text-xs font-medium"
          >
            {t("analytics.trend.weekStarting")}
          </th>
          {COUNT_KEYS.map((key) => (
            <th
              key={key}
              scope="col"
              className="joi-border--default border-b px-2 py-1 text-left text-xs font-medium"
            >
              {t(`analytics.counts.${key}`)}
            </th>
          ))}
        </tr>
      </thead>
      <tbody>
        {weekly.map((bucket) => (
          <tr key={bucket.week_start}>
            <th
              scope="row"
              className="joi-border--default border-b px-2 py-1 text-left text-xs font-normal"
            >
              {formatLocalDate(bucket.week_start)}
            </th>
            {COUNT_KEYS.map((key) => (
              <td key={key} className="joi-border--default border-b px-2 py-1 text-xs">
                {bucket[key]}
              </td>
            ))}
          </tr>
        ))}
      </tbody>
    </table>
  );
}

export default AnalyticsTrendTable;
