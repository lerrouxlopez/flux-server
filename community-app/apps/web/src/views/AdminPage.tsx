import { useMemo, useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { Link, useParams } from "react-router-dom";
import { apiFetch } from "../api/client";
import type {
  AuditLogsResponse,
  Branding,
  InviteResponse,
  MembersResponse,
  OrgsListResponse,
  Role,
  RolesResponse,
} from "../api/types";
import { Input } from "../components/Input";
import { Button } from "../components/Button";
import { useBrandingStore } from "../state/branding";

type Tab = "branding" | "members" | "audit";

export function AdminPage() {
  const { org_slug } = useParams();
  const qc = useQueryClient();
  const reloadPublicBranding = useBrandingStore((s) => s.loadBranding);
  const [tab, setTab] = useState<Tab>("branding");
  const [flash, setFlash] = useState<string | null>(null);

  const orgs = useQuery({
    queryKey: ["orgs"],
    queryFn: () => apiFetch<OrgsListResponse>("/orgs"),
  });
  const org = useMemo(
    () => orgs.data?.organizations.find((o) => o.slug === org_slug),
    [orgs.data, org_slug],
  );

  const branding = useQuery({
    enabled: !!org?.id,
    queryKey: ["branding", org?.id],
    queryFn: () => apiFetch<Branding>(`/orgs/${org!.id}/branding`),
  });

  const members = useQuery({
    enabled: !!org?.id,
    queryKey: ["members", org?.id],
    queryFn: () => apiFetch<MembersResponse>(`/orgs/${org!.id}/members`),
  });

  const roles = useQuery({
    enabled: !!org?.id,
    queryKey: ["roles", org?.id],
    queryFn: () => apiFetch<RolesResponse>(`/orgs/${org!.id}/roles`),
    staleTime: 30_000,
  });

  const audit = useQuery({
    enabled: !!org?.id && tab === "audit",
    queryKey: ["auditLogs", org?.id],
    queryFn: () => apiFetch<AuditLogsResponse>(`/orgs/${org!.id}/audit-logs?limit=100`),
  });

  const patchBranding = useMutation({
    mutationFn: async (body: Partial<Branding>) =>
      apiFetch<{ status: string }>(`/orgs/${org!.id}/branding`, {
        method: "PATCH",
        body: JSON.stringify(body),
      }),
    onSuccess: async () => {
      await qc.invalidateQueries({ queryKey: ["branding", org?.id] });
      await reloadPublicBranding(window.location.host);
      setFlash("Branding updated.");
      window.setTimeout(() => setFlash(null), 2500);
    },
  });

  const createInvite = useMutation({
    mutationFn: async () =>
      apiFetch<InviteResponse>(`/orgs/${org!.id}/invites`, {
        method: "POST",
        body: JSON.stringify({}),
      }),
    onSuccess: (r) => {
      setFlash(`Invite code created: ${r.code}`);
      window.setTimeout(() => setFlash(null), 7000);
    },
  });

  const updateRole = useMutation({
    mutationFn: async (input: { user_id: string; role: string }) =>
      apiFetch<{ status: string }>(`/orgs/${org!.id}/members/${input.user_id}`, {
        method: "PATCH",
        body: JSON.stringify({ role: input.role }),
      }),
    onSuccess: async () => {
      await qc.invalidateQueries({ queryKey: ["members", org?.id] });
      setFlash("Role updated.");
      window.setTimeout(() => setFlash(null), 2500);
    },
    onError: (e) => {
      setFlash((e as Error).message);
      window.setTimeout(() => setFlash(null), 5000);
    },
  });

  if (orgs.isLoading) return <div className="text-slate-300">Loading…</div>;
  if (orgs.isError) return <div className="text-red-400">{(orgs.error as Error).message}</div>;
  if (!org) return <div className="text-slate-300">Org not found.</div>;

  return (
    <div className="grid gap-6 md:grid-cols-[260px_1fr]">
      <aside className="rounded-xl border border-slate-800 bg-slate-900/30 p-3">
        <div className="px-2 py-2 text-sm font-semibold">{org.name}</div>
        <div className="mt-2 space-y-1">
          <Link
            to={`/app/${org.slug}`}
            className="block rounded-md px-2 py-1.5 text-sm text-slate-300 hover:bg-slate-800/60"
          >
            ← Back to client
          </Link>
        </div>
        <div className="mt-3 border-t border-slate-800 pt-3">
          <button
            className={`block w-full rounded-md px-2 py-1.5 text-left text-sm hover:bg-slate-800/60 ${
              tab === "branding" ? "text-white" : "text-slate-300"
            }`}
            onClick={() => setTab("branding")}
          >
            Branding
          </button>
          <button
            className={`mt-1 block w-full rounded-md px-2 py-1.5 text-left text-sm hover:bg-slate-800/60 ${
              tab === "members" ? "text-white" : "text-slate-300"
            }`}
            onClick={() => setTab("members")}
          >
            Members
          </button>
          <button
            className={`mt-1 block w-full rounded-md px-2 py-1.5 text-left text-sm hover:bg-slate-800/60 ${
              tab === "audit" ? "text-white" : "text-slate-300"
            }`}
            onClick={() => setTab("audit")}
          >
            Audit logs
          </button>
        </div>
      </aside>

      <section className="rounded-xl border border-slate-800 bg-slate-900/30 p-4">
        <div className="flex items-center justify-between">
          <div className="text-lg font-semibold">Admin</div>
          {flash ? <div className="text-xs text-emerald-300">{flash}</div> : null}
        </div>

        {tab === "branding" ? (
          <BrandingPanel
            branding={branding.data ?? null}
            loading={branding.isLoading}
            error={branding.isError ? (branding.error as Error).message : null}
            onSave={(v) => patchBranding.mutate(v)}
            saving={patchBranding.isPending}
          />
        ) : null}

        {tab === "members" ? (
          <MembersPanel
            members={members.data ?? null}
            loading={members.isLoading}
            error={members.isError ? (members.error as Error).message : null}
            roles={roles.data?.roles ?? null}
            rolesLoading={roles.isLoading}
            rolesError={roles.isError ? (roles.error as Error).message : null}
            onCreateInvite={() => createInvite.mutate()}
            inviteLoading={createInvite.isPending}
            onUpdateRole={(user_id, role) => updateRole.mutate({ user_id, role })}
            updatingRole={updateRole.isPending}
          />
        ) : null}

        {tab === "audit" ? (
          <AuditPanel
            entries={audit.data?.entries ?? null}
            loading={audit.isLoading}
            error={audit.isError ? (audit.error as Error).message : null}
          />
        ) : null}
      </section>
    </div>
  );
}

function BrandingPanel(props: {
  branding: Branding | null;
  loading: boolean;
  error: string | null;
  saving: boolean;
  onSave: (v: Partial<Branding>) => void;
}) {
  const b = props.branding;
  const [appName, setAppName] = useState(b?.app_name ?? "");
  const [logoUrl, setLogoUrl] = useState(b?.logo_url ?? "");
  const [primary, setPrimary] = useState(b?.primary_color ?? "");
  const [secondary, setSecondary] = useState(b?.secondary_color ?? "");
  const [privacy, setPrivacy] = useState(b?.privacy_url ?? "");
  const [terms, setTerms] = useState(b?.terms_url ?? "");

  if (props.loading) return <div className="mt-3 text-slate-300">Loading branding…</div>;
  if (props.error) return <div className="mt-3 text-red-400">{props.error}</div>;
  if (!b) return <div className="mt-3 text-slate-300">No branding profile.</div>;

  return (
    <div className="mt-4">
      <div className="text-sm text-slate-400">Configure the pre-login branding and app theme.</div>
      <form
        className="mt-4 grid gap-3 sm:grid-cols-2"
        onSubmit={(e) => {
          e.preventDefault();
          props.onSave({
            app_name: appName,
            logo_url: logoUrl || null,
            primary_color: primary || null,
            secondary_color: secondary || null,
            privacy_url: privacy || null,
            terms_url: terms || null,
          });
        }}
      >
        <div className="sm:col-span-2">
          <label className="mb-1 block text-sm text-slate-300">App name</label>
          <Input value={appName} onChange={(e) => setAppName(e.target.value)} />
        </div>
        <div className="sm:col-span-2">
          <label className="mb-1 block text-sm text-slate-300">Logo URL</label>
          <Input value={logoUrl} onChange={(e) => setLogoUrl(e.target.value)} placeholder="https://…" />
        </div>
        <div>
          <label className="mb-1 block text-sm text-slate-300">Primary color</label>
          <Input value={primary} onChange={(e) => setPrimary(e.target.value)} placeholder="#4f46e5" />
        </div>
        <div>
          <label className="mb-1 block text-sm text-slate-300">Secondary color</label>
          <Input value={secondary} onChange={(e) => setSecondary(e.target.value)} placeholder="#0ea5e9" />
        </div>
        <div>
          <label className="mb-1 block text-sm text-slate-300">Privacy URL</label>
          <Input value={privacy} onChange={(e) => setPrivacy(e.target.value)} placeholder="https://…" />
        </div>
        <div>
          <label className="mb-1 block text-sm text-slate-300">Terms URL</label>
          <Input value={terms} onChange={(e) => setTerms(e.target.value)} placeholder="https://…" />
        </div>
        <div className="sm:col-span-2 flex items-center gap-3">
          <Button disabled={props.saving} type="submit">
            {props.saving ? "Saving…" : "Save branding"}
          </Button>
          <div className="text-xs text-slate-500">
            Refresh the page to see updated header colors/logo everywhere.
          </div>
        </div>
      </form>
    </div>
  );
}

function MembersPanel(props: {
  members: MembersResponse | null;
  loading: boolean;
  error: string | null;
  roles: Role[] | null;
  rolesLoading: boolean;
  rolesError: string | null;
  onCreateInvite: () => void;
  inviteLoading: boolean;
  onUpdateRole: (userId: string, role: string) => void;
  updatingRole: boolean;
}) {
  if (props.loading) return <div className="mt-3 text-slate-300">Loading members…</div>;
  if (props.error) return <div className="mt-3 text-red-400">{props.error}</div>;
  if (!props.members) return <div className="mt-3 text-slate-300">No member data.</div>;

  if (props.rolesLoading) return <div className="mt-3 text-slate-300">Loading roles…</div>;
  if (props.rolesError) return <div className="mt-3 text-red-400">{props.rolesError}</div>;
  const roleOptions = (props.roles ?? []).map((r) => r.name).filter((n) => n !== "owner");

  return (
    <div className="mt-4">
      <div className="flex items-center justify-between">
        <div className="text-sm text-slate-400">{props.members.members.length} members</div>
        <Button disabled={props.inviteLoading} onClick={props.onCreateInvite} type="button">
          {props.inviteLoading ? "Creating…" : "Create invite"}
        </Button>
      </div>
      <div className="mt-4 overflow-hidden rounded-lg border border-slate-800">
        <table className="w-full text-left text-sm">
          <thead className="bg-slate-950/60 text-xs text-slate-400">
            <tr>
              <th className="px-3 py-2">User</th>
              <th className="px-3 py-2">Role</th>
              <th className="px-3 py-2">Joined</th>
              <th className="px-3 py-2"></th>
            </tr>
          </thead>
          <tbody>
            {props.members.members.map((m) => (
              <MemberRow
                key={m.user_id}
                member={m}
                roleOptions={roleOptions}
                updating={props.updatingRole}
                onUpdateRole={props.onUpdateRole}
              />
            ))}
          </tbody>
        </table>
      </div>
      <div className="mt-3 text-xs text-slate-500">
        Joining by invite is currently API-only (`POST /orgs/:org_id/members` with `invite_code`).
      </div>
    </div>
  );
}

function MemberRow(props: {
  member: MembersResponse["members"][number];
  roleOptions: string[];
  updating: boolean;
  onUpdateRole: (userId: string, role: string) => void;
}) {
  const { member } = props;
  const [role, setRole] = useState(member.role);

  const canEdit = member.role !== "owner";
  const dirty = role !== member.role;

  return (
    <tr className="border-t border-slate-800">
      <td className="px-3 py-2">
        <div className="text-slate-200">{member.display_name}</div>
        <div className="text-xs text-slate-500">{member.email}</div>
        <div className="mt-1 font-mono text-[11px] text-slate-500">{member.user_id}</div>
      </td>
      <td className="px-3 py-2 text-slate-200">
        {canEdit ? (
          <select
            className="w-full rounded-md border border-slate-800 bg-slate-900 px-2 py-1 text-sm text-slate-200 outline-none focus:border-indigo-500"
            value={role}
            onChange={(e) => setRole(e.target.value)}
          >
            {props.roleOptions.map((r) => (
              <option key={r} value={r}>
                {r}
              </option>
            ))}
          </select>
        ) : (
          <span className="font-semibold">{member.role}</span>
        )}
      </td>
      <td className="px-3 py-2 text-slate-400">{member.joined_at}</td>
      <td className="px-3 py-2 text-right">
        {canEdit ? (
          <button
            className="rounded-md bg-slate-800 px-3 py-1.5 text-xs text-slate-200 hover:bg-slate-700 disabled:opacity-50"
            disabled={!dirty || props.updating}
            onClick={() => props.onUpdateRole(member.user_id, role)}
            type="button"
          >
            {props.updating ? "Updating…" : "Update"}
          </button>
        ) : (
          <span className="text-xs text-slate-500">Owner</span>
        )}
      </td>
    </tr>
  );
}

function AuditPanel(props: { entries: AuditLogsResponse["entries"] | null; loading: boolean; error: string | null }) {
  if (props.loading) return <div className="mt-3 text-slate-300">Loading audit logs…</div>;
  if (props.error) return <div className="mt-3 text-red-400">{props.error}</div>;
  if (!props.entries) return <div className="mt-3 text-slate-300">No audit data.</div>;

  return (
    <div className="mt-4">
      <div className="text-sm text-slate-400">Latest events</div>
      <div className="mt-3 space-y-2">
        {props.entries.map((e) => (
          <div key={e.id} className="rounded-lg border border-slate-800 bg-slate-950/30 p-3">
            <div className="flex items-center justify-between">
              <div className="text-sm text-slate-200">{e.action}</div>
              <div className="text-xs text-slate-500">{e.created_at}</div>
            </div>
            <div className="mt-1 text-xs text-slate-400">
              {e.actor ? `${e.actor.display_name} (${e.actor.email})` : "system"}{" "}
              {e.target_type ? `→ ${e.target_type}` : ""}
              {e.target_id ? ` ${e.target_id}` : ""}
            </div>
          </div>
        ))}
      </div>
    </div>
  );
}
