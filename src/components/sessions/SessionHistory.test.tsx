import { afterAll, beforeEach, describe, expect, it, vi } from "vitest";
import { render, screen } from "@testing-library/react";
import i18n from "i18next";
import { SUPPORTED_LOCALES } from "@/lib/i18n/locale";
import SessionHistory from "@/components/sessions/SessionHistory";

vi.mock("@/lib/api/sessionApi", () => ({
  listSessionsForTask: vi.fn(),
}));

import { listSessionsForTask } from "@/lib/api/sessionApi";

const session = {
  id: "session-1",
  task_id: "task-1",
  block_id: null,
  daily_plan_task_id: null,
  started_at: null,
  ended_at: "2026-10-15T15:00:00.000Z",
  status: "finished",
  outcome_markdown: null,
  next_step_markdown: "<script>authored & preserved</script>",
  created_at: "2026-10-15T14:00:00.000Z",
  updated_at: "2026-10-15T15:00:00.000Z",
};

describe.each([...SUPPORTED_LOCALES])("SessionHistory in %s", (locale) => {
  beforeEach(async () => {
    await i18n.changeLanguage(locale);
    vi.mocked(listSessionsForTask).mockReset();
  });

  it("localizes history labels without transforming authored session text", async () => {
    vi.mocked(listSessionsForTask).mockResolvedValue([session] as never);

    const { container } = render(<SessionHistory taskId="task-1" />);

    expect(await screen.findByText(i18n.t("today.session.history.notStarted"))).toBeTruthy();
    expect(container.textContent).toContain(i18n.t("today.session.history.until"));
    expect(container.textContent).toContain(i18n.t("today.session.history.next"));
    expect(container.textContent).toContain("<script>authored & preserved</script>");
  });

  it("localizes the empty state", async () => {
    vi.mocked(listSessionsForTask).mockResolvedValue([]);

    render(<SessionHistory taskId="task-1" />);

    expect(await screen.findByText(i18n.t("today.session.history.empty"))).toBeTruthy();
  });
});

afterAll(async () => {
  await i18n.changeLanguage("en-US");
});
