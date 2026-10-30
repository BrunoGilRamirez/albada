type LogLevel = "debug" | "info" | "warn" | "error";

/**
 * Minimal frontend logger. Logs to the browser console; the app has no
 * telemetry, so this is purely local and helps trace command failures.
 */
function log(level: LogLevel, context: string, detail?: unknown): void {
  const prefix = `[${context}]`;
  switch (level) {
    case "debug":
      console.debug(prefix, detail ?? "");
      break;
    case "info":
      console.info(prefix, detail ?? "");
      break;
    case "warn":
      console.warn(prefix, detail ?? "");
      break;
    case "error":
      console.error(prefix, detail ?? "");
      break;
  }
}

export const logger = {
  debug: (context: string, detail?: unknown) => log("debug", context, detail),
  info: (context: string, detail?: unknown) => log("info", context, detail),
  warn: (context: string, detail?: unknown) => log("warn", context, detail),
  error: (context: string, detail?: unknown) => log("error", context, detail),
};

/**
 * Lightweight startup timing marker: local console only, no telemetry, no
 * user data (the point name is a static code label). `performance.now()` is
 * relative to the document time origin, so marks are comparable across dev
 * and built windows and give evidence for cold-start hypotheses: module
 * evaluation vs first React commit vs first native IPC call. Returns the
 * elapsed ms so callers can reuse the same measurement for the native
 * `report_frontend_startup` mirror (see `src/lib/tauri.ts`).
 */
export function markStartup(point: string): number {
  const elapsed = typeof performance !== "undefined" ? Math.round(performance.now()) : -1;
  console.info(`[startup] ${point} @ +${elapsed}ms`);
  return elapsed;
}
