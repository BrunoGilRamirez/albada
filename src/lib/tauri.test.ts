import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

const { invokeMock, isTauriMock } = vi.hoisted(() => ({
  invokeMock: vi.fn(),
  isTauriMock: vi.fn(),
}));

vi.mock("@tauri-apps/api/core", () => ({
  invoke: invokeMock,
  isTauri: isTauriMock,
}));

import {
  dedupeInFlight,
  invoke as boundaryInvoke,
  markStartupPhase,
  reportFrontendError,
  reportFrontendStartup,
  reportUnknownError,
} from "@/lib/tauri";

describe("dedupeInFlight", () => {
  it("shares one in-flight promise between concurrent callers of the same key", async () => {
    let resolveFirst: ((value: number) => void) | undefined;
    const factory = vi.fn(
      () =>
        new Promise<number>((resolve) => {
          resolveFirst = resolve;
        }),
    );

    const first = dedupeInFlight("k1", factory);
    const second = dedupeInFlight("k1", factory);

    expect(factory).toHaveBeenCalledTimes(1);
    resolveFirst?.(42);

    await expect(first).resolves.toBe(42);
    await expect(second).resolves.toBe(42);
  });

  it("re-runs the factory for sequential calls after the first settles", async () => {
    const factory = vi.fn().mockResolvedValue("ok");

    await dedupeInFlight("k2", factory);
    await dedupeInFlight("k2", factory);

    expect(factory).toHaveBeenCalledTimes(2);
  });

  it("keeps different keys independent", async () => {
    const factoryA = vi.fn().mockResolvedValue("a");
    const factoryB = vi.fn().mockResolvedValue("b");

    const [a, b] = await Promise.all([
      dedupeInFlight("k3:a", factoryA),
      dedupeInFlight("k3:b", factoryB),
    ]);

    expect(a).toBe("a");
    expect(b).toBe("b");
    expect(factoryA).toHaveBeenCalledTimes(1);
    expect(factoryB).toHaveBeenCalledTimes(1);
  });

  it("shares the rejection concurrently and clears the key so a later call retries", async () => {
    const failure = new Error("command failed");
    const factory = vi.fn().mockRejectedValueOnce(failure).mockResolvedValueOnce("recovered");

    const first = dedupeInFlight("k4", factory);
    const second = dedupeInFlight("k4", factory);

    await expect(first).rejects.toBe(failure);
    await expect(second).rejects.toBe(failure);
    expect(factory).toHaveBeenCalledTimes(1);

    await expect(dedupeInFlight("k4", factory)).resolves.toBe("recovered");
    expect(factory).toHaveBeenCalledTimes(2);
  });

  it("does not poison the key when the factory throws synchronously", async () => {
    const boom = () => {
      throw new Error("sync boom");
    };

    await expect(dedupeInFlight("k5", boom)).rejects.toThrow("sync boom");
    await expect(dedupeInFlight("k5", async () => "fine")).resolves.toBe("fine");
  });
});

describe("reportFrontendError", () => {
  beforeEach(() => {
    invokeMock.mockReset();
    isTauriMock.mockReset();
    isTauriMock.mockReturnValue(true);
    invokeMock.mockResolvedValue(undefined);
  });

  afterEach(() => {
    vi.useRealTimers();
    vi.restoreAllMocks();
  });

  it("is a no-op outside Tauri (tests, plain browser dev)", () => {
    isTauriMock.mockReturnValue(false);

    reportFrontendError({ source: "window_error", message: "no native here" });

    expect(invokeMock).not.toHaveBeenCalled();
  });

  it("invokes the native report command with the typed payload", () => {
    reportFrontendError({
      source: "error_boundary",
      message: "boundary A",
      name: "TypeError",
      stack: "at Thing",
    });

    expect(invokeMock).toHaveBeenCalledTimes(1);
    expect(invokeMock).toHaveBeenCalledWith("report_frontend_error", {
      report: {
        source: "error_boundary",
        message: "boundary A",
        name: "TypeError",
        stack: "at Thing",
      },
    });
  });

  it("swallows report failures without re-entering the report channel", async () => {
    const warnSpy = vi.spyOn(console, "warn").mockImplementation(() => {});
    invokeMock.mockRejectedValue(new Error("command report_frontend_error not found"));

    expect(() =>
      reportFrontendError({ source: "unhandled_rejection", message: "swallow me" }),
    ).not.toThrow();
    await Promise.resolve();
    await Promise.resolve();

    expect(invokeMock).toHaveBeenCalledTimes(1);
    expect(warnSpy).toHaveBeenCalled();
  });

  it("suppresses identical consecutive reports and releases after the window", () => {
    vi.useFakeTimers();
    vi.setSystemTime(0);

    reportFrontendError({ source: "window_error", message: "loop error" });
    reportFrontendError({ source: "window_error", message: "loop error" });
    expect(invokeMock).toHaveBeenCalledTimes(1);

    vi.advanceTimersByTime(6000);
    reportFrontendError({ source: "window_error", message: "loop error" });
    expect(invokeMock).toHaveBeenCalledTimes(2);

    // Different messages are never suppressed.
    reportFrontendError({ source: "window_error", message: "other error" });
    expect(invokeMock).toHaveBeenCalledTimes(3);
  });
});

