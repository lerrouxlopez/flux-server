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
import { ExperienceProvider } from "../features/experience/ExperienceProvider";
import { useExperience } from "../features/experience/useExperience";
import { ToastViewport } from "./ToastViewport";
import { useRef } from "react";
import type { CSSProperties } from "react";
import { useUserThemeStore } from "../state/userTheme";
import { getThemePreset } from "../branding/presets";

function HeaderExtras(props: { orgSlug: string | null }) {
  const nav = useNavigate();
  const experience = useExperience();
  const [q, setQ] = useState("");

  if (props.orgSlug) {
    return (
      <div className="flex flex-1 items-center justify-center gap-3 px-4">
        <form
          className="w-full max-w-2xl"
          onSubmit={(e) => {
            e.preventDefault();
            const query = q.trim();
            if (!query) return;
            nav(`/app/${props.orgSlug}/search?q=${encodeURIComponent(query)}`);
          }}
        >
          <input
            className="w-full rounded-xl border border-slate-800 bg-slate-950/40 px-4 py-2 text-sm text-slate-200 outline-none placeholder:text-slate-500 focus:border-[color:var(--flux-focus-border)]"
            placeholder="Search this org..."
            value={q}
            onChange={(e) => setQ(e.target.value)}
          />
        </form>

        <div className="flex items-center overflow-hidden rounded-md border border-slate-800 bg-slate-950/40">
          <button
            className={`px-3 py-1.5 text-sm ${experience.rawMode === "work" ? "flux-btn-primary" : "text-slate-200 hover:bg-slate-900"}`}
            onClick={() => experience.setMode("work")}
            type="button"
          >
            Work
          </button>
          <button
            className={`px-3 py-1.5 text-sm ${experience.rawMode === "play" ? "flux-btn-primary" : "text-slate-200 hover:bg-slate-900"}`}
            onClick={() => experience.setMode("play")}
            type="button"
          >
            Play
          </button>
        </div>
      </div>
    );
  }

  return (
    <div className="flex flex-1 items-center justify-center gap-3 px-4">
      <form
        className="w-full max-w-2xl"
        onSubmit={(e) => {
          e.preventDefault();
          const query = q.trim();
          if (!query) return;
          nav(`/orgs?tab=discover&q=${encodeURIComponent(query)}`);
        }}
      >
        <input
          className="w-full rounded-xl border border-slate-800 bg-slate-950/40 px-4 py-2 text-sm text-slate-200 outline-none placeholder:text-slate-500 focus:border-[color:var(--flux-focus-border)]"
          placeholder="Search organizations, channels, messages..."
          value={q}
          onChange={(e) => setQ(e.target.value)}
        />
      </form>

      <div className="flex items-center overflow-hidden rounded-md border border-slate-800 bg-slate-950/40">
        <button
          className={`px-3 py-2 text-sm ${experience.rawMode === "work" ? "flux-btn-primary" : "text-slate-200 hover:bg-slate-900"}`}
          onClick={() => experience.setMode("work")}
          type="button"
          title="Switch to Work Mode"
        >
          Work
        </button>
        <button
          className={`px-3 py-2 text-sm ${experience.rawMode === "play" ? "flux-btn-primary" : "text-slate-200 hover:bg-slate-900"}`}
          onClick={() => experience.setMode("play")}
          type="button"
          title="Switch to Game Mode"
        >
          Play
        </button>
      </div>
    </div>
  );
}

