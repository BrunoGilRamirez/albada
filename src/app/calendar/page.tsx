import { useCallback, useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import CalendarGrid from "@/components/calendar/CalendarGrid";
import PageContainer from "@/components/navigation/PageContainer";
import { listBlocksBetween } from "@/lib/api/blockApi";
import { listDailyPlansBetween } from "@/lib/api/dailyPlanApi";
import { listTasks } from "@/lib/api/taskApi";
import { ensureDefaultWorkspace } from "@/lib/api/workspaceApi";
import { toDisplayMessage } from "@/lib/errors";
import { formatDate as intlFormatDate } from "@/lib/format";
import type { Block, DailyPlan, Task } from "@/lib/types";

function pad(value: number): string {
  return String(value).padStart(2, "0");
}

function isoDate(year: number, month: number, day: number): string {
  return `${year}-${pad(month + 1)}-${pad(day)}`;
}

function localToday(): string {
  const now = new Date();
  return isoDate(now.getFullYear(), now.getMonth(), now.getDate());
}

function CalendarPage() {
  const { t } = useTranslation();
  const now = new Date();
  const [year, setYear] = useState(now.getFullYear());
  const [month, setMonth] = useState(now.getMonth());
  const [mode, setMode] = useState<"month" | "week">("month");
  const [plans, setPlans] = useState<DailyPlan[]>([]);
  const [blocks, setBlocks] = useState<Block[]>([]);
  const [deadlines, setDeadlines] = useState<Task[]>([]);
  const [error, setError] = useState<string | null>(null);

  const today = localToday();

  const range = useCallback(() => {
    const first = isoDate(year, month, 1);
    const last = isoDate(year, month, new Date(year, month + 1, 0).getDate());
    return { first, last };
  }, [year, month]);

  const load = useCallback(async () => {
    setError(null);
    try {
      const workspace = await ensureDefaultWorkspace();
      const { first, last } = range();
      const [rangePlans, rangeBlocks, tasks] = await Promise.all([
        listDailyPlansBetween(workspace.id, first, last),
        listBlocksBetween(workspace.id, first, last),
        listTasks(workspace.id),
      ]);
      setPlans(rangePlans);
      setBlocks(rangeBlocks);
      setDeadlines(
        tasks.filter(
          (task) => task.target_date && task.target_date >= first && task.target_date <= last,
        ),
      );
    } catch (err) {
      setError(toDisplayMessage(err));
    }
  }, [range]);

  useEffect(() => {
    void load();
  }, [load]);

  const shiftMonth = (delta: number) => {
    const next = new Date(year, month + delta, 1);
    setYear(next.getFullYear());
    setMonth(next.getMonth());
  };

  const planDates = new Set(plans.map((plan) => plan.date));
  const blockDates = new Set(blocks.map((block) => block.date));
  const deadlineDates = new Set(deadlines.map((task) => task.target_date as string));

  // Month title is an application-owned date: Intl renders it in the UI
  // language (e.g. "October 2026" / "octubre de 2026" / "Oktober 2026").
  const monthLabel = intlFormatDate(new Date(year, month, 1), {
    month: "long",
    year: "numeric",
  });

  return (
    <PageContainer className="flex h-full flex-col">
      <div className="flex flex-wrap items-center justify-between gap-3">
        <div>
          <h1 className="text-2xl font-semibold">{t("calendar.title")}</h1>
          <p className="joi-text--muted mt-1 text-sm">{monthLabel}</p>
        </div>
        <div className="flex items-center gap-1">
          <button
            type="button"
            onClick={() => shiftMonth(-1)}
            className="joi-btn joi-btn--secondary rounded-md border px-2 py-1 text-sm"
          >
            {t("common.actions.previous")}
          </button>
          <button
            type="button"
            onClick={() => shiftMonth(1)}
            className="joi-btn joi-btn--secondary rounded-md border px-2 py-1 text-sm"
          >
            {t("common.actions.next")}
          </button>
          <span className="joi-border--strong mx-2 h-4 border-l" />
          <button
            type="button"
            onClick={() => setMode(mode === "month" ? "week" : "month")}
            className="joi-btn joi-btn--secondary rounded-md border px-2 py-1 text-sm"
          >
            {mode === "month" ? t("calendar.weekView") : t("calendar.monthView")}
          </button>
        </div>
      </div>

      <div className="joi-text--muted mt-4 flex gap-4 text-xs">
        <span className="flex items-center gap-1">
          <span aria-hidden="true" className="joi-dot joi-dot--info" />
          {t("calendar.legend.dailyPlan")}
        </span>
        <span className="flex items-center gap-1">
          <span aria-hidden="true" className="joi-dot joi-dot--warning" />
          {t("calendar.legend.blocks")}
        </span>
        <span className="flex items-center gap-1">
          <span aria-hidden="true" className="joi-dot joi-dot--danger" />
          {t("calendar.legend.deadlines")}
        </span>
      </div>

      {error ? (
        <p role="alert" className="joi-text--danger mt-4">
          {error}
        </p>
      ) : null}

      <div className="mt-4 min-h-0 flex-1">
        <CalendarGrid
          year={year}
          month={month}
          mode={mode}
          today={today}
          planDates={planDates}
          blockDates={blockDates}
          deadlineDates={deadlineDates}
          onSelectDate={() => void 0}
        />
      </div>
    </PageContainer>
  );
}

export default CalendarPage;
