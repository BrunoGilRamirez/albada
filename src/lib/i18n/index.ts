import { locale as readOsLocale } from "@tauri-apps/plugin-os";
import i18n from "i18next";
import { initReactI18next } from "react-i18next";
import { ensureDefaultWorkspace } from "../api/workspaceApi";
import { logger } from "../logger";
import {
  FALLBACK_LOCALE,
  SUPPORTED_LOCALES,
  applyDocumentLanguage,
  mapSystemLocale,
  parseWorkspaceLocalePreference,
  type AppLocale,
  type WorkspaceLocalePreference,
} from "./locale";
import { resources } from "./resources";

/**
 * The Tauri OS plugin is the only system-locale source (browser/WebView
 * preferences are deliberately ignored). Outside Tauri — plain browser dev,
 * tests — the plugin call fails; that is treated as "unknown system" and
 * resolves to English without blocking anything.
 */
export async function readSystemLocale(): Promise<string | null> {
  try {
    return await readOsLocale();
  } catch {
    return null;
  }
}

/** Resolve a persisted preference to a concrete catalog locale. Never throws. */
export async function resolveLocaleForPreference(
  preference: WorkspaceLocalePreference,
): Promise<AppLocale> {
  if (preference === "system") {
    return mapSystemLocale(await readSystemLocale());
  }
  return preference;
}

/**
 * Read the default workspace's persisted locale preference. Returns `null`
 * when the workspace load/ensure fails: per the settled contract that is a
 * startup-resolution failure and must select the English fallback directly
 * (no OS query, nothing persisted). A successful load never rewrites the
 * stored preference during resolution.
 */
export async function readStoredPreference(): Promise<WorkspaceLocalePreference | null> {
  try {
    const workspace = await ensureDefaultWorkspace();
    return parseWorkspaceLocalePreference(workspace.locale);
  } catch {
    return null;
  }
}

/**
 * Initialize the i18next singleton with the statically bundled catalogs, or
 * switch its language when it is already initialized. Registers the runtime
 * `<html lang>` sync once.
 */
let documentSyncRegistered = false;

function registerDocumentLanguageSync(): void {
  if (documentSyncRegistered) return;
  documentSyncRegistered = true;
  i18n.on("languageChanged", (lng) => applyDocumentLanguage(lng));
}

export async function ensureI18nReady(language: AppLocale): Promise<void> {
  if (!i18n.isInitialized) {
    await i18n.use(initReactI18next).init({
      lng: language,
      fallbackLng: FALLBACK_LOCALE,
      supportedLngs: [...SUPPORTED_LOCALES],
      resources,
      interpolation: {
        // React escapes rendered values; i18next must not double-escape.
        escapeValue: false,
      },
      react: { useSuspense: false },
    });
  } else if (i18n.language !== language) {
    await i18n.changeLanguage(language);
  }
  registerDocumentLanguageSync();
  applyDocumentLanguage(i18n.language);
}

/**
 * Startup sequence awaited before the first React commit: ensure the default
 * workspace, resolve its persisted preference (OS locale when `system`), then
 * initialize i18next so the very first render is already in the right
 * language. All failure paths land on the English fallback and this never
 * rejects: a failed workspace load/ensure selects the en-US fallback
 * directly — the OS locale is not queried on that path — and if initializing
 * the resolved language fails, startup retries with the documented en-US
 * fallback (the stored preference is never rewritten by resolution) and
 * still settles so the caller can render. Any English fallback attempt
 * happens before this promise settles, so no fallback render can precede
 * resolution. Normal `system` preferences from a successfully loaded
 * workspace still resolve through the OS plugin mapping.
 */
export async function initI18n(): Promise<AppLocale> {
  const preference = await readStoredPreference();
  // Startup-resolution failure: pin to English immediately, without
  // consulting the OS locale or persisting anything.
  const resolved =
    preference === null ? FALLBACK_LOCALE : await resolveLocaleForPreference(preference);
  try {
    await ensureI18nReady(resolved);
    return resolved;
  } catch (initError) {
    // i18next could not initialize for the resolved language. Establish the
    // English fallback before startup continues; a total init failure must
    // still settle (never block render) because every pre-init consumer
    // falls back to the English constants in `src/lib/errors.ts`.
    logger.warn("i18n:init-failed", initError);
    try {
      await ensureI18nReady(FALLBACK_LOCALE);
    } catch (fallbackError) {
      logger.warn("i18n:english-fallback-failed", fallbackError);
    }
    return FALLBACK_LOCALE;
  }
}

/**
 * Apply a (freshly persisted) preference to the live UI immediately: used by
 * the Settings selector after `update_workspace_locale` succeeds. Returns the
 * concrete locale that took effect.
 */
export async function applyWorkspacePreference(
  preference: WorkspaceLocalePreference,
): Promise<AppLocale> {
  const resolved = await resolveLocaleForPreference(preference);
  await ensureI18nReady(resolved);
  return resolved;
}

/**
 * The resolved UI language to hand to native commands that localize their
 * own application-authored text (notifications). `null` before bootstrap
 * completed, which Rust treats as "use your own resolution rules".
 */
export function getRuntimeLocale(): string | null {
  return i18n.isInitialized ? i18n.language : null;
}

export * from "./locale";
export type { Translation } from "./resources";
