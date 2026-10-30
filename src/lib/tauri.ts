import { invoke as tauriInvoke, isTauri } from "@tauri-apps/api/core";
import { logger, markStartup } from "./logger";

export class CommandInvocationError extends Error {
  readonly command: string;
  readonly cause: unknown;

  constructor(command: string, cause: unknown) {
    super(`Failed to invoke Tauri command: ${command}`);
    this.name = "CommandInvocationError";
    this.command = command;
    this.cause = cause;
  }
}

function describeError(error: unknown): string {
  if (typeof error === "string") return error;
  if (error instanceof Error) return `${error.name}: ${error.message}`;
  try {
    return JSON.stringify(error);
  } catch {
    return String(error);
  }
}

// --- startup timing ---------------------------------------------------------

export type FrontendStartupPhase = "module-eval" | "first-commit" | "first-invoke";

/**
 * Report one startup milestone to the native log through the
 * `report_frontend_startup` command boundary. Fixed contract (matches the
 * Rust `report_frontend_startup(report: FrontendStartupReportDto)`):
 * `{ report: { phase, elapsedMs, command? } }` with kebab-case phases and
 * elapsed measured against the document time origin. The Rust side mirrors
 * it to the `tauri dev` terminal and `albada.log`, so cold-start
 * evidence survives a closed WebView console. Fire-and-forget and
 * recursion-safe: it never throws, the failure fallback is a plain console
 * line (never a re-entry), and the payload is diagnostics only — phase
 * labels, a non-negative number, and a static command name, never user
 * data. Uses the raw IPC call on purpose: the diagnostics channel must not
 * consume the business first-invoke slot or double-log through the command
 * wrapper. Outside Tauri (tests, plain browser dev) it is a no-op.
 */
export function reportFrontendStartup(
  phase: FrontendStartupPhase,
  elapsedMs: number,
  command?: string,
): void {
  try {
    if (!isTauri()) return;
    // The Rust DTO rejects negative elapsed values; only report real marks.
    if (elapsedMs < 0) return;
    const report: { phase: FrontendStartupPhase; elapsedMs: number; command?: string } = {
      phase,
      elapsedMs,
    };
    if (command) {
      report.command = command;
    }
    void tauriInvoke("report_frontend_startup", { report }).catch(() => {
      // Native command not wired yet (or IPC down). Console-only fallback.
      logger.warn("startup-report:failed", phase);
    });
  } catch {
    // Reporting must never become a new error source.
  }
}

/**
 * One startup phase, both sinks: the local `[startup]` console line and the
 * native `report_frontend_startup` mirror.
 */
export function markStartupPhase(phase: FrontendStartupPhase, command?: string): void {
  const elapsedMs = markStartup(command ? `${phase}:${command}` : phase);
  reportFrontendStartup(phase, elapsedMs, command);
}

/**
 * Diagnostic commands are reporting channels, not business calls: they must
 * not consume the once-per-session first-invoke mark.
 */
const DIAGNOSTIC_COMMANDS = new Set(["report_frontend_error", "report_frontend_startup"]);

let firstInvokeMarked = false;

export async function invoke<T>(command: string, payload?: unknown): Promise<T> {
  if (!firstInvokeMarked && !DIAGNOSTIC_COMMANDS.has(command)) {
    firstInvokeMarked = true;
    // Static command label only; never the payload (which may hold user data).
    markStartupPhase("first-invoke", command);
  }
  logger.debug(`command`, { command, payload });
  try {
    const result = await tauriInvoke<T>(command, payload as Record<string, unknown> | undefined);
    logger.debug(`command:ok`, command);
    return result;
  } catch (error) {
    logger.error(`command:failed`, { command, raw: describeError(error), error });
    throw new CommandInvocationError(command, error);
  }
}

// --- in-flight deduplication --------------------------------------------------

const inFlight = new Map<string, Promise<unknown>>();

