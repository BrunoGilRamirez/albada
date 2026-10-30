import React, { useEffect } from "react";
import ReactDOM from "react-dom/client";
import App from "./App";
import ErrorBoundary from "./components/ErrorBoundary";
import NotificationConsentModal from "./components/notifications/NotificationConsentModal";
import "./index.css";
import { initI18n } from "./lib/i18n";
import { ThemeProvider } from "./lib/theme";
import { startNotificationDeliveryLoop } from "./lib/notifications/deliveryLoop";
import { NotificationIntentProvider } from "./lib/notifications/notificationIntent";
import { markStartupPhase, reportUnknownError } from "./lib/tauri";

// Startup timing: this fires once the whole static module graph (App, router,
// CSS, providers) has been fetched and evaluated. Together with the
// first-commit and first-invoke marks it separates "module fetch/eval" from
// "React mount" from "native IPC warm-up" — in the console and, through the
// `report_frontend_startup` boundary, in the Rust terminal/log, which is the
// evidence needed for any cold-start hypothesis. Local only.
markStartupPhase("module-eval");

// Log unhandled rejections and errors so command failures are traceable even
// when the UI only shows a generic message. The console lines stay for the
// (often closed) WebView console, and reportUnknownError mirrors them to the
// native log/terminal through the typed command boundary. Both paths are
// fire-and-forget and never re-enter the reporting channel.
window.addEventListener("unhandledrejection", (event) => {
  console.error("[unhandledrejection]", event.reason);
  reportUnknownError("unhandled_rejection", event.reason);
});
window.addEventListener("error", (event) => {
  console.error("[window.error]", event.message, event.error);
  reportUnknownError("window_error", event.error ?? event.message);
});

let firstCommitMarked = false;

/**
 * Zero-size probe: its effect runs after the first React commit of the app
 * shell, which is also the moment the inline splash hands off to React.
 * The module-level flag keeps the mark single even under StrictMode's
 * double-invoked effects.
 */
function StartupProbe() {
  useEffect(() => {
    if (firstCommitMarked) return;
    firstCommitMarked = true;
    markStartupPhase("first-commit");
  }, []);
  return null;
}

function renderApp(): void {
  ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
    <ErrorBoundary>
      <StartupProbe />
      <React.StrictMode>
        <ThemeProvider>
          <NotificationIntentProvider>
            <App />
          </NotificationIntentProvider>
          <NotificationConsentModal />
        </ThemeProvider>
      </React.StrictMode>
    </ErrorBoundary>,
  );
}

// Locale bootstrap before the first React commit: ensure the default
// workspace, resolve its persisted preference (Tauri OS locale when it is
// `system`), and initialize i18next from the statically bundled catalogs.
// The inline HTML splash remains visible throughout, so the app never renders
// fallback English and then switches. Every resolution failure inside
// initI18n lands on the English fallback; this guard only covers a
// pathological init error, which still must render the app (splash copy is
// intentionally English-only).
void initI18n()
  .catch((error) => {
    console.error("[i18n] locale bootstrap failed; rendering with defaults", error);
    reportUnknownError("window_error", error);
  })
  .finally(() => {
    // Start only after i18next resolved, so the first delivery poll can send
    // the real runtime locale to Rust for system-mode localization. The loop
    // only checks permission state; the consent dialog is the first-startup
    // place that may ask the user for it.
    startNotificationDeliveryLoop();
    renderApp();
  });
