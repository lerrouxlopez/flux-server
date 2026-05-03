import { useParams } from "react-router-dom";

export function AdminPage() {
  const { org_slug } = useParams();
  return (
    <div className="rounded-xl border border-slate-800 bg-slate-900/30 p-4">
      <h1 className="text-lg font-semibold">Admin</h1>
      <div className="mt-2 text-sm text-slate-300">Org: {org_slug}</div>
      <div className="mt-4 text-sm text-slate-400">TODO: audit log, members, branding editor.</div>
    </div>
  );
}

