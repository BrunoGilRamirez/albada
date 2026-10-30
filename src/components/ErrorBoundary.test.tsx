import { beforeEach, describe, expect, it, vi } from "vitest";
import { render, screen } from "@testing-library/react";

vi.mock("@/lib/tauri", () => ({
  reportUnknownError: vi.fn(),
}));

import ErrorBoundary from "@/components/ErrorBoundary";
import { reportUnknownError } from "@/lib/tauri";

function Boom(): never {
  throw new Error("render exploded");
}

describe("ErrorBoundary", () => {
  beforeEach(() => {
    vi.mocked(reportUnknownError).mockReset();
  });

  it("renders children normally and reports nothing when the tree is healthy", () => {
    render(
      <ErrorBoundary>
        <p>all fine</p>
      </ErrorBoundary>,
    );

    expect(screen.getByText("all fine")).toBeInTheDocument();
    expect(reportUnknownError).not.toHaveBeenCalled();
  });

  it("shows the fallback and reports render errors through the typed boundary", () => {
    // React logs caught errors via console.error in dev; silence the noise.
    const consoleSpy = vi.spyOn(console, "error").mockImplementation(() => {});

    try {
      render(
        <ErrorBoundary>
          <Boom />
        </ErrorBoundary>,
      );

      expect(screen.getByText("We could not display this view")).toBeInTheDocument();
      expect(reportUnknownError).toHaveBeenCalledTimes(1);

      const [source, error, componentStack] = vi.mocked(reportUnknownError).mock.calls[0];
      expect(source).toBe("error_boundary");
      expect(error).toBeInstanceOf(Error);
      expect((error as Error).message).toBe("render exploded");
      expect(componentStack).toContain("Boom");
    } finally {
      consoleSpy.mockRestore();
    }
  });
});
