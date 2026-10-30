import i18n from "i18next";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { act, fireEvent, render, screen, waitFor } from "@testing-library/react";
import ImportSettingsPage from "@/app/settings/import/page";
import {
  commitLegacyImport,
  getImportBatch,
  getImportBatchStatus,
  previewLegacyImport,
  startLegacyImport,
} from "@/lib/api/importApi";
import { ensureDefaultWorkspace } from "@/lib/api/workspaceApi";
import { startImportPoll } from "@/lib/api/jobApi";
import { SUPPORTED_LOCALES, type AppLocale } from "@/lib/i18n/locale";

vi.mock("@/lib/api/importApi", () => ({
  previewLegacyImport: vi.fn(),
  startLegacyImport: vi.fn(),
  commitLegacyImport: vi.fn(),
  getImportBatchStatus: vi.fn(),
  getImportBatch: vi.fn(),
}));

// The job-progress modal opens from the commit path; the native poller is
// replaced by a controllable stub so the test can drive a running (non
// terminal) progress state.
vi.mock("@/lib/api/jobApi", () => ({
  startImportPoll: vi.fn(),
}));

vi.mock("@/lib/api/workspaceApi", () => ({
  ensureDefaultWorkspace: vi.fn(),
}));

// 8.6.11 Legacy-import locale coverage: the import form chrome, the preview
// fingerprint hint, the shared job-progress chrome, and the report view
// (summary, warnings, review decisions) all follow the active locale. The
// legacy source root, source paths, snippets, and fingerprints are imported
// data and stay verbatim. Synthetic fixtures only.

const SOURCE_ROOT = "C:\\legacy\\proyecto-vida";

const report = {
  batch_id: "batch-1",
  source_root: SOURCE_ROOT,
  status: "completed",
  started_at: "2026-09-06T10:00:00Z",
  completed_at: "2026-09-06T10:00:05Z",
  counts: [{ entity_type: "task", count: 12 }],
  warnings: [
    {
      severity: "warning",
      source_path: "records/2026/q1/planning/goals.md",
      line: 4,
      message: "Synthetic filename date mismatch.",
    },
  ],
  review_items: [
    {
      id: "item-1",
      source_path: "journal/entries/2026-06-25-raw.md",
      proposed_entity_type: "journal_entry",
      title: "Synthetic raw entry",
      snippet: "Synthetic snippet line",
      confidence: "review",
      source_hash: "ab".repeat(32),
    },
  ],
  preview_fingerprint: "fp-1",
};

const emptyReport = { ...report, counts: [], warnings: [], review_items: [] };

interface ImportCopy {
  title: string;
  sourceRoot: string;
  fullScan: string;
  includePaths: string;
  includePathsHelp: string;
  previewSubmit: string;
  fingerprintLabel: string;
  commitSubmit: string;
  needDecisions: string;
  runSubmit: string;
  importing: string;
  reportTitle: string;
  summary: string;
  warnings: string;
  showWarnings: string;
  reviewItems: string;
  decision: string;
  decisionSelect: string;
  journalEntry: string;
  needsReview: string;
  ignore: string;
}

