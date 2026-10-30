import { Link } from "react-router";
import { useTranslation } from "react-i18next";

export interface BreadcrumbItem {
  label: string;
  to?: string;
}

interface BreadcrumbProps {
  items: BreadcrumbItem[];
}

/**
 * File-manager style breadcrumb. The last item is the current page and is not
 * a link; earlier items link back so a child view can return to its parent.
 * Item labels come from the calling screen (translated user-facing copy or
 * user-authored titles) and are rendered verbatim here.
 */
function Breadcrumb({ items }: BreadcrumbProps) {
  const { t } = useTranslation();
  if (items.length === 0) return null;
  return (
    <nav
      aria-label={t("shell.breadcrumb")}
      className="joi-breadcrumb mb-4 flex flex-wrap items-center gap-1 text-sm"
    >
      {items.map((item, index) => {
        const isLast = index === items.length - 1;
        return (
          <span key={`${item.label}-${index}`} className="flex items-center gap-1">
            {index > 0 ? <span className="joi-text--subtle">/</span> : null}
            {item.to && !isLast ? (
              <Link
                to={item.to}
                className="joi-breadcrumb__item joi-btn joi-btn--secondary joi-btn--sm px-2 py-1"
              >
                {item.label}
              </Link>
            ) : (
              <span className="truncate joi-text">{item.label}</span>
            )}
          </span>
        );
      })}
    </nav>
  );
}

export default Breadcrumb;
