import i18n from "i18next";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { MemoryRouter, Route, Routes } from "react-router";
import GoalsPage from "@/app/goals/page";
import { SUPPORTED_LOCALES, type AppLocale } from "@/lib/i18n/locale";

// 8.6.9 Goals list locale coverage: heading, new-goal action, subtitle, empty
// state, row status/progress/deadline badges, and the localized deadline
// tooltip render from the active locale; the user-authored goal title stays
// verbatim. Synthetic fixtures only.

// 8.6.9 Goals list card + layout switcher: each goal is one fully clickable
// card (a single link wrapping title, progress, and badges, never nested links
// or buttons), and the switcher only swaps the grid the cards live in. The
// selected layout is a UI-state preference persisted under one localStorage
// key; an unreadable or unknown stored value falls back to "list".

const LAYOUT_STORAGE_KEY = "joi-projectlife-goal-list-layout";

vi.mock("@/lib/api/workspaceApi", () => ({
  ensureDefaultWorkspace: vi.fn(),
}));

vi.mock("@/lib/api/goalApi", () => ({
  listGoals: vi.fn(),
}));

vi.mock("@/lib/api/progressApi", () => ({
  getGoalProgress: vi.fn(),
}));

vi.mock("@/lib/api/deadlineApi", () => ({
  getGoalDeadlineStatus: vi.fn(),
}));

import { ensureDefaultWorkspace } from "@/lib/api/workspaceApi";
import { listGoals } from "@/lib/api/goalApi";
import { getGoalProgress } from "@/lib/api/progressApi";
import { getGoalDeadlineStatus } from "@/lib/api/deadlineApi";

const goalFixture = {
  id: "g1",
  workspace_id: "w1",
  year_theme_id: null,
  title: "Run the Maratón",
  description_markdown: null,
  why_it_matters_markdown: null,
  success_definition_markdown: null,
  status: "active",
  priority: "high",
  target_date: "2026-12-01",
  created_at: "2026-09-08T10:00:00Z",
  updated_at: "2026-09-08T10:00:00Z",
  archived_at: null,
};

interface GoalsCopy {
  title: string;
  newGoal: string;
  subtitle: string;
  empty: string;
  statusActive: string;
  progressNotStarted: string;
  deadlineApproaching: string;
  deadlineTooltip: string;
  layoutGroupLabel: string;
  layoutList: string;
  layoutColumns: string;
  layoutTiles: string;
}

const COPY: Record<AppLocale, GoalsCopy> = {
  "en-US": {
    title: "Goals",
    newGoal: "New goal",
    subtitle:
      "Meaningful directions. Progress includes direct tasks and the leaves of the projects linked to each goal.",
    empty: "No goals yet. Start with a direction you want to make real.",
    statusActive: "Active",
    progressNotStarted: "Not started",
    deadlineApproaching: "Approaching",
    deadlineTooltip: "deadline 2026-12-01",
    layoutGroupLabel: "List layout",
    layoutList: "List",
    layoutColumns: "Two columns",
    layoutTiles: "Tiles",
  },
  "es-ES": {
    title: "Objetivos",
    newGoal: "Nuevo objetivo",
    subtitle:
      "Direcciones con significado. El progreso incluye las tareas directas y las hojas de los proyectos vinculados a cada objetivo.",
    empty: "Aún no hay objetivos. Empieza por una dirección que quieras hacer realidad.",
    statusActive: "Activo",
    progressNotStarted: "Sin comenzar",
    deadlineApproaching: "Próxima",
    deadlineTooltip: "fecha límite 2026-12-01",
    layoutGroupLabel: "Diseño de lista",
    layoutList: "Lista",
    layoutColumns: "Dos columnas",
    layoutTiles: "Mosaico",
  },
  "de-DE": {
    title: "Ziele",
    newGoal: "Neues Ziel",
    subtitle:
      "Bedeutungsvolle Richtungen. Der Fortschritt umfasst direkte Aufgaben und die Endaufgaben der mit jedem Ziel verknüpften Projekte.",
    empty: "Noch keine Ziele. Beginnen Sie mit einer Richtung, die Sie verwirklichen möchten.",
    statusActive: "Aktiv",
    progressNotStarted: "Nicht begonnen",
    deadlineApproaching: "Bald fällig",
    deadlineTooltip: "Frist 2026-12-01",
    layoutGroupLabel: "Listenlayout",
    layoutList: "Liste",
    layoutColumns: "Zwei Spalten",
    layoutTiles: "Kacheln",
  },
  "fr-FR": {
    title: "Objectifs",
    newGoal: "Nouvel objectif",
    subtitle:
      "Des directions qui ont du sens. La progression inclut les tâches directes et les feuilles des projets liés à chaque objectif.",
    empty:
      "Aucun objectif pour l'instant. Commencez par une direction que vous voulez concrétiser.",
    statusActive: "Actif",
    progressNotStarted: "Non démarré",
    deadlineApproaching: "Proche",
    deadlineTooltip: "échéance 2026-12-01",
    layoutGroupLabel: "Affichage de la liste",
    layoutList: "Liste",
    layoutColumns: "Deux colonnes",
    layoutTiles: "Tuiles",
  },
};

