import i18n from "i18next";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import LanguageSelector from "@/components/settings/LanguageSelector";
import { ensureDefaultWorkspace, updateWorkspaceLocale } from "@/lib/api/workspaceApi";
import type { Workspace } from "@/lib/types";

// Synthetic fixtures only. The Tauri api/plugin modules are mocked; language
// switching itself runs against the real bundled catalogs initialized by the
// shared Vitest setup.

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

  vi.mocked(ensureDefaultWorkspace).mockResolvedValue(storedWorkspace("system"));
  vi.mocked(locale).mockResolvedValue("en-US");
});

afterEach(async () => {
  await i18n.changeLanguage("en-US");
});

describe("LanguageSelector", () => {
  it("renders the System option plus the four supported locales", async () => {
    render(<LanguageSelector />);
    const select = await screen.findByLabelText("Language");
    const options = Array.from(select.querySelectorAll("option")).map((node) => node.value);
    expect(options).toEqual(["system", "en-US", "es-ES", "de-DE", "fr-FR"]);
    expect(screen.getByText("System")).toBeInTheDocument();
  });

  it("persists an explicit locale per workspace and switches immediately", async () => {
    vi.mocked(updateWorkspaceLocale).mockResolvedValue(storedWorkspace("es-ES"));

    render(<LanguageSelector />);
    const select = await screen.findByLabelText("Language");
    fireEvent.change(select, { target: { value: "es-ES" } });

    await waitFor(() => expect(updateWorkspaceLocale).toHaveBeenCalledWith("ws-1", "es-ES"));
    await waitFor(() => expect(i18n.language).toBe("es-ES"));

    // The very same selector is now rendered from the es-ES catalog.
    await waitFor(() => expect(screen.getByLabelText("Idioma")).toHaveValue("es-ES"));
    await waitFor(() =>
      expect(screen.getByRole("status")).toHaveTextContent(
        "Preferencia de idioma guardada. La aplicación ahora usa Español (España).",
      ),
    );
    // Explicit tags never consult the OS plugin.
    expect(locale).not.toHaveBeenCalled();
  });

  it("selecting System persists system and resolves through the OS mapping", async () => {
    vi.mocked(ensureDefaultWorkspace).mockResolvedValue(storedWorkspace("en-US"));
    vi.mocked(updateWorkspaceLocale).mockResolvedValue(storedWorkspace("system"));
    vi.mocked(locale).mockResolvedValue("de-AT");

    render(<LanguageSelector />);
    const select = await screen.findByLabelText("Language");
    await waitFor(() => expect(select).toHaveValue("en-US"));
    fireEvent.change(select, { target: { value: "system" } });

    await waitFor(() => expect(updateWorkspaceLocale).toHaveBeenCalledWith("ws-1", "system"));
    await waitFor(() => expect(i18n.language).toBe("de-DE"));
    await waitFor(() => expect(screen.getByLabelText("Sprache")).toHaveValue("system"));
    // The notice must be computed from the newly active locale, not from the
    // language bound to the render that started the switch.
    await waitFor(() =>
      expect(screen.getByRole("status")).toHaveTextContent(
        "Spracheinstellung gespeichert. Die App verwendet jetzt Deutsch (Deutschland).",
      ),
    );
  });

  it("falls back to English when System hits an OS failure, keeping system stored", async () => {
    vi.mocked(ensureDefaultWorkspace).mockResolvedValue(storedWorkspace("en-US"));
    vi.mocked(updateWorkspaceLocale).mockResolvedValue(storedWorkspace("system"));
    vi.mocked(locale).mockRejectedValue(new Error("os api unavailable"));

    render(<LanguageSelector />);
    const select = await screen.findByLabelText("Language");
    await waitFor(() => expect(select).toHaveValue("en-US"));
    fireEvent.change(select, { target: { value: "system" } });

    await waitFor(() => expect(updateWorkspaceLocale).toHaveBeenCalledWith("ws-1", "system"));
    await waitFor(() => expect(i18n.language).toBe("en-US"));
    // Stored preference stays `system`; resolution failure only affects this
    // session's active language, never the database.
    expect(select).toHaveValue("system");
    expect(screen.queryByRole("alert")).not.toBeInTheDocument();
  });

  it("reverts the selection and shows a localized error when persistence fails", async () => {
    vi.mocked(ensureDefaultWorkspace).mockResolvedValue(storedWorkspace("en-US"));
    vi.mocked(updateWorkspaceLocale).mockRejectedValue(
      JSON.stringify({ code: "VALIDATION", message: "The workspace locale is invalid." }),
    );

    render(<LanguageSelector />);
    const select = await screen.findByLabelText("Language");
    await waitFor(() => expect(select).toHaveValue("en-US"));
    fireEvent.change(select, { target: { value: "fr-FR" } });

    await waitFor(() =>
      expect(screen.getByRole("alert")).toHaveTextContent(
        "Please check the information and try again.",
      ),
    );
    expect(select).toHaveValue("en-US");
    expect(i18n.language).toBe("en-US");
  });
});
