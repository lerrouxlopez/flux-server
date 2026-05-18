import type { MediaConnectStatus } from "../types";

export function ConnectionBanner(props: {
  status: MediaConnectStatus;
  error?: string | null;
  onRetry?: () => void;
  onEnd?: () => void;
}) {
  const tone =
    props.status === "failed"
      ? "border-red-900/40 bg-red-950/30 text-red-200"
      : props.status === "reconnecting" || props.status === "connecting"
        ? "border-amber-900/40 bg-amber-950/20 text-amber-200"
        : props.status === "ended"
          ? "border-slate-800 bg-slate-950/30 text-slate-200"
          : "border-emerald-900/30 bg-emerald-950/10 text-emerald-200";

  const label =
    props.status === "connecting"
      ? "Connecting…"
      : props.status === "connected"
        ? "Connected"
        : props.status === "reconnecting"
          ? "Reconnecting…"
          : props.status === "failed"
            ? "Connection failed"
            : "Ended";

  return (
    <div className={`rounded-lg border px-3 py-2 text-xs ${tone}`}>
      <div className="flex items-start justify-between gap-2">
        <div className="min-w-0">
          <div className="font-semibold">{label}</div>
          {props.error ? <div className="mt-1 text-slate-200/80">{props.error}</div> : null}
        </div>
        <div className="flex shrink-0 items-center gap-2">
          {props.status === "failed" && props.onRetry ? (
            <button
              className="rounded-md border border-slate-800 bg-slate-900 px-2 py-1 text-[11px] text-slate-200 hover:bg-slate-800/60"
              onClick={props.onRetry}
              type="button"
            >
              Retry
            </button>
          ) : null}
          {props.status !== "ended" && props.onEnd ? (
            <button
              className="rounded-md border border-slate-800 bg-slate-900 px-2 py-1 text-[11px] text-slate-200 hover:bg-slate-800/60"
              onClick={props.onEnd}
              type="button"
            >
              End
            </button>
          ) : null}
        </div>
      </div>
    </div>
  );
}