/**
 * Share one in-flight promise per key so duplicate startup mounts collapse
 * into a single native command: React StrictMode double-effects and several
 * components resolving the same resource (Today, NotificationBell) each call
 * the typed boundary, but only the first one reaches Tauri while the call is
 * still pending.
 *
 * Only concurrent calls are shared. The entry is cleared as soon as the
 * promise settles — including on failure, so the next caller retries — and
 * sequential refresh calls always re-run the command. Command semantics are
 * unchanged; this is transport-level deduplication, not caching.
 */
export function dedupeInFlight<T>(key: string, factory: () => Promise<T>): Promise<T> {
  const existing = inFlight.get(key);
  if (existing) return existing as Promise<T>;

  let started: Promise<T>;
  try {
    started = factory();
  } catch (error) {
    // A synchronous factory failure must not poison the key for later calls.
    return Promise.reject(error);
  }

  const tracked = started.finally(() => {
    if (inFlight.get(key) === tracked) {
      inFlight.delete(key);
    }
  });
  inFlight.set(key, tracked);
  return tracked;
}

// --- frontend error reporting -------------------------------------------------

export type FrontendErrorSource =
  "error_boundary" | "route_error" | "window_error" | "unhandled_rejection";

export interface FrontendErrorReport {
  source: FrontendErrorSource;
  message: string;
  name?: string;
  stack?: string;
  componentStack?: string;
}

/** Keep pathological stacks from flooding the native log file. */
const MESSAGE_LIMIT = 500;
const STACK_LIMIT = 2000;

/** Identical consecutive reports inside this window are suppressed. */
const DUPLICATE_SUPPRESS_MS = 5000;

let lastReport: { key: string; at: number } | null = null;

function clamp(value: string, limit: number): string {
  return value.length > limit ? `${value.slice(0, limit)}…` : value;
}

/**
 * Forward a visible copy of a frontend failure to the native log through the
 * `report_frontend_error` command boundary, so window errors, unhandled
 * rejections, and React render errors reach the `tauri dev` terminal and
 * `albada.log` instead of living only in the (often closed) WebView
 * console. Local-only: no telemetry, no external transmission.
 *
 * Recursion-safe by construction: it is fire-and-forget, every synchronous
 * and asynchronous failure is swallowed here (the fallback is a plain console
 * line, never a re-entry into this function), and repeated identical reports
 * are rate-limited so an error loop cannot spam the IPC channel. Outside
 * Tauri (tests, plain browser dev) it is a no-op.
 */
export function reportFrontendError(report: FrontendErrorReport): void {
  try {
    if (!isTauri()) return;

    const key = `${report.source}|${report.message}`;
    const now = Date.now();
    if (lastReport && lastReport.key === key && now - lastReport.at < DUPLICATE_SUPPRESS_MS) {
      return;
    }
    lastReport = { key, at: now };

    void invoke("report_frontend_error", { report }).catch(() => {
      // The report channel itself failed (typically: the native command is
      // not registered yet). Console-only fallback; must never call
      // reportFrontendError again.
      logger.warn("error-report:failed", key);
    });
  } catch {
    // Synchronous IPC failure (e.g. Tauri internals missing mid-shutdown).
    // Reporting must never become a new error source.
  }
}

/**
 * Build a `FrontendErrorReport` from a caught value of unknown shape (Error,
 * string, or serialized Rust `AppError`) and report it. Used by the window
 * listeners in main.tsx and by ErrorBoundary.
 */
export function reportUnknownError(
  source: FrontendErrorSource,
  error: unknown,
  // React 19 types `ErrorInfo.componentStack` as `string | null`.
  componentStack?: string | null,
): void {
  const base: FrontendErrorReport =
    error instanceof Error
      ? {
          source,
          message: clamp(error.message || error.name || "Unknown error", MESSAGE_LIMIT),
          name: error.name,
          stack: error.stack ? clamp(error.stack, STACK_LIMIT) : undefined,
        }
      : { source, message: clamp(describeError(error), MESSAGE_LIMIT) };
  if (componentStack) {
    base.componentStack = clamp(componentStack, STACK_LIMIT);
  }
  reportFrontendError(base);
}
