import { describe, expect, it, vi } from "vitest";
import { fireEvent, render, screen } from "@testing-library/react";
import AppToaster from "@/components/AppToaster";

/**
 * Phase 8.5.6 semantic UI contract: toast kind maps to the semantic
 * `joi-toast--*` modifier, never to a raw Tailwind palette utility. Dismissal
 * behavior and the polite live region are asserted as-is, without changing
 * component behavior.
 */

const RAW_PALETTE_UTILITY =
  /(?:^|:)(?:bg|text|border|ring|from|to|via|outline|accent|fill|stroke|shadow)-(?:slate|zinc|neutral|stone|red|orange|amber|yellow|lime|green|emerald|teal|cyan|sky|blue|indigo|violet|purple|fuchsia|pink|rose)-[0-9]{2,3}$/;

function expectNoRawPaletteUtility(element: Element) {
  const offenders = Array.from(element.classList).filter((token) =>
    RAW_PALETTE_UTILITY.test(token),
  );
  expect(offenders).toEqual([]);
}

describe("AppToaster", () => {
  it("renders no toast when the queue is empty", () => {
    const { container } = render(<AppToaster toasts={[]} onDismiss={() => undefined} />);
    expect(container.querySelectorAll(".joi-toast")).toHaveLength(0);
  });

  it("exposes the toast area as a polite live region", () => {
    const { container } = render(<AppToaster toasts={[]} onDismiss={() => undefined} />);
    const live = container.querySelector('[aria-live="polite"]');
    expect(live).not.toBeNull();
  });

  it("maps error kind to the error toast modifier", () => {
    render(
      <AppToaster
        toasts={[{ id: 1, message: "Could not save the task", kind: "error" }]}
        onDismiss={() => undefined}
      />,
    );
    const toast = screen.getByText("Could not save the task");
    expect(toast.classList.contains("joi-toast")).toBe(true);
    expect(toast.classList.contains("joi-toast--error")).toBe(true);
    expectNoRawPaletteUtility(toast);
  });

  it("maps success kind to the success toast modifier", () => {
    render(
      <AppToaster
        toasts={[{ id: 1, message: "Task saved", kind: "success" }]}
        onDismiss={() => undefined}
      />,
    );
    const toast = screen.getByText("Task saved");
    expect(toast.classList.contains("joi-toast--success")).toBe(true);
    expect(toast.classList.contains("joi-toast--error")).toBe(false);
    expectNoRawPaletteUtility(toast);
  });

  it("maps info kind to the info toast modifier", () => {
    render(
      <AppToaster
        toasts={[{ id: 1, message: "Import finished", kind: "info" }]}
        onDismiss={() => undefined}
      />,
    );
    const toast = screen.getByText("Import finished");
    expect(toast.classList.contains("joi-toast--info")).toBe(true);
    expectNoRawPaletteUtility(toast);
  });

  it("gives each toast a distinct modifier in a mixed queue", () => {
    const { container } = render(
      <AppToaster
        toasts={[
          { id: 1, message: "error toast", kind: "error" },
          { id: 2, message: "success toast", kind: "success" },
          { id: 3, message: "info toast", kind: "info" },
        ]}
        onDismiss={() => undefined}
      />,
    );
    const toasts = Array.from(container.querySelectorAll(".joi-toast"));
    expect(toasts).toHaveLength(3);
    const modifiers = toasts.map((toast) =>
      Array.from(toast.classList).find((token) => token.startsWith("joi-toast--")),
    );
    expect(new Set(modifiers).size).toBe(3);
    for (const toast of toasts) expectNoRawPaletteUtility(toast);
  });

  it("dismisses a toast by id when it is activated", () => {
    const onDismiss = vi.fn();
    render(
      <AppToaster
        toasts={[
          { id: 7, message: "first toast", kind: "info" },
          { id: 8, message: "second toast", kind: "error" },
        ]}
        onDismiss={onDismiss}
      />,
    );

    fireEvent.click(screen.getByText("second toast"));
    expect(onDismiss).toHaveBeenCalledWith(8);
    expect(onDismiss).not.toHaveBeenCalledWith(7);
  });
});
