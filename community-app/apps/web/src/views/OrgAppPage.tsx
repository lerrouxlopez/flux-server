import { useParams, Link } from "react-router-dom";
import { useQuery } from "@tanstack/react-query";
import { apiFetch } from "../api/client";
import type { OrgsListResponse, ChannelsResponse } from "../api/types";

export function OrgAppPage() {
  const { org_slug } = useParams();

  const orgs = useQuery({
    queryKey: ["orgs"],
    queryFn: () => apiFetch<OrgsListResponse>("/orgs"),
  });

  const org = orgs.data?.organizations.find((o) => o.slug === org_slug);

  const channels = useQuery({
    enabled: !!org?.id,
    queryKey: ["channels", org?.id],
    queryFn: () => apiFetch<ChannelsResponse>(`/orgs/${org!.id}/channels`),
  });

  if (orgs.isLoading) return <div className="text-slate-300">Loading…</div>;
  if (!org) return <div className="text-slate-300">Org not found.</div>;

  return (
    <div className="grid gap-6 md:grid-cols-[260px_1fr]">
      <aside className="rounded-xl border border-slate-800 bg-slate-900/30 p-3">
        <div className="px-2 py-2 text-sm font-semibold">{org.name}</div>
        <div className="mt-2 space-y-1">
          {(channels.data?.channels ?? []).map((c) => (
            <Link
              key={c.id}
              to={`/app/${org.slug}/channels/${c.id}`}
              className="block rounded-md px-2 py-1.5 text-sm text-slate-200 hover:bg-slate-800/60"
            >
              # {c.name}
            </Link>
          ))}
        </div>
        <div className="mt-3 border-t border-slate-800 pt-3">
          <Link
            to={`/admin/${org.slug}`}
            className="block rounded-md px-2 py-1.5 text-sm text-slate-300 hover:bg-slate-800/60"
          >
            Admin
          </Link>
        </div>
      </aside>
      <section className="rounded-xl border border-slate-800 bg-slate-900/30 p-4">
        <div className="text-slate-300">Pick a channel.</div>
      </section>
    </div>
  );
}

