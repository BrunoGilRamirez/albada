import { useEffect, useState } from "react";
import { Link } from "react-router";
import { useTranslation } from "react-i18next";
import layoutColumnsIcon from "@/assets/layout-icons/layout-columns.svg";
import layoutListIcon from "@/assets/layout-icons/layout-list.svg";
import layoutTilesIcon from "@/assets/layout-icons/layout-tiles.svg";
import PageContainer from "@/components/navigation/PageContainer";
import DeadlineStatusBadge from "@/components/deadlines/DeadlineStatusBadge";
import ProgressBar from "@/components/progress/ProgressBar";
import { getProjectDeadlineStatus } from "@/lib/api/deadlineApi";
import { getProjectProgress } from "@/lib/api/progressApi";
import { listProjects } from "@/lib/api/projectApi";
import { ensureDefaultWorkspace } from "@/lib/api/workspaceApi";
import { toDisplayMessage } from "@/lib/errors";
import type { DeadlineStatusDto, ProgressDetails, Project } from "@/lib/types";

// Presentation-only layout choice. Each project stays one fully clickable card;
// the switcher only changes the grid it sits in. No project data changes meaning
// between modes.
type ProjectListLayout = "list" | "columns" | "tiles";

const LAYOUT_CLASS: Record<ProjectListLayout, string> = {
  list: "grid grid-cols-1 gap-3",
  columns: "grid grid-cols-1 gap-3 md:grid-cols-2",
  tiles: "grid gap-3 [grid-template-columns:repeat(auto-fill,minmax(14rem,1fr))]",
};

const LAYOUT_ORDER: readonly ProjectListLayout[] = ["list", "columns", "tiles"];

const LAYOUT_LABEL_KEY = {
  list: "projects.layoutList",
  columns: "projects.layoutColumns",
  tiles: "projects.layoutTiles",
} as const satisfies Record<ProjectListLayout, `projects.${string}`>;

const LAYOUT_ICON = {
  list: layoutListIcon,
  columns: layoutColumnsIcon,
  tiles: layoutTilesIcon,
} as const satisfies Record<ProjectListLayout, string>;

// Only the selected layout name is persisted. No project, task, journal, or
// therapy content ever reaches storage; the read is re-validated on every start.
const LAYOUT_STORAGE_KEY = "joi-projectlife-project-list-layout";

function isProjectListLayout(value: string | null): value is ProjectListLayout {
  return value !== null && (LAYOUT_ORDER as readonly string[]).includes(value);
}

function readStoredLayout(): ProjectListLayout {
  try {
    const stored = localStorage.getItem(LAYOUT_STORAGE_KEY);
    if (isProjectListLayout(stored)) return stored;
  } catch {
    // Storage unavailable (private mode, blocked storage); keep the default.
  }
  return "list";
}

function storeLayout(layout: ProjectListLayout): void {
  try {
    localStorage.setItem(LAYOUT_STORAGE_KEY, layout);
  } catch {
    // Storage full or unavailable; the switcher still works for this session.
  }
}

function nextProjectListLayout(layout: ProjectListLayout): ProjectListLayout {
  const index = LAYOUT_ORDER.indexOf(layout);
  return LAYOUT_ORDER[(index + 1) % LAYOUT_ORDER.length];
}

function ProjectsPage() {
  const { t } = useTranslation();
  const [projects, setProjects] = useState<Project[]>([]);
  const [progressByProject, setProgressByProject] = useState<Record<string, ProgressDetails>>({});
  const [deadlineByProject, setDeadlineByProject] = useState<Record<string, DeadlineStatusDto>>({});
  const [error, setError] = useState<string | null>(null);
  const [workspaceId, setWorkspaceId] = useState<string | null>(null);
  const [layout, setLayout] = useState<ProjectListLayout>(readStoredLayout);

  useEffect(() => {
    storeLayout(layout);
  }, [layout]);

  useEffect(() => {
    void ensureDefaultWorkspace()
      .then((workspace) => setWorkspaceId(workspace.id))
      .catch((err) => setError(toDisplayMessage(err)));
  }, []);

  useEffect(() => {
    if (!workspaceId) return;
    void listProjects(workspaceId)
      .then(setProjects)
      .catch((err) => setError(toDisplayMessage(err)));
  }, [workspaceId]);

  useEffect(() => {
    const loadStatus = async () => {
      const progressMap: Record<string, ProgressDetails> = {};
      const deadlineMap: Record<string, DeadlineStatusDto> = {};
      await Promise.all(
        projects.map(async (project) => {
          try {
            const [progress, deadline] = await Promise.all([
              getProjectProgress(project.id),
              getProjectDeadlineStatus(project.id),
            ]);
            progressMap[project.id] = progress;
            deadlineMap[project.id] = deadline;
          } catch (err) {
            setError(toDisplayMessage(err));
          }
        }),
      );
      setProgressByProject(progressMap);
      setDeadlineByProject(deadlineMap);
    };
    if (projects.length > 0) {
      void loadStatus();
    }
  }, [projects]);

  // Only the tiles mode pins a card height; list and columns keep the grid row
  // height so a long title or a wrapped badge row grows the card.
  const tileCard = layout === "tiles";

  return (
    <PageContainer>
      <div className="flex items-center justify-between">
        <h1 className="text-2xl font-semibold">{t("projects.title")}</h1>
        <Link to="/projects/new" className="joi-btn joi-btn--primary px-3 py-1.5">
          {t("projects.new")}
        </Link>
      </div>
      <p className="joi-text--muted mt-2">{t("projects.subtitle")}</p>

      {error ? <p className="joi-text--danger mt-2 text-sm">{error}</p> : null}

      {projects.length === 0 ? (
        <p className="joi-empty__state mt-6">{t("projects.empty")}</p>
      ) : (
        <div className="mt-6 flex flex-col gap-3">
          <div role="group" aria-label={t("projects.layoutGroupLabel")} className="flex">
            <button
              type="button"
              onClick={() => setLayout(nextProjectListLayout(layout))}
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
            {projects.map((project) => {
              const progress = progressByProject[project.id];
              const deadline = deadlineByProject[project.id];
              return (
                <li
                  key={project.id}
                  className={`joi-surface--base joi-border--default rounded-lg border${
                    tileCard ? " aspect-square" : ""
                  }`}
                >
                  {/* The whole card is one link: title, progress, and badges
                      are plain spans and divs, never nested links or buttons. */}
                  <Link to={`/projects/${project.id}`} className="block h-full p-3 joi-interactive">
                    <div className="flex h-full flex-col gap-2">
                      <div
                        className={`joi-text text-sm font-medium break-words${
                          tileCard ? " line-clamp-3" : " line-clamp-2"
                        }`}
                      >
                        {project.title}
                      </div>
                      <div>
                        <ProgressBar
                          percentage={progress?.percentage}
                          label={progress?.label}
                          size="sm"
                        />
                      </div>
                      <div
                        className={`flex flex-wrap items-center gap-2${tileCard ? " mt-auto" : ""}`}
                      >
                        {deadline ? (
                          <span className="shrink-0 whitespace-nowrap">
                            <DeadlineStatusBadge
                              status={deadline.status}
                              deadlineAt={project.target_date}
                            />
                          </span>
                        ) : null}
                        <span className="joi-badge joi-badge--neutral shrink-0 whitespace-nowrap">
                          {t(`common.labels.projectStatus.${project.status}`)}
                        </span>
                      </div>
                    </div>
                  </Link>
                </li>
              );
            })}
          </ul>
        </div>
      )}
    </PageContainer>
  );
}

export default ProjectsPage;
