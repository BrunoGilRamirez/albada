import i18n from "i18next";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { ensureDefaultWorkspace, updateWorkspaceLocale } from "@/lib/api/workspaceApi";
import { getRuntimeLocale, initI18n } from "@/lib/i18n";
import type { Workspace } from "@/lib/types";

// Synthetic fixtures only. The shared Vitest setup has already initialized
// the i18next singleton with the four bundled catalogs, so every initI18n()
// call here exercises the real resolution + language-switch pipeline.

const createRootMock = vi.hoisted(() => vi.fn(() => ({ render: vi.fn() })));

vi.mock("react-dom/client", () => ({ default: { createRoot: createRootMock } }));

vi.mock("@/lib/api/workspaceApi", () => ({
  ensureDefaultWorkspace: vi.fn(),
  updateWorkspaceLocale: vi.fn(),
}));

vi.mock("@tauri-apps/plugin-os", () => ({
  locale: vi.fn(),
}));

import { locale } from "@tauri-apps/plugin-os";

function storedWorkspace(localeValue: string): Workspace {
  return { id: "ws-1", locale: localeValue } as unknown as Workspace;
}

beforeEach(() => {
  vi.mocked(ensureDefaultWorkspace).mockReset();
  vi.mocked(updateWorkspaceLocale).mockReset();
  vi.mocked(locale).mockReset();
  createRootMock.mockClear();
});

afterEach(async () => {
  await i18n.changeLanguage("en-US");
});

describe("initI18n startup resolution", () => {
  it("applies an explicit stored locale without consulting the OS plugin", async () => {
    vi.mocked(ensureDefaultWorkspace).mockResolvedValue(storedWorkspace("es-ES"));
    vi.mocked(locale).mockResolvedValue("de-DE");

    await expect(initI18n()).resolves.toBe("es-ES");

    expect(i18n.language).toBe("es-ES");
    expect(getRuntimeLocale()).toBe("es-ES");
    expect(document.documentElement.lang).toBe("es-ES");
    expect(locale).not.toHaveBeenCalled();
    expect(updateWorkspaceLocale).not.toHaveBeenCalled();
  });

  it("resolves the system preference through the Tauri OS plugin mapping", async () => {
    vi.mocked(ensureDefaultWorkspace).mockResolvedValue(storedWorkspace("system"));
    vi.mocked(locale).mockResolvedValue("de-AT");

    await expect(initI18n()).resolves.toBe("de-DE");

    expect(i18n.language).toBe("de-DE");
    expect(document.documentElement.lang).toBe("de-DE");
  });

  it("falls back to English when the OS query fails, keeping the stored system preference", async () => {
    vi.mocked(ensureDefaultWorkspace).mockResolvedValue(storedWorkspace("system"));
    vi.mocked(locale).mockRejectedValue(new Error("native os call unavailable"));

    await expect(initI18n()).resolves.toBe("en-US");

    expect(i18n.language).toBe("en-US");
    // Resolution failure must never rewrite the persisted preference.
    expect(updateWorkspaceLocale).not.toHaveBeenCalled();
  });

  it("settles in English when the workspace load fails, without querying the OS", async () => {
    // Settled contract: any startup-resolution failure resolves to en-US
    // while preserving the stored preference. A workspace load/ensure error
    // must NOT fall through to `system` OS resolution and must not persist.
    vi.mocked(ensureDefaultWorkspace).mockRejectedValue(new Error("ipc unavailable"));
    vi.mocked(locale).mockResolvedValue("fr-FR");

    await expect(initI18n()).resolves.toBe("en-US");

    expect(i18n.language).toBe("en-US");
    expect(document.documentElement.lang).toBe("en-US");
    expect(locale).not.toHaveBeenCalled();
    expect(updateWorkspaceLocale).not.toHaveBeenCalled();
  });

  it("treats a corrupted stored value like system", async () => {
    vi.mocked(ensureDefaultWorkspace).mockResolvedValue(storedWorkspace("pt-BR"));
    vi.mocked(locale).mockResolvedValue("en-US");

    await expect(initI18n()).resolves.toBe("en-US");

    expect(updateWorkspaceLocale).not.toHaveBeenCalled();
  });
});

