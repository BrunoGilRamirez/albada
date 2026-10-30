import { describe, expect, it, vi } from "vitest";
import { fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import GoalForm from "@/components/goals/GoalForm";

function fillBasicFields() {
  fireEvent.change(screen.getByLabelText("Title"), { target: { value: "Synthetic goal" } });
}

describe("GoalForm live Markdown previews", () => {
  it("pairs all three Markdown fields with named preview regions", () => {
    render(<GoalForm onSubmit={vi.fn()} />);

    for (const label of ["Description", "Why it matters", "Success definition"]) {
      expect(screen.getByRole("region", { name: `${label} (Markdown) preview` })).toBeTruthy();
    }
  });

  it("updates the matching preview live and leaves the others alone", () => {
    render(<GoalForm onSubmit={vi.fn()} />);

    fireEvent.change(screen.getByLabelText("Why it matters (Markdown)"), {
      target: { value: "## Synthetic reason" },
    });

    const whyRegion = screen.getByRole("region", { name: "Why it matters (Markdown) preview" });
    expect(
      within(whyRegion).getByRole("heading", { level: 2, name: "Synthetic reason" }),
    ).toBeTruthy();
    expect(within(whyRegion).queryByText(/No content yet/)).toBeNull();
    const descriptionRegion = screen.getByRole("region", {
      name: "Description (Markdown) preview",
    });
    expect(within(descriptionRegion).getByText(/No content yet/)).toBeTruthy();
  });

  it("submits all Markdown fields verbatim", async () => {
    const onSubmit = vi.fn();
    render(<GoalForm onSubmit={onSubmit} />);

    fillBasicFields();
    fireEvent.change(screen.getByLabelText("Description (Markdown)"), {
      target: { value: "# raw description\n*one*" },
    });
    fireEvent.change(screen.getByLabelText("Why it matters (Markdown)"), {
      target: { value: "## raw why" },
    });
    fireEvent.change(screen.getByLabelText("Success definition (Markdown)"), {
      target: { value: "### raw success\n- [ ] done marker" },
    });
    fireEvent.click(screen.getByRole("button", { name: "Save goal" }));

    await waitFor(() => expect(onSubmit).toHaveBeenCalledTimes(1));
    expect(onSubmit.mock.calls[0][0]).toEqual(
      expect.objectContaining({
        description_markdown: "# raw description\n*one*",
        why_it_matters_markdown: "## raw why",
        success_definition_markdown: "### raw success\n- [ ] done marker",
      }),
    );
  });
});
