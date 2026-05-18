import { useMemo } from "react";
import { Link, useParams } from "react-router-dom";
import { useMutation, useQuery } from "@tanstack/react-query";
import { apiFetch } from "../api/client";
import type { NotificationsContextResponse, OrgsListResponse } from "../api/types";
import { Button } from "../components/Button";

const WORK_DEFAULT_PROFILE_ID = "11111111-1111-1111-1111-111111111111";
const PLAY_DEFAULT_PROFILE_ID = "22222222-2222-2222-2222-222222222222";

function RuleRow(props: { label: string; enabled: boolean; hint?: string }) {
  return (
    <div className="flex items-start justify-between gap-3 rounded-lg border border-slate-800 bg-slate-950/20 px-3 py-2">
      <div className="min-w-0">
        <div className="text-sm font-medium text-slate-200">{props.label}</div>
        {props.hint ? <div className="mt-0.5 text-xs text-slate-400">{props.hint}</div> : null}
      </div>
      <div className={`text-xs font-semibold ${props.enabled ? "flux-text-success" : "text-slate-500"}`}>
        {props.enabled ? "On" : "Off"}
      </div>
    </div>
  );
}

export function NotificationSettingsPage() {
  const { org_slug } = useParams();

  const orgs = useQuery({
    queryKey: ["orgs"],
    queryFn: () => apiFetch<OrgsListResponse>("/orgs"),
    staleTime: 30_000,
  });

  const org = useMemo(() => {
    const slug = (org_slug ?? "").trim();
    return (orgs.data?.organizations ?? []).find((o) => o.slug === slug) ?? null;
  }, [org_slug, orgs.data]);

  const ctx = useQuery({
    queryKey: ["notifications", "context", org?.id],
    queryFn: () => apiFetch<NotificationsContextResponse>(`/notifications/context?org_id=${org!.id}`),
    enabled: !!org?.id,
    staleTime: 10_000,
  });

  const setWorkDefault = useMutation({
    mutationFn: async () => {
      if (!org?.id) throw new Error("Missing org");
      await apiFetch<{ status: string }>("/notifications/overrides/user", {
        method: "PATCH",
        body: JSON.stringify({ org_id: org.id, mode: "work", profile_id: null }),
      });
    },
    onSuccess: () => ctx.refetch(),
  });

  const setPlayDefault = useMutation({
    mutationFn: async () => {
      if (!org?.id) throw new Error("Missing org");
      await apiFetch<{ status: string }>("/notifications/overrides/user", {
        method: "PATCH",
        body: JSON.stringify({ org_id: org.id, mode: "play", profile_id: null }),
      });
    },
    onSuccess: () => ctx.refetch(),
  });

  const setWorkMinimal = useMutation({
    mutationFn: async () => {
      if (!org?.id) throw new Error("Missing org");
      await apiFetch<{ status: string }>("/notifications/overrides/user", {
        method: "PATCH",
        body: JSON.stringify({ org_id: org.id, mode: "work", profile_id: PLAY_DEFAULT_PROFILE_ID }),
      });
    },
    onSuccess: () => ctx.refetch(),
  });

  const setPlayWorky = useMutation({
    mutationFn: async () => {
      if (!org?.id) throw new Error("Missing org");
      await apiFetch<{ status: string }>("/notifications/overrides/user", {
        method: "PATCH",
        body: JSON.stringify({ org_id: org.id, mode: "play", profile_id: WORK_DEFAULT_PROFILE_ID }),
      });
    },
    onSuccess: () => ctx.refetch(),
  });

  if (orgs.isLoading) return <div className="text-slate-300">Loading…</div>;
  if (!org) return <div className="text-slate-300">Org not found.</div>;

  return (
    <div className="mx-auto max-w-2xl">
      <div className="flex items-end justify-between gap-3">
        <div>
          <div className="text-xl font-semibold">Notifications</div>
          <div className="mt-1 text-xs text-slate-400">
            Per-org notification profiles (Work/Play) with user overrides. Defaults avoid all-message spam.
          </div>
        </div>
        <Link className="flux-link text-sm" to={`/app/${org.slug}`}>
          Back
        </Link>
      </div>

      <div className="mt-4 rounded-xl border border-slate-800 bg-slate-900/30 p-4">
        {ctx.isLoading ? <div className="text-slate-300">Loading context…</div> : null}
        {ctx.isError ? <div className="flux-text-danger text-sm">Failed to load notification context.</div> : null}

        {ctx.data ? (
          <>
            <div className="flex flex-wrap items-center justify-between gap-2">
              <div className="text-sm text-slate-300">
                Current mode: <span className="font-semibold text-slate-100">{ctx.data.mode}</span>{" "}
                <span className="text-slate-500">({ctx.data.profile_source})</span>
              </div>
              <div className="flex flex-wrap gap-2">
                <Button className="bg-slate-800 hover:bg-slate-700" onClick={() => setWorkDefault.mutate()} type="button">
                  Work: Default
                </Button>
                <Button className="bg-slate-800 hover:bg-slate-700" onClick={() => setWorkMinimal.mutate()} type="button">
                  Work: Minimal
                </Button>
                <Button className="bg-slate-800 hover:bg-slate-700" onClick={() => setPlayDefault.mutate()} type="button">
                  Play: Default
                </Button>
                <Button className="bg-slate-800 hover:bg-slate-700" onClick={() => setPlayWorky.mutate()} type="button">
                  Play: Work-ish
                </Button>
              </div>
            </div>

            <div className="mt-4 grid gap-2">
              <RuleRow
                label="All messages"
                enabled={ctx.data.behavior.message_all}
                hint="Off by default to avoid spam."
              />
              <RuleRow label="Mentions" enabled={ctx.data.behavior.message_mentions} />
              <RuleRow label="Thread replies" enabled={ctx.data.behavior.thread_replies} />
              <RuleRow label="Pin changes" enabled={ctx.data.behavior.pin_changes} />
              <RuleRow label="Media events" enabled={ctx.data.behavior.media_events} />
            </div>

            <div className="mt-3 text-xs text-slate-500">
              UI skeleton: selection will become a real profile picker once profile listing + editing is wired.
            </div>
          </>
        ) : null}
      </div>
    </div>
  );
}