describe("startup render gate", () => {
  it("never renders React before the persisted locale has resolved", async () => {
    document.body.innerHTML = '<div id="root"></div>';
    let releaseWorkspace!: (workspace: Workspace) => void;
    const gate = new Promise<Workspace>((resolve) => {
      releaseWorkspace = resolve;
    });
    vi.mocked(ensureDefaultWorkspace).mockReturnValue(gate);
    vi.mocked(locale).mockResolvedValue("fr-FR");

    await import("@/main");

    // The gate is still pending after full macrotask flushes: React must not
    // have mounted, so no fallback-English frame can ever exist.
    await new Promise((resolve) => window.setTimeout(resolve, 0));
    expect(createRootMock).not.toHaveBeenCalled();
    expect(i18n.language).toBe("en-US");

    releaseWorkspace(storedWorkspace("system"));
    await vi.waitFor(() => expect(createRootMock).toHaveBeenCalledTimes(1));

    expect(i18n.language).toBe("fr-FR");
    expect(document.documentElement.lang).toBe("fr-FR");
    // The delivery loop only starts after i18n resolved, so the first poll
    // can send the real runtime locale (isTauri is false in jsdom: no poll).
    expect(getRuntimeLocale()).toBe("fr-FR");
  });
});

describe("initI18n catalog-init failure fallback", () => {
  // Fresh module graph per test: a fake i18next singleton lets the bundled
  // catalog init fail deterministically. Only the project's transformed
  // source imports see the doMocked instance; the externalized react-i18next
  // keeps the real one, so nothing else in the graph is disturbed.

  interface FakeI18n {
    isInitialized: boolean;
    language: string;
    initCalls: string[];
    use(): FakeI18n;
    init(options: { lng?: string }): Promise<void>;
    changeLanguage(lng: string): Promise<void>;
    on(): void;
    /** Completes (or, when overwritten, fails) the pending English init attempt. */
    resolvePendingEnglish(): void;
  }

  function makeFakeI18n(): FakeI18n {
    let englishResolver: (() => void) | null = null;
    let englishReleaseRequested = false;
    const fake: FakeI18n = {
      isInitialized: false,
      language: "",
      initCalls: [],
      use() {
        return fake;
      },
      init(options: { lng?: string }) {
        const lng = String(options?.lng ?? "");
        fake.initCalls.push(lng);
        if (lng !== "en-US") {
          return Promise.reject(new Error(`catalog init failed for ${lng}`));
        }
        // English init completes only when the test releases it, so tests can
        // assert nothing settles before the English fallback actually ran.
        // A release requested before the attempt started applies immediately.
        return new Promise<void>((resolve) => {
          englishResolver = () => {
            fake.isInitialized = true;
            fake.language = "en-US";
            resolve();
          };
          if (englishReleaseRequested && englishResolver) englishResolver();
        });
      },
      async changeLanguage(lng: string) {
        fake.language = lng;
      },
      on() {},
      resolvePendingEnglish: () => {
        englishReleaseRequested = true;
        if (englishResolver) englishResolver();
      },
    };
    return fake;
  }

  afterEach(() => {
    vi.doUnmock("i18next");
    vi.doUnmock("@/lib/api/workspaceApi");
    vi.doUnmock("@tauri-apps/plugin-os");
    vi.resetModules();
  });

  async function importFresh(fake: FakeI18n) {
    vi.resetModules();
    const freshEnsure = vi.fn();
    const freshUpdate = vi.fn();
    const freshLocale = vi.fn();
    vi.doMock("i18next", () => ({ default: fake }));
    vi.doMock("@/lib/api/workspaceApi", () => ({
      ensureDefaultWorkspace: freshEnsure,
      updateWorkspaceLocale: freshUpdate,
    }));
    vi.doMock("@tauri-apps/plugin-os", () => ({ locale: freshLocale }));
    const i18nLib = await import("@/lib/i18n");
    return { i18nLib, freshEnsure, freshUpdate, freshLocale };
  }

  it("retries with English fallback, resolves, and never rewrites the stored preference", async () => {
    const fake = makeFakeI18n();
    const { i18nLib, freshEnsure, freshUpdate } = await importFresh(fake);
    freshEnsure.mockResolvedValue(storedWorkspace("es-ES"));
    fake.resolvePendingEnglish();

    await expect(i18nLib.initI18n()).resolves.toBe("en-US");

    // The resolved language's init failed first; the retry used en-US only.
    expect(fake.initCalls).toEqual(["es-ES", "en-US"]);
    expect(fake.isInitialized).toBe(true);
    expect(fake.language).toBe("en-US");
    expect(document.documentElement.lang).toBe("en-US");
    // Fallback is runtime-only: neither the explicit nor the system stored
    // preference is ever rewritten by resolution.
    expect(freshUpdate).not.toHaveBeenCalled();
  });

  it("settles to en-US even when the English fallback attempt also fails (never blocks startup)", async () => {
    const fake = makeFakeI18n();
    // Total catalog failure: every init (including the English retry) rejects.
    fake.init = (options: { lng?: string }) => {
      fake.initCalls.push(String(options?.lng ?? ""));
      return Promise.reject(new Error("catalog unavailable"));
    };
    const { i18nLib, freshEnsure, freshUpdate } = await importFresh(fake);
    freshEnsure.mockResolvedValue(storedWorkspace("system"));

    // Unmocked freshLocale resolves to undefined → system maps to en-US, so
    // the first attempt and the retry both target en-US.
    await expect(i18nLib.initI18n()).resolves.toBe("en-US");
    expect(fake.initCalls).toEqual(["en-US", "en-US"]);
    expect(fake.isInitialized).toBe(false);
    expect(freshUpdate).not.toHaveBeenCalled();
  });

  it("renders only after the English fallback init completes when the catalog init for the stored language fails", async () => {
    document.body.innerHTML = '<div id="root"></div>';
    const fake = makeFakeI18n();
    createRootMock.mockClear();

    vi.resetModules();
    let releaseWorkspace!: (workspace: Workspace) => void;
    const gate = new Promise<Workspace>((resolve) => {
      releaseWorkspace = resolve;
    });
    const freshEnsure = vi.fn().mockReturnValue(gate);
    const freshUpdate = vi.fn();
    vi.doMock("i18next", () => ({ default: fake }));
    vi.doMock("@/lib/api/workspaceApi", () => ({
      ensureDefaultWorkspace: freshEnsure,
      updateWorkspaceLocale: freshUpdate,
    }));
    vi.doMock("@tauri-apps/plugin-os", () => ({ locale: vi.fn().mockResolvedValue("de-DE") }));
    vi.doMock("react-dom/client", () => ({ default: { createRoot: createRootMock } }));

    await import("@/main");

    await new Promise((resolve) => window.setTimeout(resolve, 0));
    expect(createRootMock).not.toHaveBeenCalled();

    // Workspace resolves to explicit es-ES; its catalog init fails, so the
    // English retry begins. Until that retry completes, no render may occur.
    releaseWorkspace(storedWorkspace("es-ES"));
    await vi.waitFor(() => expect(fake.initCalls).toEqual(["es-ES", "en-US"]));
    expect(createRootMock).not.toHaveBeenCalled();
    expect(fake.isInitialized).toBe(false);

    // The app still reaches render after the English fallback attempt.
    fake.resolvePendingEnglish();
    await vi.waitFor(() => expect(createRootMock).toHaveBeenCalledTimes(1));
    expect(fake.language).toBe("en-US");
    expect(document.documentElement.lang).toBe("en-US");
    expect(freshUpdate).not.toHaveBeenCalled();
  });
});
