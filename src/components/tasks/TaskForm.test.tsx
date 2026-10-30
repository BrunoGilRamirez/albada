import { describe, expect, it, vi } from "vitest";
import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import TaskForm from "@/components/tasks/TaskForm";

describe("TaskForm design-system controls", () => {
  it("keeps the disabled submit button semantic and natively disabled", () => {
    render(<TaskForm disabled onSubmit={vi.fn()} />);

    const submit = screen.getByRole("button", { name: "Save task" });
    expect(submit.getAttribute("disabled")).not.toBeNull();
    expect(submit.classList.contains("joi-btn")).toBe(true);
    expect(submit.classList.contains("joi-btn--primary")).toBe(true);
  });
});

describe("TaskForm live Markdown previews", () => {
  it("pairs both Markdown fields with named preview regions", () => {
    render(<TaskForm onSubmit={vi.fn()} />);

    expect(screen.getByRole("region", { name: "Description (Markdown) preview" })).toBeTruthy();
    expect(screen.getByRole("region", { name: "Notes (Markdown) preview" })).toBeTruthy();
  });

  it("updates the matching preview live and keeps the other field independent", () => {
    render(<TaskForm onSubmit={vi.fn()} />);

    fireEvent.change(screen.getByLabelText("Notes (Markdown)"), {
      target: { value: "## Synthetic note" },
    });

    const notesRegion = screen.getByRole("region", { name: "Notes (Markdown) preview" });
    const descriptionRegion = screen.getByRole("region", {
      name: "Description (Markdown) preview",
    });
    expect(notesRegion.textContent).toContain("Synthetic note");
    expect(notesRegion.querySelector("h2")).not.toBeNull();
    expect(descriptionRegion.textContent).toMatch(/No content yet/);
  });

  it("submits the Markdown source verbatim", async () => {
    const onSubmit = vi.fn();
    render(<TaskForm onSubmit={onSubmit} />);

    fireEvent.change(screen.getByLabelText("Title"), {
      target: { value: "Write the changelog" },
    });
    fireEvent.change(screen.getByLabelText("Description (Markdown)"), {
      target: { value: "# raw heading stays raw" },
    });
    fireEvent.change(screen.getByLabelText("Notes (Markdown)"), {
      target: { value: "1. raw list\n\\# raw escape" },
    });
    fireEvent.click(screen.getByRole("button", { name: "Save task" }));

    await waitFor(() => expect(onSubmit).toHaveBeenCalledTimes(1));
    expect(onSubmit.mock.calls[0][0]).toEqual(
      expect.objectContaining({
        description_markdown: "# raw heading stays raw",
        notes_markdown: "1. raw list\n\\# raw escape",
      }),
    );
  });
});

describe("TaskForm matrix selection", () => {
  it("submits the effort and priority chosen through the matrix", async () => {
    const onSubmit = vi.fn();
    render(<TaskForm onSubmit={onSubmit} />);

    fireEvent.change(screen.getByLabelText("Title"), {
      target: { value: "Divide the report" },
    });
    // The "Address today" cell is the trivial/critical intersection.
    fireEvent.click(screen.getByRole("button", { name: "Address today" }));
    fireEvent.click(screen.getByRole("button", { name: "Save task" }));

    await waitFor(() => expect(onSubmit).toHaveBeenCalledTimes(1));
    expect(onSubmit.mock.calls[0][0]).toEqual(
      expect.objectContaining({
        title: "Divide the report",
        effort_size: "trivial",
        priority: "critical",
      }),
    );
  });

  it("does not submit when the title is blank", () => {
    const onSubmit = vi.fn();
    render(<TaskForm onSubmit={onSubmit} />);

    fireEvent.click(screen.getByRole("button", { name: "Save task" }));

    expect(onSubmit).not.toHaveBeenCalled();
  });
});
