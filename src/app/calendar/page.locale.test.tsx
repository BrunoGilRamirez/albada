import i18n from "i18next";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { fireEvent, render, screen } from "@testing-library/react";
import { MemoryRouter } from "react-router";
import CalendarPage from "@/app/calendar/page";
import { SUPPORTED_LOCALES, type AppLocale } from "@/lib/i18n/locale";
import { listBlocksBetween } from "@/lib/api/blockApi";
import { listDailyPlansBetween } from "@/lib/api/dailyPlanApi";
import { listTasks } from "@/lib/api/taskApi";
import { ensureDefaultWorkspace } from "@/lib/api/workspaceApi";

// 8.6.8 Calendar locale coverage: titles, legend, controls, and day-cell dot
// tooltips render from the Calendar catalog in every supported locale, and
// the month heading plus weekday header come from the shared Intl formatter.
// The month under test is the current one (the page opens there), so the
// expected labels are computed with Intl instead of being hardcoded.

vi.mock("@/lib/api/workspaceApi", () => ({
  ensureDefaultWorkspace: vi.fn(),
}));

vi.mock("@/lib/api/dailyPlanApi", () => ({
  listDailyPlansBetween: vi.fn(),
}));

vi.mock("@/lib/api/blockApi", () => ({
  listBlocksBetween: vi.fn(),
}));

vi.mock("@/lib/api/taskApi", () => ({
  listTasks: vi.fn(),
}));

const now = new Date();
const year = now.getFullYear();
const month = now.getMonth();
const pad = (value: number) => String(value).padStart(2, "0");
const firstOfMonth = `${year}-${pad(month + 1)}-01`;

function renderPage() {
  return render(
    <MemoryRouter initialEntries={["/calendar"]}>
      <CalendarPage />
    </MemoryRouter>,
  );
}

interface CalendarCopy {
  title: string;
  previous: string;
  next: string;
  weekView: string;
  monthView: string;
  legendDailyPlan: string;
  legendBlocks: string;
  legendDeadlines: string;
  dotPlan: string;
  dotBlocks: string;
  dotDeadline: string;
}

const CALENDAR_COPY: Record<AppLocale, CalendarCopy> = {
  "en-US": {
    title: "Calendar",
    previous: "Prev",
    next: "Next",
    weekView: "Week view",
    monthView: "Month view",
    legendDailyPlan: "Daily plan",
    legendBlocks: "Blocks",
    legendDeadlines: "Deadlines",
    dotPlan: "Plan",
    dotBlocks: "Blocks",
    dotDeadline: "Deadline",
  },
  "es-ES": {
    title: "Calendario",
    previous: "Anterior",
    next: "Siguiente",
    weekView: "Vista semanal",
    monthView: "Vista mensual",
    legendDailyPlan: "Plan diario",
    legendBlocks: "Bloques",
    legendDeadlines: "Fechas límite",
    dotPlan: "Plan",
    dotBlocks: "Bloques",
    dotDeadline: "Fecha límite",
  },
  "de-DE": {
    title: "Kalender",
    previous: "Zurück",
    next: "Weiter",
    weekView: "Wochenansicht",
    monthView: "Monatsansicht",
    legendDailyPlan: "Tagesplan",
    legendBlocks: "Blöcke",
    legendDeadlines: "Fristen",
    dotPlan: "Plan",
    dotBlocks: "Blöcke",
    dotDeadline: "Frist",
  },
  "fr-FR": {
    title: "Calendrier",
    previous: "Précédent",
    next: "Suivant",
    weekView: "Vue hebdomadaire",
    monthView: "Vue mensuelle",
    legendDailyPlan: "Plan du jour",
    legendBlocks: "Blocs",
    legendDeadlines: "Échéances",
    dotPlan: "Plan",
    dotBlocks: "Blocs",
    dotDeadline: "Échéance",
  },
};

