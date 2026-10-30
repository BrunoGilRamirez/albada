import { describe, expect, it } from "vitest";
import { resolveEffectiveTheme } from "./theme";

describe("resolveEffectiveTheme", () => {
  it("returns dark when mode is dark", () => {
    expect(resolveEffectiveTheme("dark", false)).toBe("dark");
    expect(resolveEffectiveTheme("dark", true)).toBe("dark");
  });

  it("returns light when mode is light", () => {
    expect(resolveEffectiveTheme("light", false)).toBe("light");
    expect(resolveEffectiveTheme("light", true)).toBe("light");
  });

  it("returns OS preference when mode is system and prefersDark is false", () => {
    expect(resolveEffectiveTheme("system", false)).toBe("light");
  });

  it("returns dark when mode is system and prefersDark is true", () => {
    expect(resolveEffectiveTheme("system", true)).toBe("dark");
  });
});
