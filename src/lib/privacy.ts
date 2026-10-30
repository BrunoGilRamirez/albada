/** Labels used by journal privacy badges and other privacy-aware UI. */
export const PRIVACY_SCOPE_LABELS: Record<string, string> = {
  private: "Private",
  personal: "Personal",
  therapy_private: "Therapy private",
  export_eligible: "Export eligible",
};

export function isTherapyPrivate(scope: string): boolean {
  return scope === "therapy_private";
}

export function isJournalPrivate(scope: string): boolean {
  return scope === "private" || isTherapyPrivate(scope);
}

export function isExportEligible(scope: string): boolean {
  return scope === "export_eligible";
}

/**
 * Returns true when a record may appear in a generic (non-explicit) export.
 * Therapy-private content and private journal entries are excluded; only
 * explicit selection and preview may include them.
 */
export function canIncludeInGenericExport(scope: string): boolean {
  return scope === "export_eligible" || scope === "personal";
}

/**
 * Builds lock-screen-safe notification text.
 * Journal and therapy content must never be passed as `baseMessage`.
 */
export function buildPrivacySafeNotification(
  baseMessage: string,
  sensitiveContent?: string | null,
): { title: string; body: string } {
  // Deliberately do not read or interpolate sensitiveContent.
  void sensitiveContent;
  return { title: baseMessage, body: "Open Albada to view." };
}
