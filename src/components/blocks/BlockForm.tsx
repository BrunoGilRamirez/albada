import { useEffect, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import {
  createBlock,
  createBlockRecurrence,
  getBlockRecurrence,
  updateBlock,
  updateBlockRecurrence,
  type BlockRecurrenceInput,
  type CreateBlockInput,
  type UpdateBlockInput,
} from "../../lib/api/blockApi";
import { listTasks } from "../../lib/api/taskApi";
import { eligibleBlockTasks, localTodayDate } from "../../lib/blockTaskEligibility";
import { toDisplayMessage } from "../../lib/errors";
import type {
  Block,
  BlockRecurrence,
  BlockType,
  Flexibility,
  Task,
  TimeWindowLabel,
} from "../../lib/types";
import { BLOCK_TYPES, FLEXIBILITIES, TIME_WINDOW_LABELS } from "../../lib/types";
import { ISO_WEEKDAY_KEYS } from "../../lib/weekdays";

// Mirror of the native `parse_time_of_day` grammar (exact 24-hour "HH:MM").
// This is only a pre-submit hint; the native command stays authoritative.
const START_TIME_PATTERN = /^([01]\d|2[0-3]):[0-5]\d$/;

export interface BlockFormProps {
  workspaceId: string;
  date: string;
  dailyPlanId?: string | null;
  defaultType?: BlockType;
  /** When set, the form edits this persisted block and prefills its values. */
  editing?: Block | null;
  /** Edits a recurring rule without requiring a materialized occurrence. */
  editingRecurrenceId?: string | null;
  /** Render this provisional row in the visible list before the response. */
  onPending: (pending: Block) => void;
  /** One-off create/update resolved; commit the native response. */
  onCommitted: (pending: Block, committed: Block) => void;
  /** A recurrence rule was saved; the list owner re-reads the day. */
  onRecurrenceCommitted: () => void;
  /** The native write was rejected; restore the previous visible rows. */
  onRollback: () => void;
  /** Close the form. Cancelling never persists anything. */
  onClose: () => void;
}

function minutesToSeconds(minutes: number | null): number | null {
  return minutes === null ? null : minutes * 60;
}

function secondsToMinutes(seconds: number | null): string {
  // Legacy rows may hold exact seconds that are not whole minutes; the UI
  // shows the nearest minute and a re-save persists rounded whole minutes.
  return seconds === null ? "" : String(Math.round(seconds / 60));
}

function pendingId(): string {
  return `pending-${Math.random().toString(36).slice(2, 10)}`;
}

function BlockForm({
  workspaceId,
  date,
  dailyPlanId,
  defaultType = "focus",
  editing = null,
  editingRecurrenceId = null,
  onPending,
  onCommitted,
  onRecurrenceCommitted,
  onRollback,
  onClose,
}: BlockFormProps) {
  const { t } = useTranslation();
  const [blockType, setBlockType] = useState<BlockType>(editing?.block_type ?? defaultType);
  const [label, setLabel] = useState(editing?.label ?? "");
  const [flexibility, setFlexibility] = useState<Flexibility>(editing?.flexibility ?? "flexible");
  const [plannedStartAt, setPlannedStartAt] = useState(editing?.planned_start_at ?? "");
  const [durationMinutes, setDurationMinutes] = useState(
    secondsToMinutes(editing?.estimated_duration_seconds ?? null),
  );
  const [timeWindowLabel, setTimeWindowLabel] = useState<TimeWindowLabel | "">(
    editing?.time_window_label ?? "",
  );
  const [note, setNote] = useState(editing?.note_markdown ?? "");

  const [weekly, setWeekly] = useState(false);
  const [weekdays, setWeekdays] = useState<number[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [submitting, setSubmitting] = useState(false);

  // Optional task reference (8.7.9). The task list is loaded lazily the
  // first time the picker is revealed and then kept in state, so collapsing
  // and reopening it never refetches. A block only references a task; it
  // never changes the task's status or progress.
  const [taskEnabled, setTaskEnabled] = useState(editing?.task_id != null);
  const [taskId, setTaskId] = useState<string | null>(editing?.task_id ?? null);
  const [taskOptions, setTaskOptions] = useState<Task[] | null>(null);
  const [taskError, setTaskError] = useState<string | null>(null);
  // The label value this form generated for the selected task. A manual edit
  // clears it, so a later task selection never overwrites the user's text.
  const [autoLabel, setAutoLabel] = useState<string | null>(null);

  // Weekly editing targets the logical rule, not the occurrence row: the
  // native `update_block` rejects recurrence materializations ("Recurring
  // occurrences must be changed through their recurrence"), so the loaded
  // rule supplies the authoritative prefill and the save goes through
  // `updateBlockRecurrence`.
  const recurrenceId = editing?.recurrence_id ?? editingRecurrenceId;
  const [rule, setRule] = useState<BlockRecurrence | null>(null);
  const [ruleError, setRuleError] = useState<string | null>(null);

  useEffect(() => {
    if (!recurrenceId) {
      return;
    }
    let cancelled = false;
    getBlockRecurrence(workspaceId, recurrenceId).then(
      (loaded) => {
        if (cancelled) {
          return;
        }
        setRule(loaded);
        setWeekdays([...loaded.weekdays].sort((a, b) => a - b));
        setBlockType(loaded.block_type);
        setLabel(loaded.label ?? "");
        setFlexibility(loaded.flexibility);
        setPlannedStartAt(loaded.local_start_time);
        setDurationMinutes(secondsToMinutes(loaded.estimated_duration_seconds));
        setNote(loaded.note_markdown ?? "");
        // The rule owns the task reference of a weekly series.
        setTaskEnabled(loaded.task_id != null);
        setTaskId(loaded.task_id);
        setAutoLabel(null);
      },
      (err: unknown) => {
        if (!cancelled) {
          setRuleError(toDisplayMessage(err));
        }
      },
    );
    return () => {
      cancelled = true;
    };
  }, [recurrenceId, workspaceId]);

  // The task list is fetched once, the first time the picker is revealed.
  // The loaded rows stay in state afterwards, so collapsing and reopening
  // the picker never refetches.
  useEffect(() => {
    if (!taskEnabled || taskOptions !== null) {
      return;
    }
    let cancelled = false;
    listTasks(workspaceId).then(
      (loaded) => {
        if (!cancelled) {
          setTaskOptions(loaded);
        }
      },
      (err: unknown) => {
        if (!cancelled) {
          setTaskError(toDisplayMessage(err));
        }
      },
    );
    return () => {
      cancelled = true;
    };
  }, [taskEnabled, taskOptions, workspaceId]);

  const isRecurring = recurrenceId !== null || weekly;
  const isEditing = editing !== null || recurrenceId !== null;
  const ruleLoading = recurrenceId !== null && rule === null && ruleError === null;

  const toggleWeekday = (iso: number) => {
    setWeekdays((prev) =>
      prev.includes(iso) ? prev.filter((day) => day !== iso) : [...prev, iso].sort((a, b) => a - b),
    );
  };

  const eligibleTasks = taskOptions ? eligibleBlockTasks(taskOptions, localTodayDate()) : [];

  // A block keeps its historical task reference, so a task that is no longer
  // eligible (already completed, for example) stays visible as the current
  // selection instead of silently disappearing from the picker.
  const taskOptionsList = useMemo(() => {
    if (!taskOptions) {
      return [];
    }
    if (taskId && !eligibleTasks.some((task) => task.id === taskId)) {
      const current = taskOptions.find((task) => task.id === taskId);
      return current ? [current, ...eligibleTasks] : eligibleTasks;
    }
    return eligibleTasks;
  }, [taskOptions, taskId, eligibleTasks]);

  const toggleTaskEnabled = (enabled: boolean) => {
    setTaskEnabled(enabled);
    if (!enabled) {
      // Unchecking removes the optional task reference.
      setTaskId(null);
    }
  };

  const selectTask = (value: string) => {
    if (value === "") {
      setTaskId(null);
      return;
    }
    setTaskId(value);
    const task = taskOptions?.find((candidate) => candidate.id === value);
    if (!task) {
      return;
    }
    // The label is only prefilled while it is still empty or still the value
    // this picker generated. A manually written label always survives.
    const generated = t("today.block.task.workOnLabel", { title: task.title });
    if (label.trim() === "" || label === autoLabel) {
      setLabel(generated);
      setAutoLabel(generated);
    }
  };

  const buildPending = (seconds: number | null): Block => {
    const now = new Date().toISOString();
    const trimmedLabel = label.trim();
    const trimmedNote = note.trim();
    if (editing) {
      return {
        ...editing,
        block_type: blockType,
        label: trimmedLabel || null,
        planned_start_at: plannedStartAt || null,
        local_start_time:
          recurrenceId && plannedStartAt ? plannedStartAt : editing.local_start_time,
        estimated_duration_seconds: seconds,
        time_window_label: timeWindowLabel || null,
        flexibility,
        note_markdown: trimmedNote || null,
        task_id: taskEnabled ? taskId : null,
        updated_at: now,
      };
    }
    return {
      id: pendingId(),
      workspace_id: workspaceId,
      daily_plan_id: dailyPlanId ?? null,
      date,
      block_type: blockType,
      status: "planned",
      label: trimmedLabel || null,
      planned_start_at: plannedStartAt || null,
      // Provisional display only; the deprecated field is never submitted.
      planned_end_at: null,
      estimated_duration_seconds: seconds,
      recurrence_id: null,
      recurrence_version_id: null,
      local_date: null,
      local_start_time: weekly ? plannedStartAt || null : null,
      timezone_id: null,
      resolved_start_at: null,
      time_window_label: weekly ? null : timeWindowLabel || null,
      actual_start_at: null,
      actual_end_at: null,
      flexibility,
      note_markdown: trimmedNote || null,
      created_at: now,
      updated_at: now,
      lifecycle_state: "active",
      task_id: taskEnabled ? taskId : null,
      reactivated_from_block_id: null,
    };
  };

  const handleSubmit = async (event: React.FormEvent) => {
    event.preventDefault();
    setError(null);
    const trimmedLabel = label.trim();
    if (!trimmedLabel && blockType !== "planning") {
      setError(t("today.block.form.needLabel"));
      return;
    }
    const durationText = durationMinutes.trim();
    const minutes = durationText === "" ? null : Number(durationText);
    if (durationText !== "" && (minutes === null || !Number.isInteger(minutes) || minutes <= 0)) {
      setError(t("today.block.form.badDuration"));
      return;
    }
    if (plannedStartAt && !START_TIME_PATTERN.test(plannedStartAt)) {
      setError(t("today.block.form.badStart"));
      return;
    }
    if (isRecurring) {
      // Settled recurrence requirements (sections 8.3/8.7 of the model doc):
      // an exact local HH:MM start and a strictly positive duration.
      if (!plannedStartAt) {
        setError(t("today.block.form.needStartRecurring"));
        return;
      }
      if (minutes === null) {
        setError(t("today.block.form.badDuration"));
        return;
      }
      if (weekdays.length === 0) {
        setError(t("today.block.form.needDays"));
        return;
      }
    }
    if (recurrenceId !== null && rule === null) {
      setError(ruleError ?? t("today.block.form.recurrenceLoadFailed"));
      return;
    }
    const seconds = minutesToSeconds(minutes);
    setSubmitting(true);
    const pending = buildPending(seconds);
    if (!editingRecurrenceId) onPending(pending);
    try {
      if (recurrenceId !== null && rule) {
        // Preserve the loaded rule's enabled flag and effective window; the
        // form edits the recurring dimensions only.
        const input: BlockRecurrenceInput = {
          workspace_id: workspaceId,
          weekdays: [...weekdays].sort((a, b) => a - b),
          local_start_time: plannedStartAt,
          estimated_duration_seconds: seconds as number,
          block_type: blockType,
          label: trimmedLabel || null,
          flexibility,
          note_markdown: note.trim() || null,
          enabled: rule.enabled,
          effective_from: rule.effective_from,
          effective_until: rule.effective_until,
          task_id: taskEnabled ? taskId : null,
        };
        await updateBlockRecurrence(recurrenceId, input);
        onRecurrenceCommitted();
      } else if (weekly) {
        const input: BlockRecurrenceInput = {
          workspace_id: workspaceId,
          weekdays: [...weekdays].sort((a, b) => a - b),
          local_start_time: plannedStartAt,
          estimated_duration_seconds: seconds as number,
          block_type: blockType,
          label: trimmedLabel || null,
          flexibility,
          note_markdown: note.trim() || null,
          enabled: true,
          // New weekly routines take effect from the day being planned.
          effective_from: date,
          effective_until: null,
          task_id: taskEnabled ? taskId : null,
        };
        await createBlockRecurrence(input);
        onRecurrenceCommitted();
      } else if (editing) {
        const input: UpdateBlockInput = {
          daily_plan_id: editing.daily_plan_id,
          date: editing.date,
          block_type: blockType,
          label: trimmedLabel || null,
          planned_start_at: plannedStartAt || null,
          estimated_duration_seconds: seconds,
          time_window_label: timeWindowLabel || null,
          flexibility,
          note_markdown: note.trim() || null,
          task_id: taskEnabled ? taskId : null,
        };
        const committed = await updateBlock(editing.id, input);
        onCommitted(pending, committed);
      } else {
        const input: CreateBlockInput = {
          workspace_id: workspaceId,
          daily_plan_id: dailyPlanId ?? null,
          date,
          block_type: blockType,
          label: trimmedLabel || null,
          planned_start_at: plannedStartAt || null,
          estimated_duration_seconds: seconds,
          time_window_label: timeWindowLabel || null,
          flexibility,
          note_markdown: note.trim() || null,
          task_id: taskEnabled ? taskId : null,
        };
        const committed = await createBlock(input);
        onCommitted(pending, committed);
      }
    } catch (err) {
      if (!editingRecurrenceId) onRollback();
      setError(toDisplayMessage(err));
    } finally {
      setSubmitting(false);
    }
  };

  // Field classes own the surface, the control boundary, the focus ring, and both
  // themes, so no palette utility or theme restatement is needed here.
  const inputClass = "joi-field__input";
  const selectClass = "joi-field__select";
  const textareaClass = "joi-field__textarea";
  const labelClass = "joi-field__label mb-1 block";

  return (
    // noValidate keeps the submit path in app code: the native constraint
    // bubbles are not localized, so the translated inline validation below
    // is the feedback surface. min/step stay on the duration input as
    // accessibility affordances for the spinbutton.
    <form onSubmit={handleSubmit} noValidate className="flex flex-col gap-3">
      <div className="grid grid-cols-1 gap-3 sm:grid-cols-2">
        <div>
          <label htmlFor="block-type" className={labelClass}>
            {t("today.block.form.type")}
          </label>
          <select
            id="block-type"
            className={selectClass}
            value={blockType}
            onChange={(e) => setBlockType(e.target.value as BlockType)}
          >
            {BLOCK_TYPES.map((type) => (
              <option key={type} value={type}>
                {t(`today.block.types.${type}`)}
              </option>
            ))}
          </select>
        </div>
        <div>
          <label htmlFor="block-flexibility" className={labelClass}>
            {t("today.block.form.flexibility")}
          </label>
          <select
            id="block-flexibility"
            className={selectClass}
            value={flexibility}
            onChange={(e) => setFlexibility(e.target.value as Flexibility)}
          >
            {FLEXIBILITIES.map((value) => (
              <option key={value} value={value}>
                {t(`today.block.flexibilities.${value}`)}
              </option>
            ))}
          </select>
        </div>
      </div>

      <div>
        <label htmlFor="block-label" className={labelClass}>
          {t("today.block.form.label")}
        </label>
        <input
          id="block-label"
          className={inputClass}
          value={label}
          onChange={(e) => {
            setLabel(e.target.value);
            // A manual edit takes the label away from the picker, so a later
            // task selection never overwrites the user's own wording.
            setAutoLabel(null);
          }}
          placeholder={t("today.block.form.labelPlaceholder")}
        />
      </div>

      {/* Optional task reference (8.7.9). The task list loads lazily the
          first time the picker is revealed and is then kept in state. The
          block only points at the task; it never changes the task's status
          or progress. */}
      <div className="flex flex-col gap-2">
        <label htmlFor="block-task-enable" className="flex items-center gap-2 text-sm">
          <input
            id="block-task-enable"
            type="checkbox"
            checked={taskEnabled}
            onChange={(e) => toggleTaskEnabled(e.target.checked)}
          />
          {t("today.block.task.enable")}
        </label>
        {taskEnabled ? (
          <div>
            <label htmlFor="block-task" className={labelClass}>
              {t("today.block.task.pick")}
            </label>
            <select
              id="block-task"
              className={selectClass}
              value={taskId ?? ""}
              onChange={(e) => selectTask(e.target.value)}
            >
              <option value="">{t("today.block.task.none")}</option>
              {taskOptionsList.map((task) => (
                <option key={task.id} value={task.id}>
                  {task.title}
                </option>
              ))}
            </select>
            {taskOptions !== null && eligibleTasks.length === 0 ? (
              <p className="joi-text--muted mt-1 text-xs">{t("today.block.task.empty")}</p>
            ) : null}
            {taskError ? (
              <p role="alert" className="joi-text--danger mt-1 text-xs">
                {taskError}
              </p>
            ) : null}
          </div>
        ) : null}
      </div>

      <div className="grid grid-cols-1 gap-3 sm:grid-cols-2">
        <div>
          <label htmlFor="block-start" className={labelClass}>
            {t("today.block.form.start")}
          </label>
          <input
            id="block-start"
            type="time"
            className={inputClass}
            value={plannedStartAt}
            onChange={(e) => setPlannedStartAt(e.target.value)}
          />
        </div>
        <div>
          <label htmlFor="block-duration" className={labelClass}>
            {t("today.block.form.duration")}
          </label>
          <input
            id="block-duration"
            type="number"
            min={1}
            step={1}
            className={inputClass}
            value={durationMinutes}
            onChange={(e) => setDurationMinutes(e.target.value)}
          />
        </div>
      </div>

      {editing ? null : (
        <label htmlFor="block-weekly" className="flex items-center gap-2 text-sm">
          <input
            id="block-weekly"
            type="checkbox"
            checked={weekly}
            onChange={(e) => setWeekly(e.target.checked)}
          />
          {t("today.block.form.weekly")}
        </label>
      )}

      {isRecurring ? (
        <div>
          <span className={labelClass}>{t("today.block.form.repeatDays")}</span>
          <div
            className="flex flex-wrap gap-1"
            role="group"
            aria-label={t("today.block.form.repeatDays")}
          >
            {ISO_WEEKDAY_KEYS.map((key, index) => {
              const iso = index + 1;
              const selected = weekdays.includes(iso);
              return (
                <button
                  key={key}
                  type="button"
                  aria-pressed={selected}
                  onClick={() => toggleWeekday(iso)}
                  className={`joi-btn rounded-md border px-2 py-1 text-xs font-medium ${
                    selected ? "joi-btn--primary" : "joi-btn--secondary"
                  }`}
                >
                  {t(`common.labels.weekday.${key}`)}
                </button>
              );
            })}
          </div>
        </div>
      ) : null}

      {rule ? (
        <p className="joi-text--muted text-xs">{t("today.block.form.recurrenceNotice")}</p>
      ) : null}

      {/* B5 (unresolved blocker): no time-of-day band is derived from the
          start time, and the manual selection stays available. Explicit
          persisted `time_window_label` values are preserved unchanged; the
          recurrence DTO has no band field, so weekly rules do not store one. */}
      <div>
        <label htmlFor="block-window" className={labelClass}>
          {t("today.block.form.window")}
        </label>
        <select
          id="block-window"
          className={selectClass}
          value={timeWindowLabel}
          disabled={isRecurring}
          onChange={(e) => setTimeWindowLabel(e.target.value as TimeWindowLabel | "")}
        >
          <option value="">{t("today.block.form.windowNone")}</option>
          {TIME_WINDOW_LABELS.map((window) => (
            <option key={window} value={window}>
              {t(`today.block.windows.${window}`)}
            </option>
          ))}
        </select>
        {isRecurring ? (
          <p className="joi-text--muted mt-1 text-xs">{t("today.block.form.windowNotStored")}</p>
        ) : null}
      </div>

      <div>
        <label htmlFor="block-note" className={labelClass}>
          {t("today.block.form.note")}
        </label>
        <textarea
          id="block-note"
          className={textareaClass}
          rows={2}
          value={note}
          onChange={(e) => setNote(e.target.value)}
        />
      </div>

      {ruleError ? (
        <p role="alert" className="joi-text--danger">
          {ruleError}
        </p>
      ) : null}
      {error ? (
        <p role="alert" className="joi-text--danger">
          {error}
        </p>
      ) : null}

      <div className="flex gap-2">
        <button
          type="submit"
          disabled={submitting || ruleLoading}
          className="joi-btn joi-btn--primary px-4 py-2"
        >
          {submitting
            ? isEditing
              ? t("common.saving")
              : t("common.creating")
            : isEditing
              ? t("today.block.form.submitEdit")
              : t("today.block.form.submit")}
        </button>
        <button
          type="button"
          onClick={onClose}
          disabled={submitting}
          className="joi-btn joi-btn--secondary px-4 py-2"
        >
          {t("common.actions.cancel")}
        </button>
      </div>
    </form>
  );
}

export default BlockForm;
