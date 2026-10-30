import { useCallback, useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { isTauri } from "@tauri-apps/api/core";
import { isPermissionGranted, requestPermission } from "@tauri-apps/plugin-notification";
import {
  getNotificationConsentDecision,
  setNotificationConsentDecision,
} from "@/lib/notifications/consent";
import { triggerNotificationPoll } from "@/lib/notifications/deliveryLoop";

/**
 * First-startup consent dialog for local desktop notifications.
 *
 * Shown once inside Tauri when the OS permission is not granted yet and the
 * user has no stored prompt decision. "Allow" is the only place in the app
 * that calls `requestPermission()` outside the settings retry path; "Not now"
 * never touches the OS prompt. After an explicit choice the small local
 * decision flag is persisted so the dialog does not repeat on every startup.
 * The dialog only contains generic privacy-safe copy - never task titles,
 * journal excerpts, or therapy content.
 */
function NotificationConsentModal() {
  const { t } = useTranslation();
  const [visible, setVisible] = useState(false);
  const [busy, setBusy] = useState(false);
  const dialogRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    // Only ask inside the desktop shell; the permission API is unavailable
    // in plain browser/test runs outside Tauri.
    if (!isTauri()) return;
    // A stored decision means the user (or the OS) already resolved this.
    if (getNotificationConsentDecision() !== null) return;

    let cancelled = false;
    const checkPermission = async () => {
      try {
        const granted = await isPermissionGranted();
        if (cancelled) return;
        if (granted) {
          // Permission already exists (e.g. granted from Windows settings or
          // the retry path); record the outcome so the dialog stays hidden.
          setNotificationConsentDecision("already-granted");
        } else {
          setVisible(true);
        }
      } catch {
        // The notification plugin cannot resolve outside a configured desktop
        // shell; leave the retry path in Settings instead of blocking startup.
      }
    };
    void checkPermission();
    return () => {
      cancelled = true;
    };
  }, []);

  // Move focus into the dialog when it opens so keyboard and screen-reader
  // users land on the choice they have to make.
  useEffect(() => {
    if (visible) dialogRef.current?.focus();
  }, [visible]);

  const handleAllow = useCallback(async () => {
    setBusy(true);
    let granted: boolean;
    try {
      granted = (await requestPermission()) === "granted";
    } catch {
      granted = false;
    }
    setNotificationConsentDecision("allowed");
    setBusy(false);
    setVisible(false);
    if (granted) {
      // Deliver app-open deadline alerts now instead of waiting for the next
      // scheduled poll tick.
      void triggerNotificationPoll();
    }
  }, []);

  const handleNotNow = useCallback(() => {
    // Explicitly chosen: do not call requestPermission() here; the settings
    // page keeps the retry path available.
    setNotificationConsentDecision("not-now");
    setVisible(false);
  }, []);

  if (!visible) return null;

  return (
    <div className="joi-dialog__backdrop z-50">
      <div
        ref={dialogRef}
        role="dialog"
        aria-modal="true"
        aria-labelledby="notification-consent-title"
        aria-describedby="notification-consent-description"
        tabIndex={-1}
        className="joi-surface--base w-full max-w-lg rounded-xl p-6 shadow-xl"
      >
        <h2 id="notification-consent-title" className="joi-dialog__title text-lg font-semibold">
          {t("notifications.consent.title")}
        </h2>
        <p id="notification-consent-description" className="joi-text--muted mt-2 text-sm">
          {t("notifications.consent.body")}
        </p>
        <p className="joi-text--muted mt-3 text-sm">{t("notifications.consent.privacy")}</p>
        <div className="mt-6 flex flex-col gap-3 sm:flex-row">
          <button
            type="button"
            onClick={handleAllow}
            disabled={busy}
            className="joi-btn joi-btn--primary"
          >
            {busy ? t("notifications.consent.requesting") : t("notifications.consent.allow")}
          </button>
          <button
            type="button"
            onClick={handleNotNow}
            disabled={busy}
            className="joi-btn joi-btn--secondary"
          >
            {t("notifications.consent.notNow")}
          </button>
        </div>
        <p className="joi-text--muted mt-4 text-xs">{t("notifications.consent.later")}</p>
      </div>
    </div>
  );
}

export default NotificationConsentModal;
