import { useState } from "react";
import { useTranslation } from "react-i18next";
import { reactivateBlock, type ReactivateBlockInput } from "../../lib/api/blockApi";
import { toDisplayMessage } from "../../lib/errors";
import { reactivateBlockSchema } from "../../lib/schemas/schemas";
import type { Block } from "../../lib/types";
import { ISO_WEEKDAY_KEYS, localIsoWeekday, nextIsoWeekday } from "../../lib/weekdays";

// Mirror of the native `parse_time_of_day` grammar (exact 24-hour "HH:MM").
// This is only a pre-submit hint; the native command stays authoritative.
const START_TIME_PATTERN = /^([01]\d|2[0-3]):[0-5]\d$/;

export interface ReactivateScheduleFormProps {
  workspaceId: string;
  /** The inactive source block that is being reactivated. */
  block: Block;
  /** Close the form without reactivating anything. */
  onClose: () => void;
  /** The native reactivation succeeded; the owner refreshes its lists. */
  onReactivated: () => void;
  /** Injected clock for the weekday default (deterministic in tests). */
  now?: Date;
}

/**
 * Default weekday of a reactivation: the active local weekday, or the
 * following day once the local day is over (23:00 or later). Presentation
 * default only; the native command resolves the weekday to the next
 * matching workspace-local date on or after today.
 */
export function defaultReactivateWeekday(now: Date): number {
  return now.getHours() >= 23 ? nextIsoWeekday(now) : localIsoWeekday(now);
}

function secondsToMinutes(seconds: number | null): string {
  // Legacy rows may hold exact seconds that are not whole minutes; the UI
  // shows the nearest minute.
  return seconds === null ? "" : String(Math.round(seconds / 60));
}

/**
 * Scheduling controls for reactivating an inactive one-time block (8.7.9).
 *
 * The user picks an ISO weekday and explicitly chooses a single new
 * occurrence ("once") or a new weekly series ("recurring"). A recurring
 * reactivation needs an exact local start time and a positive duration, so
 * the form rejects those cases with the same translated inline errors the
 * block form uses. The source block itself is never modified: the native
 * command creates a new linked row and keeps the source row, its status, and
 * its timestamps untouched.
 */
