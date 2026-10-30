import { useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import type { Project } from "../../lib/types";

interface AddExistingProjectModalProps {
  open: boolean;
  title: string;
  description?: string;
  projects: Project[];
  onAttach: (projectId: string) => void | Promise<void>;
  onClose: () => void;
}

/**
 * Modal to link an already existing project to a goal. The parent supplies the
 * candidate projects (projects not already linked to the target goal) and the
 * localized title/description for that context.
 */
function AddExistingProjectModal({
  open,
  title,
  description,
  projects,
  onAttach,
  onClose,
}: AddExistingProjectModalProps) {
  const { t } = useTranslation();
  const [query, setQuery] = useState("");
  const [attachingId, setAttachingId] = useState<string | null>(null);

  const filtered = useMemo(() => {
    const q = query.trim().toLowerCase();
    if (!q) return projects;
    return projects.filter((project) => project.title.toLowerCase().includes(q));
  }, [projects, query]);

  if (!open) return null;

  const handleAttach = async (projectId: string) => {
    setAttachingId(projectId);
    try {
      await onAttach(projectId);
    } finally {
      setAttachingId(null);
    }
  };

  return (
    <div
      role="dialog"
      aria-modal="true"
      aria-label={title}
      className="joi-dialog__backdrop z-50"
      onClick={onClose}
    >
      <div
        className="joi-surface--base flex max-h-[80vh] w-full max-w-lg flex-col rounded-xl p-6 shadow-xl"
        onClick={(event) => event.stopPropagation()}
      >
        <div className="flex items-start justify-between gap-4">
          <div>
            <h2 className="joi-dialog__title text-lg font-semibold">{title}</h2>
            {description ? <p className="joi-text--muted mt-1 text-sm">{description}</p> : null}
          </div>
          <button
            type="button"
            onClick={onClose}
            aria-label={t("common.actions.close")}
            className="joi-btn joi-btn--ghost px-2 py-1"
          >
            ✕
          </button>
        </div>

        <input
          type="search"
          aria-label={t("projects.searchLabel")}
          placeholder={t("projects.searchPlaceholder")}
          value={query}
          onChange={(event) => setQuery(event.target.value)}
          className="joi-field__input mt-4"
        />

        <div className="mt-3 flex-1 overflow-y-auto">
          {filtered.length === 0 ? (
            <p className="joi-empty__state">{t("projects.addExistingEmpty")}</p>
          ) : (
            <ul className="flex flex-col gap-2">
              {filtered.map((project) => (
                <li
                  key={project.id}
                  className="flex items-center gap-3 rounded-lg border px-3 py-2 joi-border--default"
                >
                  <span className="joi-text min-w-0 flex-1 truncate text-sm">{project.title}</span>
                  <span className="joi-badge joi-badge--neutral shrink-0">
                    {t(`common.labels.projectStatus.${project.status}`)}
                  </span>
                  <button
                    type="button"
                    disabled={attachingId === project.id}
                    onClick={() => void handleAttach(project.id)}
                    className="joi-btn joi-btn--primary joi-btn--sm shrink-0"
                  >
                    {attachingId === project.id
                      ? t("common.actions.adding")
                      : t("common.actions.add")}
                  </button>
                </li>
              ))}
            </ul>
          )}
        </div>
      </div>
    </div>
  );
}

export default AddExistingProjectModal;
