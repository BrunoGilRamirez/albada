import { beforeEach, describe, expect, it, vi } from "vitest";
import { fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import NotificationHistory from "@/components/notifications/NotificationHistory";
import {
  clearNotificationHistory,
  dismissNotification,
  listNotificationDeliveries,
  listReminderRules,
  snoozeNotification,
} from "@/lib/api/notificationApi";
import {
  notificationIntentPath,
  publishNotificationIntent,
} from "@/lib/notifications/notificationIntent";
import type { NotificationDelivery, ReminderRule } from "@/lib/types";

vi.mock("@/lib/api/notificationApi", () => ({
  listNotificationDeliveries: vi.fn(),
  listReminderRules: vi.fn(),
  dismissNotification: vi.fn(),
  snoozeNotification: vi.fn(),
  clearNotificationHistory: vi.fn(),
}));

// Only the publish side is mocked; the routable-target classification stays
// real so the navigation gating itself is exercised.
vi.mock("@/lib/notifications/notificationIntent", async (importOriginal) => {
  const actual = await importOriginal<typeof import("@/lib/notifications/notificationIntent")>();
  return { ...actual, publishNotificationIntent: vi.fn() };
});

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
  reminder_rule_id: "r2",
  status: "delivered",
  message: "Ready for a focus block?",
  delivered_at: "2026-10-05T09:01:00Z",
};

const failedDelivery: NotificationDelivery = {
  ...scheduledDelivery,
  id: "d3",
  reminder_rule_id: "r3",
  status: "failed",
  message: "You have tasks approaching their deadline",
  error_message: "The desktop notification could not be displayed.",
};

// Synthetic ruleless backup delivery: exactly what the native dispatcher
// persists for a due backup cycle (no rule, no target, fixed safe text, and a
// cycle key that must stay internal).
const backupDelivery: NotificationDelivery = {
  id: "bk1",
  workspace_id: "w1",
  delivery_kind: "backup",
  reminder_rule_id: null,
  target: null,
  scheduled_at: "2026-10-05T09:00:00Z",
  delivered_at: null,
  status: "scheduled",
  channel: "desktop",
  message: "A validated backup is due.",
  error_message: null,
  created_at: "2026-10-05T08:59:00Z",
  cycle_key: "2026-10-05T09:00:00Z",
};

// Synthetic targeted deliveries (Phase 8.7.5): a task, a block, and a
// unsupported `session` target, exactly as the native layer returns them.
const taskTargetDelivery: NotificationDelivery = {
  ...deliveredDelivery,
  id: "dt1",
  status: "delivered",
  message: "Your task reminder was delivered",
  target: { entity_type: "task", entity_id: "task-9" },
};

const blockTargetDelivery: NotificationDelivery = {
  ...deliveredDelivery,
  id: "dbk1",
  delivery_kind: "block",
  reminder_rule_id: null,
  status: "delivered",
  message: "Your block start reminder was delivered",
  target: { entity_type: "block", entity_id: "block-1" },
};

const sessionTargetDelivery: NotificationDelivery = {
  ...deliveredDelivery,
  id: "dse1",
  status: "dismissed",
  message: "A session-related notice",
  target: { entity_type: "session", entity_id: "session-1" },
};

function mockSuccessfulLoad() {
  vi.mocked(listNotificationDeliveries).mockResolvedValue([
    scheduledDelivery,
    deliveredDelivery,
    failedDelivery,
  ]);
  vi.mocked(listReminderRules).mockResolvedValue([planningRule]);
}