function renderPage() {
  return render(
    <MemoryRouter initialEntries={["/goals"]}>
      <Routes>
        <Route path="/goals" element={<GoalsPage />} />
      </Routes>
    </MemoryRouter>,
  );
}

/** Renders the single-fixture list with its progress and deadline details. */
async function renderPageWithGoal() {
  vi.mocked(listGoals).mockResolvedValue([goalFixture] as never);
  vi.mocked(getGoalProgress).mockResolvedValue({
    percentage: null,
    label: "not_started",
    completed_points: 0,
    total_points: 0,
    contributors: [],
  } as never);
  vi.mocked(getGoalDeadlineStatus).mockResolvedValue({
    status: "approaching",
    at_risk_tasks: [],
    warning_window_days: 7,
  } as never);
  const result = renderPage();
  await screen.findByText("Run the Maratón");
  // Progress and deadline badges land asynchronously after the list renders.
  // Wait for them so callers that inspect the badge row don't read a partial
  // card. Use the active locale's catalog because the localized suites run
  // this helper under every supported locale.
  const copy = COPY[i18n.language as AppLocale] ?? COPY["en-US"];
  await screen.findByText(copy.progressNotStarted);
  await screen.findByText(copy.deadlineApproaching);
  return result;
}

function goalList(container: HTMLElement): HTMLElement {
  const list = container.querySelector("ul");
  if (!list) throw new Error("goal list not rendered");
  return list;
}

function goalCard(container: HTMLElement): HTMLElement {
  const card = container.querySelector("li");
  if (!card) throw new Error("goal card not rendered");
  return card;
}

/** The badge row is the last flex child of the card link's column. */
function badgeRow(card: HTMLElement): HTMLElement {
  const row = card.querySelector<HTMLElement>("a > div > div:last-of-type");
  if (!row) throw new Error("badge row not rendered");
  return row;
}

/** The card is expected to expose exactly one link; nothing else is interactive. */
function cardLink(card: HTMLElement): HTMLAnchorElement {
  const links = card.querySelectorAll("a");
  if (links.length !== 1) throw new Error(`expected one card link, found ${links.length}`);
  return links[0];
}

function cardTitle(card: HTMLElement): HTMLElement {
  const title = card.querySelector<HTMLElement>("a > div > div:first-of-type");
  if (!title) throw new Error("card title not rendered");
  return title;
}

beforeEach(() => {
  localStorage.clear();
  vi.mocked(ensureDefaultWorkspace).mockReset();
  vi.mocked(listGoals).mockReset();
  vi.mocked(getGoalProgress).mockReset();
  vi.mocked(getGoalDeadlineStatus).mockReset();
  vi.mocked(ensureDefaultWorkspace).mockResolvedValue({ id: "w1", name: "Default" } as never);
});

afterEach(async () => {
  await i18n.changeLanguage("en-US");
});

describe.each([...SUPPORTED_LOCALES])("Goals list in %s", (locale) => {
  it("renders the heading, action, subtitle, and empty state", async () => {
    await i18n.changeLanguage(locale);
    const e = COPY[locale];
    vi.mocked(listGoals).mockResolvedValue([]);

    renderPage();

    expect(await screen.findByRole("heading", { name: e.title })).toBeTruthy();
    expect(screen.getByRole("link", { name: e.newGoal })).toBeTruthy();
    expect(screen.getByText(e.subtitle)).toBeTruthy();
    expect(screen.getByText(e.empty)).toBeTruthy();
    // Nothing to lay out: the switcher stays hidden behind the empty state.
    expect(screen.queryByRole("group", { name: e.layoutGroupLabel })).toBeNull();
  });

  it("renders localized row badges while keeping the authored title verbatim", async () => {
    await i18n.changeLanguage(locale);
    const e = COPY[locale];
    vi.mocked(listGoals).mockResolvedValue([goalFixture] as never);
    vi.mocked(getGoalProgress).mockResolvedValue({
      percentage: null,
      label: "not_started",
      completed_points: 0,
      total_points: 0,
      contributors: [],
    } as never);
    vi.mocked(getGoalDeadlineStatus).mockResolvedValue({
      status: "approaching",
      at_risk_tasks: [],
      warning_window_days: 7,
    } as never);

    renderPage();

    expect(await screen.findByText("Run the Maratón")).toBeTruthy();
    expect(await screen.findByText(e.statusActive)).toBeTruthy();
    // Progress/deadline state lands asynchronously after the list renders.
    expect(await screen.findByText(e.progressNotStarted)).toBeTruthy();
    const badge = await screen.findByText(e.deadlineApproaching);
    expect(badge.getAttribute("title")).toBe(e.deadlineTooltip);
  });
});

