import { Link } from "react-router";
import { useTranslation } from "react-i18next";
import { formatDateTime } from "../../lib/format";
import type { DeadlineStatus, DeadlineTaskInfo } from "../../lib/types";

const SEVERITY_ORDER: Record<DeadlineStatus, number> = {
  overdue: 4,
  blocked_risk: 3,
  approaching: 2,
  normal: 1,
  none: 0,
};

interface DeadlineWarningListProps {
  tasks: DeadlineTaskInfo[];
}

function DeadlineWarningList({ tasks }: DeadlineWarningListProps) {
  const { t } = useTranslation();
  const sorted = [...tasks].sort((a, b) => {
    const severity = SEVERITY_ORDER[b.deadline_status] - SEVERITY_ORDER[a.deadline_status];
    if (severity !== 0) return severity;
    if (a.deadline_at && b.deadline_at) return a.deadline_at.localeCompare(b.deadline_at);
    if (a.deadline_at) return -1;
    if (b.deadline_at) return 1;
    return a.title.localeCompare(b.title);
  });

  if (sorted.length === 0) {
    return <p className="joi-empty__state">{t("today.deadline.empty")}</p>;
  }

  return (
    <ul className="flex flex-col gap-2">
      {sorted.map((task) => (
        <li
          key={task.task_id}
          className="joi-surface--base flex items-center gap-3 rounded-lg border joi-border--default px-3 py-2"
        >
          <Link
            to={`/tasks/${task.task_id}`}
            className="joi-text min-w-0 flex-1 truncate text-sm hover:underline"
          >
            {task.title}
          </Link>
          {task.deadline_at ? (
            <span className="joi-text--subtle shrink-0 text-xs tabular-nums">
              {formatDateTime(task.deadline_at)}
            </span>
          ) : null}
          <span
            className={`joi-badge shrink-0 ${
              task.deadline_status === "overdue"
                ? "joi-badge--danger"
                : task.deadline_status === "blocked_risk"
                  ? "joi-badge--caution"
                  : "joi-badge--warning"
            }`}
          >
            {t(`today.deadline.statuses.${task.deadline_status}`)}
          </span>
        </li>
      ))}
    </ul>
  );
}

export default DeadlineWarningList;
