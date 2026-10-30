import { useEffect, useRef, useState } from "react";
import { Outlet, useLocation, useNavigate } from "react-router";
import AppSidebar from "../components/navigation/AppSidebar";
import CommandPalette from "../components/CommandPalette";
import { BackupOperationProvider } from "../lib/backupOperation";
import {
  notificationIntentPath,
  useNotificationIntent,
} from "../lib/notifications/notificationIntent";

export function NotificationIntentBridge() {
  const navigate = useNavigate();
  const location = useLocation();
  const { intent, clearIntent } = useNotificationIntent();
  const routedIntent = useRef<string | null>(null);

  useEffect(() => {
    if (!intent) return;
    const key = JSON.stringify(intent);
    if (routedIntent.current === key) return;
    routedIntent.current = key;
    navigate(notificationIntentPath(intent));
    clearIntent();
  }, [clearIntent, intent, navigate]);

  useEffect(() => {
    const params = new URLSearchParams(location.search);
    const target = params.get("notificationTarget") ?? params.get("notification");
    if (!target) return;

    let cancelled = false;
    let timer: number | undefined;
    // The target highlight lasts about one second (Phase 8.7.5). Reduced
    // motion swaps the smooth scroll for an instant jump; the highlight is a
    // theme-aware outline (shape cue), never color-only.
    const prefersReducedMotion =
      window.matchMedia?.("(prefers-reduced-motion: reduce)").matches ?? false;
    const findAndHighlight = () => {
      if (cancelled) return;
      const elements = Array.from(
        document.querySelectorAll<HTMLElement>("[data-notification-target]"),
      );
      const element = elements.find((candidate) => candidate.dataset.notificationTarget === target);
      if (!element) {
        // Deleted or unavailable targets simply never highlight; the routes
        // show their existing not-found behavior. No unrelated fallback page.
        timer = window.setTimeout(findAndHighlight, 100);
        return;
      }
      element.classList.add("notification-target-highlight");
      element.scrollIntoView({
        behavior: prefersReducedMotion ? "auto" : "smooth",
        block: "center",
      });
      if (element.matches("button, a, input, select, textarea"))
        element.focus({ preventScroll: true });
      timer = window.setTimeout(
        () => element.classList.remove("notification-target-highlight"),
        1000,
      );
    };
    findAndHighlight();
    return () => {
      cancelled = true;
      if (timer !== undefined) window.clearTimeout(timer);
    };
  }, [location.pathname, location.search]);

  return null;
}

function Layout() {
  const [isPaletteOpen, setIsPaletteOpen] = useState(false);
  const location = useLocation();

  return (
    <BackupOperationProvider>
      <div className="joi-shell flex h-dvh min-w-0 overflow-x-clip">
        <AppSidebar />
        <main className="joi-shell__main min-w-0 flex-1 overflow-y-auto p-4 sm:p-6 lg:p-8">
          <NotificationIntentBridge />
          <div
            key={`${location.pathname}${location.search}`}
            className="joi-route-transition min-w-0"
          >
            <Outlet />
          </div>
        </main>
        <CommandPalette isOpen={isPaletteOpen} onOpenChange={setIsPaletteOpen} />
      </div>
    </BackupOperationProvider>
  );
}

export default Layout;
