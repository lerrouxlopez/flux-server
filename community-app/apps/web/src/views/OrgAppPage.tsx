import { useEffect } from "react";
import { useQuery } from "@tanstack/react-query";
import { Link, useNavigate, useParams } from "react-router-dom";
import { apiFetch } from "../api/client";
import type { ChannelsResponse, OrgsListResponse } from "../api/types";
import { OrgSidebar } from "../components/OrgSidebar";
import { useBrandingStore } from "../state/branding";
import { useExperience } from "../features/experience/useExperience";

export function OrgAppPage() {
  const { org_slug } = useParams();
  const nav = useNavigate();
  const loadOrgBranding = useBrandingStore((s) => s.loadOrgBranding);
  const { rawMode: uiMode } = useExperience();

  const orgs = useQuery({
    queryKey: ["orgs"],
    queryFn: () => apiFetch<OrgsListResponse>("/orgs"),
  });

  const org = orgs.data?.organizations.find((o) => o.slug === org_slug);
  useEffect(() => {
    if (!org?.id) return;
    loadOrgBranding(org.id).catch(() => {});
  }, [loadOrgBranding, org?.id]);

  const channels = useQuery({
    enabled: !!org?.id,
    queryKey: ["channels", org?.id],
    queryFn: () => apiFetch<ChannelsResponse>(`/orgs/${org!.id}/channels`),
  });

  // Redirect to the mode-appropriate General channel on entry.
  useEffect(() => {
    if (!org?.slug) return;
    if (channels.isLoading || channels.isError) return;
    const list = channels.data?.channels ?? [];
    if (!list.length) return;
    const modeChannels = list.filter((c) => !c.experience_mode_hint || c.experience_mode_hint === uiMode);
    const target =
      modeChannels.find((c) => c.name.toLowerCase() === "general" && c.kind === "text") ??
      modeChannels.find((c) => c.kind === "text") ??
      modeChannels[0] ??
      list[0];
    if (!target?.id) return;
    nav(`/app/${org.slug}/channels/${target.id}`, { replace: true });
  }, [channels.data, channels.isError, channels.isLoading, nav, org?.slug, uiMode]);

  if (orgs.isLoading) return <div className="text-slate-300">Loading...</div>;
  if (!org) return <div className="text-slate-300">Org not found.</div>;

  return (
    <div className="grid gap-6 md:grid-cols-[280px_1fr]">
      <OrgSidebar org={org} />

      <section className="rounded-xl border border-slate-800 bg-slate-900/30 p-4">
        <div className="flex items-center justify-between gap-2">
          <div className="text-lg font-semibold">Client</div>
        </div>
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
            <div className="text-slate-300">No channels yet—create one with +.</div>
          )}
        </div>
      </section>

    </div>
  );
}
