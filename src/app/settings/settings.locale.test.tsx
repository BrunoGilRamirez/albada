import i18n from "i18next";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { act, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { MemoryRouter } from "react-router";
import SettingsPage from "@/app/settings/page";
import { createBackup, listBackups, restoreBackup, validateBackup } from "@/lib/api/backupApi";
import { getEnvironmentDiagnostics } from "@/lib/api/diagnosticsApi";
import { exportTemplateCapability } from "@/lib/api/exportApi";
import { listArchivedTasks, restoreArchivedTask } from "@/lib/api/taskApi";
import {
  ensureDefaultWorkspace,
  getBackupReminderPreference,
  getWarningWindow,
  updateBackupReminderPreference,
  updateWarningWindow,
} from "@/lib/api/workspaceApi";
import { SUPPORTED_LOCALES, type AppLocale } from "@/lib/i18n/locale";
import type { BackupDto, BackupReminderPreference, Task } from "@/lib/types";

vi.mock("@/lib/api/backupApi", () => ({
  createBackup: vi.fn(),
  listBackups: vi.fn(),
  restoreBackup: vi.fn(),
  validateBackup: vi.fn(),
}));

vi.mock("@/lib/api/taskApi", () => ({
  listArchivedTasks: vi.fn(),
  restoreArchivedTask: vi.fn(),
}));

vi.mock("@/lib/api/exportApi", () => ({
  exportTemplateCapability: vi.fn(),
}));

vi.mock("@/lib/api/diagnosticsApi", () => ({
  getEnvironmentDiagnostics: vi.fn(),
}));

vi.mock("@/lib/api/workspaceApi", () => ({
  ensureDefaultWorkspace: vi.fn(),
  getBackupReminderPreference: vi.fn(),
  getWarningWindow: vi.fn(),
  updateBackupReminderPreference: vi.fn(),
  updateWarningWindow: vi.fn(),
  updateWorkspaceLocale: vi.fn(),
}));

// 8.6.11 Settings locale coverage: the page chrome, section navigation,
// warning-window editor, archived-task list, backup list, and the backup
// reminder preference all follow the active locale, including the
// restore confirmation and its privacy warning. User-authored task titles,
// workspace identifiers, checksums, and file paths stay verbatim in every
// language. Synthetic fixtures only: no real paths or personal content.

const FAKE_PATH = "C:\\synthetic\\backups\\fixture.sqlite";
const FAKE_CHECKSUM = "a".repeat(64);

const backup: BackupDto = {
  path: FAKE_PATH,
  created_at: "2026-10-01T08:00:00Z",
  size_bytes: 2048,
  checksum: FAKE_CHECKSUM,
  schema_version: 35,
  source_workspace_id: "w1",
  kind: "user",
  validation_status: "valid",
};

const archivedTask: Task = {
  id: "task-1",
  workspace_id: "w1",
  goal_id: null,
  project_id: null,
  parent_task_id: null,
  title: "Synthetic archived task",
  description_markdown: null,
  notes_markdown: null,
  blocking_note_markdown: null,
  status: "completed",
  priority: "normal",
  effort_size: null,
  effort_points: 0,
  target_date: null,
  scheduled_date: null,
  completed_at: "2026-09-20T10:00:00Z",
  cancelled_at: null,
  closure_mode: null,
  closure_reason_markdown: null,
  archived_at: "2026-09-21T10:00:00Z",
  created_at: "2026-09-19T10:00:00Z",
  updated_at: "2026-09-21T10:00:00Z",
};

const preference: BackupReminderPreference = {
  enabled: false,
  interval_days: 7,
  last_successful_backup_at: null,
};

const capabilityFixture = {
  name: "foundation-journal",
  version: "1.0.0",
  valid: true,
  compiler_version: "typst 0.15.1 (embedded)",
  unavailable_resources: [],
};

const diagnosticsFixture = {
  sqlite_version: "1.2.3-test",
  tauri_version: "4.5.6-test",
  os_platform: "test-os",
  os_version: "7.8.9-test",
  typst_compiler_version: "typst 0.15.1 (embedded)",
  template_name: "foundation-journal",
  template_version: "1.0.0",
  template_valid: true,
  bundled_font_count: 3,
};

interface SettingsCopy {
  title: string;
  subtitle: string;
  sectionsAria: string;
  commitments: string;
  availability: string;
  importSection: string;
  notificationsSection: string;
  workspace: string;
  warningLabel: string;
  warningHelp: string;
  warningInvalid: string;
  warningSaved: string;
  archivedTitle: string;
  archivedEmpty: string;
  archivedRestored: string;
  backupsTitle: string;
  createBackup: string;
  backupCreated: string;
  restoreAria: string;
  restore: string;
  backupKind: string;
  restored: string;
  reminderTitle: string;
  reminderEnable: string;
  reminderInterval: string;
  reminderSave: string;
  reminderNone: string;
  reminderRangeError: string;
  language: string;
  diagnosticsTitle: string;
  diagnosticsCapability: string;
}

const COPY: Record<AppLocale, SettingsCopy> = {
  "en-US": {
    title: "Settings",
    subtitle: "Workspace preferences, archived tasks, and backups.",
    sectionsAria: "Settings sections",
    commitments: "Fixed commitments",
    availability: "Availability",
    importSection: "Legacy import",
    notificationsSection: "Notifications",
    workspace: "Workspace",
    warningLabel: "Deadline warning window",
    warningHelp:
      "How many days before a deadline a task counts as approaching. Between 0 and 90. Currently 3 days.",
    warningInvalid: "The warning window must be between 0 and 90 days.",
    warningSaved: "Deadline warning window set to 5 days.",
    archivedTitle: "Archived tasks",
    archivedEmpty: "No archived tasks.",
    archivedRestored: "Task restored.",
    backupsTitle: "Backups",
    createBackup: "Create backup",
    backupCreated: "Backup created. Backups may contain sensitive data.",
    restoreAria: "Restore this backup",
    restore: "Restore",
    backupKind: "Backup",
    restored: "Backup restored. All data now reflects the restored backup.",
    reminderTitle: "Backup reminders",
    reminderEnable: "Enable backup reminders",
    reminderInterval: "Reminder interval (days)",
    reminderSave: "Save preference",
    reminderNone: "No validated manual backup recorded yet.",
    reminderRangeError: "The backup reminder interval must be between 1 and 30 days.",
    language: "Language",
    diagnosticsTitle: "Diagnostics",
    diagnosticsCapability: "Bundled capability: Available",
  },
  "es-ES": {
    title: "Configuración",
    subtitle: "Preferencias del espacio de trabajo, tareas archivadas y copias de seguridad.",
    sectionsAria: "Secciones de configuración",
    commitments: "Compromisos fijos",
    availability: "Disponibilidad",
    importSection: "Importación heredada",
    notificationsSection: "Notificaciones",
    workspace: "Espacio de trabajo",
    warningLabel: "Ventana de aviso de fecha límite",
    warningHelp:
      "Cuántos días antes de la fecha límite una tarea se considera próxima a cumplirse. Entre 0 y 90. Actualmente 3 días.",
    warningInvalid: "La ventana de aviso debe estar entre 0 y 90 días.",
    warningSaved: "Ventana de aviso de fecha límite fijada en 5 días.",
    archivedTitle: "Tareas archivadas",
    archivedEmpty: "No hay tareas archivadas.",
    archivedRestored: "Tarea restaurada.",
    backupsTitle: "Copias de seguridad",
    createBackup: "Crear copia de seguridad",
    backupCreated: "Copia de seguridad creada. Las copias pueden contener datos sensibles.",
    restoreAria: "Restaurar esta copia de seguridad",
    restore: "Restaurar",
    backupKind: "Copia de seguridad",
    restored: "Copia de seguridad restaurada. Todos los datos reflejan ahora la copia restaurada.",
    reminderTitle: "Recordatorios de copia de seguridad",
    reminderEnable: "Activar los recordatorios de copia de seguridad",
    reminderInterval: "Intervalo de recordatorio (días)",
    reminderSave: "Guardar preferencia",
    reminderNone: "Aún no se ha registrado ninguna copia manual validada.",
    reminderRangeError:
      "El intervalo del recordatorio de copia de seguridad debe estar entre 1 y 30 días.",
    language: "Idioma",
    diagnosticsTitle: "Diagnóstico",
    diagnosticsCapability: "Capacidad integrada: Disponible",
  },
  "de-DE": {
    title: "Einstellungen",
    subtitle: "Arbeitsbereichseinstellungen, archivierte Aufgaben und Sicherungen.",
    sectionsAria: "Einstellungsbereiche",
    commitments: "Feste Termine",
    availability: "Verfügbarkeit",
    importSection: "Legacy-Import",
    notificationsSection: "Benachrichtigungen",
    workspace: "Arbeitsbereich",
    warningLabel: "Fristwarnfenster",
    warningHelp:
      "Wie viele Tage vor einer Frist eine Aufgabe als bevorstehend gilt. Zwischen 0 und 90. Derzeit 3 Tage.",
    warningInvalid: "Das Warnfenster muss zwischen 0 und 90 Tagen liegen.",
    warningSaved: "Fristwarnfenster auf 5 Tage gesetzt.",
    archivedTitle: "Archivierte Aufgaben",
    archivedEmpty: "Keine archivierten Aufgaben.",
    archivedRestored: "Aufgabe wiederhergestellt.",
    backupsTitle: "Sicherungen",
    createBackup: "Sicherung erstellen",
    backupCreated: "Sicherung erstellt. Sicherungen können sensible Daten enthalten.",
    restoreAria: "Diese Sicherung wiederherstellen",
    restore: "Wiederherstellen",
    backupKind: "Sicherung",
    restored:
      "Sicherung wiederhergestellt. Alle Daten entsprechen nun der wiederhergestellten Sicherung.",
    reminderTitle: "Sicherungserinnerungen",
    reminderEnable: "Sicherungserinnerungen aktivieren",
    reminderInterval: "Erinnerungsintervall (Tage)",
    reminderSave: "Einstellung speichern",
    reminderNone: "Noch keine geprüfte manuelle Sicherung erfasst.",
    reminderRangeError:
      "Das Intervall der Sicherungserinnerung muss zwischen 1 und 30 Tagen liegen.",
    language: "Sprache",
    diagnosticsTitle: "Diagnose",
    diagnosticsCapability: "Mitgelieferte Fähigkeit: Verfügbar",
  },
  "fr-FR": {
    title: "Paramètres",
    subtitle: "Préférences de l'espace de travail, tâches archivées et sauvegardes.",
    sectionsAria: "Sections des paramètres",
    commitments: "Engagements fixes",
    availability: "Disponibilité",
    importSection: "Importation héritée",
    notificationsSection: "Notifications",
    workspace: "Espace de travail",
    warningLabel: "Fenêtre d'alerte d'échéance",
    warningHelp:
      "Combien de jours avant l'échéance une tâche est considérée comme imminente. Entre 0 et 90. Actuellement 3 jours.",
    warningInvalid: "La fenêtre d'alerte doit être comprise entre 0 et 90 jours.",
    warningSaved: "Fenêtre d'alerte d'échéance réglée sur 5 jours.",
    archivedTitle: "Tâches archivées",
    archivedEmpty: "Aucune tâche archivée.",
    archivedRestored: "Tâche restaurée.",
    backupsTitle: "Sauvegardes",
    createBackup: "Créer une sauvegarde",
    backupCreated: "Sauvegarde créée. Les sauvegardes peuvent contenir des données sensibles.",
    restoreAria: "Restaurer cette sauvegarde",
    restore: "Restaurer",
    backupKind: "Sauvegarde",
    restored:
      "Sauvegarde restaurée. Toutes les données reflètent désormais la sauvegarde restaurée.",
    reminderTitle: "Rappels de sauvegarde",
    reminderEnable: "Activer les rappels de sauvegarde",
    reminderInterval: "Intervalle de rappel (jours)",
    reminderSave: "Enregistrer la préférence",
    reminderNone: "Aucune sauvegarde manuelle validée n'a encore été enregistrée.",
    reminderRangeError:
      "L'intervalle du rappel de sauvegarde doit être compris entre 1 et 30 jours.",
    language: "Langue",
    diagnosticsTitle: "Diagnostic",
    diagnosticsCapability: "Capacité intégrée: Disponible",
  },
};

function renderPage() {
  return render(
    <MemoryRouter initialEntries={["/settings"]}>
      <SettingsPage />
    </MemoryRouter>,
  );
}

beforeEach(() => {
  vi.mocked(createBackup).mockReset();
  vi.mocked(listBackups).mockReset();
  vi.mocked(restoreBackup).mockReset();
  vi.mocked(validateBackup).mockReset();
  vi.mocked(listArchivedTasks).mockReset();
  vi.mocked(restoreArchivedTask).mockReset();
  vi.mocked(ensureDefaultWorkspace).mockReset();
  vi.mocked(getWarningWindow).mockReset();
  vi.mocked(updateWarningWindow).mockReset();
  vi.mocked(getBackupReminderPreference).mockReset();
  vi.mocked(updateBackupReminderPreference).mockReset();
  vi.mocked(exportTemplateCapability).mockReset();
  vi.mocked(getEnvironmentDiagnostics).mockReset();

  vi.mocked(ensureDefaultWorkspace).mockResolvedValue({ id: "w1" } as never);
  vi.mocked(listArchivedTasks).mockResolvedValue([archivedTask]);
  vi.mocked(listBackups).mockResolvedValue([backup]);
  vi.mocked(getWarningWindow).mockResolvedValue(3);
  vi.mocked(updateWarningWindow).mockResolvedValue(undefined as never);
  vi.mocked(getBackupReminderPreference).mockResolvedValue(preference);
  vi.mocked(restoreArchivedTask).mockResolvedValue(archivedTask);
  vi.mocked(createBackup).mockResolvedValue(backup);
  vi.mocked(validateBackup).mockResolvedValue({
    status: "valid",
    checksum: FAKE_CHECKSUM,
    schema_version: 35,
    size_bytes: 2048,
    kind: "user",
    message: "",
  });
  vi.mocked(restoreBackup).mockResolvedValue({ id: "w1" } as never);
  vi.mocked(exportTemplateCapability).mockResolvedValue(capabilityFixture as never);
  vi.mocked(getEnvironmentDiagnostics).mockResolvedValue(diagnosticsFixture as never);
  vi.spyOn(window, "confirm").mockReturnValue(false);
});

afterEach(async () => {
  vi.restoreAllMocks();
  // The tree is still mounted here (auto-cleanup runs after this hook), so the
  // language event must be flushed inside act.
  await act(async () => {
    await i18n.changeLanguage("en-US");
  });
});

describe.each([...SUPPORTED_LOCALES])("SettingsPage in %s", (locale) => {
  it("renders the page chrome, navigation, warning window, and backups in the active locale", async () => {
    await i18n.changeLanguage(locale);
    const e = COPY[locale];

    renderPage();

    // The archived/backup lists load after the workspace promise resolves;
    // awaiting the row first makes the remaining assertions deterministic.
    expect(await screen.findByText("Synthetic archived task")).toBeTruthy();
    expect(screen.getByRole("heading", { level: 1, name: e.title })).toBeTruthy();
    expect(screen.getByText(e.subtitle)).toBeTruthy();
    expect(screen.getByRole("navigation", { name: e.sectionsAria })).toBeTruthy();
    expect(screen.getByRole("link", { name: e.commitments })).toBeTruthy();
    expect(screen.getByRole("link", { name: e.availability })).toBeTruthy();
    expect(screen.getByRole("link", { name: e.importSection })).toBeTruthy();
    expect(screen.getByRole("link", { name: e.notificationsSection })).toBeTruthy();

    expect(screen.getByLabelText(e.workspace)).toBeTruthy();
    expect(screen.getByText(e.language)).toBeTruthy();
    expect(screen.getByLabelText(e.warningLabel)).toBeTruthy();
    // The current window value is interpolated through the active locale.
    expect(screen.getByText(e.warningHelp)).toBeTruthy();

    expect(screen.getByText(e.archivedTitle)).toBeTruthy();
    expect(screen.getByText(e.backupsTitle)).toBeTruthy();
    expect(screen.getByRole("button", { name: e.createBackup })).toBeTruthy();
    expect(screen.getByRole("button", { name: e.restoreAria })).toBeTruthy();
    // Backup kind badge and restore action localize around the verbatim
    // timestamp/size metadata.
    expect(screen.getByText(e.backupKind)).toBeTruthy();
    expect(screen.getByRole("button", { name: e.restore })).toBeTruthy();

    // 9.7 diagnostics section follows the active locale while native
    // versions and template names stay verbatim.
    expect(screen.getByText(e.diagnosticsTitle)).toBeTruthy();
    expect(await screen.findByText("typst 0.15.1 (embedded)", { exact: false })).toBeTruthy();
    expect(screen.getByText(e.diagnosticsCapability, { exact: false })).toBeTruthy();
    expect(screen.getByText("foundation-journal v1.0.0", { exact: false })).toBeTruthy();
  });

  it("localizes the backup reminder preference separately from the archived list", async () => {
    await i18n.changeLanguage(locale);
    const e = COPY[locale];

    renderPage();

    expect(await screen.findByText(e.reminderTitle)).toBeTruthy();
    expect(await screen.findByText("Synthetic archived task")).toBeTruthy();
    // Let the page's initial loads (workspace, backups, preference) settle so
    // the assertions below never race the form's draft re-sync.
    await act(async () => {
      await Promise.resolve();
    });
    expect(screen.getByRole("checkbox", { name: e.reminderEnable })).not.toBeChecked();
    expect(screen.getByLabelText(e.reminderInterval)).toHaveValue(7);
    expect(screen.getByTestId("last-validated-backup").textContent).toBe(e.reminderNone);
    expect(screen.getByRole("button", { name: e.reminderSave })).toBeTruthy();
    // The fixed native delivery text is a contract string and stays verbatim.
    expect(document.body.textContent).toContain("A validated backup is due.");
    // The schedule note keeps the fixed UTC slot in every language.
    expect(screen.getByTestId("backup-reminder-schedule-note").textContent).toContain("09:00 UTC");
  });

  it("keeps the archived task title verbatim while the status badge translates", async () => {
    await i18n.changeLanguage(locale);
    const e = COPY[locale];

    renderPage();

    expect(await screen.findByText("Synthetic archived task")).toBeTruthy();
    const statusBadge = screen.getByText(i18n.t("common.labels.taskStatus.completed"));
    expect(statusBadge).toBeTruthy();
    // The archive help sentence names the tasks-page action inline.
    expect(screen.getByText(i18n.t("settings.archived.help"))).toBeTruthy();
    expect(screen.getByText(e.archivedTitle)).toBeTruthy();
  });

  it("validates and saves the warning window in the active locale", async () => {
    await i18n.changeLanguage(locale);
    const e = COPY[locale];

    renderPage();
    const input = await screen.findByLabelText(e.warningLabel);

    fireEvent.change(input, { target: { value: "120" } });
    fireEvent.click(screen.getByRole("button", { name: i18n.t("common.actions.save") }));
    expect(await screen.findByRole("alert")).toHaveTextContent(e.warningInvalid);
    expect(updateWarningWindow).not.toHaveBeenCalled();

    fireEvent.change(input, { target: { value: "5" } });
    fireEvent.click(screen.getByRole("button", { name: i18n.t("common.actions.save") }));
    await waitFor(() => expect(updateWarningWindow).toHaveBeenCalledWith("w1", 5));
    expect(await screen.findByRole("status")).toHaveTextContent(e.warningSaved);
  });

  it("restores an archived task with localized feedback", async () => {
    await i18n.changeLanguage(locale);
    const e = COPY[locale];

    renderPage();

    fireEvent.click(await screen.findByRole("button", { name: e.restore }));
    await waitFor(() => expect(restoreArchivedTask).toHaveBeenCalledWith("task-1"));
    expect(await screen.findByRole("status")).toHaveTextContent(e.archivedRestored);
  });

  it("confirms backup restore with localized copy, privacy warning, and verbatim checksum", async () => {
    await i18n.changeLanguage(locale);
    const e = COPY[locale];
    vi.mocked(window.confirm).mockReturnValue(true);

    renderPage();

    fireEvent.click(await screen.findByRole("button", { name: e.restoreAria }));

    await waitFor(() => expect(window.confirm).toHaveBeenCalledTimes(1));
    const prompt = vi.mocked(window.confirm).mock.calls[0][0] as unknown as string;
    expect(prompt).toContain(
      i18n.t("settings.backups.restoreConfirmTitle", {
        kind: i18n.t("settings.backups.restoreKind.user"),
        createdAt: new Date(backup.created_at).toLocaleString(locale),
      }),
    );
    expect(prompt).toContain(
      i18n.t("settings.backups.restoreConfirmDetail", {
        size: new Intl.NumberFormat(locale).format(2),
        schema: new Intl.NumberFormat(locale).format(35),
        checksum: FAKE_CHECKSUM,
      }),
    );
    expect(prompt).toContain(i18n.t("settings.backups.restoreConfirmWarning"));
    expect(prompt).toContain(i18n.t("settings.backups.restoreConfirmPrivacy"));
    // The local path is never part of the confirmation prompt.
    expect(prompt).not.toContain(FAKE_PATH);

    await waitFor(() => expect(restoreBackup).toHaveBeenCalledWith(FAKE_PATH, FAKE_CHECKSUM));
    expect(await screen.findByRole("status")).toHaveTextContent(e.restored);
  });

  it("reports a failed backup validation with localized chrome and the native message", async () => {
    await i18n.changeLanguage(locale);
    const e = COPY[locale];
    vi.mocked(validateBackup).mockResolvedValue({
      status: "invalid",
      checksum: null,
      schema_version: null,
      size_bytes: null,
      kind: null,
      message: "Synthetic checksum mismatch.",
    });

    renderPage();

    fireEvent.click(await screen.findByRole("button", { name: e.restoreAria }));

    // Native validation text stays verbatim; the surrounding chrome is ours.
    expect((await screen.findByRole("alert")).textContent).toBe("Synthetic checksum mismatch.");
    expect(window.confirm).not.toHaveBeenCalled();
  });

  it("falls back to the localized validation error when the native message is empty", async () => {
    await i18n.changeLanguage(locale);
    vi.mocked(validateBackup).mockResolvedValue({
      status: "invalid",
      checksum: null,
      schema_version: null,
      size_bytes: null,
      kind: null,
      message: "",
    });

    renderPage();

    fireEvent.click(await screen.findByRole("button", { name: COPY[locale].restoreAria }));

    expect(await screen.findByRole("alert")).toHaveTextContent(
      i18n.t("settings.backups.restoreFailed"),
    );
    expect(window.confirm).not.toHaveBeenCalled();
  });

  it("cancels the restore when validation returns no checksum", async () => {
    await i18n.changeLanguage(locale);
    vi.mocked(validateBackup).mockResolvedValue({
      status: "valid",
      checksum: null,
      schema_version: 35,
      size_bytes: 2048,
      kind: "user",
      message: "",
    });

    renderPage();

    fireEvent.click(await screen.findByRole("button", { name: COPY[locale].restoreAria }));

    expect(await screen.findByRole("alert")).toHaveTextContent(
      i18n.t("settings.backups.restoreMissingChecksum"),
    );
    expect(window.confirm).not.toHaveBeenCalled();
    expect(restoreBackup).not.toHaveBeenCalled();
  });

  it("reports the localized backup reminder range error without calling the command", async () => {
    await i18n.changeLanguage(locale);
    const e = COPY[locale];

    renderPage();

    const intervalInput = await screen.findByLabelText(e.reminderInterval);
    // The page reloads the stored preference on mount and the form re-syncs
    // its draft from it; let those loads settle before entering an invalid
    // draft so the re-sync cannot overwrite it.
    await act(async () => {
      await Promise.resolve();
    });
    fireEvent.change(intervalInput, { target: { value: "31" } });
    fireEvent.click(screen.getByRole("button", { name: e.reminderSave }));

    expect((await screen.findByRole("alert")).textContent).toBe(e.reminderRangeError);
    expect(updateBackupReminderPreference).not.toHaveBeenCalled();
  });
});

describe("SettingsPage runtime locale switching", () => {
  it("re-renders chrome in the new language and keeps user content verbatim", async () => {
    renderPage();

    expect(await screen.findByText("Synthetic archived task")).toBeTruthy();
    expect(screen.getByRole("heading", { level: 1, name: "Settings" })).toBeTruthy();
    expect(screen.getByText("Completed")).toBeTruthy();

    await act(async () => {
      await i18n.changeLanguage("es-ES");
    });

    await waitFor(() =>
      expect(screen.getByRole("heading", { level: 1, name: "Configuración" })).toBeTruthy(),
    );
    expect(screen.queryByRole("heading", { level: 1, name: "Settings" })).toBeNull();
    expect(screen.getByText("Completada")).toBeTruthy();
    // User-authored title and the workspace identifier never translate.
    expect(screen.getByText("Synthetic archived task")).toBeTruthy();
    expect((screen.getByLabelText("Espacio de trabajo") as HTMLInputElement).value).toBe(
      "Espacio de trabajo w1…",
    );

    fireEvent.click(screen.getByRole("button", { name: "Restaurar" }));
    await waitFor(() => expect(restoreArchivedTask).toHaveBeenCalledWith("task-1"));
    expect(await screen.findByRole("status")).toHaveTextContent("Tarea restaurada.");
  });
});
