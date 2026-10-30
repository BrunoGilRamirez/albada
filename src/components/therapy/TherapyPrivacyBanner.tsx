import { useState } from "react";
import { useTranslation } from "react-i18next";

const THERAPY_PRIVACY_BANNER_STORAGE_KEY = "albada-therapy-privacy-banner-dismissed";

function wasBannerDismissed(): boolean {
  try {
    return localStorage.getItem(THERAPY_PRIVACY_BANNER_STORAGE_KEY) === "true";
  } catch {
    return false;
  }
}

function TherapyPrivacyBanner() {
  const { t } = useTranslation();
  const [dismissed, setDismissed] = useState(wasBannerDismissed);

  if (dismissed) return null;

  const dismiss = () => {
    setDismissed(true);
    try {
      localStorage.setItem(THERAPY_PRIVACY_BANNER_STORAGE_KEY, "true");
    } catch {
      // Storage unavailable; the banner remains dismissed for this session.
    }
  };

  return (
    <div role="note" className="joi-alert joi-alert--info relative pr-10">
      <span>{t("therapy.privacyBanner")}</span>
      <button
        type="button"
        className="joi-btn joi-btn--ghost joi-btn--sm absolute right-2 top-1/2 -translate-y-1/2"
        aria-label={t("common.actions.close")}
        title={t("common.actions.close")}
        onClick={dismiss}
      >
        ×
      </button>
    </div>
  );
}

export default TherapyPrivacyBanner;
