import { useCallback, useEffect, useMemo, useState, type FormEvent } from "react";
import { useNavigate } from "react-router";
import { useTranslation } from "react-i18next";
import AnalyticsSeriesCard from "@/components/analytics/AnalyticsSeriesCard";
import PageContainer from "@/components/navigation/PageContainer";
import PageSkeleton from "@/components/navigation/PageSkeleton";
import { getBlockRecurrenceAnalytics } from "@/lib/api/blockApi";
import { ensureDefaultWorkspace } from "@/lib/api/workspaceApi";
import { localTodayDate } from "@/lib/blockTaskEligibility";
import { toDisplayMessage } from "@/lib/errors";
import { formatDate } from "@/lib/format";
import { blockRecurrenceAnalyticsInputSchema } from "@/lib/schemas/schemas";
import type { RecurrenceAnalyticsResponse } from "@/lib/types";

interface AnalyticsRange {
  from: string;
  to: string;
}

/** Length of the default window: the last 30 local days, inclusive. */
const DEFAULT_RANGE_DAYS = 30;

/**
 * Shift a local `YYYY-MM-DD` date by whole days. The date is anchored at
 * local noon so a daylight-saving change never moves the calendar day.
 */
function shiftDays(date: string, days: number): string {
  const [year, month, day] = date.split("-").map(Number);
  const shifted = new Date(year, month - 1, day, 12, 0, 0);
  shifted.setDate(shifted.getDate() + days);
  const shiftedYear = shifted.getFullYear();
  const shiftedMonth = String(shifted.getMonth() + 1).padStart(2, "0");
  const shiftedDay = String(shifted.getDate()).padStart(2, "0");
  return `${shiftedYear}-${shiftedMonth}-${shiftedDay}`;
}

/** Default window: today and the 29 days before it. */
function defaultRange(): AnalyticsRange {
  const to = localTodayDate();
  return { from: shiftDays(to, -(DEFAULT_RANGE_DAYS - 1)), to };
}

/**
 * Application-owned local date rendered through the shared Intl formatter with
 * a local-noon anchor (the established date-only pattern).
 */
function formatLocalDate(date: string): string {
  return formatDate(new Date(`${date}T12:00:00`), { dateStyle: "medium" });
}

/**
 * Read-only habit analytics for recurring block series (8.7.13).
 *
 * The screen reads history and nothing else: the only calls are the default
 * workspace lookup and the read-only analytics command. It never changes a
 * block, a routine, a task, or a notification; the single action it offers is
 * navigating to the Today view, where the routine editor lives.
 */
function AnalyticsPage() {
  const { t } = useTranslation();
  const navigate = useNavigate();

  const initialRange = useMemo(() => defaultRange(), []);
  const [range, setRange] = useState<AnalyticsRange>(initialRange);
  const [fromInput, setFromInput] = useState(initialRange.from);
  const [toInput, setToInput] = useState(initialRange.to);
  const [response, setResponse] = useState<RecurrenceAnalyticsResponse | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [rangeError, setRangeError] = useState<string | null>(null);

  const load = useCallback(async (target: AnalyticsRange, guard?: { cancelled: boolean }) => {
    const isStale = () => guard?.cancelled === true;
    setError(null);
    setLoading(true);
    try {
      const workspace = await ensureDefaultWorkspace();
      if (isStale()) return;
      const result = await getBlockRecurrenceAnalytics(workspace.id, target.from, target.to);
      if (isStale()) return;
      setResponse(result);
    } catch (err) {
      if (!isStale()) setError(toDisplayMessage(err));
    } finally {
      if (!isStale()) setLoading(false);
    }
  }, []);

  useEffect(() => {
    // StrictMode mounts effects twice in development. The guard stops the
    // first (superseded) run from writing state after cleanup, so only the
    // live run's results reach the UI.
    const guard = { cancelled: false };
    void load(range, guard);
    return () => {
      guard.cancelled = true;
    };
  }, [load, range]);

  const handleApply = (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    // The shared schema validates the request shape; only the two local dates
    // are user input here, so the workspace id is a placeholder. The real id
    // comes from `ensureDefaultWorkspace` inside `load`.
    const parsed = blockRecurrenceAnalyticsInputSchema.safeParse({
      workspace_id: "local",
      from_date: fromInput,
      to_date: toInput,
    });
    if (!parsed.success) {
      // An inverted or malformed range never reaches the command.
      setRangeError(t("analytics.range.invalid"));
      return;
    }
    setRangeError(null);
    setRange({ from: fromInput, to: toInput });
  };

  const series = response?.series ?? [];

  return (
    <PageContainer>
      <h1 className="text-2xl font-semibold">{t("analytics.title")}</h1>
      <p className="joi-text--muted mt-1 text-sm">{t("analytics.subtitle")}</p>

      <form
        onSubmit={handleApply}
        className="joi-border--default mt-4 flex flex-wrap items-end gap-3 rounded-lg border p-3"
      >
        <div>
          <label htmlFor="analytics-from" className="joi-field__label">
            {t("analytics.range.from")}
          </label>
          <input
            id="analytics-from"
            type="date"
            className="joi-field__input"
            value={fromInput}
            onChange={(event) => setFromInput(event.target.value)}
          />
        </div>
        <div>
          <label htmlFor="analytics-to" className="joi-field__label">
            {t("analytics.range.to")}
          </label>
          <input
            id="analytics-to"
            type="date"
            className="joi-field__input"
            value={toInput}
            onChange={(event) => setToInput(event.target.value)}
          />
        </div>
        <button type="submit" className="joi-btn joi-btn--secondary">
          {t("common.actions.apply")}
        </button>
      </form>

      {rangeError ? (
        <p role="alert" className="joi-text--danger mt-2 text-sm">
          {rangeError}
        </p>
      ) : null}

      {error ? (
        <p role="alert" className="joi-text--danger mt-2 text-sm">
          {error}
        </p>
      ) : null}

      {loading ? <PageSkeleton variant="analytics" /> : null}

      {response ? (
        <p className="joi-text--muted mt-3 text-sm">
          {t("analytics.range.showing", {
            from: formatLocalDate(response.effective_from),
            to: formatLocalDate(response.effective_to),
          })}
        </p>
      ) : null}

      {response && series.length === 0 ? (
        <p className="joi-empty__state mt-4">{t("analytics.empty")}</p>
      ) : null}

      {series.length > 0 ? (
        <div className="mt-4 flex flex-col gap-4">
          {series.map((entry) => (
            <AnalyticsSeriesCard
              key={entry.series_id}
              series={entry}
              onReview={() => navigate("/")}
            />
          ))}
        </div>
      ) : null}
    </PageContainer>
  );
}

export default AnalyticsPage;
