import { describe, expect, it } from "vitest";
import { render, screen } from "@testing-library/react";
import ExportJobStatus from "@/components/export/ExportJobStatus";
import type { ExportJob } from "@/lib/types";

const baseJob: ExportJob = {
  id: "job-1",
  workspace_id: "w1",
  export_type: "pdf",
  scope_type: "journal",
  title: "Journal export",
  template_name: "default",
  status: "completed",
  output_path: null,
  generated_at: "2026-09-06T10:00:00Z",
  error_message: null,
  selection_fingerprint: "fp-1",
  warnings: [],
  created_at: "2026-09-06T10:00:00Z",
  updated_at: "2026-09-06T10:00:05Z",
  artifacts: [],
};

describe("ExportJobStatus", () => {
  it("renders the status with live-region semantics", () => {
    render(<ExportJobStatus job={{ ...baseJob, status: "running" }} />);

    const region = screen.getByRole("status");
    expect(region).toHaveAttribute("aria-live", "polite");
    expect(screen.getByText("Running")).toBeTruthy();
  });

  it("distinguishes Markdown, Typst, and PDF artifacts without raw paths", () => {
    const job: ExportJob = {
      ...baseJob,
      artifacts: [
        {
          id: "a1",
          format: "md",
          path: "C:\\data\\exports\\job-1\\input.md",
          checksum: "aaaabbbbccccdddd",
          size_bytes: 2048,
        },
        {
          id: "a2",
          format: "typ",
          path: "C:\\data\\exports\\job-1\\input.typ",
          checksum: "1111222233334444",
          size_bytes: 4096,
        },
        {
          id: "a3",
          format: "pdf",
          path: "C:\\data\\exports\\job-1\\output.pdf",
          checksum: "9999888877776666",
          size_bytes: 1048576,
        },
      ],
    };

    render(<ExportJobStatus job={job} />);

    expect(screen.getByText("Markdown")).toBeTruthy();
    expect(screen.getByText("Typst")).toBeTruthy();
    expect(screen.getByText("PDF")).toBeTruthy();
    // Sizes rendered in a human-readable form.
    expect(screen.getByText("2.0 KB")).toBeTruthy();
    expect(screen.getByText("4.0 KB")).toBeTruthy();
    expect(screen.getByText("1.0 MB")).toBeTruthy();
    // Compact checksums, never the full value.
    expect(screen.getByText("aaaabbbb…")).toBeTruthy();
    expect(screen.queryByText("aaaabbbbccccdddd")).toBeNull();
    // Raw filesystem paths must never be shown.
    expect(screen.queryByText(/C:\\data\\exports/)).toBeNull();
  });

  it("labels unknown artifact formats as diagnostic", () => {
    const job: ExportJob = {
      ...baseJob,
      artifacts: [
        {
          id: "a1",
          format: "log",
          path: "C:\\data\\exports\\job-1\\trace.log",
          checksum: null,
          size_bytes: 128,
        },
      ],
    };

    render(<ExportJobStatus job={job} />);

    expect(screen.getByText("Diagnostic")).toBeTruthy();
    expect(screen.getByText("128 B")).toBeTruthy();
  });

  it("renders safe warnings with severity labels", () => {
    const job: ExportJob = {
      ...baseJob,
      status: "failed",
      error_message: "The document could not be compiled.",
      warnings: [
        { code: "compile_warning", severity: "warning", message: "A heading level was skipped." },
        {
          code: "compile_error",
          severity: "error",
          message: "The document could not be compiled.",
        },
      ],
    };

    render(<ExportJobStatus job={job} />);

    expect(screen.getByText(/Diagnostics \(2\)/)).toBeTruthy();
    expect(screen.getByText("Warning")).toBeTruthy();
    expect(screen.getByText("Error")).toBeTruthy();
    expect(screen.getByText("A heading level was skipped.")).toBeTruthy();
  });

  it("retains the Markdown source note on failure", () => {
    const job: ExportJob = {
      ...baseJob,
      status: "failed",
      error_message: "The document could not be compiled.",
      artifacts: [
        {
          id: "a1",
          format: "md",
          path: "C:\\data\\exports\\job-1\\input.md",
          checksum: "aaaabbbbccccdddd",
          size_bytes: 2048,
        },
        {
          id: "a2",
          format: "typ",
          path: "C:\\data\\exports\\job-1\\input.typ",
          checksum: "1111222233334444",
          size_bytes: 4096,
        },
      ],
    };

    render(<ExportJobStatus job={job} />);

    expect(screen.getByText(/Markdown source was saved/i)).toBeTruthy();
    expect(screen.getByText("Failed")).toBeTruthy();
  });
});
