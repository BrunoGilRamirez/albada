import { useCallback, useEffect, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import MarkdownPreview from "../markdown/MarkdownPreview";
import {
  addTherapyReference,
  listReferencesForRecord,
  removeTherapyReference,
} from "../../lib/api/therapyReferenceApi";
import { listTasks } from "../../lib/api/taskApi";
import { toDisplayMessage } from "../../lib/errors";
import type { Task, TherapyReferenceRelation, TherapyTaskReference } from "../../lib/types";
import { THERAPY_REFERENCE_RELATIONS } from "../../lib/types";

interface TherapyReferenceEditorProps {
  recordId: string;
  workspaceId: string;
}

function TherapyReferenceEditor({ recordId, workspaceId }: TherapyReferenceEditorProps) {
  const { t } = useTranslation();
  const [references, setReferences] = useState<TherapyTaskReference[]>([]);
  const [tasks, setTasks] = useState<Task[]>([]);
  const [taskId, setTaskId] = useState("");
  const [relation, setRelation] = useState<TherapyReferenceRelation>("related_to");
  const [noteMarkdown, setNoteMarkdown] = useState("");
  const [loading, setLoading] = useState(false);
  const [tasksLoading, setTasksLoading] = useState(false);
  const [tasksFailed, setTasksFailed] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [showForm, setShowForm] = useState(false);

  // A record without its workspace cannot offer linkable tasks; nothing is
  // invoked in that state.
  const missingContext = !recordId || !workspaceId;

  const tasksById = useMemo(() => new Map(tasks.map((task) => [task.id, task])), [tasks]);

  const reloadReferences = useCallback(async () => {
    if (!recordId || !workspaceId) return;
    try {
      setReferences(await listReferencesForRecord(recordId));
    } catch (err) {
      setError(toDisplayMessage(err));
    }
  }, [recordId, workspaceId]);

  // The linked references and the candidate tasks are both plain local reads
  // and load together so the selector and the list never disagree about what
  // is linkable. Only display-safe text reaches the screen; no record or note
  // content is echoed into the error state.
  useEffect(() => {
    if (!recordId || !workspaceId) return;
    let active = true;
    setError(null);
    setTasksFailed(false);
    setTasksLoading(true);
    void Promise.all([listReferencesForRecord(recordId), listTasks(workspaceId)])
      .then(([loadedReferences, loadedTasks]) => {
        if (!active) return;
        setReferences(loadedReferences);
        setTasks(loadedTasks);
      })
      .catch((err: unknown) => {
        if (!active) return;
        setError(toDisplayMessage(err));
        setTasksFailed(true);
      })
      .finally(() => {
        if (active) setTasksLoading(false);
      });
    return () => {
      active = false;
    };
  }, [recordId, workspaceId]);

  const handleAdd = async (event: React.FormEvent) => {
    event.preventDefault();
    if (missingContext) {
      setError(t("errors.VALIDATION"));
      return;
    }
    const selectedTaskId = taskId.trim();
    if (!selectedTaskId) {
      setError(t("therapy.reference.needTaskId"));
      return;
    }
    setLoading(true);
    setError(null);
    try {
      await addTherapyReference(recordId, selectedTaskId, relation, noteMarkdown.trim() || null);
      setTaskId("");
      setRelation("related_to");
      setNoteMarkdown("");
      await reloadReferences();
    } catch (err) {
      setError(toDisplayMessage(err));
    } finally {
      setLoading(false);
    }
  };

  const handleRemove = async (id: string) => {
    try {
      await removeTherapyReference(id);
      await reloadReferences();
    } catch (err) {
      setError(toDisplayMessage(err));
    }
  };

  let taskHint: string | null = null;
  if (tasksLoading) {
    taskHint = t("therapy.reference.taskLoading");
  } else if (tasksFailed) {
    taskHint = t("therapy.reference.taskLoadError");
  } else if (tasks.length === 0) {
    taskHint = t("therapy.reference.taskEmpty");
  }

  const selectClass = "joi-field__select";
  const labelClass = "joi-field__label mb-1 block";

  return (
    <div className="flex flex-col gap-4">
      <h2 className="text-lg font-medium">{t("therapy.linkedTasks")}</h2>
      {missingContext ? <p className="joi-text--danger text-sm">{t("errors.VALIDATION")}</p> : null}
      {error ? <p className="joi-text--danger text-sm">{error}</p> : null}

      {missingContext ? null : (
        <>
          <button
            type="button"
            onClick={() => setShowForm((open) => !open)}
            aria-expanded={showForm}
            aria-controls="therapy-reference-form"
            className="joi-btn joi-btn--secondary self-start px-3 py-1.5"
          >
            {t(showForm ? "common.actions.cancel" : "therapy.reference.add")}
          </button>

          {showForm ? (
            <form
              id="therapy-reference-form"
              onSubmit={handleAdd}
              className="joi-surface--base joi-border--default flex flex-col gap-3 rounded-lg border p-3"
            >
              <div>
                <label htmlFor="reference-task-id" className={labelClass}>
                  {t("therapy.reference.taskId")}
                </label>
                <select
                  id="reference-task-id"
                  className={selectClass}
                  value={taskId}
                  onChange={(e) => setTaskId(e.target.value)}
                  disabled={tasksLoading}
                >
                  <option value="" disabled>
                    {t("therapy.reference.taskIdPlaceholder")}
                  </option>
                  {tasks.map((task) => (
                    <option key={task.id} value={task.id}>
                      {`${task.title} (${t(`common.labels.taskStatus.${task.status}`)})`}
                    </option>
                  ))}
                </select>
                {taskHint ? <p className="joi-text--muted mt-1 text-xs">{taskHint}</p> : null}
              </div>
              <div>
                <label htmlFor="reference-relation" className={labelClass}>
                  {t("therapy.reference.relation")}
                </label>
                <select
                  id="reference-relation"
                  className={selectClass}
                  value={relation}
                  onChange={(e) => setRelation(e.target.value as TherapyReferenceRelation)}
                >
                  {THERAPY_REFERENCE_RELATIONS.map((value) => (
                    <option key={value} value={value}>
                      {t(`therapy.reference.relations.${value}`)}
                    </option>
                  ))}
                </select>
              </div>
              <div>
                <label htmlFor="reference-note" className={labelClass}>
                  {t("therapy.reference.note")}
                </label>
                <textarea
                  id="reference-note"
                  className="joi-field__textarea"
                  rows={3}
                  value={noteMarkdown}
                  onChange={(e) => setNoteMarkdown(e.target.value)}
                />
              </div>
              <button
                type="submit"
                disabled={loading}
                className="joi-btn joi-btn--primary px-3 py-1.5"
              >
                {t("therapy.reference.add")}
              </button>
            </form>
          ) : null}

          {references.length === 0 ? (
            <p className="joi-empty__state">{t("therapy.reference.empty")}</p>
          ) : (
            <ul className="flex flex-col gap-2">
              {references.map((reference) => {
                const linkedTaskId = reference.task_id;
                const linkedTask = linkedTaskId ? tasksById.get(linkedTaskId) : undefined;
                return (
                  <li
                    key={reference.id}
                    className="joi-surface--base joi-border--default flex items-start justify-between rounded-lg border p-3"
                  >
                    <div>
                      {/* Task titles are user-authored and render verbatim; an
                          unresolvable or absent task_id falls back to the
                          opaque stored identifier exactly as it is. */}
                      <p className="joi-text text-sm font-medium">
                        {linkedTask ? linkedTask.title : reference.task_id}
                      </p>
                      <p className="joi-text--muted text-xs">
                        {t(`therapy.reference.relations.${reference.relation}`)}
                      </p>
                      {reference.note_markdown ? (
                        <div className="joi-text mt-1 text-sm">
                          <MarkdownPreview
                            content={reference.note_markdown}
                            emptyText={t("common.markdown.empty")}
                          />
                        </div>
                      ) : null}
                    </div>
                    <button
                      type="button"
                      onClick={() => void handleRemove(reference.id)}
                      className="joi-btn joi-btn--danger joi-btn--sm"
                    >
                      {t("common.actions.remove")}
                    </button>
                  </li>
                );
              })}
            </ul>
          )}
        </>
      )}
    </div>
  );
}

export default TherapyReferenceEditor;