function localizedWeekdayLabels(locale: string): string[] {
  // Same fixed reference Sunday the grid uses (Jan 2, 2000).
  return Array.from({ length: 7 }, (_, index) =>
    new Date(2000, 0, 2 + index).toLocaleDateString(locale, { weekday: "short" }),
  );
}

beforeEach(() => {
  vi.mocked(ensureDefaultWorkspace).mockReset();
  vi.mocked(listDailyPlansBetween).mockReset();
  vi.mocked(listBlocksBetween).mockReset();
  vi.mocked(listTasks).mockReset();

  vi.mocked(ensureDefaultWorkspace).mockResolvedValue({ id: "w1", name: "Default" } as never);
  vi.mocked(listDailyPlansBetween).mockResolvedValue([{ date: firstOfMonth } as never]);
  vi.mocked(listBlocksBetween).mockResolvedValue([{ date: firstOfMonth } as never]);
  vi.mocked(listTasks).mockResolvedValue([{ id: "t1", target_date: firstOfMonth } as never]);
});

afterEach(async () => {
  await i18n.changeLanguage("en-US");
});

describe.each([...SUPPORTED_LOCALES])("Calendar page in %s", (locale) => {
  it("renders titles, controls, legend, and Intl-formatted headings", async () => {
    await i18n.changeLanguage(locale);
    const e = CALENDAR_COPY[locale];

    renderPage();

    expect(await screen.findByRole("heading", { name: e.title })).toBeTruthy();

    // Month heading: application-owned date through the shared Intl pattern.
    const monthLabel = new Date(year, month, 1).toLocaleDateString(locale, {
      month: "long",
      year: "numeric",
    });
    expect(screen.getByText(monthLabel)).toBeTruthy();

    // Localized navigation and view controls.
    expect(screen.getByRole("button", { name: e.previous })).toBeTruthy();
    expect(screen.getByRole("button", { name: e.next })).toBeTruthy();
    expect(screen.getByRole("button", { name: e.weekView })).toBeTruthy();

    // Legend entries come from the Calendar catalog.
    expect(screen.getByText(e.legendDailyPlan)).toBeTruthy();
    expect(screen.getByText(e.legendBlocks)).toBeTruthy();
    expect(screen.getByText(e.legendDeadlines)).toBeTruthy();

    // Weekday header row: seven short localized names from the Intl formatter.
    for (const label of localizedWeekdayLabels(locale)) {
      expect(screen.getByText(label)).toBeTruthy();
    }

    // The first-of-month fixture cell carries all three indicator dots with
    // localized tooltips, and its day number renders through the Intl number
    // formatter.
    expect(await screen.findByTitle(e.dotPlan)).toBeTruthy();
    expect(screen.getByTitle(e.dotBlocks)).toBeTruthy();
    expect(screen.getByTitle(e.dotDeadline)).toBeTruthy();
    expect(screen.getByText(new Intl.NumberFormat(locale).format(1))).toBeTruthy();
  });

  it("switches to the week view with localized controls", async () => {
    await i18n.changeLanguage(locale);
    const e = CALENDAR_COPY[locale];

    renderPage();
    await screen.findByRole("heading", { name: e.title });

    fireEvent.click(screen.getByRole("button", { name: e.weekView }));
    expect(await screen.findByRole("button", { name: e.monthView })).toBeTruthy();
  });
});

describe("Calendar page runtime locale switching", () => {
  it("re-renders the calendar in the new language without remounting", async () => {
    renderPage();
    expect(await screen.findByRole("heading", { name: "Calendar" })).toBeTruthy();

    await i18n.changeLanguage("es-ES");
    expect(await screen.findByRole("heading", { name: "Calendario" })).toBeTruthy();

    const monthLabel = new Date(year, month, 1).toLocaleDateString("es-ES", {
      month: "long",
      year: "numeric",
    });
    expect(screen.getByText(monthLabel)).toBeTruthy();
    expect(screen.getByText("Vista semanal")).toBeTruthy();
  });
});
