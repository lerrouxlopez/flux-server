import { Outlet, Link } from "react-router-dom";
import { useBrandingStore } from "../state/branding";
import { useAuthStore } from "../state/auth";

export function AppShell() {
  const branding = useBrandingStore((s) => s.branding);
  const user = useAuthStore((s) => s.user);
  const logout = useAuthStore((s) => s.logout);

  return (
    <div className="min-h-dvh">
      <header className="border-b border-slate-800 bg-slate-950/80 backdrop-blur">
        <div className="mx-auto flex max-w-6xl items-center justify-between px-4 py-3">
          <Link to="/orgs" className="font-semibold tracking-tight">
            {branding?.app_name ?? "Community"}
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

