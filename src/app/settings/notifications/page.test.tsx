import { beforeEach, describe, expect, it, vi } from "vitest";
import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import NotificationsSettingsPage from "@/app/settings/notifications/page";
import { listReminderRules, updateReminderRule } from "@/lib/api/notificationApi";
import { ensureDefaultWorkspace } from "@/lib/api/workspaceApi";
import { isPermissionGranted, requestPermission } from "@tauri-apps/plugin-notification";
import type { ReminderRule } from "@/lib/types";

vi.mock("@/lib/api/notificationApi", () => ({
  listReminderRules: vi.fn(),
  updateReminderRule: vi.fn(),
}));

vi.mock("@/lib/api/workspaceApi", () => ({
  ensureDefaultWorkspace: vi.fn(),
}));

vi.mock("@tauri-apps/api/core", () => ({
  isTauri: () => true,
}));

vi.mock("@tauri-apps/plugin-notification", () => ({
  isPermissionGranted: vi.fn(),
  requestPermission: vi.fn(),
}));

const planningRule: ReminderRule = {
  id: "r1",
  workspace_id: "w1",
  reminder_type: "planning",
  enabled: true,
  recurrence_rule: null,
  preferred_time: "09:00:00",
  time_window_start: null,
  time_window_end: null,
  quiet_hours_start: "22:00:00",
  quiet_hours_end: "07:00:00",
  deadline_notification_window_days: 3,
  max_reminders_per_day: 5,
  snooze_minutes: [10, 30, 60],
  channel: "desktop",
  escalation: "none",
  privacy_safe_message: "Time to plan your day",
  deliveries_today: 0,
  has_delivery_today: false,
};

describe("NotificationsSettingsPage", () => {
  beforeEach(() => {
    vi.mocked(listReminderRules).mockReset();
    vi.mocked(updateReminderRule).mockReset();
    vi.mocked(ensureDefaultWorkspace).mockReset();
    vi.mocked(isPermissionGranted).mockReset();
    vi.mocked(requestPermission).mockReset();

    vi.mocked(ensureDefaultWorkspace).mockResolvedValue({ id: "w1" } as never);
    vi.mocked(listReminderRules).mockResolvedValue([planningRule]);
    vi.mocked(updateReminderRule).mockResolvedValue({ ...planningRule, enabled: false });
    vi.mocked(isPermissionGranted).mockResolvedValue(true);
  });

  it("lists the seeded rules with their editable settings", async () => {
    render(<NotificationsSettingsPage />);

    expect(await screen.findByText("Daily planning")).toBeInTheDocument();
    expect(screen.getByLabelText(/Privacy-safe message/i)).toHaveValue("Time to plan your day");
    expect(screen.getByLabelText(/Max reminders per day/i)).toHaveValue(5);
    expect(screen.getByLabelText(/Snooze options/i)).toHaveValue("10, 30, 60");
    expect(listReminderRules).toHaveBeenCalledWith("w1");
  });

  it("saves a rule toggle and echoes unchanged values in the native HH:MM form", async () => {
    render(<NotificationsSettingsPage />);

    const toggle = await screen.findByLabelText(/Enable Daily planning reminders/i);
    fireEvent.click(toggle);
    fireEvent.click(screen.getByRole("button", { name: /Save rule/i }));

    await waitFor(() =>
      expect(updateReminderRule).toHaveBeenCalledWith("r1", {
        enabled: false,
        preferred_time: "09:00",
        time_window_start: null,
        time_window_end: null,
        quiet_hours_start: "22:00",
        quiet_hours_end: "07:00",
        deadline_notification_window_days: 3,
        max_reminders_per_day: 5,
        snooze_minutes: [10, 30, 60],
        channel: "desktop",
        escalation: "none",
        privacy_safe_message: "Time to plan your day",
      }),
    );
    await waitFor(() => expect(screen.getByText("Rule saved.")).toBeInTheDocument());
    expect(toggle).not.toBeChecked();
  });

  it("rejects inverted quiet hours before calling the native command", async () => {
    render(<NotificationsSettingsPage />);

    const start = await screen.findByLabelText(/Quiet hours start/i);
    fireEvent.change(start, { target: { value: "07:00" } });
    fireEvent.click(screen.getByRole("button", { name: /Save rule/i }));

    expect(
      await screen.findByText(/quiet hours need different start and end times/i),
    ).toBeInTheDocument();
    expect(updateReminderRule).not.toHaveBeenCalled();
  });

  it("validates and persists the deadline notification lead window", async () => {
    const deadlineRule: ReminderRule = {
      ...planningRule,
      id: "deadline-rule",
      reminder_type: "deadline",
    };
    vi.mocked(listReminderRules).mockResolvedValue([deadlineRule]);
    vi.mocked(updateReminderRule).mockResolvedValue({
      ...deadlineRule,
      deadline_notification_window_days: 7,
    });
    render(<NotificationsSettingsPage />);

    const window = await screen.findByLabelText(/Deadline notification lead window/i);
    fireEvent.change(window, { target: { value: "8" } });
    fireEvent.submit(screen.getByRole("form", { name: /Deadline check reminder settings/i }));
    expect(await screen.findByText(/must be between 1 and 7 days/i)).toBeInTheDocument();
    expect(updateReminderRule).not.toHaveBeenCalled();

    fireEvent.change(window, { target: { value: "7" } });
    fireEvent.click(screen.getByRole("button", { name: /Save rule/i }));
    await waitFor(() =>
      expect(updateReminderRule).toHaveBeenCalledWith(
        "deadline-rule",
        expect.objectContaining({ deadline_notification_window_days: 7 }),
      ),
    );
  });

  it("requests the desktop notification permission when it is not granted", async () => {
    vi.mocked(isPermissionGranted).mockResolvedValue(false);
    vi.mocked(requestPermission).mockResolvedValue("granted");

    render(<NotificationsSettingsPage />);

    const status = await screen.findByTestId("permission-status");
    expect(status).toHaveTextContent(/not allowed yet/i);

    fireEvent.click(screen.getByRole("button", { name: /Request notification permission/i }));

    await waitFor(() => expect(requestPermission).toHaveBeenCalled());
    await waitFor(() => expect(status).toHaveTextContent(/Notifications are allowed/i));
  });
});
