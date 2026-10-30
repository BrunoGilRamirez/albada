import { beforeEach, describe, expect, it, vi } from "vitest";
import { isTauri } from "@tauri-apps/api/core";
import {
  isPermissionGranted,
  requestPermission,
  sendNotification,
} from "@tauri-apps/plugin-notification";
import { getDueNotifications, recordNotificationDelivery } from "@/lib/api/notificationApi";
import {
  startNotificationDeliveryLoop,
  triggerNotificationPoll,
} from "@/lib/notifications/deliveryLoop";

vi.mock("@tauri-apps/api/core", () => ({
  isTauri: vi.fn(),
}));

vi.mock("@tauri-apps/plugin-notification", () => ({
  isPermissionGranted: vi.fn(),
  onAction: vi.fn().mockResolvedValue(() => undefined),
  pending: vi.fn().mockResolvedValue([]),
  requestPermission: vi.fn(),
  sendNotification: vi.fn(),
}));

vi.mock("@/lib/api/notificationApi", () => ({
  getDueNotifications: vi.fn(),
  recordNotificationDelivery: vi.fn(),
}));

describe("notification delivery loop", () => {
  beforeEach(() => {
    vi.mocked(isTauri).mockReset().mockReturnValue(true);
    vi.mocked(isPermissionGranted).mockReset();
    vi.mocked(requestPermission).mockReset();
    vi.mocked(sendNotification).mockReset();
    vi.mocked(getDueNotifications).mockReset();
    vi.mocked(recordNotificationDelivery).mockReset();
  });

  it("never requests the OS permission implicitly from the startup poll", async () => {
    vi.mocked(getDueNotifications).mockResolvedValue([
      {
        id: "loop-d1",
        safeMessage: "You have tasks approaching their deadline",
        reminderType: "deadline",
      },
    ]);
    vi.mocked(isPermissionGranted).mockResolvedValue(false);

    startNotificationDeliveryLoop();

    await waitForPermissionCheck();
    expect(requestPermission).not.toHaveBeenCalled();
    expect(sendNotification).not.toHaveBeenCalled();
  });

  it("triggerNotificationPoll delivers due notifications once permission exists", async () => {
    vi.mocked(getDueNotifications).mockResolvedValue([
      {
        id: "loop-d2",
        safeMessage: "You have tasks approaching their deadline",
        reminderType: "deadline",
      },
    ]);
    vi.mocked(isPermissionGranted).mockResolvedValue(true);
    vi.mocked(sendNotification).mockResolvedValue(undefined);
    vi.mocked(recordNotificationDelivery).mockResolvedValue(undefined);

    await triggerNotificationPoll();

    // The poll sends the resolved UI language so Rust can localize its own
    // application-authored defaults for `system`-mode workspaces only.
    expect(getDueNotifications).toHaveBeenCalledWith("en-US");
    expect(requestPermission).not.toHaveBeenCalled();
    expect(sendNotification).toHaveBeenCalledWith({
      title: "Albada",
      body: "You have tasks approaching their deadline",
      extra: { kind: "deadline" },
      autoCancel: true,
    });
    expect(recordNotificationDelivery).toHaveBeenCalledWith("loop-d2", true);
  });

  it("triggerNotificationPoll resolves without touching the plugin outside Tauri", async () => {
    vi.mocked(isTauri).mockReturnValue(false);

    await expect(triggerNotificationPoll()).resolves.toBeUndefined();

    expect(getDueNotifications).not.toHaveBeenCalled();
    expect(requestPermission).not.toHaveBeenCalled();
  });
});

// The startup poll is fired without awaiting; give its promise chain a chance
// to settle before asserting on the module-level mocks.
async function waitForPermissionCheck(): Promise<void> {
  await vi.waitFor(() => expect(isPermissionGranted).toHaveBeenCalled());
  await new Promise((resolve) => window.setTimeout(resolve, 0));
}
