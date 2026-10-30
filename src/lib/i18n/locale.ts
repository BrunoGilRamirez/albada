/**
 * Locale contract (L3, 8.6.7): a workspace persists exactly `system` or one
 * of the four supported tags; an explicit tag overrides the OS locale. The
 * Tauri OS plugin (not browser/WebView settings) is authoritative for system
 * resolution, and every unresolvable value falls back to en-US without ever
 * blocking startup. Stored preferences are never rewritten by resolution.
 */

export const SUPPORTED_LOCALES = ["en-US", "es-ES", "de-DE", "fr-FR"] as const;

export type AppLocale = (typeof SUPPORTED_LOCALES)[number];

export type WorkspaceLocalePreference = "system" | AppLocale;

export const FALLBACK_LOCALE: AppLocale = "en-US";

/**
 * Normalization of well-formed BCP-47 tags by primary language subtag,
 * case-insensitive: en-* → en-US, es-* → es-ES, de-* → de-DE, fr-* → fr-FR.
 * The tag is structurally validated first (`Intl.getCanonicalLocales`), so
 * malformed subtags (`en--US`, `es-`, …) reach the English fallback instead
 * of being guessed from their prefix. Null, unsupported languages, and
 * unparseable values also resolve to the fallback. Underscore separators are
 * tolerated because some OS APIs emit `en_US` style values.
 */
const PRIMARY_LANGUAGE_TO_LOCALE: Record<string, AppLocale> = {
  en: "en-US",
  es: "es-ES",
  de: "de-DE",
  fr: "fr-FR",
};

export function mapSystemLocale(raw: string | null | undefined): AppLocale {
  if (typeof raw !== "string") return FALLBACK_LOCALE;
  // Documented underscore compatibility (some OS APIs emit `en_US` style):
  // normalize before validation, not after.
  const candidate = raw.trim().replace(/_/g, "-");
  if (candidate.length === 0) return FALLBACK_LOCALE;
  let primary: string;
  try {
    // Validate the full tag structure before mapping: malformed subtags such
    // as `en--US` or `es-` must reach the English fallback and must never be
    // guessed from their prefix. `Intl.getCanonicalLocales` accepts only
    // well-formed BCP-47 tags and canonicalizes case.
    const canonical = Intl.getCanonicalLocales(candidate)[0];
    primary = canonical.split("-")[0].toLowerCase();
  } catch {
    return FALLBACK_LOCALE;
  }
  return PRIMARY_LANGUAGE_TO_LOCALE[primary] ?? FALLBACK_LOCALE;
}

/**
 * Defensive read of the persisted `workspaces.locale` value. Anything the
 * Rust validation would not write (including null) behaves like `system`:
 * resolution continues with the OS locale and English fallback.
 */
export function parseWorkspaceLocalePreference(raw: unknown): WorkspaceLocalePreference {
  if (raw === "system") return "system";
  if (typeof raw === "string" && (SUPPORTED_LOCALES as readonly string[]).includes(raw)) {
    return raw as AppLocale;
  }
  return "system";
}

/** Keep `<html lang>` truthful at runtime; the static splash stays English. */
export function applyDocumentLanguage(language: string): void {
  if (typeof document === "undefined") return;
  document.documentElement.lang = language;
}
