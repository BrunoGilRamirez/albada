import { useState } from "react";
import { useTranslation } from "react-i18next";
import { updateReminderRule, type UpdateReminderRuleInput } from "../../lib/api/notificationApi";
import { toDisplayMessage } from "../../lib/errors";
import { formatNumber } from "../../lib/format";
import type { ReminderRule } from "../../lib/types";

// Mirrors the native domain limits in src-tauri/src/domain/notifications.rs.
// The Rust layer remains the authority; these bounds only give immediate
// feedback before the command round-trip.
const MAX_REMINDERS_PER_DAY_LIMIT = 24;
const SNOOZE_MIN_MINUTES = 1;
const SNOOZE_MAX_MINUTES = 720;
const MAX_SNOOZE_OPTIONS = 8;
const MAX_SAFE_MESSAGE_CHARS = 500;

interface ReminderRuleFormProps {
  rule: ReminderRule;
  onSaved: (rule: ReminderRule) => void;
}

interface DraftState {
  enabled: boolean;
  preferred_time: string;
  quiet_hours_start: string;
  quiet_hours_end: string;
  deadline_notification_window_days: string;
  max_reminders_per_day: string;
  snooze_minutes: string;
  privacy_safe_message: string;
}

// Snooze parsing produces either the parsed options or the catalog key of the
// message to show, so the validation copy lives in `notifications.rule.*`.
type SnoozeParse = { ok: true; minutes: number[] } | { ok: false; errorKey: SnoozeErrorKey };

type SnoozeErrorKey =
  | "notifications.rule.snoozeWholeError"
  | "notifications.rule.snoozeRangeError"
  | "notifications.rule.snoozeDuplicateError"
  | "notifications.rule.snoozeCountError";

/**
 * The list command serializes times as `HH:MM:SS` (chrono `NaiveTime`) while
 * the update DTO requires the exact `HH:MM` form validated natively. Normalize
 * at this boundary so echoing unchanged values back never trips validation.
 */
function toInputTime(value: string | null): string {
  return value && value.length > 5 ? value.slice(0, 5) : (value ?? "");
}

function toWireTime(value: string | null): string | null {
  const normalized = toInputTime(value);
  return normalized === "" ? null : normalized;
}

function draftFromRule(rule: ReminderRule): DraftState {
  return {
    enabled: rule.enabled,
    preferred_time: toInputTime(rule.preferred_time),
    quiet_hours_start: toInputTime(rule.quiet_hours_start),
    quiet_hours_end: toInputTime(rule.quiet_hours_end),
    deadline_notification_window_days: String(rule.deadline_notification_window_days),
    max_reminders_per_day: String(rule.max_reminders_per_day),
    snooze_minutes: rule.snooze_minutes.join(", "),
    privacy_safe_message: rule.privacy_safe_message,
  };
}

/**
 * Parse the comma-separated snooze list into minutes. Returns the parsed
 * options or the key of the user-facing error message. Empty input is a valid
 * empty list.
 */
function parseSnoozeOptions(raw: string): SnoozeParse {
  const parts = raw
    .split(",")
    .map((part) => part.trim())
    .filter((part) => part.length > 0);
  const minutes: number[] = [];
  for (const part of parts) {
    const value = Number.parseInt(part, 10);
    if (!Number.isInteger(value) || String(value) !== part) {
      return { ok: false, errorKey: "notifications.rule.snoozeWholeError" };
    }
    if (value < SNOOZE_MIN_MINUTES || value > SNOOZE_MAX_MINUTES) {
      return { ok: false, errorKey: "notifications.rule.snoozeRangeError" };
    }
    if (minutes.includes(value)) {
      return { ok: false, errorKey: "notifications.rule.snoozeDuplicateError" };
    }
    minutes.push(value);
  }
  if (minutes.length > MAX_SNOOZE_OPTIONS) {
    return { ok: false, errorKey: "notifications.rule.snoozeCountError" };
  }
  return { ok: true, minutes };
}

