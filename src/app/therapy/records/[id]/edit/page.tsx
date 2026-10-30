import { useEffect, useState } from "react";
import { useNavigate, useParams } from "react-router";
import { useTranslation } from "react-i18next";
import TherapyRecordForm, {
  type TherapyRecordFormState,
} from "@/components/therapy/TherapyRecordForm";
import Breadcrumb from "@/components/navigation/Breadcrumb";
import PageContainer from "@/components/navigation/PageContainer";
import PageSkeleton from "@/components/navigation/PageSkeleton";
import { getTherapyRecord, updateTherapyRecord } from "@/lib/api/therapyApi";
import { toDisplayMessage } from "@/lib/errors";

function EditTherapyRecordPage() {
  const { t } = useTranslation();
  const { id } = useParams<{ id: string }>();
  const navigate = useNavigate();
  const [initial, setInitial] = useState<Partial<TherapyRecordFormState> | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (!id) return;
    void getTherapyRecord(id)
      .then((record) => {
        setInitial({
          record_type: record.record_type,
          title: record.title,
          occurred_on: record.occurred_on ?? "",
          content_markdown: record.content_markdown,
          status: record.status,
        });
      })
      .catch((err) => setError(toDisplayMessage(err)));
  }, [id]);

  const handleSubmit = async (values: TherapyRecordFormState) => {
    if (!id) return;
    try {
      const record = await updateTherapyRecord(id, {
        record_type: values.record_type,
        title: values.title,
        content_markdown: values.content_markdown,
        occurred_on: values.occurred_on || null,
        status: values.status,
      });
      navigate(`/therapy/records/${record.id}`);
    } catch (err) {
      setError(toDisplayMessage(err));
    }
  };

  if (!initial) {
    return (
      <section>
        <h1 className="text-2xl font-semibold">{t("therapy.editRecordTitle")}</h1>
        {error ? (
          <p className="joi-text--danger mt-2 text-sm">{error}</p>
        ) : (
          <PageSkeleton variant="form" />
        )}
      </section>
    );
  }

  return (
    <PageContainer>
      <Breadcrumb
        items={[
          { label: t("therapy.title"), to: "/therapy" },
          { label: t("therapy.collectionFallbackTitle"), to: "/therapy" },
          { label: t("therapy.recordDetailTitle"), to: `/therapy/records/${id}` },
          { label: t("therapy.editRecordTitle") },
        ]}
      />
      <h1 className="text-2xl font-semibold">{t("therapy.editRecordTitle")}</h1>
      {error ? <p className="joi-text--danger mt-2 text-sm">{error}</p> : null}
      <div className="mt-4">
        <TherapyRecordForm
          initial={initial}
          isEdit
          onSubmit={handleSubmit}
          submitLabel={t("common.actions.saveChanges")}
        />
      </div>
    </PageContainer>
  );
}

export default EditTherapyRecordPage;
