import { useEffect, useId, type ReactNode } from "react";

export function Modal(props: {
  open: boolean;
  title?: string;
  children: ReactNode;
  onClose: () => void;
}) {
  const titleId = useId();

  useEffect(() => {
    if (!props.open) return;
    function onKeyDown(e: KeyboardEvent) {
      if (e.key === "Escape") props.onClose();
    }
    document.addEventListener("keydown", onKeyDown);
    return () => document.removeEventListener("keydown", onKeyDown);
  }, [props.open, props.onClose]);

  if (!props.open) return null;

  return (
    <div className="fixed inset-0 z-50">
      <button
        aria-label="Close modal"
        className="absolute inset-0 bg-black/60"
        onClick={props.onClose}
        type="button"
      />
      <div
        aria-labelledby={props.title ? titleId : undefined}
        aria-modal="true"
        aria-label={props.title ? undefined : "Modal dialog"}
        role="dialog"
        className="relative mx-auto mt-20 w-[92vw] max-w-lg rounded-xl border border-slate-800 bg-slate-950 p-4 shadow-2xl"
      >
        <div className="flex items-center justify-between gap-4">
          <div id={titleId} className="text-sm font-semibold text-slate-100">
            {props.title ?? ""}
          </div>
          <button
            aria-label="Close"
            className="rounded-md px-2 py-1 text-sm text-slate-400 hover:bg-slate-800/60 hover:text-slate-200"
            onClick={props.onClose}
            type="button"
          >
            ×
          </button>
        </div>
        <div className="mt-3">{props.children}</div>
      </div>
    </div>
  );
}

