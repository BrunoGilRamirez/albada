import { describe, expect, it, vi, beforeEach } from "vitest";
import { render, screen } from "@testing-library/react";
import { MemoryRouter } from "react-router";
import type { PrerequisiteTask } from "@/lib/types";

vi.mock("@/lib/api/taskApi", () => ({
  whyIsBlocked: vi.fn(),
}));

import { whyIsBlocked } from "@/lib/api/taskApi";
import BlockedReasonPanel from "@/components/tasks/BlockedReasonPanel";

/**
 * Phase 8.5.6 semantic UI contract: a blocked panel renders through the
 * semantic `joi-alert--warning` modifier and never through a raw Tailwind
 * palette utility. `whyIsBlocked` is mocked with synthetic prerequisites only;
 * no app database or private data is touched.
 */

const RAW_PALETTE_UTILITY =
  /(?:^|:)(?:bg|text|border|ring|from|to|via|outline|accent|fill|stroke|shadow)-(?:slate|zinc|neutral|stone|red|orange|amber|yellow|lime|green|emerald|teal|cyan|sky|blue|indigo|violet|purple|fuchsia|pink|rose)-[0-9]{2,3}$/;

function expectNoRawPaletteUtility(element: Element) {
  const offenders = Array.from(element.classList).filter((token) =>
    RAW_PALETTE_UTILITY.test(token),
  );
  expect(offenders).toEqual([]);
}

function blocker(id: string, title: string): PrerequisiteTask {
  return { dependency_id: `dep-${id}`, task_id: id, title, status: "open" };
}

async function renderPanel() {
  const result = render(
    <MemoryRouter>
      <BlockedReasonPanel taskId="task-1" />
    </MemoryRouter>,
  );
  // Let the mocked promise settle so the panel leaves its loading state.
  await screen.findByText("Why is this blocked?");
  return result;
}

describe("BlockedReasonPanel", () => {
  beforeEach(() => {
    vi.mocked(whyIsBlocked).mockReset();
  });

  it("renders nothing while the blocked state is loading", () => {
    vi.mocked(whyIsBlocked).mockReturnValue(new Promise(() => undefined));
    const { container } = render(
      <MemoryRouter>
        <BlockedReasonPanel taskId="task-1" />
      </MemoryRouter>,
    );
    expect(container.firstChild).toBeNull();
  });

  it("explains an unblocked task without an alert", async () => {
    vi.mocked(whyIsBlocked).mockResolvedValue([]);
    const { container } = render(
      <MemoryRouter>
        <BlockedReasonPanel taskId="task-1" />
      </MemoryRouter>,
    );

    const message = await screen.findByText("This task is not blocked by any open prerequisite.");
    expect(message).toBeTruthy();
    expect(container.querySelector(".joi-alert")).toBeNull();
    expectNoRawPaletteUtility(container.firstElementChild as Element);
  });

  it("renders the blocked explanation in the warning alert modifier", async () => {
    vi.mocked(whyIsBlocked).mockResolvedValue([blocker("task-2", "Finish the schema draft")]);
    const { container } = await renderPanel();

    const alert = container.querySelector(".joi-alert");
    expect(alert).not.toBeNull();
    expect(alert?.classList.contains("joi-alert--warning")).toBe(true);
    expectNoRawPaletteUtility(alert as Element);
  });

  it("lists every open prerequisite as a link", async () => {
    vi.mocked(whyIsBlocked).mockResolvedValue([
      blocker("task-2", "Finish the schema draft"),
      blocker("task-3", "Review the migration plan"),
    ]);
    await renderPanel();

    const links = screen.getAllByRole("link");
    expect(links.map((link) => link.textContent)).toEqual([
      "Finish the schema draft",
      "Review the migration plan",
    ]);
    expect(links[0].getAttribute("href")).toBe("/tasks/task-2");
    expect(links[1].getAttribute("href")).toBe("/tasks/task-3");
  });

  it("requests the blocked state for the given task id", async () => {
    vi.mocked(whyIsBlocked).mockResolvedValue([blocker("task-2", "Finish the schema draft")]);
    await renderPanel();

    expect(whyIsBlocked).toHaveBeenCalledWith("task-1");
  });

  it("falls back to the unblocked message when the native call fails", async () => {
    vi.mocked(whyIsBlocked).mockRejectedValue(new Error("command failed"));
    const { container } = render(
      <MemoryRouter>
        <BlockedReasonPanel taskId="task-1" />
      </MemoryRouter>,
    );

    const message = await screen.findByText("This task is not blocked by any open prerequisite.");
    expect(message).toBeTruthy();
    expect(container.querySelector(".joi-alert")).toBeNull();
  });
});
