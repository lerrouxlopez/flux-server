import { useMemo } from "react";
import { Link, useParams, useSearchParams } from "react-router-dom";
import { useQuery } from "@tanstack/react-query";
import { apiFetch } from "../api/client";
import type { ChannelsResponse, MembersResponse, OrgsListResponse } from "../api/types";

export function OrgSearchPage() {
  const { org_slug } = useParams();
  const [sp, setSp] = useSearchParams();
  const q = (sp.get("q") ?? "").trim();

  const orgs = useQuery({
    queryKey: ["orgs"],
    queryFn: () => apiFetch<OrgsListResponse>("/orgs"),
    staleTime: 30_000,
  });

  const org = useMemo(() => (orgs.data?.organizations ?? []).find((o) => o.slug === org_slug) ?? null, [orgs.data, org_slug]);

  const channels = useQuery({
    enabled: !!org?.id,
    queryKey: ["channels", org?.id],
    queryFn: () => apiFetch<ChannelsResponse>(`/orgs/${org!.id}/channels`),
    staleTime: 10_000,
  });

  const members = useQuery({
    enabled: !!org?.id,
    queryKey: ["members", org?.id],
    queryFn: () => apiFetch<MembersResponse>(`/orgs/${org!.id}/members`),
    staleTime: 10_000,
  });

  const qLower = q.toLowerCase();
  const channelResults = useMemo(() => {
    if (!qLower) return [];
    return (channels.data?.channels ?? []).filter((c) => c.name.toLowerCase().includes(qLower));
  }, [channels.data, qLower]);

  const memberResults = useMemo(() => {
    if (!qLower) return [];
    return (members.data?.members ?? []).filter((m) => m.display_name.toLowerCase().includes(qLower) || m.email.toLowerCase().includes(qLower));
  }, [members.data, qLower]);

  if (orgs.isLoading) return <div className="text-slate-300">Loading…</div>;
  if (orgs.isError) return <div className="text-red-400">{(orgs.error as Error).message}</div>;
  if (!org) return <div className="text-slate-300">Org not found.</div>;

  return (
    <div className="mx-auto max-w-4xl">
      <div className="flex flex-wrap items-end justify-between gap-3">
        <div>
          <h1 className="text-xl font-semibold text-slate-100">Search</h1>
          <div className="mt-1 text-xs text-slate-400">
            In <span className="font-semibold text-slate-200">{org.name}</span>
          </div>
        </div>
        <Link className="flux-link text-sm" to={`/app/${org.slug}`}>
          Back to org
        </Link>
      </div>

      <div className="mt-4">
        <label className="sr-only" htmlFor="org-search-q">
          Search query
        </label>
        <input
          id="org-search-q"
          className="w-full rounded-xl border border-slate-800 bg-slate-950/40 px-4 py-2 text-sm text-slate-200 outline-none placeholder:text-slate-500 focus:border-[color:var(--flux-focus-border)]"
          placeholder="Search channels and members…"
          value={q}
          onChange={(e) => {
            const next = new URLSearchParams(sp);
            const v = e.target.value.trim();
            if (v) next.set("q", v);
            else next.delete("q");
            setSp(next, { replace: true });
          }}
        />
      </div>

      <div className="mt-6 grid gap-6 md:grid-cols-2">
        <div className="rounded-xl border border-slate-800 bg-slate-900/30 p-4">
          <div className="text-sm font-semibold text-slate-100">Channels</div>
          {channels.isLoading ? <div className="mt-3 text-sm text-slate-300">Loading…</div> : null}
          {channels.isError ? <div className="mt-3 text-sm text-red-400">{(channels.error as Error).message}</div> : null}
          {!channels.isLoading && q ? (
            channelResults.length ? (
              <div className="mt-3 space-y-1">
                {channelResults.slice(0, 20).map((c) => (
                  <Link key={c.id} className="block rounded-md px-2 py-1.5 text-sm text-slate-200 hover:bg-slate-800/60" to={`/app/${org.slug}/channels/${c.id}`}>
                    #{c.name}
                  </Link>
                ))}
              </div>
            ) : (
              <div className="mt-3 text-sm text-slate-400">No channel matches.</div>
            )
          ) : null}
          {!q ? <div className="mt-3 text-sm text-slate-400">Type to search.</div> : null}
        </div>

        <div className="rounded-xl border border-slate-800 bg-slate-900/30 p-4">
          <div className="text-sm font-semibold text-slate-100">Members</div>
          {members.isLoading ? <div className="mt-3 text-sm text-slate-300">Loading…</div> : null}
          {members.isError ? <div className="mt-3 text-sm text-red-400">{(members.error as Error).message}</div> : null}
          {!members.isLoading && q ? (
            memberResults.length ? (
              <div className="mt-3 space-y-1">
                {memberResults.slice(0, 20).map((m) => (
                  <div key={m.user_id} className="flex items-center justify-between gap-3 rounded-md px-2 py-1.5 hover:bg-slate-800/60">
                    <div className="min-w-0">
                      <div className="truncate text-sm text-slate-200">{m.display_name}</div>
                      <div className="truncate text-xs text-slate-500">{m.email}</div>
                    </div>
                    <div className="shrink-0 text-xs text-slate-500">{m.role}</div>
                  </div>
                ))}
              </div>
            ) : (
              <div className="mt-3 text-sm text-slate-400">No member matches.</div>
            )
          ) : null}
          {!q ? <div className="mt-3 text-sm text-slate-400">Type to search.</div> : null}
        </div>
      </div>
    </div>
  );
}