function ReactivateScheduleForm({
  workspaceId,
  block,
  onClose,
  onReactivated,
  now,
}: ReactivateScheduleFormProps) {
  const { t } = useTranslation();
  const clock = now ?? new Date();
  const [schedule, setSchedule] = useState<"once" | "recurring">("once");
  const [weekday, setWeekday] = useState(() => defaultReactivateWeekday(clock));
  const [startTime, setStartTime] = useState(
    block.planned_start_at ?? block.local_start_time ?? "",
  );
  const [durationMinutes, setDurationMinutes] = useState(
    secondsToMinutes(block.estimated_duration_seconds),
  );
  const [error, setError] = useState<string | null>(null);
  const [submitting, setSubmitting] = useState(false);

  // The source block carries an exact time or duration whenever it had one;
  // those inputs stay visible so the user can keep or correct them.
  const sourceHasTime =
    (block.planned_start_at ?? block.local_start_time ?? "") !== "" ||
    block.estimated_duration_seconds !== null;
  const showTimeFields = schedule === "recurring" || sourceHasTime;

  const handleSubmit = async (event: React.FormEvent) => {
    event.preventDefault();
    setError(null);

    const trimmedStart = startTime.trim();
    const durationText = durationMinutes.trim();
    const parsedMinutes = durationText === "" ? null : Number(durationText);
    const startValid = trimmedStart === "" || START_TIME_PATTERN.test(trimmedStart);
    const minutesValid =
      durationText === "" || (Number.isInteger(parsedMinutes) && (parsedMinutes ?? 0) > 0);

    // The shared schema carries the once/recurring rules; the two text
    // checks keep an explicitly typed but malformed value from silently
    // becoming "not set".
    const candidate = {
      schedule,
      weekday,
      local_start_time: startValid && trimmedStart !== "" ? trimmedStart : null,
      estimated_duration_seconds:
        minutesValid && parsedMinutes !== null ? parsedMinutes * 60 : null,
    };
    const parsed = reactivateBlockSchema.safeParse(candidate);
    if (!parsed.success) {
      const path = parsed.error.issues[0]?.path[0];
      setError(
        path === "local_start_time"
          ? t("today.block.form.needStartRecurring")
          : t("today.block.form.badDuration"),
      );
      return;
    }
    if (!startValid) {
      setError(t("today.block.form.badStart"));
      return;
    }
    if (!minutesValid) {
      setError(t("today.block.form.badDuration"));
      return;
    }

    setSubmitting(true);
    try {
      const input: ReactivateBlockInput =
        schedule === "once"
          ? {
              workspace_id: workspaceId,
              block_id: block.id,
              schedule: "once",
              weekday,
            }
          : {
              workspace_id: workspaceId,
              block_id: block.id,
              schedule: "recurring",
              weekday,
              local_start_time: trimmedStart,
              estimated_duration_seconds: (parsedMinutes as number) * 60,
            };
      await reactivateBlock(input);
      onReactivated();
      onClose();
    } catch (err) {
      setError(toDisplayMessage(err));
    } finally {
      setSubmitting(false);
    }
  };

  const inputClass = "joi-field__input";
  const labelClass = "joi-field__label mb-1 block";

  return (
    <form
      onSubmit={handleSubmit}
      noValidate
      className="joi-surface--base joi-border--default flex flex-col gap-3 rounded-lg border p-3"
    >
      <h4 className="joi-text text-sm font-medium">{t("today.block.reactivate.title")}</h4>

      <fieldset className="flex flex-col gap-1">
        <legend className={labelClass}>{t("today.block.reactivate.schedule")}</legend>
        <label htmlFor="reactivate-schedule-once" className="flex items-center gap-2 text-sm">
          <input
            id="reactivate-schedule-once"
            type="radio"
            name="reactivate-schedule"
            checked={schedule === "once"}
            onChange={() => setSchedule("once")}
          />
          {t("today.block.reactivate.once")}
        </label>
        <label htmlFor="reactivate-schedule-recurring" className="flex items-center gap-2 text-sm">
          <input
            id="reactivate-schedule-recurring"
            type="radio"
            name="reactivate-schedule"
            checked={schedule === "recurring"}
            onChange={() => setSchedule("recurring")}
          />
          {t("today.block.reactivate.recurring")}
        </label>
      </fieldset>

      <div>
        <span className={labelClass}>{t("today.block.reactivate.weekday")}</span>
        <div
          className="flex flex-wrap gap-1"
          role="group"
          aria-label={t("today.block.reactivate.weekday")}
        >
          {ISO_WEEKDAY_KEYS.map((key, index) => {
            const iso = index + 1;
            const selected = weekday === iso;
            return (
              <button
                key={key}
                type="button"
                aria-pressed={selected}
                onClick={() => setWeekday(iso)}
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

      {showTimeFields ? (
        <div className="grid grid-cols-1 gap-3 sm:grid-cols-2">
          <div>
            <label htmlFor="reactivate-start" className={labelClass}>
              {t("today.block.reactivate.start")}
            </label>
            <input
              id="reactivate-start"
              type="time"
              className={inputClass}
              value={startTime}
              onChange={(e) => setStartTime(e.target.value)}
            />
          </div>
          <div>
            <label htmlFor="reactivate-duration" className={labelClass}>
              {t("today.block.reactivate.duration")}
            </label>
            <input
              id="reactivate-duration"
              type="number"
              min={1}
              step={1}
              className={inputClass}
              value={durationMinutes}
              onChange={(e) => setDurationMinutes(e.target.value)}
            />
          </div>
        </div>
      ) : null}

      {error ? (
        <p role="alert" className="joi-text--danger">
          {error}
        </p>
      ) : null}

      <div className="flex gap-2">
        <button
          type="submit"
          disabled={submitting}
          className="joi-btn joi-btn--primary px-4 py-2 disabled:opacity-50"
        >
          {submitting ? t("common.saving") : t("today.block.reactivate.submit")}
        </button>
        <button
          type="button"
          onClick={onClose}
          disabled={submitting}
          className="joi-btn joi-btn--secondary px-4 py-2 disabled:opacity-50"
        >
          {t("common.actions.cancel")}
        </button>
      </div>
    </form>
  );
}

export default ReactivateScheduleForm;