export function AppShell() {
  const user = useAuthStore((s) => s.user);
  const logout = useAuthStore((s) => s.logout);
  const loadMe = useAuthStore((s) => s.loadMe);
  const [menuOpen, setMenuOpen] = useState(false);
  const menuRef = useRef<HTMLDivElement | null>(null);
  const nav = useNavigate();
  const loc = useLocation();
  const userThemeId = useUserThemeStore((s) => s.themeId);
  const userThemePreset = getThemePreset(userThemeId);
  const userMenuStyle = {
    colorScheme: userThemePreset.colorScheme,
    ["--brand-primary" as any]: userThemePreset.vars.brandPrimary,
    ["--brand-secondary" as any]: userThemePreset.vars.brandSecondary,
    ["--flux-on-accent" as any]: userThemePreset.vars.onPrimary,
    ["--app-bg" as any]: userThemePreset.vars.appBg,
    ["--app-surface" as any]: userThemePreset.vars.appSurface,
    ["--app-border" as any]: userThemePreset.vars.appBorder,
    ["--app-text" as any]: userThemePreset.vars.appText,
    ["--app-muted" as any]: userThemePreset.vars.appMuted,
  } as CSSProperties;

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

  const fallbackOrg = orgs.data?.organizations?.[0] ?? null;
  const fallbackOrgId = fallbackOrg?.id ?? null;
  const currentOrg = currentOrgSlug ? orgs.data?.organizations.find((o) => o.slug === currentOrgSlug) ?? null : null;

  const orgForAdmin = currentOrg ?? fallbackOrg;
  const orgForAdminSlug = orgForAdmin?.slug ?? null;
  const orgForAdminId = orgForAdmin?.id ?? null;

  const members = useQuery({
    enabled: !!user && !!orgForAdminId,
    queryKey: ["members", orgForAdminId],
    queryFn: () => apiFetch<MembersResponse>(`/orgs/${orgForAdminId}/members`),
    staleTime: 10_000,
  });

  const myRole = members.data?.members.find((m) => m.user_id === user?.id)?.role ?? null;
  const canSeeAdmin = myRole === "owner" || myRole === "admin";

  const currentChannelId = (() => {
    const m = loc.pathname.match(/^\/app\/[^/]+\/channels\/([^/]+)/);
    return m?.[1] ?? null;
  })();

  useEffect(() => {
    // Keep user populated when reloading pages.
    loadMe().catch(() => {});
  }, [loadMe]);

  useEffect(() => {
    if (!menuOpen) return;
    const onDown = (ev: MouseEvent | TouchEvent) => {
      const el = menuRef.current;
      if (!el) return;
      const target = ev.target as Node | null;
      if (target && el.contains(target)) return;
      setMenuOpen(false);
    };
    const onKey = (ev: KeyboardEvent) => {
      if (ev.key === "Escape") setMenuOpen(false);
    };
    document.addEventListener("mousedown", onDown);
    document.addEventListener("touchstart", onDown, { passive: true });
    document.addEventListener("keydown", onKey);
    return () => {
      document.removeEventListener("mousedown", onDown);
      document.removeEventListener("touchstart", onDown);
      document.removeEventListener("keydown", onKey);
    };
  }, [menuOpen]);

  return (
    <ExperienceProvider orgId={currentOrg?.id ?? null} channelId={currentChannelId} fallbackOrgId={fallbackOrgId}>
      <div className="flex min-h-dvh">
        {user ? <OrgRail /> : null}

        <div className="min-w-0 flex-1">
          <ToastViewport />
          <header className="border-b border-slate-800 bg-slate-950/80 backdrop-blur">
            <div className="flex items-center justify-between px-4 py-3">
              <Link to="/orgs" className="font-semibold tracking-tight">
                <BrandLogo showText={true} height={70} width={80} />
              </Link>

              {user ? <HeaderExtras orgSlug={currentOrgSlug} /> : null}

              <div className="flex items-center gap-3">
                {user ? (
                  <>
                    {currentOrgSlug ? (
                      <button
                        className="grid h-9 w-9 place-items-center rounded-md border border-slate-800 bg-slate-950/40 text-slate-200 hover:bg-slate-900"
                        onClick={() => nav(`/app/${currentOrgSlug}/settings/notifications`)}
                        type="button"
                        title="Notifications"
                        aria-label="Notifications"
                      >
                        <span aria-hidden="true">🔔</span>
                      </button>
                    ) : null}
                    <div className="relative" ref={menuRef}>
                      <button
                        aria-expanded={menuOpen}
                        aria-haspopup="menu"
                        className="flex items-center gap-2 rounded-md px-2 py-1 hover:bg-slate-900"
                        onClick={() => setMenuOpen((v) => !v)}
                        type="button"
                      >
                        <Avatar name={user.display_name} size={28} src={user.avatar_url ?? null} />
                        <span className="text-sm text-slate-200">{user.display_name}</span>
                      </button>

                      {menuOpen ? (
                        <div
                          aria-label="User menu"
                          className="absolute right-0 mt-2 w-48 overflow-hidden rounded-xl border border-slate-800 bg-slate-950 shadow-xl"
                          role="menu"
                          data-color-scheme={userThemePreset.colorScheme}
                          style={userMenuStyle}
                        >
                          <button
                            className="block w-full px-3 py-2 text-left text-sm text-slate-200 hover:bg-slate-900"
                            onClick={() => {
                              setMenuOpen(false);
                              nav("/profile");
                            }}
                            role="menuitem"
                            type="button"
                          >
                            Profile
                          </button>
                          {canSeeAdmin && orgForAdminSlug ? (
                            <button
                              className="block w-full px-3 py-2 text-left text-sm text-slate-200 hover:bg-slate-900"
                              onClick={() => {
                                setMenuOpen(false);
                                nav(`/admin/${orgForAdminSlug}`);
                              }}
                              role="menuitem"
                              type="button"
                            >
                              Admin
                            </button>
                          ) : null}
                          <button
                            className="block w-full px-3 py-2 text-left text-sm text-slate-200 hover:bg-slate-900"
                            onClick={async () => {
                              setMenuOpen(false);
                              await logout();
                              nav("/login");
                            }}
                            role="menuitem"
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
    </ExperienceProvider>
  );
}
