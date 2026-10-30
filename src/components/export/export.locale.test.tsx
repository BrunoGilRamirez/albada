import i18n from "i18next";
import { afterEach, describe, expect, it, vi } from "vitest";
import { act, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { ErrorBoundary } from "@/components/ErrorBoundary";
import { ExportHistory } from "@/components/export/ExportHistory";
import ExportJobStatus from "@/components/export/ExportJobStatus";
import ExportPreview from "@/components/export/ExportPreview";
import JobProgressModal, { type JobProgressState } from "@/components/jobs/JobProgressModal";
import { SUPPORTED_LOCALES, type AppLocale } from "@/lib/i18n/locale";
import type { ExportJob, ExportPreviewItem } from "@/lib/types";

vi.mock("@/lib/tauri", () => ({
  reportUnknownError: vi.fn(),
}));

// 8.6.11 Shared export/job/feedback chrome (deferred from 8.6.10): the export
// preview, export job status, previous-exports history, job progress dialog,
// and the error-boundary fallback are catalog-driven in all four locales.
// Item titles, job titles, checksums, sizes, phases, and privacy scopes are
// application-owned data and stay verbatim; therapy content is never surfaced
// beyond the explicit privacy chip. Synthetic fixtures only.

const previewItems: ExportPreviewItem[] = [
  {
    id: "j1",
    entity_type: "journal_entry",
    title: "Synthetic quiet day",
    occurred_on: "2026-09-04",
    privacy_scope: "private",
    content_hash: "abc12345",
  },
  {
    id: "t1",
    entity_type: "therapy_record",
    title: "Synthetic therapy note",
    occurred_on: "2026-09-05",
    privacy_scope: "therapy_private",
    content_hash: "def45678",
  },
];

function job(overrides: Partial<ExportJob> = {}): ExportJob {
  return {
    id: "job-1",
    workspace_id: "w1",
    export_type: "pdf",
    scope_type: "journal",
    title: "Synthetic export",
    template_name: "default",
    status: "completed",
    output_path: null,
    generated_at: "2026-09-06T10:00:00Z",
    error_message: null,
    selection_fingerprint: "fp-1",
    warnings: [],
    created_at: "2026-09-06T10:00:00Z",
    updated_at: "2026-09-06T10:00:05Z",
    artifacts: [],
    ...overrides,
  };
}

function runningState(): JobProgressState {
  return {
    phase: "Running…",
    percent: null,
    status: "running",
    terminal: false,
    error: null,
    recoveryLabel: null,
    onRecovery: vi.fn(),
    onCancelRequest: vi.fn(),
  };
}

interface ExportCopy {
  previewTitle: string;
  itemCountOne: string;
  itemCountOther: string;
  therapyWarning: string;
  fingerprintLabel: string;
  fingerprintHelp: string;
  jobTitle: string;
  failedSourceSaved: string;
  diagnostics: string;
  diagnosticCounts: string;
  copyDiagnostics: string;
  open: string;
  localArtifacts: string;
  kilobytes: string;
  normalizedMd: string;
  severityWarning: string;
  severityError: string;
  historyTitle: string;
  historySubtitle: string;
  historyCountOne: string;
  historyEmpty: string;
  deleteConfirm: string;
  openFormat: string;
  saveCopy: string;
  noArtifact: string;
  jobProgressTitle: string;
  pending: string;
  backgroundNote: string;
  closeAria: string;
  cancel: string;
  errorTitle: string;
  reload: string;
}

const COPY: Record<AppLocale, ExportCopy> = {
  "en-US": {
    previewTitle: "Export preview",
    itemCountOne: "1 item",
    itemCountOther: "2 items",
    therapyWarning: "This selection includes therapy-private content. Confirm before exporting.",
    fingerprintLabel: "Selection fingerprint",
    fingerprintHelp: "Confirms the exact items and content you are exporting.",
    jobTitle: "Export job",
    failedSourceSaved: "The export failed, but the Markdown source was saved.",
    diagnostics: "Diagnostics (2)",
    diagnosticCounts: "1 errors · 1 warnings · 0 info",
    copyDiagnostics: "Copy safe diagnostic summary",
    open: "Open",
    localArtifacts: "Local artifacts (2)",
    kilobytes: "2.0 KB",
    normalizedMd: "Normalized Markdown",
    severityWarning: "Warning",
    severityError: "Error",
    historyTitle: "Previous exports",
    historySubtitle: "Reopen generated files from this workspace.",
    historyCountOne: "1 export",
    historyEmpty: "No exports have been generated yet.",
    deleteConfirm: 'Delete export "Synthetic export" and its files?',
    openFormat: "Open Markdown",
    saveCopy: "Save copy",
    noArtifact: "No output artifact was produced.",
    jobProgressTitle: "Job progress",
    pending: "Pending…",
    backgroundNote: "You can navigate elsewhere; the job continues in the background.",
    closeAria: "Close progress dialog",
    cancel: "Cancel",
    errorTitle: "We could not display this view",
    reload: "Reload application",
  },
  "es-ES": {
    previewTitle: "Vista previa de la exportación",
    itemCountOne: "1 elemento",
    itemCountOther: "2 elementos",
    therapyWarning:
      "Esta selección incluye contenido privado de terapia. Confirma antes de exportar.",
    fingerprintLabel: "Huella de la selección",
    fingerprintHelp: "Confirma los elementos y el contenido exactos que vas a exportar.",
    jobTitle: "Exportación",
    failedSourceSaved: "La exportación falló, pero se guardó el origen Markdown.",
    diagnostics: "Diagnóstico (2)",
    diagnosticCounts: "1 errores · 1 advertencias · 0 información",
    copyDiagnostics: "Copiar resumen seguro del diagnóstico",
    open: "Abrir",
    localArtifacts: "Artefactos locales (2)",
    kilobytes: "2,0 KB",
    normalizedMd: "Markdown normalizado",
    severityWarning: "Advertencia",
    severityError: "Error",
    historyTitle: "Exportaciones anteriores",
    historySubtitle: "Vuelve a abrir los archivos generados desde este espacio de trabajo.",
    historyCountOne: "1 exportación",
    historyEmpty: "Aún no se ha generado ninguna exportación.",
    deleteConfirm: "¿Eliminar la exportación «Synthetic export» y sus archivos?",
    openFormat: "Abrir Markdown",
    saveCopy: "Guardar copia",
    noArtifact: "No se produjo ningún artefacto de salida.",
    jobProgressTitle: "Progreso de la tarea",
    pending: "Pendiente…",
    backgroundNote: "Puedes navegar a otra parte; la tarea continúa en segundo plano.",
    closeAria: "Cerrar el diálogo de progreso",
    cancel: "Cancelar",
    errorTitle: "No pudimos mostrar esta vista",
    reload: "Recargar la aplicación",
  },
  "de-DE": {
    previewTitle: "Exportvorschau",
    itemCountOne: "1 Element",
    itemCountOther: "2 Elemente",
    therapyWarning:
      "Diese Auswahl enthält privat markierte Therapieinhalte. Bestätigen Sie vor dem Export.",
    fingerprintLabel: "Auswahlsignatur",
    fingerprintHelp: "Bestätigt die genauen Elemente und Inhalte, die Sie exportieren.",
    jobTitle: "Exportauftrag",
    failedSourceSaved: "Der Export ist fehlgeschlagen, aber die Markdown-Quelle wurde gespeichert.",
    diagnostics: "Diagnose (2)",
    diagnosticCounts: "1 Fehler · 1 Warnungen · 0 Hinweise",
    copyDiagnostics: "Sichere Diagnosezusammenfassung kopieren",
    open: "Öffnen",
    localArtifacts: "Lokale Artefakte (2)",
    kilobytes: "2,0 KB",
    normalizedMd: "Normalisiertes Markdown",
    severityWarning: "Warnung",
    severityError: "Fehler",
    historyTitle: "Frühere Exporte",
    historySubtitle: "Öffnen Sie Dateien, die in diesem Arbeitsbereich erzeugt wurden, erneut.",
    historyCountOne: "1 Export",
    historyEmpty: "Es wurden noch keine Exporte erzeugt.",
    deleteConfirm: "Export „Synthetic export“ und seine Dateien löschen?",
    openFormat: "Markdown öffnen",
    saveCopy: "Kopie speichern",
    noArtifact: "Es wurde kein Ausgabeartefakt erzeugt.",
    jobProgressTitle: "Auftragsfortschritt",
    pending: "Ausstehend…",
    backgroundNote: "Sie können anderswo navigieren; der Auftrag läuft im Hintergrund weiter.",
    closeAria: "Fortschrittsdialog schließen",
    cancel: "Abbrechen",
    errorTitle: "Diese Ansicht konnte nicht angezeigt werden",
    reload: "App neu laden",
  },
  "fr-FR": {
    previewTitle: "Aperçu de l'export",
    itemCountOne: "1 élément",
    itemCountOther: "2 éléments",
    therapyWarning:
      "Cette sélection contient du contenu de thérapie privé. Confirmez avant d'exporter.",
    fingerprintLabel: "Empreinte de la sélection",
    fingerprintHelp: "Confirme les éléments et le contenu exacts que vous exportez.",
    jobTitle: "Tâche d'export",
    failedSourceSaved: "L'export a échoué, mais la source Markdown a été enregistrée.",
    diagnostics: "Diagnostic (2)",
    diagnosticCounts: "1 erreurs · 1 avertissements · 0 informations",
    copyDiagnostics: "Copier le résumé de diagnostic sûr",
    open: "Ouvrir",
    localArtifacts: "Artefacts locaux (2)",
    kilobytes: "2,0 Ko",
    normalizedMd: "Markdown normalisé",
    severityWarning: "Avertissement",
    severityError: "Erreur",
    historyTitle: "Exports précédents",
    historySubtitle: "Rouvrez les fichiers générés depuis cet espace de travail.",
    historyCountOne: "1 export",
    historyEmpty: "Aucun export n'a encore été généré.",
    deleteConfirm: "Supprimer l'export « Synthetic export » et ses fichiers ?",
    openFormat: "Ouvrir Markdown",
    saveCopy: "Enregistrer une copie",
    noArtifact: "Aucun artefact de sortie n'a été produit.",
    jobProgressTitle: "Progression de la tâche",
    pending: "En attente…",
    backgroundNote: "Vous pouvez naviguer ailleurs ; la tâche continue en arrière-plan.",
    closeAria: "Fermer la boîte de progression",
    cancel: "Annuler",
    errorTitle: "Nous n'avons pas pu afficher cette vue",
    reload: "Recharger l'application",
  },
};

afterEach(async () => {
  vi.restoreAllMocks();
  await act(async () => {
    await i18n.changeLanguage("en-US");
  });
});

describe.each([...SUPPORTED_LOCALES])("ExportPreview in %s", (locale) => {
  it("localizes the preview chrome and pluralizes the item count", async () => {
    await i18n.changeLanguage(locale);
    const e = COPY[locale];

    const { rerender } = render(
      <ExportPreview
        preview={{
          item_count: previewItems.length,
          items: previewItems,
          selection_fingerprint: "0123456789abcdef",
          has_therapy_private: true,
        }}
      />,
    );

    expect(screen.getByText(e.previewTitle)).toBeTruthy();
    expect(screen.getByText(e.itemCountOther)).toBeTruthy();
    expect(screen.getByText(e.therapyWarning)).toBeTruthy();
    expect(screen.getByText(e.fingerprintLabel)).toBeTruthy();
    expect(screen.getByText(e.fingerprintHelp)).toBeTruthy();
    // Compact fingerprints and the item titles stay verbatim.
    expect(screen.getByText("01234567…")).toBeTruthy();
    expect(screen.getByText("Synthetic quiet day")).toBeTruthy();

    // Application-owned date-only values use the shared Intl formatter.
    expect(
      screen.getAllByText(
        new Date("2026-09-04T12:00:00").toLocaleDateString(locale, { dateStyle: "medium" }),
      ),
    ).toHaveLength(1);
    expect(screen.queryByText("2026-09-04")).toBeNull();

    // Single-item preview uses the locale's `one` plural form.
    rerender(
      <ExportPreview
        preview={{
          item_count: 1,
          items: [previewItems[0]],
          selection_fingerprint: "0123456789abcdef",
          has_therapy_private: false,
        }}
      />,
    );
    expect(screen.getByText(e.itemCountOne)).toBeTruthy();
    expect(screen.queryByText(e.therapyWarning)).toBeNull();
  });

  it("shows the localized truncated-selection notice and keeps privacy scopes verbatim", async () => {
    await i18n.changeLanguage(locale);
    const e = COPY[locale];

    const { container } = render(
      <ExportPreview
        preview={{
          item_count: 12,
          total_count: 12,
          truncated: true,
          items: previewItems,
          selection_fingerprint: "0123456789abcdef",
          has_therapy_private: false,
        }}
      />,
    );

    expect(
      screen.getByText(i18n.t("common.exportPreview.largeSelection", { shown: 2, total: 12 })),
    ).toBeTruthy();
    // The "(showing first N)" suffix shares a span with the item count.
    expect(container.textContent).toContain(
      i18n.t("common.exportPreview.truncatedCount", { shown: 2 }),
    );
    // Stored privacy scopes are identifiers, not copy.
    expect(screen.getByText("private")).toBeTruthy();
    expect(screen.getByText("therapy_private")).toBeTruthy();
    expect(screen.getByText(e.fingerprintLabel)).toBeTruthy();
  });
});

describe.each([...SUPPORTED_LOCALES])("ExportJobStatus in %s", (locale) => {
  it("localizes the job chrome, diagnostics, and artifact sizes", async () => {
    await i18n.changeLanguage(locale);
    const e = COPY[locale];

    render(
      <ExportJobStatus
        job={job({
          status: "failed",
          error_message: "Synthetic compile failure.",
          warnings: [
            {
              code: "compile_warning",
              severity: "warning",
              message: "Synthetic warning message.",
            },
            {
              code: "compile_error",
              severity: "error",
              message: "Synthetic compile failure.",
            },
          ],
          artifacts: [
            {
              id: "a1",
              format: "normalized_md",
              path: "C:\\synthetic\\exports\\job-1\\input.md",
              checksum: "aaaabbbbccccdddd",
              size_bytes: 2048,
            },
            {
              id: "a2",
              format: "log",
              path: "C:\\synthetic\\exports\\job-1\\trace.log",
              checksum: null,
              size_bytes: 128,
            },
          ],
        })}
        onOpenArtifact={vi.fn()}
        onSaveCopyArtifact={vi.fn()}
      />,
    );

    expect(screen.getByText(e.jobTitle)).toBeTruthy();
    expect(screen.getByText(i18n.t("common.labels.jobStatus.failed"))).toBeTruthy();
    expect(screen.getByRole("status")).toHaveAttribute(
      "aria-label",
      i18n.t("common.exportJob.ariaLabel", { status: i18n.t("common.labels.jobStatus.failed") }),
    );
    // Representative error copy is localized; native failure text is verbatim.
    expect(screen.getByText(e.failedSourceSaved)).toBeTruthy();
    // Native failure text is verbatim and appears both as the job error and
    // as the matching diagnostic warning.
    expect(screen.getAllByText("Synthetic compile failure.")).toHaveLength(2);

    // The diagnostics heading combines the total with the per-severity counts in
    // one element, so both halves are asserted on its text.
    const headings = screen.getAllByRole("heading", { level: 4 });
    expect(headings[0].textContent).toContain(e.diagnostics);
    expect(headings[0].textContent).toContain(e.diagnosticCounts);
    expect(headings[1].textContent).toContain(e.localArtifacts);
    expect(screen.getByRole("button", { name: e.copyDiagnostics })).toBeTruthy();
    expect(screen.getByText(e.severityWarning)).toBeTruthy();
    expect(screen.getByText(e.severityError)).toBeTruthy();

    expect(screen.getByText(e.localArtifacts)).toBeTruthy();
    expect(screen.getByText(e.normalizedMd)).toBeTruthy();
    // Unknown artifact formats fall back to the generic diagnostic label.
    expect(screen.getByText(i18n.t("common.exportJob.formats.unknown"))).toBeTruthy();
    // One Open/Save-copy pair per artifact.
    expect(screen.getAllByText(e.open)).toHaveLength(2);
    expect(screen.getAllByText(e.saveCopy)).toHaveLength(2);
    expect(screen.getByText(e.kilobytes)).toBeTruthy();
    expect(screen.getByText(i18n.t("common.exportJob.bytes", { size: "128" }))).toBeTruthy();
    // Compact checksum and no raw path.
    expect(screen.getByText("aaaabbbb…")).toBeTruthy();
    expect(screen.queryByText(/synthetic\\exports/)).toBeNull();
  });
});

describe.each([...SUPPORTED_LOCALES])("ExportHistory in %s", (locale) => {
  it("localizes the history chrome and the delete confirmation", async () => {
    await i18n.changeLanguage(locale);
    const e = COPY[locale];
    vi.spyOn(window, "confirm").mockReturnValue(true);
    const onDeleteJob = vi.fn();
    const onOpenArtifact = vi.fn();
    const onSaveCopyArtifact = vi.fn();

    render(
      <ExportHistory
        jobs={[
          job({
            artifacts: [
              {
                id: "a1",
                format: "md",
                path: "C:\\synthetic\\exports\\job-1\\input.md",
                checksum: "aaaabbbbccccdddd",
                size_bytes: 2048,
              },
            ],
          }),
        ]}
        onOpenArtifact={onOpenArtifact}
        onSaveCopyArtifact={onSaveCopyArtifact}
        onDeleteJob={onDeleteJob}
      />,
    );

    expect(screen.getByText(e.historyTitle)).toBeTruthy();
    expect(screen.getByText(e.historySubtitle)).toBeTruthy();
    // Count-only plural text for a single previous export.
    expect(screen.getByText(e.historyCountOne)).toBeTruthy();
    expect(screen.getByText(e.openFormat)).toBeTruthy();
    expect(screen.getByText(e.saveCopy)).toBeTruthy();
    // The created-at/status line is one paragraph, so it is asserted on the
    // element text rather than as a single text node.
    const meta = screen.getByText("Synthetic export").closest("article");
    expect(meta?.textContent).toContain(i18n.t("common.labels.jobStatus.completed"));
    // The job title is caller-owned export metadata and stays verbatim.
    expect(screen.getByText("Synthetic export")).toBeTruthy();

    fireEvent.click(screen.getByRole("button", { name: i18n.t("common.actions.delete") }));
    expect(window.confirm).toHaveBeenCalledWith(e.deleteConfirm);
    fireEvent.click(screen.getByRole("button", { name: e.openFormat }));
    expect(onOpenArtifact).toHaveBeenCalled();
    fireEvent.click(screen.getByRole("button", { name: e.saveCopy }));
    expect(onSaveCopyArtifact).toHaveBeenCalled();
  });

  it("localizes the empty state and the missing-artifact notice", async () => {
    await i18n.changeLanguage(locale);
    const e = COPY[locale];

    const { rerender } = render(
      <ExportHistory
        jobs={[]}
        onOpenArtifact={vi.fn()}
        onSaveCopyArtifact={vi.fn()}
        onDeleteJob={vi.fn()}
      />,
    );
    expect(screen.getByText(e.historyEmpty)).toBeTruthy();

    rerender(
      <ExportHistory
        jobs={[job()]}
        onOpenArtifact={vi.fn()}
        onSaveCopyArtifact={vi.fn()}
        onDeleteJob={vi.fn()}
      />,
    );
    expect(screen.getByText(e.noArtifact)).toBeTruthy();
    // Two jobs would pluralize the count; one job keeps the `one` form.
    expect(screen.getByText(e.historyCountOne)).toBeTruthy();
  });
});

describe.each([...SUPPORTED_LOCALES])("JobProgressModal in %s", (locale) => {
  it("localizes the shared progress chrome around the caller-owned phase", async () => {
    await i18n.changeLanguage(locale);
    const e = COPY[locale];
    const state = runningState();
    const onClose = vi.fn();

    render(<JobProgressModal state={state} onClose={onClose} />);

    expect(screen.getByText(e.jobProgressTitle)).toBeTruthy();
    expect(screen.getByRole("status")).toHaveAttribute(
      "aria-label",
      i18n.t("common.jobProgress.ariaLabel", { phase: state.phase }),
    );
    expect(screen.getByText(state.phase)).toBeTruthy();
    expect(screen.getByText(e.pending)).toBeTruthy();
    expect(screen.getByText(e.backgroundNote)).toBeTruthy();
    expect(screen.getByText(i18n.t("common.jobProgress.cancelNote"))).toBeTruthy();
    expect(screen.getByLabelText(e.closeAria)).toBeTruthy();
    expect(screen.getByRole("button", { name: e.cancel })).toBeTruthy();

    fireEvent.click(screen.getByRole("button", { name: e.cancel }));
    expect(state.onCancelRequest).toHaveBeenCalledTimes(1);
    expect(onClose).toHaveBeenCalledTimes(1);

    fireEvent.click(screen.getByLabelText(e.closeAria));
    expect(state.onCancelRequest).toHaveBeenCalledTimes(1);
    expect(onClose).toHaveBeenCalledTimes(2);
  });

  it("formats the determinate percentage through the locale number formatter", async () => {
    await i18n.changeLanguage(locale);
    const state = { ...runningState(), percent: 42 };

    render(<JobProgressModal state={state} onClose={vi.fn()} />);

    const percent = new Intl.NumberFormat(locale).format(42);
    expect(screen.getByText(i18n.t("common.jobProgress.percent", { percent }))).toBeTruthy();
    expect(screen.getByText(i18n.t("common.jobProgress.percentMax"))).toBeTruthy();
  });
});

describe.each([...SUPPORTED_LOCALES])("ErrorBoundary in %s", (locale) => {
  it("localizes the fallback chrome and the representative error message", async () => {
    await i18n.changeLanguage(locale);
    const e = COPY[locale];
    const consoleSpy = vi.spyOn(console, "error").mockImplementation(() => {});

    try {
      function Boom(): never {
        throw new Error("render exploded");
      }

      render(
        <ErrorBoundary>
          <Boom />
        </ErrorBoundary>,
      );

      expect(screen.getByRole("heading", { name: e.errorTitle })).toBeTruthy();
      expect(screen.getByRole("button", { name: e.reload })).toBeTruthy();
      // A stable native error code resolves through the localized catalog.
      function BoomWithCode(): never {
        throw { code: "IO", message: "ignored" };
      }
      render(
        <ErrorBoundary>
          <BoomWithCode />
        </ErrorBoundary>,
      );
      await waitFor(() =>
        expect(screen.getAllByText(i18n.t("errors.IO")).length).toBeGreaterThan(0),
      );
      // The product name stays verbatim.
      expect(screen.getAllByText("Albada").length).toBeGreaterThan(0);
    } finally {
      consoleSpy.mockRestore();
    }
  });
});

describe("Shared export chrome runtime locale switching", () => {
  it("re-renders the chrome while exported data stays byte-exact", async () => {
    render(
      <ExportPreview
        preview={{
          item_count: previewItems.length,
          items: previewItems,
          selection_fingerprint: "0123456789abcdef",
          has_therapy_private: true,
        }}
      />,
    );

    expect(screen.getByText("Export preview")).toBeTruthy();
    expect(screen.getByText("Synthetic quiet day")).toBeTruthy();
    expect(screen.getByText("2 items")).toBeTruthy();

    await act(async () => {
      await i18n.changeLanguage("de-DE");
    });

    await waitFor(() => expect(screen.getByText("Exportvorschau")).toBeTruthy());
    expect(screen.queryByText("Export preview")).toBeNull();
    expect(screen.getByText("2 Elemente")).toBeTruthy();
    // Item titles, privacy scopes, and fingerprints are never rewritten.
    expect(screen.getByText("Synthetic quiet day")).toBeTruthy();
    expect(screen.getByText("therapy_private")).toBeTruthy();
    expect(screen.getByText("01234567…")).toBeTruthy();
  });
});
