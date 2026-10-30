import { describe, expect, it } from "vitest";
import { fireEvent, render, screen } from "@testing-library/react";
import ImportReportView from "@/components/import/ImportReportView";
import type { ImportReportDto } from "@/lib/types";

const report: ImportReportDto = {
  batch_id: "batch-1",
  source_root: "C:\\legacy\\proyecto-vida",
  status: "completed",
  started_at: "2026-09-06T10:00:00Z",
  completed_at: "2026-09-06T10:00:05Z",
  counts: [
    { entity_type: "task", count: 12 },
    { entity_type: "journal_entry", count: 5 },
  ],
  warnings: [
    {
      severity: "warning",
      source_path: "records/2026/q1/planning/goals.md",
      line: 4,
      message: "Filename date does not match the title date.",
    },
    {
      severity: "error",
      source_path: "notes/raw/todo.md",
      line: null,
      message: "Broken relative link.",
    },
  ],
  review_items: [
    {
      id: "review-1",
      source_path: "records/2026/q1/planning/tasks.md",
      proposed_entity_type: "task",
      title: "Review quarterly goals",
      snippet: "- [ ] Review quarterly goals",
      confidence: "review",
      source_hash: "a".repeat(64),
    },
  ],
  preview_fingerprint: "b".repeat(64),
};

describe("ImportReportView", () => {
  it("renders the summary counts", () => {
    render(<ImportReportView report={report} />);

    // Entity types now come from the shared `import.entityTypes` catalog
    // (sentence case) instead of the mechanical title-cased identifier.
    expect(screen.getByText("Journal entry")).toBeTruthy();
    expect(screen.getByText("12")).toBeTruthy();
    expect(screen.getByText("5")).toBeTruthy();
    expect(screen.getByText("C:\\legacy\\proyecto-vida")).toBeTruthy();
  });

  it("keeps warnings collapsed until expanded", () => {
    render(<ImportReportView report={report} />);

    expect(screen.queryByText("Filename date does not match the title date.")).toBeNull();

    fireEvent.click(screen.getByRole("button", { name: "Show warnings (2)" }));

    expect(screen.getByText("Filename date does not match the title date.")).toBeTruthy();
    expect(screen.getByText("Broken relative link.")).toBeTruthy();
    expect(screen.getByText("Warning")).toBeTruthy();
    expect(screen.getByText("Error")).toBeTruthy();
  });

  it("shows review items with placeholder actions", () => {
    render(<ImportReportView report={report} />);

    expect(screen.getByText("Review quarterly goals")).toBeTruthy();
    expect(screen.getByText("Needs review")).toBeTruthy();

    fireEvent.click(screen.getByRole("button", { name: "Accept" }));

    expect(screen.getByText(/Review decisions are not saved yet/)).toBeTruthy();
  });
});
