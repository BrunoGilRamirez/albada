import i18n from "i18next";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { act, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { MemoryRouter, Route, Routes } from "react-router";
import TherapyPage from "@/app/therapy/page";
import TherapyCollectionPage from "@/app/therapy/collections/[id]/page";
import TherapyRecordDetailPage from "@/app/therapy/records/[id]/page";
import TherapyExportPage from "@/app/therapy/export/page";
import JournalPage from "@/app/journal/page";
import {
  archiveTherapyCollection,
  archiveTherapyRecord,
  createTherapyCollection,
  getTherapyRecord,
  listTherapyCollections,
  listTherapyRecords,
  updateTherapyCollection,
} from "@/lib/api/therapyApi";
import { listJournalEntries } from "@/lib/api/journalApi";
import { listReferencesForRecord } from "@/lib/api/therapyReferenceApi";
import {
  createExportJob,
  exportTemplateCapability,
  previewExportSelection,
  saveExportArtifactCopy,
} from "@/lib/api/exportApi";
import { ensureDefaultWorkspace } from "@/lib/api/workspaceApi";
import { SUPPORTED_LOCALES, type AppLocale } from "@/lib/i18n/locale";

vi.mock("@/lib/api/therapyApi", () => ({
  archiveTherapyCollection: vi.fn(),
  archiveTherapyRecord: vi.fn(),
  createTherapyCollection: vi.fn(),
  getTherapyRecord: vi.fn(),
  listTherapyCollections: vi.fn(),
  listTherapyRecords: vi.fn(),
  updateTherapyCollection: vi.fn(),
}));

vi.mock("@/lib/api/therapyReferenceApi", () => ({
  addTherapyReference: vi.fn(),
  listReferencesForRecord: vi.fn(),
  removeTherapyReference: vi.fn(),
}));

vi.mock("@/lib/api/exportApi", () => ({
  createExportJob: vi.fn(),
  exportTemplateCapability: vi.fn(),
  previewExportSelection: vi.fn(),
  saveExportArtifactCopy: vi.fn(),
}));

vi.mock("@/lib/api/journalApi", () => ({
  listJournalEntries: vi.fn(),
  archiveJournalEntries: vi.fn(),
  setJournalEntriesExportEligible: vi.fn(),
}));

vi.mock("@/lib/api/workspaceApi", () => ({
  ensureDefaultWorkspace: vi.fn(),
}));

// 8.6.10 Therapy page locale coverage: the collections surface (banner,
// isolation note, create/edit/archive actions, and empty state) renders from
// the active catalog; user-authored collection names stay verbatim across
// runtime language changes; and therapy UI copy never appears on the Journal
// surface. Synthetic fixtures only — no real therapy content.

const collection = {
  id: "c1",
  workspace_id: "w1",
  name: "Coping toolkit",
  privacy_scope: "therapy_private" as const,
  encryption_status: "at_rest",
  created_at: "2026-10-13T09:00:00Z",
  updated_at: "2026-10-13T09:00:00Z",
  archived_at: null,
};

/**
 * Record fixture with an authored title and Markdown body plus a date-only
 * `occurred_on`. The read-only date labels render through the Intl formatter
 * in every locale; the authored values always stay verbatim.
 */
const therapyRecord = {
  id: "tr1",
  therapy_collection_id: "c1",
  record_type: "note" as const,
  title: "Grounding note",
  content_markdown: "4-7-8 breathing stays verbatim.",
  occurred_on: "2026-10-05",
  status: "open" as const,
  privacy_scope: "therapy_private" as const,
  created_at: "2026-10-05T09:00:00Z",
  updated_at: "2026-10-05T09:00:00Z",
  archived_at: null,
};

interface TherapyPageCopy {
  title: string;
  banner: string;
  isolationNote: string;
  newCollection: string;
  namePlaceholder: string;
  create: string;
  collections: string;
  collectionsEmpty: string;
  edit: string;
  save: string;
  cancel: string;
  archive: string;
  confirmArchive: string;
  exportAction: string;
  journalHeading: string;
  addRecord: string;
  recordTypeNote: string;
  statusOpen: string;
  referenceTasksEmpty: string;
  exportTitle: string;
  exportNoDate: string;
}

const COPY: Record<AppLocale, TherapyPageCopy> = {
  "en-US": {
    title: "Therapy",
    banner:
      "Privacy mode: therapy records are kept separate from the executive dashboard and are not included in generic exports.",
    isolationNote:
      "This area is kept separate from the executive dashboard. Therapy content is never shown in task views or included in a generic project export.",
    newCollection: "New collection",
    namePlaceholder: "Collection name",
    create: "Create",
    collections: "Collections",
    collectionsEmpty: "No therapy collections yet.",
    edit: "Edit",
    save: "Save",
    cancel: "Cancel",
    archive: "Archive",
    confirmArchive: "Confirm archive",
    exportAction: "Export",
    journalHeading: "Journal",
    addRecord: "Add record",
    recordTypeNote: "Note",
    statusOpen: "Open",
    referenceTasksEmpty: "No linked tasks yet.",
    exportTitle: "Export therapy records",
    exportNoDate: "no date",
  },
  "es-ES": {
    title: "Terapia",
    banner:
      "Modo de privacidad: los registros de terapia se mantienen separados del panel ejecutivo y no se incluyen en exportaciones genéricas.",
    isolationNote:
      "Esta área se mantiene separada del panel ejecutivo. El contenido de terapia nunca se muestra en las vistas de tareas ni se incluye en una exportación genérica del proyecto.",
    newCollection: "Nueva colección",
    namePlaceholder: "Nombre de la colección",
    create: "Crear",
    collections: "Colecciones",
    collectionsEmpty: "Aún no hay colecciones de terapia.",
    edit: "Editar",
    save: "Guardar",
    cancel: "Cancelar",
    archive: "Archivar",
    confirmArchive: "Confirmar archivado",
    exportAction: "Exportar",
    journalHeading: "Diario",
    addRecord: "Añadir registro",
    recordTypeNote: "Nota",
    statusOpen: "Abierto",
    referenceTasksEmpty: "Aún no hay tareas vinculadas.",
    exportTitle: "Exportar registros de terapia",
    exportNoDate: "sin fecha",
  },
  "de-DE": {
    title: "Therapie",
    banner:
      "Privatsphäre-Modus: Therapienotizen bleiben vom Executive-Dashboard getrennt und werden nicht in allgemeine Exporte aufgenommen.",
    isolationNote:
      "Dieser Bereich bleibt vom Executive-Dashboard getrennt. Therapieinhalte erscheinen nie in Aufgabenansichten und werden nie in einen allgemeinen Projektexport aufgenommen.",
    newCollection: "Neue Sammlung",
    namePlaceholder: "Name der Sammlung",
    create: "Erstellen",
    collections: "Sammlungen",
    collectionsEmpty: "Noch keine Therapie-Sammlungen.",
    edit: "Bearbeiten",
    save: "Speichern",
    cancel: "Abbrechen",
    archive: "Archivieren",
    confirmArchive: "Archivieren bestätigen",
    exportAction: "Exportieren",
    journalHeading: "Tagebuch",
    addRecord: "Notiz hinzufügen",
    recordTypeNote: "Notiz",
    statusOpen: "Offen",
    referenceTasksEmpty: "Noch keine verknüpften Aufgaben.",
    exportTitle: "Therapienotizen exportieren",
    exportNoDate: "ohne Datum",
  },
  "fr-FR": {
    title: "Thérapie",
    banner:
      "Mode confidentialité : les dossiers de thérapie restent séparés du tableau de bord exécutif et ne sont pas inclus dans les exports génériques.",
    isolationNote:
      "Cette zone reste séparée du tableau de bord exécutif. Le contenu de thérapie n'apparaît jamais dans les vues de tâches et n'est jamais inclus dans un export générique du projet.",
    newCollection: "Nouvelle collection",
    namePlaceholder: "Nom de la collection",
    create: "Créer",
    collections: "Collections",
    collectionsEmpty: "Aucune collection de thérapie pour le moment.",
    edit: "Modifier",
    save: "Enregistrer",
    cancel: "Annuler",
    archive: "Archiver",
    confirmArchive: "Confirmer l'archivage",
    exportAction: "Exporter",
    journalHeading: "Journal",
    addRecord: "Ajouter un dossier",
    recordTypeNote: "Note",
    statusOpen: "Ouvert",
    referenceTasksEmpty: "Aucune tâche liée pour le moment.",
    exportTitle: "Exporter les dossiers de thérapie",
    exportNoDate: "sans date",
  },
};

function renderTherapyPage() {
  return render(
    <MemoryRouter initialEntries={["/therapy"]}>
      <Routes>
        <Route path="/therapy" element={<TherapyPage />} />
      </Routes>
    </MemoryRouter>,
  );
}

function renderJournalPage() {
  return render(
    <MemoryRouter initialEntries={["/journal"]}>
      <Routes>
        <Route path="/journal" element={<JournalPage />} />
      </Routes>
    </MemoryRouter>,
  );
}

function renderCollectionPage() {
  return render(
    <MemoryRouter initialEntries={["/therapy/collections/c1"]}>
      <Routes>
        {/* The page reads useParams<{ id: string }>() (file route [id]), so the
            test route param must be named :id, not :collectionId. */}
        <Route path="/therapy/collections/:id" element={<TherapyCollectionPage />} />
      </Routes>
    </MemoryRouter>,
  );
}

function renderRecordPage() {
  return render(
    <MemoryRouter initialEntries={["/therapy/records/tr1"]}>
      <Routes>
        {/* Same as the collection helper: useParams<{ id: string }>() requires
            the route param to be named :id. */}
        <Route path="/therapy/records/:id" element={<TherapyRecordDetailPage />} />
      </Routes>
    </MemoryRouter>,
  );
}

function renderTherapyExportPage() {
  return render(
    <MemoryRouter initialEntries={["/therapy/export"]}>
      <Routes>
        <Route path="/therapy/export" element={<TherapyExportPage />} />
      </Routes>
    </MemoryRouter>,
  );
}

beforeEach(() => {
  vi.mocked(ensureDefaultWorkspace).mockReset();
  vi.mocked(listTherapyCollections).mockReset();
  vi.mocked(createTherapyCollection).mockReset();
  vi.mocked(updateTherapyCollection).mockReset();
  vi.mocked(archiveTherapyCollection).mockReset();
  vi.mocked(listJournalEntries).mockReset();
  vi.mocked(listTherapyRecords).mockReset();
  vi.mocked(getTherapyRecord).mockReset();
  vi.mocked(archiveTherapyRecord).mockReset();
  vi.mocked(listReferencesForRecord).mockReset();
  vi.mocked(createExportJob).mockReset();
  vi.mocked(exportTemplateCapability).mockReset();
  vi.mocked(previewExportSelection).mockReset();
  vi.mocked(saveExportArtifactCopy).mockReset();

  vi.mocked(ensureDefaultWorkspace).mockResolvedValue({ id: "w1", name: "Default" } as never);
  vi.mocked(listTherapyCollections).mockResolvedValue([collection] as never);
  vi.mocked(listJournalEntries).mockResolvedValue([]);
  vi.mocked(listTherapyRecords).mockResolvedValue([therapyRecord] as never);
  vi.mocked(getTherapyRecord).mockResolvedValue(therapyRecord as never);
  vi.mocked(listReferencesForRecord).mockResolvedValue([]);
  vi.mocked(exportTemplateCapability).mockResolvedValue({
    name: "default",
    version: "1.0.0-mvp",
    valid: true,
    compiler_version: "typst 0.15.1 (embedded)",
    unavailable_resources: [],
  } as never);
  vi.mocked(archiveTherapyCollection).mockResolvedValue({
    ...collection,
    archived_at: "2026-10-13T10:00:00Z",
  } as never);
});

afterEach(async () => {
  vi.restoreAllMocks();
  // RTL's auto-cleanup runs after this hook, so the tree can still be
  // mounted: the language event re-renders subscribers and must stay in act.
  await act(async () => {
    await i18n.changeLanguage("en-US");
  });
});

describe.each([...SUPPORTED_LOCALES])("Therapy page in %s", (locale) => {
  it("renders localized chrome with the authored collection name verbatim", async () => {
    await i18n.changeLanguage(locale);
    const e = COPY[locale];

    renderTherapyPage();

    // Row actions render only after the mocked collection resolves; await the
    // first row before asserting the rest of the chrome.
    const editButton = await screen.findByRole("button", { name: e.edit });
    expect(editButton).toBeTruthy();
    expect(screen.getByRole("heading", { name: e.title })).toBeTruthy();
    expect(screen.getByRole("note")).toHaveTextContent(e.banner);
    expect(screen.getByText(e.isolationNote)).toBeTruthy();
    expect(screen.getByRole("link", { name: e.exportAction })).toBeTruthy();
    expect(screen.getByLabelText(e.newCollection)).toBeTruthy();
    expect((screen.getByLabelText(e.newCollection) as HTMLInputElement).placeholder).toBe(
      e.namePlaceholder,
    );
    expect(screen.getByRole("button", { name: e.create })).toBeDisabled();
    expect(screen.getByText(e.collections)).toBeTruthy();
    expect(screen.getByRole("button", { name: e.archive })).toBeTruthy();

    // Authored collection names never translate with the UI language.
    expect(screen.getByText("Coping toolkit")).toBeTruthy();
  });

  it("renames a collection with localized Save/Cancel and localized empty state", async () => {
    await i18n.changeLanguage(locale);
    const e = COPY[locale];
    const renamed = { ...collection, name: "Grounding toolkit" };
    // First load shows the authored collection; later reloads show the
    // renamed one, and after archiving the collection is gone.
    vi.mocked(listTherapyCollections)
      .mockReset()
      .mockResolvedValueOnce([collection] as never)
      .mockResolvedValue([renamed] as never);
    vi.mocked(updateTherapyCollection).mockResolvedValue(renamed as never);

    renderTherapyPage();

    fireEvent.click(await screen.findByRole("button", { name: e.edit }));
    fireEvent.click(screen.getByRole("button", { name: e.cancel }));
    expect(screen.queryByDisplayValue("Coping toolkit")).toBeNull();

    fireEvent.click(screen.getByRole("button", { name: e.edit }));
    fireEvent.change(screen.getByDisplayValue("Coping toolkit"), {
      target: { value: "Grounding toolkit" },
    });
    fireEvent.click(screen.getByRole("button", { name: e.save }));

    await waitFor(() =>
      expect(updateTherapyCollection).toHaveBeenCalledWith("c1", { name: "Grounding toolkit" }),
    );
    expect(await screen.findByText("Grounding toolkit")).toBeTruthy();

    // After archiving, the localized empty state renders.
    vi.mocked(listTherapyCollections).mockResolvedValue([] as never);
    fireEvent.click(await screen.findByRole("button", { name: e.archive }));
    fireEvent.click(await screen.findByRole("button", { name: e.confirmArchive }));
    expect(await screen.findByText(e.collectionsEmpty)).toBeTruthy();
  });
});

describe.each([...SUPPORTED_LOCALES])("Therapy collection detail in %s", (locale) => {
  it("Intl-formats the record date and keeps authored titles verbatim", async () => {
    await i18n.changeLanguage(locale);
    const e = COPY[locale];

    renderCollectionPage();

    // The heading shows the authored collection name only after the mocked
    // collections resolve; awaiting it also flushes the record row load.
    expect((await screen.findAllByText("Coping toolkit")).length).toBeGreaterThan(0);
    expect(screen.getByText("Grounding note")).toBeTruthy();

    // The application-owned date-only value renders through the shared Intl
    // formatter; the raw stored ISO never appears as a label.
    const expectedDate = new Date("2026-10-05T12:00:00").toLocaleDateString(locale, {
      dateStyle: "medium",
    });
    expect(await screen.findByText(expectedDate)).toBeTruthy();
    expect(screen.queryByText("2026-10-05")).toBeNull();

    // Localized enum badges and action chrome surround the verbatim row.
    expect(screen.getByText(e.recordTypeNote)).toBeTruthy();
    expect(screen.getByText(e.statusOpen)).toBeTruthy();
    expect(screen.getByRole("link", { name: e.addRecord })).toBeTruthy();
  });
});

describe.each([...SUPPORTED_LOCALES])("Therapy record detail in %s", (locale) => {
  it("Intl-formats the occurred-on label and keeps the Markdown body verbatim", async () => {
    await i18n.changeLanguage(locale);
    const e = COPY[locale];

    renderRecordPage();

    const expectedDate = new Date("2026-10-05T12:00:00").toLocaleDateString(locale, {
      dateStyle: "medium",
    });
    expect(await screen.findByText(expectedDate)).toBeTruthy();
    expect(screen.queryByText("2026-10-05")).toBeNull();

    // Authored title and Markdown body render verbatim around localized
    // badges and actions.
    expect(screen.getByRole("heading", { level: 1, name: "Grounding note" })).toBeTruthy();
    expect(screen.getByText("4-7-8 breathing stays verbatim.")).toBeTruthy();
    expect(screen.getByText(e.recordTypeNote)).toBeTruthy();
    expect(screen.getByText(e.statusOpen)).toBeTruthy();
    expect(screen.getByRole("link", { name: e.edit })).toBeTruthy();
    expect(screen.getByRole("button", { name: e.archive })).toBeTruthy();

    // The linked-tasks editor flushes with its localized empty state.
    expect(await screen.findByText(e.referenceTasksEmpty)).toBeTruthy();
  });
});

describe.each([...SUPPORTED_LOCALES])("Therapy export in %s", (locale) => {
  it("Intl-formats record dates and localizes the no-date fallback", async () => {
    await i18n.changeLanguage(locale);
    const e = COPY[locale];
    vi.mocked(listTherapyRecords).mockResolvedValue([
      therapyRecord,
      { ...therapyRecord, id: "tr2", title: "Undated reflection", occurred_on: null },
    ] as never);

    renderTherapyExportPage();

    expect(await screen.findByRole("heading", { name: e.exportTitle })).toBeTruthy();

    const expectedDate = new Date("2026-10-05T12:00:00").toLocaleDateString(locale, {
      dateStyle: "medium",
    });
    expect(await screen.findByText(expectedDate)).toBeTruthy();
    expect(await screen.findByText(e.exportNoDate)).toBeTruthy();
    expect(screen.queryByText("2026-10-05")).toBeNull();

    // Authored record titles render verbatim in the selection list.
    expect(screen.getByText("Grounding note")).toBeTruthy();
    expect(screen.getByText("Undated reflection")).toBeTruthy();
  });
});

describe("Therapy isolation", () => {
  it("keeps therapy UI copy out of the journal surface in every locale", async () => {
    for (const locale of SUPPORTED_LOCALES) {
      await i18n.changeLanguage(locale);
      const e = COPY[locale];

      const { unmount } = renderJournalPage();

      expect(await screen.findByRole("heading", { name: e.journalHeading })).toBeTruthy();
      // The privacy banner, isolation note, and therapy action copy are only
      // rendered inside the therapy area: none of them reaches the journal
      // list, and the journal area keeps its own namespace chrome.
      expect(screen.queryByRole("note")).toBeNull();
      expect(screen.queryByText(e.banner)).toBeNull();
      expect(screen.queryByText(e.isolationNote)).toBeNull();
      expect(screen.queryByText(e.collectionsEmpty)).toBeNull();
      unmount();
    }
  });
});

describe("Therapy runtime locale switching", () => {
  it("re-renders chrome around the verbatim collection name and archive confirmation", async () => {
    vi.mocked(listTherapyCollections)
      .mockReset()
      .mockResolvedValueOnce([collection] as never)
      .mockResolvedValue([] as never);
    renderTherapyPage();

    // Await the row action: it exists only once the mocked collection
    // resolves. The first Archive click just moves the row into the
    // confirmation step.
    const archiveButton = await screen.findByRole("button", { name: "Archive" });
    fireEvent.click(archiveButton);
    expect(archiveTherapyCollection).not.toHaveBeenCalled();

    // The confirm step switches language together with the rest of the
    // chrome; the mounted tree re-renders inside act.
    await act(async () => {
      await i18n.changeLanguage("es-ES");
    });
    await waitFor(() =>
      expect(screen.getByRole("button", { name: "Confirmar archivado" })).toBeTruthy(),
    );
    expect(screen.getByRole("heading", { name: "Terapia" })).toBeTruthy();
    expect(screen.getByText("Coping toolkit")).toBeTruthy();
    expect(screen.getByText(/Modo de privacidad/)).toBeTruthy();
    expect(screen.queryByRole("button", { name: "Archive" })).toBeNull();

    fireEvent.click(screen.getByRole("button", { name: "Confirmar archivado" }));
    await waitFor(() => expect(archiveTherapyCollection).toHaveBeenCalledWith("c1"));
    expect(await screen.findByText("Aún no hay colecciones de terapia.")).toBeTruthy();
  });
});
