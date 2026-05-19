import { useEffect, useMemo, useState } from "react";
import { Link, useParams } from "react-router-dom";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { apiFetch } from "../api/client";
import type {
  DiscoverySettingsResponse,
  JoinRequestsListResponse,
  OrgsListResponse,
  PatchDiscoverySettingsRequest,
} from "../api/types";
import { Button } from "../components/Button";
import { Input } from "../components/Input";
import { TextArea } from "../components/TextArea";
import { useExperience } from "../features/experience/useExperience";
import { OrgCard } from "../features/orgs/components/OrgCard";

function parseTags(value: string): string[] {
  return value
    .split(",")
    .map((t) => t.trim())
    .filter((t) => t.length > 0)
    .slice(0, 20);
}

export function AdminAccessPage() {
  const { org_slug } = useParams();
  const qc = useQueryClient();
  const experience = useExperience();

  const orgs = useQuery({
    queryKey: ["orgs"],
    queryFn: () => apiFetch<OrgsListResponse>("/orgs"),
    staleTime: 30_000,
    retry: false,
  });
  const org = useMemo(() => (orgs.data?.organizations ?? []).find((o) => o.slug === org_slug) ?? null, [orgs.data, org_slug]);

  const settings = useQuery({
    enabled: !!org?.id,
    queryKey: ["orgs", org?.id, "discovery-settings"],
    queryFn: () => apiFetch<DiscoverySettingsResponse>(`/orgs/${org!.id}/discovery-settings`),
    retry: false,
  });

  const requests = useQuery({
    enabled: !!org?.id,
    queryKey: ["orgs", org?.id, "join-requests"],
    queryFn: () => apiFetch<JoinRequestsListResponse>(`/orgs/${org!.id}/join-requests`),
    staleTime: 5_000,
    retry: false,
  });

  const [discoverable, setDiscoverable] = useState(false);
  const [joinPolicy, setJoinPolicy] = useState<DiscoverySettingsResponse["join_policy"]>("invite_only");
  const [description, setDescription] = useState("");
  const [category, setCategory] = useState("");
  const [tagsText, setTagsText] = useState("");
  const [memberCountVisible, setMemberCountVisible] = useState(true);
  const [onlineCountVisible, setOnlineCountVisible] = useState(false);
  const [avatarUrl, setAvatarUrl] = useState("");
  const [bannerUrl, setBannerUrl] = useState("");
  const [flash, setFlash] = useState<string | null>(null);

  useEffect(() => {
    if (!settings.data) return;
    setDiscoverable(settings.data.discoverable);
    setJoinPolicy(settings.data.join_policy);
    setDescription((settings.data.description ?? "").trim());
    setCategory((settings.data.category ?? "").trim());
    setTagsText((settings.data.tags ?? []).join(", "));
    setMemberCountVisible(settings.data.member_count_visible);
    setOnlineCountVisible(settings.data.online_count_visible);
    setAvatarUrl((settings.data.avatar_url ?? "").trim());
    setBannerUrl((settings.data.banner_url ?? "").trim());
  }, [settings.data]);

  const isDirty = useMemo(() => {
    const s = settings.data;
    if (!s) return false;
    return (
      discoverable !== s.discoverable ||
      joinPolicy !== s.join_policy ||
      description.trim() !== (s.description ?? "").trim() ||
      category.trim() !== (s.category ?? "").trim() ||
      tagsText.trim() !== (s.tags ?? []).join(", ").trim() ||
      memberCountVisible !== s.member_count_visible ||
      onlineCountVisible !== s.online_count_visible ||
      avatarUrl.trim() !== (s.avatar_url ?? "").trim() ||
      bannerUrl.trim() !== (s.banner_url ?? "").trim()
    );
  }, [
    settings.data,
    discoverable,
    joinPolicy,
    description,
    category,
    tagsText,
    memberCountVisible,
    onlineCountVisible,
    avatarUrl,
    bannerUrl,
  ]);

  const patch = useMutation({
    mutationFn: async (body: PatchDiscoverySettingsRequest) =>
      apiFetch<{ status: string }>(`/orgs/${org!.id}/discovery-settings`, { method: "PATCH", body: JSON.stringify(body) }),
    onSuccess: async () => {
      await qc.invalidateQueries({ queryKey: ["orgs", org?.id, "discovery-settings"] });
      setFlash("Discovery settings updated.");
      window.setTimeout(() => setFlash(null), 2500);
    },
    onError: (e) => {
      setFlash((e as Error).message);
      window.setTimeout(() => setFlash(null), 5000);
    },
  });

  const approve = useMutation({
    mutationFn: async (requestId: string) =>
      apiFetch<{ status: string }>(`/orgs/${org!.id}/join-requests/${requestId}/approve`, { method: "POST" }),
    onSuccess: async () => {
      await qc.invalidateQueries({ queryKey: ["orgs", org?.id, "join-requests"] });
      await qc.invalidateQueries({ queryKey: ["orgs"] });
    },
  });

  const reject = useMutation({
    mutationFn: async (requestId: string) =>
      apiFetch<{ status: string }>(`/orgs/${org!.id}/join-requests/${requestId}/reject`, { method: "POST" }),
    onSuccess: async () => {
      await qc.invalidateQueries({ queryKey: ["orgs", org?.id, "join-requests"] });
    },
  });

  if (orgs.isLoading) return <div className="text-slate-300">Loadingâ€¦</div>;
  if (orgs.isError) return <div className="text-red-400">{(orgs.error as Error).message}</div>;
  if (!org) return <div className="text-slate-300">Org not found.</div>;

  const tableTone =
    experience.rawMode === "work"
      ? "bg-white text-slate-900 border-slate-200"
      : "bg-slate-900/30 text-slate-100 border-slate-800";
  const panelTone =
    experience.rawMode === "work"
      ? "bg-white text-slate-900 border-slate-200"
      : "bg-slate-900/30 text-slate-100 border-slate-800";

  const previewOrg = {
    id: org.id,
    slug: org.slug,
    name: org.name,
    description: description.trim() || null,
    avatar_url: avatarUrl.trim() || null,
    banner_url: bannerUrl.trim() || null,
    join_policy: joinPolicy,
    category: category.trim() || null,
    tags: parseTags(tagsText),
    member_count: memberCountVisible ? 0 : null,
    online_count: onlineCountVisible ? 0 : null,
    current_user_status: "member" as const,
  };

  return (
    <div className="grid gap-6 md:grid-cols-[260px_1fr]">
      <aside className={`rounded-xl border p-3 ${panelTone}`}>
        <div className="px-2 py-2 text-sm font-semibold">{org.name}</div>
        <div className="mt-2 space-y-1">
          <Link
            to={`/app/${org.slug}`}
            className="block rounded-md px-2 py-1.5 text-sm text-slate-300 hover:bg-slate-800/60"
          >
            â† Back to client
          </Link>
        </div>
        <div className="mt-3 border-t border-slate-200/20 pt-3">
          <Link
            to={`/admin/${org.slug}`}
            className="block rounded-md px-2 py-1.5 text-left text-sm text-slate-300 hover:bg-slate-800/60"
          >
            Admin Overview
          </Link>
          <div className="mt-1 block rounded-md px-2 py-1.5 text-left text-sm font-semibold text-slate-100">
            Organization Access
          </div>
        </div>
      </aside>

      <section className="min-w-0">
        <div className="flex flex-wrap items-end justify-between gap-3">
          <div>
            <div className="text-xl font-semibold">Organization Access</div>
            <div className="mt-1 text-xs text-slate-400">
              Public discovery and join approval. Preview renders in your current mode:{" "}
              <span className="font-semibold text-slate-200">{experience.label}</span>
            </div>
          </div>
          <div className="flex items-center gap-2">
            <Button
              className="flux-btn-primary"
              disabled={!isDirty || patch.isPending || settings.isLoading || !settings.data}
              onClick={() =>
                patch.mutate({
                  discoverable,
                  join_policy: joinPolicy,
                  description,
                  category,
                  tags: parseTags(tagsText),
                  member_count_visible: memberCountVisible,
                  online_count_visible: onlineCountVisible,
                  avatar_url: avatarUrl,
                  banner_url: bannerUrl,
                })
              }
              type="button"
            >
              {patch.isPending ? "Savingâ€¦" : "Save changes"}
            </Button>
          </div>
        </div>

        {flash ? <div className="mt-3 text-sm text-slate-200">{flash}</div> : null}

        <div className="mt-4 grid gap-6 lg:grid-cols-2">
          <div className={`rounded-xl border p-4 ${panelTone}`}>
            <div className="text-sm font-semibold">Discovery settings</div>
            {settings.isLoading ? <div className="mt-3 text-slate-300">Loadingâ€¦</div> : null}
            {settings.isError ? <div className="mt-3 text-sm text-red-400">{(settings.error as Error).message}</div> : null}

            {!settings.isLoading && settings.data ? (
              <div className="mt-4 grid gap-3">
                <label className="flex items-center gap-2 text-sm">
                  <input
                    type="checkbox"
                    checked={discoverable}
                    onChange={(e) => setDiscoverable(e.target.checked)}
                  />
                  <span>Discoverable in public gallery</span>
                </label>

                <div>
                  <label className="mb-1 block text-sm text-slate-300" htmlFor="join-policy">
                    Join policy
                  </label>
                  <select
                    id="join-policy"
                    className="w-full rounded-md border border-slate-800 bg-slate-900 px-3 py-2 text-sm text-slate-200 outline-none focus:border-[color:var(--flux-focus-border)]"
                    value={joinPolicy}
                    onChange={(e) => setJoinPolicy(e.target.value as any)}
                  >
                    <option value="open">Open</option>
                    <option value="invite_only">Invite-only</option>
                    <option value="request">Request access</option>
                    <option value="closed">Closed</option>
                  </select>
                  <div className="mt-1 text-xs text-slate-500">Closed orgs never appear in discovery for non-members.</div>
                </div>

                <div>
                  <label className="mb-1 block text-sm text-slate-300" htmlFor="org-description">
                    Description
                  </label>
                  <TextArea
                    id="org-description"
                    rows={3}
                    value={description}
                    onChange={(e) => setDescription(e.target.value)}
                    placeholder="Short public description"
                  />
                </div>

                <div className="grid gap-3 sm:grid-cols-2">
                  <div>
                    <label className="mb-1 block text-sm text-slate-300" htmlFor="org-category">
                      Category
                    </label>
                    <Input id="org-category" value={category} onChange={(e) => setCategory(e.target.value)} placeholder="Ops" />
                  </div>
                  <div>
                    <label className="mb-1 block text-sm text-slate-300" htmlFor="org-tags">
                      Tags (comma-separated)
                    </label>
                    <Input
                      id="org-tags"
                      value={tagsText}
                      onChange={(e) => setTagsText(e.target.value)}
                      placeholder="dev, community, gaming"
                    />
                  </div>
                </div>

                <div className="grid gap-3 sm:grid-cols-2">
                  <label className="flex items-center gap-2 text-sm">
                    <input
                      type="checkbox"
                      checked={memberCountVisible}
                      onChange={(e) => setMemberCountVisible(e.target.checked)}
                    />
                    <span>Public member count</span>
                  </label>
                  <label className="flex items-center gap-2 text-sm">
                    <input
                      type="checkbox"
                      checked={onlineCountVisible}
                      onChange={(e) => setOnlineCountVisible(e.target.checked)}
                    />
                    <span>Public online count</span>
                  </label>
                </div>

                <div className="grid gap-3 sm:grid-cols-2">
                  <div>
                    <label className="mb-1 block text-sm text-slate-300" htmlFor="avatar-url">
                      Avatar URL
                    </label>
                    <Input id="avatar-url" value={avatarUrl} onChange={(e) => setAvatarUrl(e.target.value)} placeholder="https://..." />
                  </div>
                  <div>
                    <label className="mb-1 block text-sm text-slate-300" htmlFor="banner-url">
                      Banner URL
                    </label>
                    <Input id="banner-url" value={bannerUrl} onChange={(e) => setBannerUrl(e.target.value)} placeholder="https://..." />
                  </div>
                </div>
              </div>
            ) : null}
          </div>

          <div className="space-y-6">
            <div className={`rounded-xl border p-4 ${panelTone}`}>
              <div className="text-sm font-semibold">Public gallery preview</div>
              <div className="mt-1 text-xs text-slate-400">This is how the org card appears in `/orgs`.</div>
              <div className="mt-4">
                <OrgCard
                  org={previewOrg as any}
                  density={experience.density}
                  onJoinOpen={() => {}}
                  onJoinByInvite={() => {}}
                  onRequestAccess={() => {}}
                />
              </div>
            </div>

            <div className={`rounded-xl border p-4 ${panelTone}`}>
              <div className="flex items-center justify-between gap-2">
                <div>
                  <div className="text-sm font-semibold">Join requests</div>
                  <div className="mt-1 text-xs text-slate-400">Approve or reject pending requests.</div>
                </div>
                <Button
                  className="bg-slate-800 hover:bg-slate-700"
                  disabled={requests.isFetching}
                  onClick={() => requests.refetch()}
                  type="button"
                >
                  Refresh
                </Button>
              </div>

              {requests.isLoading ? <div className="mt-3 text-slate-300">Loadingâ€¦</div> : null}
              {requests.isError ? <div className="mt-3 text-sm text-red-400">{(requests.error as Error).message}</div> : null}

              {!requests.isLoading && !requests.isError ? (
                (requests.data?.requests ?? []).filter((r) => r.status === "pending").length ? (
                  <div className={`mt-4 overflow-hidden rounded-lg border ${tableTone}`}>
                    <table className="w-full text-left text-sm">
                      <thead className={experience.rawMode === "work" ? "bg-slate-50 text-xs text-slate-500" : "bg-slate-950/60 text-xs text-slate-400"}>
                        <tr>
                          <th className="px-3 py-2">User</th>
                          <th className="px-3 py-2">Message</th>
                          <th className="px-3 py-2">Created</th>
                          <th className="px-3 py-2"></th>
                        </tr>
                      </thead>
                      <tbody>
                        {(requests.data?.requests ?? [])
                          .filter((r) => r.status === "pending")
                          .map((r) => (
                            <tr key={r.id} className={experience.rawMode === "work" ? "border-t border-slate-200" : "border-t border-slate-800"}>
                              <td className="px-3 py-2">
                                <div className="font-mono text-[11px] text-slate-500">{r.user_id}</div>
                              </td>
                              <td className="px-3 py-2">
                                <div className="max-w-[420px] truncate text-slate-300">{(r.message ?? "").trim() || "(no message)"}</div>
                              </td>
                              <td className="px-3 py-2 text-xs text-slate-500">{r.created_at}</td>
                              <td className="px-3 py-2 text-right">
                                <div className="flex justify-end gap-2">
                                  <Button
                                    className="flux-btn-primary px-2 py-1 text-xs"
                                    disabled={approve.isPending}
                                    onClick={() => approve.mutate(r.id)}
                                    type="button"
                                  >
                                    Approve
                                  </Button>
                                  <Button
                                    className="bg-slate-800 px-2 py-1 text-xs hover:bg-slate-700"
                                    disabled={reject.isPending}
                                    onClick={() => reject.mutate(r.id)}
                                    type="button"
                                  >
                                    Reject
                                  </Button>
                                </div>
                              </td>
                            </tr>
                          ))}
                      </tbody>
                    </table>
                  </div>
                ) : (
                  <div className="mt-3 text-sm text-slate-300">No pending requests.</div>
                )
              ) : null}
            </div>
          </div>
        </div>
      </section>
    </div>
  );
}

