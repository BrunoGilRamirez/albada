import { useTranslation } from "react-i18next";

interface PageSkeletonProps {
  variant?: "detail" | "form" | "analytics";
}

function SkeletonLine({ className = "" }: { className?: string }) {
  return <div className={`joi-skeleton ${className}`} />;
}

function PageSkeleton({ variant = "detail" }: PageSkeletonProps) {
  const { t } = useTranslation();

  return (
    <div role="status" aria-label={t("common.loading")} className="joi-skeleton-page">
      <span className="sr-only">{t("common.loading")}</span>
      <div aria-hidden="true" className="joi-skeleton-page__visual">
        <div className="joi-skeleton-page__crumbs">
          <SkeletonLine className="joi-skeleton--crumb" />
          <SkeletonLine className="joi-skeleton--crumb joi-skeleton--crumb-short" />
        </div>

        {variant === "form" ? (
          <>
            <SkeletonLine className="joi-skeleton--title" />
            <div className="joi-skeleton-page__form">
              <SkeletonLine className="joi-skeleton--field-label" />
              <SkeletonLine className="joi-skeleton--field" />
              <SkeletonLine className="joi-skeleton--field-label" />
              <SkeletonLine className="joi-skeleton--field joi-skeleton--field-large" />
              <div className="joi-skeleton-page__actions">
                <SkeletonLine className="joi-skeleton--button" />
                <SkeletonLine className="joi-skeleton--button joi-skeleton--button-short" />
              </div>
            </div>
          </>
        ) : variant === "analytics" ? (
          <>
            <SkeletonLine className="joi-skeleton--title" />
            <SkeletonLine className="joi-skeleton--short" />
            <div className="joi-skeleton-page__toolbar">
              <SkeletonLine className="joi-skeleton--control" />
              <SkeletonLine className="joi-skeleton--control" />
              <SkeletonLine className="joi-skeleton--button joi-skeleton--button-short" />
            </div>
            <div className="joi-skeleton-page__cards">
              <SkeletonLine className="joi-skeleton--card" />
              <SkeletonLine className="joi-skeleton--card" />
            </div>
          </>
        ) : (
          <>
            <div className="joi-skeleton-page__heading">
              <SkeletonLine className="joi-skeleton--title" />
              <div className="joi-skeleton-page__badges">
                <SkeletonLine className="joi-skeleton--badge" />
                <SkeletonLine className="joi-skeleton--badge joi-skeleton--badge-short" />
              </div>
            </div>
            <div className="joi-skeleton-page__actions">
              <SkeletonLine className="joi-skeleton--button" />
              <SkeletonLine className="joi-skeleton--button joi-skeleton--button-short" />
              <SkeletonLine className="joi-skeleton--button joi-skeleton--button-short" />
            </div>
            <div className="joi-skeleton-page__body">
              <SkeletonLine className="joi-skeleton--panel" />
              <SkeletonLine className="joi-skeleton--panel joi-skeleton--panel-tall" />
              <SkeletonLine className="joi-skeleton--panel" />
            </div>
          </>
        )}
      </div>
    </div>
  );
}

export default PageSkeleton;