const COPY: Record<AppLocale, ImportCopy> = {
  "en-US": {
    title: "Legacy import",
    sourceRoot: "Legacy repository root path",
    fullScan: "Import the full repository",
    includePaths: "Include only these paths (one per line, relative to the repository root)",
    includePathsHelp:
      "Only the listed subtrees are read. Leave the checkbox above unchecked to restrict the import, or check it to scan the whole repository.",
    previewSubmit: "Preview import",
    fingerprintLabel: "Fingerprint:",
    commitSubmit: "Review decisions and commit",
    needDecisions: "Choose a decision for every review item before committing.",
    runSubmit: "Import without review",
    importing: "Importing…",
    reportTitle: "Import report",
    summary: "Summary",
    warnings: "Warnings",
    showWarnings: "Show warnings (1)",
    reviewItems: "Items to review (1)",
    decision: "Decision",
    decisionSelect: "Select…",
    journalEntry: "Journal entry",
    needsReview: "Needs review",
    ignore: "Ignore",
  },
  "es-ES": {
    title: "Importación heredada",
    sourceRoot: "Ruta raíz del repositorio heredado",
    fullScan: "Importar el repositorio completo",
    includePaths: "Importar solo estas rutas (una por línea, relativas a la raíz del repositorio)",
    includePathsHelp:
      "Solo se leen los subárboles indicados. Deja sin marcar la casilla de arriba para restringir la importación, o márcala para explorar todo el repositorio.",
    previewSubmit: "Previsualizar importación",
    fingerprintLabel: "Huella:",
    commitSubmit: "Revisar decisiones y confirmar",
    needDecisions:
      "Elige una decisión para cada elemento pendiente de revisión antes de confirmar.",
    runSubmit: "Importar sin revisión",
    importing: "Importando…",
    reportTitle: "Informe de importación",
    summary: "Resumen",
    warnings: "Advertencias",
    showWarnings: "Mostrar advertencias (1)",
    reviewItems: "Elementos para revisar (1)",
    decision: "Decisión",
    decisionSelect: "Seleccionar…",
    journalEntry: "Entrada de diario",
    needsReview: "Requiere revisión",
    ignore: "Ignorar",
  },
  "de-DE": {
    title: "Legacy-Import",
    sourceRoot: "Pfad zur Wurzel des Legacy-Repositorys",
    fullScan: "Ganzes Repository importieren",
    includePaths: "Nur diese Pfade importieren (einer pro Zeile, relativ zur Repositorywurzel)",
    includePathsHelp:
      "Es werden nur die aufgeführten Teilbäume gelesen. Lassen Sie das Kontrollkästchen oben deaktiviert, um den Import einzuschränken, oder aktivieren Sie es, um das ganze Repository zu durchsuchen.",
    previewSubmit: "Importvorschau erstellen",
    fingerprintLabel: "Signatur:",
    commitSubmit: "Entscheidungen prüfen und übernehmen",
    needDecisions: "Wählen Sie für jeden Prüfpunkt eine Entscheidung, bevor Sie übernehmen.",
    runSubmit: "Ohne Prüfung importieren",
    importing: "Wird importiert…",
    reportTitle: "Importbericht",
    summary: "Zusammenfassung",
    warnings: "Warnungen",
    showWarnings: "Warnungen anzeigen (1)",
    reviewItems: "Zu prüfende Elemente (1)",
    decision: "Entscheidung",
    decisionSelect: "Auswählen…",
    journalEntry: "Tagebucheintrag",
    needsReview: "Prüfung erforderlich",
    ignore: "Ignorieren",
  },
  "fr-FR": {
    title: "Importation héritée",
    sourceRoot: "Chemin de la racine du dépôt hérité",
    fullScan: "Importer tout le dépôt",
    includePaths: "Importer uniquement ces chemins (un par ligne, relatifs à la racine du dépôt)",
    includePathsHelp:
      "Seules les sous-arborescences listées sont lues. Décochez la case ci-dessus pour restreindre l'importation, ou cochez-la pour analyser tout le dépôt.",
    previewSubmit: "Prévisualiser l'importation",
    fingerprintLabel: "Empreinte :",
    commitSubmit: "Vérifier les décisions et valider",
    needDecisions: "Choisissez une décision pour chaque élément à vérifier avant de valider.",
    runSubmit: "Importer sans vérification",
    importing: "Importation…",
    reportTitle: "Rapport d'importation",
    summary: "Résumé",
    warnings: "Avertissements",
    showWarnings: "Afficher les avertissements (1)",
    reviewItems: "Éléments à vérifier (1)",
    decision: "Décision",
    decisionSelect: "Sélectionner…",
    journalEntry: "Entrée de journal",
    needsReview: "À vérifier",
    ignore: "Ignorer",
  },
};

function renderPage() {
  return render(<ImportSettingsPage />);
}

beforeEach(() => {
  vi.mocked(previewLegacyImport).mockReset();
  vi.mocked(startLegacyImport).mockReset();
  vi.mocked(commitLegacyImport).mockReset();
  vi.mocked(getImportBatchStatus).mockReset();
  vi.mocked(getImportBatch).mockReset();
  vi.mocked(ensureDefaultWorkspace).mockReset();
  vi.mocked(startImportPoll).mockReset();
  // Non-terminal running state: the shared modal keeps its pending spinner and
  // background-notice copy.
  vi.mocked(startImportPoll).mockImplementation((_batchId, onProgress) => {
    onProgress({
      phase: null,
      percent: null,
      status: "running",
      terminal: false,
      error: null,
      recoveryLabel: null,
    });
    return vi.fn();
  });
  vi.mocked(previewLegacyImport).mockResolvedValue(emptyReport as never);
  vi.mocked(ensureDefaultWorkspace).mockResolvedValue({ id: "w1" } as never);
  vi.mocked(commitLegacyImport).mockResolvedValue(report as never);
});

afterEach(async () => {
  await act(async () => {
    await i18n.changeLanguage("en-US");
  });
});

