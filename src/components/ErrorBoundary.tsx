import { Component, useEffect, type ErrorInfo, type ReactNode } from "react";
import { useTranslation } from "react-i18next";
import { useLocation, useRouteError } from "react-router";
import { toDisplayMessage } from "../lib/errors";
import { reportUnknownError } from "../lib/tauri";

interface ErrorBoundaryProps {
  children: ReactNode;
}

interface ErrorBoundaryState {
  hasError: boolean;
  error: unknown;
}

/**
 * Fallback chrome for an unrecoverable render failure. Kept as a function
 * component so the localized strings (and the localized error-code message
 * from `toDisplayMessage`) re-render when the active locale changes; the
 * class above only owns the error lifecycle and reporting.
 */
export function ErrorFallback({ error }: { error: unknown }) {
  const { t } = useTranslation();
  return (
    <main
      role="alert"
      className="joi-surface--canvas box-border flex min-h-screen items-center justify-center p-6"
    >
      <section
        aria-labelledby="error-title"
        className="joi-surface--base box-border w-full max-w-lg rounded-lg border joi-border--default p-6 text-center"
      >
        {/* Product name stays verbatim; only the surrounding chrome translates. */}
        <p className="mb-2 joi-text--muted">Albada</p>
        <h1 id="error-title" className="mb-4 joi-text">
          {t("common.feedback.errorTitle")}
        </h1>
        <p className="mb-6 leading-relaxed joi-text--muted">{toDisplayMessage(error)}</p>
        <button
          type="button"
          onClick={() => window.location.reload()}
          className="joi-btn joi-btn--primary"
        >
          {t("common.feedback.reload")}
        </button>
      </section>
    </main>
  );
}

/**
 * React Router's boundary for route matching, lazy-route, loader, and action
 * failures. The outer ErrorBoundary still covers failures outside the router.
 */
export function RouteErrorElement() {
  const error = useRouteError();
  const location = useLocation();

  useEffect(() => {
    reportUnknownError("route_error", error, `Route: ${location.pathname}`);
  }, [error, location.pathname]);

  return <ErrorFallback error={error} />;
}

export class ErrorBoundary extends Component<ErrorBoundaryProps, ErrorBoundaryState> {
  state: ErrorBoundaryState = {
    hasError: false,
    error: undefined,
  };

  static getDerivedStateFromError(error: unknown): ErrorBoundaryState {
    return {
      hasError: true,
      error,
    };
  }

  componentDidCatch(error: Error, errorInfo: ErrorInfo): void {
    console.error("Unhandled React render error", error, errorInfo);
    // Mirror the render failure (with the component stack) to the native log
    // through the typed boundary. Fire-and-forget and recursion-safe: a
    // failing report channel must never throw inside the error path.
    reportUnknownError("error_boundary", error, errorInfo.componentStack);
  }

  render(): ReactNode {
    if (!this.state.hasError) {
      return this.props.children;
    }

    return <ErrorFallback error={this.state.error} />;
  }
}

export default ErrorBoundary;
