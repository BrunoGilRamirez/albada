import { describe, expect, it, vi, beforeEach } from "vitest";
import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { MemoryRouter } from "react-router";
import TherapyPage from "@/app/therapy/page";
import {
  archiveTherapyCollection,
  createTherapyCollection,
  listTherapyCollections,
  updateTherapyCollection,
} from "@/lib/api/therapyApi";
import { ensureDefaultWorkspace } from "@/lib/api/workspaceApi";
import { CommandInvocationError } from "@/lib/tauri";

vi.mock("@/lib/api/therapyApi", () => ({
  archiveTherapyCollection: vi.fn(),
  createTherapyCollection: vi.fn(),
  listTherapyCollections: vi.fn(),
  updateTherapyCollection: vi.fn(),
}));

vi.mock("@/lib/api/workspaceApi", () => ({
  ensureDefaultWorkspace: vi.fn(),
}));

const workspace = { id: "w1", name: "Default" };

const collection = {
  id: "c1",
  workspace_id: "w1",
  name: "Coping toolkit",
  privacy_scope: "therapy_private",
  encryption_status: "at_rest",
  created_at: "2026-10-13T09:00:00Z",
  updated_at: "2026-10-13T09:00:00Z",
  archived_at: null,
};

const renamedCollection = { ...collection, name: "Grounding toolkit" };

function renderPage() {
  return render(
    <MemoryRouter initialEntries={["/therapy"]}>
      <TherapyPage />
    </MemoryRouter>,
  );
}

describe("TherapyPage collection actions", () => {
  beforeEach(() => {
    vi.mocked(ensureDefaultWorkspace).mockReset();
    vi.mocked(listTherapyCollections).mockReset();
    vi.mocked(createTherapyCollection).mockReset();
    vi.mocked(updateTherapyCollection).mockReset();
    vi.mocked(archiveTherapyCollection).mockReset();
    vi.mocked(ensureDefaultWorkspace).mockResolvedValue(workspace as never);
  });

  it("renames a collection and refreshes the list with the saved name", async () => {
    // Only `name` is sent: an omitted encryption_status must not be part of
    // the rename payload (the Rust boundary preserves the stored value).
    vi.mocked(listTherapyCollections)
      .mockResolvedValueOnce([collection] as never)
      .mockResolvedValue([renamedCollection] as never);
    vi.mocked(updateTherapyCollection).mockResolvedValue(renamedCollection as never);

    renderPage();

    fireEvent.click(await screen.findByRole("button", { name: "Edit" }));
    const input = screen.getByDisplayValue("Coping toolkit");
    fireEvent.change(input, { target: { value: "Grounding toolkit" } });
    fireEvent.click(screen.getByRole("button", { name: "Save" }));

    await waitFor(() =>
      expect(updateTherapyCollection).toHaveBeenCalledWith("c1", { name: "Grounding toolkit" }),
    );
    expect(await screen.findByText("Grounding toolkit")).toBeTruthy();
    expect(listTherapyCollections).toHaveBeenCalledTimes(2);
  });

  it("archives a collection only after explicit confirmation, then refreshes", async () => {
    vi.mocked(listTherapyCollections)
      .mockResolvedValueOnce([collection] as never)
      .mockResolvedValue([] as never);
    vi.mocked(archiveTherapyCollection).mockResolvedValue({
      ...collection,
      archived_at: "2026-10-13T10:00:00Z",
    } as never);

    renderPage();

    const archiveButton = await screen.findByRole("button", { name: "Archive" });
    fireEvent.click(archiveButton);
    expect(archiveTherapyCollection).not.toHaveBeenCalled();

    fireEvent.click(await screen.findByRole("button", { name: "Confirm archive" }));
    await waitFor(() => expect(archiveTherapyCollection).toHaveBeenCalledWith("c1"));
    expect(await screen.findByText("No therapy collections yet.")).toBeTruthy();
    expect(listTherapyCollections).toHaveBeenCalledTimes(2);
  });

  it("shows a save failure once as a generic privacy-safe message", async () => {
    vi.mocked(listTherapyCollections).mockResolvedValue([collection] as never);
    // Exactly what the typed boundary throws on a native failure: the
    // serialized Rust AppError travels as the cause, never as the message.
    const rawSerializedError = JSON.stringify({
      code: "NOT_FOUND",
      message: "therapy collection c1 not found in workspace w1",
    });
    vi.mocked(updateTherapyCollection).mockRejectedValue(
      new CommandInvocationError("update_therapy_collection", rawSerializedError),
    );

    renderPage();

    fireEvent.click(await screen.findByRole("button", { name: "Edit" }));
    fireEvent.change(screen.getByDisplayValue("Coping toolkit"), {
      target: { value: "Grounding toolkit" },
    });
    fireEvent.click(screen.getByRole("button", { name: "Save" }));

    const error = await screen.findByText("The requested item could not be found.");
    expect(screen.getAllByText("The requested item could not be found.")).toHaveLength(1);
    // The raw serialized error (IDs, technical detail) never reaches the UI.
    expect(screen.queryByText(/not found in workspace/)).toBeNull();
    expect(error.className).toContain("joi-text--danger");
  });
});
