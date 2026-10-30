import { createContext, useContext, useEffect, useMemo, useState, type ReactNode } from "react";
import { isTauri } from "@tauri-apps/api/core";
import type { NotificationTargetEntityType } from "@/lib/types";

export type NotificationIntentKind =
  "planning" | "focus" | "deadline" | NotificationTargetEntityType;

export interface NotificationIntent {
  kind: NotificationIntentKind;
  entityType?: NotificationTargetEntityType;
  entityId?: string;
}

type IntentListener = (intent: NotificationIntent) => void;
const listeners = new Set<IntentListener>();
let pendingIntent: NotificationIntent | null = null;

export function subscribeToNotificationIntents(listener: IntentListener): () => void {
  listeners.add(listener);
  return () => listeners.delete(listener);
}

export function publishNotificationIntent(intent: NotificationIntent): void {
  pendingIntent = intent;
  for (const listener of listeners) listener(intent);
}

const NotificationIntentContext = createContext<{
  intent: NotificationIntent | null;
  clearIntent: () => void;
}>({ intent: null, clearIntent: () => undefined });

export function NotificationIntentProvider({ children }: { children: ReactNode }) {
  const [intent, setIntent] = useState<NotificationIntent | null>(() => pendingIntent);

  useEffect(() => subscribeToNotificationIntents(setIntent), []);

  const value = useMemo(
    () => ({
      intent,
      clearIntent: () => {
        pendingIntent = null;
        setIntent(null);
      },
    }),
    [intent],
  );
  return (
    <NotificationIntentContext.Provider value={value}>
      {children}
    </NotificationIntentContext.Provider>
  );
}

export function useNotificationIntent() {
  return useContext(NotificationIntentContext);
}

const ENTITY_TYPES: readonly NotificationTargetEntityType[] = [
  "task",
  "project",
  "goal",
  "block",
  "session",
];

/**
 * Target entity types that have an in-app destination page. A `session`
 * target—or any future unknown type—has no dedicated route, so history rows
 * carrying it must keep their informational presentation and never silently
 * navigate to an unrelated page (Phase 8.7.5).
 */
const ROUTABLE_TARGET_ENTITY_TYPES: readonly NotificationTargetEntityType[] = [
  "task",
  "project",
  "goal",
  "block",
];

export function isRoutableNotificationTarget(entityType: NotificationTargetEntityType): boolean {
  return ROUTABLE_TARGET_ENTITY_TYPES.includes(entityType);
}

export function intentFromNotificationExtra(extra: unknown): NotificationIntent | null {
  if (!extra || typeof extra !== "object") return null;
  const data = extra as Record<string, unknown>;
  const entityType = data.entityType ?? data.entity_type;
  const entityId = data.entityId ?? data.entity_id;
  const kind = data.kind;

  if (typeof kind === "string" && ["planning", "focus", "deadline"].includes(kind)) {
    return { kind: kind as NotificationIntentKind, entityType: undefined, entityId: undefined };
  }
  if (
    typeof entityType === "string" &&
    ENTITY_TYPES.includes(entityType as NotificationTargetEntityType) &&
    typeof entityId === "string" &&
    entityId.length > 0
  ) {
    return {
      kind: entityType as NotificationTargetEntityType,
      entityType: entityType as NotificationTargetEntityType,
      entityId,
    };
  }
  return null;
}

export function notificationIntentPath(intent: NotificationIntent): string {
  const target = intent.entityId
    ? `&notificationTarget=${encodeURIComponent(intent.entityId)}`
    : "";
  const type = intent.entityType ? `&notificationTargetType=${intent.entityType}` : "";
  switch (intent.kind) {
    case "task":
      return `/tasks/${encodeURIComponent(intent.entityId ?? "")}?notification=task${target}${type}`;
    case "project":
      return `/projects/${encodeURIComponent(intent.entityId ?? "")}?notification=project${target}${type}`;
    case "goal":
      return `/goals/${encodeURIComponent(intent.entityId ?? "")}?notification=goal${target}${type}`;
    case "block":
      // Blocks are reviewed on the Today view; the block card carries the
      // matching `data-notification-target` marker for the highlight bridge.
      return `/?notification=block${target}${type}`;
    case "deadline":
      return `/tasks?notification=deadline`;
    case "planning":
    case "focus":
      return `/?notification=${intent.kind}`;
    default:
      return "/";
  }
}

export async function focusMainWindow(): Promise<void> {
  try {
    if (!isTauri()) return;
    // Keep the window API as a genuine lazy chunk: it is referenced only
    // here, unlike @tauri-apps/api/core which the app already imports
    // statically.
    const { getCurrentWindow } = await import("@tauri-apps/api/window");
    const window = getCurrentWindow();
    await window.unminimize();
    await window.show();
    await window.setFocus();
  } catch {
    // Browser/test environments and unsupported window states are harmless.
  }
}
