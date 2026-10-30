import { useEffect } from "react";
import { toDisplayMessage } from "../lib/errors";

interface Toast {
  id: number;
  message: string;
  kind: "error" | "info" | "success";
}

interface AppToasterProps {
  toasts: Toast[];
  onDismiss: (id: number) => void;
}

const KIND_STYLES = {
  error: "joi-toast--error",
  info: "joi-toast--info",
  success: "joi-toast--success",
};

/**
 * Toast area for command errors and transient feedback. This component owns
 * no copy of its own: every toast message is supplied by the caller and must
 * already be in the active locale (error text is localized upstream by
 * `toDisplayMessage` through the `errors.*` catalog keys, and page-level
 * feedback through the `settings.*`/`common.*` keys). There is deliberately no
 * literal chrome to translate here.
 */
function AppToaster({ toasts, onDismiss }: AppToasterProps) {
  useEffect(() => {
    const timers = toasts.map((toast) => window.setTimeout(() => onDismiss(toast.id), 6000));
    return () => timers.forEach((timer) => window.clearTimeout(timer));
  }, [toasts, onDismiss]);

  return (
    <div
      aria-live="polite"
      className="pointer-events-none fixed bottom-4 right-4 z-50 flex flex-col gap-2"
    >
      {toasts.map((toast) => (
        <button
          key={toast.id}
          type="button"
          onClick={() => onDismiss(toast.id)}
          className={`joi-toast pointer-events-auto font-medium ${KIND_STYLES[toast.kind]}`}
        >
          {toast.message}
        </button>
      ))}
    </div>
  );
}

export { AppToaster, toDisplayMessage };
export default AppToaster;
