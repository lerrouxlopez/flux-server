import { Link, useLocation } from "react-router-dom";
import { useQuery } from "@tanstack/react-query";
import { apiFetch } from "../api/client";
import type { OrgsListResponse } from "../api/types";
import { BrandLogo } from "./BrandLogo";

function orgInitials(name: string): string {
  const v = name.trim();
  if (!v) return "?";
  const parts = v.split(/\s+/).filter(Boolean);
  const a = parts[0]?.[0] ?? "?";
  const b = parts.length > 1 ? parts[parts.length - 1]?.[0] ?? "" : v[1] ?? "";
  return (a + b).toUpperCase();
}

export function OrgRail() {
  const loc = useLocation();
  const orgs = useQuery({
    queryKey: ["orgs"],
    queryFn: () => apiFetch<OrgsListResponse>("/orgs"),
    staleTime: 30_000,
  });

  return (
    <nav className="flex h-dvh w-16 flex-col items-center gap-3 border-r border-slate-800 bg-slate-950/80 py-3">
      <Link
        to="/orgs"
        className="grid h-10 w-10 place-items-center rounded-xl bg-slate-900 text-sm font-semibold text-slate-100 hover:bg-slate-800"
        title="Organizations"
      >
        <BrandLogo showText={false} height={22} />
      </Link>

      <div className="h-px w-10 bg-slate-800" />

      <div className="flex w-full flex-1 flex-col items-center gap-2 overflow-auto px-2">
        {(orgs.data?.organizations ?? []).map((o) => {
          const active = loc.pathname.startsWith(`/app/${o.slug}`);
          return (
            <Link
              key={o.id}
              to={`/app/${o.slug}`}
              title={o.name}
              className={`grid h-10 w-10 place-items-center rounded-xl text-xs font-semibold ${
                active ? "bg-indigo-600 text-white" : "bg-slate-900 text-slate-200 hover:bg-slate-800"
              }`}
            >
              {orgInitials(o.name)}
            </Link>
          );
        })}
      </div>

      <div className="pb-1">
        <Link
          to="/orgs"
          className="grid h-10 w-10 place-items-center rounded-xl bg-slate-900 text-lg text-slate-200 hover:bg-slate-800"
          title="Create / Join org"
        >
          +
        </Link>
      </div>
    </nav>
  );
}
