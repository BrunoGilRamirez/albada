import { useNavigate, useParams } from "react-router";
import { useTranslation } from "react-i18next";
import TherapyRecordForm, {
  type TherapyRecordFormState,
} from "@/components/therapy/TherapyRecordForm";
import Breadcrumb from "@/components/navigation/Breadcrumb";
import PageContainer from "@/components/navigation/PageContainer";
import { createTherapyRecord } from "@/lib/api/therapyApi";
import { toDisplayMessage } from "@/lib/errors";
import { useState } from "react";

function NewTherapyRecordPage() {
  const { t } = useTranslation();
  const { id } = useParams<{ id: string }>();
  const navigate = useNavigate();
  const [error, setError] = useState<string | null>(null);

  const handleSubmit = async (values: TherapyRecordFormState) => {
    if (!id) return;
    try {
      const record = await createTherapyRecord({
        therapy_collection_id: id,
        record_type: values.record_type,
        title: values.title,
        content_markdown: values.content_markdown,
        occurred_on: values.occurred_on || null,
      });
      navigate(`/therapy/records/${record.id}`);
    } catch (err) {
      setError(toDisplayMessage(err));
    }
  };

  if (!id) {
    return (
      <section>
        <h1 className="text-2xl font-semibold">{t("therapy.newRecordTitle")}</h1>
        <p className="joi-text--danger mt-2 text-sm">{t("therapy.invalidCollectionId")}</p>
      </section>
    );
  }

  return (
    <PageContainer>
      <Breadcrumb
        items={[
          { label: t("therapy.title"), to: "/therapy" },
          { label: t("therapy.collectionFallbackTitle"), to: `/therapy/collections/${id}` },
          { label: t("therapy.newRecordTitle") },
        ]}
      />
      <h1 className="text-2xl font-semibold">{t("therapy.newRecordTitle")}</h1>
      {error ? <p className="joi-text--danger mt-2 text-sm">{error}</p> : null}
      <div className="mt-4">
        <TherapyRecordForm onSubmit={handleSubmit} submitLabel={t("therapy.createRecordSubmit")} />
      </div>
    </PageContainer>
  );
}

export default NewTherapyRecordPage;
