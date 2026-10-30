import { useTranslation } from "react-i18next";
import { isTherapyPrivate } from "@/lib/privacy";
import { formatDate } from "@/lib/format";
import type { ExportPreviewItem } from "@/lib/types";

interface ExportPreviewProps {
  preview: {
    item_count: number;
    total_count?: number;
    truncated?: boolean;
    items: ExportPreviewItem[];
    selection_fingerprint: string;
    has_therapy_private: boolean;
  } | null;
}

// Privacy classification stays explicit: the scope text is rendered next to the
// chip, so the therapy-private state is never color-only. The scope value
// itself is stored data (`private`, `therapy_private`) and is rendered
// verbatim; only the surrounding chrome is translated.
function privacyBadgeClass(scope: string): string {
  if (isTherapyPrivate(scope)) {
    return "joi-badge--neutral";
  }
  return "joi-badge--neutral";
}

/** Compact, non-content representation of a content hash (first 8 chars). */
function shortHash(hash: string): string {
  return hash.length > 8 ? `${hash.slice(0, 8)}…` : hash;
}

/**
 * Application-owned date-only value. Rendered through the shared Intl
 * formatter (midday anchor to keep the calendar day stable across time zones);
 * the stored ISO string is never shown.
 */
function formatOccurredOn(occurredOn: string): string {
  return formatDate(new Date(`${occurredOn}T12:00:00`), { dateStyle: "medium" });
}

export function ExportPreview({ preview }: ExportPreviewProps) {
  const { t } = useTranslation();
  if (!preview) return null;
  const total = preview.total_count ?? preview.item_count;

  return (
    <div className="joi-border--default rounded-lg border p-4">
      <div className="flex items-center justify-between">
        <h3 className="text-sm font-medium">{t("common.exportPreview.title")}</h3>
        <span className="joi-text--muted text-xs">
          {t("common.exportPreview.itemCount", { count: total })}
          {preview.truncated
            ? ` ${t("common.exportPreview.truncatedCount", { shown: preview.items.length })}`
            : ""}
        </span>
      </div>

      {preview.truncated ? (
        <p className="joi-text--muted mt-2 text-xs" role="status">
          {t("common.exportPreview.largeSelection", {
            shown: preview.items.length,
            total,
          })}
        </p>
      ) : null}

      {preview.has_therapy_private ? (
        <p className="joi-alert joi-alert--warning mt-2 rounded-md p-2 text-xs">
          {t("common.exportPreview.therapyWarning")}
        </p>
      ) : null}

      <div className="joi-text--muted mt-3 flex flex-wrap items-center gap-2 text-xs">
        <span className="font-medium">{t("common.exportPreview.fingerprintLabel")}</span>
        <code className="joi-badge--neutral rounded px-1.5 py-0.5 font-mono">
          {shortHash(preview.selection_fingerprint)}
        </code>
        <span>{t("common.exportPreview.fingerprintHelp")}</span>
      </div>

      <ul className="mt-3 flex flex-col gap-2">
        {preview.items.map((item) => (
          <li
            key={`${item.entity_type}-${item.id}`}
            className="joi-border--default flex items-center justify-between rounded-md border px-2 py-1.5 text-sm"
          >
            <span className="joi-text truncate">{item.title}</span>
            <span className="flex shrink-0 items-center gap-2 text-xs">
              <span
                className={`rounded-full px-2 py-0.5 font-medium ${privacyBadgeClass(item.privacy_scope)}`}
              >
                {item.privacy_scope}
              </span>
              {item.occurred_on ? (
                <span className="joi-text--muted">{formatOccurredOn(item.occurred_on)}</span>
              ) : null}
              <span
                className="joi-badge--neutral rounded px-1.5 py-0.5 font-mono"
                title={t("common.exportPreview.contentFingerprintTitle")}
              >
                {shortHash(item.content_hash)}
              </span>
            </span>
          </li>
        ))}
      </ul>
    </div>
  );
}

export default ExportPreview;
