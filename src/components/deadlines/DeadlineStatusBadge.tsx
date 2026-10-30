import { useTranslation } from "react-i18next";
import type { DeadlineStatus } from "../../lib/types";

// Presentation-only map. Keys and derived `DeadlineStatus` values are
// unchanged; the visible labels come from the shared enum catalog
// (`common.labels.deadlineStatus`), and `blocked_risk` keeps its distinct
// orange `caution` role.
const STATUS_STYLES: Record<DeadlineStatus, string> = {
  none: "joi-badge--neutral",
  normal: "joi-badge--success",
  approaching: "joi-badge--warning",
  overdue: "joi-badge--danger",
  blocked_risk: "joi-badge--caution",
};

interface DeadlineStatusBadgeProps {
  status: DeadlineStatus;
  deadlineAt?: string | null;
}

function DeadlineStatusBadge({ status, deadlineAt }: DeadlineStatusBadgeProps) {
  const { t } = useTranslation();
  if (status === "none") return null;
  const statusLabel = t(`common.labels.deadlineStatus.${status}`);
  return (
    <span
      className={`joi-badge shrink-0 ${STATUS_STYLES[status]}`}
      title={deadlineAt ? t("common.deadlineDate", { date: deadlineAt }) : statusLabel}
    >
      {statusLabel}
      {deadlineAt ? <span className="tabular-nums">{deadlineAt}</span> : null}
    </span>
  );
}

export default DeadlineStatusBadge;
