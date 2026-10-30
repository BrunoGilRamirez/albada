import { useEffect, useMemo, useState } from "react";
import { Link } from "react-router";
import { useTranslation } from "react-i18next";
import {
  getCapacitySuggestion,
  selectTaskForDay,
  updateDailyPlanIntention,
  updateDailyPlanStatus,
} from "../../lib/api/dailyPlanApi";
import { setDailyPlanPlanningBlock } from "../../lib/api/dailyPlanApi";
import { createBlock } from "../../lib/api/blockApi";
import { listTasks, whyIsBlocked } from "../../lib/api/taskApi";
import { toDisplayMessage } from "../../lib/errors";
import { formatDate as intlFormatDate, formatNumber } from "../../lib/format";
import type {
  CapacitySuggestion,
  DailyPlan,
  DailyPlanTask,
  SelectionReason,
  Task,
} from "../../lib/types";
import { SELECTION_REASONS } from "../../lib/types";
import type { Translation } from "../../lib/i18n/resources";

const PRIORITY_ORDER: Record<string, number> = {
  none: 0,
  low: 1,
  normal: 2,
  high: 3,
  critical: 4,
};

interface PlanningRitualModalProps {
  workspaceId: string;
  date: string;
  plan: DailyPlan;
  existingPlanTasks: DailyPlanTask[];
  onCompleted: () => void;
  onClose: () => void;
}

const STEP_KEYS: (keyof Translation["today"]["ritual"]["steps"])[] = [
  "unblocked",
  "blocked",
  "confirm",
  "planning",
];

function formatDeadlineDate(targetDate: string): string {
  return intlFormatDate(new Date(`${targetDate}T12:00:00`), { dateStyle: "medium" });
}

