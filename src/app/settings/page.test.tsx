import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { fireEvent, render, screen, waitFor } from "@testing-library/react";
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
import type { BackupDto, BackupReminderPreference, BackupValidationDto } from "@/lib/types";

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

// Synthetic fixtures only: no real paths, journal, or therapy content.
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

const validResult: BackupValidationDto = {
  status: "valid",
  checksum: FAKE_CHECKSUM,
  schema_version: 35,
  size_bytes: 2048,
  kind: "user",
  message: "Backup validation succeeded. It is ready for explicit restore confirmation.",
};

const invalidResult: BackupValidationDto = {
  status: "invalid",
  checksum: null,
  schema_version: null,
  size_bytes: null,
  kind: null,
  message: "The backup checksum does not match its metadata.",
};

const defaultPreference: BackupReminderPreference = {
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

function renderPage() {
  return render(
    <MemoryRouter initialEntries={["/settings"]}>
      <SettingsPage />
    </MemoryRouter>,
  );
}

describe("SettingsPage backup restore", () => {
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
    vi.mocked(listArchivedTasks).mockResolvedValue([]);
    vi.mocked(listBackups).mockResolvedValue([backup]);
    vi.mocked(getWarningWindow).mockResolvedValue(3);
    vi.mocked(getBackupReminderPreference).mockResolvedValue(defaultPreference);
    vi.mocked(createBackup).mockResolvedValue(backup);
    vi.mocked(validateBackup).mockResolvedValue(validResult);
    vi.mocked(restoreBackup).mockResolvedValue({ id: "w1" } as never);
    vi.mocked(exportTemplateCapability).mockResolvedValue(capabilityFixture as never);
    vi.mocked(getEnvironmentDiagnostics).mockResolvedValue(diagnosticsFixture as never);
    vi.spyOn(window, "confirm").mockReturnValue(false);
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  it("pre-validates the backup before asking for confirmation", async () => {
    renderPage();

    fireEvent.click(await screen.findByRole("button", { name: "Restore this backup" }));

    await waitFor(() => expect(validateBackup).toHaveBeenCalledWith(FAKE_PATH));
    await waitFor(() => expect(window.confirm).toHaveBeenCalledTimes(1));
    expect(restoreBackup).not.toHaveBeenCalled();
  });

  it("does not restore anything when the confirmation is cancelled", async () => {
    renderPage();

    fireEvent.click(await screen.findByRole("button", { name: "Restore this backup" }));

    await waitFor(() => expect(window.confirm).toHaveBeenCalledTimes(1));
    expect(restoreBackup).not.toHaveBeenCalled();
    expect(listBackups).toHaveBeenCalledTimes(1);
    expect(screen.queryByRole("status")).toBeNull();
  });

  it("shows a safe error and skips confirmation when validation is rejected", async () => {
    vi.mocked(validateBackup).mockRejectedValue({
      code: "IO",
      message: "The backup file was not found.",
    });
    renderPage();

    fireEvent.click(await screen.findByRole("button", { name: "Restore this backup" }));

    expect((await screen.findByRole("alert")).textContent).toBe(
      "The local file operation could not be completed.",
    );
    expect(window.confirm).not.toHaveBeenCalled();
    expect(restoreBackup).not.toHaveBeenCalled();
  });

  it("shows the native validation message and skips confirmation when invalid", async () => {
    vi.mocked(validateBackup).mockResolvedValue(invalidResult);
    renderPage();

    fireEvent.click(await screen.findByRole("button", { name: "Restore this backup" }));

    expect((await screen.findByRole("alert")).textContent).toBe(
      "The backup checksum does not match its metadata.",
    );
    expect(window.confirm).not.toHaveBeenCalled();
    expect(restoreBackup).not.toHaveBeenCalled();
  });

  it("restores with the validated checksum and refreshes after explicit confirmation", async () => {
    vi.mocked(window.confirm).mockReturnValue(true);
    renderPage();

    fireEvent.click(await screen.findByRole("button", { name: "Restore this backup" }));

    await waitFor(() => expect(restoreBackup).toHaveBeenCalledWith(FAKE_PATH, FAKE_CHECKSUM));
    expect((await screen.findByRole("status")).textContent).toBe(
      "Backup restored. All data now reflects the restored backup.",
    );
    // Initial load plus the post-restore refresh of the backup list.
    await waitFor(() => expect(listBackups).toHaveBeenCalledTimes(2));
  });

  it("never renders the local backup path in the page", async () => {
    const { container } = renderPage();

    const button = await screen.findByRole("button", { name: "Restore this backup" });
    expect(container.textContent).not.toContain(FAKE_PATH);
    expect(container.textContent).not.toContain("synthetic");

    fireEvent.click(button);
    await waitFor(() => expect(validateBackup).toHaveBeenCalledWith(FAKE_PATH));
    expect(container.textContent).not.toContain(FAKE_PATH);
  });

  it("renders the backup reminder preference separately from reminder categories", async () => {
    const { container } = renderPage();

    const checkbox = await screen.findByRole("checkbox", { name: "Enable backup reminders" });
    expect(checkbox).not.toBeChecked();
    expect(screen.getByLabelText("Reminder interval (days)")).toHaveValue(7);
    expect(screen.getByTestId("last-validated-backup").textContent).toBe(
      "No validated manual backup recorded yet.",
    );
    expect(screen.getByTestId("backup-reminder-schedule-note").textContent).toContain("09:00 UTC");
    expect(container.textContent).toContain("A validated backup is due.");
    expect(container.textContent).not.toContain("not available in this build yet");
    expect(getBackupReminderPreference).toHaveBeenCalledWith("w1");
  });

  it("shows the recorded validated backup timestamp from the preference", async () => {
    vi.mocked(getBackupReminderPreference).mockResolvedValue({
      enabled: true,
      interval_days: 14,
      last_successful_backup_at: "2026-10-05T09:00:00Z",
    });
    const { container } = renderPage();

    expect(await screen.findByRole("checkbox", { name: "Enable backup reminders" })).toBeChecked();
    expect(screen.getByLabelText("Reminder interval (days)")).toHaveValue(14);
    await waitFor(() =>
      expect(screen.getByTestId("last-validated-backup").textContent).toContain(
        "Last validated manual backup:",
      ),
    );
    expect(container.textContent).not.toContain("No validated manual backup recorded yet.");
  });

  it("refreshes the reminder preference after a backup is created", async () => {
    renderPage();
    await waitFor(() => expect(getBackupReminderPreference).toHaveBeenCalledTimes(1));

    fireEvent.click(await screen.findByRole("button", { name: "Create backup" }));

    await waitFor(() => expect(createBackup).toHaveBeenCalledTimes(1));
    await waitFor(() => expect(getBackupReminderPreference).toHaveBeenCalledTimes(2));
  });
});
