import { Link } from "react-router-dom";
import type { Org } from "../../../api/types";

export function MyOrgCard(props: { org: Org; density: "comfortable" | "compact" }) {
  const pad = props.density === "comfortable" ? "p-4" : "p-3";
  return (
    <Link
      to={`/app/${props.org.slug}`}
      className={`block rounded-xl border border-slate-800 bg-slate-900/30 hover:border-slate-700 ${pad}`}
    >
      <div className="truncate text-sm font-semibold text-slate-100">{props.org.name}</div>
      <div className="mt-1 text-xs text-slate-400">/{props.org.slug}</div>
    </Link>
  );
}