describe("Goals list runtime locale switching", () => {
  it("re-renders the page chrome in the new language without remounting", async () => {
    vi.mocked(listGoals).mockResolvedValue([]);
    renderPage();

    expect(await screen.findByRole("heading", { name: "Goals" })).toBeTruthy();

    await i18n.changeLanguage("de-DE");
    await waitFor(() => expect(screen.getByRole("heading", { name: "Ziele" })).toBeTruthy());
    expect(screen.getByRole("link", { name: "Neues Ziel" })).toBeTruthy();
    expect(screen.queryByRole("heading", { name: "Goals" })).toBeNull();
  });
});

describe("Goals list card", () => {
  it("renders the whole card as one link that carries the title and every badge", async () => {
    const { container } = await renderPageWithGoal();

    const card = goalCard(container);
    expect(card.querySelectorAll("a")).toHaveLength(1);
    const link = cardLink(card);
    expect(link.getAttribute("href")).toBe("/goals/g1");

    // No nested link or button inside the card: progress and badges stay
    // plain spans, so the card is the only interactive surface.
    expect(link.querySelector("a")).toBeNull();
    expect(card.querySelector("button")).toBeNull();

    // Authored title stays verbatim; the badges live in the same link.
    expect(link).toHaveTextContent("Run the Maratón");
    expect(await screen.findByText("Active")).toBeTruthy();
    expect(await screen.findByText("Approaching")).toBeTruthy();
    expect(await screen.findByText("Not started")).toBeTruthy();
  });

  it("keeps the progress bar non-interactive inside the card link", async () => {
    vi.mocked(listGoals).mockResolvedValue([goalFixture] as never);
    vi.mocked(getGoalProgress).mockResolvedValue({
      percentage: 40,
      label: null,
      completed_points: 4,
      total_points: 10,
      contributors: [],
    } as never);
    vi.mocked(getGoalDeadlineStatus).mockResolvedValue({
      status: "none",
      at_risk_tasks: [],
      warning_window_days: 7,
    } as never);

    const { container } = renderPage();

    // The list itself renders asynchronously; wait for the progress bar
    // before grabbing the card.
    const bar = await screen.findByRole("progressbar");
    const card = goalCard(container);
    const link = cardLink(card);
    expect(bar.closest("a")).toBe(link);
    expect(card.querySelector("button")).toBeNull();
    expect(link.querySelector("a")).toBeNull();
    // "none" deadline hides the deadline badge without breaking the card.
    expect(card.querySelectorAll(".joi-badge")).toHaveLength(1);
  });

  it("keeps every badge wrapper from truncating or overlapping", async () => {
    const { container } = await renderPageWithGoal();

    const wrappers = Array.from(badgeRow(goalCard(container)).children);
    expect(wrappers).toHaveLength(2);
    for (const wrapper of wrappers) {
      expect(wrapper.className).toContain("shrink-0");
      expect(wrapper.className).toContain("whitespace-nowrap");
    }
  });
});