function PlanningRitualModal({
  workspaceId,
  date,
  plan,
  existingPlanTasks,
  onCompleted,
  onClose,
}: PlanningRitualModalProps) {
  const { t, i18n } = useTranslation();
  const [step, setStep] = useState(0);
  const [tasks, setTasks] = useState<Task[]>([]);
  const [blockedIds, setBlockedIds] = useState<Set<string>>(new Set());
  const [selectedReasons, setSelectedReasons] = useState<Map<string, SelectionReason>>(new Map());
  const [capacity, setCapacity] = useState<CapacitySuggestion | null>(null);
  const [intention, setIntention] = useState("");
  const [capacityNote, setCapacityNote] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  const formattedDate = intlFormatDate(new Date(`${date}T12:00:00`), {
    weekday: "short",
    year: "numeric",
    month: "short",
    day: "numeric",
  });

  useEffect(() => {
    const load = async () => {
      setError(null);
      try {
        const [allTasks, suggestion] = await Promise.all([
          listTasks(workspaceId),
          getCapacitySuggestion(workspaceId, date).catch(() => null),
        ]);
        setTasks(allTasks);
        setCapacity(suggestion);
        if (suggestion) {
          // The capacity-note prefill is generated content translated at
          // creation in the active UI locale (via the stable `i18n`
          // singleton, so a later locale change never re-runs this load or
          // overwrites user edits). Interpolated clock times and the stored
          // `energy_expectation` value stay verbatim.
          setCapacityNote(
            suggestion.energy_expectation
              ? i18n.t("today.ritual.capacityNotePrefillEnergy", {
                  start: suggestion.start_time,
                  end: suggestion.end_time,
                  energy: suggestion.energy_expectation,
                })
              : i18n.t("today.ritual.capacityNotePrefill", {
                  start: suggestion.start_time,
                  end: suggestion.end_time,
                }),
          );
        }
        const existing = new Map<string, SelectionReason>();
        for (const row of existingPlanTasks) {
          existing.set(row.task_id, row.selection_reason);
        }
        setSelectedReasons(existing);

        const candidates = allTasks.filter((task) =>
          ["inbox", "planned", "in_progress", "blocked", "deferred"].includes(task.status),
        );
        const blocked = new Set<string>();
        await Promise.all(
          candidates.map(async (task) => {
            const blockers = await whyIsBlocked(task.id).catch(() => []);
            if (blockers.length > 0 || task.status === "blocked") {
              blocked.add(task.id);
            }
          }),
        );
        setBlockedIds(blocked);
      } catch (err) {
        setError(toDisplayMessage(err));
      }
    };
    void load();
  }, [workspaceId, date, existingPlanTasks, i18n]);

  const recommended = useMemo(() => {
    return tasks
      .filter((task) => !blockedIds.has(task.id))
      .filter((task) => !["completed", "cancelled"].includes(task.status))
      .sort((a, b) => {
        const priorityDiff = PRIORITY_ORDER[b.priority] - PRIORITY_ORDER[a.priority];
        if (priorityDiff !== 0) return priorityDiff;
        if (a.target_date && b.target_date) return a.target_date.localeCompare(b.target_date);
        if (a.target_date) return -1;
        if (b.target_date) return 1;
        return a.title.localeCompare(b.title);
      });
  }, [tasks, blockedIds]);

  const blockedTasks = useMemo(
    () =>
      tasks
        .filter((task) => blockedIds.has(task.id))
        .filter((task) => !["completed", "cancelled"].includes(task.status)),
    [tasks, blockedIds],
  );

  const selectedCount = selectedReasons.size;

  const renderDeadline = (task: Task): string => {
    if (!task.target_date) return "";
    return ` · ${t("common.deadlineDate", { date: formatDeadlineDate(task.target_date) })}`;
  };

  const toggle = (taskId: string) => {
    setSelectedReasons((prev) => {
      const next = new Map(prev);
      if (next.has(taskId)) {
        next.delete(taskId);
      } else {
        next.set(taskId, "manual");
      }
      return next;
    });
  };

  const setReason = (taskId: string, reason: SelectionReason) => {
    setSelectedReasons((prev) => {
      const next = new Map(prev);
      next.set(taskId, reason);
      return next;
    });
  };

  const handleFinish = async () => {
    setError(null);
    setBusy(true);
    try {
      await updateDailyPlanIntention(plan.id, {
        intention_markdown: intention.trim() || null,
        capacity_note_markdown: capacityNote.trim() || null,
      });

      for (const [taskId, reason] of selectedReasons) {
        const alreadySelected = existingPlanTasks.some(
          (row) => row.task_id === taskId && row.planned_status !== "skipped",
        );
        if (!alreadySelected) {
          await selectTaskForDay({
            daily_plan_id: plan.id,
            task_id: taskId,
            reason,
          });
        }
      }

      // The planning block's label is translated at creation in the active
      // UI locale; once stored it behaves like user-authored content and is
      // never re-translated (same rule as the capacity-note prefill).
      const planningBlock = await createBlock({
        workspace_id: workspaceId,
        daily_plan_id: plan.id,
        date,
        block_type: "planning",
        label: t("today.ritual.planningBlockLabel"),
        planned_start_at: null,
        planned_end_at: null,
        time_window_label: null,
        flexibility: "flexible",
        note_markdown: null,
      });
      await setDailyPlanPlanningBlock(plan.id, planningBlock.id);
      await updateDailyPlanStatus(plan.id, "planned");

      onCompleted();
      onClose();
    } catch (err) {
      setError(toDisplayMessage(err));
    } finally {
      setBusy(false);
    }
  };

  const inputClass = "joi-field__textarea";
  const labelClass = "joi-field__label mb-1 block";

  return (
    <div
      role="dialog"
      aria-modal="true"
      // `.joi-dialog__backdrop` owns the scrim and centering; `items-start` and the
      // scrolling behavior stay as preserved layout utilities.
      className="joi-dialog__backdrop z-50 items-start overflow-y-auto"
    >
      <div className="joi-dialog__panel mt-8 max-w-2xl p-6">
        <div className="flex items-center justify-between">
          <h2 className="joi-dialog__title">{t("today.ritual.title", { date: formattedDate })}</h2>
          <button type="button" onClick={onClose} className="joi-btn joi-btn--ghost joi-btn--sm">
            {t("common.actions.close")}
          </button>
        </div>

        <ol className="mt-4 flex flex-wrap gap-2">
          {STEP_KEYS.map((key, index) => (
            <li
              key={key}
              aria-current={index === step ? "step" : undefined}
              // The current step uses the ratified selected-state brand badge and
              // `aria-current`; completed and upcoming steps remain status-neutral.
              className={`joi-badge rounded-full px-3 py-1 text-xs font-medium ${
                index === step
                  ? "joi-badge--brand"
                  : index < step
                    ? "joi-badge--success"
                    : "joi-badge--neutral"
              }`}
            >
              {index + 1}. {t(`today.ritual.steps.${key}`)}
            </li>
          ))}
        </ol>

        {error ? (
          <p role="alert" className="joi-text--danger mt-4">
            {error}
          </p>
        ) : null}

        <div className="mt-4">
          {step === 0 ? (
            <div>
              <p className="joi-text text-sm">{t("today.ritual.unblockedIntro")}</p>
              {recommended.length === 0 ? (
                <p className="joi-empty__state mt-3">{t("today.ritual.unblockedEmpty")}</p>
              ) : (
                <ul className="mt-3 flex max-h-80 flex-col gap-2 overflow-y-auto">
                  {recommended.map((task) => (
                    <li
                      key={task.id}
                      className="joi-surface--base joi-border--default rounded-lg border px-3 py-2"
                    >
                      <div className="flex items-center gap-2">
                        <input
                          id={`recommend-${task.id}`}
                          type="checkbox"
                          checked={selectedReasons.has(task.id)}
                          onChange={() => toggle(task.id)}
                          className="joi-checkbox h-4 w-4"
                        />
                        <label
                          htmlFor={`recommend-${task.id}`}
                          className="min-w-0 flex-1 cursor-pointer"
                        >
                          <span className="joi-text block truncate text-sm">{task.title}</span>
                          <span className="joi-text--muted text-xs">
                            {t(`common.labels.priority.${task.priority}`)}
                            {renderDeadline(task)}
                          </span>
                        </label>
                        {selectedReasons.has(task.id) ? (
                          <select
                            aria-label={t("today.ritual.reasonFor", { title: task.title })}
                            className="joi-field__select w-auto px-2 py-1 text-xs"
                            value={selectedReasons.get(task.id)}
                            onChange={(e) => setReason(task.id, e.target.value as SelectionReason)}
                          >
                            {SELECTION_REASONS.map((reason) => (
                              <option key={reason} value={reason}>
                                {t(`today.ritual.reasons.${reason}`)}
                              </option>
                            ))}
                          </select>
                        ) : null}
                      </div>
                    </li>
                  ))}
                </ul>
              )}
            </div>
          ) : null}

          {step === 1 ? (
            <div>
              <p className="joi-text text-sm">{t("today.ritual.blockedIntro")}</p>
              {blockedTasks.length === 0 ? (
                <p className="joi-empty__state mt-3">{t("today.ritual.blockedEmpty")}</p>
              ) : (
                <ul className="mt-3 flex max-h-80 flex-col gap-2 overflow-y-auto">
                  {blockedTasks.map((task) => (
                    <li key={task.id} className="joi-alert--warning rounded-lg border px-3 py-2">
                      <Link
                        to={`/tasks/${task.id}`}
                        className="joi-text block truncate text-sm font-medium hover:underline"
                      >
                        {task.title}
                      </Link>
                      <p className="joi-text--warning mt-0.5 text-xs">
                        {task.blocking_note_markdown
                          ? t("today.ritual.blockingNote", { note: task.blocking_note_markdown })
                          : t("today.ritual.blockedByDependency")}
                      </p>
                    </li>
                  ))}
                </ul>
              )}
            </div>
          ) : null}

          {step === 2 ? (
            <div>
              <p className="joi-text text-sm">{t("today.ritual.confirmIntro")}</p>
              {capacity ? (
                <p className="joi-alert--info mt-3 rounded-lg px-3 py-2 text-sm">
                  {capacity.preferred_block_length_minutes
                    ? t("today.ritual.capacityHintPreferred", {
                        start: capacity.start_time,
                        end: capacity.end_time,
                        count: capacity.preferred_block_length_minutes,
                        countDisplay: formatNumber(capacity.preferred_block_length_minutes),
                      })
                    : t("today.ritual.capacityHint", {
                        start: capacity.start_time,
                        end: capacity.end_time,
                      })}
                </p>
              ) : null}
              {selectedCount === 0 ? (
                <p className="joi-empty__state mt-3">{t("today.ritual.selectionEmpty")}</p>
              ) : (
                <ul className="mt-3 flex max-h-80 flex-col gap-2 overflow-y-auto">
                  {[...selectedReasons.entries()].map(([taskId, reason]) => {
                    const task = tasks.find((item) => item.id === taskId);
                    if (!task) return null;
                    return (
                      <li
                        key={taskId}
                        className="joi-surface--base joi-border--default flex items-center justify-between gap-2 rounded-lg border px-3 py-2"
                      >
                        <span className="joi-text min-w-0 flex-1 truncate text-sm">
                          {task.title}
                        </span>
                        <span className="joi-badge joi-badge--neutral shrink-0">
                          {t(`today.ritual.reasons.${reason}`)}
                        </span>
                      </li>
                    );
                  })}
                </ul>
              )}
            </div>
          ) : null}

          {step === 3 ? (
            <div className="flex flex-col gap-4">
              <div>
                <label htmlFor="ritual-intention" className={labelClass}>
                  {t("today.ritual.intention")}
                </label>
                <textarea
                  id="ritual-intention"
                  className={inputClass}
                  rows={3}
                  value={intention}
                  onChange={(e) => setIntention(e.target.value)}
                  placeholder={t("today.ritual.intentionPlaceholder")}
                />
              </div>
              <div>
                <label htmlFor="ritual-capacity" className={labelClass}>
                  {t("today.ritual.capacityNote")}
                </label>
                <textarea
                  id="ritual-capacity"
                  className={inputClass}
                  rows={2}
                  value={capacityNote}
                  onChange={(e) => setCapacityNote(e.target.value)}
                />
              </div>
              <p className="joi-text text-sm">
                {t("today.ritual.finishHelp", { date: formattedDate })}
              </p>
            </div>
          ) : null}
        </div>

        <div className="mt-6 flex items-center justify-between">
          <button
            type="button"
            disabled={step === 0 || busy}
            onClick={() => setStep((value) => value - 1)}
            className="joi-btn joi-btn--secondary px-4 py-2 disabled:opacity-40"
          >
            {t("common.actions.back")}
          </button>
          {step < STEP_KEYS.length - 1 ? (
            <button
              type="button"
              onClick={() => setStep((value) => value + 1)}
              className="joi-btn joi-btn--primary px-4 py-2"
            >
              {t("common.actions.continue")}
            </button>
          ) : (
            // Primary actions stay neutral slate in both themes, so the previous
            // emerald fill is expressed by the neutral slate primary variant.
            <button
              type="button"
              disabled={busy}
              onClick={handleFinish}
              className="joi-btn joi-btn--primary px-4 py-2 disabled:opacity-50"
            >
              {busy ? t("common.saving") : t("today.ritual.finish")}
            </button>
          )}
        </div>
      </div>
    </div>
  );
}

export default PlanningRitualModal;
