import { describe, expect, it, vi } from "vitest";
import { fireEvent, render, screen } from "@testing-library/react";
import PriorityEffortMatrix from "@/components/tasks/PriorityEffortMatrix";

/**
 * Phase 8.5.6 semantic UI contract: the selected cell is expressed with the
 * `joi-btn--primary` semantic modifier and unselected cells with
 * `joi-btn--secondary`, while `aria-pressed` remains the recoverable state.
 * No raw palette utility is asserted here, and no class mapping is faked.
 */

const RAW_PALETTE_UTILITY =
  /(?:^|:)(?:bg|text|border|ring|from|to|via|outline|accent|fill|stroke|shadow)-(?:slate|zinc|neutral|stone|red|orange|amber|yellow|lime|green|emerald|teal|cyan|sky|blue|indigo|violet|purple|fuchsia|pink|rose)-[0-9]{2,3}$/;

function expectNoRawPaletteUtility(element: Element) {
  const offenders = Array.from(element.classList).filter((token) =>
    RAW_PALETTE_UTILITY.test(token),
  );
  expect(offenders).toEqual([]);
}

function cells(): HTMLButtonElement[] {
  return screen.getAllByRole("button");
}

describe("PriorityEffortMatrix", () => {
  it("renders all priority columns and effort rows", () => {
    render(<PriorityEffortMatrix effort="medium" priority="high" onSelect={() => undefined} />);

    for (const label of ["Low", "Normal", "High", "Critical"]) {
      expect(screen.getByRole("columnheader", { name: label })).toBeTruthy();
    }
    for (const label of ["Trivial", "Small", "Medium", "Large", "Very large"]) {
      expect(screen.getByText(label)).toBeTruthy();
    }
  });

  it("calls onSelect with the chosen effort and priority", () => {
    const onSelect = vi.fn();
    render(<PriorityEffortMatrix effort={null} priority="none" onSelect={onSelect} />);

    fireEvent.click(screen.getByRole("button", { name: "Address today" }));
    expect(onSelect).toHaveBeenCalledWith("trivial", "critical");
  });

  it("highlights the selected cell", () => {
    const onSelect = vi.fn();
    render(<PriorityEffortMatrix effort="large" priority="critical" onSelect={onSelect} />);

    const selected = screen.getByRole("button", { name: "Urgently divide" });
    expect(selected.getAttribute("aria-pressed")).toBe("true");
  });

  it("marks exactly the selected cell as pressed and the rest as unpressed", () => {
    render(<PriorityEffortMatrix effort="small" priority="normal" onSelect={() => undefined} />);

    const pressed = cells().filter((cell) => cell.getAttribute("aria-pressed") === "true");
    const unpressed = cells().filter((cell) => cell.getAttribute("aria-pressed") === "false");

    expect(pressed).toHaveLength(1);
    // 5 effort rows x 4 priority columns = 20 cells, one of them pressed.
    expect(unpressed).toHaveLength(19);
    expect(pressed[0].textContent).toBe("Plan");
  });

  it("applies joi-btn--primary only to the pressed cell", () => {
    render(<PriorityEffortMatrix effort="large" priority="critical" onSelect={() => undefined} />);

    const selected = screen.getByRole("button", { name: "Urgently divide" });
    expect(selected.classList.contains("joi-btn--primary")).toBe(true);
    expect(selected.classList.contains("joi-btn--secondary")).toBe(false);

    const primaryCells = cells().filter((cell) => cell.classList.contains("joi-btn--primary"));
    expect(primaryCells).toHaveLength(1);
    expect(primaryCells[0]).toBe(selected);

    for (const cell of cells()) {
      expect(cell.classList.contains("joi-btn")).toBe(true);
      if (cell !== selected) {
        expect(cell.classList.contains("joi-btn--secondary")).toBe(true);
        expect(cell.classList.contains("joi-btn--primary")).toBe(false);
      }
      expectNoRawPaletteUtility(cell);
    }
  });

  it("keeps the same modifier split for the very_large / none fallback cell", () => {
    // `priority="none"` falls back to the normal column, so the selected cell is
    // very_large/normal and there is still exactly one primary cell.
    render(<PriorityEffortMatrix effort="very_large" priority="none" onSelect={() => undefined} />);

    const pressed = cells().filter((cell) => cell.getAttribute("aria-pressed") === "true");
    expect(pressed).toHaveLength(1);
    expect(pressed[0].textContent).toBe("Divide");

    const primaryCells = cells().filter((cell) => cell.classList.contains("joi-btn--primary"));
    expect(primaryCells).toHaveLength(1);
    const secondaryCells = cells().filter((cell) => cell.classList.contains("joi-btn--secondary"));
    expect(secondaryCells).toHaveLength(19);
  });
});