describe("reportUnknownError", () => {
  beforeEach(() => {
    invokeMock.mockReset();
    isTauriMock.mockReset();
    isTauriMock.mockReturnValue(true);
    invokeMock.mockResolvedValue(undefined);
  });

  it("extracts name, message, and stack from an Error", () => {
    const error = new Error("probe failure");
    error.stack = "Error: probe failure\n    at somewhere";

    reportUnknownError("window_error", error);

    const [, payload] = invokeMock.mock.calls[0];
    expect(payload).toEqual({
      report: {
        source: "window_error",
        message: "probe failure",
        name: "Error",
        stack: "Error: probe failure\n    at somewhere",
      },
    });
  });

  it("stringifies non-Error rejection reasons and passes the component stack through", () => {
    reportUnknownError("error_boundary", "plain string reason", "\n    in Boom");

    const [, payload] = invokeMock.mock.calls[0];
    expect(payload).toEqual({
      report: {
        source: "error_boundary",
        message: "plain string reason",
        componentStack: "\n    in Boom",
      },
    });
  });

  it("clamps pathological messages and stacks", () => {
    const error = new Error("x".repeat(2000));
    error.stack = "y".repeat(5000);

    reportUnknownError("unhandled_rejection", error);

    const [, payload] = invokeMock.mock.calls[0] as [
      string,
      { report: { message: string; stack: string } },
    ];
    expect(payload.report.message.length).toBeLessThanOrEqual(501);
    expect(payload.report.stack.length).toBeLessThanOrEqual(2001);
  });
});

describe("reportFrontendStartup", () => {
  beforeEach(() => {
    invokeMock.mockReset();
    isTauriMock.mockReset();
    isTauriMock.mockReturnValue(true);
    invokeMock.mockResolvedValue(undefined);
  });

  it("is a no-op outside Tauri", () => {
    isTauriMock.mockReturnValue(false);

    reportFrontendStartup("module-eval", 1234);

    expect(invokeMock).not.toHaveBeenCalled();
  });

  it("sends the fixed contract payload with the command label", () => {
    reportFrontendStartup("first-invoke", 9876, "ensure_default_workspace");

    expect(invokeMock).toHaveBeenCalledWith("report_frontend_startup", {
      report: {
        phase: "first-invoke",
        elapsedMs: 9876,
        command: "ensure_default_workspace",
      },
    });
  });

  it("omits the command key when there is no command", () => {
    reportFrontendStartup("first-commit", 42);

    expect(invokeMock).toHaveBeenCalledWith("report_frontend_startup", {
      report: { phase: "first-commit", elapsedMs: 42 },
    });
  });

  it("skips the sentinel mark when performance timing is unavailable", () => {
    reportFrontendStartup("module-eval", -1);

    expect(invokeMock).not.toHaveBeenCalled();
  });

  it("swallows report failures without re-entering the startup channel", async () => {
    const warnSpy = vi.spyOn(console, "warn").mockImplementation(() => {});
    invokeMock.mockRejectedValue(new Error("command report_frontend_startup not found"));

    expect(() => reportFrontendStartup("module-eval", 1)).not.toThrow();
    await Promise.resolve();

    expect(invokeMock).toHaveBeenCalledTimes(1);
    expect(warnSpy).toHaveBeenCalled();
    warnSpy.mockRestore();
  });

  it("markStartupPhase writes the console line and mirrors it natively", () => {
    const infoSpy = vi.spyOn(console, "info").mockImplementation(() => {});

    markStartupPhase("module-eval");

    expect(infoSpy).toHaveBeenCalledWith(expect.stringContaining("[startup] module-eval @"));
    expect(invokeMock).toHaveBeenCalledWith("report_frontend_startup", {
      report: { phase: "module-eval", elapsedMs: expect.any(Number) },
    });
    infoSpy.mockRestore();
  });
});

describe("first-invoke marking", () => {
  it("marks the first business command once and ignores diagnostic commands", async () => {
    invokeMock.mockReset();
    isTauriMock.mockReset();
    isTauriMock.mockReturnValue(true);
    invokeMock.mockResolvedValue(undefined);
    const infoSpy = vi.spyOn(console, "info").mockImplementation(() => {});

    const startupReports = () =>
      invokeMock.mock.calls.filter((call) => call[0] === "report_frontend_startup");

    try {
      // A diagnostic error report must not consume the business slot.
      reportFrontendError({ source: "window_error", message: "diagnostic first" });
      expect(startupReports()).toHaveLength(0);

      await boundaryInvoke("list_tasks", { workspaceId: "w1" });
      expect(startupReports()).toHaveLength(1);
      expect(startupReports()[0][1]).toEqual({
        report: {
          phase: "first-invoke",
          elapsedMs: expect.any(Number),
          command: "list_tasks",
        },
      });

      // Only the first business invoke is ever marked per session.
      await boundaryInvoke("list_tasks", { workspaceId: "w2" });
      expect(startupReports()).toHaveLength(1);
    } finally {
      infoSpy.mockRestore();
    }
  });
});
