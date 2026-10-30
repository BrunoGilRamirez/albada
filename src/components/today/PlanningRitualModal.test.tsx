import { afterEach, describe, expect, it, vi, beforeEach } from "vitest";
import i18n from "i18next";
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { MemoryRouter } from "react-router";
import PlanningRitualModal from "@/components/today/PlanningRitualModal";
import { SUPPORTED_LOCALES, type AppLocale } from "@/lib/i18n/locale";
import {
  getCapacitySuggestion,
  selectTaskForDay,
  setDailyPlanPlanningBlock,
  updateDailyPlanIntention,
  updateDailyPlanStatus,
} from "@/lib/api/dailyPlanApi";
import { createBlock } from "@/lib/api/blockApi";
import { listTasks, whyIsBlocked } from "@/lib/api/taskApi";

vi.mock("@/lib/api/dailyPlanApi", () => ({
  getCapacitySuggestion: vi.fn(),
  selectTaskForDay: vi.fn(),
  setDailyPlanPlanningBlock: vi.fn(),
  updateDailyPlanIntention: vi.fn(),
  updateDailyPlanStatus: vi.fn(),
}));

vi.mock("@/lib/api/blockApi", () => ({
  createBlock: vi.fn(),
}));

vi.mock("@/lib/api/taskApi", () => ({
  listTasks: vi.fn(),
  whyIsBlocked: vi.fn(),
}));

const plan = {
  id: "plan-1",
  workspace_id: "w1",
  date: "2026-09-08",
  status: "inbox",
  intention_markdown: null,
  capacity_note_markdown: null,
  review_markdown: null,
  planning_block_id: null,
  created_at: "2026-09-08T08:00:00Z",
  updated_at: "2026-09-08T08:00:00Z",
};

const task = {
  id: "t1",
  workspace_id: "w1",
  project_id: null,
  goal_id: null,
  parent_task_id: null,
  title: "Write the changelog",
  status: "in_progress",
  priority: "high",
  effort_size: "medium",
  target_date: null,
};

const planningBlock = {
  id: "block-1",
  workspace_id: "w1",
  daily_plan_id: "plan-1",
  date: "2026-09-08",
  block_type: "planning",
  status: "planned",
  label: "Daily planning",
};

function renderModal(onCompleted = vi.fn(), onClose = vi.fn()) {
  render(
    <MemoryRouter>
      <PlanningRitualModal
        workspaceId="w1"
        date="2026-09-08"
        plan={plan as never}
        existingPlanTasks={[]}
        onCompleted={onCompleted}
        onClose={onClose}
      />
    </MemoryRouter>,
  );
  return { onCompleted, onClose };
}

describe("PlanningRitualModal", () => {
  beforeEach(() => {
    vi.mocked(getCapacitySuggestion).mockReset();
    vi.mocked(selectTaskForDay).mockReset();
    vi.mocked(setDailyPlanPlanningBlock).mockReset();
    vi.mocked(updateDailyPlanIntention).mockReset();
    vi.mocked(updateDailyPlanStatus).mockReset();
    vi.mocked(createBlock).mockReset();
    vi.mocked(listTasks).mockReset();
    vi.mocked(whyIsBlocked).mockReset();

    vi.mocked(getCapacitySuggestion).mockResolvedValue(null);
    vi.mocked(listTasks).mockResolvedValue([task] as never);
    vi.mocked(whyIsBlocked).mockResolvedValue([]);
    vi.mocked(updateDailyPlanIntention).mockResolvedValue(plan as never);
    vi.mocked(createBlock).mockResolvedValue(planningBlock as never);
    vi.mocked(setDailyPlanPlanningBlock).mockResolvedValue(plan as never);
    vi.mocked(updateDailyPlanStatus).mockResolvedValue({ ...plan, status: "planned" } as never);
  });

  it("plans the day: selects the task, saves the intention, and closes", async () => {
    const { onCompleted, onClose } = renderModal();

    // Step 1: review unblocked tasks and select one.
    const checkbox = await screen.findByRole("checkbox", { name: /Write the changelog/ });
    fireEvent.click(checkbox);

    fireEvent.click(screen.getByRole("button", { name: "Continue" }));
    // Step 2: blocked review shows nothing blocked for this synthetic workspace.
    expect(screen.getByText("Nothing is blocked right now.")).toBeTruthy();

    fireEvent.click(screen.getByRole("button", { name: "Continue" }));
    // Step 3: confirm selection.
    expect(screen.getByText("Manual")).toBeTruthy();

    fireEvent.click(screen.getByRole("button", { name: "Continue" }));
    // Step 4: intention note.
    fireEvent.change(screen.getByLabelText("Intention note"), {
      target: { value: "Focus on the changelog first." },
    });

    fireEvent.click(screen.getByRole("button", { name: "Finish planning" }));

    await waitFor(() => expect(updateDailyPlanStatus).toHaveBeenCalledWith("plan-1", "planned"));
    expect(updateDailyPlanIntention).toHaveBeenCalledWith("plan-1", {
      intention_markdown: "Focus on the changelog first.",
      capacity_note_markdown: null,
    });
    expect(selectTaskForDay).toHaveBeenCalledWith({
      daily_plan_id: "plan-1",
      task_id: "t1",
      reason: "manual",
    });
    expect(createBlock).toHaveBeenCalledWith(
      expect.objectContaining({
        workspace_id: "w1",
        daily_plan_id: "plan-1",
        date: "2026-09-08",
        block_type: "planning",
        label: "Daily planning",
      }),
    );
    expect(setDailyPlanPlanningBlock).toHaveBeenCalledWith("plan-1", "block-1");
    expect(onCompleted).toHaveBeenCalledTimes(1);
    expect(onClose).toHaveBeenCalledTimes(1);
  });

  it("closes without writing when dismissed early", async () => {
    const { onCompleted, onClose } = renderModal();

    await screen.findByRole("checkbox", { name: /Write the changelog/ });
    fireEvent.click(screen.getByRole("button", { name: "Close" }));

    expect(onClose).toHaveBeenCalledTimes(1);
    expect(onCompleted).not.toHaveBeenCalled();
    expect(selectTaskForDay).not.toHaveBeenCalled();
    expect(updateDailyPlanStatus).not.toHaveBeenCalled();
  });
});

