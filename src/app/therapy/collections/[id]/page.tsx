import { useEffect, useState } from "react";
import { Link, useParams } from "react-router";
import { useTranslation } from "react-i18next";
import layoutColumnsIcon from "@/assets/layout-icons/layout-columns.svg";
import layoutListIcon from "@/assets/layout-icons/layout-list.svg";
import layoutTilesIcon from "@/assets/layout-icons/layout-tiles.svg";
import { archiveTherapyRecord, listTherapyRecords } from "@/lib/api/therapyApi";
import { listTherapyCollections } from "@/lib/api/therapyApi";
import { ensureDefaultWorkspace } from "@/lib/api/workspaceApi";
import { toDisplayMessage } from "@/lib/errors";
import { formatDate } from "@/lib/format";
import type { TherapyRecord } from "@/lib/types";
import TherapyPrivacyBanner from "@/components/therapy/TherapyPrivacyBanner";
import Breadcrumb from "@/components/navigation/Breadcrumb";
import PageContainer from "@/components/navigation/PageContainer";

type TherapyRecordListLayout = "list" | "columns" | "tiles";

const LAYOUT_CLASS: Record<TherapyRecordListLayout, string> = {
  list: "grid grid-cols-1 gap-3",
  columns: "grid grid-cols-1 gap-3 md:grid-cols-2",
  tiles: "grid gap-3 [grid-template-columns:repeat(auto-fill,minmax(14rem,1fr))]",
};

const LAYOUT_ORDER: readonly TherapyRecordListLayout[] = ["list", "columns", "tiles"];

const LAYOUT_LABEL_KEY = {
  list: "therapy.layoutList",
  columns: "therapy.layoutColumns",
  tiles: "therapy.layoutTiles",
} as const satisfies Record<TherapyRecordListLayout, `therapy.${string}`>;

const LAYOUT_ICON = {
  list: layoutListIcon,
  columns: layoutColumnsIcon,
  tiles: layoutTilesIcon,
} as const satisfies Record<TherapyRecordListLayout, string>;

const LAYOUT_STORAGE_KEY = "joi-projectlife-therapy-record-list-layout";

function isTherapyRecordListLayout(value: string | null): value is TherapyRecordListLayout {
  return value !== null && (LAYOUT_ORDER as readonly string[]).includes(value);
}

function readStoredLayout(): TherapyRecordListLayout {
  try {
    const stored = localStorage.getItem(LAYOUT_STORAGE_KEY);
    if (isTherapyRecordListLayout(stored)) return stored;
  } catch {
    // Storage unavailable; keep the default.
  }
  return "list";
}

function nextTherapyRecordListLayout(layout: TherapyRecordListLayout): TherapyRecordListLayout {
  const index = LAYOUT_ORDER.indexOf(layout);
  return LAYOUT_ORDER[(index + 1) % LAYOUT_ORDER.length];
}

/**
 * The record occurred-on date-only value is an application-owned date: the
 * read-only label renders through the shared Intl date formatter with a
 * local-noon anchor (the established 8.6.8/8.6.9 pattern). Stored values and
 * the record form's date input keep the exact ISO string; authored record
 * titles never pass through this.
 */
function formatRecordDate(occurredOn: string): string {
  return formatDate(new Date(`${occurredOn}T12:00:00`), { dateStyle: "medium" });
}