function ReminderRuleForm({ rule, onSaved }: ReminderRuleFormProps) {
  const { t } = useTranslation();
  const [draft, setDraft] = useState<DraftState>(() => draftFromRule(rule));
  const [error, setError] = useState<string | null>(null);
  const [notice, setNotice] = useState<string | null>(null);
  const [saving, setSaving] = useState(false);

  // The rule's privacy-safe message is user-configured copy stored natively;
  // it is rendered verbatim. Only the surrounding chrome is translated.
  const typeLabel = t(`notifications.types.${rule.reminder_type}`);

  const set = <K extends keyof DraftState>(key: K, value: DraftState[K]) => {
    setDraft((prev) => ({ ...prev, [key]: value }));
    setNotice(null);
  };

  const handleSubmit = async (event: React.FormEvent) => {
    event.preventDefault();
    setError(null);
    setNotice(null);

    const maxPerDay = Number.parseInt(draft.max_reminders_per_day, 10);
    if (!Number.isInteger(maxPerDay) || maxPerDay < 0 || maxPerDay > MAX_REMINDERS_PER_DAY_LIMIT) {
      setError(
        t("notifications.rule.dailyLimitError", { max: formatNumber(MAX_REMINDERS_PER_DAY_LIMIT) }),
      );
      return;
    }

    const deadlineWindow = Number(draft.deadline_notification_window_days);
    if (!Number.isInteger(deadlineWindow) || deadlineWindow < 1 || deadlineWindow > 7) {
      setError(t("notifications.rule.deadlineWindowError"));
      return;
    }

    if (
      draft.quiet_hours_start &&
      draft.quiet_hours_end &&
      draft.quiet_hours_start === draft.quiet_hours_end
    ) {
      setError(t("notifications.rule.quietHoursError"));
      return;
    }

    const snooze = parseSnoozeOptions(draft.snooze_minutes);
    if (!snooze.ok) {
      setError(
        t(snooze.errorKey, {
          min: formatNumber(SNOOZE_MIN_MINUTES),
          max: formatNumber(SNOOZE_MAX_MINUTES),
          options: formatNumber(MAX_SNOOZE_OPTIONS),
        }),
      );
      return;
    }

    const message = draft.privacy_safe_message.trim();
    if (message.length === 0) {
      setError(t("notifications.rule.safeMessageRequired"));
      return;
    }
    if (message.length > MAX_SAFE_MESSAGE_CHARS) {
      setError(
        t("notifications.rule.safeMessageTooLong", { max: formatNumber(MAX_SAFE_MESSAGE_CHARS) }),
      );
      return;
    }

    // Time windows, channel, and escalation are not edited on this screen in
    // the MVP; they are echoed back unchanged so the full-DTO update never
    // silently clears system-owned configuration.
    const input: UpdateReminderRuleInput = {
      enabled: draft.enabled,
      preferred_time: draft.preferred_time || null,
      time_window_start: toWireTime(rule.time_window_start),
      time_window_end: toWireTime(rule.time_window_end),
      quiet_hours_start: draft.quiet_hours_start || null,
      quiet_hours_end: draft.quiet_hours_end || null,
      deadline_notification_window_days: deadlineWindow,
      max_reminders_per_day: maxPerDay,
      snooze_minutes: snooze.minutes,
      channel: rule.channel,
      escalation: rule.escalation,
      privacy_safe_message: message,
    };

    setSaving(true);
    try {
      const updated = await updateReminderRule(rule.id, input);
      setDraft(draftFromRule(updated));
      onSaved(updated);
      setNotice(t("notifications.rule.saved"));
    } catch (err) {
      setError(toDisplayMessage(err));
    } finally {
      setSaving(false);
    }
  };

  const inputClass = "joi-field__input";
  const labelClass = "joi-field__label mb-1 block";

  return (
    <form
      onSubmit={handleSubmit}
      aria-label={t("notifications.rule.formAria", { type: typeLabel })}
      className="joi-surface--base joi-border--default rounded-lg border p-4"
    >
      <div className="flex flex-wrap items-start justify-between gap-3">
        <div>
          <h3 className="joi-text text-base font-semibold">{typeLabel}</h3>
          <p className="joi-text--muted mt-1 text-xs">
            {t(`notifications.channels.${rule.channel}`)} ·{" "}
            {t(`notifications.escalations.${rule.escalation}`)}
            {" · "}
            {rule.deliveries_today > 0
              ? t("notifications.rule.sentToday", { count: rule.deliveries_today })
              : t("notifications.rule.sentNone")}
          </p>
        </div>
        <label className="joi-text--muted flex items-center gap-2 text-sm">
          <input
            type="checkbox"
            checked={draft.enabled}
            onChange={(e) => set("enabled", e.target.checked)}
            className="joi-checkbox h-4 w-4"
            aria-label={t("notifications.rule.enableAria", { type: typeLabel })}
          />
          {t("notifications.rule.enabled")}
        </label>
      </div>

      <div className="mt-4 grid grid-cols-1 gap-4 sm:grid-cols-3">
        <div>
          <label htmlFor={`preferred-time-${rule.id}`} className={labelClass}>
            {t("notifications.rule.preferredTime")}
          </label>
          <input
            id={`preferred-time-${rule.id}`}
            type="time"
            className={inputClass}
            value={draft.preferred_time}
            onChange={(e) => set("preferred_time", e.target.value)}
          />
          <p className="joi-field__help mt-1">{t("notifications.rule.preferredTimeHelp")}</p>
        </div>
        <div>
          <label htmlFor={`quiet-start-${rule.id}`} className={labelClass}>
            {t("notifications.rule.quietStart")}
          </label>
          <input
            id={`quiet-start-${rule.id}`}
            type="time"
            className={inputClass}
            value={draft.quiet_hours_start}
            onChange={(e) => set("quiet_hours_start", e.target.value)}
          />
        </div>
        <div>
          <label htmlFor={`quiet-end-${rule.id}`} className={labelClass}>
            {t("notifications.rule.quietEnd")}
          </label>
          <input
            id={`quiet-end-${rule.id}`}
            type="time"
            className={inputClass}
            value={draft.quiet_hours_end}
            onChange={(e) => set("quiet_hours_end", e.target.value)}
          />
          <p className="joi-field__help mt-1">{t("notifications.rule.quietEndHelp")}</p>
        </div>
      </div>

      <div className="mt-4 grid grid-cols-1 gap-4 sm:grid-cols-2">
        {rule.reminder_type === "deadline" ? (
          <div>
            <label htmlFor={`deadline-window-${rule.id}`} className={labelClass}>
              {t("notifications.rule.deadlineWindow")}
            </label>
            <input
              id={`deadline-window-${rule.id}`}
              type="number"
              min={1}
              max={7}
              step={1}
              className={inputClass}
              value={draft.deadline_notification_window_days}
              onChange={(e) => set("deadline_notification_window_days", e.target.value)}
            />
            <p className="joi-field__help mt-1">{t("notifications.rule.deadlineWindowHelp")}</p>
          </div>
        ) : null}
        <div>
          <label htmlFor={`max-per-day-${rule.id}`} className={labelClass}>
            {t("notifications.rule.maxPerDay")}
          </label>
          <input
            id={`max-per-day-${rule.id}`}
            type="number"
            min={0}
            max={MAX_REMINDERS_PER_DAY_LIMIT}
            className={inputClass}
            value={draft.max_reminders_per_day}
            onChange={(e) => set("max_reminders_per_day", e.target.value)}
          />
        </div>
        <div>
          <label htmlFor={`snooze-${rule.id}`} className={labelClass}>
            {t("notifications.rule.snooze")}
          </label>
          <input
            id={`snooze-${rule.id}`}
            type="text"
            inputMode="numeric"
            className={inputClass}
            value={draft.snooze_minutes}
            onChange={(e) => set("snooze_minutes", e.target.value)}
            placeholder="10, 30, 60"
          />
          <p className="joi-field__help mt-1">
            {t("notifications.rule.snoozeHelp", {
              min: formatNumber(SNOOZE_MIN_MINUTES),
              max: formatNumber(SNOOZE_MAX_MINUTES),
              options: formatNumber(MAX_SNOOZE_OPTIONS),
            })}
          </p>
        </div>
      </div>

      <div className="mt-4">
        <label htmlFor={`safe-message-${rule.id}`} className={labelClass}>
          {t("notifications.rule.safeMessage")}
        </label>
        <input
          id={`safe-message-${rule.id}`}
          type="text"
          maxLength={MAX_SAFE_MESSAGE_CHARS}
          className={inputClass}
          value={draft.privacy_safe_message}
          onChange={(e) => set("privacy_safe_message", e.target.value)}
        />
        <p className="joi-field__help mt-1">{t("notifications.rule.safeMessageHelp")}</p>
      </div>

      {error ? (
        <p role="alert" className="joi-text--danger mt-3 text-sm">
          {error}
        </p>
      ) : null}
      {notice ? (
        <p role="status" className="joi-text--success mt-3 text-sm">
          {notice}
        </p>
      ) : null}

      <div className="mt-4">
        <button
          type="submit"
          disabled={saving}
          className="joi-btn joi-btn--primary rounded-lg px-4 py-2"
        >
          {saving ? t("common.saving") : t("notifications.rule.saveSubmit")}
        </button>
      </div>
    </form>
  );
}

export default ReminderRuleForm;
