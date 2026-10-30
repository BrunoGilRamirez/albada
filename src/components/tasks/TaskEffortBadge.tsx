import { useTranslation } from "react-i18next";
import type { EffortSize } from "../../lib/types";
import { EFFORT_POINTS } from "../../lib/types";

// Presentation-only map. Effort size stays distinguishable through the label
// (from the shared enum catalog `common.labels.effortSize`) and the
// effort-points suffix, so every size shares the neutral badge role.
const EFFORT_STYLES: Record<EffortSize, string> = {
  trivial: "joi-badge--neutral",
  small: "joi-badge--neutral",
  medium: "joi-badge--neutral",
  large: "joi-badge--neutral",
  very_large: "joi-badge--neutral",
};

export function TaskEffortBadge({ effort }: { effort: EffortSize | null }) {
  const { t } = useTranslation();
  if (!effort) {
    return null;
  }
  return (
    <span className={`joi-badge ${EFFORT_STYLES[effort]}`}>
      {t(`common.labels.effortSize.${effort}`)}
      <span className="opacity-70">
        {t("tasks.pointsSuffix", { points: EFFORT_POINTS[effort] })}
      </span>
    </span>
  );
}

export default TaskEffortBadge;
