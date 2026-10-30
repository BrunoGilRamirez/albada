import { useCallback, useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import NotificationHistory from "./NotificationHistory";
import { listNotificationDeliveries } from "@/lib/api/notificationApi";
import { toDisplayMessage } from "@/lib/errors";
import { formatNumber } from "@/lib/format";
import { ensureDefaultWorkspace } from "@/lib/api/workspaceApi";

// The delivery model keeps no read receipts (Phase 8.0), so the badge counts
// the actionable pending set: `scheduled` deliveries that have not been
// delivered, dismissed, or snoozed yet. `delivered` rows remain as
// informational history and never re-badge the shell.
const PENDING_STATUS = "scheduled";
// The native command caps one history page at 500 rows; pending reminders are
// bounded far below that by the per-rule daily limits.
const COUNT_LIMIT = 500;

/**
 * App-shell notification bell (Phase 8.6). Shows the pending delivery count
 * and opens the history panel. It only calls the typed notification commands;
 * the panel inherits the same privacy guarantee: safe message text and fixed
 * technical errors, never journal or therapy content.
 */
function NotificationBell() {
  const { t } = useTranslation();
  const [workspaceId, setWorkspaceId] = useState<string | null>(null);
  const [workspaceError, setWorkspaceError] = useState<string | null>(null);
  const [pendingCount, setPendingCount] = useState(0);
  const [open, setOpen] = useState(false);
  const containerRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    let cancelled = false;
    ensureDefaultWorkspace()
      .then((workspace) => {
        if (!cancelled) setWorkspaceId(workspace.id);
      })
      .catch((err) => {
        // The bell must never block the shell; the panel surfaces the failure.
        if (!cancelled) setWorkspaceError(toDisplayMessage(err));
      });
    return () => {
      cancelled = true;
    };
  }, []);

  const refreshPendingCount = useCallback(async () => {
    if (!workspaceId) return;
    try {
      const pending = await listNotificationDeliveries({
        workspace_id: workspaceId,
        status: PENDING_STATUS,
        limit: COUNT_LIMIT,
      });
      setPendingCount(pending.length);
    } catch {
      // Keep the previous count; the panel shows the error when opened.
    }
  }, [workspaceId]);

  useEffect(() => {
    void refreshPendingCount();
  }, [refreshPendingCount]);

  useEffect(() => {
    if (!open) return;
    const handleClick = (event: MouseEvent) => {
      if (containerRef.current && !containerRef.current.contains(event.target as Node)) {
        setOpen(false);
      }
    };
    const handleKeyDown = (event: KeyboardEvent) => {
      if (event.key === "Escape") setOpen(false);
    };
    document.addEventListener("click", handleClick);
    document.addEventListener("keydown", handleKeyDown);
    return () => {
      document.removeEventListener("click", handleClick);
      document.removeEventListener("keydown", handleKeyDown);
    };
  }, [open]);

  const handleToggle = () => {
    const next = !open;
    setOpen(next);
    if (next) void refreshPendingCount();
  };

  const badgeLabel = pendingCount > 99 ? "99+" : formatNumber(pendingCount);
  const bellAriaLabel =
    pendingCount > 0 ? t("shell.bell.unread", { count: pendingCount }) : t("shell.bell.label");

  return (
    <div ref={containerRef} className="relative shrink-0">
      <button
        type="button"
        onClick={handleToggle}
        aria-expanded={open}
        aria-haspopup="true"
        aria-label={bellAriaLabel}
        className="joi-btn joi-btn--ghost relative p-2"
      >
        <svg
          aria-hidden="true"
          viewBox="0 0 24 24"
          fill="none"
          stroke="currentColor"
          strokeWidth="1.5"
          className="h-5 w-5"
        >
          <path
            strokeLinecap="round"
            strokeLinejoin="round"
            d="M14.857 17.082a23.848 23.848 0 0 0 5.454-1.31A8.967 8.967 0 0 1 18 9.75V9A6 6 0 0 0 6 9v.75a8.967 8.967 0 0 1-2.312 6.022c1.733.64 3.56 1.085 5.455 1.31m5.714 0a24.255 24.255 0 0 1-5.714 0m5.714 0a3 3 0 1 1-5.714 0"
          />
        </svg>
        {pendingCount > 0 ? (
          <span
            aria-hidden="true"
            className="joi-badge joi-badge--info absolute -right-0.5 -top-0.5 h-4 min-w-4 px-1 py-0 text-[10px] leading-none font-semibold"
          >
            {badgeLabel}
          </span>
        ) : null}
      </button>

      {open ? (
        <div
          aria-label={t("shell.bell.label")}
          className="joi-surface--base absolute left-full top-0 z-30 ml-2 max-h-[70vh] w-96 max-w-[calc(100vw-16rem)] overflow-y-auto rounded-lg border shadow-xl joi-border--default"
        >
          {workspaceId ? (
            <NotificationHistory
              workspaceId={workspaceId}
              onChanged={() => void refreshPendingCount()}
              // A navigated target's page must be visible: close the panel.
              onNavigate={() => setOpen(false)}
            />
          ) : workspaceError ? (
            <p role="alert" className="joi-text--danger p-4 text-sm">
              {workspaceError}
            </p>
          ) : (
            <p className="joi-empty__loading p-4">{t("shell.bell.loading")}</p>
          )}
        </div>
      ) : null}
    </div>
  );
}

export default NotificationBell;
