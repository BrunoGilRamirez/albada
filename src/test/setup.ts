import "@testing-library/jest-dom/vitest";
import i18n from "i18next";
import { initReactI18next } from "react-i18next";
import deDE from "@/lib/i18n/locales/de-DE.json";
import enUS from "@/lib/i18n/locales/en-US.json";
import esES from "@/lib/i18n/locales/es-ES.json";
import frFR from "@/lib/i18n/locales/fr-FR.json";

// Existing text assertions in the suite are written in English, so the
// shared i18next singleton starts in en-US for every test file (mirroring
// the app's documented fallback). All four bundled catalogs are preloaded so
// i18n-focused tests can exercise real runtime switching. Only i18next and
// the static JSON are touched here: importing `@/lib/i18n` at setup time
// would eagerly load the Tauri api/plugin modules and interfere with the
// per-file `vi.mock` interception those tests rely on.
await i18n.use(initReactI18next).init({
  lng: "en-US",
  fallbackLng: "en-US",
  supportedLngs: ["en-US", "es-ES", "de-DE", "fr-FR"],
  resources: {
    "en-US": { translation: enUS },
    "es-ES": { translation: esES },
    "de-DE": { translation: deDE },
    "fr-FR": { translation: frFR },
  },
  interpolation: { escapeValue: false },
  react: { useSuspense: false },
});
