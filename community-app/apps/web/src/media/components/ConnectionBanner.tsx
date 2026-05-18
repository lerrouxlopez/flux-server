import type { MediaConnectStatus } from "../types";

export function ConnectionBanner(props: {
  status: MediaConnectStatus;
  error?: string | null;
  onRetry?: () => void;
  onEnd?: () => void;
}) {
  const tone =
    props.status === "failed"
      ? "flux-status-danger"
      : props.status === "reconnecting" || props.status === "connecting"
        ? "flux-status-warning"
        : props.status === "ended"
          ? "border-slate-800 bg-slate-950/30 text-slate-200"
          : "flux-status-success";

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
