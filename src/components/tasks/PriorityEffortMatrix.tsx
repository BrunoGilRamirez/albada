import { useTranslation } from "react-i18next";
import type { EffortSize, Priority } from "../../lib/types";
import { EFFORT_POINTS } from "../../lib/types";

interface PriorityEffortMatrixProps {
  effort: EffortSize | null;
  priority: Priority;
  onSelect: (effort: EffortSize, priority: Priority) => void;
}

const PRIORITY_ORDER: Priority[] = ["low", "normal", "high", "critical"];
const EFFORT_ORDER: EffortSize[] = ["trivial", "small", "medium", "large", "very_large"];

// Stable catalog keys (`tasks.matrix.recommendation.*`), not visible copy: the
// matrix cell text comes from the active locale. Keys never derive from user
// or backend values.
type RecommendationKey =
  | "whenSpace"
  | "pending"
  | "plan"
  | "addressSoon"
  | "addressToday"
  | "prioritize"
  | "immediatePriority"
  | "divideAndPrioritize"
  | "divideAndAddress"
  | "divide"
  | "urgentlyDivide";

const RECOMMENDATION_KEYS: Record<EffortSize, Record<Priority, RecommendationKey>> = {
  trivial: {
    none: "whenSpace",
    low: "whenSpace",
    normal: "plan",
    high: "addressSoon",
    critical: "addressToday",
  },
  small: {
    none: "pending",
    low: "pending",
    normal: "plan",
    high: "prioritize",
    critical: "immediatePriority",
  },
  medium: {
    none: "plan",
    low: "plan",
    normal: "plan",
    high: "divideAndPrioritize",
    critical: "divideAndAddress",
  },
  large: {
    none: "divide",
    low: "divide",
    normal: "divide",
    high: "prioritize",
    critical: "urgentlyDivide",
  },
  very_large: {
    none: "divide",
    low: "divide",
    normal: "divide",
    high: "divide",
    critical: "divideAndAddress",
  },
};

function PriorityEffortMatrix({ effort, priority, onSelect }: PriorityEffortMatrixProps) {
  const { t } = useTranslation();
  const selectedEffort = effort ?? "trivial";
  const selectedPriority = priority === "none" ? "normal" : priority;

  return (
    <div>
      <p className="joi-text mb-2 text-sm font-medium">{t("tasks.matrix.title")}</p>
      <div className="overflow-x-auto">
        <table className="w-full border-collapse text-sm">
          <thead>
            <tr>
              <th className="joi-text--muted px-2 py-1 text-left text-xs font-medium">
                {t("tasks.matrix.corner")}
              </th>
              {PRIORITY_ORDER.map((p) => (
                <th key={p} className="joi-text--muted px-2 py-1 text-xs font-medium">
                  {t(`common.labels.priority.${p}`)}
                </th>
              ))}
            </tr>
          </thead>
          <tbody>
            {EFFORT_ORDER.map((e) => (
              <tr key={e}>
                <td className="joi-text px-2 py-1 font-medium">
                  {t(`common.labels.effortSize.${e}`)}{" "}
                  <span className="joi-text--subtle text-xs">
                    ({t("tasks.pointsSuffix", { points: EFFORT_POINTS[e] })})
                  </span>
                </td>
                {PRIORITY_ORDER.map((p) => {
                  const selected = e === selectedEffort && p === selectedPriority;
                  return (
                    <td key={p} className="p-0.5">
                      {/* Selected state stays recoverable from `aria-pressed`; the
                          neutral slate primary fill only reinforces it. */}
                      <button
                        type="button"
                        onClick={() => onSelect(e, p)}
                        aria-pressed={selected}
                        className={`joi-btn w-full justify-start px-2 py-2 text-left text-xs transition-colors ${
                          selected ? "joi-btn--primary" : "joi-btn--secondary"
                        }`}
                      >
                        {t(`tasks.matrix.recommendation.${RECOMMENDATION_KEYS[e][p]}`)}
                      </button>
                    </td>
                  );
                })}
              </tr>
            ))}
          </tbody>
        </table>
      </div>
      <p className="joi-text--subtle mt-1 text-xs">{t("tasks.matrix.hint")}</p>
    </div>
  );
}

export default PriorityEffortMatrix;
