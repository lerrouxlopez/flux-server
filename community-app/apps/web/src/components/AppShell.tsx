import { Outlet, Link, useLocation } from "react-router-dom";
import { useAuthStore } from "../state/auth";
import { useEffect } from "react";
import { BrandLogo } from "./BrandLogo";
import { OrgRail } from "./OrgRail";
import { Avatar } from "./Avatar";
import { useState } from "react";
import { useNavigate } from "react-router-dom";
import { useQuery } from "@tanstack/react-query";
import { apiFetch } from "../api/client";
import type { MembersResponse, OrgsListResponse } from "../api/types";

export function AppShell() {
  const user = useAuthStore((s) => s.user);
  const logout = useAuthStore((s) => s.logout);
  const loadMe = useAuthStore((s) => s.loadMe);
  const [menuOpen, setMenuOpen] = useState(false);
  const nav = useNavigate();
  const loc = useLocation();

  const currentOrgSlug = (() => {
    const m = loc.pathname.match(/^\/(?:app|admin)\/([^/]+)/);
    return m?.[1] ?? null;
  })();

  const orgs = useQuery({
    enabled: !!user,
    queryKey: ["orgs"],
    queryFn: () => apiFetch<OrgsListResponse>("/orgs"),
    staleTime: 30_000,
  });

  const currentOrg = currentOrgSlug ? orgs.data?.organizations.find((o) => o.slug === currentOrgSlug) ?? null : null;

  const members = useQuery({
    enabled: !!user && !!currentOrg?.id,
    queryKey: ["members", currentOrg?.id],
    queryFn: () => apiFetch<MembersResponse>(`/orgs/${currentOrg!.id}/members`),
    staleTime: 10_000,
  });

  const myRole = members.data?.members.find((m) => m.user_id === user?.id)?.role ?? null;
  const canSeeAdmin = myRole === "owner" || myRole === "admin";

  useEffect(() => {
    // Keep user populated when reloading pages.
    loadMe().catch(() => {});
  }, [loadMe]);

  return (
    <div className="flex min-h-dvh">
      {user ? <OrgRail /> : null}

      <div className="min-w-0 flex-1">
        <header className="border-b border-slate-800 bg-slate-950/80 backdrop-blur">
          <div className="flex items-center justify-between px-4 py-3">
            <Link to="/orgs" className="font-semibold tracking-tight">
              <BrandLogo showText={true} height={26} />
            </Link>

            <div className="flex items-center gap-3">
              {user ? (
                <>
                  <div className="relative">
                    <button
                      className="flex items-center gap-2 rounded-md px-2 py-1 hover:bg-slate-900"
                      onClick={() => setMenuOpen((v) => !v)}
                      type="button"
                    >
                      <Avatar name={user.display_name} size={28} src={user.avatar_url ?? null} />
                      <span className="text-sm text-slate-200">{user.display_name}</span>
                    </button>

                    {menuOpen ? (
                      <div className="absolute right-0 mt-2 w-48 overflow-hidden rounded-xl border border-slate-800 bg-slate-950 shadow-xl">
                        {canSeeAdmin && currentOrgSlug ? (
                          <button
                            className="block w-full px-3 py-2 text-left text-sm text-slate-200 hover:bg-slate-900"
                            onClick={() => {
                              setMenuOpen(false);
                              nav(`/admin/${currentOrgSlug}`);
                            }}
                            type="button"
                          >
                            Admin
                          </button>
                        ) : null}
                        <button
                          className="block w-full px-3 py-2 text-left text-sm text-slate-200 hover:bg-slate-900"
                          onClick={() => {
                            setMenuOpen(false);
                            nav("/profile");
                          }}
                          type="button"
                        >
                          Profile
                        </button>
                        <button
                          className="block w-full px-3 py-2 text-left text-sm text-slate-200 hover:bg-slate-900"
                          onClick={async () => {
                            setMenuOpen(false);
                            await logout();
                            nav("/login");
                          }}
                          type="button"
                        >
                          Logout
                        </button>
                      </div>
                    ) : null}
                  </div>
                </>
              ) : (
                <Link className="text-sm text-slate-300 hover:text-white" to="/login">
                  Login
                </Link>
              )}
            </div>
          </div>
        </header>

        <main className="px-4 py-4">
          <Outlet />
        </main>
      </div>
    </div>
  );
}
