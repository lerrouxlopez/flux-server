import { Navigate, useLocation } from "react-router-dom";
import { useAuthStore } from "../state/auth";

export function RequireAuth(props: { children: React.ReactNode }) {
  useAuthStore((s) => s.hydrate)();
  const user = useAuthStore((s) => s.user);
  const token = useAuthStore((s) => s.accessToken);
  const loc = useLocation();

  if (!token) return <Navigate to="/login" replace state={{ from: loc.pathname }} />;
  if (!user) return <div className="text-slate-300">Loading session…</div>;
  return <>{props.children}</>;
}

export function RequireGuest(props: { children: React.ReactNode }) {
  useAuthStore((s) => s.hydrate)();
  const token = useAuthStore((s) => s.accessToken);
  if (token) return <Navigate to="/orgs" replace />;
  return <>{props.children}</>;
}

