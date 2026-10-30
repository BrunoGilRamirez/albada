import i18next from "i18next";
import { logger } from "./logger";

export type ErrorCode = "VALIDATION" | "NOT_FOUND" | "DATABASE" | "IO" | "UNKNOWN";

export interface AppError {
  readonly code: ErrorCode;
  readonly message: string;
}

/**
 * English display text, kept as the pre-bootstrap fallback and mirrored by
 * the `errors.*` keys in every bundled catalog (the catalog-parity test
 * guards that mirror). Once i18next is initialized the catalogs below win.
 */
const DISPLAY_MESSAGES: Record<ErrorCode, string> = {
  VALIDATION: "Please check the information and try again.",
  NOT_FOUND: "The requested item could not be found.",
  DATABASE: "Your local data could not be saved. Please try again.",
  IO: "The local file operation could not be completed.",
  UNKNOWN: "Something unexpected happened. Please try again.",
};

/**
 * Localize a stable Rust error code through i18next. Known codes map to
 * `errors.<CODE>`; an unknown code string uses the localized generic
 * fallback. Before i18next finishes bootstrapping (or outside the i18n
 * runtime) the English constant is used, so errors are never rendered as
 * raw keys during startup.
 */
function translateErrorCode(code: string): string {
  const fallback = isErrorCode(code) ? code : "UNKNOWN";
  if (i18next.isInitialized) {
    return i18next.t(`errors.${fallback}`);
  }
  return DISPLAY_MESSAGES[fallback];
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null;
}

function isErrorCode(value: unknown): value is ErrorCode {
  return (
    value === "VALIDATION" ||
    value === "NOT_FOUND" ||
    value === "DATABASE" ||
    value === "IO" ||
    value === "UNKNOWN"
  );
}

/**
 * Walk the thrown error (and its `cause` chain) looking for any serialized
 * `code` string. Tauri serializes Rust `AppError` values as JSON objects or
 * JSON strings, so both shapes are handled here.
 */
function findErrorCodeValue(error: unknown): string | undefined {
  const visited = new Set<object>();
  let current: unknown = error;

  while (current !== undefined && current !== null) {
    if (typeof current === "string") {
      try {
        current = JSON.parse(current) as unknown;
        continue;
      } catch {
        return undefined;
      }
    }

    if (!isRecord(current)) {
      return undefined;
    }

    if (visited.has(current)) {
      return undefined;
    }
    visited.add(current);

    if (typeof current.code === "string" && current.code.length > 0) {
      return current.code;
    }

    // Tauri v2 may wrap the error in `{ error: ... }` or `{ message: ... }`.
    if (current.error !== undefined) {
      current = current.error;
      continue;
    }

    current = current.cause;
  }

  return undefined;
}

function getErrorCode(error: unknown): ErrorCode | undefined {
  const code = findErrorCodeValue(error);
  return code !== undefined && isErrorCode(code) ? code : undefined;
}

/**
 * Best-effort extraction of a human-readable message from the error chain.
 * Falls back to a generic message; logs the raw error so the console shows the
 * actual failure for debugging.
 */
function getRawMessage(error: unknown): string | undefined {
  const visited = new Set<object>();
  let current: unknown = error;

  while (current !== undefined && current !== null) {
    if (typeof current === "string") {
      try {
        const parsed = JSON.parse(current) as unknown;
        if (isRecord(parsed) && typeof parsed.message === "string") {
          return parsed.message;
        }
      } catch {
        return current;
      }
    }

    if (!isRecord(current)) {
      return undefined;
    }
    if (visited.has(current)) {
      return undefined;
    }
    visited.add(current);

    // `invoke` wraps native failures in CommandInvocationError. Its own
    // message is only a command label; inspect the cause first so callers can
    // display the actual serialized Rust error.
    if (
      typeof current.message === "string" &&
      !current.message.startsWith("Failed to invoke Tauri command:")
    ) {
      return current.message;
    }
    if (current.error !== undefined) {
      current = current.error;
      continue;
    }
    if (current.cause !== undefined) {
      current = current.cause;
      continue;
    }
    current = current.cause;
  }

  return undefined;
}

export function isUserError(error: unknown): boolean {
  const code = getErrorCode(error);
  return code === "VALIDATION" || code === "NOT_FOUND";
}

const STALE_PREVIEW_MARKERS = [
  "changed since it was previewed",
  "preview again",
  "preview confirmation is required",
];

/**
 * True when the error indicates the confirmed preview no longer matches the
 * current records (the selection changed, or a preview confirmation is missing
 * for a format that requires one). The UI should prompt the user to re-preview
 * before retrying the export.
 */
export function isStalePreviewError(error: unknown): boolean {
  const raw = getRawMessage(error);
  if (!raw) return false;
  const lower = raw.toLowerCase();
  return STALE_PREVIEW_MARKERS.some((marker) => lower.includes(marker));
}

export function isRetryable(error: unknown): boolean {
  const code = getErrorCode(error);
  return code === "DATABASE" || code === "IO" || code === "UNKNOWN";
}

export function toDisplayMessage(error: unknown): string {
  const code = findErrorCodeValue(error);
  if (code !== undefined) {
    // Known stable codes map to localized `errors.<CODE>` keys; any other
    // code string gets the localized generic fallback and never leaks the
    // native raw text.
    return translateErrorCode(code);
  }
  // Unknown errors (no code at all): log the full detail and show the raw
  // message when safe, so the user can report it and we can debug from the
  // console. This pre-i18n "safe unknown error" behavior is preserved.
  const raw = getRawMessage(error);
  if (raw && raw.length <= 300 && /^[A-Za-z0-9 ,.()'"\-/:_]/.test(raw)) {
    logger.warn("error:unclassified", error);
    return raw;
  }
  logger.warn("error:unclassified", error);
  return translateErrorCode("UNKNOWN");
}
