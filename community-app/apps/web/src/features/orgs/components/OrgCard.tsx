import { Link } from "react-router-dom";
import type { DiscoverOrg } from "../../../api/types";
import { Button } from "../../../components/Button";
import { OrgVisibilityBadge } from "./OrgVisibilityBadge";

function orgInitials(name: string): string {
  const v = name.trim();
  if (!v) return "?";
  const parts = v.split(/\s+/).filter(Boolean);
  const a = parts[0]?.[0] ?? "?";
  const b = parts.length > 1 ? parts[parts.length - 1]?.[0] ?? "" : v[1] ?? "";
  return (a + b).toUpperCase();
}

export type OrgCardAction =
  | { kind: "open"; href: string }
  | { kind: "join_open"; orgId: string }
  | { kind: "join_invite"; slug: string }
  | { kind: "request_access"; orgId: string; name: string }
  | { kind: "pending" };

function actionForOrg(org: DiscoverOrg): OrgCardAction {
  if (org.current_user_status === "member") return { kind: "open", href: `/app/${org.slug}` };
  if (org.current_user_status === "pending_request") return { kind: "pending" };

  if (org.join_policy === "open") return { kind: "join_open", orgId: org.id };
  if (org.join_policy === "invite_only") return { kind: "join_invite", slug: org.slug };
  if (org.join_policy === "request") return { kind: "request_access", orgId: org.id, name: org.name };
  return { kind: "pending" };
}

export function OrgCard(props: {
  org: DiscoverOrg;
  density: "comfortable" | "compact";
  onJoinOpen: (orgId: string) => void;
  onJoinByInvite: (slug: string) => void;
  onRequestAccess: (orgId: string, name: string) => void;
}) {
  const { org } = props;
  const action = actionForOrg(org);

  const bodyText = (org.description ?? "").trim();
  const showMeta = props.density === "comfortable";
  const showDetails = !!bodyText;

  return (
    <div className="rounded-xl border border-slate-800 bg-slate-900/30 p-4">
      <div className="flex items-start justify-between gap-3">
        <div className="flex min-w-0 gap-3">
          <div className="shrink-0">
            {org.avatar_url ? (
              <img
                alt={`${org.name} logo`}
                className="h-10 w-10 rounded-lg border border-slate-800 bg-slate-950/30 object-cover"
                src={org.avatar_url}
              />
            ) : (
              <div className="grid h-10 w-10 place-items-center rounded-lg border border-slate-800 bg-slate-950/30 text-xs font-semibold text-slate-200">
                {orgInitials(org.name)}
              </div>
            )}
          </div>
          <div className="min-w-0">
            <div className="flex flex-wrap items-center gap-2">
              <div className="truncate text-sm font-semibold text-slate-100">{org.name}</div>
              <OrgVisibilityBadge policy={org.join_policy} />
              {org.current_user_status === "rejected" ? (
                <span className="inline-flex items-center rounded-md border border-slate-800 bg-slate-950/20 px-2 py-0.5 text-xs font-semibold text-slate-200">
                  Rejected
                </span>
              ) : null}
            </div>
            <div className="mt-0.5 text-xs text-slate-400">/{org.slug}</div>
            {showDetails ? (
              <div className={`mt-2 text-sm text-slate-300 ${showMeta ? "line-clamp-2" : "line-clamp-1"}`}>{bodyText}</div>
            ) : null}
          </div>
        </div>

        <div className="shrink-0">
          {action.kind === "open" ? (
            <Link className="flux-link text-sm" to={action.href}>
              Open
            </Link>
          ) : action.kind === "pending" ? (
            <span className="text-xs font-semibold text-slate-400">Pending</span>
          ) : action.kind === "join_open" ? (
            <Button className="flux-btn-primary px-3 py-2 text-sm" onClick={() => props.onJoinOpen(action.orgId)} type="button">
              Join
            </Button>
          ) : action.kind === "join_invite" ? (
            <Button className="bg-slate-800 px-3 py-2 text-sm hover:bg-slate-700" onClick={() => props.onJoinByInvite(action.slug)} type="button">
              Enter code
            </Button>
          ) : (
            <Button className="bg-slate-800 px-3 py-2 text-sm hover:bg-slate-700" onClick={() => props.onRequestAccess(action.orgId, action.name)} type="button">
              Request
            </Button>
          )}
        </div>
      </div>

      {showMeta ? (
        <div className="mt-3 flex flex-wrap items-center gap-3 text-xs text-slate-500">
          {typeof org.member_count === "number" ? <span>{org.member_count} members</span> : null}
          {typeof org.online_count === "number" ? <span>{org.online_count} online</span> : null}
          {org.category ? <span className="truncate">Category: {org.category}</span> : null}
          {(org.tags ?? []).length ? <span className="truncate">Tags: {org.tags.slice(0, 3).join(", ")}</span> : null}
        </div>
      ) : null}
    </div>
  );
}