function TherapyCollectionPage() {
  const { t } = useTranslation();
  const { id } = useParams<{ id: string }>();
  const [records, setRecords] = useState<TherapyRecord[]>([]);
  const [collectionName, setCollectionName] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [confirmArchiveId, setConfirmArchiveId] = useState<string | null>(null);
  const [layout, setLayout] = useState<TherapyRecordListLayout>(readStoredLayout);

  useEffect(() => {
    try {
      localStorage.setItem(LAYOUT_STORAGE_KEY, layout);
    } catch {
      // Storage unavailable; the switcher still works for this session.
    }
  }, [layout]);

  const load = async () => {
    if (!id) return;
    setError(null);
    try {
      const data = await listTherapyRecords(id);
      const sorted = [...data].sort(
        (a, b) => new Date(b.created_at).getTime() - new Date(a.created_at).getTime(),
      );
      setRecords(sorted);
    } catch (err) {
      setError(toDisplayMessage(err));
    }
  };

  useEffect(() => {
    void load();
  }, [id]);

  useEffect(() => {
    if (!id) return;
    void ensureDefaultWorkspace()
      .then(async (workspace) => {
        const collections = await listTherapyCollections(workspace.id);
        const match = collections.find((collection) => collection.id === id);
        if (match) {
          setCollectionName(match.name);
        }
      })
      .catch((err) => setError(toDisplayMessage(err)));
  }, [id]);

  const handleArchive = async (recordId: string) => {
    if (confirmArchiveId !== recordId) {
      setConfirmArchiveId(recordId);
      return;
    }
    try {
      await archiveTherapyRecord(recordId);
      setConfirmArchiveId(null);
      await load();
    } catch (err) {
      setError(toDisplayMessage(err));
    }
  };

  if (!id) {
    return (
      <section>
        <h1 className="text-2xl font-semibold">{t("therapy.collection")}</h1>
        <p className="joi-text--danger mt-2 text-sm">{t("therapy.invalidCollectionId")}</p>
      </section>
    );
  }

  return (
    <PageContainer>
      <Breadcrumb
        items={[
          { label: t("therapy.title"), to: "/therapy" },
          { label: collectionName ?? t("therapy.collectionFallbackTitle") },
        ]}
      />
      <div className="flex items-center justify-between">
        <h1 className="text-2xl font-semibold">
          {collectionName ?? t("therapy.collectionFallbackTitle")}
        </h1>
        <Link
          to={`/therapy/collections/${id}/records/new`}
          className="joi-btn joi-btn--primary px-3 py-1.5"
        >
          {t("therapy.addRecord")}
        </Link>
      </div>
      <div className="mt-3">
        <TherapyPrivacyBanner />
      </div>
      {error ? <p className="joi-text--danger mt-2 text-sm">{error}</p> : null}

      {records.length === 0 ? (
        <p className="joi-empty__state mt-4">{t("therapy.recordsEmpty")}</p>
      ) : (
        <div className="mt-4 flex flex-col gap-3">
          <div role="group" aria-label={t("therapy.layoutGroupLabel")} className="flex">
            <button
              type="button"
              onClick={() => setLayout(nextTherapyRecordListLayout(layout))}
              aria-label={t(LAYOUT_LABEL_KEY[layout])}
              title={t(LAYOUT_LABEL_KEY[layout])}
              className="joi-btn joi-btn--secondary joi-btn--sm"
            >
              <img
                src={LAYOUT_ICON[layout]}
                alt=""
                aria-hidden="true"
                className="joi-layout-icon"
              />
            </button>
          </div>
          <ul className={LAYOUT_CLASS[layout]}>
            {records.map((record) => (
              <li
                key={record.id}
                className={`joi-surface--base joi-border--default rounded-lg border p-3 ${
                  layout === "tiles"
                    ? "flex aspect-square flex-col gap-3"
                    : "flex items-center justify-between"
                }`}
              >
                {/* Record titles are user-authored and render verbatim; only the
                  enum badges and action chrome localize with the UI language. */}
                <Link
                  to={`/therapy/records/${record.id}`}
                  className="flex min-w-0 flex-1 self-stretch flex-col justify-center gap-1 rounded-md p-1"
                >
                  <span className="joi-text text-sm font-medium">{record.title}</span>
                  <span className="joi-text--muted flex flex-wrap items-center gap-2 text-xs">
                    <span className="joi-badge joi-badge--neutral">
                      {t(`therapy.recordTypes.${record.record_type}`)}
                    </span>
                    <span className="joi-badge joi-badge--neutral">
                      {t(`therapy.recordStatuses.${record.status}`)}
                    </span>
                    {record.occurred_on ? (
                      <span>{formatRecordDate(record.occurred_on)}</span>
                    ) : null}
                  </span>
                </Link>
                <div className="flex items-center gap-2">
                  <Link
                    to={`/therapy/records/${record.id}/edit`}
                    className="joi-btn joi-btn--secondary px-3 py-1.5"
                  >
                    {t("common.actions.edit")}
                  </Link>
                  <button
                    type="button"
                    onClick={() => void handleArchive(record.id)}
                    className={`joi-btn px-3 py-1.5 ${
                      confirmArchiveId === record.id ? "joi-btn--danger" : "joi-btn--secondary"
                    }`}
                  >
                    {confirmArchiveId === record.id
                      ? t("common.actions.confirmArchive")
                      : t("common.actions.archive")}
                  </button>
                </div>
              </li>
            ))}
          </ul>
        </div>
      )}
    </PageContainer>
  );
}

export default TherapyCollectionPage;
