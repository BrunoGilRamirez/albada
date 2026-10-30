import { useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import type { Task } from "../../lib/types";

interface AddExistingTaskModalProps {
  open: boolean;
  title: string;
  description?: string;
  tasks: Task[];
  onAttach: (taskId: string) => void | Promise<void>;
  onClose: () => void;
}

/**
 * Modal to attach an already existing task to a project or goal. The parent
 * supplies only the eligible candidates (leaf root tasks not already in the
 * target scope) and the localized title/description for that context.
 */
function AddExistingTaskModal({
  open,
  title,
  description,
  tasks,
  onAttach,
  onClose,
}: AddExistingTaskModalProps) {
  const { t } = useTranslation();
  const [query, setQuery] = useState("");
  const [attachingId, setAttachingId] = useState<string | null>(null);

  const filtered = useMemo(() => {
    const q = query.trim().toLowerCase();
    if (!q) return tasks;
    return tasks.filter((task) => task.title.toLowerCase().includes(q));
  }, [tasks, query]);

  if (!open) return null;

  const handleAttach = async (taskId: string) => {
    setAttachingId(taskId);
    try {
      await onAttach(taskId);
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
            className="joi-btn joi-btn--ghost"
          >
            ✕
          </button>
        </div>

        <input
          type="search"
          aria-label={t("tasks.searchLabel")}
          placeholder={t("tasks.searchPlaceholder")}
          value={query}
          onChange={(event) => setQuery(event.target.value)}
          className="joi-field__input mt-4"
        />

        <div className="mt-3 flex-1 overflow-y-auto">
          {filtered.length === 0 ? (
            <p className="joi-empty__state">{t("tasks.addExistingEmpty")}</p>
          ) : (
            <ul className="flex flex-col gap-2">
              {filtered.map((task) => (
                <li
                  key={task.id}
                  className="flex items-center gap-3 rounded-lg border px-3 py-2 joi-border--default"
                >
                  <span className="joi-text min-w-0 flex-1 truncate text-sm">{task.title}</span>
                  <span className="joi-badge joi-badge--neutral shrink-0">
                    {t(`common.labels.taskStatus.${task.status}`)}
                  </span>
                  <button
                    type="button"
                    disabled={attachingId === task.id}
                    onClick={() => void handleAttach(task.id)}
                    className="joi-btn joi-btn--primary joi-btn--sm shrink-0"
                  >
                    {attachingId === task.id ? t("common.actions.adding") : t("common.actions.add")}
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

export default AddExistingTaskModal;
