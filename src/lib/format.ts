import i18n from "i18next";

/**
 * Tiny shared Intl formatters for application-owned dates and numbers.
 * The locale always follows the resolved i18next UI language (English before
 * bootstrap completes), never `undefined`/browser preferences, so output is
 * stable and switches with the language. User-authored content (journal,
 * therapy, Markdown, imports, export templates) must never be passed through
 * these helpers — they format only application-owned values.
 */

function formatterLocale(): string {
  return i18n.isInitialized ? i18n.language : "en-US";
}

function toDate(value: string | number | Date): Date {
  return value instanceof Date ? value : new Date(value);
}

export function formatDate(
  value: string | number | Date,
  options: Intl.DateTimeFormatOptions,
): string {
  return toDate(value).toLocaleDateString(formatterLocale(), options);
}

/** Locale-default full date+time rendering (replaces bare `toLocaleString()`). */
export function formatDateTime(value: string | number | Date): string {
  return toDate(value).toLocaleString(formatterLocale());
}

/** Locale-default time-only rendering (replaces bare `toLocaleTimeString()`). */
export function formatTime(value: string | number | Date): string {
  return toDate(value).toLocaleTimeString(formatterLocale());
}

export function formatNumber(value: number, options?: Intl.NumberFormatOptions): string {
  return new Intl.NumberFormat(formatterLocale(), options).format(value);
}
