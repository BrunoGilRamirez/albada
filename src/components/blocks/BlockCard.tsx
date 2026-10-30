import { useState } from "react";
import { useTranslation } from "react-i18next";
import { cancelBlock, completeBlock, skipBlock, startBlock } from "../../lib/api/blockApi";
import { toDisplayMessage } from "../../lib/errors";
import type { Block, BlockStatus } from "../../lib/types";

// Presentation-only map. Keys, labels and `BlockStatus` values are unchanged;
// `completed` keeps its success role, `active` its informational role, and every
// other status stays neutral. Every status keeps its visible label in JSX (the
// localized text now comes from the Today catalog, `today.block.statuses`).
const STATUS_BADGE_CLASSES: Record<BlockStatus, string> = {
  planned: "joi-badge--neutral",
  active: "joi-badge--info",
  completed: "joi-badge--success",
  skipped: "joi-badge--neutral",
  cancelled: "joi-badge--neutral",
};

interface BlockCardProps {
  block: Block;
  onChanged: (block: Block) => void;
  /** Opens the block form in edit mode. Hidden for terminal statuses. */
  onEdit: (block: Block) => void;
  /**
   * Disables every action while an optimistic block save is in flight, so a
   * concurrent status write cannot interleave with the save's pending row
   * or its scoped rollback.
   */
  actionDisabled?: boolean;
  /**
   * Explicit lifecycle deactivation. Rendered only for an active block that
   * has not started yet: a started block must be ended first, so the native
   * command would reject it.
   */
  onDeactivate?: (block: Block) => Promise<void>;
  /** Explicit soft delete. Destructive, so it always confirms first. */
  onDelete?: (block: Block) => Promise<void>;
}

