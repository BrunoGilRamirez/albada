import { invoke } from "../tauri";
import type {
  NotificationChannel,
  NotificationDelivery,
  NotificationDeliveryKind,
  NotificationDeliveryStatus,
  ReminderEscalation,
  ReminderRule,
} from "../types";

export interface DueNotification {
  id: string;
  safeMessage: string;
  reminderType: string;
  /** Optional target entity type (e.g. "task", "project", "goal", "block", "session"). */
  targetEntityType?: string | null;
  /** Optional target entity id for deep-linking. */
  targetEntityId?: string | null;
}

/**
 * Fetch due, lock-screen-safe notifications. `runtimeLocale` is the resolved
 * frontend UI language: Rust uses it only for workspaces persisting `system`
 * and never for user content. `null`/omitted lets Rust apply its own
 * resolution rules.
 */
export function getDueNotifications(runtimeLocale?: string | null): Promise<DueNotification[]> {
  return invoke<DueNotification[]>("get_due_notifications", { runtimeLocale });
}

export function recordNotificationDelivery(id: string, delivered: boolean): Promise<void> {
  return invoke<void>("record_notification_delivery", { id, delivered });
}

export interface UpdateReminderRuleInput {
  enabled: boolean;
  preferred_time?: string | null;
  time_window_start?: string | null;
  time_window_end?: string | null;
  quiet_hours_start?: string | null;
  quiet_hours_end?: string | null;
  deadline_notification_window_days: number;
  max_reminders_per_day: number;
  snooze_minutes: number[];
  channel: NotificationChannel;
  escalation: ReminderEscalation;
  privacy_safe_message: string;
}

export interface NotificationDeliveryFiltersInput {
  workspace_id: string;
  reminder_rule_id?: string | null;
  delivery_kind?: NotificationDeliveryKind | null;
  status?: NotificationDeliveryStatus | null;
  from?: string | null;
  to?: string | null;
  limit?: number | null;
}

export function listReminderRules(workspaceId: string): Promise<ReminderRule[]> {
  return invoke<ReminderRule[]>("list_reminder_rules", { workspaceId });
}

export function updateReminderRule(
  id: string,
  input: UpdateReminderRuleInput,
): Promise<ReminderRule> {
  return invoke<ReminderRule>("update_reminder_rule", { id, dto: input });
}

export function listNotificationDeliveries(
  filters: NotificationDeliveryFiltersInput,
): Promise<NotificationDelivery[]> {
  return invoke<NotificationDelivery[]>("list_notification_deliveries", { filters });
}

export function dismissNotification(id: string): Promise<NotificationDelivery> {
  return invoke<NotificationDelivery>("dismiss_notification", { id });
}

export function snoozeNotification(id: string, minutes: number): Promise<NotificationDelivery> {
  return invoke<NotificationDelivery>("snooze_notification", { id, minutes });
}

/**
 * Clear the in-app notification history for one workspace. Only terminal
 * (`delivered`, `dismissed`, `failed`) rows are removed; scheduled and snoozed
 * rows, future scheduled deliveries, delivery-cycle deduplication, and the
 * related blocks/tasks/projects are untouched. This never dismisses Windows
 * toast notifications. Resolves with the number of removed history rows.
 */
export function clearNotificationHistory(workspaceId: string): Promise<number> {
  return invoke<number>("clear_notification_history", { workspaceId });
}
