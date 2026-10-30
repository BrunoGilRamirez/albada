import i18n from "i18next";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { render, screen, waitFor } from "@testing-library/react";
import { MemoryRouter } from "react-router";
import AppSidebar from "@/components/navigation/AppSidebar";
import Breadcrumb from "@/components/navigation/Breadcrumb";
import CommandPalette from "@/components/CommandPalette";
import { SUPPORTED_LOCALES, type AppLocale } from "@/lib/i18n/locale";

// 8.6.8 shared-shell locale coverage: the sidebar frame, breadcrumb landmark,
// and command palette must render every application-owned string from the
// active locale, including at runtime without a remount. Synthetic fixtures
// only; the bell's history panel itself is owned by the 8.6.11 notification
// area, so only the bell control labels are asserted here.

vi.mock("@/lib/api/workspaceApi", () => ({
  ensureDefaultWorkspace: vi.fn(),
}));

vi.mock("@/lib/api/notificationApi", () => ({
  listNotificationDeliveries: vi.fn(),
  listReminderRules: vi.fn(),
  dismissNotification: vi.fn(),
  snoozeNotification: vi.fn(),
}));

import { listNotificationDeliveries } from "@/lib/api/notificationApi";
import { ensureDefaultWorkspace } from "@/lib/api/workspaceApi";

interface ShellCopy {
  primaryNav: string;
  breadcrumb: string;
  navToday: string;
  navCalendar: string;
  navSettings: string;
  themeSystem: string;
  switchToDark: string;
  paletteLabel: string;
  paletteSearch: string;
  palettePlaceholder: string;
  bellUnread: string;
}

const SHELL_COPY: Record<AppLocale, ShellCopy> = {
  "en-US": {
    primaryNav: "Primary navigation",
    breadcrumb: "Breadcrumb",
    navToday: "Today",
    navCalendar: "Calendar",
    navSettings: "Settings",
    themeSystem: "System",
    switchToDark: "Switch to dark mode",
    paletteLabel: "Command palette",
    paletteSearch: "Command palette search",
    palettePlaceholder: "Search is coming soon...",
    bellUnread: "Notifications (2 unread)",
  },
  "es-ES": {
    primaryNav: "Navegación principal",
    breadcrumb: "Ruta de navegación",
    navToday: "Hoy",
    navCalendar: "Calendario",
    navSettings: "Configuración",
    themeSystem: "Sistema",
    switchToDark: "Cambiar a modo oscuro",
    paletteLabel: "Paleta de comandos",
    paletteSearch: "Búsqueda de la paleta de comandos",
    palettePlaceholder: "La búsqueda llegará pronto...",
    bellUnread: "Notificaciones (2 sin leer)",
  },
  "de-DE": {
    primaryNav: "Hauptnavigation",
    breadcrumb: "Brotkrumennavigation",
    navToday: "Heute",
    navCalendar: "Kalender",
    navSettings: "Einstellungen",
    themeSystem: "System",
    switchToDark: "Zum dunklen Modus wechseln",
    paletteLabel: "Befehlspalette",
    paletteSearch: "Befehlspalette durchsuchen",
    palettePlaceholder: "Die Suche folgt in einer späteren Version...",
    bellUnread: "Benachrichtigungen (2 ungelesen)",
  },
  "fr-FR": {
    primaryNav: "Navigation principale",
    breadcrumb: "Fil d'Ariane",
    navToday: "Aujourd'hui",
    navCalendar: "Calendrier",
    navSettings: "Paramètres",
    themeSystem: "Système",
    switchToDark: "Passer en mode sombre",
    paletteLabel: "Palette de commandes",
    paletteSearch: "Recherche de la palette de commandes",
    palettePlaceholder: "La recherche arrive bientôt...",
    bellUnread: "Notifications (2 non lues)",
  },
};

beforeEach(() => {
  vi.mocked(ensureDefaultWorkspace).mockReset();
  vi.mocked(listNotificationDeliveries).mockReset();
  vi.mocked(ensureDefaultWorkspace).mockResolvedValue({ id: "w1", name: "Default" } as never);
  vi.mocked(listNotificationDeliveries).mockResolvedValue([
    { id: "d1", status: "scheduled" },
    { id: "d2", status: "scheduled" },
  ] as never);
});

afterEach(async () => {
  await i18n.changeLanguage("en-US");
});

describe.each([...SUPPORTED_LOCALES])("Shared shell in %s", (locale) => {
  it("renders navigation, theme control, breadcrumb, and bell labels", async () => {
    await i18n.changeLanguage(locale);
    const expected = SHELL_COPY[locale];

    render(
      <MemoryRouter initialEntries={["/"]}>
        <AppSidebar />
      </MemoryRouter>,
    );

    const nav = await screen.findByRole("navigation", { name: expected.primaryNav });
    expect(nav).toBeTruthy();
    expect(screen.getByRole("link", { name: expected.navToday })).toBeTruthy();
    expect(screen.getByRole("link", { name: expected.navCalendar })).toBeTruthy();
    expect(screen.getByRole("link", { name: expected.navSettings })).toBeTruthy();
    // The default theme mode is "system": its visible label comes from the
    // catalog, and the toggle's accessible name is the localized switch aria.
    expect(screen.getByText(expected.themeSystem)).toBeTruthy();
    expect(screen.getByRole("button", { name: expected.switchToDark })).toBeTruthy();
    // The bell control's accessible name is the localized pluralized unread
    // count (its history panel is owned by the 8.6.11 notification area); the
    // count lands after the async workspace + pending-delivery load.
    expect(await screen.findByRole("button", { name: expected.bellUnread })).toBeTruthy();

    render(
      <MemoryRouter>
        <Breadcrumb
          items={[
            { label: "Synthetic parent", to: "/synthetic-parent" },
            { label: "Synthetic item" },
          ]}
        />
      </MemoryRouter>,
    );
    expect(screen.getByRole("navigation", { name: expected.breadcrumb })).toBeTruthy();
    expect(screen.getByRole("link", { name: "Synthetic parent" })).toHaveClass(
      "joi-btn",
      "joi-btn--secondary",
    );
  });

  it("renders the command palette labels", async () => {
    await i18n.changeLanguage(locale);
    const expected = SHELL_COPY[locale];
    render(<CommandPalette isOpen onOpenChange={vi.fn()} />);

    expect(screen.getByRole("dialog", { name: expected.paletteLabel })).toBeTruthy();
    expect(screen.getByRole("textbox", { name: expected.paletteSearch })).toBeTruthy();
    expect(screen.getByPlaceholderText(expected.palettePlaceholder)).toBeTruthy();
  });
});

describe("Shared shell runtime locale switching", () => {
  it("re-renders the sidebar in the new language without remounting", async () => {
    render(
      <MemoryRouter initialEntries={["/"]}>
        <AppSidebar />
      </MemoryRouter>,
    );

    expect(await screen.findByRole("link", { name: "Today" })).toBeTruthy();

    await i18n.changeLanguage("es-ES");
    await waitFor(() => expect(screen.getByRole("link", { name: "Hoy" })).toBeTruthy());
    expect(screen.queryByRole("link", { name: "Today" })).toBeNull();
    await waitFor(() =>
      expect(screen.getByRole("button", { name: "Notificaciones (2 sin leer)" })).toBeTruthy(),
    );
  });
});
