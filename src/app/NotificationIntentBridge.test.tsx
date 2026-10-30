import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { act, cleanup, render } from "@testing-library/react";
import { MemoryRouter, Route, Routes, useLocation } from "react-router";
import { NotificationIntentBridge } from "@/app/layout";
import {
  NotificationIntentProvider,
  publishNotificationIntent,
  type NotificationIntent,
} from "@/lib/notifications/notificationIntent";

/**
 * Phase 8.7.5 focused bridge coverage: the target id survives navigation, the
 * highlight lasts about one second, the treatment is reduced-motion-aware and
 * non-color (theme outline), and missing targets fail safely without
 * navigating anywhere else. Synthetic fixtures only.
 */

let lastLocation: string | null = null;
// This jsdom environment ships no window.matchMedia at all; restore whatever
// existed (possibly undefined) after the suite.
let originalMatchMedia: typeof window.matchMedia | undefined;

function LocationProbe() {
  const location = useLocation();
  lastLocation = `${location.pathname}${location.search}`;
  return null;
}

function stubMatchMedia(matches: boolean) {
  originalMatchMedia = window.matchMedia;
  const stub = ((query: string) => {
    return {
      matches,
      media: query,
      onchange: null,
      addListener: vi.fn(),
      removeListener: vi.fn(),
      addEventListener: vi.fn(),
      removeEventListener: vi.fn(),
      dispatchEvent: vi.fn(),
    } as unknown as MediaQueryList;
  }) as unknown as typeof window.matchMedia;
  window.matchMedia = stub;
}

function renderBridge(initialEntry: string, targetMarker?: string) {
  return render(
    <MemoryRouter initialEntries={[initialEntry]}>
      <NotificationIntentBridge />
      {targetMarker ? (
        <div data-notification-target={targetMarker} tabIndex={-1}>
          Marker
        </div>
      ) : null}
    </MemoryRouter>,
  );
}

describe("NotificationIntentBridge (Phase 8.7.5)", () => {
  beforeEach(() => {
    // jsdom does not implement scrollIntoView; the bridge must call it without
    // crashing so its behavior is observable here.
    Element.prototype.scrollIntoView = vi.fn();
    lastLocation = null;
  });

  afterEach(() => {
    cleanup();
    vi.useRealTimers();
    window.matchMedia = originalMatchMedia as typeof window.matchMedia;
    vi.restoreAllMocks();
  });

  it("navigates a published task intent and preserves the target id", () => {
    render(
      <NotificationIntentProvider>
        <MemoryRouter initialEntries={["/"]}>
          <NotificationIntentBridge />
          <Routes>
            <Route path="*" element={<LocationProbe />} />
          </Routes>
        </MemoryRouter>
      </NotificationIntentProvider>,
    );

    act(() => {
      publishNotificationIntent({
        kind: "task",
        entityType: "task",
        entityId: "task-9",
      });
    });

    expect(lastLocation).toBe(
      "/tasks/task-9?notification=task&notificationTarget=task-9&notificationTargetType=task",
    );
  });

  it("routes a block intent to the Today view with the preserved block id", () => {
    render(
      <NotificationIntentProvider>
        <MemoryRouter initialEntries={["/"]}>
          <NotificationIntentBridge />
          <Routes>
            <Route path="*" element={<LocationProbe />} />
          </Routes>
        </MemoryRouter>
      </NotificationIntentProvider>,
    );

    act(() => {
      publishNotificationIntent({
        kind: "block",
        entityType: "block",
        entityId: "block-1",
      });
    });

    // The Today block card carries the matching `data-notification-target`.
    expect(lastLocation).toBe(
      "/?notification=block&notificationTarget=block-1&notificationTargetType=block",
    );
    expect(Element.prototype.scrollIntoView).not.toHaveBeenCalled();
  });

  it("highlights a found target for about one second and scrolls smoothly by default", () => {
    vi.useFakeTimers();
    stubMatchMedia(false);

    renderBridge("/?notification=block&notificationTarget=b1", "b1");

    const marker = document.querySelector<HTMLElement>("[data-notification-target='b1']");
    expect(marker?.classList.contains("notification-target-highlight")).toBe(true);
    expect(Element.prototype.scrollIntoView).toHaveBeenCalledWith({
      behavior: "smooth",
      block: "center",
    });

    act(() => {
      vi.advanceTimersByTime(1000);
    });
    expect(marker?.classList.contains("notification-target-highlight")).toBe(false);
  });

  it("jumps instantly instead of smooth-scrolling under prefers-reduced-motion", () => {
    vi.useFakeTimers();
    stubMatchMedia(true);

    renderBridge("/?notification=block&notificationTarget=b1", "b1");

    const marker = document.querySelector<HTMLElement>("[data-notification-target='b1']");
    expect(marker?.classList.contains("notification-target-highlight")).toBe(true);
    expect(Element.prototype.scrollIntoView).toHaveBeenCalledWith({
      behavior: "auto",
      block: "center",
    });

    act(() => {
      vi.advanceTimersByTime(1000);
    });
    expect(marker?.classList.contains("notification-target-highlight")).toBe(false);
  });

  it("fails safely for a missing target: no highlight and no unrelated navigation", () => {
    vi.useFakeTimers();
    stubMatchMedia(false);

    renderBridge("/?notification=task&notificationTarget=missing-1", "other-1");

    act(() => {
      // Repeated polling retries stay harmless; nothing else is highlighted
      // and the page is never rerouted.
      vi.advanceTimersByTime(500);
    });

    expect(document.querySelector(".notification-target-highlight")).toBeNull();
    expect(lastLocation).toBeNull();
  });

  it("does not re-navigate a repeated identical intent", () => {
    render(
      <NotificationIntentProvider>
        <MemoryRouter initialEntries={["/"]}>
          <NotificationIntentBridge />
          <Routes>
            <Route path="*" element={<LocationProbe />} />
          </Routes>
        </MemoryRouter>
      </NotificationIntentProvider>,
    );

    const intent: NotificationIntent = {
      kind: "task",
      entityType: "task",
      entityId: "task-9",
    };
    act(() => {
      publishNotificationIntent(intent);
    });
    act(() => {
      publishNotificationIntent(intent);
    });

    expect(lastLocation).toBe(
      "/tasks/task-9?notification=task&notificationTarget=task-9&notificationTargetType=task",
    );
  });
});