function BlockCard({
  block,
  onChanged,
  onEdit,
  actionDisabled = false,
  onDeactivate,
  onDelete,
}: BlockCardProps) {
  const { t } = useTranslation();
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  const run = async (action: () => Promise<Block>) => {
    if (busy || actionDisabled) {
      return;
    }
    setError(null);
    setBusy(true);
    try {
      onChanged(await action());
    } catch (err) {
      setError(toDisplayMessage(err));
    } finally {
      setBusy(false);
    }
  };

  // Lifecycle writes are owned by the list that renders the card: the
  // handler performs the native call and updates the owning list. The card
  // only surfaces a failure through its own alert area.
  const runLifecycle = async (action: () => Promise<void>) => {
    if (busy || actionDisabled) {
      return;
    }
    setError(null);
    setBusy(true);
    try {
      await action();
    } catch (err) {
      setError(toDisplayMessage(err));
    } finally {
      setBusy(false);
    }
  };

  // Planned times are user-entered clock values and stay verbatim; the
  // en-dash join mirrors the commitment list rendering. `planned_end_at` is
  // a deprecated legacy value: it renders when present but the form never
  // submits or edits it (Phase 8.7.2).
  const times = [block.planned_start_at, block.planned_end_at].filter(Boolean).join(" – ");
  const durationMinutes =
    typeof block.estimated_duration_seconds === "number"
      ? Math.round(block.estimated_duration_seconds / 60)
      : null;
  // A materialized occurrence is managed through its weekly series, so the
  // lifecycle actions carry the routine-level labels.
  const isRoutine = block.recurrence_id !== null;

  return (
    <article
      data-notification-target={block.id}
      className="joi-surface--base joi-border--default flex min-w-0 flex-col gap-2 rounded-lg border px-3 py-2"
    >
      <div className="flex min-w-0 flex-col gap-2">
        <div className="min-w-0 flex-1">
          <div className="flex min-w-0 flex-wrap items-center gap-2">
            <span className="joi-text min-w-0 break-words text-sm font-medium">
              {block.label ?? t(`today.block.types.${block.block_type}`)}
            </span>
            <span className="joi-badge joi-badge--neutral shrink-0">
              {t(`today.block.types.${block.block_type}`)}
            </span>
            <span className={`joi-badge shrink-0 ${STATUS_BADGE_CLASSES[block.status]}`}>
              {t(`today.block.statuses.${block.status}`)}
            </span>
            {block.recurrence_id ? (
              <span className="joi-badge joi-badge--info shrink-0">
                {t("today.block.recurring")}
              </span>
            ) : null}
          </div>
          <p className="joi-text--muted mt-1 break-words text-xs">
            {times ? `${times} · ` : ""}
            {block.time_window_label
              ? t(`today.block.windows.${block.time_window_label}`)
              : t("today.block.noTime")}
            {durationMinutes !== null
              ? ` · ${t("today.block.durationMinutes", { count: durationMinutes })}`
              : ""}
            {block.flexibility !== "flexible"
              ? ` · ${t(`today.block.flexibilities.${block.flexibility}`)}`
              : ""}
          </p>
          {block.note_markdown ? (
            <p className="joi-text mt-1 break-words text-xs">{block.note_markdown}</p>
          ) : null}
        </div>
        <div className="flex min-w-0 flex-wrap justify-start gap-1">
          {block.status === "planned" || block.status === "active" ? (
            <button
              type="button"
              disabled={busy || actionDisabled}
              onClick={() => onEdit(block)}
              className="joi-btn joi-btn--secondary rounded-md border px-2 py-1 text-xs font-medium disabled:opacity-50"
            >
              {t("common.actions.edit")}
            </button>
          ) : null}
          {block.status === "planned" ? (
            <button
              type="button"
              disabled={busy || actionDisabled}
              onClick={() => run(() => startBlock(block.id))}
              className="joi-btn joi-btn--primary rounded-md px-2 py-1 text-xs font-medium disabled:opacity-50"
            >
              {t("common.actions.start")}
            </button>
          ) : null}
          {block.status === "active" ? (
            <button
              type="button"
              disabled={busy || actionDisabled}
              onClick={() => run(() => completeBlock(block.id))}
              className="joi-btn joi-btn--primary rounded-md px-2 py-1 text-xs font-medium disabled:opacity-50"
            >
              {t("common.actions.complete")}
            </button>
          ) : null}
          {block.status === "planned" || block.status === "active" ? (
            <>
              <button
                type="button"
                disabled={busy || actionDisabled}
                onClick={() => run(() => skipBlock(block.id))}
                className="joi-btn joi-btn--secondary rounded-md border px-2 py-1 text-xs font-medium disabled:opacity-50"
              >
                {t("common.actions.skip")}
              </button>
              <button
                type="button"
                disabled={busy || actionDisabled}
                onClick={() => run(() => cancelBlock(block.id))}
                className="joi-btn joi-btn--secondary rounded-md border px-2 py-1 text-xs font-medium disabled:opacity-50"
              >
                {t("common.actions.cancel")}
              </button>
            </>
          ) : null}
          {onDeactivate && block.lifecycle_state === "active" && block.status !== "active" ? (
            <button
              type="button"
              disabled={busy || actionDisabled}
              onClick={() => runLifecycle(() => onDeactivate(block))}
              className="joi-btn joi-btn--secondary rounded-md border px-2 py-1 text-xs font-medium disabled:opacity-50"
            >
              {isRoutine ? t("today.block.deactivateRoutine") : t("today.block.deactivate")}
            </button>
          ) : null}
          {onDelete && block.lifecycle_state === "active" ? (
            <button
              type="button"
              disabled={busy || actionDisabled}
              onClick={() => {
                // Deletion is soft but explicit: it always confirms first,
                // matching the existing destructive-action convention.
                if (!window.confirm(t("today.block.deleteConfirm"))) {
                  return;
                }
                void runLifecycle(() => onDelete(block));
              }}
              className="joi-btn joi-btn--danger rounded-md px-2 py-1 text-xs font-medium disabled:opacity-50"
            >
              {isRoutine ? t("today.block.deleteRoutine") : t("today.block.delete")}
            </button>
          ) : null}
        </div>
      </div>
      {error ? (
        <p role="alert" className="joi-text--danger mt-2 text-xs">
          {error}
        </p>
      ) : null}
    </article>
  );
}

export default BlockCard;
