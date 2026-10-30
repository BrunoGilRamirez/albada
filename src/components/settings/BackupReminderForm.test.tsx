import { beforeEach, describe, expect, it, vi } from "vitest";
import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import BackupReminderForm from "@/components/settings/BackupReminderForm";
import { updateBackupReminderPreference } from "@/lib/api/workspaceApi";
import type { BackupReminderPreference } from "@/lib/types";

vi.mock("@/lib/api/workspaceApi", () => ({
  updateBackupReminderPreference: vi.fn(),
}));

// Synthetic fixtures only: no real workspace, paths, or personal content.
const preference: BackupReminderPreference = {
  enabled: false,
  interval_days: 7,
  last_successful_backup_at: null,
};

function renderForm(initial: BackupReminderPreference = preference, onSaved = vi.fn()) {
  const result = render(
    <BackupReminderForm workspaceId="w1" preference={initial} onSaved={onSaved} />,
  );
  return { ...result, onSaved };
}

describe("BackupReminderForm", () => {
  beforeEach(() => {
    vi.mocked(updateBackupReminderPreference).mockReset();
  });

  it("presents the preference separately from reminder categories with the live schedule", () => {
    const { container } = renderForm();

    expect(screen.getByRole("checkbox", { name: "Enable backup reminders" })).not.toBeChecked();
    expect(screen.getByLabelText("Reminder interval (days)")).toHaveValue(7);
    expect(container.textContent).toContain("not one of the reminder categories");
    expect(container.textContent).toContain("never creates a backup automatically");
    expect(container.textContent).toContain("7 days is the recommended interval");
    expect(container.textContent).toContain("A validated backup is due.");
    expect(screen.getByTestId("backup-reminder-schedule-note").textContent).toContain("09:00 UTC");
    expect(screen.getByTestId("backup-reminder-schedule-note").textContent).toContain(
      "at most once per interval cycle",
    );
    expect(container.textContent).not.toContain("not available in this build yet");
  });

  it("shows the last validated backup only when the preference records one", () => {
    const { rerender } = renderForm();
    expect(screen.getByTestId("last-validated-backup").textContent).toBe(
      "No validated manual backup recorded yet.",
    );

    rerender(
      <BackupReminderForm
        workspaceId="w1"
        preference={{ ...preference, last_successful_backup_at: "2026-10-05T09:00:00Z" }}
        onSaved={vi.fn()}
      />,
    );
    expect(screen.getByTestId("last-validated-backup").textContent).toContain(
      "Last validated manual backup:",
    );
  });

  it.each(["0", "31"])(
    "rejects the interval %s without calling the native command",
    async (value) => {
      renderForm();

      fireEvent.change(screen.getByLabelText("Reminder interval (days)"), {
        target: { value },
      });
      fireEvent.click(screen.getByRole("button", { name: "Save preference" }));

      expect((await screen.findByRole("alert")).textContent).toBe(
        "The backup reminder interval must be between 1 and 30 days.",
      );
      expect(updateBackupReminderPreference).not.toHaveBeenCalled();
    },
  );

  it("rejects a fractional interval without rounding or calling the native command", async () => {
    renderForm();

    fireEvent.change(screen.getByLabelText("Reminder interval (days)"), {
      target: { value: "14.5" },
    });
    fireEvent.click(screen.getByRole("button", { name: "Save preference" }));

    expect((await screen.findByRole("alert")).textContent).toBe(
      "The backup reminder interval must be a whole number of days.",
    );
    expect(updateBackupReminderPreference).not.toHaveBeenCalled();
  });

  it("saves the enabled state and interval through the typed command", async () => {
    const updated: BackupReminderPreference = {
      enabled: true,
      interval_days: 14,
      last_successful_backup_at: null,
    };
    vi.mocked(updateBackupReminderPreference).mockResolvedValue(updated);
    const { onSaved } = renderForm();

    fireEvent.click(screen.getByRole("checkbox", { name: "Enable backup reminders" }));
    fireEvent.change(screen.getByLabelText("Reminder interval (days)"), {
      target: { value: "14" },
    });
    fireEvent.click(screen.getByRole("button", { name: "Save preference" }));

    await waitFor(() =>
      expect(updateBackupReminderPreference).toHaveBeenCalledWith("w1", {
        enabled: true,
        interval_days: 14,
      }),
    );
    await waitFor(() => expect(onSaved).toHaveBeenCalledWith(updated));
    expect((await screen.findByRole("status")).textContent).toBe(
      "Backup reminder preference saved.",
    );
  });

  it("surfaces a safe display message when the native command rejects", async () => {
    vi.mocked(updateBackupReminderPreference).mockRejectedValue({
      code: "VALIDATION",
      message: "The backup reminder interval must be between 1 and 30 days.",
    });
    const { onSaved } = renderForm();

    fireEvent.click(screen.getByRole("button", { name: "Save preference" }));

    expect((await screen.findByRole("alert")).textContent).toBe(
      "Please check the information and try again.",
    );
    expect(onSaved).not.toHaveBeenCalled();
  });

  it("re-syncs the draft when the stored preference changes", () => {
    const { rerender } = renderForm();

    fireEvent.change(screen.getByLabelText("Reminder interval (days)"), {
      target: { value: "21" },
    });
    expect(screen.getByLabelText("Reminder interval (days)")).toHaveValue(21);

    // The page reloads the preference after a backup or restore; the form must
    // not keep saving onto the stale draft.
    rerender(
      <BackupReminderForm
        workspaceId="w1"
        preference={{ ...preference, interval_days: 10 }}
        onSaved={vi.fn()}
      />,
    );
    expect(screen.getByLabelText("Reminder interval (days)")).toHaveValue(10);
  });
});
