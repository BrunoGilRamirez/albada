import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import {
  updateBackupReminderPreference,
  type UpdateBackupReminderPreferenceInput,
} from "../../lib/api/workspaceApi";
import { toDisplayMessage } from "../../lib/errors";
import { formatDateTime, formatNumber } from "../../lib/format";
import type { BackupReminderPreference } from "../../lib/types";

// Mirrors the native domain limits in src-tauri/src/domain/settings.rs.
// The Rust layer remains the authority; these bounds only give immediate
// feedback before the command round-trip.
const MIN_INTERVAL_DAYS = 1;
const MAX_INTERVAL_DAYS = 30;
const RECOMMENDED_INTERVAL_DAYS = 7;

// Fixed lock-screen-safe delivery text from the settled backup-reminder
// contract. It is owned by the native dispatcher (not by the catalogs), is not
// user-editable, and never carries paths or personal content, so it is shown
// verbatim and only the surrounding chrome is translated.
const BACKUP_REMINDER_SAFE_MESSAGE = "A validated backup is due.";

interface BackupReminderFormProps {
  workspaceId: string;
  preference: BackupReminderPreference;
  onSaved: (preference: BackupReminderPreference) => void;
}

interface DraftState {
  enabled: boolean;
  interval_days: string;
}

function draftFromPreference(preference: BackupReminderPreference): DraftState {
  return {
    enabled: preference.enabled,
    interval_days: String(preference.interval_days),
  };
}

function formatBackupTimestamp(value: string): string {
  const date = new Date(value);
  return Number.isNaN(date.getTime()) ? value : formatDateTime(date);
}

/**
 * Workspace preference for the periodic manual-backup reminder. This is a
 * separate preference, not a reminder category, and saving it never creates a
 * backup. Delivery runs through the existing persisted native due-notification
 * flow: the dispatcher evaluates the enabled preference at the fixed 09:00 UTC
 * slot, at most once per cycle. All visible copy comes from
 * `settings.backupReminder.*`.
 */
function BackupReminderForm({ workspaceId, preference, onSaved }: BackupReminderFormProps) {
  const { t } = useTranslation();
  const [draft, setDraft] = useState<DraftState>(() => draftFromPreference(preference));
  const [error, setError] = useState<string | null>(null);
  const [notice, setNotice] = useState<string | null>(null);
  const [saving, setSaving] = useState(false);

  // The preference is system-owned outside the two editable fields, and the
  // page reloads it after backup/restore actions. Re-sync the draft whenever
  // the stored preference changes so the form never saves onto stale state.
  useEffect(() => {
    setDraft(draftFromPreference(preference));
  }, [preference]);

  const set = <K extends keyof DraftState>(key: K, value: DraftState[K]) => {
    setDraft((prev) => ({ ...prev, [key]: value }));
    setNotice(null);
  };

  const handleSubmit = async (event: React.FormEvent) => {
    event.preventDefault();
    setError(null);
    setNotice(null);

    // `Number` (not `parseInt`) so trailing junk like "12px" is rejected, and
    // `Number.isInteger` so fractional drafts never round into a saved value.
    const intervalDays = Number(draft.interval_days.trim());
    if (!Number.isInteger(intervalDays)) {
      setError(t("settings.backupReminder.intervalWholeError"));
      return;
    }
    if (intervalDays < MIN_INTERVAL_DAYS || intervalDays > MAX_INTERVAL_DAYS) {
      setError(
        t("settings.backupReminder.intervalRangeError", {
          min: formatNumber(MIN_INTERVAL_DAYS),
          max: formatNumber(MAX_INTERVAL_DAYS),
        }),
      );
      return;
    }

    const input: UpdateBackupReminderPreferenceInput = {
      enabled: draft.enabled,
      interval_days: intervalDays,
    };

    setSaving(true);
    try {
      const updated = await updateBackupReminderPreference(workspaceId, input);
      setDraft(draftFromPreference(updated));
      onSaved(updated);
      setNotice(t("settings.backupReminder.saved"));
    } catch (err) {
      setError(toDisplayMessage(err));
    } finally {
      setSaving(false);
    }
  };

  return (
    <form
      onSubmit={handleSubmit}
      aria-label={t("settings.backupReminder.aria")}
      // Native constraint validation (min/max/step) would silently block the
      // submit event before our handler runs, leaving no accessible error.
      // `noValidate` keeps the custom `role="alert"` path as the only error
      // surface; the input attributes remain as native hints.
      noValidate
      className="joi-surface--base joi-border--default rounded-lg border p-4"
    >
      <div className="flex flex-wrap items-start justify-between gap-3">
        <div>
          <h2 className="joi-text text-base font-semibold">{t("settings.backupReminder.title")}</h2>
          <p className="joi-text--muted mt-1 text-xs">{t("settings.backupReminder.help")}</p>
        </div>
        <label className="joi-text--muted flex items-center gap-2 text-sm">
          <input
            type="checkbox"
            checked={draft.enabled}
            onChange={(e) => set("enabled", e.target.checked)}
            className="joi-checkbox h-4 w-4"
            aria-label={t("settings.backupReminder.enableAria")}
          />
          {t("settings.backupReminder.enabled")}
        </label>
      </div>

      <div className="mt-4">
        <label htmlFor="backup-reminder-interval" className="joi-field__label mb-1 block">
          {t("settings.backupReminder.interval")}
        </label>
        <input
          id="backup-reminder-interval"
          type="number"
          min={MIN_INTERVAL_DAYS}
          max={MAX_INTERVAL_DAYS}
          step={1}
          className="joi-field__input w-32"
          value={draft.interval_days}
          onChange={(e) => set("interval_days", e.target.value)}
        />
        <p className="joi-field__help mt-1">
          {t("settings.backupReminder.intervalHelp", {
            min: formatNumber(MIN_INTERVAL_DAYS),
            max: formatNumber(MAX_INTERVAL_DAYS),
            recommended: formatNumber(RECOMMENDED_INTERVAL_DAYS),
          })}
        </p>
      </div>

      <p className="joi-text--muted mt-4 text-sm" data-testid="last-validated-backup">
        {preference.last_successful_backup_at
          ? t("settings.backupReminder.lastValidated", {
              timestamp: formatBackupTimestamp(preference.last_successful_backup_at),
            })
          : t("settings.backupReminder.lastValidatedNone")}
      </p>
      <p className="joi-text--muted mt-1 text-xs">{t("settings.backupReminder.timestampNote")}</p>

      <p className="joi-text--muted mt-4 text-xs">
        {t("settings.backupReminder.safeMessageNote", { message: BACKUP_REMINDER_SAFE_MESSAGE })}
      </p>

      <p className="joi-text--muted mt-2 text-xs" data-testid="backup-reminder-schedule-note">
        {t("settings.backupReminder.scheduleNote")}
      </p>

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
          {saving ? t("common.saving") : t("settings.backupReminder.saveSubmit")}
        </button>
      </div>
    </form>
  );
}

export default BackupReminderForm;
