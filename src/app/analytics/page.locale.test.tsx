import i18n from "i18next";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { render, screen } from "@testing-library/react";
import { MemoryRouter, Route, Routes } from "react-router";
import AnalyticsPage from "@/app/analytics/page";
import { SUPPORTED_LOCALES, type AppLocale } from "@/lib/i18n/locale";
import { getBlockRecurrenceAnalytics } from "@/lib/api/blockApi";
import { ensureDefaultWorkspace } from "@/lib/api/workspaceApi";

// 8.7.13 analytics locale coverage: every application-owned label of the
// read-only habit analytics screen renders from the active locale. The four
// catalogs carry the same keys (guarded by the catalog-parity test); here the
// translations themselves are asserted, so an English copy would fail.

vi.mock("@/lib/api/workspaceApi", () => ({
  ensureDefaultWorkspace: vi.fn(),
}));

vi.mock("@/lib/api/blockApi", () => ({
  getBlockRecurrenceAnalytics: vi.fn(),
}));

interface AnalyticsCopy {
  title: string;
  from: string;
  to: string;
  review: string;
  unrecorded: string;
}

const ANALYTICS_COPY: Record<AppLocale, AnalyticsCopy> = {
  "en-US": {
    title: "Habit analytics",
    from: "From date",
    to: "To date",
    review: "Review or edit routine",
    unrecorded: "Not recorded",
  },
  "es-ES": {
    title: "Analítica de hábitos",
    from: "Desde la fecha",
    to: "Hasta la fecha",
    review: "Revisar o editar la rutina",
    unrecorded: "Sin registrar",
  },
  "de-DE": {
    title: "Gewohnheitsanalyse",
    from: "Von Datum",
    to: "Bis Datum",
    review: "Routine prüfen oder bearbeiten",
    unrecorded: "Nicht erfasst",
  },
  "fr-FR": {
    title: "Analyse des habitudes",
    from: "Date de début",
    to: "Date de fin",
    review: "Vérifier ou modifier la routine",
    unrecorded: "Non enregistrés",
  },
};

const RESPONSE = {
  effective_from: "2026-01-05",
  effective_to: "2026-02-03",
  series: [
    {
      series_id: "series-1",
      display_label: "Morning walk",
      display_label_is_current_fallback: false,
      coverage: { start: "2025-11-03", intervals: [{ from: "2025-11-03", until: null }] },
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
      ],
    },
  ],
};

function renderPage() {
  return render(
    <MemoryRouter initialEntries={["/analytics"]}>
      <Routes>
        <Route path="/analytics" element={<AnalyticsPage />} />
      </Routes>
    </MemoryRouter>,
  );
}

beforeEach(() => {
  vi.clearAllMocks();
  vi.mocked(ensureDefaultWorkspace).mockResolvedValue({ id: "w1", name: "Default" } as never);
  vi.mocked(getBlockRecurrenceAnalytics).mockResolvedValue(RESPONSE as never);
});

afterEach(async () => {
  await i18n.changeLanguage("en-US");
});

describe.each([...SUPPORTED_LOCALES])("Analytics page in %s", (locale) => {
  it("renders the screen labels from the active locale", async () => {
    await i18n.changeLanguage(locale);
    const expected = ANALYTICS_COPY[locale];

    renderPage();

    expect(await screen.findByRole("heading", { name: expected.title })).toBeTruthy();
    expect(screen.getByLabelText(expected.from)).toBeTruthy();
    expect(screen.getByLabelText(expected.to)).toBeTruthy();
    expect(await screen.findByRole("button", { name: expected.review })).toBeTruthy();
    // The neutral "not recorded" counter label is translated too.
    expect((await screen.findAllByText(expected.unrecorded)).length).toBeGreaterThan(0);
  });
});
