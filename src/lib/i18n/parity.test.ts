import i18n from "i18next";
import { afterAll, describe, expect, it } from "vitest";
import { SUPPORTED_LOCALES } from "@/lib/i18n/locale";
import { catalogs, resources } from "@/lib/i18n/resources";
import enUS from "@/lib/i18n/locales/en-US.json";

// Catalog parity: every supported locale must carry exactly the same keys,
// the same interpolation placeholders per key, and complete plural variants,
// so English fallback can never conceal a missing translation.

type Nested = { [key: string]: string | Nested };

function flattenKeys(value: unknown, prefix = ""): string[] {
  if (typeof value === "string") return [prefix];
  const out: string[] = [];
  for (const [key, child] of Object.entries(value as Nested)) {
    out.push(...flattenKeys(child, prefix.length > 0 ? `${prefix}.${key}` : key));
  }
  return out.sort();
}

function placeholders(value: unknown): string[] {
  if (typeof value === "string") {
    return [...value.matchAll(/\{\{\s*([\w.]+)\s*(?:,[^}]*)?\}\}/g)].map((match) => match[1]);
  }
  return Object.values(value as Nested).flatMap((child) => placeholders(child));
}

function leafValues(value: unknown): string[] {
  if (typeof value === "string") return [value];
  return Object.values(value as Nested).flatMap((child) => leafValues(child));
}

function byKey(value: unknown, prefix = ""): Map<string, string> {
  const map = new Map<string, string>();
  if (typeof value === "string") {
    map.set(prefix, value);
    return map;
  }
  for (const [key, child] of Object.entries(value as Nested)) {
    for (const [path, leaf] of byKey(child, prefix.length > 0 ? `${prefix}.${key}` : key)) {
      map.set(path, leaf);
    }
  }
  return map;
}

describe("catalog parity", () => {
  const reference = enUS as unknown as Nested;

  it("bundles exactly the four supported locales with a translation namespace", () => {
    expect(Object.keys(resources).sort()).toEqual([...SUPPORTED_LOCALES].sort());
    for (const locale of SUPPORTED_LOCALES) {
      expect(Object.keys(resources[locale] ?? {})).toEqual(["translation"]);
    }
  });

  it.each([...SUPPORTED_LOCALES])("%s has the same key set as en-US", (locale) => {
    expect(flattenKeys(catalogs[locale])).toEqual(flattenKeys(reference));
  });

  it.each([...SUPPORTED_LOCALES])(
    "%s matches en-US interpolation placeholders per key",
    (locale) => {
      const expected = byKey(reference);
      const actual = byKey(catalogs[locale]);
      for (const [key, englishValue] of expected) {
        expect(actual.has(key), `missing key: ${key}`).toBe(true);
        expect(placeholders(actual.get(key)), `placeholders for ${key}`).toEqual(
          placeholders(englishValue),
        );
      }
    },
  );

  it.each([...SUPPORTED_LOCALES])("%s carries complete _one/_other plural variants", (locale) => {
    const keys = flattenKeys(catalogs[locale]);
    for (const key of keys) {
      if (key.endsWith("_one")) {
        expect(keys, `missing plural sibling for ${key}`).toContain(`${key.slice(0, -4)}_other`);
      }
      if (key.endsWith("_other")) {
        expect(keys, `missing plural sibling for ${key}`).toContain(`${key.slice(0, -6)}_one`);
      }
    }
  });

  it.each([...SUPPORTED_LOCALES])("%s has no empty translation values", (locale) => {
    for (const value of leafValues(catalogs[locale])) {
      expect(value.trim().length).toBeGreaterThan(0);
    }
  });
});

describe("i18next runtime behavior with the bundled catalogs", () => {
  afterAll(async () => {
    await i18n.changeLanguage("en-US");
  });

  it("resolves plurals and interpolation per locale through the singleton", async () => {
    await i18n.changeLanguage("en-US");
    expect(i18n.t("common.taskCount", { count: 1 })).toBe("1 task");
    expect(i18n.t("common.taskCount", { count: 3 })).toBe("3 tasks");

    await i18n.changeLanguage("es-ES");
    expect(i18n.t("common.taskCount", { count: 2 })).toBe("2 tareas");

    await i18n.changeLanguage("fr-FR");
    // CLDR French treats 0 as the `one` form.
    expect(i18n.t("common.taskCount", { count: 0 })).toBe("0 tâche");
    expect(i18n.t("common.taskCount", { count: 4 })).toBe("4 tâches");

    await i18n.changeLanguage("de-DE");
    expect(i18n.t("settings.language.saved", { language: "Deutsch (Deutschland)" })).toBe(
      "Spracheinstellung gespeichert. Die App verwendet jetzt Deutsch (Deutschland).",
    );
  });
});
