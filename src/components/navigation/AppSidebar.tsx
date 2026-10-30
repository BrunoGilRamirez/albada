import { NavLink } from "react-router";
import { useTranslation } from "react-i18next";
import { useTheme } from "@/lib/theme";
import type { Translation } from "@/lib/i18n/resources";
import NotificationBell from "@/components/notifications/NotificationBell";

interface NavigationItem {
  to: string;
  labelKey: keyof Translation["shell"]["nav"];
  end?: boolean;
}

const NAV_ITEMS: NavigationItem[] = [
  { to: "/", labelKey: "today", end: true },
  { to: "/calendar", labelKey: "calendar" },
  { to: "/projects", labelKey: "projects" },
  { to: "/goals", labelKey: "goals" },
  { to: "/tasks", labelKey: "tasks" },
  { to: "/journal", labelKey: "journal" },
  { to: "/therapy", labelKey: "therapy" },
  { to: "/analytics", labelKey: "analytics" },
  { to: "/settings", labelKey: "settings" },
];

function AppSidebar() {
  const { mode, toggle } = useTheme();
  const { t } = useTranslation();

  const modeLabel = t(`shell.theme.${mode}`);
  const switchAria =
    mode === "dark" ? t("shell.theme.switchToLight") : t("shell.theme.switchToDark");

  return (
    <nav
      aria-label={t("shell.primaryNav")}
      className="joi-shell__sidebar relative w-56 shrink-0 gap-0 border-r 2xl:w-64"
    >
      <div className="flex items-center justify-between gap-2 px-4 py-5">
        <p className="text-sm font-semibold tracking-wide joi-text--muted">Albada</p>
        <NotificationBell />
      </div>
      <ul className="flex flex-col gap-1 px-2">
        {NAV_ITEMS.map((item) => (
          <li key={item.to}>
            <NavLink
              to={item.to}
              end={item.end}
              className={({ isActive }) =>
                `joi-nav__item block rounded-md px-3 py-2 text-sm font-medium${isActive ? " joi-nav__item--active" : ""}`
              }
            >
              {t(`shell.nav.${item.labelKey}`)}
            </NavLink>
          </li>
        ))}
        <li className="flex flex-col gap-2 pt-4">
          <button
            onClick={toggle}
            aria-label={switchAria}
            aria-pressed={mode === "dark"}
            className="joi-nav__item flex flex-col items-center gap-1 rounded-md px-3 py-2 text-sm font-medium"
          >
            {modeLabel}
          </button>
        </li>
      </ul>
    </nav>
  );
}

export default AppSidebar;
