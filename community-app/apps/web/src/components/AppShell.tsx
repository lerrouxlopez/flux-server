import { Outlet, Link } from "react-router-dom";
import { useAuthStore } from "../state/auth";
import { useEffect } from "react";
import { BrandLogo } from "./BrandLogo";

export function AppShell() {
  const user = useAuthStore((s) => s.user);
  const logout = useAuthStore((s) => s.logout);
  const loadMe = useAuthStore((s) => s.loadMe);

  useEffect(() => {
    // Keep user populated when reloading pages.
    loadMe().catch(() => {});
  }, [loadMe]);

  return (
    <div className="min-h-dvh">
      <header className="border-b border-slate-800 bg-slate-950/80 backdrop-blur">
        <div className="mx-auto flex max-w-6xl items-center justify-between px-4 py-3">
          <Link to="/orgs" className="font-semibold tracking-tight">
            <BrandLogo showText={true} height={28} />
          </Link>
          <div className="flex items-center gap-3">
            {user ? (
              <>
                <span className="text-sm text-slate-300">{user.display_name}</span>
                <button
                  className="rounded-md bg-slate-800 px-3 py-1.5 text-sm hover:bg-slate-700"
                  onClick={logout}
                >
                  Logout
                </button>
              </>
            ) : (
              <Link className="text-sm text-slate-300 hover:text-white" to="/login">
                Login
              </Link>
            )}
          </div>
        </div>
      </header>
      <main className="mx-auto max-w-6xl px-4 py-6">
        <Outlet />
      </main>
    </div>
  );
}
