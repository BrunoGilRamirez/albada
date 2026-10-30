import { useTranslation } from "react-i18next";
import type { Priority } from "../../lib/types";

// Presentation-only map. Every `Priority` key is unchanged; the visible label
// comes from the shared enum catalog (`common.labels.priority`).
const PRIORITY_STYLES: Record<Priority, string> = {
  none: "joi-badge--neutral",
  low: "joi-badge--neutral",
  normal: "joi-badge--info",
  high: "joi-badge--warning",
  critical: "joi-badge--danger",
};

export function TaskPriorityBadge({ priority }: { priority: Priority }) {
  const { t } = useTranslation();
  return (
    <span className={`joi-badge ${PRIORITY_STYLES[priority]}`}>
      {t(`common.labels.priority.${priority}`)}
    </span>
  );
}

export default TaskPriorityBadge;
