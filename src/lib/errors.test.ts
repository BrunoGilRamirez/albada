import i18n from "i18next";
import { afterEach, describe, expect, it } from "vitest";
import { CommandInvocationError } from "@/lib/tauri";
import { isRetryable, isStalePreviewError, isUserError, toDisplayMessage } from "@/lib/errors";

// Synthetic fixtures only. The singleton is initialized in English by the
// shared Vitest setup with the same bundled catalogs the app uses.

function serializedAppError(code: string, message: string): CommandInvocationError {
  return new CommandInvocationError("create_task", JSON.stringify({ code, message }));
}

afterEach(async () => {
  await i18n.changeLanguage("en-US");
});

describe("toDisplayMessage with stable Rust error codes", () => {
  it("maps every known code to its localized English text", () => {
    expect(toDisplayMessage(serializedAppError("VALIDATION", "native detail"))).toBe(
      "Please check the information and try again.",
    );
    expect(toDisplayMessage(serializedAppError("NOT_FOUND", "native detail"))).toBe(
      "The requested item could not be found.",
    );
    expect(toDisplayMessage(serializedAppError("DATABASE", "native detail"))).toBe(
      "Your local data could not be saved. Please try again.",
    );
    expect(toDisplayMessage(serializedAppError("IO", "native detail"))).toBe(
      "The local file operation could not be completed.",
    );
    expect(toDisplayMessage(serializedAppError("UNKNOWN", "native detail"))).toBe(
      "Something unexpected happened. Please try again.",
    );
  });

  it("localizes the same codes after a runtime language switch", async () => {
    await i18n.changeLanguage("es-ES");
    expect(toDisplayMessage(serializedAppError("VALIDATION", "native detail"))).toBe(
      "Revisa la información e inténtalo de nuevo.",
    );
    await i18n.changeLanguage("de-DE");
    expect(toDisplayMessage(serializedAppError("NOT_FOUND", "native detail"))).toBe(
      "Das angeforderte Element wurde nicht gefunden.",
    );
    await i18n.changeLanguage("fr-FR");
    expect(toDisplayMessage(serializedAppError("DATABASE", "native detail"))).toBe(
      "Vos données locales n'ont pas pu être enregistrées. Veuillez réessayer.",
    );
  });

  it("handles a plain {code,message} object (non-serialized shape)", () => {
    expect(toDisplayMessage({ code: "IO", message: "disk said no" })).toBe(
      "The local file operation could not be completed.",
    );
  });

  it("uses the localized generic fallback for unknown code strings, never the raw text", () => {
    const error = serializedAppError("RATE_LIMITED", "Too many requests /home/private/path");
    expect(toDisplayMessage(error)).toBe("Something unexpected happened. Please try again.");
  });

  it("preserves the safe raw message for errors without any code", () => {
    expect(toDisplayMessage(new Error("Plain diagnostic, nothing else"))).toBe(
      "Plain diagnostic, nothing else",
    );
    expect(toDisplayMessage({ message: "A safe legacy message" })).toBe("A safe legacy message");
  });

  it("falls back to the generic localized text for unsafe code-less payloads", () => {
    expect(toDisplayMessage({ code: 42, message: "<html>not safe" })).toBe(
      "Something unexpected happened. Please try again.",
    );
    expect(toDisplayMessage(new Error("<div>markup leak"))).toBe(
      "Something unexpected happened. Please try again.",
    );
  });
});

describe("classification helpers keep their semantics", () => {
  it("still classifies known codes only", () => {
    expect(isUserError(serializedAppError("VALIDATION", "x"))).toBe(true);
    expect(isUserError(serializedAppError("NOT_FOUND", "x"))).toBe(true);
    expect(isUserError(serializedAppError("DATABASE", "x"))).toBe(false);
    expect(isRetryable(serializedAppError("DATABASE", "x"))).toBe(true);
    expect(isRetryable(serializedAppError("RATE_LIMITED", "x"))).toBe(false);
    expect(
      isStalePreviewError({
        code: "VALIDATION",
        message: "Selection changed since it was previewed",
      }),
    ).toBe(true);
  });
});
