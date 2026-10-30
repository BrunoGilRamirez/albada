import { useEffect, useState } from "react";
import { Link, useNavigate, useParams } from "react-router";
import { useTranslation } from "react-i18next";
import MarkdownPreview from "@/components/markdown/MarkdownPreview";
import TherapyReferenceEditor from "@/components/therapy/TherapyReferenceEditor";
import { archiveTherapyRecord, getTherapyRecord } from "@/lib/api/therapyApi";
import { ensureDefaultWorkspace } from "@/lib/api/workspaceApi";
import { toDisplayMessage } from "@/lib/errors";
import { formatDate } from "@/lib/format";
import type { TherapyRecord } from "@/lib/types";
import TherapyPrivacyBanner from "@/components/therapy/TherapyPrivacyBanner";
import Breadcrumb from "@/components/navigation/Breadcrumb";
import PageContainer from "@/components/navigation/PageContainer";
import PageSkeleton from "@/components/navigation/PageSkeleton";

/**
 * The occurred-on date-only value is an application-owned date: the read-only
 * label renders through the shared Intl date formatter with a local-noon
 * anchor (the established 8.6.8/8.6.9 pattern). Stored values and the edit
 * form's date input keep the exact ISO string; authored record content never
 * passes through this.
 */
function formatRecordDate(occurredOn: string): string {
  return formatDate(new Date(`${occurredOn}T12:00:00`), { dateStyle: "medium" });
}

function TherapyRecordDetailPage() {
  const { t } = useTranslation();
  const { id } = useParams<{ id: string }>();
  const navigate = useNavigate();
  const [record, setRecord] = useState<TherapyRecord | null>(null);
  const [workspaceId, setWorkspaceId] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  const load = async () => {
    if (!id) return;
    setError(null);
    try {
      const data = await getTherapyRecord(id);
      setRecord(data);
    } catch (err) {
      setError(toDisplayMessage(err));
    }
  };

  useEffect(() => {
    void load();
  }, [id]);

  useEffect(() => {
    void ensureDefaultWorkspace()
      .then((workspace) => setWorkspaceId(workspace.id))
      .catch((err) => setError(toDisplayMessage(err)));
  }, []);

  const handleArchive = async () => {
    if (!id || !record) return;
    try {
      await archiveTherapyRecord(id);
      navigate(`/therapy/collections/${record.therapy_collection_id}`);
    } catch (err) {
      setError(toDisplayMessage(err));
    }
  };

  if (!record) {
    return (
      <section>
        <h1 className="text-2xl font-semibold">{t("therapy.recordDetailTitle")}</h1>
        {error ? <p className="joi-text--danger mt-2 text-sm">{error}</p> : <PageSkeleton />}
      </section>
    );
  }

  return (
    <PageContainer>
      <Breadcrumb
        items={[
          { label: t("therapy.title"), to: "/therapy" },
          {
            label: t("therapy.collectionFallbackTitle"),
            to: `/therapy/collections/${record.therapy_collection_id}`,
          },
          { label: record.title },
        ]}
      />
      <TherapyPrivacyBanner />
      <div className="flex flex-wrap items-center gap-3">
        {/* Authored title renders verbatim; only application-owned chrome localizes. */}
        <h1 className="text-2xl font-semibold">{record.title}</h1>
        <span className="joi-badge joi-badge--neutral">
          {t(`therapy.recordTypes.${record.record_type}`)}
        </span>
        <span className="joi-badge joi-badge--neutral">
          {t(`therapy.recordStatuses.${record.status}`)}
        </span>
      </div>

      {record.occurred_on ? (
        <p className="joi-text--muted mt-1 text-sm">{formatRecordDate(record.occurred_on)}</p>
      ) : null}

      {error ? <p className="joi-text--danger mt-2 text-sm">{error}</p> : null}

      <div className="mt-4 flex flex-wrap gap-2">
        <Link
          to={`/therapy/records/${record.id}/edit`}
          className="joi-btn joi-btn--primary px-3 py-1.5"
        >
          {t("common.actions.edit")}
        </Link>
        <button
          type="button"
          onClick={handleArchive}
          className="joi-btn joi-btn--danger px-3 py-1.5"
        >
          {t("common.actions.archive")}
        </button>
      </div>

      <div className="mt-6">
        <h2 className="joi-text mb-1 text-sm font-medium">{t("common.labels.content")}</h2>
        <MarkdownPreview content={record.content_markdown} emptyText={t("common.markdown.empty")} />
      </div>

      {workspaceId ? (
        <div className="mt-6">
          <TherapyReferenceEditor recordId={record.id} workspaceId={workspaceId} />
        </div>
      ) : null}
    </PageContainer>
  );
}

export default TherapyRecordDetailPage;
