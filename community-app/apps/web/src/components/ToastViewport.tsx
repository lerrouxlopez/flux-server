import { useEffect } from "react";
import { useToastStore } from "../state/toasts";

export function ToastViewport() {
  const toasts = useToastStore((s) => s.toasts);
  const remove = useToastStore((s) => s.remove);

  useEffect(() => {
    if (!toasts.length) return;
    const timers = toasts.map((t) => window.setTimeout(() => remove(t.id), 4500));
    return () => timers.forEach((id) => window.clearTimeout(id));
  }, [remove, toasts]);

  if (!toasts.length) return null;

  return (
    <div className="pointer-events-none fixed right-3 top-3 z-50 space-y-2">
      {toasts.map((t) => (
        <div
          key={t.id}
          className="pointer-events-auto w-[320px] overflow-hidden rounded-xl border border-slate-800 bg-slate-950/90 p-3 shadow-xl backdrop-blur"
          role="status"
        >
          <div className="flex items-start justify-between gap-3">
            <div className="min-w-0">
              <div className="truncate text-sm font-semibold text-slate-100">{t.title}</div>
              {t.message ? <div className="mt-0.5 line-clamp-2 text-xs text-slate-300">{t.message}</div> : null}
            </div>
            <button
              className="rounded-md px-2 py-1 text-xs text-slate-400 hover:bg-slate-900 hover:text-slate-200"
              onClick={() => remove(t.id)}
              type="button"
              aria-label="Dismiss notification"
            >
              ✕
            </button>
          </div>
        </div>
      ))}
    </div>
  );
}

