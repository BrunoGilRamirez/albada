import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { listSessionsForTask } from "../../lib/api/sessionApi";
import { toDisplayMessage } from "../../lib/errors";
import { formatDateTime, formatTime } from "../../lib/format";
import type { TaskSession } from "../../lib/types";
import { SESSION_STATUS_LABELS } from "../../lib/types";

interface SessionHistoryProps {
  taskId: string;
}

function SessionHistory({ taskId }: SessionHistoryProps) {
  const { t } = useTranslation();
  const [sessions, setSessions] = useState<TaskSession[]>([]);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    void listSessionsForTask(taskId)
      .then(setSessions)
      .catch((err) => setError(toDisplayMessage(err)));
  }, [taskId]);

  if (error) {
    return <p className="joi-text--danger">{error}</p>;
  }

  if (sessions.length === 0) {
    return <p className="joi-empty__state">{t("today.session.history.empty")}</p>;
  }

  return (
    <ul className="flex flex-col gap-2">
      {sessions.map((session) => {
        const started = session.started_at
          ? formatDateTime(session.started_at)
          : t("today.session.history.notStarted");
        const ended = session.ended_at ? formatTime(session.ended_at) : null;
        return (
          <li
            key={session.id}
            className="joi-surface--base joi-border--default rounded-lg border px-3 py-2 text-sm"
          >
            <div className="flex items-center justify-between gap-2">
              <span className="joi-text">{started}</span>
              <span className="joi-badge joi-badge--neutral shrink-0">
                {SESSION_STATUS_LABELS[session.status]}
              </span>
            </div>
            {ended ? (
              <p className="joi-text--muted mt-0.5 text-xs">
                {t("today.session.history.until")} {ended}
              </p>
            ) : null}
            {session.outcome_markdown ? (
              <p className="joi-text mt-1">{session.outcome_markdown}</p>
            ) : null}
            {session.next_step_markdown ? (
              <p className="joi-text--muted mt-0.5 text-xs italic">
                {t("today.session.history.next")} {session.next_step_markdown}
              </p>
            ) : null}
          </li>
        );
      })}
    </ul>
  );
}

export default SessionHistory;
