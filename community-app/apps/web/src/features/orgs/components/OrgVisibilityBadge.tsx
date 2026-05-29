import type { JoinPolicy } from "../../../api/types";

export function OrgVisibilityBadge(props: { policy: JoinPolicy }) {
  const { policy } = props;
  const label =
    policy === "open"
      ? "Open"
      : policy === "invite_only"
        ? "Invite-only"
        : policy === "request"
          ? "Request access"
          : "Closed";

  const klass =
    policy === "open"
      ? "flux-status-success"
      : policy === "invite_only"
        ? "flux-status-warning"
        : policy === "request"
          ? "border border-slate-800 bg-slate-950/20 text-slate-200"
          : "flux-status-danger";

  return <span className={`inline-flex items-center rounded-md border px-2 py-0.5 text-xs font-semibold ${klass}`}>{label}</span>;
}

