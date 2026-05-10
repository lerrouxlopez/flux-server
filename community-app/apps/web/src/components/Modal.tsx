import type { ReactNode } from "react";

export function Modal(props: {
  open: boolean;
  title?: string;
  children: ReactNode;
  onClose: () => void;
}) {
  if (!props.open) return null;

  return (
    <div className="fixed inset-0 z-50">
      <button
        aria-label="Close modal"
        className="absolute inset-0 bg-black/60"
        onClick={props.onClose}
        type="button"
      />
      <div className="relative mx-auto mt-20 w-[92vw] max-w-lg rounded-xl border border-slate-800 bg-slate-950 p-4 shadow-2xl">
        <div className="flex items-center justify-between gap-4">
          <div className="text-sm font-semibold text-slate-100">{props.title ?? ""}</div>
          <button
            className="rounded-md px-2 py-1 text-sm text-slate-400 hover:bg-slate-800/60 hover:text-slate-200"
            onClick={props.onClose}
            type="button"
          >
            ✕
          </button>
        </div>
        <div className="mt-3">{props.children}</div>
      </div>
    </div>
  );
}

