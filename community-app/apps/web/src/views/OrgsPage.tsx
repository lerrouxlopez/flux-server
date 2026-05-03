import { useQuery } from "@tanstack/react-query";
import { Link } from "react-router-dom";
import { apiFetch } from "../api/client";
import type { OrgsListResponse } from "../api/types";
import { useAuthStore } from "../state/auth";

export function OrgsPage() {
  useAuthStore((s) => s.hydrate)();

  const q = useQuery({
    queryKey: ["orgs"],
    queryFn: () => apiFetch<OrgsListResponse>("/orgs"),
  });

  if (q.isLoading) return <div className="text-slate-300">Loading orgs…</div>;
  if (q.isError) return <div className="text-red-400">{(q.error as Error).message}</div>;
  if (!q.data) return <div className="text-slate-300">No data.</div>;

  return (
    <div>
      <h1 className="text-xl font-semibold">Organizations</h1>
      <div className="mt-4 grid gap-3 sm:grid-cols-2">
        {q.data.organizations.map((o) => (
          <Link
            key={o.id}
            to={`/app/${o.slug}`}
            className="rounded-xl border border-slate-800 bg-slate-900/40 p-4 hover:border-slate-700"
          >
            <div className="font-medium">{o.name}</div>
            <div className="mt-1 text-sm text-slate-400">/{o.slug}</div>
          </Link>
        ))}
      </div>
    </div>
  );
}
