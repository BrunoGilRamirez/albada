import i18n from "i18next";
import { afterEach, describe, expect, it } from "vitest";
import { render, screen } from "@testing-library/react";
import ProgressBar from "@/components/progress/ProgressBar";

afterEach(async () => {
  await i18n.changeLanguage("en-US");
});

describe("ProgressBar", () => {
  it("renders a percentage when one is present", () => {
    render(<ProgressBar percentage={50} />);
    expect(screen.getByText("50.0%")).toBeTruthy();
  });

  it("renders 'no tasks' label when percentage is null", () => {
    render(<ProgressBar percentage={null} label="no_tasks" />);
    expect(screen.getByText("No tasks")).toBeTruthy();
  });

  it("renders 'not started' label when percentage is undefined", () => {
    render(<ProgressBar label="not_started" />);
    expect(screen.getByText("Not started")).toBeTruthy();
  });

  it("renders closed scope labels", () => {
    render(<ProgressBar percentage={null} label="archived" />);
    expect(screen.getByText("Archived")).toBeTruthy();
    render(<ProgressBar percentage={null} label="cancelled" />);
    expect(screen.getByText("Cancelled")).toBeTruthy();
  });

  it("clamps the percentage to 100", () => {
    render(<ProgressBar percentage={150} />);
    const bar = screen.getByRole("progressbar");
    expect(bar.getAttribute("aria-valuenow")).toBe("100");
  });

  it("formats the derived percentage and state label in the active locale", async () => {
    await i18n.changeLanguage("es-ES");
    const esPercent = render(<ProgressBar percentage={50} />);
    expect(esPercent.getByText("50,0%")).toBeTruthy();
    esPercent.unmount();

    const esLabel = render(<ProgressBar percentage={null} label="no_tasks" />);
    expect(esLabel.getByText("Sin tareas")).toBeTruthy();
    esLabel.unmount();

    await i18n.changeLanguage("fr-FR");
    const frPercent = render(<ProgressBar percentage={50} />);
    expect(frPercent.getByText("50,0%")).toBeTruthy();
    frPercent.unmount();
    await i18n.changeLanguage("en-US");
  });
});
