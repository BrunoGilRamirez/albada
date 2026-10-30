import { useCallback, useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import {
  createFixedCommitment,
  deleteFixedCommitment,
  listFixedCommitments,
  setFixedCommitmentActive,
} from "../../../lib/api/fixedCommitmentApi";
import { ensureDefaultWorkspace } from "../../../lib/api/workspaceApi";
import { toDisplayMessage } from "../../../lib/errors";
import { formatNumber } from "../../../lib/format";
import type { FixedCommitment, FixedCommitmentType } from "../../../lib/types";
import PageContainer from "../../../components/navigation/PageContainer";
import { FIXED_COMMITMENT_TYPES } from "../../../lib/types";

interface FormState {
  title: string;
  commitment_type: FixedCommitmentType;
  start_at: string;
  end_at: string;
  duration_minutes: string;
  movable: boolean;
  description_markdown: string;
}

const EMPTY: FormState = {
  title: "",
  commitment_type: "work",
  start_at: "",
  end_at: "",
  duration_minutes: "",
  movable: false,
  description_markdown: "",
};

function CommitmentRow({
  commitment,
  onChanged,
}: {
  commitment: FixedCommitment;
  onChanged: () => void;
}) {
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const { t } = useTranslation();

  const run = async (action: () => Promise<unknown>) => {
    setError(null);
    setBusy(true);
    try {
      await action();
      onChanged();
    } catch (err) {
      setError(toDisplayMessage(err));
    } finally {
      setBusy(false);
    }
  };

  const window = [commitment.start_at, commitment.end_at].filter(Boolean).join(" – ");

  return (
    <li className="joi-surface--base joi-border--default rounded-lg border px-3 py-2">
      <div className="flex items-center justify-between gap-3">
        <div className="min-w-0">
          <span className="joi-text block truncate text-sm">{commitment.title}</span>
          <span className="joi-text--muted text-xs">
            {t(`common.labels.commitmentType.${commitment.commitment_type}`)}
            {window ? ` · ${window}` : ""}
            {commitment.duration_minutes
              ? ` · ${t("settings.commitments.minutes", {
                  minutes: formatNumber(commitment.duration_minutes),
                })}`
              : ""}
            {commitment.movable
              ? ` · ${t("settings.commitments.movableBadge")}`
              : ` · ${t("settings.commitments.fixedBadge")}`}
          </span>
        </div>
        <div className="flex shrink-0 gap-1">
          <button
            type="button"
            disabled={busy}
            onClick={() => run(() => setFixedCommitmentActive(commitment.id, !commitment.active))}
            className={`joi-btn rounded-md px-2 py-1 text-xs font-medium disabled:opacity-50 ${
              commitment.active ? "joi-btn--secondary" : "joi-btn--primary"
            }`}
          >
            {commitment.active ? t("common.actions.deactivate") : t("common.actions.activate")}
          </button>
          <button
            type="button"
            disabled={busy}
            onClick={() => run(() => deleteFixedCommitment(commitment.id))}
            className="joi-btn joi-btn--danger rounded-md px-2 py-1 text-xs font-medium"
          >
            {t("common.actions.delete")}
          </button>
        </div>
      </div>
      {error ? (
        <p role="alert" className="joi-text--danger mt-1 text-xs">
          {error}
        </p>
      ) : null}
    </li>
  );
}

function CommitmentsPage() {
  const { t } = useTranslation();
  const [workspaceId, setWorkspaceId] = useState<string | null>(null);
  const [commitments, setCommitments] = useState<FixedCommitment[]>([]);
  const [form, setForm] = useState<FormState>(EMPTY);
  const [error, setError] = useState<string | null>(null);
  const [notice, setNotice] = useState<string | null>(null);
  const [submitting, setSubmitting] = useState(false);

  const load = useCallback(async () => {
    try {
      const workspace = await ensureDefaultWorkspace();
      setWorkspaceId(workspace.id);
      setCommitments(await listFixedCommitments(workspace.id, false));
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

  const handleCreate = async (event: React.FormEvent) => {
    event.preventDefault();
    setError(null);
    setNotice(null);
    if (!workspaceId || !form.title.trim()) return;
    setSubmitting(true);
    try {
      await createFixedCommitment({
        workspace_id: workspaceId,
        title: form.title,
        description_markdown: form.description_markdown.trim() || null,
        commitment_type: form.commitment_type,
        start_at: form.start_at || null,
        end_at: form.end_at || null,
        duration_minutes: form.duration_minutes ? Number.parseInt(form.duration_minutes, 10) : null,
        recurrence_rule: null,
        movable: form.movable,
        privacy_scope: "private",
      });
      setNotice(t("settings.commitments.added"));
      setForm(EMPTY);
      await load();
    } catch (err) {
      setError(toDisplayMessage(err));
    } finally {
      setSubmitting(false);
    }
  };

  const inputClass = "joi-field__input";
  const selectClass = "joi-field__select";
  const labelClass = "joi-field__label mb-1 block";

  return (
    <PageContainer>
      <h1 className="text-2xl font-semibold">{t("settings.commitments.title")}</h1>
      <p className="joi-text--muted mt-2">{t("settings.commitments.subtitle")}</p>

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
        onSubmit={handleCreate}
        className="joi-surface--base joi-border--default mt-6 grid grid-cols-1 gap-4 rounded-lg border p-4"
      >
        <div className="grid grid-cols-1 gap-4 sm:grid-cols-2">
          <div>
            <label htmlFor="commitment-title" className={labelClass}>
              {t("common.fields.title")}
            </label>
            <input
              id="commitment-title"
              className={inputClass}
              value={form.title}
              onChange={(e) => set("title", e.target.value)}
              required
            />
          </div>
          <div>
            <label htmlFor="commitment-type" className={labelClass}>
              {t("settings.commitments.type")}
            </label>
            <select
              id="commitment-type"
              className={selectClass}
              value={form.commitment_type}
              onChange={(e) => set("commitment_type", e.target.value as FixedCommitmentType)}
            >
              {FIXED_COMMITMENT_TYPES.map((type) => (
                <option key={type} value={type}>
                  {t(`common.labels.commitmentType.${type}`)}
                </option>
              ))}
            </select>
          </div>
        </div>
        <div className="grid grid-cols-1 gap-4 sm:grid-cols-3">
          <div>
            <label htmlFor="commitment-start" className={labelClass}>
              {t("settings.commitments.start")}
            </label>
            <input
              id="commitment-start"
              type="time"
              className={inputClass}
              value={form.start_at}
              onChange={(e) => set("start_at", e.target.value)}
            />
          </div>
          <div>
            <label htmlFor="commitment-end" className={labelClass}>
              {t("settings.commitments.end")}
            </label>
            <input
              id="commitment-end"
              type="time"
              className={inputClass}
              value={form.end_at}
              onChange={(e) => set("end_at", e.target.value)}
            />
          </div>
          <div>
            <label htmlFor="commitment-duration" className={labelClass}>
              {t("settings.commitments.duration")}
            </label>
            <input
              id="commitment-duration"
              type="number"
              min={5}
              className={inputClass}
              value={form.duration_minutes}
              onChange={(e) => set("duration_minutes", e.target.value)}
            />
          </div>
        </div>
        <label className="joi-text--muted flex items-center gap-2 text-sm">
          <input
            type="checkbox"
            checked={form.movable}
            onChange={(e) => set("movable", e.target.checked)}
            className="joi-checkbox h-4 w-4"
          />
          {t("settings.commitments.movable")}
        </label>
        <button
          type="submit"
          disabled={submitting}
          className="joi-btn joi-btn--primary rounded-lg px-4 py-2"
        >
          {submitting ? t("common.actions.adding") : t("settings.commitments.addSubmit")}
        </button>
      </form>

      <div className="mt-6">
        <h2 className="text-lg font-medium">{t("settings.commitments.listTitle")}</h2>
        {commitments.length === 0 ? (
          <p className="joi-empty__state mt-2">{t("settings.commitments.empty")}</p>
        ) : (
          <ul className="mt-2 flex flex-col gap-2">
            {commitments.map((commitment) => (
              <CommitmentRow
                key={commitment.id}
                commitment={commitment}
                onChanged={() => void load()}
              />
            ))}
          </ul>
        )}
      </div>
    </PageContainer>
  );
}

export default CommitmentsPage;
