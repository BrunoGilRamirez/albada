import i18n from "i18next";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { act, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { MemoryRouter, Route, Routes } from "react-router";
import JournalPage from "@/app/journal/page";
import JournalDetailPage from "@/app/journal/[id]/page";
import JournalExportPage from "@/app/journal/export/page";
import {
  archiveJournalEntries,
  archiveJournalEntry,
  getJournalEntry,
  listJournalEntries,
  setJournalEntriesExportEligible,
} from "@/lib/api/journalApi";
import { listReferencesForEntry } from "@/lib/api/journalReferenceApi";
import {
  createExportJob,
  exportTemplateCapability,
  listExportJobs,
  previewExportSelection,
} from "@/lib/api/exportApi";
import { ensureDefaultWorkspace } from "@/lib/api/workspaceApi";
import { SUPPORTED_LOCALES, type AppLocale } from "@/lib/i18n/locale";
import type { JournalEntry } from "@/lib/types";

vi.mock("@/lib/api/journalApi", () => ({
  listJournalEntries: vi.fn(),
  archiveJournalEntries: vi.fn(),
  archiveJournalEntry: vi.fn(),
  getJournalEntry: vi.fn(),
  setJournalEntriesExportEligible: vi.fn(),
}));

vi.mock("@/lib/api/journalReferenceApi", () => ({
  addJournalReference: vi.fn(),
  listReferencesForEntry: vi.fn(),
  removeJournalReference: vi.fn(),
}));

vi.mock("@/lib/api/exportApi", () => ({
  createExportJob: vi.fn(),
  previewExportSelection: vi.fn(),
  runExportJob: vi.fn(),
  exportTemplateCapability: vi.fn(),
  listExportJobs: vi.fn(),
  openExportArtifact: vi.fn(),
  saveExportArtifactCopy: vi.fn(),
  deleteExportJob: vi.fn(),
}));

vi.mock("@/lib/api/workspaceApi", () => ({
  ensureDefaultWorkspace: vi.fn(),
}));

// 8.6.10 Journal page locale coverage: the list chrome, bulk actions,
// count-only pluralized feedback, privacy/exportability badges, and the
// export screen all follow the active locale. Authored entry titles stay
// verbatim in every language — the UI locale never rewrites journal content.
// Synthetic fixtures only; no personal journal text is used anywhere here.

function syntheticEntry(id: string, title: string, exportEligible: boolean): JournalEntry {
  return {
    id,
    workspace_id: "w1",
    occurred_on: "2026-10-02",
    title,
    content_markdown: "Synthetic body.",
    template_id: null,
    template_version: null,
    template_values: {},
    mood_label: null,
    energy_label: null,
    emotions: [],
    export_eligible: exportEligible,
    source: "user",
    created_at: "2026-10-02T09:00:00Z",
    updated_at: "2026-10-02T09:00:00Z",
    archived_at: null,
  };
}

const alpha = (): JournalEntry => syntheticEntry("j1", "Synthetic alpha", true);
const beta = (): JournalEntry => syntheticEntry("j2", "Synthetic beta", false);

/**
 * Detail fixture with authored prose, mood/energy labels, and emotions. The
 * date-only `occurred_on` is the application-owned value whose read-only
 * display renders through the Intl formatter in every locale.
 */
function detailEntry(): JournalEntry {
  return {
    ...syntheticEntry("j1", "Synthetic alpha", true),
    content_markdown: "# Synthetic quiet body\n\nprose stays verbatim",
    mood_label: "Synthetic calma",
    energy_label: "Synthetic alta",
    emotions: ["grata"],
  };
}

interface JournalPageCopy {
  title: string;
  newEntry: string;
  exportAction: string;
  selectAll: string;
  deselectAll: string;
  setExportable: string;
  setPrivate: string;
  archiveSelected: string;
  exportableBadge: string;
  privateBadge: string;
  empty: string;
  selectEntry: string;
  exportTitle: string;
  exportFormat: string;
  exportTemplate: string;
  exportAll: string;
  exportFrom: string;
  exportTo: string;
  exportSelected: string;
  exportEmpty: string;
  exportSubmit: string;
  editAction: string;
  archiveAction: string;
  olderEntry: string;
  referenceEmpty: string;
}

const COPY: Record<AppLocale, JournalPageCopy> = {
  "en-US": {
    title: "Journal",
    newEntry: "New entry",
    exportAction: "Export",
    selectAll: "Select all",
    deselectAll: "Deselect all",
    setExportable: "Set Exportable",
    setPrivate: "Set Private",
    archiveSelected: "Archive selected",
    exportableBadge: "Exportable",
    privateBadge: "Private",
    empty: "No journal entries yet. Write your first entry for today.",
    selectEntry: "Select {{title}}",
    exportTitle: "Export journal",
    exportFormat: "Format",
    exportTemplate: "Template",
    exportAll: "Export all",
    exportFrom: "From",
    exportTo: "To",
    exportSelected: "Selected entries",
    exportEmpty: "No journal entries yet.",
    exportSubmit: "Export PDF",
    editAction: "Edit",
    archiveAction: "Archive",
    olderEntry: "← Older entry",
    referenceEmpty: "No linked items yet.",
  },
  "es-ES": {
    title: "Diario",
    newEntry: "Nueva entrada",
    exportAction: "Exportar",
    selectAll: "Seleccionar todo",
    deselectAll: "Deseleccionar todo",
    setExportable: "Marcar como exportable",
    setPrivate: "Marcar como privada",
    archiveSelected: "Archivar selección",
    exportableBadge: "Exportable",
    privateBadge: "Privada",
    empty: "Aún no hay entradas de diario. Escribe tu primera entrada de hoy.",
    selectEntry: "Seleccionar {{title}}",
    exportTitle: "Exportar diario",
    exportFormat: "Formato",
    exportTemplate: "Plantilla",
    exportAll: "Exportar todo",
    exportFrom: "Desde",
    exportTo: "Hasta",
    exportSelected: "Entradas seleccionadas",
    exportEmpty: "Aún no hay entradas de diario.",
    exportSubmit: "Exportar como PDF",
    editAction: "Editar",
    archiveAction: "Archivar",
    olderEntry: "← Entrada anterior",
    referenceEmpty: "Aún no hay elementos vinculados.",
  },
  "de-DE": {
    title: "Tagebuch",
    newEntry: "Neuer Eintrag",
    exportAction: "Exportieren",
    selectAll: "Alle auswählen",
    deselectAll: "Auswahl aufheben",
    setExportable: "Als exportierbar markieren",
    setPrivate: "Als privat markieren",
    archiveSelected: "Auswahl archivieren",
    exportableBadge: "Exportierbar",
    privateBadge: "Privat",
    empty: "Noch keine Tagebucheinträge. Schreibe heute deinen ersten Eintrag.",
    selectEntry: "{{title}} auswählen",
    exportTitle: "Tagebuch exportieren",
    exportFormat: "Format",
    exportTemplate: "Vorlage",
    exportAll: "Alle exportieren",
    exportFrom: "Von",
    exportTo: "Bis",
    exportSelected: "Ausgewählte Einträge",
    exportEmpty: "Noch keine Tagebucheinträge.",
    exportSubmit: "PDF exportieren",
    editAction: "Bearbeiten",
    archiveAction: "Archivieren",
    olderEntry: "← Älterer Eintrag",
    referenceEmpty: "Noch keine verknüpften Elemente.",
  },
  "fr-FR": {
    title: "Journal",
    newEntry: "Nouvelle entrée",
    exportAction: "Exporter",
    selectAll: "Tout sélectionner",
    deselectAll: "Tout désélectionner",
    setExportable: "Marquer comme exportable",
    setPrivate: "Marquer comme privée",
    archiveSelected: "Archiver la sélection",
    exportableBadge: "Exportable",
    privateBadge: "Privée",
    empty: "Aucune entrée de journal pour l'instant. Rédigez votre première entrée du jour.",
    selectEntry: "Sélectionner {{title}}",
    exportTitle: "Exporter le journal",
    exportFormat: "Format",
    exportTemplate: "Modèle",
    exportAll: "Tout exporter",
    exportFrom: "Du",
    exportTo: "Au",
    exportSelected: "Entrées sélectionnées",
    exportEmpty: "Aucune entrée de journal.",
    exportSubmit: "Exporter en PDF",
    editAction: "Modifier",
    archiveAction: "Archiver",
    olderEntry: "← Entrée plus ancienne",
    referenceEmpty: "Aucun élément lié pour le moment.",
  },
};

function renderPage() {
  return render(
    <MemoryRouter initialEntries={["/journal"]}>
      <Routes>
        <Route path="/journal" element={<JournalPage />} />
      </Routes>
    </MemoryRouter>,
  );
}

function renderExportPage() {
  return render(
    <MemoryRouter initialEntries={["/journal/export"]}>
      <Routes>
        <Route path="/journal/export" element={<JournalExportPage />} />
      </Routes>
    </MemoryRouter>,
  );
}

function renderDetailPage() {
  return render(
    <MemoryRouter initialEntries={["/journal/j1"]}>
      <Routes>
        <Route path="/journal/:id" element={<JournalDetailPage />} />
      </Routes>
    </MemoryRouter>,
  );
}

beforeEach(() => {
  vi.mocked(ensureDefaultWorkspace).mockReset();
  vi.mocked(listJournalEntries).mockReset();
  vi.mocked(archiveJournalEntries).mockReset();
  vi.mocked(setJournalEntriesExportEligible).mockReset();
  vi.mocked(createExportJob).mockReset();
  vi.mocked(previewExportSelection).mockReset();
  vi.mocked(exportTemplateCapability).mockReset();
  vi.mocked(listExportJobs).mockReset();
  vi.mocked(getJournalEntry).mockReset();
  vi.mocked(archiveJournalEntry).mockReset();
  vi.mocked(listReferencesForEntry).mockReset();

  vi.mocked(ensureDefaultWorkspace).mockResolvedValue({ id: "w1" } as never);
  vi.mocked(listJournalEntries).mockResolvedValue([alpha(), beta()]);
  vi.mocked(archiveJournalEntries).mockResolvedValue(2);
  vi.mocked(setJournalEntriesExportEligible).mockResolvedValue(1);
  vi.mocked(getJournalEntry).mockResolvedValue(detailEntry() as never);
  vi.mocked(listReferencesForEntry).mockResolvedValue([]);
  vi.mocked(previewExportSelection).mockResolvedValue({
    item_count: 0,
    items: [],
    resolved_selection: {},
    selection_fingerprint: "0123456789abcdef",
    has_therapy_private: false,
  } as never);
  vi.mocked(exportTemplateCapability).mockResolvedValue({
    name: "foundation-journal",
    version: "1.0.1",
    valid: true,
    compiler_version: "typst 0.15.1 (embedded)",
    unavailable_resources: [],
  } as never);
  vi.mocked(listExportJobs).mockResolvedValue([]);
  vi.spyOn(window, "confirm").mockReturnValue(true);
});

afterEach(async () => {
  vi.restoreAllMocks();
  // RTL's auto-cleanup runs after this hook, so the tree can still be
  // mounted: the language event re-renders subscribers and must stay in act.
  await act(async () => {
    await i18n.changeLanguage("en-US");
  });
});

describe.each([...SUPPORTED_LOCALES])("Journal list in %s", (locale) => {
  it("renders localized chrome while authored titles stay verbatim", async () => {
    await i18n.changeLanguage(locale);
    const e = COPY[locale];

    renderPage();

    // The bulk bar, badges, and count status render only after the mocked
    // list resolves; await the first row before asserting row-scoped chrome.
    const alphaBox = await screen.findByRole("checkbox", {
      name: e.selectEntry.replace("{{title}}", "Synthetic alpha"),
    });
    expect(alphaBox).toBeTruthy();
    expect(screen.getByRole("heading", { name: e.title })).toBeTruthy();
    expect(screen.getByRole("link", { name: e.newEntry })).toBeTruthy();
    expect(screen.getByRole("link", { name: e.exportAction })).toBeTruthy();
    expect(screen.getByRole("button", { name: e.selectAll })).toBeTruthy();
    expect(screen.getByRole("button", { name: e.deselectAll })).toBeTruthy();
    expect(screen.getByRole("button", { name: e.setExportable })).toBeTruthy();
    expect(screen.getByRole("button", { name: e.setPrivate })).toBeTruthy();
    expect(screen.getByRole("button", { name: e.archiveSelected })).toBeTruthy();

    // Privacy/exportability badges localize; the authored row title and its
    // verbatim embedding into the checkbox accessible name never translate.
    expect(screen.getByText(e.exportableBadge)).toBeTruthy();
    expect(screen.getByText(e.privateBadge)).toBeTruthy();
    expect(screen.getByText("Synthetic alpha")).toBeTruthy();
    expect(
      screen.getByRole("checkbox", {
        name: e.selectEntry.replace("{{title}}", "Synthetic beta"),
      }),
    ).toBeTruthy();

    // The application-owned occurred-on date-only value renders through the
    // shared Intl date formatter; the raw stored ISO never appears as a label.
    const expectedDate = new Date("2026-10-02T12:00:00").toLocaleDateString(locale, {
      dateStyle: "medium",
    });
    expect(screen.getAllByText(expectedDate)).toHaveLength(2);
    expect(screen.queryByText("2026-10-02")).toBeNull();

    // Count-only status line resolves the locale's plural form.
    expect(screen.getByText(i18n.t("journal.listedCount", { count: 2 }))).toBeTruthy();
  });

  it("confirms, archives, and reports bulk actions in the active locale", async () => {
    await i18n.changeLanguage(locale);

    renderPage();

    fireEvent.click(await screen.findByRole("button", { name: COPY[locale].selectAll }));
    fireEvent.click(screen.getByRole("button", { name: COPY[locale].archiveSelected }));

    expect(window.confirm).toHaveBeenCalledWith(i18n.t("journal.archiveConfirm", { count: 2 }));
    await waitFor(() => expect(archiveJournalEntries).toHaveBeenCalledWith("w1", ["j1", "j2"]));
    expect(await screen.findByText(i18n.t("journal.archived", { count: 2 }))).toBeTruthy();
    // Rows left; the localized empty state shows the authored content was only
    // removed by the native success, never by the locale switch.
    expect(screen.queryByText("Synthetic alpha")).toBeNull();
    expect(screen.getByText(COPY[locale].empty)).toBeTruthy();
  });

  it("reports single-item eligibility feedback in the locale's one form", async () => {
    await i18n.changeLanguage(locale);

    renderPage();

    fireEvent.click(
      await screen.findByRole("checkbox", {
        name: COPY[locale].selectEntry.replace("{{title}}", "Synthetic beta"),
      }),
    );
    fireEvent.click(screen.getByRole("button", { name: COPY[locale].setExportable }));

    await waitFor(() =>
      expect(setJournalEntriesExportEligible).toHaveBeenCalledWith("w1", ["j2"], true),
    );
    expect(await screen.findByText(i18n.t("journal.markedExportable", { count: 1 }))).toBeTruthy();
  });
});

describe.each([...SUPPORTED_LOCALES])("Journal export in %s", (locale) => {
  it("renders the localized export form with verbatim template identifiers", async () => {
    await i18n.changeLanguage(locale);
    const e = COPY[locale];

    renderExportPage();

    expect(await screen.findByRole("heading", { name: e.exportTitle })).toBeTruthy();
    expect(screen.getByText(e.exportFormat)).toBeTruthy();
    expect(screen.getByText(e.exportTemplate)).toBeTruthy();
    expect(screen.getByText(e.exportFrom)).toBeTruthy();
    expect(screen.getByText(e.exportTo)).toBeTruthy();
    expect(screen.getByText(e.exportSelected)).toBeTruthy();
    // The Export-all label is catalog chrome now (the 8.6.10 fix for the
    // previously Spanish-only literal on the English surface).
    expect(await screen.findByLabelText(new RegExp(e.exportAll))).toBeTruthy();
    expect(screen.getByRole("button", { name: e.exportSubmit })).toBeTruthy();
    // Template identifiers are export-template content and stay verbatim.
    expect(screen.getByRole("option", { name: "foundation-journal" })).toBeTruthy();

    // Selected-entry rows Intl-format the application-owned date-only value
    // while the authored entry titles render verbatim.
    const expectedDate = new Date("2026-10-02T12:00:00").toLocaleDateString(locale, {
      dateStyle: "medium",
    });
    expect(await screen.findAllByText(expectedDate)).toHaveLength(2);
    expect(screen.queryByText("2026-10-02")).toBeNull();
    expect(screen.getByText("Synthetic alpha")).toBeTruthy();
    expect(screen.getByText("Synthetic beta")).toBeTruthy();
  });
});

describe.each([...SUPPORTED_LOCALES])("Journal detail in %s", (locale) => {
  it("Intl-formats the occurred-on label and keeps authored content byte-exact", async () => {
    await i18n.changeLanguage(locale);
    const e = COPY[locale];

    renderDetailPage();

    // The application-owned date-only value renders through the shared Intl
    // formatter; the raw stored ISO never appears as a label.
    const expectedDate = new Date("2026-10-02T12:00:00").toLocaleDateString(locale, {
      dateStyle: "medium",
    });
    expect(await screen.findByText(expectedDate)).toBeTruthy();
    expect(screen.queryByText("2026-10-02")).toBeNull();

    // Authored title, structured mood/energy labels, and the Markdown body
    // render verbatim: only the surrounding chrome is localized.
    expect(screen.getByRole("heading", { level: 1, name: "Synthetic alpha" })).toBeTruthy();
    expect(screen.getByText("Synthetic calma · Synthetic alta")).toBeTruthy();
    expect(screen.getByRole("heading", { level: 1, name: "Synthetic quiet body" })).toBeTruthy();
    expect(screen.getByText("prose stays verbatim")).toBeTruthy();
    expect(screen.getByText(e.exportableBadge)).toBeTruthy();

    // Localized detail chrome around the verbatim content; the prev/next
    // links and the reference editor follow the active locale too.
    expect(screen.getByRole("link", { name: e.editAction })).toBeTruthy();
    expect(screen.getByRole("button", { name: e.archiveAction })).toBeTruthy();
    expect(screen.getByRole("link", { name: e.olderEntry })).toBeTruthy();
    expect(await screen.findByText(e.referenceEmpty)).toBeTruthy();
  });
});

describe("Journal list runtime locale switching", () => {
  it("keeps selection and verbatim titles across a language change", async () => {
    renderPage();

    const checkbox = await screen.findByRole("checkbox", { name: "Select Synthetic alpha" });
    fireEvent.click(checkbox);
    expect(screen.getByText("1 selected")).toBeTruthy();

    // The tree is mounted, so the locale event re-renders subscribers
    // inside act to keep React's warning-free contract.
    await act(async () => {
      await i18n.changeLanguage("de-DE");
    });
    await waitFor(() => expect(screen.getByRole("heading", { name: "Tagebuch" })).toBeTruthy());
    // Selection, rows, and authored titles survive the switch untouched; the
    // checkbox accessible name re-renders around the verbatim title.
    expect(screen.getByRole("checkbox", { name: "Synthetic alpha auswählen" })).toBeChecked();
    expect(screen.getByText("1 ausgewählt")).toBeTruthy();
    expect(screen.getByText("Exportierbar")).toBeTruthy();
    expect(screen.getByText("Privat")).toBeTruthy();
    expect(screen.queryByText("Select all")).toBeNull();
    expect(screen.getByRole("button", { name: "Alle auswählen" })).toBeTruthy();

    // The English bulk-action literals are gone and the German ones work.
    fireEvent.click(screen.getByRole("button", { name: "Als privat markieren" }));
    await waitFor(() =>
      expect(setJournalEntriesExportEligible).toHaveBeenCalledWith("w1", ["j1"], false),
    );
    expect(await screen.findByText("1 Tagebucheintrag als Privat markiert.")).toBeTruthy();
  });
});