describe("NotificationHistory", () => {
  beforeEach(() => {
    vi.mocked(listNotificationDeliveries).mockReset();
    vi.mocked(listReminderRules).mockReset();
    vi.mocked(dismissNotification).mockReset();
    vi.mocked(snoozeNotification).mockReset();
    vi.mocked(clearNotificationHistory).mockReset();
    vi.mocked(publishNotificationIntent).mockReset();
    vi.spyOn(window, "confirm").mockReturnValue(false);
    mockSuccessfulLoad();
  });

  it("lists recent deliveries with safe status and date display", async () => {
    render(<NotificationHistory workspaceId="w1" />);

    expect(await screen.findByText("Time to plan your day")).toBeInTheDocument();
    expect(screen.getByText("Ready for a focus block?")).toBeInTheDocument();
    expect(screen.getByText("You have tasks approaching their deadline")).toBeInTheDocument();
    expect(
      screen.getByText("The desktop notification could not be displayed."),
    ).toBeInTheDocument();
    expect(listNotificationDeliveries).toHaveBeenCalledWith({ workspace_id: "w1", limit: 30 });

    const rows = screen.getAllByTestId("notification-row");
    expect(rows).toHaveLength(3);
    expect(within(rows[0]).getByText("Scheduled")).toBeInTheDocument();
    expect(within(rows[1]).getByText("Delivered")).toBeInTheDocument();
    expect(within(rows[2]).getByText("Failed")).toBeInTheDocument();
  });

  it("offers dismiss and snooze actions only for pending scheduled rows", async () => {
    render(<NotificationHistory workspaceId="w1" />);

    const rows = await screen.findAllByTestId("notification-row");
    expect(within(rows[0]).getByRole("button", { name: "Dismiss" })).toBeInTheDocument();
    expect(within(rows[0]).getByRole("button", { name: "Snooze 10 min" })).toBeInTheDocument();
    expect(within(rows[0]).getByRole("button", { name: "Snooze 30 min" })).toBeInTheDocument();
    expect(within(rows[1]).queryByRole("button", { name: /dismiss|snooze/i })).toBeNull();
    expect(within(rows[2]).queryByRole("button", { name: /dismiss|snooze/i })).toBeNull();
  });

  it("dismisses a delivery, reloads the history, and notifies the shell", async () => {
    const onChanged = vi.fn();
    vi.mocked(dismissNotification).mockResolvedValue({
      ...scheduledDelivery,
      status: "dismissed",
    });

    render(<NotificationHistory workspaceId="w1" onChanged={onChanged} />);
    fireEvent.click(await screen.findByRole("button", { name: "Dismiss" }));

    await waitFor(() => expect(dismissNotification).toHaveBeenCalledWith("d1"));
    await waitFor(() => expect(onChanged).toHaveBeenCalled());
    await waitFor(() => expect(listNotificationDeliveries).toHaveBeenCalledTimes(2));
  });

  it("snoozes a delivery using one of the rule's configured durations", async () => {
    vi.mocked(snoozeNotification).mockResolvedValue({
      ...scheduledDelivery,
      id: "d4",
      status: "scheduled",
    });

    render(<NotificationHistory workspaceId="w1" />);
    fireEvent.click(await screen.findByRole("button", { name: "Snooze 30 min" }));

    await waitFor(() => expect(snoozeNotification).toHaveBeenCalledWith("d1", 30));
  });

  it("renders and dismisses a ruleless backup delivery without assuming a rule", async () => {
    vi.mocked(listNotificationDeliveries).mockResolvedValue([backupDelivery]);
    vi.mocked(dismissNotification).mockResolvedValue({ ...backupDelivery, status: "dismissed" });
    const onChanged = vi.fn();

    const { container } = render(<NotificationHistory workspaceId="w1" onChanged={onChanged} />);

    const row = await screen.findByTestId("notification-row");
    expect(within(row).getByText("A validated backup is due.")).toBeInTheDocument();
    expect(within(row).getByText("Scheduled")).toBeInTheDocument();
    expect(within(row).getByRole("button", { name: "Dismiss" })).toBeInTheDocument();
    // Snooze durations are rule-backed and the native domain rejects snoozing
    // backup deliveries, so a ruleless row never shows snooze affordances.
    expect(within(row).queryByRole("button", { name: /snooze/i })).toBeNull();
    // Internal identifiers and dedup keys are never rendered as content.
    expect(container.textContent).not.toContain("bk1");
    expect(container.textContent).not.toContain("2026-10-05T09:00:00Z");

    fireEvent.click(within(row).getByRole("button", { name: "Dismiss" }));

    await waitFor(() => expect(dismissNotification).toHaveBeenCalledWith("bk1"));
    await waitFor(() => expect(onChanged).toHaveBeenCalled());
    // Initial load plus the post-dismiss reload.
    await waitFor(() => expect(listNotificationDeliveries).toHaveBeenCalledTimes(2));
  });

  it("shows a friendly alert when the history cannot be loaded", async () => {
    vi.mocked(listNotificationDeliveries).mockRejectedValue({ code: "DATABASE" });

    render(<NotificationHistory workspaceId="w1" />);

    expect(
      await screen.findByText("Your local data could not be saved. Please try again."),
    ).toBeInTheDocument();
  });

  // --- Phase 8.7.5: navigation, unsupported targets, and clear history ---

  it("navigates a supported task target, preserves its id, and closes the panel", async () => {
    vi.mocked(listNotificationDeliveries).mockResolvedValue([taskTargetDelivery]);
    const onNavigate = vi.fn();

    render(<NotificationHistory workspaceId="w1" onNavigate={onNavigate} />);

    fireEvent.click(await screen.findByText("Your task reminder was delivered"));

    await waitFor(() =>
      expect(publishNotificationIntent).toHaveBeenCalledWith({
        kind: "task",
        entityType: "task",
        entityId: "task-9",
      }),
    );
    await waitFor(() => expect(onNavigate).toHaveBeenCalledTimes(1));
  });

  it("navigates a block target through the same intent contract", async () => {
    vi.mocked(listNotificationDeliveries).mockResolvedValue([blockTargetDelivery]);

    render(<NotificationHistory workspaceId="w1" />);

    fireEvent.click(await screen.findByText("Your block start reminder was delivered"));

    await waitFor(() =>
      expect(publishNotificationIntent).toHaveBeenCalledWith({
        kind: "block",
        entityType: "block",
        entityId: "block-1",
      }),
    );
  });

  // Phase 8.7.6 regression: clicking a block-targeted notification must invoke
  // the settled block-navigation intent and that intent must resolve, through
  // the real path builder, to the established Today route with the preserved
  // block id. Synthetic fixture only; production code is untouched.
  it("routes a clicked block-targeted notification to the established block-navigation intent", async () => {
    vi.mocked(listNotificationDeliveries).mockResolvedValue([blockTargetDelivery]);

    render(<NotificationHistory workspaceId="w1" />);

    fireEvent.click(await screen.findByTestId("notification-row"));

    await waitFor(() => {
      expect(publishNotificationIntent).toHaveBeenCalledTimes(1);
      const intent = vi.mocked(publishNotificationIntent).mock.calls[0][0];
      expect(intent).toEqual({ kind: "block", entityType: "block", entityId: "block-1" });
      // The settled contract: the block intent resolves to the Today view,
      // preserving the target id for the highlight bridge (8.7.5).
      expect(notificationIntentPath(intent)).toBe(
        "/?notification=block&notificationTarget=block-1&notificationTargetType=block",
      );
    });
  });

  it("keeps a keyboard activation equivalent to clicking for a routable target", async () => {
    vi.mocked(listNotificationDeliveries).mockResolvedValue([blockTargetDelivery]);

    render(<NotificationHistory workspaceId="w1" />);

    fireEvent.keyDown(await screen.findByTestId("notification-row"), { key: "Enter" });

    await waitFor(() =>
      expect(publishNotificationIntent).toHaveBeenCalledWith({
        kind: "block",
        entityType: "block",
        entityId: "block-1",
      }),
    );
  });

  it("never navigates an unsupported session target and hides the click affordance", async () => {
    vi.mocked(listNotificationDeliveries).mockResolvedValue([sessionTargetDelivery]);

    render(<NotificationHistory workspaceId="w1" />);

    const row = await screen.findByTestId("notification-row");
    // The row stays informational: no button role, no activation affordance.
    expect(row.getAttribute("role")).toBeNull();
    expect(row.getAttribute("tabindex")).toBe("-1");

    fireEvent.click(row);
    expect(publishNotificationIntent).not.toHaveBeenCalled();
  });

  it("keeps a row without a target informational", async () => {
    vi.mocked(listNotificationDeliveries).mockResolvedValue([deliveredDelivery]);

    render(<NotificationHistory workspaceId="w1" />);

    const row = await screen.findByTestId("notification-row");
    expect(row.getAttribute("role")).toBeNull();

    fireEvent.click(row);
    expect(publishNotificationIntent).not.toHaveBeenCalled();
  });

  it("clears in-app history after confirmation with localized feedback", async () => {
    vi.spyOn(window, "confirm").mockReturnValue(true);
    vi.mocked(clearNotificationHistory).mockResolvedValue(3);
    const onChanged = vi.fn();

    render(<NotificationHistory workspaceId="w1" onChanged={onChanged} />);

    fireEvent.click(await screen.findByRole("button", { name: "Clear notification history" }));

    await waitFor(() => expect(clearNotificationHistory).toHaveBeenCalledWith("w1"));
    await waitFor(() => expect(screen.getByText("Cleared 3 history items.")).toBeInTheDocument());
    await waitFor(() => expect(onChanged).toHaveBeenCalled());
    // Confirm prompt is the destructive-action convention.
    await waitFor(() =>
      expect(window.confirm).toHaveBeenCalledWith(
        "Clear all notification history? This only removes the in-app history; future scheduled deliveries are kept.",
      ),
    );
  });

  it("uses the singular cleared feedback when exactly one row was removed", async () => {
    vi.spyOn(window, "confirm").mockReturnValue(true);
    vi.mocked(clearNotificationHistory).mockResolvedValue(1);

    render(<NotificationHistory workspaceId="w1" />);

    fireEvent.click(await screen.findByRole("button", { name: "Clear notification history" }));

    await waitFor(() => expect(screen.getByText("Cleared 1 history item.")).toBeInTheDocument());
  });

  it("does not clear anything when the confirmation is declined", async () => {
    render(<NotificationHistory workspaceId="w1" />);

    fireEvent.click(await screen.findByRole("button", { name: "Clear notification history" }));

    await waitFor(() => expect(window.confirm).toHaveBeenCalledTimes(1));
    expect(clearNotificationHistory).not.toHaveBeenCalled();
    expect(listNotificationDeliveries).toHaveBeenCalledTimes(1);
  });

  it("shows a friendly alert when clearing fails", async () => {
    vi.spyOn(window, "confirm").mockReturnValue(true);
    vi.mocked(clearNotificationHistory).mockRejectedValue({ code: "DATABASE" });

    render(<NotificationHistory workspaceId="w1" />);

    fireEvent.click(await screen.findByRole("button", { name: "Clear notification history" }));

    expect(
      await screen.findByText("Your local data could not be saved. Please try again."),
    ).toBeInTheDocument();
    expect(screen.queryByRole("status")).toBeNull();
  });
});
