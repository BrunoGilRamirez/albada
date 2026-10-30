import { describe, expect, it } from "vitest";
import {
  FALLBACK_LOCALE,
  SUPPORTED_LOCALES,
  applyDocumentLanguage,
  mapSystemLocale,
  parseWorkspaceLocalePreference,
} from "@/lib/i18n/locale";

// Synthetic values only. The Tauri OS plugin is the production source of the
// raw tags below (Windows emits BCP-47 such as "es-ES").

describe("mapSystemLocale", () => {
  it("maps exact supported tags to themselves", () => {
    for (const locale of SUPPORTED_LOCALES) {
      expect(mapSystemLocale(locale)).toBe(locale);
    }
  });

  it("normalizes regional BCP-47 variants by primary language, case-insensitively", () => {
    expect(mapSystemLocale("en-GB")).toBe("en-US");
    expect(mapSystemLocale("en")).toBe("en-US");
    expect(mapSystemLocale("ES-mx")).toBe("es-ES");
    expect(mapSystemLocale("es-419")).toBe("es-ES");
    expect(mapSystemLocale("de-AT")).toBe("de-DE");
    expect(mapSystemLocale("FR-CA")).toBe("fr-FR");
    expect(mapSystemLocale("fr")).toBe("fr-FR");
    expect(mapSystemLocale("  en-US  ")).toBe("en-US");
  });

  it("tolerates underscore-separated OS values", () => {
    expect(mapSystemLocale("de_DE")).toBe("de-DE");
    expect(mapSystemLocale("fr_FR")).toBe("fr-FR");
  });

  it("validates the full tag before mapping, so malformed subtags never inherit their prefix's language", () => {
    // Structural regressions: the old first-subtag parser mapped these to
    // es-ES/de-DE/fr-FR; the contract says they must reach the English
    // fallback instead.
    expect(mapSystemLocale("es-")).toBe("en-US");
    expect(mapSystemLocale("de-")).toBe("en-US");
    expect(mapSystemLocale("fr-")).toBe("en-US");
    expect(mapSystemLocale("en--US")).toBe("en-US");
    expect(mapSystemLocale("en-US-")).toBe("en-US");
    expect(mapSystemLocale("es_US-")).toBe("en-US");
    expect(mapSystemLocale("e-US")).toBe("en-US");
    expect(mapSystemLocale("en..US")).toBe("en-US");
    // Underscore compatibility is normalized before validation, not after.
    expect(mapSystemLocale("de_AT")).toBe("de-DE");
    expect(mapSystemLocale("ES_mx")).toBe("es-ES");
  });

  it("falls back to English for null, empty, malformed, or unsupported values", () => {
    expect(FALLBACK_LOCALE).toBe("en-US");
    expect(mapSystemLocale(null)).toBe("en-US");
    expect(mapSystemLocale(undefined)).toBe("en-US");
    expect(mapSystemLocale("")).toBe("en-US");
    expect(mapSystemLocale("   ")).toBe("en-US");
    expect(mapSystemLocale("pt-BR")).toBe("en-US");
    expect(mapSystemLocale("!!")).toBe("en-US");
    expect(mapSystemLocale("日本語")).toBe("en-US");
    expect(mapSystemLocale("-en-US")).toBe("en-US");
  });
});

describe("parseWorkspaceLocalePreference", () => {
  it("accepts exactly the persisted contract values", () => {
    expect(parseWorkspaceLocalePreference("system")).toBe("system");
    for (const locale of SUPPORTED_LOCALES) {
      expect(parseWorkspaceLocalePreference(locale)).toBe(locale);
    }
  });

  it("treats anything else (legacy noise, null, wrong case) as system", () => {
    expect(parseWorkspaceLocalePreference("EN-us")).toBe("system");
    expect(parseWorkspaceLocalePreference("pt-BR")).toBe("system");
    expect(parseWorkspaceLocalePreference("")).toBe("system");
    expect(parseWorkspaceLocalePreference(null)).toBe("system");
    expect(parseWorkspaceLocalePreference(undefined)).toBe("system");
    expect(parseWorkspaceLocalePreference(42)).toBe("system");
  });
});

describe("applyDocumentLanguage", () => {
  it("updates <html lang> at runtime", () => {
    applyDocumentLanguage("de-DE");
    expect(document.documentElement.lang).toBe("de-DE");
    applyDocumentLanguage("en-US");
    expect(document.documentElement.lang).toBe("en-US");
  });
});
