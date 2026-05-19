import { useEffect, useMemo, useState } from "react";
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
import { applyBrandingToDom, useBrandingStore } from "../state/branding";
import { COLOR_PALETTES, THEME_PRESETS, getThemePreset, type UIMode } from "../branding/presets";

type Tab = "branding" | "members" | "audit";

export function AdminPage() {
  const { org_slug } = useParams();
  const qc = useQueryClient();
  const loadOrgBranding = useBrandingStore((s) => s.loadOrgBranding);
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
      await loadOrgBranding(org!.id);
      setFlash("Branding updated.");
      window.setTimeout(() => setFlash(null), 2500);
    },
    onError: (e) => {
      setFlash((e as Error).message);
      window.setTimeout(() => setFlash(null), 5000);
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
          <Link
            to={`/admin/${org.slug}/access`}
            className="block rounded-md px-2 py-1.5 text-left text-sm text-slate-300 hover:bg-slate-800/60"
          >
            Organization Access
          </Link>
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
  const [mode, setMode] = useState<UIMode>(b?.ui_mode ?? "work");
  const [themeId, setThemeId] = useState<string>(b?.ui_theme ?? (mode === "play" ? "play-01" : "work-01"));
  const [logoDataUrl, setLogoDataUrl] = useState<string>(b?.logo_url ?? "");
  const initialPreset = getThemePreset(mode, themeId);
  const [primary, setPrimary] = useState<string>(b?.primary_color ?? initialPreset.vars.brandPrimary);
  const [secondary, setSecondary] = useState<string>(b?.secondary_color ?? initialPreset.vars.brandSecondary);
  const [bg, setBg] = useState<string>(b?.bg_color ?? initialPreset.vars.appBg);
  const [surface, setSurface] = useState<string>(b?.surface_color ?? initialPreset.vars.appSurface);
  const [text, setText] = useState<string>(b?.text_color ?? initialPreset.vars.appText);
  const [muted, setMuted] = useState<string>(b?.muted_color ?? initialPreset.vars.appMuted);
  const [border, setBorder] = useState<string>(b?.border_color ?? initialPreset.vars.appBorder);
  const [selectionBg, setSelectionBg] = useState<string>(b?.selection_bg ?? initialPreset.vars.brandPrimary);
  const [selectionText, setSelectionText] = useState<string>(b?.selection_text ?? initialPreset.vars.appText);
  const [dropdownBg, setDropdownBg] = useState<string>(b?.dropdown_bg ?? initialPreset.vars.appSurface);
  const [dropdownText, setDropdownText] = useState<string>(b?.dropdown_text ?? initialPreset.vars.appText);
  const [bubbleMeBg, setBubbleMeBg] = useState<string>(b?.chat_bubble_me_bg ?? initialPreset.vars.brandPrimary);
  const [bubbleMeText, setBubbleMeText] = useState<string>(b?.chat_bubble_me_text ?? "#ffffff");
  const [bubbleOtherBg, setBubbleOtherBg] = useState<string>(b?.chat_bubble_other_bg ?? initialPreset.vars.appSurface);
  const [bubbleOtherText, setBubbleOtherText] = useState<string>(b?.chat_bubble_other_text ?? initialPreset.vars.appText);

  const bOrgId = b?.organization_id ?? "";
  const bAppName = b?.app_name ?? "";
  const bUpdatedAt = b?.updated_at ?? "";

  useEffect(() => {
    if (!b) return;
    const preset = getThemePreset(mode, themeId);
    applyBrandingToDom({
      organization_id: b.organization_id,
      app_name: appName || b.app_name,
      theme: preset.colorScheme,
      ui_mode: mode,
      ui_theme: themeId,
      logo_url: logoDataUrl || null,
      primary_color: primary || null,
      secondary_color: secondary || null,
      bg_color: bg || null,
      surface_color: surface || null,
      text_color: text || null,
      muted_color: muted || null,
      border_color: border || null,
      selection_bg: selectionBg || null,
      selection_text: selectionText || null,
      dropdown_bg: dropdownBg || null,
      dropdown_text: dropdownText || null,
      chat_bubble_me_bg: bubbleMeBg || null,
      chat_bubble_me_text: bubbleMeText || null,
      chat_bubble_other_bg: bubbleOtherBg || null,
      chat_bubble_other_text: bubbleOtherText || null,
      updated_at: b.updated_at,
    });
    return () => {
      applyBrandingToDom(useBrandingStore.getState().branding);
    };
  }, [
    appName,
    bg,
    border,
    bubbleMeBg,
    bubbleMeText,
    bubbleOtherBg,
    bubbleOtherText,
    dropdownBg,
    dropdownText,
    logoDataUrl,
    mode,
    muted,
    primary,
    secondary,
    selectionBg,
    selectionText,
    surface,
    text,
    themeId,
    bOrgId,
    bAppName,
    bUpdatedAt,
  ]);

  if (props.loading) return <div className="mt-3 text-slate-300">Loading branding…</div>;
  if (props.error) return <div className="mt-3 text-red-400">{props.error}</div>;
  if (!b) return <div className="mt-3 text-slate-300">No branding profile.</div>;

  return (
    <div className="mt-4">
      <div className="text-sm text-slate-400">
        Work/Play branding with Slate + Material accents. Colors are chosen from palettes (no URLs / free-form hex).
      </div>
      <form
        className="mt-4 grid gap-3 sm:grid-cols-2"
        onSubmit={(e) => {
          e.preventDefault();
          props.onSave({
            app_name: appName,
            ui_mode: mode,
            ui_theme: themeId,
            logo_url: logoDataUrl || null,
            primary_color: primary || null,
            secondary_color: secondary || null,
            bg_color: bg || null,
            surface_color: surface || null,
            text_color: text || null,
            muted_color: muted || null,
            border_color: border || null,
            selection_bg: selectionBg || null,
            selection_text: selectionText || null,
            dropdown_bg: dropdownBg || null,
            dropdown_text: dropdownText || null,
            chat_bubble_me_bg: bubbleMeBg || null,
            chat_bubble_me_text: bubbleMeText || null,
            chat_bubble_other_bg: bubbleOtherBg || null,
            chat_bubble_other_text: bubbleOtherText || null,
          });
        }}
      >
        <div>
          <label className="mb-1 block text-sm text-slate-300">Mode</label>
          <select
            className="w-full rounded-md border border-slate-800 bg-slate-900 px-3 py-2 text-sm text-slate-200 outline-none focus:border-indigo-500"
            value={mode}
            onChange={(e) => {
              const nextMode = e.target.value as UIMode;
              setMode(nextMode);
              const nextTheme = nextMode === "play" ? "play-01" : "work-01";
              setThemeId(nextTheme);
              const p = getThemePreset(nextMode, nextTheme);
              setPrimary(p.vars.brandPrimary);
              setSecondary(p.vars.brandSecondary);
              setBg(p.vars.appBg);
              setSurface(p.vars.appSurface);
              setText(p.vars.appText);
              setMuted(p.vars.appMuted);
              setBorder(p.vars.appBorder);
              setSelectionBg(p.vars.brandPrimary);
              setSelectionText(p.vars.appText);
              setDropdownBg(p.vars.appSurface);
              setDropdownText(p.vars.appText);
              setBubbleMeBg(p.vars.brandPrimary);
              setBubbleMeText("#ffffff");
              setBubbleOtherBg(p.vars.appSurface);
              setBubbleOtherText(p.vars.appText);
            }}
          >
            <option value="work">Work Mode</option>
            <option value="play">Play Mode</option>
          </select>
        </div>
        <div>
          <label className="mb-1 block text-sm text-slate-300">Theme</label>
          <select
            className="w-full rounded-md border border-slate-800 bg-slate-900 px-3 py-2 text-sm text-slate-200 outline-none focus:border-indigo-500"
            value={themeId}
            onChange={(e) => {
              const next = e.target.value;
              setThemeId(next);
              const p = getThemePreset(mode, next);
              setPrimary(p.vars.brandPrimary);
              setSecondary(p.vars.brandSecondary);
              setBg(p.vars.appBg);
              setSurface(p.vars.appSurface);
              setText(p.vars.appText);
              setMuted(p.vars.appMuted);
              setBorder(p.vars.appBorder);
              setSelectionBg(p.vars.brandPrimary);
              setSelectionText(p.vars.appText);
              setDropdownBg(p.vars.appSurface);
              setDropdownText(p.vars.appText);
              setBubbleMeBg(p.vars.brandPrimary);
              setBubbleMeText("#ffffff");
              setBubbleOtherBg(p.vars.appSurface);
              setBubbleOtherText(p.vars.appText);
            }}
          >
            {THEME_PRESETS.filter((t) => t.mode === mode).map((t) => (
              <option key={t.id} value={t.id}>
                {t.label}
              </option>
            ))}
          </select>
          <div className="mt-1 text-xs text-slate-500">{getThemePreset(mode, themeId).description}</div>
        </div>
        <div>
          <label className="mb-1 block text-sm text-slate-300">Scheme</label>
          <div className="rounded-md border border-slate-800 bg-slate-900 px-3 py-2 text-sm text-slate-200">
            {getThemePreset(mode, themeId).colorScheme === "light" ? "Light" : "Dark"} (from theme)
          </div>
        </div>
        <div className="sm:col-span-2">
          <label className="mb-1 block text-sm text-slate-300">App name</label>
          <Input value={appName} onChange={(e) => setAppName(e.target.value)} />
        </div>
        <div className="sm:col-span-2">
          <label className="mb-1 block text-sm text-slate-300">Logo</label>
          <div className="flex items-center gap-3">
            <input
              type="file"
              accept="image/*"
              className="block w-full text-sm text-slate-300 file:mr-4 file:rounded-md file:border-0 file:bg-slate-800 file:px-3 file:py-2 file:text-sm file:text-slate-200 hover:file:bg-slate-700"
              onChange={(e) => {
                const f = e.target.files?.[0];
                if (!f) return;
                if (f.size > 1_000_000) {
                  alert("Logo too large (max 1MB).");
                  e.target.value = "";
                  return;
                }
                const fr = new FileReader();
                fr.onload = () => setLogoDataUrl(typeof fr.result === "string" ? fr.result : "");
                fr.readAsDataURL(f);
              }}
            />
            {logoDataUrl ? (
              <button
                type="button"
                className="rounded-md border border-slate-800 bg-slate-900 px-3 py-2 text-xs text-slate-200 hover:bg-slate-800/60"
                onClick={() => setLogoDataUrl("")}
              >
                Clear
              </button>
            ) : null}
          </div>
          {logoDataUrl ? (
            <div className="mt-2 rounded-lg border border-slate-800 bg-slate-950/30 p-3">
              <img src={logoDataUrl} alt="Logo preview" className="h-10 w-auto" />
            </div>
          ) : (
            <div className="mt-1 text-xs text-slate-500">Upload an image to use as logo (no URLs).</div>
          )}
        </div>
        <div className="sm:col-span-2">
          <div className="mb-2 flex items-center justify-between">
            <div className="text-sm font-semibold text-slate-200">Colors</div>
            <button
              type="button"
              className="rounded-md border border-slate-800 bg-slate-900 px-3 py-1.5 text-xs text-slate-200 hover:bg-slate-800/60"
              onClick={() => {
                const p = getThemePreset(mode, themeId);
                setPrimary(p.vars.brandPrimary);
                setSecondary(p.vars.brandSecondary);
                setBg(p.vars.appBg);
                setSurface(p.vars.appSurface);
                setText(p.vars.appText);
                setMuted(p.vars.appMuted);
                setBorder(p.vars.appBorder);
                setSelectionBg(p.vars.brandPrimary);
                setSelectionText(p.vars.appText);
                setDropdownBg(p.vars.appSurface);
                setDropdownText(p.vars.appText);
                setBubbleMeBg(p.vars.brandPrimary);
                setBubbleMeText("#ffffff");
                setBubbleOtherBg(p.vars.appSurface);
                setBubbleOtherText(p.vars.appText);
              }}
            >
              Reset to theme
            </button>
          </div>
          <div className="grid gap-3 sm:grid-cols-2">
            <ColorPicker label="Primary (buttons)" value={primary} palette={COLOR_PALETTES.primary} onPick={setPrimary} />
            <ColorPicker label="Secondary" value={secondary} palette={COLOR_PALETTES.primary} onPick={setSecondary} />
            <ColorPicker label="Background" value={bg} palette={COLOR_PALETTES.background} onPick={setBg} />
            <ColorPicker label="Surface" value={surface} palette={COLOR_PALETTES.surface} onPick={setSurface} />
            <ColorPicker label="Text" value={text} palette={COLOR_PALETTES.text} onPick={setText} />
            <ColorPicker label="Muted text" value={muted} palette={COLOR_PALETTES.muted} onPick={setMuted} />
            <ColorPicker label="Border" value={border} palette={COLOR_PALETTES.border} onPick={setBorder} />
            <ColorPicker
              label="Selected text bg"
              value={selectionBg}
              palette={COLOR_PALETTES.primary}
              onPick={setSelectionBg}
            />
            <ColorPicker
              label="Selected text color"
              value={selectionText}
              palette={COLOR_PALETTES.text}
              onPick={setSelectionText}
            />
            <ColorPicker label="Dropdown bg" value={dropdownBg} palette={COLOR_PALETTES.surface} onPick={setDropdownBg} />
            <ColorPicker label="Dropdown text" value={dropdownText} palette={COLOR_PALETTES.text} onPick={setDropdownText} />
            <ColorPicker
              label="Chat bubble (me) bg"
              value={bubbleMeBg}
              palette={COLOR_PALETTES.primary}
              onPick={setBubbleMeBg}
            />
            <ColorPicker
              label="Chat bubble (me) text"
              value={bubbleMeText}
              palette={["#ffffff", ...COLOR_PALETTES.text]}
              onPick={setBubbleMeText}
            />
            <ColorPicker
              label="Chat bubble (other) bg"
              value={bubbleOtherBg}
              palette={COLOR_PALETTES.surface}
              onPick={setBubbleOtherBg}
            />
            <ColorPicker
              label="Chat bubble (other) text"
              value={bubbleOtherText}
              palette={COLOR_PALETTES.text}
              onPick={setBubbleOtherText}
            />
          </div>
          <div className="mt-2 text-xs text-slate-500">Privacy/terms URLs are intentionally not editable here.</div>
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

function ColorPicker(props: { label: string; value: string; palette: string[]; onPick: (v: string) => void }) {
  return (
    <div className="rounded-lg border border-slate-800 bg-slate-950/30 p-3">
      <div className="mb-2 flex items-center justify-between gap-2">
        <div className="text-xs font-semibold text-slate-300">{props.label}</div>
        <div className="font-mono text-[11px] text-slate-500">{props.value}</div>
      </div>
      <div className="flex flex-wrap gap-2">
        {props.palette.map((c) => {
          const selected = c.toLowerCase() === props.value.toLowerCase();
          return (
            <button
              key={c}
              type="button"
              className={`h-6 w-6 rounded-full border ${selected ? "border-white" : "border-slate-700"} hover:scale-[1.03]`}
              style={{ backgroundColor: c }}
              onClick={() => props.onPick(c)}
              title={c}
              aria-label={`${props.label} ${c}`}
            />
          );
        })}
      </div>
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
