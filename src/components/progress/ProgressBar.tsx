import { useTranslation } from "react-i18next";
import { formatNumber } from "../../lib/format";

// Known derived progress-state labels come from the shared enum catalog
// (`common.labels.progressState`). Anything else is an arbitrary backend
// value: it keeps the legacy de-underscored display and is never keyed into
// the catalogs, so unknown values cannot crash rendering.
const KNOWN_PROGRESS_LABELS = [
  "no_tasks",
  "not_started",
  "archived",
  "cancelled",
  "abandoned",
] as const;

type ProgressStateKey = (typeof KNOWN_PROGRESS_LABELS)[number];

function isProgressStateKey(value: string): value is ProgressStateKey {
  return (KNOWN_PROGRESS_LABELS as readonly string[]).includes(value);
}

interface ProgressBarProps {
  percentage?: number | null;
  label?: string | null;
  size?: "sm" | "md";
}

function ProgressBar({ percentage, label, size = "md" }: ProgressBarProps) {
  const { t } = useTranslation();
  // Progress is derived from eligible task leaves only; therapy and journal records never contribute.
  const height = size === "sm" ? "h-1.5" : "h-2.5";
  const textSize = size === "sm" ? "text-xs" : "text-sm";

  const progressLabelText = (value: string): string => {
    // Narrowed by the guard below, the template literal type resolves to the
    // finite union of known `common.labels.progressState.*` keys; unknown
    // backend values keep the legacy de-underscored display and are never
    // keyed into the catalogs.
    if (isProgressStateKey(value)) {
      return t(`common.labels.progressState.${value}`);
    }
    return value.split("_").join(" ");
  };

  if (percentage === null || percentage === undefined) {
    return (
      <span className={`joi-text--subtle inline-flex items-center ${textSize}`}>
        {progressLabelText(label ?? "")}
      </span>
    );
  }

  const clamped = Math.max(0, Math.min(100, percentage));

  return (
    <span
      role="progressbar"
      aria-valuenow={Math.round(clamped)}
      aria-valuemin={0}
      aria-valuemax={100}
      // `.joi-progress` declares a column stack; the row orientation and the `size`
      // prop steps stay as preserved layout utilities.
      className={`joi-progress inline-flex flex-row items-center gap-2 ${textSize}`}
    >
      <span className={`joi-progress__track w-24 ${height}`}>
        <span
          className="joi-progress__fill block transition-all"
          style={{ width: `${clamped}%` }}
        />
      </span>
      <span className="joi-text tabular-nums">
        {formatNumber(clamped, { minimumFractionDigits: 1, maximumFractionDigits: 1 })}%
      </span>
    </span>
  );
}

export default ProgressBar;
