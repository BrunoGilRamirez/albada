import { describe, expect, it, vi } from "vitest";
import { fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import ProjectForm from "@/components/projects/ProjectForm";

describe("ProjectForm live Markdown previews", () => {
  it("pairs all three Markdown fields with named preview regions", () => {
    render(<ProjectForm onSubmit={vi.fn()} />);

    for (const label of ["Description", "Why it matters", "Definition of done"]) {
      expect(screen.getByRole("region", { name: `${label} (Markdown) preview` })).toBeTruthy();
    }
  });

  it("renders the initial Markdown source in the editor and preview without transformation", () => {
    render(
      <ProjectForm
        onSubmit={vi.fn()}
        initial={{ definition_of_done_markdown: "# raw done list\n- [x] shipped" }}
      />,
    );

    const textarea = screen.getByLabelText("Definition of done (Markdown)");
    expect((textarea as HTMLTextAreaElement).value).toBe("# raw done list\n- [x] shipped");
    const region = screen.getByRole("region", { name: "Definition of done (Markdown) preview" });
    expect(within(region).getByText("shipped")).toBeTruthy();
  });

  it("submits all Markdown fields verbatim", async () => {
    const onSubmit = vi.fn();
    render(<ProjectForm onSubmit={onSubmit} />);

    fireEvent.change(screen.getByLabelText("Title"), { target: { value: "Synthetic project" } });
    fireEvent.change(screen.getByLabelText("Description (Markdown)"), {
      target: { value: "# raw description" },
    });
    fireEvent.change(screen.getByLabelText("Why it matters (Markdown)"), {
      target: { value: "## raw why" },
    });
    fireEvent.change(screen.getByLabelText("Definition of done (Markdown)"), {
      target: { value: "### raw done" },
    });
    fireEvent.click(screen.getByRole("button", { name: "Save project" }));

    await waitFor(() => expect(onSubmit).toHaveBeenCalledTimes(1));
    expect(onSubmit.mock.calls[0][0]).toEqual(
      expect.objectContaining({
        description_markdown: "# raw description",
        why_it_matters_markdown: "## raw why",
        definition_of_done_markdown: "### raw done",
      }),
    );
  });
});
