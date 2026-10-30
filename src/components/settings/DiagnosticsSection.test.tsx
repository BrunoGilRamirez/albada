import { describe, expect, it, vi } from "vitest";
import { render, screen } from "@testing-library/react";
import DiagnosticsSection from "@/components/settings/DiagnosticsSection";
import { getEnvironmentDiagnostics } from "@/lib/api/diagnosticsApi";
import { exportTemplateCapability } from "@/lib/api/exportApi";
import type { EnvironmentDiagnostics, TemplateCapability } from "@/lib/types";

vi.mock("@/lib/api/exportApi", () => ({
  exportTemplateCapability: vi.fn(),
}));

vi.mock("@/lib/api/diagnosticsApi", () => ({
  getEnvironmentDiagnostics: vi.fn(),
}));

const capability: TemplateCapability = {
  name: "foundation-journal",
  version: "1.0.0",
  valid: true,
  compiler_version: "typst 0.15.1 (embedded)",
  unavailable_resources: [],
};

const diagnostics: EnvironmentDiagnostics = {
  sqlite_version: "1.2.3-test",
  tauri_version: "4.5.6-test",
  os_platform: "test-os",
  os_version: "7.8.9-test",
  typst_compiler_version: "typst 0.15.1 (embedded)",
  template_name: "foundation-journal",
  template_version: "1.0.0",
  template_valid: true,
  bundled_font_count: 3,
};

describe("DiagnosticsSection", () => {
  it("renders compiler, capability, template, font, and environment versions", async () => {
    vi.mocked(exportTemplateCapability).mockResolvedValue(capability);
    vi.mocked(getEnvironmentDiagnostics).mockResolvedValue(diagnostics);

    render(<DiagnosticsSection />);

    expect(await screen.findByText("Diagnostics")).toBeTruthy();
    expect(await screen.findByText("typst 0.15.1 (embedded)", { exact: false })).toBeTruthy();
    expect(screen.getByText("Available", { exact: false })).toBeTruthy();
    expect(screen.getByText("1.2.3-test", { exact: false })).toBeTruthy();
    expect(screen.getByText("4.5.6-test", { exact: false })).toBeTruthy();
    expect(screen.getByText("test-os 7.8.9-test", { exact: false })).toBeTruthy();
  });

  it("degrades to a muted note without an alert when the commands fail", async () => {
    vi.mocked(exportTemplateCapability).mockRejectedValue(new Error("no runtime"));
    vi.mocked(getEnvironmentDiagnostics).mockRejectedValue(new Error("no runtime"));

    render(<DiagnosticsSection />);

    expect(await screen.findByText("Diagnostics are unavailable in this session.")).toBeTruthy();
    expect(screen.queryByRole("alert")).toBeNull();
  });
});
