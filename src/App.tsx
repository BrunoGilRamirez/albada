import { Suspense, lazy, type ComponentType } from "react";
import { RouterProvider, createBrowserRouter } from "react-router";
import Layout from "./app/layout";
import { RouteErrorElement } from "./components/ErrorBoundary";
import PageSkeleton from "./components/navigation/PageSkeleton";

// Start fetching the first route immediately while keeping it out of the
// initial bundle. This preserves the fast first screen without inflating the
// shared chunk used by every route.
const todayRoute = import("./app/today/page");

/**
 * Suspense fallback shown in the content area while a lazily loaded route
 * chunk is fetched. The shared skeleton keeps the first route transition
 * visually stable instead of flashing a plain loading label.
 */
function RouteFallback() {
  return <PageSkeleton />;
}

/**
 * Route-level code split: each page module becomes its own Vite chunk, fetched
 * on first visit, so the main chunk stays under the 500 kB build warning.
 * The boundary sits inside the route element (not above RouterProvider) so
 * the eager Layout shell keeps its state and never flashes.
 */
function lazyRoute(loader: () => Promise<{ default: ComponentType }>) {
  const RouteComponent = lazy(loader);
  return (
    <Suspense fallback={<RouteFallback />}>
      <RouteComponent />
    </Suspense>
  );
}

const router = createBrowserRouter([
  {
    path: "/",
    element: <Layout />,
    errorElement: <RouteErrorElement />,
    children: [
      { index: true, element: lazyRoute(() => todayRoute) },
      { path: "analytics", element: lazyRoute(() => import("./app/analytics/page")) },
      { path: "projects", element: lazyRoute(() => import("./app/projects/page")) },
      { path: "projects/new", element: lazyRoute(() => import("./app/projects/new/page")) },
      { path: "projects/:id", element: lazyRoute(() => import("./app/projects/[id]/page")) },
      {
        path: "projects/:id/tasks/new",
        element: lazyRoute(() => import("./app/projects/[id]/tasks/new/page")),
      },
      {
        path: "projects/:id/edit",
        element: lazyRoute(() => import("./app/projects/[id]/edit/page")),
      },
      { path: "goals", element: lazyRoute(() => import("./app/goals/page")) },
      { path: "goals/new", element: lazyRoute(() => import("./app/goals/new/page")) },
      { path: "goals/:id", element: lazyRoute(() => import("./app/goals/[id]/page")) },
      { path: "goals/:id/edit", element: lazyRoute(() => import("./app/goals/[id]/edit/page")) },
      { path: "tasks", element: lazyRoute(() => import("./app/tasks/page")) },
      { path: "tasks/new", element: lazyRoute(() => import("./app/tasks/new/page")) },
      { path: "tasks/:id", element: lazyRoute(() => import("./app/tasks/[id]/page")) },
      { path: "tasks/:id/edit", element: lazyRoute(() => import("./app/tasks/[id]/edit/page")) },
      {
        path: "tasks/:id/children/new",
        element: lazyRoute(() => import("./app/tasks/[id]/children/new/page")),
      },
      { path: "journal", element: lazyRoute(() => import("./app/journal/page")) },
      { path: "journal/new", element: lazyRoute(() => import("./app/journal/new/page")) },
      { path: "journal/:id", element: lazyRoute(() => import("./app/journal/[id]/page")) },
      {
        path: "journal/:id/edit",
        element: lazyRoute(() => import("./app/journal/[id]/edit/page")),
      },
      { path: "journal/export", element: lazyRoute(() => import("./app/journal/export/page")) },
      { path: "therapy", element: lazyRoute(() => import("./app/therapy/page")) },
      {
        path: "therapy/collections/:id",
        element: lazyRoute(() => import("./app/therapy/collections/[id]/page")),
      },
      {
        path: "therapy/collections/:id/records/new",
        element: lazyRoute(() => import("./app/therapy/collections/[id]/records/new/page")),
      },
      {
        path: "therapy/records/:id",
        element: lazyRoute(() => import("./app/therapy/records/[id]/page")),
      },
      {
        path: "therapy/records/:id/edit",
        element: lazyRoute(() => import("./app/therapy/records/[id]/edit/page")),
      },
      { path: "therapy/export", element: lazyRoute(() => import("./app/therapy/export/page")) },
      { path: "calendar", element: lazyRoute(() => import("./app/calendar/page")) },
      { path: "settings", element: lazyRoute(() => import("./app/settings/page")) },
      {
        path: "settings/commitments",
        element: lazyRoute(() => import("./app/settings/commitments/page")),
      },
      {
        path: "settings/availability",
        element: lazyRoute(() => import("./app/settings/availability/page")),
      },
      { path: "settings/import", element: lazyRoute(() => import("./app/settings/import/page")) },
      {
        path: "settings/notifications",
        element: lazyRoute(() => import("./app/settings/notifications/page")),
      },
    ],
  },
]);

function App() {
  return <RouterProvider router={router} />;
}

export default App;
