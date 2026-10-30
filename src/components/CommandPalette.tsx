import { useEffect } from "react";
import { useTranslation } from "react-i18next";

interface CommandPaletteProps {
  isOpen: boolean;
  onOpenChange: (open: boolean) => void;
}

function CommandPalette({ isOpen, onOpenChange }: CommandPaletteProps) {
  const { t } = useTranslation();

  useEffect(() => {
    function handleKeyDown(event: KeyboardEvent) {
      if (event.ctrlKey && event.key.toLowerCase() === "k") {
        event.preventDefault();
        onOpenChange(!isOpen);
      }

      if (event.key === "Escape" && isOpen) {
        event.preventDefault();
        onOpenChange(false);
      }
    }

    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [isOpen, onOpenChange]);

  if (!isOpen) {
    return null;
  }

  return (
    <div
      role="dialog"
      aria-modal="true"
      aria-label={t("shell.palette.label")}
      className="joi-dialog__backdrop z-50 items-start justify-center p-4 pt-24"
      onClick={() => onOpenChange(false)}
    >
      <div
        className="joi-surface--base w-full max-w-lg rounded-lg border p-4 shadow-xl joi-border--default"
        onClick={(event) => event.stopPropagation()}
      >
        <input
          type="text"
          placeholder={t("shell.palette.placeholder")}
          aria-label={t("shell.palette.searchLabel")}
          className="joi-palette__input w-full"
          disabled
        />
        <p className="joi-text--muted mt-3 text-xs">{t("shell.palette.note")}</p>
      </div>
    </div>
  );
}

export default CommandPalette;
