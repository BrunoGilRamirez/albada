import i18n from "i18next";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { fireEvent, render, screen } from "@testing-library/react";
import { MemoryRouter } from "react-router";
import StandaloneTaskList from "@/components/tasks/StandaloneTaskList";
import { SUPPORTED_LOCALES, type AppLocale } from "@/lib/i18n/locale";
import type { DeadlineTaskInfo, Task } from "@/lib/types";

// 8.6.7 Standalone task list card + layout switcher: each task is one fully
// clickable card (a single link wrapping title and badges, never nested links
// or buttons), and the switcher only swaps the grid the cards live in. The
// selected layout is a UI-state preference persisted under one localStorage
// key; an unreadable or unknown stored value falls back to "list".
// Synthetic fixtures only — no journal or therapy content.

const LAYOUT_STORAGE_KEY = "joi-projectlife-task-list-layout";

const taskFixture: Task = {
  id: "t1",
  workspace_id: "w1",
  project_id: null,
  goal_id: null,
  parent_task_id: null,
  title: "Water the ferns",
  description_markdown: null,
  notes_markdown: null,
  blocking_note_markdown: null,
  status: "planned",
  priority: "high",
  effort_size: "medium",
  effort_points: 3,
  target_date: "2026-10-20",
  scheduled_date: null,
  completed_at: null,
  cancelled_at: null,
  closure_mode: null,
  closure_reason_markdown: null,
  created_at: "2026-09-08T10:00:00Z",
  updated_at: "2026-09-08T10:00:00Z",
  archived_at: null,
};

const deadlineFixture: DeadlineTaskInfo = {
  task_id: "t1",
  title: "Water the ferns",
  status: "planned",
  deadline_at: "2026-10-20",
  deadline_status: "overdue",
};

interface ListCopy {
  groupLabel: string;
  list: string;
  columns: string;
  tiles: string;
  statusPlanned: string;
  priorityHigh: string;
  effortMedium: string;
  pointsSuffix: string;
  overdue: string;
}

const COPY: Record<AppLocale, ListCopy> = {
  "en-US": {
    groupLabel: "List layout",
    list: "List",
    columns: "Two columns",
    tiles: "Tiles",
    statusPlanned: "Planned",
    priorityHigh: "High",
    effortMedium: "Medium",
    pointsSuffix: "3p",
    overdue: "Overdue",
  },
  "es-ES": {
    groupLabel: "Diseño de lista",
    list: "Lista",
    columns: "Dos columnas",
    tiles: "Mosaico",
    statusPlanned: "Planificada",
    priorityHigh: "Alta",
    effortMedium: "Mediana",
    pointsSuffix: "3 p",
    overdue: "Vencida",
  },
  "de-DE": {
    groupLabel: "Listenlayout",
    list: "Liste",
    columns: "Zwei Spalten",
    tiles: "Kacheln",
    statusPlanned: "Geplant",
    priorityHigh: "Hoch",
    effortMedium: "Mittel",
    pointsSuffix: "3 P",
    overdue: "Überfällig",
  },
  "fr-FR": {
    groupLabel: "Affichage de la liste",
    list: "Liste",
    columns: "Deux colonnes",
    tiles: "Tuiles",
    statusPlanned: "Planifiée",
    priorityHigh: "Haute",
    effortMedium: "Moyenne",
    pointsSuffix: "3 p",
    overdue: "En retard",
  },
};

function renderList(deadlines: Record<string, DeadlineTaskInfo> = { t1: deadlineFixture }) {
  return render(
    <MemoryRouter initialEntries={["/tasks"]}>
      <StandaloneTaskList tasks={[taskFixture]} deadlines={deadlines} />
    </MemoryRouter>,
  );
}

function card(container: HTMLElement, id: string): HTMLElement {
  const found = container.querySelector<HTMLElement>(`li[data-notification-target="${id}"]`);
  if (!found) throw new Error(`card ${id} not rendered`);
  return found;
}

/** The badge row is the second flex child of the card content column. */
function badgeRow(cardEl: HTMLElement): HTMLElement {
  const row = cardEl.querySelector<HTMLElement>(".flex.h-full.flex-col > div.flex.flex-wrap");
  if (!row) throw new Error("badge row not rendered");
  return row;
}

