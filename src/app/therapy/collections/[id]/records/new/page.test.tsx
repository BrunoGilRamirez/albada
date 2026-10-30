import { describe, expect, it, vi, beforeEach } from "vitest";
import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { MemoryRouter, Route, Routes } from "react-router";
import NewTherapyRecordPage from "@/app/therapy/collections/[id]/records/new/page";
import { createTherapyRecord } from "@/lib/api/therapyApi";

vi.mock("@/lib/api/therapyApi", () => ({
  createTherapyRecord: vi.fn(),
}));

const createdRecord = {
  id: "tr1",
  therapy_collection_id: "c1",
  record_type: "note",
  title: "Grounding exercise",
  content_markdown: "4-7-8 breathing before the review call.",
  occurred_on: null,
  status: "open",
  privacy_scope: "therapy_private",
  created_at: "2026-09-08T20:00:00Z",
  updated_at: "2026-09-08T20:00:00Z",
  archived_at: null,
};

function renderPage() {
  return render(
    <MemoryRouter initialEntries={["/therapy/collections/c1/records/new"]}>
      <Routes>
        <Route path="/therapy/collections/:id/records/new" element={<NewTherapyRecordPage />} />
        <Route path="/therapy/records/:recordId" element={<p>Therapy record stub</p>} />
      </Routes>
    </MemoryRouter>,
  );
}

describe("NewTherapyRecordPage", () => {
  beforeEach(() => {
    vi.mocked(createTherapyRecord).mockReset();
    vi.mocked(createTherapyRecord).mockResolvedValue(createdRecord as never);
  });

  it("creates a therapy record inside the selected collection only", async () => {
    renderPage();

    fireEvent.change(await screen.findByLabelText("Title"), {
      target: { value: "Grounding exercise" },
    });
    fireEvent.change(screen.getByLabelText("Content (Markdown)"), {
      target: { value: "4-7-8 breathing before the review call." },
    });
    fireEvent.click(screen.getByRole("button", { name: "Create record" }));

    await waitFor(() => expect(createTherapyRecord).toHaveBeenCalledTimes(1));
    expect(createTherapyRecord).toHaveBeenCalledWith({
      therapy_collection_id: "c1",
      record_type: "note",
      title: "Grounding exercise",
      content_markdown: "4-7-8 breathing before the review call.",
      occurred_on: null,
    });
    expect(await screen.findByText("Therapy record stub")).toBeTruthy();
  });

  it("keeps the record form private-scoped: no executive fields are rendered", async () => {
    renderPage();

    await screen.findByLabelText("Title");
    // The creation surface stays inside the therapy area: no task/project
    // pickers, no progress, and no link into the executive dashboard.
    expect(screen.queryByLabelText(/priority/i)).toBeNull();
    expect(screen.queryByRole("button", { name: /Create task/i })).toBeNull();
    expect(screen.queryByText(/progress/i)).toBeNull();
  });

  it("shows the API error when the record cannot be created", async () => {
    vi.mocked(createTherapyRecord).mockRejectedValue(new Error("Collection not found"));
    renderPage();

    fireEvent.change(await screen.findByLabelText("Title"), {
      target: { value: "Grounding exercise" },
    });
    fireEvent.click(screen.getByRole("button", { name: "Create record" }));

    expect(await screen.findByText("Collection not found")).toBeTruthy();
    expect(screen.queryByText("Therapy record stub")).toBeNull();
  });
});
