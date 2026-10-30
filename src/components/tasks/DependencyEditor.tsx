import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import {
  addDependency,
  listAllPrerequisites,
  listTasks,
  removeDependency,
} from "../../lib/api/taskApi";
import type { PrerequisiteTask, Task } from "../../lib/types";
import { toDisplayMessage } from "../../lib/errors";

interface DependencyEditorProps {
  workspaceId: string;
  taskId: string;
}

function DependencyEditor({ workspaceId, taskId }: DependencyEditorProps) {
  const { t } = useTranslation();
  const [prerequisites, setPrerequisites] = useState<PrerequisiteTask[]>([]);
  const [candidates, setCandidates] = useState<Task[]>([]);
  const [selected, setSelected] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);

  const reload = async () => {
    setError(null);
    try {
      const [blocking, all] = await Promise.all([
        listAllPrerequisites(taskId),
        listTasks(workspaceId),
      ]);
      setPrerequisites(blocking);
      const prerequisiteIds = new Set(blocking.map((p) => p.task_id));
      setCandidates(all.filter((t) => t.id !== taskId && !prerequisiteIds.has(t.id)));
      setSelected("");
    } catch (err) {
      setError(toDisplayMessage(err));
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    void reload();
  }, [taskId, workspaceId]);

  const handleAdd = async () => {
    if (!selected) return;
    setError(null);
    try {
      await addDependency({ blocked_task_id: taskId, prerequisite_task_id: selected });
      setSelected("");
      await reload();
    } catch (err) {
      setError(toDisplayMessage(err));
    }
  };

  const handleRemove = async (dependencyId: string) => {
    setError(null);
    try {
      await removeDependency(dependencyId);
      await reload();
    } catch (err) {
      setError(toDisplayMessage(err));
    }
  };

  if (loading) {
    return <p className="joi-empty__loading">{t("tasks.dependenciesLoading")}</p>;
  }

  return (
    <div className="flex flex-col gap-2">
      <div className="flex gap-2">
        <select
          aria-label={t("tasks.addPrerequisite")}
          value={selected}
          onChange={(e) => setSelected(e.target.value)}
          className="joi-field__select"
        >
          <option value="">{t("tasks.selectPrerequisite")}</option>
          {candidates.map((candidate) => (
            <option key={candidate.id} value={candidate.id}>
              {candidate.title}
            </option>
          ))}
        </select>
        <button
          type="button"
          onClick={handleAdd}
          disabled={!selected}
          className="joi-btn joi-btn--primary px-3 py-2"
        >
          {t("common.actions.add")}
        </button>
      </div>

      {error ? <p className="joi-text--danger text-sm">{error}</p> : null}

      <ul className="flex flex-col gap-1">
        {prerequisites.map((prereq) => (
          <li
            key={prereq.dependency_id}
            className="joi-surface--raised flex items-center justify-between rounded-md px-3 py-2 text-sm"
          >
            <div className="flex min-w-0 items-center gap-2">
              <span className="joi-text truncate">{prereq.title}</span>
              {prereq.status === "completed" ? (
                <span className="joi-badge joi-badge--success shrink-0">
                  {t("common.labels.taskStatus.completed")}
                </span>
              ) : null}
            </div>
            <button
              type="button"
              onClick={() => handleRemove(prereq.dependency_id)}
              className="joi-text--danger text-xs font-medium hover:underline"
            >
              {t("common.actions.remove")}
            </button>
          </li>
        ))}
        {prerequisites.length === 0 ? (
          <li className="joi-empty__state">{t("tasks.noPrerequisites")}</li>
        ) : null}
      </ul>
    </div>
  );
}

export default DependencyEditor;