beforeEach(() => {
  localStorage.clear();
});

afterEach(async () => {
  await i18n.changeLanguage("en-US");
  vi.restoreAllMocks();
});

describe("StandaloneTaskList card", () => {
  it("shows project and goal labels as separate links", () => {
    render(
      <MemoryRouter initialEntries={["/tasks"]}>
        <StandaloneTaskList
          tasks={[{ ...taskFixture, project_id: "p1" }]}
          projects={[
            {
              id: "p1",
              workspace_id: "w1",
              goal_id: "g1",
              title: "Publish the guide",
            } as never,
          ]}
          goals={[{ id: "g1", title: "Share knowledge" } as never]}
        />
      </MemoryRouter>,
    );

    expect(screen.getByRole("link", { name: "Publish the guide" }).getAttribute("href")).toBe(
      "/projects/p1",
    );
    expect(screen.getByRole("link", { name: "Share knowledge" }).getAttribute("href")).toBe(
      "/goals/g1",
    );
  });

  it("renders the whole card as one link that carries the title and every badge", () => {
    const { container } = renderList();

    const el = card(container, "t1");
    const links = el.querySelectorAll("a");
    expect(links).toHaveLength(1);
    const link = links[0];
    expect(link.getAttribute("href")).toBe("/tasks/t1");

    // The task title is a link; application labels are separate links when present.
    expect(link.querySelector("a")).toBeNull();
    expect(el.querySelector("button")).toBeNull();
    expect(el.querySelectorAll(".joi-badge")).toHaveLength(4);

    // Authored title stays verbatim; application-owned chrome is inside the
    // same link, so the whole card is clickable.
    expect(link).toHaveTextContent("Water the ferns");
    expect(screen.getByText("Planned")).toBeTruthy();
    expect(screen.getByText("High")).toBeTruthy();
    expect(screen.getByText("Medium")).toBeTruthy();
    expect(screen.getByText("3p")).toBeTruthy();
    expect(screen.getAllByText("Overdue").length).toBeGreaterThan(0);

    const expectedDate = new Date("2026-10-20T12:00:00").toLocaleDateString("en-US", {
      dateStyle: "medium",
    });
    expect(screen.getByText(expectedDate)).toBeTruthy();
  });

  it("keeps every badge wrapper from truncating or overlapping", () => {
    const { container } = renderList();

    const badges = Array.from(badgeRow(card(container, "t1")).children).filter((child) =>
      child.classList.contains("joi-badge"),
    );
    expect(badges.length).toBe(4);
    for (const badge of badges) {
      expect(badge.className).toContain("joi-badge");
    }
  });

  it("hides the switcher behind the localized empty state when there is nothing to lay out", () => {
    render(
      <MemoryRouter initialEntries={["/tasks"]}>
        <StandaloneTaskList tasks={[]} />
      </MemoryRouter>,
    );

    const empty = "No tasks yet. Create one to capture an action.";
    expect(screen.getByText(empty)).toBeTruthy();
    expect(screen.queryByRole("group", { name: "List layout" })).toBeNull();
  });
});

