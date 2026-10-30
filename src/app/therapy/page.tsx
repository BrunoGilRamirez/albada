import { useEffect, useState } from "react";
import { Link } from "react-router";
import { useTranslation } from "react-i18next";
import layoutColumnsIcon from "@/assets/layout-icons/layout-columns.svg";
import layoutListIcon from "@/assets/layout-icons/layout-list.svg";
import layoutTilesIcon from "@/assets/layout-icons/layout-tiles.svg";
import {
  archiveTherapyCollection,
  createTherapyCollection,
  listTherapyCollections,
  updateTherapyCollection,
} from "@/lib/api/therapyApi";
import { ensureDefaultWorkspace } from "@/lib/api/workspaceApi";
import { toDisplayMessage } from "@/lib/errors";
import type { TherapyCollection } from "@/lib/types";
import TherapyPrivacyBanner from "@/components/therapy/TherapyPrivacyBanner";
import PageContainer from "@/components/navigation/PageContainer";

type TherapyListLayout = "list" | "columns" | "tiles";

const LAYOUT_CLASS: Record<TherapyListLayout, string> = {
  list: "grid grid-cols-1 gap-3",
  columns: "grid grid-cols-1 gap-3 md:grid-cols-2",
  tiles: "grid gap-3 [grid-template-columns:repeat(auto-fill,minmax(14rem,1fr))]",
};

const LAYOUT_ORDER: readonly TherapyListLayout[] = ["list", "columns", "tiles"];

const LAYOUT_LABEL_KEY = {
  list: "therapy.layoutList",
  columns: "therapy.layoutColumns",
  tiles: "therapy.layoutTiles",
} as const satisfies Record<TherapyListLayout, `therapy.${string}`>;

const LAYOUT_ICON = {
  list: layoutListIcon,
  columns: layoutColumnsIcon,
  tiles: layoutTilesIcon,
} as const satisfies Record<TherapyListLayout, string>;

const LAYOUT_STORAGE_KEY = "joi-projectlife-therapy-collection-list-layout";

function isTherapyListLayout(value: string | null): value is TherapyListLayout {
  return value !== null && (LAYOUT_ORDER as readonly string[]).includes(value);
}

function readStoredLayout(): TherapyListLayout {
  try {
    const stored = localStorage.getItem(LAYOUT_STORAGE_KEY);
    if (isTherapyListLayout(stored)) return stored;
  } catch {
    // Storage unavailable; keep the default.
  }
  return "list";
}

function nextTherapyListLayout(layout: TherapyListLayout): TherapyListLayout {
  const index = LAYOUT_ORDER.indexOf(layout);
  return LAYOUT_ORDER[(index + 1) % LAYOUT_ORDER.length];
}

