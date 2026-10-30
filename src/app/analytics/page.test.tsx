import { describe, expect, it, vi, beforeEach } from "vitest";
import { fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { MemoryRouter, Route, Routes } from "react-router";
import AnalyticsPage from "@/app/analytics/page";
import {
  cancelBlock,
  completeBlock,
  createBlock,
  deleteBlock,
  getBlockRecurrenceAnalytics,
  skipBlock,
  startBlock,
  updateBlock,
} from "@/lib/api/blockApi";
import { ensureDefaultWorkspace } from "@/lib/api/workspaceApi";
import { listJournalEntries } from "@/lib/api/journalApi";
import { listTherapyCollections, listTherapyRecords } from "@/lib/api/therapyApi";
import type { RecurrenceAnalyticsResponse } from "@/lib/types";

// 8.7.13 habit analytics UI: a read-only view over the 8.7.12 typed API.
// Synthetic fixtures only, with fixed dates, so the expectations never depend
// on the current day.

const { navigateMock } = vi.hoisted(() => ({ navigateMock: vi.fn() }));

vi.mock("react-router", async (importOriginal) => {
  const actual = await importOriginal<typeof import("react-router")>();
  return { ...actual, useNavigate: () => navigateMock };
});

vi.mock("@/lib/api/workspaceApi", () => ({
  ensureDefaultWorkspace: vi.fn(),
}));

vi.mock("@/lib/api/blockApi", () => ({
  getBlockRecurrenceAnalytics: vi.fn(),
  // Mutating commands that must stay untouched on this read-only screen.
  createBlock: vi.fn(),
  updateBlock: vi.fn(),
  startBlock: vi.fn(),
  completeBlock: vi.fn(),
  skipBlock: vi.fn(),
  cancelBlock: vi.fn(),
  deleteBlock: vi.fn(),
}));

vi.mock("@/lib/api/therapyApi", () => ({
  listTherapyCollections: vi.fn(),
  listTherapyRecords: vi.fn(),
}));

vi.mock("@/lib/api/journalApi", () => ({
  listJournalEntries: vi.fn(),
}));

const RESPONSE: RecurrenceAnalyticsResponse = {
  effective_from: "2026-01-05",
  effective_to: "2026-02-03",
  series: [
    {
      series_id: "series-1",
      display_label: "Morning walk",
      display_label_is_current_fallback: false,
      coverage: {
        start: "2025-11-03",
        intervals: [{ from: "2025-11-03", until: null }],
      },
      counts: {
        scheduled: 20,
        completed: 8,
        started: 2,
        skipped: 1,
        cancelled: 1,
        unrecorded: 8,
      },
      rate: { denominator: 19, completed: 8, percent: 42 },
      weekly: [
        {
          week_start: "2026-01-05",
          scheduled: 4,
          completed: 2,
          started: 1,
          skipped: 0,
          cancelled: 0,
          unrecorded: 1,
        },
        {
          week_start: "2026-01-12",
          scheduled: 0,
          completed: 0,
          started: 0,
          skipped: 0,
          cancelled: 0,
          unrecorded: 0,
        },
      ],
    },
    {
      // Partial coverage: no recorded start, a closed (paused) interval, no
      // visible rate, and a label that only makes sense outside the range.
      series_id: "series-2",
      display_label: null,
      display_label_is_current_fallback: true,
      coverage: {
        start: null,
        intervals: [{ from: "2025-12-01", until: "2026-01-19" }],
      },
      counts: {
        scheduled: 6,
        completed: 3,
        started: 1,
        skipped: 1,
        cancelled: 0,
        unrecorded: 1,
      },
      rate: null,
      weekly: [
        {
          week_start: "2026-01-05",
          scheduled: 6,
          completed: 3,
          started: 1,
          skipped: 1,
          cancelled: 0,
          unrecorded: 1,
        },
      ],
    },
  ],
};

/** Application-owned local date, computed from a fixed date (never `new Date()`). */
function formatFixedDate(date: string): string {
  return new Date(`${date}T12:00:00`).toLocaleDateString("en-US", { dateStyle: "medium" });
}

function renderPage() {
  return render(
    <MemoryRouter initialEntries={["/analytics"]}>
      <Routes>
        <Route path="/analytics" element={<AnalyticsPage />} />
        <Route path="/" element={<p>Today</p>} />
      </Routes>
    </MemoryRouter>,
  );
}

/** Counter label and its value inside one card. */
function countValue(card: HTMLElement, label: string): string {
  const term = within(card).getAllByText(label)[0];
  return term.nextElementSibling?.textContent ?? "";
}

beforeEach(() => {
  vi.clearAllMocks();
  vi.mocked(ensureDefaultWorkspace).mockResolvedValue({ id: "w1", name: "Default" } as never);
  vi.mocked(getBlockRecurrenceAnalytics).mockResolvedValue(RESPONSE);
});

describe("Analytics page (read-only habit analytics)", () => {
  it("applies a new range and re-queries the analytics command with the new dates", async () => {
    renderPage();
    await screen.findByRole("heading", { name: "Morning walk" });

    expect(getBlockRecurrenceAnalytics).toHaveBeenCalledTimes(1);

    fireEvent.change(screen.getByLabelText("From date"), { target: { value: "2026-01-01" } });
    fireEvent.change(screen.getByLabelText("To date"), { target: { value: "2026-01-31" } });
    fireEvent.click(screen.getByRole("button", { name: "Apply" }));

    await waitFor(() => expect(getBlockRecurrenceAnalytics).toHaveBeenCalledTimes(2));
    expect(getBlockRecurrenceAnalytics).toHaveBeenLastCalledWith("w1", "2026-01-01", "2026-01-31");
  });

  it("rejects an inverted or malformed range inline and never calls the analytics command", async () => {
    renderPage();
    await screen.findByRole("heading", { name: "Morning walk" });
    const callsBefore = vi.mocked(getBlockRecurrenceAnalytics).mock.calls.length;

    fireEvent.change(screen.getByLabelText("From date"), { target: { value: "2026-02-01" } });
    fireEvent.change(screen.getByLabelText("To date"), { target: { value: "2026-01-01" } });
    fireEvent.click(screen.getByRole("button", { name: "Apply" }));

    const alert = await screen.findByRole("alert");
    expect(alert.textContent).toContain("The range start date cannot be after its end date.");
    expect(getBlockRecurrenceAnalytics).toHaveBeenCalledTimes(callsBefore);

    // A malformed (empty) end date is rejected the same way.
    fireEvent.change(screen.getByLabelText("To date"), { target: { value: "" } });
    fireEvent.click(screen.getByRole("button", { name: "Apply" }));

    expect((await screen.findAllByRole("alert"))[0].textContent).toContain(
      "The range start date cannot be after its end date.",
    );
    expect(getBlockRecurrenceAnalytics).toHaveBeenCalledTimes(callsBefore);
  });

  it("shows the empty state when the workspace has no recurring routine", async () => {
    vi.mocked(getBlockRecurrenceAnalytics).mockResolvedValue({
      effective_from: "2026-01-05",
      effective_to: "2026-02-03",
      series: [],
    });

    renderPage();

    expect(await screen.findByText("No recurring routines in this workspace yet.")).toBeTruthy();
  });

  it("states partial coverage, a hidden rate, a pause, and the out-of-range label plainly", async () => {
    renderPage();

    const card = await screen.findByTestId("analytics-series-series-2");

    // No recorded usage: the coverage line says so instead of guessing.
    expect(within(card).getByText("No recorded usage in this range.")).toBeTruthy();
    // No denominator: the rate stays hidden rather than showing a fake 0%.
    expect(within(card).getByText("No schedulable days in this period yet.")).toBeTruthy();
    // The closed interval is the visible incomplete-coverage indicator.
    expect(
      within(card).getByText(
        `Paused ${formatFixedDate("2025-12-01")} → ${formatFixedDate("2026-01-19")}`,
      ),
    ).toBeTruthy();
    // The label is the routine's current name, flagged as out-of-range context.
    expect(
      within(card).getByText(
        "No scheduled days fall in this range; the name shown is the routine's current name.",
      ),
    ).toBeTruthy();
    // A series without a label still gets a plain, translated heading.
    expect(within(card).getByRole("heading", { name: "Unnamed routine" })).toBeTruthy();
  });

  it("renders every outcome counter and the exact rate text without failure wording", async () => {
    renderPage();

    const card = await screen.findByTestId("analytics-series-series-1");

    expect(countValue(card, "Scheduled")).toBe("20");
    expect(countValue(card, "Completed")).toBe("8");
    expect(countValue(card, "Started (not completed)")).toBe("2");
    expect(countValue(card, "Skipped / expired")).toBe("1");
    expect(countValue(card, "Cancelled / withdrawn")).toBe("1");
    expect(countValue(card, "Not recorded")).toBe("8");

    expect(within(card).getByText("8 of 19 scheduled — 42%")).toBeTruthy();

    // "Not recorded" is a neutral label, never styled as a failure and never
    // described with failure wording.
    const unrecorded = within(card).getAllByText("Not recorded")[0];
    expect(unrecorded.parentElement?.className ?? "").not.toMatch(/danger|warning|success|caution/);
    expect(document.body.textContent ?? "").not.toMatch(/missed|failed/i);
  });

  it("navigates to the Today view from the review action", async () => {
    renderPage();

    const card = await screen.findByTestId("analytics-series-series-1");
    const review = within(card).getByRole("button", { name: "Review or edit routine" });
    fireEvent.click(review);

    expect(navigateMock).toHaveBeenCalledWith("/");
  });

  it("reads history only: no mutating, journal, or therapy command is called", async () => {
    renderPage();
    await screen.findByRole("heading", { name: "Morning walk" });

    expect(ensureDefaultWorkspace).toHaveBeenCalled();
    expect(getBlockRecurrenceAnalytics).toHaveBeenCalled();
    for (const mutating of [
      createBlock,
      updateBlock,
      startBlock,
      completeBlock,
      skipBlock,
      cancelBlock,
      deleteBlock,
    ]) {
      expect(mutating).not.toHaveBeenCalled();
    }
    expect(listTherapyCollections).not.toHaveBeenCalled();
    expect(listTherapyRecords).not.toHaveBeenCalled();
    expect(listJournalEntries).not.toHaveBeenCalled();
  });

  it("exposes headings, labelled range inputs, a captioned trend table, and status regions", async () => {
    renderPage();

    expect(screen.getByRole("heading", { name: "Habit analytics" })).toBeTruthy();
    expect(await screen.findByRole("heading", { name: "Morning walk" })).toBeTruthy();
    expect(screen.getByLabelText("From date")).toBeTruthy();
    expect(screen.getByLabelText("To date")).toBeTruthy();

    const card = await screen.findByTestId("analytics-series-series-1");
    const table = within(card).getByRole("table");
    expect(table.querySelector("caption")?.textContent).toBe("Weekly trend");
    expect(within(table).getByRole("columnheader", { name: "Week starting" })).toBeTruthy();
    // Zero-count weeks still render their row.
    expect(
      within(table).getByRole("rowheader", { name: formatFixedDate("2026-01-12") }),
    ).toBeTruthy();

    // The effective range is the clamped range echoed by the command.
    expect(
      screen.getByText(
        `Showing dates ${formatFixedDate("2026-01-05")} – ${formatFixedDate("2026-02-03")}`,
      ),
    ).toBeTruthy();
  });

  it("announces loading and failure states", async () => {
    vi.mocked(getBlockRecurrenceAnalytics).mockReturnValue(new Promise(() => {}) as never);
    const loadingView = renderPage();
    expect(await screen.findByRole("status")).toHaveTextContent("Loading…");
    loadingView.unmount();

    vi.mocked(getBlockRecurrenceAnalytics).mockRejectedValue({ code: "UNKNOWN" });
    renderPage();
    const alert = await screen.findByRole("alert");
    expect(alert.textContent).toContain("Something unexpected happened. Please try again.");
  });
});
