import { describe, expect, it } from "vitest";
import { render, screen } from "@testing-library/react";
import MarkdownPreview from "@/components/markdown/MarkdownPreview";

describe("MarkdownPreview", () => {
  it("renders GFM tables", () => {
    render(<MarkdownPreview content={"| A | B |\n|---|---|\n| 1 | 2 |"} />);
    expect(screen.getByRole("table")).toBeTruthy();
    expect(screen.getByText("1")).toBeTruthy();
  });

  it("sanitizes raw HTML instead of injecting it", () => {
    render(
      <MarkdownPreview
        content={'<img src="x" onerror="window.__pwned = true"><script>alert(1)</script>Hello'}
      />,
    );
    // Text content is preserved.
    expect(screen.getByText(/Hello/)).toBeTruthy();
    // Raw HTML must not be rendered as elements.
    expect(document.querySelector("img")).toBeNull();
    expect(document.querySelector("script")).toBeNull();
  });

  it("shows a placeholder for empty content", () => {
    render(<MarkdownPreview content="" />);
    expect(screen.getByText(/No content yet/)).toBeTruthy();
  });

  it("shows a caller-provided localized placeholder, including for whitespace-only source", () => {
    render(<MarkdownPreview content="   " emptyText="Noch kein Inhalt." />);
    expect(screen.getByText("Noch kein Inhalt.")).toBeTruthy();
    expect(screen.queryByText(/No content yet/)).toBeNull();
  });
});
