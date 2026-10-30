import { describe, expect, it } from "vitest";
import { render, screen } from "@testing-library/react";
import ExportPreview from "@/components/export/ExportPreview";
import type { ExportPreviewItem } from "@/lib/types";

const items: ExportPreviewItem[] = [
  {
    id: "j1",
    entity_type: "journal_entry",
    title: "A quiet day",
    occurred_on: "2026-09-04",
    privacy_scope: "private",
    content_hash: "abc123",
  },
  {
    id: "t1",
    entity_type: "therapy_record",
    title: "Therapy session",
    occurred_on: "2026-09-05",
    privacy_scope: "therapy_private",
    content_hash: "def456",
  },
];

const fingerprint = "0123456789abcdef0123456789abcdef";

describe("ExportPreview", () => {
  it("renders the exact selected items", () => {
    render(
      <ExportPreview
        preview={{
          item_count: items.length,
          items,
          selection_fingerprint: fingerprint,
          has_therapy_private: true,
        }}
      />,
    );

    expect(screen.getByText("A quiet day")).toBeTruthy();
    expect(screen.getByText("Therapy session")).toBeTruthy();
    // The application-owned date-only value renders through the shared Intl
    // formatter; the raw stored ISO string never reaches the screen.
    const expectedDate = new Date("2026-09-04T12:00:00").toLocaleDateString("en-US", {
      dateStyle: "medium",
    });
    expect(screen.getByText(expectedDate)).toBeTruthy();
    expect(screen.queryByText("2026-09-04")).toBeNull();
    expect(screen.getByText("2 items")).toBeTruthy();
  });

  it("warns when a therapy-private item is selected", () => {
    render(
      <ExportPreview
        preview={{
          item_count: items.length,
          items,
          selection_fingerprint: fingerprint,
          has_therapy_private: true,
        }}
      />,
    );

    expect(screen.getByText(/therapy-private content/i)).toBeTruthy();
    expect(screen.getByText("therapy_private")).toBeTruthy();
  });

  it("does not warn when no therapy-private item is selected", () => {
    const safeItems = [items[0]];
    render(
      <ExportPreview
        preview={{
          item_count: safeItems.length,
          items: safeItems,
          selection_fingerprint: fingerprint,
          has_therapy_private: false,
        }}
      />,
    );

    expect(screen.queryByText(/therapy-private content/i)).toBeNull();
  });

  it("shows the resolved selection fingerprint in a compact form", () => {
    render(
      <ExportPreview
        preview={{
          item_count: items.length,
          items,
          selection_fingerprint: fingerprint,
          has_therapy_private: false,
        }}
      />,
    );

    expect(screen.getByText("Selection fingerprint")).toBeTruthy();
    // Compact form: first 8 chars plus an ellipsis, never the full fingerprint.
    expect(screen.getByText("01234567…")).toBeTruthy();
    expect(screen.queryByText(fingerprint)).toBeNull();
  });

  it("shows content hashes compactly and never the source text", () => {
    render(
      <ExportPreview
        preview={{
          item_count: items.length,
          items,
          selection_fingerprint: fingerprint,
          has_therapy_private: false,
        }}
      />,
    );

    // Compact hash badges for each item.
    expect(screen.getByText("abc123")).toBeTruthy();
    expect(screen.getByText("def456")).toBeTruthy();
    // The therapy record's source text must never appear.
    expect(screen.queryByText(/therapy session content/i)).toBeNull();
  });
});
