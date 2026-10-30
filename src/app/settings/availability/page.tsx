import { useCallback, useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import {
  createAvailabilityProfile,
  deleteAvailabilityProfile,
  listAvailabilityProfiles,
  updateAvailabilityProfile,
} from "../../../lib/api/availabilityProfileApi";
import { ensureDefaultWorkspace } from "../../../lib/api/workspaceApi";
import { toDisplayMessage } from "../../../lib/errors";
import { formatNumber } from "../../../lib/format";
import type { AvailabilityProfile } from "../../../lib/types";
import PageContainer from "../../../components/navigation/PageContainer";

interface FormState {
  weekday: number;
  start_time: string;
  end_time: string;
  preferred_block_length_minutes: string;
  energy_expectation: string;
}

const EMPTY: FormState = {
  weekday: 1,
  start_time: "09:00",
  end_time: "11:00",
  preferred_block_length_minutes: "60",
  energy_expectation: "",
};

const WEEKDAY_KEYS = [
  "sunday",
  "monday",
  "tuesday",
  "wednesday",
  "thursday",
  "friday",
  "saturday",
] as const;

/** Catalog key for a stored weekday index, or null for an out-of-range value. */
function weekdayKey(day: number): (typeof WEEKDAY_KEYS)[number] | null {
  return day >= 0 && day < WEEKDAY_KEYS.length ? WEEKDAY_KEYS[day] : null;
}

function AvailabilityPage() {
  const { t } = useTranslation();
  const [workspaceId, setWorkspaceId] = useState<string | null>(null);
  const [profiles, setProfiles] = useState<AvailabilityProfile[]>([]);
  const [form, setForm] = useState<FormState>(EMPTY);
  const [editingId, setEditingId] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [notice, setNotice] = useState<string | null>(null);
  const [submitting, setSubmitting] = useState(false);

  const load = useCallback(async () => {
    try {
      const workspace = await ensureDefaultWorkspace();
      setWorkspaceId(workspace.id);
      setProfiles(await listAvailabilityProfiles(workspace.id));
    } catch (err) {
      setError(toDisplayMessage(err));
    }
  }, []);

  useEffect(() => {
    void load();
  }, [load]);

  const set = <K extends keyof FormState>(key: K, value: FormState[K]) => {
    setForm((prev) => ({ ...prev, [key]: value }));
  };

  const startEdit = (profile: AvailabilityProfile) => {
    setEditingId(profile.id);
    setForm({
      weekday: profile.weekday,
      start_time: profile.start_time,
      end_time: profile.end_time,
      preferred_block_length_minutes: profile.preferred_block_length_minutes
        ? String(profile.preferred_block_length_minutes)
        : "",
      energy_expectation: profile.energy_expectation ?? "",
    });
  };

  const handleSubmit = async (event: React.FormEvent) => {
    event.preventDefault();
    setError(null);
    setNotice(null);
    if (!workspaceId) return;
    setSubmitting(true);
    try {
      const payload = {
        weekday: form.weekday,
        start_time: form.start_time,
        end_time: form.end_time,
        preferred_block_length_minutes: form.preferred_block_length_minutes
          ? Number.parseInt(form.preferred_block_length_minutes, 10)
          : null,
        energy_expectation: form.energy_expectation.trim() || null,
      };
      if (editingId) {
        await updateAvailabilityProfile(editingId, payload);
        setNotice(t("settings.availability.updated"));
        setEditingId(null);
      } else {
        await createAvailabilityProfile({ workspace_id: workspaceId, ...payload });
        setNotice(t("settings.availability.added"));
        setForm(EMPTY);
      }
      await load();
    } catch (err) {
      setError(toDisplayMessage(err));
    } finally {
      setSubmitting(false);
    }
  };

  const handleDelete = async (id: string) => {
    setError(null);
    setNotice(null);
    try {
      await deleteAvailabilityProfile(id);
      if (editingId === id) setEditingId(null);
      await load();
    } catch (err) {
      setError(toDisplayMessage(err));
    }
  };

  const inputClass = "joi-field__input";
  const selectClass = "joi-field__select";
  const labelClass = "joi-field__label mb-1 block";

  return (
    <PageContainer>
      <h1 className="text-2xl font-semibold">{t("settings.availability.title")}</h1>
      <p className="joi-text--muted mt-2">{t("settings.availability.subtitle")}</p>

      {error ? (
        <p role="alert" className="joi-text--danger mt-4 text-sm">
          {error}
        </p>
      ) : null}
      {notice ? (
        <p role="status" className="joi-text--success mt-4 text-sm">
          {notice}
        </p>
      ) : null}

      <form
        onSubmit={handleSubmit}
        className="joi-surface--base joi-border--default mt-6 grid grid-cols-1 gap-4 rounded-lg border p-4"
      >
        <div className="grid grid-cols-1 gap-4 sm:grid-cols-2">
          <div>
            <label htmlFor="availability-weekday" className={labelClass}>
              {t("settings.availability.weekday")}
            </label>
            <select
              id="availability-weekday"
              className={selectClass}
              value={form.weekday}
              onChange={(e) => set("weekday", Number.parseInt(e.target.value, 10))}
            >
              {WEEKDAY_KEYS.map((key, index) => (
                <option key={key} value={index}>
                  {t(`common.labels.weekday.${key}`)}
                </option>
              ))}
            </select>
          </div>
          <div>
            <label htmlFor="availability-energy" className={labelClass}>
              {t("settings.availability.energy")}
            </label>
            <input
              id="availability-energy"
              className={inputClass}
              value={form.energy_expectation}
              onChange={(e) => set("energy_expectation", e.target.value)}
              placeholder={t("settings.availability.energyPlaceholder")}
            />
          </div>
        </div>
        <div className="grid grid-cols-1 gap-4 sm:grid-cols-3">
          <div>
            <label htmlFor="availability-start" className={labelClass}>
              {t("settings.availability.start")}
            </label>
            <input
              id="availability-start"
              type="time"
              className={inputClass}
              value={form.start_time}
              onChange={(e) => set("start_time", e.target.value)}
              required
            />
          </div>
          <div>
            <label htmlFor="availability-end" className={labelClass}>
              {t("settings.availability.end")}
            </label>
            <input
              id="availability-end"
              type="time"
              className={inputClass}
              value={form.end_time}
              onChange={(e) => set("end_time", e.target.value)}
              required
            />
          </div>
          <div>
            <label htmlFor="availability-length" className={labelClass}>
              {t("settings.availability.preferredLength")}
            </label>
            <input
              id="availability-length"
              type="number"
              min={5}
              max={600}
              className={inputClass}
              value={form.preferred_block_length_minutes}
              onChange={(e) => set("preferred_block_length_minutes", e.target.value)}
            />
          </div>
        </div>
        <div className="flex items-center gap-2">
          <button
            type="submit"
            disabled={submitting}
            className="joi-btn joi-btn--primary rounded-lg px-4 py-2"
          >
            {submitting
              ? t("common.saving")
              : editingId
                ? t("settings.availability.updateSubmit")
                : t("settings.availability.addSubmit")}
          </button>
          {editingId ? (
            <button
              type="button"
              onClick={() => setEditingId(null)}
              className="joi-btn joi-btn--secondary rounded-lg border px-4 py-2"
            >
              {t("settings.availability.cancelEdit")}
            </button>
          ) : null}
        </div>
      </form>

      <div className="mt-6">
        <h2 className="text-lg font-medium">{t("settings.availability.profiles")}</h2>
        {profiles.length === 0 ? (
          <p className="joi-empty__state mt-2">{t("settings.availability.profilesEmpty")}</p>
        ) : (
          <ul className="mt-2 flex flex-col gap-2">
            {profiles.map((profile) => {
              const dayKey = weekdayKey(profile.weekday);
              return (
                <li
                  key={profile.id}
                  className="joi-surface--base joi-border--default flex items-center justify-between gap-3 rounded-lg border px-3 py-2"
                >
                  <div className="min-w-0">
                    <span className="joi-text block text-sm">
                      {dayKey ? t(`common.labels.weekday.${dayKey}`) : profile.weekday} ·{" "}
                      {profile.start_time} – {profile.end_time}
                    </span>
                    {/* `energy_expectation` is user-entered free text and stays
                        verbatim in every locale. */}
                    <span className="joi-text--muted text-xs">
                      {profile.preferred_block_length_minutes
                        ? t("settings.availability.blockLength", {
                            minutes: formatNumber(profile.preferred_block_length_minutes),
                          })
                        : t("settings.availability.noPreferredLength")}
                      {profile.energy_expectation ? ` · ${profile.energy_expectation}` : ""}
                    </span>
                  </div>
                  <div className="flex shrink-0 gap-1">
                    <button
                      type="button"
                      onClick={() => startEdit(profile)}
                      className="joi-btn joi-btn--secondary joi-btn--sm"
                    >
                      {t("common.actions.edit")}
                    </button>
                    <button
                      type="button"
                      onClick={() => handleDelete(profile.id)}
                      className="joi-btn joi-btn--danger joi-btn--sm"
                    >
                      {t("common.actions.delete")}
                    </button>
                  </div>
                </li>
              );
            })}
          </ul>
        )}
      </div>
    </PageContainer>
  );
}

export default AvailabilityPage;
