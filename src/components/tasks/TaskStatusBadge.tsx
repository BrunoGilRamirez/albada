import { useTranslation } from "react-i18next";
import type { TaskStatus } from "../../lib/types";

// Presentation-only map. Every `TaskStatus` key is unchanged; the visible
// label comes from the shared enum catalog (`common.labels.taskStatus`), and
// the palette families collapse onto the ratified semantic status roles.
const STATUS_STYLES: Record<TaskStatus, string> = {
  inbox: "joi-badge--neutral",
  planned: "joi-badge--info",
  in_progress: "joi-badge--info",
  blocked: "joi-badge--warning",
  completed: "joi-badge--success",
  cancelled: "joi-badge--neutral",
  deferred: "joi-badge--neutral",
};

export function TaskStatusBadge({ status }: { status: TaskStatus }) {
  const { t } = useTranslation();
  return (
    <span className={`joi-badge ${STATUS_STYLES[status]}`}>
      {t(`common.labels.taskStatus.${status}`)}
    </span>
  );
}

export default TaskStatusBadge;
