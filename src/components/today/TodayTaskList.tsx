import { useMemo, useState } from "react";
import { Link } from "react-router";
import { useTranslation } from "react-i18next";
import { deselectTaskForDay, replanTask, reorderDayTasks } from "../../lib/api/dailyPlanApi";
import { completeTask, transitionTaskStatus } from "../../lib/api/taskApi";
import { toDisplayMessage } from "../../lib/errors";
import { formatDate as intlFormatDate } from "../../lib/format";
import type { DailyPlanTask, ReplanDecision, Task } from "../../lib/types";
import { REPLAN_DECISIONS } from "../../lib/types";
import SessionTracker from "../sessions/SessionTracker";

interface TodayTaskListProps {
  dailyPlanId: string;
  rows: DailyPlanTask[];
  taskById: Map<string, Task>;
  onDataChanged: () => void;
}

interface ReplanMenuState {
  planTaskId: string;
  open: boolean;
}

function formatDeadlineDate(targetDate: string): string {
  return intlFormatDate(new Date(`${targetDate}T12:00:00`), { dateStyle: "medium" });
}

function TodayTaskList({ dailyPlanId, rows, taskById, onDataChanged }: TodayTaskListProps) {
  const { t } = useTranslation();
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const [menu, setMenu] = useState<ReplanMenuState>({ planTaskId: "", open: false });
  const [sessionTask, setSessionTask] = useState<string | null>(null);

  const activeRows = useMemo(
    () =>
      rows
        .filter((row) => row.planned_status === "selected" || row.planned_status === "started")
        .sort((a, b) => a.order_index - b.order_index),
    [rows],
  );

  const run = async (action: () => Promise<unknown>) => {
    setError(null);
    setBusy(true);
    try {
      await action();
      onDataChanged();
    } catch (err) {
      setError(toDisplayMessage(err));
    } finally {
      setBusy(false);
    }
  };

  const move = (index: number, direction: -1 | 1) => {
    const next = [...activeRows];
    const target = index + direction;
    if (target < 0 || target >= next.length) return;
    [next[index], next[target]] = [next[target], next[index]];
    void run(() =>
      reorderDayTasks(
        dailyPlanId,
        next.map((row) => row.id),
      ),
    );
  };

  const handleReplan = (decision: ReplanDecision) => {
    if (decision === "divide") return; // handled via link navigation
    void run(() => replanTask(menu.planTaskId, decision, null));
    setMenu({ planTaskId: "", open: false });
  };

  if (activeRows.length === 0) {
    return <p className="joi-empty__state">{t("today.task.empty")}</p>;
  }

  return (
    <div className="flex flex-col gap-2">
      {error ? (
        <p role="alert" className="joi-text--danger">
          {error}
        </p>
      ) : null}
      {activeRows.map((row, index) => {
        const task = taskById.get(row.task_id);
        if (!task) return null;
        const deadlineText = task.target_date
          ? t("common.deadlineDate", { date: formatDeadlineDate(task.target_date) })
          : null;
        return (
          <div
            key={row.id}
            className="joi-surface--base joi-border--default rounded-lg border px-3 py-2"
            data-notification-target={task.id}
          >
            <div className="flex min-w-0 flex-col gap-2">
              <div className="flex min-w-0 items-start gap-2">
                <div className="flex shrink-0 flex-col">
                  <button
                    type="button"
                    aria-label={t("today.task.moveUp")}
                    disabled={index === 0 || busy}
                    onClick={() => move(index, -1)}
                    className="joi-interactive joi-text--subtle text-xs disabled:opacity-30"
                  >
                    ▲
                  </button>
                  <button
                    type="button"
                    aria-label={t("today.task.moveDown")}
                    disabled={index === activeRows.length - 1 || busy}
                    onClick={() => move(index, 1)}
                    className="joi-interactive joi-text--subtle text-xs disabled:opacity-30"
                  >
                    ▼
                  </button>
                </div>
                <div className="min-w-0 flex-1">
                  <Link to={`/tasks/${task.id}`} className="joi-text block break-words text-sm">
                    {task.title}
                  </Link>
                  <p className="joi-text--muted break-words text-xs">
                    {t(`common.labels.taskStatus.${task.status}`)}
                    {deadlineText ? ` · ${deadlineText}` : ""}
                  </p>
                </div>
              </div>
              <div className="flex min-w-0 flex-wrap justify-start gap-1">
                <button
                  type="button"
                  disabled={busy}
                  onClick={() => run(() => completeTask(task.id))}
                  className="joi-btn joi-btn--primary rounded-md px-2 py-1 text-xs font-medium disabled:opacity-50"
                >
                  {t("common.actions.complete")}
                </button>
                <button
                  type="button"
                  disabled={busy}
                  onClick={() => run(() => transitionTaskStatus(task.id, "deferred"))}
                  className="joi-btn joi-btn--secondary rounded-md border px-2 py-1 text-xs font-medium disabled:opacity-50"
                >
                  {t("common.actions.defer")}
                </button>
                <button
                  type="button"
                  disabled={busy}
                  onClick={() => run(() => deselectTaskForDay(row.id))}
                  className="joi-btn joi-btn--secondary rounded-md border px-2 py-1 text-xs font-medium disabled:opacity-50"
                >
                  {t("common.actions.deselect")}
                </button>
                <div className="relative">
                  <button
                    type="button"
                    disabled={busy}
                    onClick={() =>
                      setMenu((prev) =>
                        prev.planTaskId === row.id && prev.open
                          ? { planTaskId: "", open: false }
                          : { planTaskId: row.id, open: true },
                      )
                    }
                    className="joi-btn joi-btn--secondary rounded-md border px-2 py-1 text-xs font-medium disabled:opacity-50"
                  >
                    {t("common.actions.replan")}
                  </button>
                  {menu.open && menu.planTaskId === row.id ? (
                    <div className="joi-surface--base joi-border--default absolute right-0 z-10 mt-1 w-48 rounded-lg border py-1 shadow-lg">
                      {REPLAN_DECISIONS.map((decision) => (
                        <button
                          key={decision}
                          type="button"
                          onClick={() => {
                            if (decision === "divide") {
                              setMenu({ planTaskId: "", open: false });
                              return;
                            }
                            handleReplan(decision);
                          }}
                          className="joi-interactive joi-text block w-full px-3 py-1.5 text-left text-xs"
                        >
                          {t(`today.replan.${decision}`)}
                        </button>
                      ))}
                      <Link
                        to={`/tasks/${task.id}/children/new`}
                        className="joi-interactive joi-text block w-full px-3 py-1.5 text-left text-xs"
                      >
                        {t("today.task.divideIntoChildren")}
                      </Link>
                    </div>
                  ) : null}
                </div>
                <button
                  type="button"
                  onClick={() => setSessionTask(sessionTask === task.id ? null : task.id)}
                  className="joi-btn joi-btn--primary rounded-md px-2 py-1 text-xs font-medium disabled:opacity-50"
                >
                  {t("today.session.title")}
                </button>
              </div>
            </div>
            {sessionTask === task.id ? (
              <div className="mt-2">
                <SessionTracker taskId={task.id} dailyPlanTaskId={row.id} />
              </div>
            ) : null}
          </div>
        );
      })}
    </div>
  );
}

export default TodayTaskList;
