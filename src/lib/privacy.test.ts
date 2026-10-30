import { describe, expect, it } from "vitest";
import {
  buildPrivacySafeNotification,
  canIncludeInGenericExport,
  isExportEligible,
  isJournalPrivate,
  isTherapyPrivate,
} from "@/lib/privacy";

describe("privacy helpers", () => {
  it("identifies therapy-private scopes", () => {
    expect(isTherapyPrivate("therapy_private")).toBe(true);
    expect(isTherapyPrivate("private")).toBe(false);
  });

  it("identifies private journal scopes", () => {
    expect(isJournalPrivate("private")).toBe(true);
    expect(isJournalPrivate("therapy_private")).toBe(true);
    expect(isJournalPrivate("personal")).toBe(false);
  });

  it("identifies export-eligible scopes", () => {
    expect(isExportEligible("export_eligible")).toBe(true);
    expect(isExportEligible("private")).toBe(false);
  });

  it("never includes sensitive content in notifications", () => {
    const notification = buildPrivacySafeNotification("Journal reminder", "private journal text");

    expect(notification).toEqual({
      title: "Journal reminder",
      body: "Open Albada to view.",
    });
    expect(JSON.stringify(notification)).not.toContain("private journal text");
  });

  it("excludes therapy-private and private scopes from generic exports", () => {
    expect(canIncludeInGenericExport("export_eligible")).toBe(true);
    expect(canIncludeInGenericExport("personal")).toBe(true);
    expect(canIncludeInGenericExport("private")).toBe(false);
    expect(canIncludeInGenericExport("therapy_private")).toBe(false);
  });
});
