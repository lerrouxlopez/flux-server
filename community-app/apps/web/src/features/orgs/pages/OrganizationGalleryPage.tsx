import { useMemo, useState } from "react";
import { useNavigate } from "react-router-dom";
import type { DiscoverOrg, JoinPolicy } from "../../../api/types";
import { Button } from "../../../components/Button";
import { useAuthStore } from "../../../state/auth";
import { useExperience } from "../../experience/useExperience";
import { useDiscoverOrganizations } from "../hooks/useDiscoverOrganizations";
import { useMyOrganizations } from "../hooks/useMyOrganizations";
import { useJoinOrganization } from "../hooks/useJoinOrganization";
import { OrgGalleryTabs, type OrgGalleryTab } from "../components/OrgGalleryTabs";
import { OrgSearchBar } from "../components/OrgSearchBar";
import { OrgCard } from "../components/OrgCard";
import { MyOrgCard } from "../components/MyOrgCard";
import { CreateOrgModal } from "../components/modals/CreateOrgModal";
import { JoinByInviteModal } from "../components/modals/JoinByInviteModal";
import { RequestInviteModal } from "../components/modals/RequestInviteModal";

function normalizePolicy(v: string): JoinPolicy | "any" {
  if (v === "open" || v === "invite_only" || v === "request" || v === "closed") return v;
  return "any";
}

function SkeletonCard(props: { compact: boolean }) {
  const pad = props.compact ? "p-3" : "p-4";
  return (
    <div className={`rounded-xl border border-slate-800 bg-slate-900/30 ${pad}`}>
      <div className="h-4 w-1/2 rounded bg-slate-800/60" />
      <div className="mt-2 h-3 w-1/3 rounded bg-slate-800/60" />
      {!props.compact ? <div className="mt-3 h-3 w-5/6 rounded bg-slate-800/60" /> : null}
    </div>
  );
}

