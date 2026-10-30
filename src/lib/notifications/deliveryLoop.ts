import { isTauri } from "@tauri-apps/api/core";
import {
  isPermissionGranted,
  onAction,
  pending,
  sendNotification,
} from "@tauri-apps/plugin-notification";
import { getDueNotifications, recordNotificationDelivery } from "../api/notificationApi";
import { getRuntimeLocale } from "../i18n";
import {
  focusMainWindow,
  intentFromNotificationExtra,
  publishNotificationIntent,
} from "./notificationIntent";

const APP_TITLE = "Albada";
const DELIVERY_POLL_INTERVAL_MS = 60_000;

let loopStarted = false;
let pollInProgress = false;
let interactionHandlersStarted = false;
const attemptedDeliveryIds = new Set<string>();

function handleNotificationAction(notification: unknown): void {
  const intent = intentFromNotificationExtra(
    notification && typeof notification === "object"
      ? (notification as { extra?: unknown }).extra
      : undefined,
  );
  if (!intent) return;
  void focusMainWindow();
  publishNotificationIntent(intent);
}

async function registerNotificationInteractionHandlers(): Promise<void> {
  if (interactionHandlersStarted || !isTauri()) return;
  interactionHandlersStarted = true;
  try {
    await onAction(handleNotificationAction);
    const pendingNotifications = await pending();
    for (const notification of pendingNotifications) handleNotificationAction(notification);
  } catch {
    console.warn("Could not register local notification actions.");
  }
}

async function pollAndDeliver(): Promise<void> {
  if (pollInProgress) return;
  pollInProgress = true;

  try {
    // The resolved UI language lets Rust localize only its own
    // application-authored defaults for workspaces in `system` mode.
    const dueNotifications = await getDueNotifications(getRuntimeLocale());
    const pending = dueNotifications.filter(({ id }) => !attemptedDeliveryIds.has(id));
    // Never request permission from the delivery loop. The in-app consent
    // dialog (and the settings page) own the only `requestPermission()` calls;
    // here we only check the current permission state.
    if (pending.length === 0 || !(await isPermissionGranted())) return;

    for (const notification of pending) {
      // Keep failures from being retried every minute in this process. The
      // repository records each attempted delivery and suppresses daily rule
      // regeneration, while stale work itself is never rescheduled.
      attemptedDeliveryIds.add(notification.id);

      // Build target metadata for deep-linking via the notification `data` field.
      // The DueNotification type now includes targetEntityType and targetEntityId
      // from the Rust side, which flow through the API and into this payload.
      const extra = {
        kind: notification.reminderType,
        ...(notification.targetEntityType && notification.targetEntityId
          ? { entityType: notification.targetEntityType, entityId: notification.targetEntityId }
          : {}),
      };

      try {
        await sendNotification({
          title: APP_TITLE,
          body: notification.safeMessage,
          extra,
          autoCancel: true,
        });
      } catch {
        await recordNotificationDelivery(notification.id, false).catch(() => {
          console.warn("Could not record a failed local notification delivery.");
        });
        continue;
      }

      await recordNotificationDelivery(notification.id, true).catch(() => {
        // Avoid an in-process duplicate if recording success failed after the
        // operating system accepted the notification.
        console.warn("Could not record a completed local notification delivery.");
      });
    }
  } catch {
    // Keep the app usable when native notifications or the local database are
    // temporarily unavailable. Do not log potentially user-configured text.
    console.warn("The local notification check could not be completed.");
  } finally {
    pollInProgress = false;
  }
}

/** Start one app-wide startup and periodic delivery loop inside Tauri. */
export function startNotificationDeliveryLoop(): void {
  if (loopStarted || !isTauri()) return;
  loopStarted = true;
  void registerNotificationInteractionHandlers();
  void pollAndDeliver();
  window.setInterval(() => void pollAndDeliver(), DELIVERY_POLL_INTERVAL_MS);
}

/**
 * Run a single delivery poll immediately, for example right after the consent
 * dialog receives permission, so app-open deadline alerts are delivered
 * without waiting for the next 60-second scheduled tick.
 */
export function triggerNotificationPoll(): Promise<void> {
  if (!isTauri()) return Promise.resolve();
  return pollAndDeliver();
}
