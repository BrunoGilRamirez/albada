import i18n from "i18next";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { act, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import NotificationsSettingsPage from "@/app/settings/notifications/page";
import NotificationHistory from "@/components/notifications/NotificationHistory";
import NotificationConsentModal from "@/components/notifications/NotificationConsentModal";
import {
  listNotificationDeliveries,
  listReminderRules,
  updateReminderRule,
} from "@/lib/api/notificationApi";
import { ensureDefaultWorkspace } from "@/lib/api/workspaceApi";
import { isPermissionGranted, requestPermission } from "@tauri-apps/plugin-notification";
import { triggerNotificationPoll } from "@/lib/notifications/deliveryLoop";
import { NOTIFICATION_CONSENT_STORAGE_KEY } from "@/lib/notifications/consent";
import { SUPPORTED_LOCALES, type AppLocale } from "@/lib/i18n/locale";
import type { NotificationDelivery, ReminderRule } from "@/lib/types";

vi.mock("@/lib/api/notificationApi", () => ({
  listReminderRules: vi.fn(),
  updateReminderRule: vi.fn(),
  listNotificationDeliveries: vi.fn(),
  dismissNotification: vi.fn(),
  snoozeNotification: vi.fn(),
  clearNotificationHistory: vi.fn(),
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

vi.mock("@/lib/notifications/deliveryLoop", () => ({
  triggerNotificationPoll: vi.fn(),
}));

// 8.6.11 Notification locale coverage: the desktop permission states, the
// rule form labels/validation, the history list chrome, and the first-startup
// consent dialog all follow the active locale. The privacy-safe message is
// user-configured copy stored natively and stays byte-exact in every language;
// only counts, statuses, and the surrounding chrome translate. Therapy
// isolation is asserted explicitly: no therapy chrome may appear on these
// shared surfaces. Synthetic fixtures only.

const SAFE_MESSAGE = "Synthetic safe reminder";

function planningRule(overrides: Partial<ReminderRule> = {}): ReminderRule {
  return {
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
    snooze_minutes: [10, 30],
    channel: "desktop",
    escalation: "none",
    privacy_safe_message: SAFE_MESSAGE,
    deliveries_today: 0,
    has_delivery_today: false,
    ...overrides,
  };
}

function scheduledDelivery(overrides: Partial<NotificationDelivery> = {}): NotificationDelivery {
  return {
    id: "d1",
    workspace_id: "w1",
    delivery_kind: "reminder_rule",
    reminder_rule_id: "r1",
    target: null,
    scheduled_at: "2026-10-05T09:00:00Z",
    delivered_at: null,
    status: "scheduled",
    channel: "desktop",
    message: SAFE_MESSAGE,
    error_message: null,
    created_at: "2026-10-05T08:00:00Z",
    cycle_key: null,
    ...overrides,
  };
}

interface NotificationCopy {
  title: string;
  permissionTitle: string;
  permissionAria: string;
  permissionGranted: string;
  permissionNotGranted: string;
  permissionRequest: string;
  privacyNote: string;
  rulesEmpty: string;
  planningType: string;
  deadlineType: string;
  preferredTime: string;
  quietStart: string;
  maxPerDay: string;
  snooze: string;
  safeMessage: string;
  saveRule: string;
  ruleSaved: string;
  channelDesktop: string;
  escalationNone: string;
  sentNone: string;
  sentOne: string;
  sentTwo: string;
  historyTitle: string;
  historyEmpty: string;
  statusScheduled: string;
  statusDelivered: string;
  statusFailed: string;
  snoozeTen: string;
  dismiss: string;
  consentTitle: string;
  consentAllow: string;
  consentNotNow: string;
  consentLater: string;
}

const COPY: Record<AppLocale, NotificationCopy> = {
  "en-US": {
    title: "Notifications",
    permissionTitle: "Desktop permission",
    permissionAria: "Notification permission",
    permissionGranted: "Notifications are allowed on this device.",
    permissionNotGranted: "Notifications are not allowed yet.",
    permissionRequest: "Request notification permission",
    privacyNote:
      "Privacy boundary: notifications show only the lock-screen-safe message configured per rule below. Task titles, journal excerpts, and therapy content are never included.",
    rulesEmpty: "No reminder rules found for this workspace yet.",
    planningType: "Daily planning",
    deadlineType: "Deadline check",
    preferredTime: "Preferred time",
    quietStart: "Quiet hours start",
    maxPerDay: "Max reminders per day",
    snooze: "Snooze options (minutes)",
    safeMessage: "Privacy-safe message",
    saveRule: "Save rule",
    ruleSaved: "Rule saved.",
    channelDesktop: "Desktop",
    escalationNone: "No escalation",
    sentNone: "no reminders sent today",
    sentOne: "1 reminder sent today",
    sentTwo: "2 reminders sent today",
    historyTitle: "Recent notifications",
    historyEmpty: "No notifications have been scheduled yet.",
    statusScheduled: "Scheduled",
    statusDelivered: "Delivered",
    statusFailed: "Failed",
    snoozeTen: "Snooze 10 min",
    dismiss: "Dismiss",
    consentTitle: "Allow desktop notifications?",
    consentAllow: "Allow notifications",
    consentNotNow: "Not now",
    consentLater: "You can change this later in Settings → Notifications.",
  },
  "es-ES": {
    title: "Notificaciones",
    permissionTitle: "Permiso de escritorio",
    permissionAria: "Permiso de notificaciones",
    permissionGranted: "Las notificaciones están permitidas en este equipo.",
    permissionNotGranted: "Las notificaciones aún no están permitidas.",
    permissionRequest: "Solicitar permiso de notificaciones",
    privacyNote:
      "Límite de privacidad: las notificaciones muestran solo el mensaje seguro para la pantalla de bloqueo configurado en cada regla. Nunca se incluyen títulos de tareas, extractos del diario ni contenido de terapia.",
    rulesEmpty: "Aún no hay reglas de recordatorio para este espacio de trabajo.",
    planningType: "Planificación diaria",
    deadlineType: "Comprobación de fechas límite",
    preferredTime: "Hora preferida",
    quietStart: "Inicio del silencio",
    maxPerDay: "Máximo de recordatorios por día",
    snooze: "Opciones de pospón (minutos)",
    safeMessage: "Mensaje seguro para la privacidad",
    saveRule: "Guardar regla",
    ruleSaved: "Regla guardada.",
    channelDesktop: "Escritorio",
    escalationNone: "Sin escalado",
    sentNone: "ningún recordatorio enviado hoy",
    sentOne: "1 recordatorio enviado hoy",
    sentTwo: "2 recordatorios enviados hoy",
    historyTitle: "Notificaciones recientes",
    historyEmpty: "Aún no se ha programado ninguna notificación.",
    statusScheduled: "Programada",
    statusDelivered: "Entregada",
    statusFailed: "Fallida",
    snoozeTen: "Posponer 10 min",
    dismiss: "Descartar",
    consentTitle: "¿Permitir notificaciones de escritorio?",
    consentAllow: "Permitir notificaciones",
    consentNotNow: "Ahora no",
    consentLater: "Puedes cambiarlo más tarde en Configuración → Notificaciones.",
  },
  "de-DE": {
    title: "Benachrichtigungen",
    permissionTitle: "Desktop-Berechtigung",
    permissionAria: "Benachrichtigungsberechtigung",
    permissionGranted: "Benachrichtigungen sind auf diesem Gerät erlaubt.",
    permissionNotGranted: "Benachrichtigungen sind noch nicht erlaubt.",
    permissionRequest: "Benachrichtigungsberechtigung anfordern",
    privacyNote:
      "Privatsphäre-Grenze: Benachrichtigungen zeigen nur die pro Regel konfigurierte, sperrbildschirm-sichere Nachricht. Aufgabentitel, Tagebuchauszüge und Therapieinhalte sind nie enthalten.",
    rulesEmpty: "Für diesen Arbeitsbereich wurden noch keine Erinnerungsregeln gefunden.",
    planningType: "Tagesplanung",
    deadlineType: "Fristenprüfung",
    preferredTime: "Bevorzugte Zeit",
    quietStart: "Beginn der Ruhezeit",
    maxPerDay: "Maximale Erinnerungen pro Tag",
    snooze: "Schummeloptionen (Minuten)",
    safeMessage: "Datenschutzfreundliche Nachricht",
    saveRule: "Regel speichern",
    ruleSaved: "Regel gespeichert.",
    channelDesktop: "Desktop",
    escalationNone: "Keine Eskalation",
    sentNone: "heute keine Erinnerung gesendet",
    sentOne: "1 Erinnerung heute gesendet",
    sentTwo: "2 Erinnerungen heute gesendet",
    historyTitle: "Letzte Benachrichtigungen",
    historyEmpty: "Es wurden noch keine Benachrichtigungen geplant.",
    statusScheduled: "Geplant",
    statusDelivered: "Zugestellt",
    statusFailed: "Fehlgeschlagen",
    snoozeTen: "10 Min. schlummern",
    dismiss: "Ausblenden",
    consentTitle: "Desktop-Benachrichtigungen erlauben?",
    consentAllow: "Benachrichtigungen erlauben",
    consentNotNow: "Jetzt nicht",
    consentLater: "Sie können das später in Einstellungen → Benachrichtigungen ändern.",
  },
  "fr-FR": {
    title: "Notifications",
    permissionTitle: "Autorisation du bureau",
    permissionAria: "Autorisation de notification",
    permissionGranted: "Les notifications sont autorisées sur cet appareil.",
    permissionNotGranted: "Les notifications ne sont pas encore autorisées.",
    permissionRequest: "Demander l'autorisation de notification",
    privacyNote:
      "Limite de confidentialité : les notifications n'affichent que le message sûr pour l'écran de verrouillage configuré pour chaque règle. Les titres de tâches, extraits de journal et contenus de thérapie n'y figurent jamais.",
    rulesEmpty: "Aucune règle de rappel n'a encore été trouvée pour cet espace de travail.",
    planningType: "Planification quotidienne",
    deadlineType: "Vérification des échéances",
    preferredTime: "Heure préférée",
    quietStart: "Début des heures silencieuses",
    maxPerDay: "Rappels maximum par jour",
    snooze: "Options de report (minutes)",
    safeMessage: "Message respectueux de la vie privée",
    saveRule: "Enregistrer la règle",
    ruleSaved: "Règle enregistrée.",
    channelDesktop: "Bureau",
    escalationNone: "Aucune relance",
    sentNone: "aucun rappel envoyé aujourd'hui",
    sentOne: "1 rappel envoyé aujourd'hui",
    sentTwo: "2 rappels envoyés aujourd'hui",
    historyTitle: "Notifications récentes",
    historyEmpty: "Aucune notification n'a encore été planifiée.",
    statusScheduled: "Planifiée",
    statusDelivered: "Distribuée",
    statusFailed: "Échouée",
    snoozeTen: "Reporter 10 min",
    dismiss: "Ignorer",
    consentTitle: "Autoriser les notifications de bureau ?",
    consentAllow: "Autoriser les notifications",
    consentNotNow: "Pas maintenant",
    consentLater: "Vous pourrez changer cela plus tard dans Paramètres → Notifications.",
  },
};

beforeEach(() => {
  localStorage.clear();
  vi.mocked(listReminderRules).mockReset();
  vi.mocked(updateReminderRule).mockReset();
  vi.mocked(listNotificationDeliveries).mockReset();
  vi.mocked(ensureDefaultWorkspace).mockReset();
  vi.mocked(isPermissionGranted).mockReset();
  vi.mocked(requestPermission).mockReset();
  vi.mocked(triggerNotificationPoll).mockReset();

  vi.mocked(ensureDefaultWorkspace).mockResolvedValue({ id: "w1" } as never);
  vi.mocked(listReminderRules).mockResolvedValue([planningRule()]);
  vi.mocked(updateReminderRule).mockResolvedValue(planningRule({ enabled: false }));
  vi.mocked(isPermissionGranted).mockResolvedValue(true);
  vi.mocked(listNotificationDeliveries).mockResolvedValue([]);
  vi.mocked(triggerNotificationPoll).mockResolvedValue(undefined);
});

afterEach(async () => {
  localStorage.clear();
  await act(async () => {
    await i18n.changeLanguage("en-US");
  });
});

describe.each([...SUPPORTED_LOCALES])("NotificationsSettingsPage in %s", (locale) => {
  it("localizes the permission state, privacy boundary, and rule-form chrome", async () => {
    await i18n.changeLanguage(locale);
    const e = COPY[locale];

    render(<NotificationsSettingsPage />);

    expect(await screen.findByRole("heading", { level: 1, name: e.title })).toBeTruthy();
    expect(
      screen.getByRole("region", { name: e.permissionAria }) ?? screen.getByText(e.permissionTitle),
    ).toBeTruthy();
    expect(screen.getByText(e.permissionTitle)).toBeTruthy();
    // The permission probe resolves asynchronously after the first render.
    await waitFor(() =>
      expect(screen.getByTestId("permission-status")).toHaveTextContent(e.permissionGranted),
    );
    expect(screen.getByText(e.privacyNote)).toBeTruthy();

    // Rule form chrome follows the locale; the stored message stays verbatim.
    expect(screen.getByText(e.planningType)).toBeTruthy();
    const form = screen.getByRole("form", {
      name: i18n.t("notifications.rule.formAria", { type: e.planningType }),
    });
    // The channel/escalation/delivery line is a single paragraph, so it is
    // asserted on the element's text rather than as one text node.
    expect(form.textContent).toContain(`${e.channelDesktop} · ${e.escalationNone} · ${e.sentNone}`);
    expect(screen.getByLabelText(e.preferredTime)).toBeTruthy();
    expect(screen.getByLabelText(e.quietStart)).toBeTruthy();
    expect(screen.getByLabelText(e.maxPerDay)).toHaveValue(5);
    expect(screen.getByLabelText(e.snooze)).toHaveValue("10, 30");
    expect(screen.getByLabelText(e.safeMessage)).toHaveValue(SAFE_MESSAGE);
    expect(
      screen.getByRole("checkbox", {
        name: i18n.t("notifications.rule.enableAria", { type: e.planningType }),
      }),
    ).toBeChecked();
    expect(screen.getByRole("button", { name: e.saveRule })).toBeTruthy();
  });

  it("pluralizes the count-only reminders-sent text in the locale", async () => {
    await i18n.changeLanguage(locale);
    vi.mocked(listReminderRules).mockResolvedValue([
      planningRule({ id: "r1", deliveries_today: 1 }),
      planningRule({ id: "r2", deliveries_today: 2 }),
      planningRule({ id: "r3", deliveries_today: 0 }),
    ]);

    render(<NotificationsSettingsPage />);

    expect(
      (await screen.findAllByRole("heading", { level: 3, name: COPY[locale].planningType })).length,
    ).toBe(3);
    const forms = screen.getAllByRole("form");
    // One rule delivered 1 reminder, one delivered 2, one delivered none:
    // the count-only text resolves each locale's plural form.
    expect(forms[0].textContent).toContain(sentToday(1));
    expect(forms[1].textContent).toContain(sentToday(2));
    expect(forms[2].textContent).toContain(COPY[locale].sentNone);
  });

  it("shows the desktop permission request path with localized copy", async () => {
    await i18n.changeLanguage(locale);
    const e = COPY[locale];
    vi.mocked(isPermissionGranted).mockResolvedValue(false);
    vi.mocked(requestPermission).mockResolvedValue("granted");

    render(<NotificationsSettingsPage />);

    await waitFor(() =>
      expect(screen.getByTestId("permission-status")).toHaveTextContent(e.permissionNotGranted),
    );
    fireEvent.click(screen.getByRole("button", { name: e.permissionRequest }));

    await waitFor(() => expect(requestPermission).toHaveBeenCalled());
    await waitFor(() =>
      expect(screen.getByTestId("permission-status")).toHaveTextContent(e.permissionGranted),
    );
  });

  it("reports the denied permission state with localized guidance", async () => {
    await i18n.changeLanguage(locale);
    vi.mocked(isPermissionGranted).mockResolvedValue(false);
    vi.mocked(requestPermission).mockResolvedValue("denied");

    render(<NotificationsSettingsPage />);

    await screen.findByRole("button", { name: COPY[locale].permissionRequest });
    fireEvent.click(screen.getByRole("button", { name: COPY[locale].permissionRequest }));

    await waitFor(() =>
      expect(screen.getByTestId("permission-status")).toHaveTextContent(
        i18n.t("notifications.permission.denied"),
      ),
    );
  });

  it("localizes the empty-rules state", async () => {
    await i18n.changeLanguage(locale);
    vi.mocked(listReminderRules).mockResolvedValue([]);

    render(<NotificationsSettingsPage />);

    expect(await screen.findByText(COPY[locale].rulesEmpty)).toBeTruthy();
  });

  it("keeps rule validation copy localized without changing the native payload", async () => {
    await i18n.changeLanguage(locale);
    const e = COPY[locale];
    const deadlineRule = planningRule({ id: "deadline-rule", reminder_type: "deadline" });
    vi.mocked(listReminderRules).mockResolvedValue([deadlineRule]);

    render(<NotificationsSettingsPage />);

    const form = await screen.findByRole("form", {
      name: i18n.t("notifications.rule.formAria", { type: e.deadlineType }),
    });
    const quietStart = await screen.findByLabelText(e.quietStart);
    fireEvent.change(quietStart, { target: { value: "07:00" } });
    fireEvent.submit(form);
    expect(await screen.findByText(i18n.t("notifications.rule.quietHoursError"))).toBeTruthy();

    // Restore a valid overnight window before checking the next rule.
    fireEvent.change(quietStart, { target: { value: "22:00" } });

    // The deadline lead window is count-only chrome and validates in-locale.
    // Submitted through the form because the native `max` would otherwise
    // block a click on an out-of-range draft.
    const window = screen.getByLabelText(i18n.t("notifications.rule.deadlineWindow"));
    fireEvent.change(window, { target: { value: "8" } });
    fireEvent.submit(form);
    expect(await screen.findByText(i18n.t("notifications.rule.deadlineWindowError"))).toBeTruthy();

    fireEvent.change(window, { target: { value: "7" } });
    fireEvent.click(screen.getByRole("button", { name: e.saveRule }));
    await waitFor(() =>
      expect(updateReminderRule).toHaveBeenCalledWith(
        "deadline-rule",
        expect.objectContaining({ deadline_notification_window_days: 7 }),
      ),
    );
    // The user-configured privacy-safe message is echoed unchanged.
    expect(updateReminderRule).toHaveBeenCalledWith(
      "deadline-rule",
      expect.objectContaining({ privacy_safe_message: SAFE_MESSAGE }),
    );
    expect(await screen.findByText(e.ruleSaved)).toBeTruthy();
  });

  it("surfaces localized representative error copy when the native command rejects", async () => {
    await i18n.changeLanguage(locale);
    const e = COPY[locale];
    vi.mocked(updateReminderRule).mockRejectedValue({ code: "DATABASE" });

    render(<NotificationsSettingsPage />);

    fireEvent.click(await screen.findByRole("button", { name: e.saveRule }));

    expect(await screen.findByRole("alert")).toHaveTextContent(i18n.t("errors.DATABASE"));
    if (locale !== "en-US") {
      expect(i18n.t("errors.DATABASE")).not.toBe(
        "Your local data could not be saved. Please try again.",
      );
    }
  });

  it("never leaks therapy chrome onto the shared notification surfaces", async () => {
    await i18n.changeLanguage(locale);
    const e = COPY[locale];

    const { container } = render(<NotificationsSettingsPage />);
    await screen.findByText(e.planningType);

    // The privacy note is the only place that may *mention* therapy content,
    // and it states the exclusion. No therapy navigation or therapy-screen
    // chrome is rendered on this shared surface: the assertions compare whole
    // strings, so a phrase that merely names the exclusion still passes.
    expect(container.textContent).toContain(i18n.t("notifications.privacyNote"));
    expect(screen.queryByText(i18n.t("therapy.title"))).toBeNull();
    expect(screen.queryByText(i18n.t("therapy.privacyBanner"))).toBeNull();
    expect(screen.queryByText(i18n.t("therapy.isolationNote"))).toBeNull();
    expect(container.querySelector('[href="/therapy"]')).toBeNull();
  });
});

describe.each([...SUPPORTED_LOCALES])("NotificationHistory in %s", (locale) => {
  it("localizes statuses, snooze actions, and dates around the verbatim safe message", async () => {
    await i18n.changeLanguage(locale);
    const e = COPY[locale];
    const delivered = scheduledDelivery({
      id: "d2",
      reminder_rule_id: "r2",
      status: "delivered",
      delivered_at: "2026-10-05T09:01:00Z",
    });
    vi.mocked(listNotificationDeliveries).mockResolvedValue([
      scheduledDelivery(),
      delivered,
      scheduledDelivery({ id: "d3", status: "failed", reminder_rule_id: "r3" }),
    ]);
    vi.mocked(listReminderRules).mockResolvedValue([planningRule()]);

    render(<NotificationHistory workspaceId="w1" />);

    // Rows load asynchronously; await the first one before asserting.
    expect((await screen.findAllByTestId("notification-row")).length).toBeGreaterThan(0);
    expect(screen.getByText(e.historyTitle)).toBeTruthy();
    const rows = screen.getAllByTestId("notification-row");
    expect(rows).toHaveLength(3);
    expect(within(rows[0]).getByText(e.statusScheduled)).toBeTruthy();
    expect(within(rows[1]).getByText(e.statusDelivered)).toBeTruthy();
    expect(within(rows[2]).getByText(e.statusFailed)).toBeTruthy();

    // The rule's privacy-safe message is stored copy and never translated.
    expect(within(rows[0]).getByText(SAFE_MESSAGE)).toBeTruthy();
    expect(rows[0].getAttribute("aria-label")).toBe(
      i18n.t("notifications.history.rowLabel", { message: SAFE_MESSAGE }),
    );

    // Snooze durations come from the rule and are count-only chrome.
    expect(within(rows[0]).getByRole("button", { name: e.snoozeTen })).toBeTruthy();
    expect(within(rows[0]).getByRole("button", { name: e.dismiss })).toBeTruthy();
    expect(
      within(rows[1]).queryByRole("button", { name: /\Snooze|Posponer|schlummern|Reporter/i }),
    ).toBeNull();

    // Application-owned timestamps render through the shared Intl formatter.
    const scheduledAt = new Date("2026-10-05T09:00:00Z").toLocaleString(locale);
    expect(
      within(rows[0]).getByText(i18n.t("notifications.history.scheduled", { date: scheduledAt })),
    ).toBeTruthy();
    const deliveredAt = new Date("2026-10-05T09:01:00Z").toLocaleString(locale);
    expect(
      within(rows[1]).getByText(
        new RegExp(
          i18n
            .t("notifications.history.delivered", { date: deliveredAt })
            .replace(/[.*+?^${}()|[\]\\]/g, "\\$&"),
        ),
      ),
    ).toBeTruthy();
  });

  it("localizes the empty state and never renders therapy chrome", async () => {
    await i18n.changeLanguage(locale);
    vi.mocked(listNotificationDeliveries).mockResolvedValue([]);

    const { container } = render(<NotificationHistory workspaceId="w1" />);

    expect(await screen.findByText(COPY[locale].historyEmpty)).toBeTruthy();
    expect(container.querySelector('[href="/therapy"]')).toBeNull();
    expect(screen.queryByText(i18n.t("therapy.title"))).toBeNull();
    expect(screen.queryByText(i18n.t("therapy.privacyBanner"))).toBeNull();
  });
});

describe.each([...SUPPORTED_LOCALES])("NotificationConsentModal in %s", (locale) => {
  it("localizes the consent dialog copy in every language", async () => {
    await i18n.changeLanguage(locale);
    const e = COPY[locale];
    vi.mocked(isPermissionGranted).mockResolvedValue(false);

    render(<NotificationConsentModal />);

    const dialog = await screen.findByRole("dialog");
    expect(dialog).toHaveTextContent(e.consentTitle);
    expect(dialog).toHaveTextContent(e.consentLater);
    expect(screen.getByRole("button", { name: e.consentAllow })).toBeTruthy();
    expect(screen.getByRole("button", { name: e.consentNotNow })).toBeTruthy();
    expect(dialog).toHaveTextContent(i18n.t("notifications.consent.privacy"));
    // Choosing "not now" must never touch the OS prompt.
    fireEvent.click(screen.getByRole("button", { name: e.consentNotNow }));
    await waitFor(() => expect(screen.queryByRole("dialog")).toBeNull());
    expect(requestPermission).not.toHaveBeenCalled();
    expect(localStorage.getItem(NOTIFICATION_CONSENT_STORAGE_KEY)).toBe("not-now");
  });
});

describe("Notification runtime locale switching", () => {
  it("keeps the verbatim safe message and selection while the chrome switches", async () => {
    vi.mocked(listNotificationDeliveries).mockResolvedValue([scheduledDelivery()]);
    vi.mocked(listReminderRules).mockResolvedValue([planningRule()]);

    render(<NotificationsSettingsPage />);
    await screen.findByText("Daily planning");
    expect(screen.getByLabelText("Privacy-safe message")).toHaveValue(SAFE_MESSAGE);

    await act(async () => {
      await i18n.changeLanguage("fr-FR");
    });

    await waitFor(() =>
      expect(screen.getByRole("heading", { level: 1, name: "Notifications" })).toBeTruthy(),
    );
    // Chrome follows the switch…
    expect(screen.getByText("Planification quotidienne")).toBeTruthy();
    expect(screen.getByLabelText("Heure préférée")).toBeTruthy();
    // …while the user-configured message stays byte-exact.
    expect(screen.getByLabelText("Message respectueux de la vie privée")).toHaveValue(SAFE_MESSAGE);
    expect(screen.queryByText("Daily planning")).toBeNull();
  });
});

/** Count-only reminder feedback resolved in the active locale. */
function sentToday(count: number): string {
  return i18n.t("notifications.rule.sentToday", { count });
}
