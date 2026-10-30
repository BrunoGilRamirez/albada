import { useEffect, useState } from "react";
import { Link } from "react-router";
import { useTranslation } from "react-i18next";
import { whyIsBlocked } from "../../lib/api/taskApi";
import type { PrerequisiteTask } from "../../lib/types";

interface BlockedReasonPanelProps {
  taskId: string;
}

/**
 * Explains why a task is blocked by listing its incomplete prerequisites.
 * Fetches the derived blocked state from the native layer.
 */
function BlockedReasonPanel({ taskId }: BlockedReasonPanelProps) {
  const { t } = useTranslation();
  const [blockers, setBlockers] = useState<PrerequisiteTask[] | null>(null);

  useEffect(() => {
    let cancelled = false;
    void whyIsBlocked(taskId)
      .then((result) => {
        if (!cancelled) setBlockers(result);
      })
      .catch(() => {
        if (!cancelled) setBlockers([]);
      });
    return () => {
      cancelled = true;
    };
  }, [taskId]);

  if (blockers === null) {
    return null;
  }

  if (blockers.length === 0) {
    return <p className="joi-text--muted text-sm">{t("tasks.notBlocked")}</p>;
  }

  return (
    <div className="joi-alert joi-alert--warning">
      <p className="mb-2 text-sm font-medium">{t("tasks.whyBlocked")}</p>
      <ul className="flex flex-col gap-1">
        {blockers.map((blocker) => (
          <li key={blocker.task_id} className="text-sm">
            <Link to={`/tasks/${blocker.task_id}`} className="underline">
              {blocker.title}
            </Link>
          </li>
        ))}
      </ul>
    </div>
  );
}

export default BlockedReasonPanel;