describe.each([...SUPPORTED_LOCALES])("ImportSettingsPage in %s", (locale) => {
  it("renders the import form chrome in the active locale", async () => {
    await i18n.changeLanguage(locale);
    const e = COPY[locale];

    renderPage();

    expect(await screen.findByRole("heading", { level: 1, name: e.title })).toBeTruthy();
    expect(screen.getByLabelText(e.sourceRoot)).toBeTruthy();
    expect(screen.getByLabelText(e.fullScan)).toBeChecked();
    expect(screen.queryByLabelText(e.includePaths)).toBeNull();
    expect(screen.getByRole("button", { name: e.previewSubmit })).toBeTruthy();

    // Restricting the scan reveals the translated include-paths editor.
    fireEvent.click(screen.getByLabelText(e.fullScan));
    expect(screen.getByLabelText(e.includePaths)).toBeTruthy();
    expect(screen.getByText(e.includePathsHelp)).toBeTruthy();
  });

  it("requires the repository root with localized validation copy", async () => {
    await i18n.changeLanguage(locale);

    renderPage();

    fireEvent.click(await screen.findByRole("button", { name: COPY[locale].previewSubmit }));

    expect(await screen.findByRole("alert")).toHaveTextContent(i18n.t("settings.import.needPath"));
    expect(previewLegacyImport).not.toHaveBeenCalled();
  });

  it("renders the preview fingerprint hint and the report view in the locale", async () => {
    await i18n.changeLanguage(locale);
    const e = COPY[locale];
    vi.mocked(previewLegacyImport).mockResolvedValue(report as never);

    renderPage();

    fireEvent.change(await screen.findByLabelText(e.sourceRoot), {
      target: { value: SOURCE_ROOT },
    });
    fireEvent.click(screen.getByRole("button", { name: e.previewSubmit }));

    // The fingerprint label is chrome; the fingerprint value stays verbatim.
    expect(await screen.findByText(e.fingerprintLabel)).toBeTruthy();
    expect(screen.getByText("fp-1")).toBeTruthy();

    expect(screen.getByText(e.reportTitle)).toBeTruthy();
    expect(screen.getByText(e.summary)).toBeTruthy();
    expect(screen.getByText(e.warnings)).toBeTruthy();
    expect(screen.getByRole("button", { name: e.showWarnings })).toBeTruthy();
    expect(screen.getByText(e.reviewItems)).toBeTruthy();
    expect(screen.getByText(e.decision)).toBeTruthy();
    expect(screen.getByRole("option", { name: e.decisionSelect })).toBeTruthy();
    expect(screen.getByRole("option", { name: e.ignore })).toBeTruthy();
    expect(screen.getByRole("option", { name: e.journalEntry })).toBeTruthy();
    // The proposed-entity badge and the decision option share the same label.
    expect(screen.getAllByText(e.journalEntry).length).toBeGreaterThan(0);
    expect(screen.getByText(e.needsReview)).toBeTruthy();
    expect(
      screen.getByLabelText(i18n.t("import.report.decisionAria", { title: "Synthetic raw entry" })),
    ).toBeTruthy();

    // Imported content (source root, path, snippet, fingerprint) is verbatim.
    expect(screen.getByText(SOURCE_ROOT)).toBeTruthy();
    expect(screen.getByText("journal/entries/2026-06-25-raw.md")).toBeTruthy();
    expect(screen.getByText(/Synthetic snippet line/)).toBeTruthy();
  });

  it("blocks the commit until every review item has a localized decision", async () => {
    await i18n.changeLanguage(locale);
    const e = COPY[locale];
    vi.mocked(previewLegacyImport).mockResolvedValue(report as never);

    renderPage();

    fireEvent.change(await screen.findByLabelText(e.sourceRoot), {
      target: { value: SOURCE_ROOT },
    });
    fireEvent.click(screen.getByRole("button", { name: e.previewSubmit }));

    const commitButton = await screen.findByRole("button", { name: e.commitSubmit });
    expect(commitButton).toBeDisabled();
    expect(screen.getByText(e.needDecisions)).toBeTruthy();

    fireEvent.change(
      screen.getByLabelText(i18n.t("import.report.decisionAria", { title: "Synthetic raw entry" })),
      { target: { value: "journal_entry" } },
    );
    expect(commitButton).toBeEnabled();

    fireEvent.click(commitButton);
    await waitFor(() =>
      expect(commitLegacyImport).toHaveBeenCalledWith(
        expect.objectContaining({
          preview_fingerprint: "fp-1",
          decisions: [{ review_item_id: "item-1", entity_type: "journal_entry" }],
        }),
      ),
    );
  });

  it("shows the shared job-progress chrome in the locale", async () => {
    await i18n.changeLanguage(locale);
    const e = COPY[locale];
    vi.mocked(previewLegacyImport).mockResolvedValue(report as never);

    renderPage();

    fireEvent.change(await screen.findByLabelText(e.sourceRoot), {
      target: { value: SOURCE_ROOT },
    });
    fireEvent.click(screen.getByRole("button", { name: e.previewSubmit }));
    const commitButton = await screen.findByRole("button", { name: e.commitSubmit });
    fireEvent.change(
      screen.getByLabelText(i18n.t("import.report.decisionAria", { title: "Synthetic raw entry" })),
      { target: { value: "journal_entry" } },
    );
    fireEvent.click(commitButton);

    // The commit opens the shared progress modal with localized chrome.
    expect(await screen.findByText(i18n.t("common.jobProgress.title"))).toBeTruthy();
    expect(screen.getByText(i18n.t("common.jobProgress.pending"))).toBeTruthy();
    expect(screen.getByText(i18n.t("common.jobProgress.backgroundNote"))).toBeTruthy();
    expect(screen.getByLabelText(i18n.t("common.jobProgress.closeAria"))).toBeTruthy();
    expect(screen.getByRole("button", { name: e.importing })).toBeTruthy();
  });

  it("reports an incomplete batch with localized copy", async () => {
    await i18n.changeLanguage(locale);
    const e = COPY[locale];
    vi.mocked(previewLegacyImport).mockResolvedValue(emptyReport as never);
    vi.mocked(startLegacyImport).mockResolvedValue({
      batch_id: "batch-1",
      status: "running",
      started_at: report.started_at,
      completed_at: null,
    });
    vi.mocked(getImportBatchStatus).mockResolvedValue({
      batch_id: "batch-1",
      status: "failed",
      started_at: report.started_at,
      completed_at: null,
    });

    renderPage();

    fireEvent.change(await screen.findByLabelText(e.sourceRoot), {
      target: { value: SOURCE_ROOT },
    });
    fireEvent.click(screen.getByRole("button", { name: e.previewSubmit }));
    await screen.findByRole("button", { name: e.runSubmit });
    fireEvent.click(screen.getByRole("button", { name: e.runSubmit }));

    expect(await screen.findByRole("alert")).toHaveTextContent(
      i18n.t("settings.import.notComplete"),
    );
    // The batch status line renders the localized status label.
    expect(
      screen.getByText(
        i18n.t("settings.import.statusLine", {
          status: i18n.t("common.labels.jobStatus.failed"),
        }),
      ),
    ).toBeTruthy();
  });

  it("appends the localized stale-fingerprint hint to the command error", async () => {
    await i18n.changeLanguage(locale);
    const e = COPY[locale];
    vi.mocked(previewLegacyImport).mockResolvedValue(report as never);
    vi.mocked(commitLegacyImport).mockRejectedValue({
      message: "The selection changed since it was previewed.",
    });

    renderPage();

    fireEvent.change(await screen.findByLabelText(e.sourceRoot), {
      target: { value: SOURCE_ROOT },
    });
    fireEvent.click(screen.getByRole("button", { name: e.previewSubmit }));
    const commitButton = await screen.findByRole("button", { name: e.commitSubmit });
    fireEvent.change(
      screen.getByLabelText(i18n.t("import.report.decisionAria", { title: "Synthetic raw entry" })),
      { target: { value: "journal_entry" } },
    );
    fireEvent.click(commitButton);

    const staleMessage = "The selection changed since it was previewed.";
    expect(await screen.findByRole("alert")).toHaveTextContent(
      i18n.t("settings.import.staleHint", { message: staleMessage }),
    );
  });
});

describe("ImportSettingsPage runtime locale switching", () => {
  it("re-renders the import chrome in the new language", async () => {
    vi.mocked(previewLegacyImport).mockResolvedValue(report as never);
    renderPage();

    fireEvent.change(await screen.findByLabelText("Legacy repository root path"), {
      target: { value: SOURCE_ROOT },
    });
    fireEvent.click(screen.getByRole("button", { name: "Preview import" }));
    await screen.findByText("Import report");

    await act(async () => {
      await i18n.changeLanguage("de-DE");
    });

    await waitFor(() =>
      expect(screen.getByRole("heading", { level: 1, name: "Legacy-Import" })).toBeTruthy(),
    );
    expect(screen.getByLabelText("Pfad zur Wurzel des Legacy-Repositorys")).toBeTruthy();
    expect(screen.getByText("Importbericht")).toBeTruthy();
    expect(screen.queryByRole("button", { name: "Preview import" })).toBeNull();
    // The typed path and the imported review title are untouched.
    expect(
      (screen.getByLabelText("Pfad zur Wurzel des Legacy-Repositorys") as HTMLInputElement).value,
    ).toBe(SOURCE_ROOT);
    expect(screen.getByText("Synthetic raw entry")).toBeTruthy();
  });
});
