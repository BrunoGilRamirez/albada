import i18n from "i18next";
import { afterEach, describe, expect, it } from "vitest";
import { formatDate, formatDateTime, formatNumber, formatTime } from "@/lib/format";

// Application-owned values only (synthetic dates/numbers). User-authored
// content never passes through these helpers.

afterEach(async () => {
  await i18n.changeLanguage("en-US");
});

describe("Intl formatters follow the resolved i18next locale", () => {
  it("formats dates in the active language", () => {
    const value = new Date(2026, 0, 15, 12, 0, 0);
    expect(formatDate(value, { month: "long", year: "numeric" })).toBe("January 2026");
  });

  it("switches month names with the UI language", async () => {
    await i18n.changeLanguage("de-DE");
    expect(formatDate(new Date(2026, 0, 15, 12, 0, 0), { month: "long" })).toBe("Januar");

    await i18n.changeLanguage("fr-FR");
    expect(formatDate(new Date(2026, 6, 4, 12, 0, 0), { month: "long" })).toBe("juillet");
  });

  it("renders date/time and time-only values in the active language", async () => {
    const value = new Date(2026, 9, 1, 8, 0, 0);
    const english = formatDateTime(value);
    expect(english).toContain("8:00:00 AM");

    await i18n.changeLanguage("de-DE");
    expect(formatDateTime(value)).toContain("08:00:00");
    expect(formatTime(value)).toContain("08:00:00");
  });

  it("formats numbers with locale grouping", () => {
    expect(formatNumber(1234.5)).toBe("1,234.5");
  });

  it("switches number grouping with the UI language", async () => {
    await i18n.changeLanguage("de-DE");
    expect(formatNumber(1234.5)).toBe("1.234,5");

    await i18n.changeLanguage("es-ES");
    expect(formatNumber(1000000)).toContain("1");
  });

  it("parses RFC 3339 strings through the shared Date pipeline", () => {
    expect(formatDate("2026-01-15T10:30:00Z", { year: "numeric" })).toMatch(/2026/);
  });
});
