import { useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { Link, useNavigate } from "react-router-dom";
import { apiFetch } from "../api/client";
import type { CreateOrgRequest, Org, OrgsListResponse } from "../api/types";
import { useAuthStore } from "../state/auth";
import { Input } from "../components/Input";
import { Button } from "../components/Button";

export function OrgsPage() {
  const nav = useNavigate();
  const qc = useQueryClient();
  const user = useAuthStore((s) => s.user);
  const [name, setName] = useState("");
  const [slug, setSlug] = useState("");
  const [err, setErr] = useState<string | null>(null);
  const [joinSlug, setJoinSlug] = useState("");
  const [inviteCode, setInviteCode] = useState("");
  const [joinErr, setJoinErr] = useState<string | null>(null);

  const q = useQuery({
    queryKey: ["orgs"],
    queryFn: () => apiFetch<OrgsListResponse>("/orgs"),
  });

  const create = useMutation({
    mutationFn: async (req: CreateOrgRequest) =>
      apiFetch<Org>("/orgs", { method: "POST", body: JSON.stringify(req) }),
    onSuccess: async (org) => {
      await qc.invalidateQueries({ queryKey: ["orgs"] });
      nav(`/app/${org.slug}`);
    },
    onError: (e) => setErr((e as Error).message),
  });

  const join = useMutation({
    mutationFn: async (input: { slug: string; invite_code: string }) =>
      apiFetch<{ status: string; organization_id: string; slug: string }>("/orgs/join", {
        method: "POST",
        body: JSON.stringify(input),
      }),
    onSuccess: async (r) => {
      await qc.invalidateQueries({ queryKey: ["orgs"] });
      nav(`/app/${r.slug}`);
    },
    onError: (e) => setJoinErr((e as Error).message),
  });

  if (q.isLoading) return <div className="text-slate-300">Loading orgs…</div>;
  if (q.isError) return <div className="text-red-400">{(q.error as Error).message}</div>;
  if (!q.data) return <div className="text-slate-300">No data.</div>;

  return (
    <div>
      <div className="flex items-center justify-between">
        <h1 className="text-xl font-semibold">Organizations</h1>
        {user ? <div className="text-sm text-slate-400">Signed in as {user.email}</div> : null}
      </div>

      <div className="mt-4 rounded-xl border border-slate-800 bg-slate-900/40 p-4">
        <div className="text-sm font-semibold">Create organization</div>
        <form
          className="mt-3 grid gap-3 sm:grid-cols-2"
          onSubmit={(e) => {
            e.preventDefault();
            setErr(null);
            create.mutate({ name, slug });
          }}
        >
          <div>
            <label className="mb-1 block text-sm text-slate-300">Name</label>
            <Input value={name} onChange={(e) => setName(e.target.value)} placeholder="Acme" />
          </div>
          <div>
            <label className="mb-1 block text-sm text-slate-300">Slug</label>
            <Input value={slug} onChange={(e) => setSlug(e.target.value)} placeholder="acme" />
          </div>
          <div className="sm:col-span-2 flex items-center gap-3">
            <Button disabled={create.isPending} type="submit">
              {create.isPending ? "Creating…" : "Create"}
            </Button>
            {err ? <div className="text-sm text-red-400">{err}</div> : null}
            <div className="text-xs text-slate-500">Creates default channels and branding.</div>
          </div>
        </form>
      </div>

      <div className="mt-4 rounded-xl border border-slate-800 bg-slate-900/40 p-4">
        <div className="text-sm font-semibold">Join organization</div>
        <div className="mt-1 text-xs text-slate-500">Paste the org slug and invite code you received.</div>
        <form
          className="mt-3 grid gap-3 sm:grid-cols-2"
          onSubmit={(e) => {
            e.preventDefault();
            setJoinErr(null);
            join.mutate({ slug: joinSlug, invite_code: inviteCode });
          }}
        >
          <div>
            <label className="mb-1 block text-sm text-slate-300">Org slug</label>
            <Input value={joinSlug} onChange={(e) => setJoinSlug(e.target.value)} placeholder="acme" />
          </div>
          <div>
            <label className="mb-1 block text-sm text-slate-300">Invite code</label>
            <Input
              value={inviteCode}
              onChange={(e) => setInviteCode(e.target.value)}
              placeholder="paste code"
              autoComplete="off"
            />
          </div>
          <div className="sm:col-span-2 flex items-center gap-3">
            <Button disabled={join.isPending} type="submit">
              {join.isPending ? "Joining…" : "Join"}
            </Button>
            {joinErr ? <div className="text-sm text-red-400">{joinErr}</div> : null}
          </div>
        </form>
      </div>

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
