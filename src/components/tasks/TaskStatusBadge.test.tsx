import { describe, expect, it } from "vitest";
import { render, screen } from "@testing-library/react";
import TaskStatusBadge from "@/components/tasks/TaskStatusBadge";

describe("TaskStatusBadge", () => {
  it("renders the correct label for in_progress", () => {
    render(<TaskStatusBadge status="in_progress" />);
    expect(screen.getByText("In progress")).toBeTruthy();
  });

  it("renders the correct label for blocked", () => {
    render(<TaskStatusBadge status="blocked" />);
    expect(screen.getByText("Blocked")).toBeTruthy();
  });
});
