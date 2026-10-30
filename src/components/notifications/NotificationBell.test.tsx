import { beforeEach, describe, expect, it, vi } from "vitest";
import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import NotificationBell from "@/components/notifications/NotificationBell";
import {
  dismissNotification,
  listNotificationDeliveries,
  listReminderRules,
  snoozeNotification,
} from "@/lib/api/notificationApi";
import { ensureDefaultWorkspace } from "@/lib/api/workspaceApi";
import type { NotificationDelivery, ReminderRule } from "@/lib/types";
import type { NotificationDeliveryFiltersInput } from "@/lib/api/notificationApi";

vi.mock("@/lib/api/workspaceApi", () => ({
  ensureDefaultWorkspace: vi.fn(),
}));

vi.mock("@/lib/api/notificationApi", () => ({
  listNotificationDeliveries: vi.fn(),
  listReminderRules: vi.fn(),
  dismissNotification: vi.fn(),
  snoozeNotification: vi.fn(),
  clearNotificationHistory: vi.fn(),
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
  quiet_hours_start: null,
  quiet_hours_end: null,
  deadline_notification_window_days: 3,
  max_reminders_per_day: 5,
  snooze_minutes: [10, 30],
  channel: "desktop",
  escalation: "none",
  privacy_safe_message: "Time to plan your day",
  deliveries_today: 1,
  has_delivery_today: true,
};

const scheduledDelivery: NotificationDelivery = {
  id: "d1",
  workspace_id: "w1",
  delivery_kind: "reminder_rule",
  reminder_rule_id: "r1",
  target: null,
  scheduled_at: "2026-10-05T09:00:00Z",
  delivered_at: null,
  status: "scheduled",
  channel: "desktop",
  message: "Time to plan your day",
  error_message: null,
  created_at: "2026-10-05T08:00:00Z",
  cycle_key: null,
};

const deliveredDelivery: NotificationDelivery = {
  ...scheduledDelivery,
  id: "d2",
  status: "delivered",
  message: "Ready for a focus block?",
  delivered_at: "2026-10-05T09:01:00Z",
};

describe("NotificationBell", () => {
  beforeEach(() => {
    vi.mocked(ensureDefaultWorkspace).mockReset();
    vi.mocked(listNotificationDeliveries).mockReset();
    vi.mocked(listReminderRules).mockReset();
    vi.mocked(dismissNotification).mockReset();
    vi.mocked(snoozeNotification).mockReset();

    vi.mocked(ensureDefaultWorkspace).mockResolvedValue({ id: "w1", name: "Life" } as never);
    // The badge queries pending deliveries with the `scheduled` status filter;
    // the history panel lists recent rows without one.
    vi.mocked(listNotificationDeliveries).mockImplementation(
      (filters: NotificationDeliveryFiltersInput) =>
        Promise.resolve(
          filters.status === "scheduled"
            ? [scheduledDelivery]
            : [scheduledDelivery, deliveredDelivery],
        ),
    );
    vi.mocked(listReminderRules).mockResolvedValue([planningRule]);
  });

  it("badges the bell with the pending delivery count", async () => {
    render(<NotificationBell />);

    expect(
      await screen.findByRole("button", { name: "Notifications (1 unread)" }),
    ).toBeInTheDocument();
  });

  it("hides the badge when nothing is pending", async () => {
    vi.mocked(listNotificationDeliveries).mockImplementation(() => Promise.resolve([]));

    render(<NotificationBell />);

    const bell = await screen.findByRole("button", { name: "Notifications" });
    expect(bell).toBeInTheDocument();
    expect(screen.queryByText("1")).toBeNull();
  });

  it("opens the history panel from the bell and closes it again", async () => {
    render(<NotificationBell />);

    const bell = await screen.findByRole("button", { name: "Notifications (1 unread)" });
    expect(screen.queryByText("Recent notifications")).toBeNull();

    fireEvent.click(bell);
    expect(await screen.findByText("Recent notifications")).toBeInTheDocument();
    expect(screen.getByText("Ready for a focus block?")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Dismiss" })).toBeInTheDocument();

    fireEvent.click(bell);
    expect(screen.queryByText("Recent notifications")).toBeNull();
  });

  // Phase 8.7.5: a row with a supported target publishes its navigation intent
  // and the shell closes the panel so the destination page is visible.
  it("closes the history panel when a supported target row is navigated", async () => {
    const taskTargetDelivery: NotificationDelivery = {
      ...deliveredDelivery,
      id: "dt1",
      message: "Your task reminder was delivered",
      target: { entity_type: "task", entity_id: "task-9" },
    };
    vi.mocked(listNotificationDeliveries).mockImplementation(
      (filters: NotificationDeliveryFiltersInput) =>
        Promise.resolve(
          filters.status === "scheduled" ? [scheduledDelivery] : [taskTargetDelivery],
        ),
    );

    render(<NotificationBell />);

    fireEvent.click(await screen.findByRole("button", { name: "Notifications (1 unread)" }));
    fireEvent.click(await screen.findByText("Your task reminder was delivered"));

    await waitFor(() => expect(screen.queryByText("Recent notifications")).toBeNull());
  });
});
