import { useCallback, useEffect, useState } from "react";
import { useSearchParams } from "react-router";
import { useTranslation } from "react-i18next";
import PlanningRitualModal from "@/components/today/PlanningRitualModal";
import PageContainer from "@/components/navigation/PageContainer";
import TodayBlockList from "@/components/today/TodayBlockList";
import TodayTaskList from "@/components/today/TodayTaskList";
import DeadlineWarningList from "@/components/deadlines/DeadlineWarningList";
import { listAtRiskTasks } from "@/lib/api/deadlineApi";
import { getOrCreateDailyPlan, listDayTasks, updateDailyPlanStatus } from "@/lib/api/dailyPlanApi";
import { listBlocksForDate } from "@/lib/api/blockApi";
import { listFixedCommitments } from "@/lib/api/fixedCommitmentApi";
import { listTasks } from "@/lib/api/taskApi";
import { ensureDefaultWorkspace } from "@/lib/api/workspaceApi";
import { toDisplayMessage } from "@/lib/errors";
import { formatDate as intlFormatDate } from "@/lib/format";
import { DAILY_PLAN_STATUSES } from "@/lib/types";
import type {
  Block,
  DailyPlan,
  DailyPlanTask,
  DeadlineTaskInfo,
  FixedCommitment,
  Task,
} from "@/lib/types";

function localToday(): string {
  const now = new Date();
  const year = now.getFullYear();
  const month = String(now.getMonth() + 1).padStart(2, "0");
  const day = String(now.getDate()).padStart(2, "0");
  return `${year}-${month}-${day}`;
}

function formatDate(date: string): string {
  return intlFormatDate(new Date(`${date}T12:00:00`), {
    weekday: "long",
    year: "numeric",
    month: "long",
    day: "numeric",
  });
}

