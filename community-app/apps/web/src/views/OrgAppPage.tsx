import { useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { Link, useNavigate, useParams } from "react-router-dom";
import { apiFetch } from "../api/client";
import type { Channel, ChannelsResponse, OrgsListResponse } from "../api/types";
import { Input } from "../components/Input";
import { Button } from "../components/Button";

export function OrgAppPage() {
  const { org_slug } = useParams();
  const nav = useNavigate();
  const qc = useQueryClient();
  const [channelName, setChannelName] = useState("");
  const [channelKind, setChannelKind] = useState<"text" | "private">("text");
  const [err, setErr] = useState<string | null>(null);

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

  const createChannel = useMutation({
    mutationFn: async () =>
      apiFetch<Channel>(`/orgs/${org!.id}/channels`, {
        method: "POST",
        body: JSON.stringify({ name: channelName, kind: channelKind }),
      }),
    onSuccess: async (ch) => {
      setChannelName("");
      setErr(null);
      await qc.invalidateQueries({ queryKey: ["channels", org?.id] });
      nav(`/app/${org!.slug}/channels/${ch.id}`);
    },
    onError: (e) => setErr((e as Error).message),
  });

  if (orgs.isLoading) return <div className="text-slate-300">Loading…</div>;
  if (!org) return <div className="text-slate-300">Org not found.</div>;

  return (
    <div className="grid gap-6 md:grid-cols-[280px_1fr]">
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
          <div className="px-2 text-xs font-semibold text-slate-400">Create channel</div>
          <form
            className="mt-2 space-y-2 px-2"
            onSubmit={(e) => {
              e.preventDefault();
              setErr(null);
              createChannel.mutate();
            }}
          >
            <Input
              value={channelName}
              onChange={(e) => setChannelName(e.target.value)}
              placeholder="e.g. product"
            />
            <div className="flex gap-2">
              <select
                className="w-full rounded-md border border-slate-800 bg-slate-900 px-2 py-2 text-sm text-slate-200 outline-none focus:border-indigo-500"
                value={channelKind}
                onChange={(e) => setChannelKind(e.target.value as any)}
              >
                <option value="text">text</option>
                <option value="private">private</option>
              </select>
              <Button disabled={createChannel.isPending} type="submit">
                {createChannel.isPending ? "…" : "Add"}
              </Button>
            </div>
            {err ? <div className="text-xs text-red-400">{err}</div> : null}
          </form>
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
        <div className="text-lg font-semibold">Client</div>
        <div className="mt-4 flex flex-wrap gap-2">
          {(channels.data?.channels ?? []).length ? (
            <>
              <Link
                className="rounded-md bg-slate-800 px-3 py-2 text-sm text-slate-200 hover:bg-slate-700"
                to={`/app/${org.slug}/channels/${(channels.data?.channels ?? [])[0].id}`}
              >
                Open first channel
              </Link>
              {(() => {
                const general = (channels.data?.channels ?? []).find((c) => c.name === "general");
                if (!general) return null;
                return (
                  <Link
                    className="rounded-md bg-slate-800 px-3 py-2 text-sm text-slate-200 hover:bg-slate-700"
                    to={`/app/${org.slug}/channels/${general.id}`}
                  >
                    Open #general
                  </Link>
                );
              })()}
            </>
          ) : (
            <div className="text-slate-300">No channels yet—create one in the sidebar.</div>
          )}
        </div>
      </section>
    </div>
  );
}
