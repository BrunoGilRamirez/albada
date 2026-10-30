import { useTranslation } from "react-i18next";
import { formatNumber } from "@/lib/format";

/** Job progress state received from the polling hook. */
export interface JobProgressState {
  /** Human-readable current phase/step (e.g. "Scanning…", "Committing…", "Running…") */
  phase: string;
  /** Progress percentage 0–100, or null when indeterminate */
  percent: number | null;
  /** Overall job status */
  status: "queued" | "running" | "completed" | "failed" | "cancelled";
  /** True when the job has reached a terminal state */
  terminal: boolean;
  /** Error message when status is "failed" */
  error: string | null;
  /** Recovery action label, e.g. "View report" */
  recoveryLabel: string | null;
  /** Callback when the user clicks the recovery action */
  onRecovery: () => void;
  /** Callback when the user requests cancellation (should NOT cancel the native job) */
  onCancelRequest: () => void;
}

/**
 * JobProgressModal – shared between import and export flows.
 *
 * - Shows current phase and determinate progress when percent is known,
 *   indeterminate spinner otherwise.
 * - Cancel button requests cooperative cancellation through onCancelRequest
 *   (the job stops at the next safe point; terminal jobs are unaffected) and
 *   dismisses the dialog. This prevents duplicate submit while running.
 * - Close/minimize buttons dismiss the modal without cancelling the job.
 * - When the job fails, a recovery link is shown; clicking it navigates
 *   the user to the report view (caller provides the navigation logic).
 * - role="status" / aria-live="polite" for screen readers.
 * - The modal's own chrome comes from `common.jobProgress.*`; `state.phase`
 *   and `state.recoveryLabel` are caller-owned strings that the caller
 *   localizes (see the 8.6.11 import screen for the `settings.import.*` and
 *   `common.exportJob.*` sources).
 */
export function JobProgressModal({
  state,
  onClose,
}: {
  state: JobProgressState;
  onClose: () => void;
}) {
  const { t } = useTranslation();
  const handleCancel = () => {
    // Cooperative cancellation: the caller cancels the native job
    // (no-op when already terminal) and dismisses the dialog.
    state.onCancelRequest();
    onClose();
  };

  // When the job reaches a terminal state and there's an error with a
  // recovery action, show the recovery link.
  const showRecovery =
    state.terminal && state.status === "failed" && state.recoveryLabel && state.onRecovery;

  return (
    <div
      role="status"
      aria-live="polite"
      aria-label={t("common.jobProgress.ariaLabel", { phase: state.phase })}
      className="joi-surface--base rounded-lg border p-4 joi-border--default"
    >
      <div className="flex items-center justify-between">
        <h3 className="joi-text text-sm font-medium">{t("common.jobProgress.title")}</h3>
        <button onClick={handleCancel} className="joi-btn joi-btn--secondary joi-btn--sm">
          {t("common.actions.cancel")}
        </button>
      </div>

      <p className="joi-text--muted mt-2 text-sm">{state.phase}</p>

      {/* Determinate progress bar or indeterminate spinner */}
      {state.percent !== null ? (
        <div className="mt-3">
          <div className="joi-text--muted flex items-center gap-2 text-xs">
            <span>{t("common.jobProgress.percent", { percent: formatNumber(state.percent) })}</span>
            <span>{t("common.jobProgress.percentMax")}</span>
          </div>
          <div className="joi-progress__track" aria-hidden="true">
            <div
              className={`joi-progress__fill transition-width duration-500 ${state.percent >= 100 ? "w-full" : ""}`}
            />
          </div>
        </div>
      ) : (
        <div className="joi-text--muted mt-3 flex items-center gap-2 text-xs">
          <span
            aria-hidden="true"
            className="joi-border--strong animate-spin h-4 w-4 rounded-full border-2"
          />
          {t("common.jobProgress.pending")}
        </div>
      )}

      {/* Cancel callout – cooperative cancel, then dismiss */}
      {!state.terminal && (
        <p className="joi-text--muted mt-3 text-xs">{t("common.jobProgress.cancelNote")}</p>
      )}

      {/* Recovery link after failure */}
      {showRecovery && (
        <div className="mt-3 text-sm">
          <button
            type="button"
            onClick={state.onRecovery}
            className="joi-interactive joi-text font-medium underline"
          >
            {state.recoveryLabel}
          </button>
        </div>
      )}

      {/* Non-blocking notice */}
      {!state.terminal && (
        <p className="joi-text--muted mt-3 text-xs">{t("common.jobProgress.backgroundNote")}</p>
      )}

      {/* Close button (minimize equivalent) – never cancels the job */}
      <button
        onClick={onClose}
        className="joi-btn joi-btn--secondary joi-btn--sm mt-3"
        aria-label={t("common.jobProgress.closeAria")}
      >
        {t("common.actions.close")}
      </button>
    </div>
  );
}

export default JobProgressModal;
