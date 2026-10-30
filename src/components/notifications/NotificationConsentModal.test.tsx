import { beforeEach, describe, expect, it, vi } from "vitest";
import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import NotificationConsentModal from "@/components/notifications/NotificationConsentModal";
import { isPermissionGranted, requestPermission } from "@tauri-apps/plugin-notification";
import { triggerNotificationPoll } from "@/lib/notifications/deliveryLoop";
import { NOTIFICATION_CONSENT_STORAGE_KEY } from "@/lib/notifications/consent";

vi.mock("@tauri-apps/api/core", () => ({
  isTauri: () => true,
}));

vi.mock("@tauri-apps/plugin-notification", () => ({
  isPermissionGranted: vi.fn(),
  requestPermission: vi.fn(),
}));

vi.mock("@/lib/notifications/deliveryLoop", () => ({
  triggerNotificationPoll: vi.fn(),
}));

describe("NotificationConsentModal", () => {
  beforeEach(() => {
    localStorage.clear();
    vi.mocked(isPermissionGranted).mockReset();
    vi.mocked(requestPermission).mockReset();
    vi.mocked(triggerNotificationPoll).mockReset();

    vi.mocked(isPermissionGranted).mockResolvedValue(false);
    vi.mocked(requestPermission).mockResolvedValue("granted");
    vi.mocked(triggerNotificationPoll).mockResolvedValue(undefined);
  });

  it("shows the consent dialog on first startup when permission is not granted", async () => {
    render(<NotificationConsentModal />);

    const dialog = await screen.findByRole("dialog");
    expect(dialog).toHaveTextContent(/Allow desktop notifications\?/i);
    expect(dialog).toHaveTextContent(/local reminders/i);
    expect(dialog).toHaveTextContent(/lock-screen safe/i);
    expect(dialog).toHaveTextContent(
      /never include task titles, journal excerpts, or therapy content/i,
    );
    // The dialog itself must never trigger the OS prompt.
    expect(requestPermission).not.toHaveBeenCalled();
  });

  it("does not show the dialog when permission is already granted and records the outcome", async () => {
    vi.mocked(isPermissionGranted).mockResolvedValue(true);

    render(<NotificationConsentModal />);

    await waitFor(() =>
      expect(localStorage.getItem(NOTIFICATION_CONSENT_STORAGE_KEY)).toBe("already-granted"),
    );
    expect(screen.queryByRole("dialog")).toBeNull();
    expect(requestPermission).not.toHaveBeenCalled();
  });

  it("stays hidden without any permission check once a prompt decision exists", () => {
    localStorage.setItem(NOTIFICATION_CONSENT_STORAGE_KEY, "not-now");

    render(<NotificationConsentModal />);

    expect(screen.queryByRole("dialog")).toBeNull();
    expect(isPermissionGranted).not.toHaveBeenCalled();
  });

  it("calls requestPermission, persists the decision, and polls deliveries after allowing", async () => {
    render(<NotificationConsentModal />);
    await screen.findByRole("dialog");

    fireEvent.click(screen.getByRole("button", { name: /Allow notifications/i }));

    await waitFor(() => expect(requestPermission).toHaveBeenCalled());
    await waitFor(() => expect(triggerNotificationPoll).toHaveBeenCalled());
    await waitFor(() => expect(screen.queryByRole("dialog")).toBeNull());
    expect(localStorage.getItem(NOTIFICATION_CONSENT_STORAGE_KEY)).toBe("allowed");
  });

  it("does not request the OS permission when the user chooses not now", async () => {
    render(<NotificationConsentModal />);
    await screen.findByRole("dialog");

    fireEvent.click(screen.getByRole("button", { name: /Not now/i }));

    await waitFor(() => expect(screen.queryByRole("dialog")).toBeNull());
    expect(requestPermission).not.toHaveBeenCalled();
    expect(triggerNotificationPoll).not.toHaveBeenCalled();
    expect(localStorage.getItem(NOTIFICATION_CONSENT_STORAGE_KEY)).toBe("not-now");

    // A later startup with the stored decision must not show the dialog again.
    render(<NotificationConsentModal />);
    expect(screen.queryByRole("dialog")).toBeNull();
  });

  it("does not trigger the delivery poll when the OS prompt ends up denied", async () => {
    vi.mocked(requestPermission).mockResolvedValue("denied");

    render(<NotificationConsentModal />);
    await screen.findByRole("dialog");

    fireEvent.click(screen.getByRole("button", { name: /Allow notifications/i }));

    await waitFor(() => expect(screen.queryByRole("dialog")).toBeNull());
    expect(triggerNotificationPoll).not.toHaveBeenCalled();
    expect(localStorage.getItem(NOTIFICATION_CONSENT_STORAGE_KEY)).toBe("allowed");
  });
});
