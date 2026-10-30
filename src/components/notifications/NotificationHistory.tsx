import { useCallback, useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import {
  clearNotificationHistory,
  dismissNotification,
  listNotificationDeliveries,
  listReminderRules,
  snoozeNotification,
} from "@/lib/api/notificationApi";
import { toDisplayMessage } from "@/lib/errors";
import { formatDateTime as intlDateTime, formatNumber } from "@/lib/format";
import type { NotificationDelivery, NotificationDeliveryStatus } from "@/lib/types";
import {
  isRoutableNotificationTarget,
  publishNotificationIntent,
} from "@/lib/notifications/notificationIntent";

interface NotificationHistoryProps {
  workspaceId: string;
  /**
   * Called after a successful dismiss/snooze so the shell (the bell badge) can
   * refresh the unread count without the panel owning that state.
   */
  onChanged?: () => void;
  /**
   * Called when a row with a supported target published its navigation intent
   * (Phase 8.7.5). The shell closes the history panel so the destination page
   * is visible; the target ID travels with the intent, not with the panel.
   */
  onNavigate?: () => void;
}

// Newest-first page shown in the shell panel. The native command defaults to
// 100 rows and caps a page at 500; the popover only needs the recent tail.
const HISTORY_LIMIT = 30;

const STATUS_TEXT_CLASSES: Record<NotificationDeliveryStatus, string> = {
  scheduled: "joi-text--warning",
  delivered: "joi-text--success",
  snoozed: "joi-text--info",
  dismissed: "joi-text--muted",
  failed: "joi-text--danger",
};

/**
 * Renders RFC 3339 timestamps in the reader's locale. A malformed value is
 * shown verbatim rather than as "Invalid Date", so history never renders
 * misleading text; the native layer only ever stores UTC timestamps.
 */
function formatDateTime(value: string): string {
  const date = new Date(value);
  return Number.isNaN(date.getTime()) ? value : intlDateTime(date);
}

/**
 * Recent notification deliveries (Phase 8.6). Rows carry only the
 * lock-screen-safe message copied from the rule at schedule time plus fixed
 * technical error text; the native list command can never return journal or
 * therapy content. Dismiss and snooze are offered only for `scheduled` rows
 * because the native domain rejects those actions for every other status.
 *
 * Phase 8.7.5: rows with a routable target (`task`, `project`, `goal`,
 * `block`) publish the existing in-app navigation intent (preserving the
 * target id) and then hand control to the shell via `onNavigate`. Unsupported
 * targets never navigate. The clear-history action uses the native
 * `clear_notification_history` command, which removes terminal in-app rows
 * only; it does not dismiss Windows toast notifications, which remain a
 * separate operating-system interaction.
 */
function NotificationHistory({ workspaceId, onChanged, onNavigate }: NotificationHistoryProps) {
  const { t } = useTranslation();
  const [deliveries, setDeliveries] = useState<NotificationDelivery[]>([]);
  const [snoozeOptionsByRule, setSnoozeOptionsByRule] = useState<Record<string, number[]>>({});
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  // Non-null after a successful clear: transient translated feedback, replaced
  // silently by any later action.
  const [clearedCount, setClearedCount] = useState<number | null>(null);

  const load = useCallback(async () => {
    try {
      const [rows, rules] = await Promise.all([
        listNotificationDeliveries({ workspace_id: workspaceId, limit: HISTORY_LIMIT }),
        listReminderRules(workspaceId),
      ]);
      const options: Record<string, number[]> = {};
      for (const rule of rules) {
        options[rule.id] = rule.snooze_minutes;
      }
      setDeliveries(rows);
      setSnoozeOptionsByRule(options);
      setError(null);
    } catch (err) {
      setError(toDisplayMessage(err));
    } finally {
      setLoading(false);
    }
  }, [workspaceId]);

  useEffect(() => {
    setLoading(true);
    void load();
  }, [load]);

  const runAction = async (action: () => Promise<unknown>) => {
    setBusy(true);
    setError(null);
    setClearedCount(null);
    try {
      await action();
      // Snooze creates a new scheduled row and dismiss rewrites the original,
      // so the list is reloaded instead of patched locally.
      await load();
      onChanged?.();
    } catch (err) {
      setError(toDisplayMessage(err));
    } finally {
      setBusy(false);
    }
  };

  async function handleClear(): Promise<void> {
    if (busy) return;
    // Destructive actions confirm with the shared window.confirm convention
    // (settings backups, journal archiving).
    if (!window.confirm(t("notifications.history.clearConfirm"))) return;
    await runAction(async () => {
      // The native command removes only terminal in-app history rows of this
      // workspace: no Windows toasts are dismissed (they are a separate
      // operating-system interaction) and future scheduled deliveries are
      // untouched.
      const cleared = await clearNotificationHistory(workspaceId);
      setClearedCount(cleared);
    });
  }

  const handleRowClick = useCallback(
    (delivery: NotificationDelivery) => {
      const target = delivery.target;
      // Unsupported targets ("session", future types) keep the informational
      // row: never silently route to an unrelated page.
      if (!target || !isRoutableNotificationTarget(target.entity_type)) return;
      publishNotificationIntent({
        kind: target.entity_type,
        entityType: target.entity_type,
        entityId: target.entity_id,
      });
      // The intent carries the preserved target id; the shell closes the
      // panel so the destination page becomes visible.
      onNavigate?.();
    },
    [onNavigate],
  );

  const handleKeyDown = useCallback(
    (event: React.KeyboardEvent, delivery: NotificationDelivery) => {
      if (event.key === "Enter" || event.key === " ") {
        event.preventDefault();
        handleRowClick(delivery);
      }
    },
    [handleRowClick],
  );

  return (
    <section aria-label={t("notifications.history.aria")} className="p-3">
      <div className="flex items-start justify-between gap-2">
        <div className="min-w-0">
          <h2 className="joi-text text-sm font-semibold">{t("notifications.history.title")}</h2>
          <p className="joi-text--muted mt-1 text-xs">{t("notifications.history.privacyNote")}</p>
        </div>
        <button
          type="button"
          disabled={busy || loading}
          onClick={() => void handleClear()}
          aria-label={t("notifications.history.clearAria")}
          className="joi-btn joi-btn--ghost joi-btn--sm shrink-0"
        >
          {t("notifications.history.clear")}
        </button>
      </div>

      {clearedCount != null && !error ? (
        <p role="status" className="joi-text--success mt-2 text-sm">
          {t("notifications.history.cleared", { count: clearedCount })}
        </p>
      ) : null}

      {error ? (
        <p role="alert" className="joi-text--danger mt-3 text-sm">
          {error}
        </p>
      ) : null}

      {loading ? (
        <p className="joi-empty__loading mt-3">{t("notifications.history.loading")}</p>
      ) : deliveries.length === 0 ? (
        <p className="joi-empty__state mt-3">{t("notifications.history.empty")}</p>
      ) : (
        <div className="mt-3 flex flex-col gap-3">
          {deliveries.map((delivery) => {
            // Snooze durations are rule-backed, and the native domain rejects
            // snoozing non-rule deliveries, so a ruleless backup row never
            // gets snooze affordances.
            const snoozeMinutes =
              delivery.reminder_rule_id != null
                ? (snoozeOptionsByRule[delivery.reminder_rule_id] ?? [])
                : [];
            const isPending = delivery.status === "scheduled";
            // Only routable targets make the row navigable; a supportable
            // target never routes an unsupported type to an unrelated page.
            const navigable =
              delivery.target != null && isRoutableNotificationTarget(delivery.target.entity_type);
            const target = delivery.target;

            return (
              <article
                key={delivery.id}
                data-testid="notification-row"
                className={`rounded-md border p-3 joi-border--default ${
                  navigable ? "cursor-pointer" : "cursor-default"
                }`}
                onClick={(event) => {
                  if ((event.target as HTMLElement).closest("button, a")) return;
                  handleRowClick(delivery);
                }}
                onKeyDown={(event) => handleKeyDown(event, delivery)}
                role={navigable ? "button" : undefined}
                tabIndex={navigable ? 0 : -1}
                aria-label={
                  navigable
                    ? // The entity type is stored data and stays verbatim.
                      t("notifications.history.rowLabelTarget", {
                        message: delivery.message,
                        entity: target?.entity_type,
                      })
                    : t("notifications.history.rowLabel", { message: delivery.message })
                }
              >
                <div className="flex flex-wrap items-start justify-between gap-2">
                  {/* The delivery message is the rule's privacy-safe copy stored
                      natively; it is rendered verbatim in every locale. */}
                  <p className="joi-text text-sm font-medium">{delivery.message}</p>
                  <span
                    className={`shrink-0 text-xs font-medium ${STATUS_TEXT_CLASSES[delivery.status]}`}
                  >
                    {t(`notifications.deliveryStatuses.${delivery.status}`)}
                  </span>
                </div>
                <p className="joi-text--muted mt-1 text-xs">
                  {t("notifications.history.scheduled", {
                    date: formatDateTime(delivery.scheduled_at),
                  })}
                  {delivery.delivered_at
                    ? ` · ${t("notifications.history.delivered", {
                        date: formatDateTime(delivery.delivered_at),
                      })}`
                    : ""}
                </p>

                {delivery.error_message ? (
                  <p className="joi-text--danger mt-2 text-xs">{delivery.error_message}</p>
                ) : null}

                {isPending ? (
                  <div className="mt-2 flex flex-wrap gap-1">
                    <button
                      type="button"
                      disabled={busy}
                      onClick={() => void runAction(() => dismissNotification(delivery.id))}
                      className="joi-btn joi-btn--secondary joi-btn--sm"
                    >
                      {t("common.actions.dismiss")}
                    </button>
                    {snoozeMinutes.map((minutes) => (
                      <button
                        key={minutes}
                        type="button"
                        disabled={busy}
                        onClick={() =>
                          void runAction(() => snoozeNotification(delivery.id, minutes))
                        }
                        className="joi-btn joi-btn--secondary joi-btn--sm"
                      >
                        {t("notifications.history.snoozeAction", {
                          minutes: formatNumber(minutes),
                        })}
                      </button>
                    ))}
                  </div>
                ) : null}
              </article>
            );
          })}
        </div>
      )}
    </section>
  );
}

export default NotificationHistory;
