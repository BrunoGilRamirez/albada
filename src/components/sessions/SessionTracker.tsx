import { useCallback, useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import {
  createSession,
  finishSession,
  listSessionsForTask,
  pauseSession,
  skipSession,
  startSession,
} from "../../lib/api/sessionApi";
import { toDisplayMessage } from "../../lib/errors";
import type { TaskSession } from "../../lib/types";

interface SessionTrackerProps {
  taskId: string;
  dailyPlanTaskId?: string | null;
  blockId?: string | null;
}

function formatElapsed(ms: number): string {
  const totalSeconds = Math.max(0, Math.floor(ms / 1000));
  const hours = Math.floor(totalSeconds / 3600);
  const minutes = Math.floor((totalSeconds % 3600) / 60);
  const seconds = totalSeconds % 60;
  const pad = (value: number) => String(value).padStart(2, "0");
  return hours > 0 ? `${hours}:${pad(minutes)}:${pad(seconds)}` : `${pad(minutes)}:${pad(seconds)}`;
}

function SessionTracker({ taskId, dailyPlanTaskId, blockId }: SessionTrackerProps) {
  const { t } = useTranslation();
  const [active, setActive] = useState<TaskSession | null>(null);
  const [elapsedMs, setElapsedMs] = useState(0);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const [finishing, setFinishing] = useState(false);
  const [outcome, setOutcome] = useState("");
  const [nextStep, setNextStep] = useState("");

  const load = useCallback(async () => {
    try {
      const all = await listSessionsForTask(taskId);
      const current = all.find(
        (session) => session.status === "started" || session.status === "paused",
      );
      if (current) {
        setActive(current);
      } else {
        setActive(null);
        setElapsedMs(0);
      }
    } catch (err) {
      setError(toDisplayMessage(err));
    }
  }, [taskId]);

  useEffect(() => {
    void load();
  }, [load]);

  useEffect(() => {
    if (!active || active.status !== "started" || !active.started_at) return;
    const startedAt = new Date(active.started_at).getTime();
    const update = () => setElapsedMs(Date.now() - startedAt);
    update();
    const timer = window.setInterval(update, 1000);
    return () => window.clearInterval(timer);
  }, [active]);

  const run = async (action: () => Promise<TaskSession>) => {
    setError(null);
    setBusy(true);
    try {
      const updated = await action();
      if (updated.status === "started") {
        setActive(updated);
      } else if (updated.status === "paused") {
        setActive(updated);
      } else if (["finished", "skipped"].includes(updated.status)) {
        setActive(null);
        setElapsedMs(0);
      }
      await load();
    } catch (err) {
      setError(toDisplayMessage(err));
    } finally {
      setBusy(false);
    }
  };

  const handleCreate = () =>
    run(() =>
      createSession({
        task_id: taskId,
        block_id: blockId ?? null,
        daily_plan_task_id: dailyPlanTaskId ?? null,
      }),
    );

  const handleFinish = async () => {
    if (!active) return;
    setError(null);
    setBusy(true);
    try {
      const updated = await finishSession(active.id, {
        outcome_markdown: outcome.trim() || null,
        next_step_markdown: nextStep.trim() || null,
      });
      setActive(null);
      setElapsedMs(0);
      setOutcome("");
      setNextStep("");
      setFinishing(false);
      await load();
      if (updated) {
        /* no-op: load refreshed the list */
      }
    } catch (err) {
      setError(toDisplayMessage(err));
    } finally {
      setBusy(false);
    }
  };

  return (
    <div className="joi-surface--base joi-border--default rounded-lg border px-3 py-2">
      <div className="flex flex-wrap items-center gap-2">
        <span className="joi-text text-sm font-medium">{t("today.session.title")}</span>
        {active ? (
          <>
            <span
              className={`joi-badge ${
                active.status === "started" ? "joi-badge--info" : "joi-badge--neutral"
              }`}
            >
              {t(`today.session.statuses.${active.status}`)}
            </span>
            {active.status === "started" ? (
              <span className="joi-text font-mono text-sm tabular-nums">
                {formatElapsed(elapsedMs)}
              </span>
            ) : null}
          </>
        ) : null}
      </div>

      {!active ? (
        <div className="mt-2 flex flex-wrap gap-1">
          <button
            type="button"
            disabled={busy}
            onClick={() => handleCreate()}
            className="joi-btn joi-btn--primary rounded-md px-2 py-1 text-xs font-medium disabled:opacity-50"
          >
            {t("today.session.start")}
          </button>
        </div>
      ) : null}

      {active?.status === "started" ? (
        <div className="mt-2 flex flex-wrap gap-1">
          <button
            type="button"
            disabled={busy}
            onClick={() => run(() => pauseSession(active.id))}
            className="joi-btn joi-btn--secondary rounded-md border px-2 py-1 text-xs font-medium disabled:opacity-50"
          >
            {t("common.actions.pause")}
          </button>
          <button
            type="button"
            disabled={busy}
            onClick={() => setFinishing(true)}
            className="joi-btn joi-btn--primary rounded-md px-2 py-1 text-xs font-medium disabled:opacity-50"
          >
            {t("common.actions.finish")}
          </button>
          <button
            type="button"
            disabled={busy}
            onClick={() => run(() => skipSession(active.id))}
            className="joi-btn joi-btn--secondary rounded-md border px-2 py-1 text-xs font-medium disabled:opacity-50"
          >
            {t("common.actions.skip")}
          </button>
        </div>
      ) : null}

      {active?.status === "paused" ? (
        <div className="mt-2 flex flex-wrap gap-1">
          <button
            type="button"
            disabled={busy}
            onClick={() => run(() => startSession(active.id))}
            className="joi-btn joi-btn--primary rounded-md px-2 py-1 text-xs font-medium disabled:opacity-50"
          >
            {t("common.actions.resume")}
          </button>
          <button
            type="button"
            disabled={busy}
            onClick={() => setFinishing(true)}
            className="joi-btn joi-btn--primary rounded-md px-2 py-1 text-xs font-medium disabled:opacity-50"
          >
            {t("common.actions.finish")}
          </button>
          <button
            type="button"
            disabled={busy}
            onClick={() => run(() => skipSession(active.id))}
            className="joi-btn joi-btn--secondary rounded-md border px-2 py-1 text-xs font-medium disabled:opacity-50"
          >
            {t("common.actions.skip")}
          </button>
        </div>
      ) : null}

      {finishing ? (
        <div className="mt-2 flex flex-col gap-2">
          <textarea
            aria-label={t("today.session.outcomeAria")}
            className="joi-field__textarea rounded-md px-2 py-1 text-sm"
            rows={2}
            placeholder={t("today.session.outcomePlaceholder")}
            value={outcome}
            onChange={(e) => setOutcome(e.target.value)}
          />
          <textarea
            aria-label={t("today.session.nextStepAria")}
            className="joi-field__textarea rounded-md px-2 py-1 text-sm"
            rows={1}
            placeholder={t("today.session.nextStepPlaceholder")}
            value={nextStep}
            onChange={(e) => setNextStep(e.target.value)}
          />
          <div className="flex gap-1">
            <button
              type="button"
              disabled={busy}
              onClick={handleFinish}
              className="joi-btn joi-btn--primary rounded-md px-2 py-1 text-xs font-medium disabled:opacity-50"
            >
              {t("today.session.saveFinish")}
            </button>
            <button
              type="button"
              disabled={busy}
              onClick={() => setFinishing(false)}
              className="joi-btn joi-btn--secondary rounded-md border px-2 py-1 text-xs font-medium disabled:opacity-50"
            >
              {t("common.actions.cancel")}
            </button>
          </div>
        </div>
      ) : null}

      {error ? (
        <p role="alert" className="joi-text--danger mt-2 text-xs">
          {error}
        </p>
      ) : null}
    </div>
  );
}

export default SessionTracker;