export function OrganizationGalleryPage() {
  const nav = useNavigate();
  const user = useAuthStore((s) => s.user);
  const experience = useExperience();

  const [tab, setTab] = useState<OrgGalleryTab>("my_orgs");
  const [query, setQuery] = useState("");
  const [policy, setPolicy] = useState<JoinPolicy | "any">("any");

  const [createOpen, setCreateOpen] = useState(false);
  const [inviteOpen, setInviteOpen] = useState(false);
  const [inviteSlug, setInviteSlug] = useState<string | null>(null);
  const [requestOpen, setRequestOpen] = useState(false);
  const [requestOrgId, setRequestOrgId] = useState<string | null>(null);
  const [requestOrgName, setRequestOrgName] = useState<string | null>(null);

  const density = experience.density;
  const compact = density === "compact";

  const myOrgs = useMyOrganizations(!!user);
  const discoverEnabled = !!user && (tab === "discover" || tab === "requests");
  const discover = useDiscoverOrganizations({ enabled: discoverEnabled, query, policy });

  const joinOpen = useJoinOrganization();

  const discoverOrgs = (discover.data?.organizations ?? []) as DiscoverOrg[];
  const requestOrgs = useMemo(() => {
    return discoverOrgs.filter((o) => o.current_user_status === "pending_request" || o.current_user_status === "rejected");
  }, [discoverOrgs]);

  return (
    <div className="mx-auto max-w-5xl">
      <div className="flex flex-wrap items-end justify-between gap-3">
        <div>
          <h1 className="text-xl font-semibold text-slate-100">Organizations</h1>
          <div className="mt-1 text-xs text-slate-400">
            Discovery gallery renders in your current mode: <span className="font-semibold text-slate-200">{experience.label}</span>
          </div>
        </div>
        <div className="flex flex-wrap gap-2">
          <Button className="bg-slate-800 hover:bg-slate-700" onClick={() => setInviteOpen(true)} type="button">
            Enter invite code
          </Button>
          <Button className="flux-btn-primary" onClick={() => setCreateOpen(true)} type="button">
            Create organization
          </Button>
        </div>
      </div>

      <div className="mt-4">
        <OrgGalleryTabs tab={tab} onTabChange={setTab} />
      </div>

      {(tab === "discover" || tab === "requests") ? (
        <div className="mt-4 grid gap-3 md:grid-cols-[1fr_220px]">
          <OrgSearchBar value={query} onChange={setQuery} placeholder="Search by name, description, tags" />
          <div>
            <label className="sr-only" htmlFor="policy-filter">
              Join policy filter
            </label>
            <select
              id="policy-filter"
              className="w-full rounded-md border border-slate-800 bg-slate-900 px-3 py-2 text-sm text-slate-200 outline-none focus:border-[color:var(--flux-focus-border)]"
              value={policy}
              onChange={(e) => setPolicy(normalizePolicy(e.target.value))}
            >
              <option value="any">Any policy</option>
              <option value="open">Open</option>
              <option value="invite_only">Invite-only</option>
              <option value="request">Request access</option>
              <option value="closed">Closed</option>
            </select>
          </div>
        </div>
      ) : null}

      <div className={`mt-4 grid gap-3 ${compact ? "sm:grid-cols-2" : "md:grid-cols-2 lg:grid-cols-3"}`}>
        {tab === "my_orgs" ? (
          myOrgs.isLoading ? (
            <>
              <SkeletonCard compact={compact} />
              <SkeletonCard compact={compact} />
              <SkeletonCard compact={compact} />
            </>
          ) : myOrgs.isError ? (
            <div className="flux-text-danger">{(myOrgs.error as Error).message}</div>
          ) : (myOrgs.data?.organizations ?? []).length ? (
            (myOrgs.data?.organizations ?? []).map((o) => <MyOrgCard key={o.id} org={o} density={density} />)
          ) : (
            <div className="rounded-xl border border-slate-800 bg-slate-900/30 p-4 text-sm text-slate-300">
              No organizations yet.
            </div>
          )
        ) : null}

        {tab === "discover" ? (
          discover.isLoading ? (
            <>
              <SkeletonCard compact={compact} />
              <SkeletonCard compact={compact} />
              <SkeletonCard compact={compact} />
            </>
          ) : discover.isError ? (
            <div className="rounded-xl border border-slate-800 bg-slate-900/30 p-4">
              <div className="text-sm font-semibold text-slate-100">Discover is unavailable</div>
              <div className="mt-1 text-sm text-slate-300">{(discover.error as Error).message}</div>
              <div className="mt-2 text-xs text-slate-500">Backend must implement `GET /orgs/discover`.</div>
            </div>
          ) : discoverOrgs.length ? (
            discoverOrgs.map((o) => (
              <OrgCard
                key={o.id}
                org={o}
                density={density}
                onJoinOpen={(orgId) =>
                  joinOpen.mutate(
                    { orgId },
                    {
                      onSuccess: (r) => {
                        const slug = typeof r.slug === "string" && r.slug ? r.slug : o.slug;
                        nav(`/app/${slug}`);
                      },
                    },
                  )
                }
                onJoinByInvite={(slug) => {
                  setInviteSlug(slug);
                  setInviteOpen(true);
                }}
                onRequestAccess={(orgId, name) => {
                  setRequestOrgId(orgId);
                  setRequestOrgName(name);
                  setRequestOpen(true);
                }}
              />
            ))
          ) : (
            <div className="rounded-xl border border-slate-800 bg-slate-900/30 p-4 text-sm text-slate-300">
              No matching organizations.
            </div>
          )
        ) : null}

        {tab === "requests" ? (
          discover.isLoading ? (
            <>
              <SkeletonCard compact={compact} />
              <SkeletonCard compact={compact} />
            </>
          ) : discover.isError ? (
            <div className="flux-text-danger">{(discover.error as Error).message}</div>
          ) : requestOrgs.length ? (
            requestOrgs.map((o) => (
              <OrgCard
                key={o.id}
                org={o}
                density={density}
                onJoinOpen={(orgId) => joinOpen.mutate({ orgId })}
                onJoinByInvite={(slug) => {
                  setInviteSlug(slug);
                  setInviteOpen(true);
                }}
                onRequestAccess={(orgId, name) => {
                  setRequestOrgId(orgId);
                  setRequestOrgName(name);
                  setRequestOpen(true);
                }}
              />
            ))
          ) : (
            <div className="rounded-xl border border-slate-800 bg-slate-900/30 p-4 text-sm text-slate-300">
              No requests yet.
            </div>
          )
        ) : null}

        {tab === "invites" ? (
          <div className="rounded-xl border border-slate-800 bg-slate-900/30 p-4 text-sm text-slate-300">
            Use “Enter invite code” to join an invite-only organization.
          </div>
        ) : null}
      </div>

      <CreateOrgModal
        open={createOpen}
        onClose={() => setCreateOpen(false)}
        onCreated={(org) => nav(`/app/${org.slug}`)}
      />
      <JoinByInviteModal
        open={inviteOpen}
        initialSlug={inviteSlug}
        onClose={() => {
          setInviteOpen(false);
          setInviteSlug(null);
        }}
        onJoined={(slug) => nav(`/app/${slug}`)}
      />
      <RequestInviteModal
        open={requestOpen}
        orgId={requestOrgId}
        orgName={requestOrgName}
        onClose={() => {
          setRequestOpen(false);
          setRequestOrgId(null);
          setRequestOrgName(null);
        }}
      />
    </div>
  );
}

