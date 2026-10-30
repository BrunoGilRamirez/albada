// Local prompt-decision flag for the first-startup notification consent dialog.
// Only the tiny decision value is stored; no task, journal, or therapy data is
// ever written here. The OS permission itself stays owned by Windows/Tauri.

export const NOTIFICATION_CONSENT_STORAGE_KEY = "joi-projectlife-notification-consent";

export type NotificationConsentDecision = "allowed" | "not-now" | "already-granted";

const KNOWN_DECISIONS: Record<NotificationConsentDecision, string> = {
  allowed: "",
  "not-now": "",
  "already-granted": "",
};

/** Read the persisted prompt decision, or `null` when no decision exists yet. */
export function getNotificationConsentDecision(): NotificationConsentDecision | null {
  try {
    const stored = localStorage.getItem(NOTIFICATION_CONSENT_STORAGE_KEY);
    if (stored && stored in KNOWN_DECISIONS) {
      return stored as NotificationConsentDecision;
    }
  } catch {
    // localStorage unavailable (private mode, blocked storage); treat as no
    // decision so the dialog can still offer an explicit choice.
  }
  return null;
}

/** Persist the prompt decision so the same consent dialog is not repeated. */
export function setNotificationConsentDecision(decision: NotificationConsentDecision): void {
  try {
    localStorage.setItem(NOTIFICATION_CONSENT_STORAGE_KEY, decision);
  } catch {
    // Storage full or unavailable; the dialog may appear again next startup.
  }
}