describe("Goals list layout switcher", () => {
  it("switches between the three grids and records the choice", async () => {
    const { container } = await renderPageWithGoal();

    // Default: list. One column, no pinned card height.
    expect(goalList(container).className).toBe("grid grid-cols-1 gap-3");
    expect(goalCard(container).className).not.toContain("aspect-square");

    fireEvent.click(screen.getByRole("button", { name: "List" }));
    expect(goalList(container).className).toBe("grid grid-cols-1 gap-3 md:grid-cols-2");
    expect(goalCard(container).className).not.toContain("aspect-square");
    expect(screen.getByRole("button", { name: "Two columns" })).toBeTruthy();
    expect(localStorage.getItem(LAYOUT_STORAGE_KEY)).toBe("columns");

    fireEvent.click(screen.getByRole("button", { name: "Two columns" }));
    expect(goalList(container).className).toBe(
      "grid gap-3 [grid-template-columns:repeat(auto-fill,minmax(14rem,1fr))]",
    );
    expect(goalCard(container).className).toContain("aspect-square");
    expect(localStorage.getItem(LAYOUT_STORAGE_KEY)).toBe("tiles");

    fireEvent.click(screen.getByRole("button", { name: "Tiles" }));
    expect(goalList(container).className).toBe("grid grid-cols-1 gap-3");
    expect(localStorage.getItem(LAYOUT_STORAGE_KEY)).toBe("list");
  });

  it("clamps the title to two lines by default and three lines in tiles", async () => {
    const { container } = await renderPageWithGoal();

    expect(cardTitle(goalCard(container)).className).toContain("line-clamp-2");
    expect(badgeRow(goalCard(container)).className).not.toContain("mt-auto");

    fireEvent.click(screen.getByRole("button", { name: "List" }));
    fireEvent.click(screen.getByRole("button", { name: "Two columns" }));
    expect(cardTitle(goalCard(container)).className).toContain("line-clamp-3");
    expect(badgeRow(goalCard(container)).className).toContain("mt-auto");
  });

  it("restores a previously persisted layout on the next render", async () => {
    localStorage.setItem(LAYOUT_STORAGE_KEY, "tiles");
    const { container } = await renderPageWithGoal();

    expect(goalList(container).className).toContain("minmax(14rem,1fr)");
    expect(screen.getByRole("button", { name: "Tiles" })).toBeTruthy();
  });

  it("falls back to the list layout when the stored value is not one of the three modes", async () => {
    localStorage.setItem(LAYOUT_STORAGE_KEY, "masonry");
    const { container } = await renderPageWithGoal();

    expect(goalList(container).className).toBe("grid grid-cols-1 gap-3");
    expect(screen.getByRole("button", { name: "List" })).toBeTruthy();
  });

  it("keeps working when storage reads and writes are unavailable", async () => {
    vi.mocked(listGoals).mockResolvedValue([goalFixture] as never);
    const getItem = vi.spyOn(Storage.prototype, "getItem").mockImplementation(() => {
      throw new Error("storage blocked");
    });
    const { container } = renderPage();
    getItem.mockRestore();
    await screen.findByText("Run the Maratón");
    expect(goalList(container).className).toBe("grid grid-cols-1 gap-3");

    const setItem = vi.spyOn(Storage.prototype, "setItem").mockImplementation(() => {
      throw new Error("storage full");
    });
    fireEvent.click(screen.getByRole("button", { name: "List" }));
    fireEvent.click(screen.getByRole("button", { name: "Two columns" }));
    expect(goalList(container).className).toContain("minmax(14rem,1fr)");
    setItem.mockRestore();
  });
});

describe.each([...SUPPORTED_LOCALES])("Goals list card + switcher in %s", (locale) => {
  it("labels the group and its three modes from the active catalog", async () => {
    await i18n.changeLanguage(locale);
    const e = COPY[locale];
    await renderPageWithGoal();

    expect(screen.getByRole("group", { name: e.layoutGroupLabel })).toBeTruthy();
    const control = screen.getByRole("button", { name: e.layoutList });
    expect(control.className).toContain("joi-btn--secondary");
    expect(control.querySelector("img")?.className).toContain("joi-layout-icon");
    fireEvent.click(control);
    expect(screen.getByRole("button", { name: e.layoutColumns })).toBeTruthy();
    fireEvent.click(screen.getByRole("button", { name: e.layoutColumns }));
    expect(screen.getByRole("button", { name: e.layoutTiles })).toBeTruthy();
  });

  it("localizes the card badges while the authored title stays verbatim", async () => {
    await i18n.changeLanguage(locale);
    const e = COPY[locale];
    const { container } = await renderPageWithGoal();

    // Progress/deadline details land asynchronously after the list renders.
    await screen.findByText(e.statusActive);
    await screen.findByText(e.deadlineApproaching);
    await screen.findByText(e.progressNotStarted);
    const card = goalCard(container);
    const link = within(card).getByRole("link", { name: /Run the Maratón/ });
    expect(link.getAttribute("href")).toBe("/goals/g1");
    expect(within(link).getByText(e.statusActive)).toBeTruthy();
    expect(within(link).getByText(e.deadlineApproaching)).toBeTruthy();
    expect(within(link).getByText(e.progressNotStarted)).toBeTruthy();
    expect(link.querySelector("a")).toBeNull();
    expect(card.querySelector("button")).toBeNull();
  });
});