function TherapyPage() {
  const { t } = useTranslation();
  const [collections, setCollections] = useState<TherapyCollection[]>([]);
  const [workspaceId, setWorkspaceId] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [newName, setNewName] = useState("");
  const [creating, setCreating] = useState(false);
  const [editingId, setEditingId] = useState<string | null>(null);
  const [editName, setEditName] = useState("");
  const [confirmArchiveId, setConfirmArchiveId] = useState<string | null>(null);
  const [layout, setLayout] = useState<TherapyListLayout>(readStoredLayout);

  useEffect(() => {
    try {
      localStorage.setItem(LAYOUT_STORAGE_KEY, layout);
    } catch {
      // Storage unavailable; the switcher still works for this session.
    }
  }, [layout]);

  useEffect(() => {
    void ensureDefaultWorkspace()
      .then((workspace) => setWorkspaceId(workspace.id))
      .catch((err) => setError(toDisplayMessage(err)));
  }, []);

  const loadCollections = async () => {
    if (!workspaceId) return;
    try {
      const data = await listTherapyCollections(workspaceId);
      setCollections(data);
    } catch (err) {
      setError(toDisplayMessage(err));
    }
  };

  useEffect(() => {
    void loadCollections();
  }, [workspaceId]);

  const handleCreate = async (event: React.FormEvent) => {
    event.preventDefault();
    if (!workspaceId || !newName.trim()) return;
    setCreating(true);
    setError(null);
    try {
      await createTherapyCollection({
        workspace_id: workspaceId,
        name: newName.trim(),
      });
      setNewName("");
      await loadCollections();
    } catch (err) {
      setError(toDisplayMessage(err));
    } finally {
      setCreating(false);
    }
  };

  const startEdit = (collection: TherapyCollection) => {
    setEditingId(collection.id);
    setEditName(collection.name);
    setConfirmArchiveId(null);
  };

  const cancelEdit = () => {
    setEditingId(null);
    setEditName("");
  };

  const handleUpdate = async (id: string) => {
    if (!editName.trim()) return;
    setError(null);
    try {
      await updateTherapyCollection(id, { name: editName.trim() });
      setEditingId(null);
      await loadCollections();
    } catch (err) {
      setError(toDisplayMessage(err));
    }
  };

  const handleArchive = async (id: string) => {
    if (confirmArchiveId !== id) {
      setConfirmArchiveId(id);
      return;
    }
    setError(null);
    try {
      await archiveTherapyCollection(id);
      setConfirmArchiveId(null);
      await loadCollections();
    } catch (err) {
      setError(toDisplayMessage(err));
    }
  };

  const inputClass = "joi-field__input";

  return (
    <PageContainer>
      <div className="flex items-center justify-between">
        <h1 className="text-2xl font-semibold">{t("therapy.title")}</h1>
        <Link to="/therapy/export" className="joi-btn joi-btn--secondary px-3 py-1.5">
          {t("common.actions.export")}
        </Link>
      </div>
      <div className="mt-3">
        <TherapyPrivacyBanner />
      </div>
      <p className="joi-text--muted mt-2 text-sm">{t("therapy.isolationNote")}</p>
      {error ? <p className="joi-text--danger mt-2 text-sm">{error}</p> : null}

      <form onSubmit={handleCreate} className="mt-6 flex items-end gap-2">
        <div className="w-full max-w-md flex-1">
          <label htmlFor="new-collection-name" className="joi-field__label mb-1 block">
            {t("therapy.newCollection")}
          </label>
          <input
            id="new-collection-name"
            className={inputClass}
            value={newName}
            onChange={(e) => setNewName(e.target.value)}
            placeholder={t("therapy.collectionNamePlaceholder")}
          />
        </div>
        <button
          type="submit"
          disabled={creating || !newName.trim()}
          className="joi-btn joi-btn--primary px-4 py-2"
        >
          {t("common.actions.create")}
        </button>
      </form>

      <h2 className="mt-6 text-lg font-medium">{t("therapy.collections")}</h2>
      {collections.length === 0 ? (
        <p className="joi-empty__state mt-2">{t("therapy.collectionsEmpty")}</p>
      ) : (
        <div className="mt-2 flex flex-col gap-3">
          <div role="group" aria-label={t("therapy.layoutGroupLabel")} className="flex">
            <button
              type="button"
              onClick={() => setLayout(nextTherapyListLayout(layout))}
              aria-label={t(LAYOUT_LABEL_KEY[layout])}
              title={t(LAYOUT_LABEL_KEY[layout])}
              className="joi-btn joi-btn--secondary joi-btn--sm"
            >
              <img
                src={LAYOUT_ICON[layout]}
                alt=""
                aria-hidden="true"
                className="joi-layout-icon"
              />
            </button>
          </div>
          <ul className={LAYOUT_CLASS[layout]}>
            {collections.map((collection) => (
              <li
                key={collection.id}
                className={`joi-surface--base joi-border--default rounded-lg border p-3 ${
                  layout === "tiles"
                    ? "flex aspect-square flex-col gap-3"
                    : "flex items-center justify-between"
                }`}
              >
                {editingId === collection.id ? (
                  <div className="flex flex-1 items-center gap-2">
                    <input
                      className={inputClass}
                      value={editName}
                      onChange={(e) => setEditName(e.target.value)}
                      onKeyDown={(e) => {
                        if (e.key === "Enter") {
                          void handleUpdate(collection.id);
                        } else if (e.key === "Escape") {
                          cancelEdit();
                        }
                      }}
                    />
                    <button
                      type="button"
                      onClick={() => void handleUpdate(collection.id)}
                      className="joi-btn joi-btn--primary px-3 py-1.5"
                    >
                      {t("common.actions.save")}
                    </button>
                    <button
                      type="button"
                      onClick={cancelEdit}
                      className="joi-btn joi-btn--secondary px-3 py-1.5"
                    >
                      {t("common.actions.cancel")}
                    </button>
                  </div>
                ) : (
                  <>
                    {/* The collection name is user-authored and renders verbatim. */}
                    <Link
                      to={`/therapy/collections/${collection.id}`}
                      className="joi-text flex min-w-0 flex-1 self-stretch items-center rounded-md p-1 text-sm font-medium"
                    >
                      {collection.name}
                    </Link>
                    <div className="flex items-center gap-2">
                      <button
                        type="button"
                        onClick={() => startEdit(collection)}
                        className="joi-btn joi-btn--secondary px-3 py-1.5"
                      >
                        {t("common.actions.edit")}
                      </button>
                      <button
                        type="button"
                        onClick={() => void handleArchive(collection.id)}
                        className={`joi-btn px-3 py-1.5 ${
                          confirmArchiveId === collection.id
                            ? "joi-btn--danger"
                            : "joi-btn--secondary"
                        }`}
                      >
                        {confirmArchiveId === collection.id
                          ? t("common.actions.confirmArchive")
                          : t("common.actions.archive")}
                      </button>
                    </div>
                  </>
                )}
              </li>
            ))}
          </ul>
        </div>
      )}
    </PageContainer>
  );
}

export default TherapyPage;