describe("StandaloneTaskList layout switcher", () => {
  it("toggles the sort direction with an icon-only accessible button", () => {
    renderList();

    const direction = screen.getByRole("button", { name: "Sort ascending" });
    expect(direction).toHaveTextContent("↓");

    fireEvent.click(direction);

    expect(screen.getByRole("button", { name: "Sort descending" })).toHaveTextContent("↑");
  });

  it("switches between the three grids and records the choice", () => {
    const { container } = renderList();
    const list = () => container.querySelector("ul") as HTMLElement;

    // Default: list. One column, no pinned card height.
    expect(list().className).toBe("grid grid-cols-1 gap-3");
    expect(card(container, "t1").className).not.toContain("aspect-square");

    fireEvent.click(screen.getByRole("button", { name: "List" }));
    expect(list().className).toBe("grid grid-cols-1 gap-3 md:grid-cols-2");
    expect(card(container, "t1").className).not.toContain("aspect-square");
    expect(screen.getByRole("button", { name: "Two columns" })).toBeTruthy();
    expect(localStorage.getItem(LAYOUT_STORAGE_KEY)).toBe("columns");

    fireEvent.click(screen.getByRole("button", { name: "Two columns" }));
    expect(list().className).toBe(
      "grid gap-3 [grid-template-columns:repeat(auto-fill,minmax(14rem,1fr))]",
    );
    expect(card(container, "t1").className).toContain("aspect-square");
    expect(localStorage.getItem(LAYOUT_STORAGE_KEY)).toBe("tiles");

    fireEvent.click(screen.getByRole("button", { name: "Tiles" }));
    expect(list().className).toBe("grid grid-cols-1 gap-3");
    expect(localStorage.getItem(LAYOUT_STORAGE_KEY)).toBe("list");
  });

  it("clamps the title to two lines by default and three lines in tiles", () => {
    const { container } = renderList();
    const title = () => card(container, "t1").querySelector("a") as HTMLElement;

    expect(title().className).toContain("line-clamp-2");
    expect(badgeRow(card(container, "t1")).className).not.toContain("mt-auto");

    fireEvent.click(screen.getByRole("button", { name: "List" }));
    fireEvent.click(screen.getByRole("button", { name: "Two columns" }));
    expect(title().className).toContain("line-clamp-3");
    expect(badgeRow(card(container, "t1")).className).toContain("mt-auto");
  });

  it("restores a previously persisted layout on the next render", () => {
    localStorage.setItem(LAYOUT_STORAGE_KEY, "tiles");

    const { container } = renderList();

    expect(container.querySelector("ul")?.className).toContain("minmax(14rem,1fr)");
    expect(screen.getByRole("button", { name: "Tiles" })).toBeTruthy();
  });

  it("falls back to the list layout when the stored value is not one of the three modes", () => {
    localStorage.setItem(LAYOUT_STORAGE_KEY, "masonry");

    const { container } = renderList();

    expect(container.querySelector("ul")?.className).toBe("grid grid-cols-1 gap-3");
    expect(screen.getByRole("button", { name: "List" })).toBeTruthy();
  });

  it("keeps working when storage reads and writes are unavailable", () => {
    const getItem = vi.spyOn(Storage.prototype, "getItem").mockImplementation(() => {
      throw new Error("storage blocked");
    });
    const { container, unmount } = renderList();
    getItem.mockRestore();
    expect(container.querySelector("ul")?.className).toBe("grid grid-cols-1 gap-3");

    const setItem = vi.spyOn(Storage.prototype, "setItem").mockImplementation(() => {
      throw new Error("storage full");
    });
    fireEvent.click(screen.getByRole("button", { name: "List" }));
    fireEvent.click(screen.getByRole("button", { name: "Two columns" }));
    expect(container.querySelector("ul")?.className).toContain("minmax(14rem,1fr)");
    setItem.mockRestore();

    unmount();
    renderList();
    expect(screen.getByRole("button", { name: "List" })).toBeTruthy();
  });
});

describe.each([...SUPPORTED_LOCALES])("StandaloneTaskList switcher in %s", (locale) => {
  it("labels the group and its three modes from the active catalog", async () => {
    await i18n.changeLanguage(locale);
    const e = COPY[locale];

    renderList();

    expect(screen.getByRole("group", { name: e.groupLabel })).toBeTruthy();
    const control = screen.getByRole("button", { name: e.list });
    expect(control.className).toContain("joi-btn--secondary");
    expect(control.querySelector("img")?.className).toContain("joi-layout-icon");
    fireEvent.click(control);
    expect(screen.getByRole("button", { name: e.columns })).toBeTruthy();
    fireEvent.click(screen.getByRole("button", { name: e.columns }));
    expect(screen.getByRole("button", { name: e.tiles })).toBeTruthy();
  });

  it("localizes the card chrome while the authored title stays verbatim", async () => {
    await i18n.changeLanguage(locale);
    const e = COPY[locale];

    const { container } = renderList();

    const link = card(container, "t1").querySelector("a");
    expect(link).toHaveTextContent("Water the ferns");
    expect(screen.getByText(e.statusPlanned)).toBeTruthy();
    expect(screen.getByText(e.priorityHigh)).toBeTruthy();
    expect(screen.getByText(e.effortMedium)).toBeTruthy();
    expect(screen.getByText(e.pointsSuffix)).toBeTruthy();
    expect(screen.getAllByText(e.overdue).length).toBeGreaterThan(0);
  });
});