// 8.6.8 acceptance correction: the two application-generated defaults the
// ritual writes into stored content (the capacity-note prefill and the new
// planning block's label) are translated at creation in the active locale.
// The interpolated clock times and the stored `energy_expectation` value
// ("high") stay verbatim in every locale. All fixtures are synthetic.
const capacity = {
  start_time: "09:00",
  end_time: "12:00",
  preferred_block_length_minutes: 1440,
  energy_expectation: "high",
  profile_id: null,
};

interface GeneratedCopy {
  capacityNoteLabel: string;
  continueButton: string;
  finishButton: string;
  prefill: string;
  planningLabel: string;
}

const GENERATED_COPY: Record<AppLocale, GeneratedCopy> = {
  "en-US": {
    capacityNoteLabel: "Capacity note",
    continueButton: "Continue",
    finishButton: "Finish planning",
    prefill: "Suggested focus window: 09:00 – 12:00 (high).",
    planningLabel: "Daily planning",
  },
  "es-ES": {
    capacityNoteLabel: "Nota de capacidad",
    continueButton: "Continuar",
    finishButton: "Terminar la planificación",
    prefill: "Franja de concentración sugerida: 09:00 – 12:00 (high).",
    planningLabel: "Planificación diaria",
  },
  "de-DE": {
    capacityNoteLabel: "Kapazitätsnotiz",
    continueButton: "Weiter",
    finishButton: "Planung abschließen",
    prefill: "Vorgeschlagenes Fokuszeitfenster: 09:00 – 12:00 (high).",
    planningLabel: "Tägliche Planung",
  },
  "fr-FR": {
    capacityNoteLabel: "Note de capacité",
    continueButton: "Continuer",
    finishButton: "Terminer la planification",
    prefill: "Créneau de concentration suggéré : 09:00 – 12:00 (high).",
    planningLabel: "Planification quotidienne",
  },
};

describe.each([...SUPPORTED_LOCALES])("PlanningRitualModal generated defaults in %s", (locale) => {
  beforeEach(() => {
    vi.mocked(getCapacitySuggestion).mockReset();
    vi.mocked(selectTaskForDay).mockReset();
    vi.mocked(setDailyPlanPlanningBlock).mockReset();
    vi.mocked(updateDailyPlanIntention).mockReset();
    vi.mocked(updateDailyPlanStatus).mockReset();
    vi.mocked(createBlock).mockReset();
    vi.mocked(listTasks).mockReset();
    vi.mocked(whyIsBlocked).mockReset();

    vi.mocked(getCapacitySuggestion).mockResolvedValue(capacity as never);
    vi.mocked(listTasks).mockResolvedValue([task] as never);
    vi.mocked(whyIsBlocked).mockResolvedValue([]);
    vi.mocked(updateDailyPlanIntention).mockResolvedValue(plan as never);
    vi.mocked(createBlock).mockResolvedValue(planningBlock as never);
    vi.mocked(setDailyPlanPlanningBlock).mockResolvedValue(plan as never);
    vi.mocked(updateDailyPlanStatus).mockResolvedValue({ ...plan, status: "planned" } as never);
  });

  afterEach(async () => {
    cleanup();
    // Keep the shared English test default for the rest of the suite.
    await i18n.changeLanguage("en-US");
  });

  it("prefills the capacity note and writes the block label in the active locale", async () => {
    await i18n.changeLanguage(locale);
    const e = GENERATED_COPY[locale];
    renderModal();

    await screen.findByRole("checkbox", { name: /Write the changelog/ });
    for (let step = 0; step < 2; step += 1) {
      fireEvent.click(screen.getByRole("button", { name: e.continueButton }));
    }
    const formattedMinutes = new Intl.NumberFormat(locale).format(1440).replace(/\s/gu, " ");
    expect(
      await screen.findByText((content) => content.replace(/\s/gu, " ").includes(formattedMinutes)),
    ).toBeTruthy();
    fireEvent.click(screen.getByRole("button", { name: e.continueButton }));

    const capacityBox = screen.getByLabelText<HTMLTextAreaElement>(e.capacityNoteLabel);
    await waitFor(() => expect(capacityBox.value).toBe(e.prefill));

    fireEvent.click(screen.getByRole("button", { name: e.finishButton }));

    await waitFor(() =>
      expect(createBlock).toHaveBeenCalledWith(expect.objectContaining({ label: e.planningLabel })),
    );
    expect(updateDailyPlanIntention).toHaveBeenCalledWith("plan-1", {
      intention_markdown: null,
      capacity_note_markdown: e.prefill,
    });
  });
});
