import { describe, expect, it } from "vitest";
import { render, screen } from "@testing-library/react";
import type { DeadlineStatus } from "@/lib/types";
import DeadlineStatusBadge from "@/components/deadlines/DeadlineStatusBadge";

/**
 * Phase 8.5.6 semantic UI contract: a deadline status is expressed through the
 * semantic badge modifier, never through a raw Tailwind palette utility. These
 * tests lock the label, the base class, the modifier role and the absence of
 * raw palette utilities so a palette regression cannot reappear.
 */

const RAW_PALETTE_UTILITY =
  /(?:^|:)(?:bg|text|border|ring|from|to|via|outline|accent|fill|stroke|shadow)-(?:slate|zinc|neutral|stone|red|orange|amber|yellow|lime|green|emerald|teal|cyan|sky|blue|indigo|violet|purple|fuchsia|pink|rose)-[0-9]{2,3}$/;

function classTokens(element: Element | null): string[] {
  if (!element) return [];
  return Array.from(element.classList);
}

function expectNoRawPaletteUtility(element: Element | null) {
  const tokens = classTokens(element);
  const offenders = tokens.filter((token) => RAW_PALETTE_UTILITY.test(token));
  expect(offenders).toEqual([]);
}

describe("DeadlineStatusBadge", () => {
  it("renders nothing when there is no deadline", () => {
    const { container } = render(<DeadlineStatusBadge status="none" />);
    expect(container.firstChild).toBeNull();
  });

  it("maps normal to the on-track success badge", () => {
    render(<DeadlineStatusBadge status="normal" />);
    const badge = screen.getByText("On track");
    expect(badge.className).toContain("joi-badge");
    expect(badge.className).toContain("joi-badge--success");
    expectNoRawPaletteUtility(badge);
  });

  it("maps approaching to the warning badge", () => {
    render(<DeadlineStatusBadge status="approaching" />);
    const badge = screen.getByText("Approaching");
    expect(badge.className).toContain("joi-badge");
    expect(badge.className).toContain("joi-badge--warning");
    expectNoRawPaletteUtility(badge);
  });

  it("maps overdue to the danger badge", () => {
    render(<DeadlineStatusBadge status="overdue" />);
    const badge = screen.getByText("Overdue");
    expect(badge.className).toContain("joi-badge");
    expect(badge.className).toContain("joi-badge--danger");
    expectNoRawPaletteUtility(badge);
  });

  it("maps blocked_risk to the caution badge", () => {
    render(<DeadlineStatusBadge status="blocked_risk" />);
    const badge = screen.getByText("Blocked at risk");
    expect(badge.className).toContain("joi-badge");
    expect(badge.className).toContain("joi-badge--caution");
    expectNoRawPaletteUtility(badge);
  });

  it("uses a distinct semantic role per non-empty status", () => {
    const cases: Array<[DeadlineStatus, string]> = [
      ["normal", "joi-badge--success"],
      ["approaching", "joi-badge--warning"],
      ["overdue", "joi-badge--danger"],
      ["blocked_risk", "joi-badge--caution"],
    ];
    const seen = new Set<string>();

    for (const [status, modifier] of cases) {
      const { container, unmount } = render(<DeadlineStatusBadge status={status} />);
      const badge = container.querySelector(".joi-badge");
      expect(badge).not.toBeNull();
      expect(badge?.className).toContain(modifier);
      const modifiers = classTokens(badge).filter((token) => token.startsWith("joi-badge--"));
      expect(modifiers).toHaveLength(1);
      seen.add(modifiers[0]);
      expectNoRawPaletteUtility(badge);
      unmount();
    }

    expect(seen.size).toBe(cases.length);
  });

  it("keeps the base badge class alongside layout utilities", () => {
    render(<DeadlineStatusBadge status="overdue" />);
    const badge = screen.getByText("Overdue");
    expect(badge.classList.contains("joi-badge")).toBe(true);
    expect(badge.classList.contains("shrink-0")).toBe(true);
    expect(badge.classList.contains("joi-badge--danger")).toBe(true);
  });

  it("shows the deadline timestamp inside the badge without a second modifier", () => {
    const { container } = render(<DeadlineStatusBadge status="overdue" deadlineAt="2026-10-10" />);
    const badge = container.querySelector(".joi-badge");
    expect(badge?.textContent).toBe("Overdue2026-10-10");
    // 8.6.9: the tooltip goes through the shared localized `common.deadlineDate`
    // key; the stored date value inside the badge stays verbatim.
    expect(badge?.getAttribute("title")).toBe("deadline 2026-10-10");

    const timestamp = screen.getByText("2026-10-10");
    expect(timestamp.classList.contains("tabular-nums")).toBe(true);
    const ownModifiers = classTokens(timestamp).filter((token) => token.startsWith("joi-badge--"));
    expect(ownModifiers).toHaveLength(0);
    expectNoRawPaletteUtility(timestamp);
  });
});