function TodayPage() {
  const { t } = useTranslation();
  const [searchParams] = useSearchParams();
  const date = searchParams.get("date") ?? localToday();

  const [workspaceId, setWorkspaceId] = useState<string | null>(null);
  const [plan, setPlan] = useState<DailyPlan | null>(null);
  const [planTasks, setPlanTasks] = useState<DailyPlanTask[]>([]);
  const [tasks, setTasks] = useState<Task[]>([]);
  const [blocks, setBlocks] = useState<Block[]>([]);
  const [commitments, setCommitments] = useState<FixedCommitment[]>([]);
  const [atRisk, setAtRisk] = useState<DeadlineTaskInfo[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [ritualOpen, setRitualOpen] = useState(false);
  const [busy, setBusy] = useState(false);

  const taskById = new Map(tasks.map((task) => [task.id, task]));

  const load = useCallback(
    async (guard?: { cancelled: boolean }) => {
      const isStale = () => guard?.cancelled === true;
      setError(null);
      try {
        const workspace = await ensureDefaultWorkspace();
        if (isStale()) return;
        setWorkspaceId(workspace.id);
        // Today only loads execution/time data (tasks, blocks, and commitments); it never calls therapy commands.
        const [dailyPlan, workspaceTasks, dayBlocks, risk, dayCommitments] = await Promise.all([
          getOrCreateDailyPlan(date),
          listTasks(workspace.id),
          listBlocksForDate(workspace.id, date),
          listAtRiskTasks(workspace.id),
          listFixedCommitments(workspace.id, true),
        ]);
        if (isStale()) return;
        setPlan(dailyPlan);
        const rows = await listDayTasks(dailyPlan.id);
        if (isStale()) return;
        setPlanTasks(rows);
        setTasks(workspaceTasks);
        setBlocks(dayBlocks);
        setAtRisk(risk);
        setCommitments(dayCommitments);
      } catch (err) {
        if (!isStale()) setError(toDisplayMessage(err));
      }
    },
    [date],
  );

  useEffect(() => {
    // StrictMode mounts effects twice in development. The guard stops the
    // first (superseded) run from writing state after cleanup, so only the
    // live run's results reach the UI. Refreshes triggered by child edits
    // pass no guard and always apply.
    const guard = { cancelled: false };
    void load(guard);
    return () => {
      guard.cancelled = true;
    };
  }, [load]);

  const reload = () => void load();

  const handleStatusChange = async (status: string) => {
    if (!plan) return;
    setError(null);
    setBusy(true);
    try {
      setPlan(await updateDailyPlanStatus(plan.id, status));
    } catch (err) {
      setError(toDisplayMessage(err));
    } finally {
      setBusy(false);
    }
  };

  const isToday = date === localToday();

  return (
    <PageContainer>
      <div className="flex flex-wrap items-start justify-between gap-3">
        <div>
          <h1 className="text-2xl font-semibold">{t("today.title")}</h1>
          <p className="joi-text--muted mt-1 text-sm">
            {formatDate(date)}
            {!isToday ? ` ${t("today.notToday")}` : ""}
          </p>
        </div>
        <button
          type="button"
          data-notification-target="planning"
          onClick={() => setRitualOpen(true)}
          className="joi-btn joi-btn--primary rounded-md px-3 py-2 text-sm font-medium"
        >
          {t("today.planningRitual")}
        </button>
      </div>

      {error ? (
        <p role="alert" className="joi-text--danger mt-2">
          {error}
        </p>
      ) : null}

      {plan ? (
        <div className="mt-4 flex items-center gap-2">
          <label htmlFor="plan-status" className="joi-text text-sm font-medium">
            {t("today.planStatus")}
          </label>
          <select
            id="plan-status"
            disabled={busy}
            value={plan.status}
            onChange={(e) => handleStatusChange(e.target.value)}
            className="joi-field__select w-auto rounded-md border px-2 py-1 text-sm disabled:opacity-50"
          >
            {DAILY_PLAN_STATUSES.map((status) => (
              <option key={status} value={status}>
                {t(`today.planStatuses.${status}`)}
              </option>
            ))}
          </select>
        </div>
      ) : null}

      <div className="mt-6 grid gap-6 xl:grid-cols-12">
        <div className="flex flex-col gap-6 xl:col-span-7 3xl:col-span-8">
          <div>
            <h2 className="mb-2 text-lg font-medium">{t("today.selectedTasks")}</h2>
            {plan ? (
              <TodayTaskList
                dailyPlanId={plan.id}
                rows={planTasks}
                taskById={taskById}
                onDataChanged={reload}
              />
            ) : (
              <div role="status" aria-label={t("common.loading")} className="flex flex-col gap-2">
                <span className="sr-only">{t("common.loading")}</span>
                <div aria-hidden="true" className="joi-skeleton h-10 w-full" />
                <div aria-hidden="true" className="joi-skeleton h-10 w-11/12" />
                <div aria-hidden="true" className="joi-skeleton h-10 w-4/5" />
              </div>
            )}
          </div>

          <div>
            <h2 data-notification-target="focus" className="mb-2 text-lg font-medium">
              {t("today.blocks.title")}
            </h2>
            {workspaceId ? (
              <TodayBlockList
                workspaceId={workspaceId}
                date={date}
                dailyPlanId={plan?.id ?? null}
                blocks={blocks}
                onChanged={setBlocks}
                onRefresh={reload}
              />
            ) : null}
          </div>
        </div>

        <div className="flex flex-col gap-6 xl:col-span-5 3xl:col-span-4">
          <div>
            <h2 className="mb-2 text-lg font-medium">{t("today.deadlineWarnings")}</h2>
            <DeadlineWarningList tasks={atRisk} />
          </div>

          <div>
            <h2 className="mb-2 text-lg font-medium">{t("today.fixedCommitments")}</h2>
            {commitments.length === 0 ? (
              <p className="joi-empty__state">{t("today.noCommitments")}</p>
            ) : (
              <ul className="flex flex-col gap-1">
                {commitments.map((commitment) => (
                  <li key={commitment.id} className="joi-text flex items-center gap-2 text-sm">
                    <span className="joi-badge joi-badge--neutral">
                      {t(`common.labels.commitmentType.${commitment.commitment_type}`)}
                    </span>
                    <span className="truncate">{commitment.title}</span>
                    {[commitment.start_at, commitment.end_at].filter(Boolean).length > 0 ? (
                      <span className="joi-text--muted shrink-0 text-xs">
                        {[commitment.start_at, commitment.end_at].filter(Boolean).join(" – ")}
                      </span>
                    ) : null}
                  </li>
                ))}
              </ul>
            )}
          </div>
        </div>
      </div>

      {ritualOpen && plan && workspaceId ? (
        <PlanningRitualModal
          workspaceId={workspaceId}
          date={date}
          plan={plan}
          existingPlanTasks={planTasks}
          onCompleted={reload}
          onClose={() => setRitualOpen(false)}
        />
      ) : null}
    </PageContainer>
  );
}

export default TodayPage;
