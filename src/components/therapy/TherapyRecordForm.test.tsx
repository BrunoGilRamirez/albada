import { describe, expect, it, vi } from "vitest";
import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import TherapyRecordForm from "@/components/therapy/TherapyRecordForm";

describe("TherapyRecordForm live Markdown preview", () => {
  it("pairs the content field with a named preview region inside the form", () => {
    const { container } = render(<TherapyRecordForm onSubmit={vi.fn()} />);

    const region = screen.getByRole("region", { name: "Content (Markdown) preview" });
    expect(container.contains(region)).toBe(true);
    expect(screen.getByLabelText("Content (Markdown)").tagName.toLowerCase()).toBe("textarea");
  });

  it("updates the preview live while typing", () => {
    render(<TherapyRecordForm onSubmit={vi.fn()} />);

    fireEvent.change(screen.getByLabelText("Content (Markdown)"), {
      target: { value: "## Synthetic exercise\n\n*breathe* in" },
    });

    expect(screen.getByRole("heading", { level: 2, name: "Synthetic exercise" })).toBeTruthy();
    expect(
      screen.getByRole("region", { name: "Content (Markdown) preview" }).textContent,
    ).toContain("breathe");
  });

  it("submits the Markdown source verbatim (heading normalization removed in 8.6.1)", async () => {
    const onSubmit = vi.fn();
    render(<TherapyRecordForm onSubmit={onSubmit} />);

    fireEvent.change(screen.getByLabelText("Title"), {
      target: { value: "Grounding exercise" },
    });
    fireEvent.change(screen.getByLabelText("Content (Markdown)"), {
      target: { value: "# Raw heading stays raw\n\n- [ ] synthetic step" },
    });
    fireEvent.click(screen.getByRole("button", { name: "Save record" }));

    await waitFor(() => expect(onSubmit).toHaveBeenCalledTimes(1));
    expect(onSubmit.mock.calls[0][0]).toEqual(
      expect.objectContaining({
        content_markdown: "# Raw heading stays raw\n\n- [ ] synthetic step",
      }),
    );
  });

  it("renders existing content in both editor and preview without transformation", () => {
    render(
      <TherapyRecordForm
        isEdit
        onSubmit={vi.fn()}
        initial={{ content_markdown: "# Stored source\n\n1. verbatim item" }}
      />,
    );

    expect((screen.getByLabelText("Content (Markdown)") as HTMLTextAreaElement).value).toBe(
      "# Stored source\n\n1. verbatim item",
    );
    const region = screen.getByRole("region", { name: "Content (Markdown) preview" });
    expect(region.textContent).toContain("verbatim item");
  });
});
