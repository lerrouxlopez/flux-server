import { useEffect, useMemo, useRef, useState } from "react";
import { Link, useNavigate } from "react-router-dom";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { apiFetch } from "../api/client";
import type {
  Channel,
  ChannelsResponse,
  DmsResponse,
  FriendRequestsResponse,
  FriendsResponse,
  MembersResponse,
  Org,
} from "../api/types";
import { useAuthStore } from "../state/auth";
import { Button } from "./Button";
import { Avatar } from "./Avatar";
import { Input } from "./Input";
import { Modal } from "./Modal";
import { useExperience } from "../features/experience/useExperience";

type PresenceStatus = "online" | "offline";

export function OrgSidebar(props: {
  org: Org;
  activeChannelId?: string | null;
  presenceByUser?: Record<string, PresenceStatus>;
}) {
  const qc = useQueryClient();
  const nav = useNavigate();
  const me = useAuthStore((s) => s.user);
  const uiMode = useExperience().rawMode;

  const [showAllMembers, setShowAllMembers] = useState(false);
  const [createOpen, setCreateOpen] = useState(false);
  const [channelName, setChannelName] = useState("");
  const [channelKind, setChannelKind] = useState<"text" | "voice" | "video">("text");
  const [createErr, setCreateErr] = useState<string | null>(null);

  const [ctxMenu, setCtxMenu] = useState<{ channel: Channel; x: number; y: number } | null>(null);
  const [editTarget, setEditTarget] = useState<{ id: string; name: string } | null>(null);
  const ctxMenuRef = useRef<HTMLDivElement | null>(null);

  useEffect(() => {
    if (!ctxMenu) return;
    function onDown(e: MouseEvent) {
      if (ctxMenuRef.current?.contains(e.target as Node)) return;
      setCtxMenu(null);
    }
    document.addEventListener("mousedown", onDown);
    return () => document.removeEventListener("mousedown", onDown);
  }, [ctxMenu]);

  const channels = useQuery({
    queryKey: ["channels", props.org.id],
    queryFn: () => apiFetch<ChannelsResponse>(`/orgs/${props.org.id}/channels`),
  });

  const dms = useQuery({
    queryKey: ["dms", props.org.id],
    queryFn: () => apiFetch<DmsResponse>(`/orgs/${props.org.id}/dms`),
    staleTime: 10_000,
  });

  const members = useQuery({
    queryKey: ["members", props.org.id],
    queryFn: () => apiFetch<MembersResponse>(`/orgs/${props.org.id}/members`),
    staleTime: 10_000,
  });

  const friends = useQuery({
    queryKey: ["friends", props.org.id],
    queryFn: () => apiFetch<FriendsResponse>(`/orgs/${props.org.id}/friends`),
    staleTime: 10_000,
  });

  const friendRequests = useQuery({
    queryKey: ["friendRequests", props.org.id],
    queryFn: () => apiFetch<FriendRequestsResponse>(`/orgs/${props.org.id}/friends/requests`),
    staleTime: 5_000,
  });

  const sendFriendRequest = useMutation({
    mutationFn: async (userId: string) =>
      apiFetch<{ status: string; request_id: string }>(`/orgs/${props.org.id}/friends/requests`, {
        method: "POST",
        body: JSON.stringify({ user_id: userId }),
      }),
    onSuccess: async () => {
      await qc.invalidateQueries({ queryKey: ["friendRequests", props.org.id] });
      await qc.invalidateQueries({ queryKey: ["friends", props.org.id] });
    },
  });

  const acceptFriendRequest = useMutation({
    mutationFn: async (requestId: string) =>
      apiFetch<{ status: string }>(`/orgs/${props.org.id}/friends/requests/${requestId}/accept`, { method: "POST" }),
    onSuccess: async () => {
      await qc.invalidateQueries({ queryKey: ["friendRequests", props.org.id] });
      await qc.invalidateQueries({ queryKey: ["friends", props.org.id] });
    },
  });

  const openDm = useMutation({
    mutationFn: async (userId: string) =>
      apiFetch<{ channel_id: string }>(`/orgs/${props.org.id}/dms/${userId}`, { method: "POST" }),
    onSuccess: async () => {
      await qc.invalidateQueries({ queryKey: ["dms", props.org.id] });
    },
  });

  const createChannel = useMutation({
    mutationFn: async () =>
      apiFetch<Channel>(`/orgs/${props.org.id}/channels`, {
        method: "POST",
        body: JSON.stringify({ name: channelName, kind: channelKind }),
      }),
    onSuccess: async (ch) => {
      setChannelName("");
      setCreateErr(null);
      setCreateOpen(false);
      await qc.invalidateQueries({ queryKey: ["channels", props.org.id] });
      nav(`/app/${props.org.slug}/channels/${ch.id}`);
    },
    onError: (e) => setCreateErr((e as Error).message),
  });

  const updateChannel = useMutation({
    mutationFn: async ({ id, name }: { id: string; name: string }) =>
      apiFetch<{ status: string }>(`/channels/${id}`, {
        method: "PATCH",
        body: JSON.stringify({ name }),
      }),
    onSuccess: async () => {
      setEditTarget(null);
      await qc.invalidateQueries({ queryKey: ["channels", props.org.id] });
    },
  });

  const deleteChannel = useMutation({
    mutationFn: async (id: string) =>
      apiFetch<{ status: string }>(`/channels/${id}`, { method: "DELETE" }),
    onSuccess: async (_data, id) => {
      await qc.invalidateQueries({ queryKey: ["channels", props.org.id] });
      if (id === props.activeChannelId) {
        const next =
          (channels.data?.channels ?? []).find((c) => c.id !== id && c.kind === "text" && c.name.toLowerCase() === "general") ??
          (channels.data?.channels ?? []).find((c) => c.id !== id && c.kind === "text") ??
          (channels.data?.channels ?? []).find((c) => c.id !== id) ??
          null;
        nav(next ? `/app/${props.org.slug}/channels/${next.id}` : `/app/${props.org.slug}`);
      }
    },
  });

  const onlineCount = useMemo(() => {
    const presence = props.presenceByUser ?? {};
    return (members.data?.members ?? []).filter((m) => presence[m.user_id] === "online").length;
  }, [members.data, props.presenceByUser]);

  const filteredChannels = useMemo(() => {
    const all = channels.data?.channels ?? [];
    return all.filter((c) => !c.experience_mode_hint || c.experience_mode_hint === uiMode);
  }, [channels.data, uiMode]);

  const filteredDms = useMemo(() => {
    return dms.data?.dms ?? [];
  }, [dms.data]);

  const visibleMembers = useMemo(() => {
    const all = members.data?.members ?? [];
    if (uiMode === "work" && !showAllMembers) return all.slice(0, 6);
    return all.slice(0, 12);
  }, [members.data, showAllMembers, uiMode]);

  return (
    <>
    <aside className="rounded-xl border border-slate-800 bg-slate-900/30 p-3">
      <div className="flex items-center justify-between gap-2 px-2 py-2">
        <div className="min-w-0 text-sm font-semibold">{props.org.name}</div>
        <button
          aria-label="Create room"
          className="grid h-8 w-8 place-items-center rounded-md border border-slate-800 bg-slate-900 text-slate-200 hover:bg-slate-800/60"
          onClick={() => setCreateOpen(true)}
          type="button"
        >
          +
        </button>
      </div>

      <div className="mt-1 flex gap-3 px-2 text-xs">
        <Link className="text-slate-300 hover:text-white" to={`/app/${props.org.slug}`}>
          Channels
        </Link>
        <Link className="text-slate-300 hover:text-white" to={`/app/${props.org.slug}/friends`}>
          Friends
        </Link>
      </div>

      <div className="mt-3 px-2">
        <button
          type="button"
          className="w-full rounded-md border border-slate-800 bg-slate-900 px-2 py-2 text-xs text-slate-200 hover:bg-slate-800/60"
          onClick={() => setCreateOpen(true)}
        >
          + New room
        </button>
      </div>

      <div className="mt-3">
        <div className="px-2 text-xs font-semibold text-slate-400">Channels</div>
        <div className="mt-2 space-y-1">
          {filteredChannels.map((c) => {
            const active = c.id === props.activeChannelId;
            return (
              <Link
                key={c.id}
                to={`/app/${props.org.slug}/channels/${c.id}`}
                className={`flex items-center gap-2 rounded-md px-2 py-2 text-sm hover:bg-slate-800/60 ${
                  active ? "bg-slate-800/60 text-white" : "text-slate-200"
                }`}
                onContextMenu={(e) => {
                  if (!c.created_by) return;
                  e.preventDefault();
                  setCtxMenu({ channel: c, x: e.clientX, y: e.clientY });
                }}
              >
                <span className="grid h-8 w-8 place-items-center rounded-lg bg-slate-900 text-slate-200">
                  {c.kind === "voice" ? "🔊" : c.kind === "video" ? "🎥" : "#"}
                </span>
                <div className="min-w-0">
                  <div className="truncate font-medium">{c.name}</div>
                  <div className="truncate text-xs text-slate-500">Channel</div>
                </div>
              </Link>
            );
          })}
        </div>
      </div>

      <div className="mt-3 border-t border-slate-800 pt-3">
        <div className="px-2 text-xs font-semibold text-slate-400">Chats</div>
        <div className="mt-2 space-y-1">
          {filteredDms.map((d) => {
            const active = d.channel_id === props.activeChannelId;
            return (
              <Link
                key={d.channel_id}
                to={`/app/${props.org.slug}/channels/${d.channel_id}`}
                className={`flex items-center gap-2 rounded-md px-2 py-2 text-sm hover:bg-slate-800/60 ${
                  active ? "bg-slate-800/60 text-white" : "text-slate-200"
                }`}
              >
                <Avatar
                  name={d.peer.display_name}
                  online={props.presenceByUser ? (props.presenceByUser[d.peer.id] === "online" ? true : false) : undefined}
                  size={32}
                />
                <div className="min-w-0">
                  <div className="truncate font-medium">{d.peer.display_name}</div>
                  <div className="truncate text-xs text-slate-500">Direct message</div>
                </div>
              </Link>
            );
          })}
          {!(dms.data?.dms ?? []).length ? (
            <div className="px-2 text-sm text-slate-400">No chats yet.</div>
          ) : null}
        </div>
      </div>

      <div className="mt-3 border-t border-slate-800 pt-3">
        <div className="px-2 text-xs font-semibold text-slate-400">
          Members {props.presenceByUser ? (members.isLoading ? "" : `(${onlineCount} online)`) : ""}
        </div>
        <div className="mt-2 space-y-1">
          {visibleMembers.map((m) => {
            const presence = props.presenceByUser ?? {};
            const online = presence[m.user_id] === "online";
            const isMe = m.user_id === me?.id;
            const isFriend = (friends.data?.friends ?? []).some((f) => f.id === m.user_id);
            const incomingReq = (friendRequests.data?.requests ?? []).find(
              (r) => r.requester.id === m.user_id && r.addressee.id === me?.id,
            );
            const outgoingReq = (friendRequests.data?.requests ?? []).find(
              (r) => r.requester.id === me?.id && r.addressee.id === m.user_id,
            );

            return (
              <div key={m.user_id} className="flex items-center justify-between rounded-md px-2 py-1 text-sm">
                <div className="flex min-w-0 items-center gap-2">
                  {props.presenceByUser ? (
                    <span className={`h-2 w-2 rounded-full ${online ? "flux-dot-online" : "flux-dot-offline"}`} />
                  ) : null}
                  <span className="truncate text-slate-200">{m.display_name}</span>
                </div>
                <div className="flex items-center gap-2">
                  {!isMe ? (
                    isFriend ? (
                      <Button
                        className="bg-slate-800 px-2 py-1 text-xs hover:bg-slate-700"
                        disabled={openDm.isPending}
                        onClick={() => openDm.mutate(m.user_id)}
                        type="button"
                      >
                        Message
                      </Button>
                    ) : incomingReq ? (
                      <Button
                        className="flux-btn-primary px-2 py-1 text-xs"
                        disabled={acceptFriendRequest.isPending}
                        onClick={() => acceptFriendRequest.mutate(incomingReq.id)}
                        type="button"
                      >
                        Accept
                      </Button>
                    ) : outgoingReq ? (
                      <span className="text-xs text-slate-500">Pending</span>
                    ) : (
                      <Button
                        className="bg-slate-800 px-2 py-1 text-xs hover:bg-slate-700"
                        disabled={sendFriendRequest.isPending}
                        onClick={() => sendFriendRequest.mutate(m.user_id)}
                        type="button"
                      >
                        Add
                      </Button>
                    )
                  ) : null}
                  <span className="text-xs text-slate-500">{m.role}</span>
                </div>
              </div>
            );
          })}
        </div>
        {uiMode === "work" && (members.data?.members ?? []).length > 6 ? (
          <div className="mt-2 px-2">
            <button
              type="button"
              className="text-xs text-slate-400 hover:text-slate-200"
              onClick={() => setShowAllMembers((v) => !v)}
            >
              {showAllMembers ? "Show less" : "Show more"}
            </button>
          </div>
        ) : null}
        {openDm.data?.channel_id ? (
          <div className="mt-2 px-2">
            <Link className="flux-link text-xs" to={`/app/${props.org.slug}/channels/${openDm.data.channel_id}`}>
              Open DM
            </Link>
          </div>
        ) : null}
        <div className="mt-2 px-2">{/* Admin moved to user dropdown */}</div>
      </div>

      <Modal open={createOpen} title="Create room" onClose={() => setCreateOpen(false)}>
        <form
          className="space-y-2"
          onSubmit={(e) => {
            e.preventDefault();
            setCreateErr(null);
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
              onChange={(e) => setChannelKind(e.target.value as "text" | "voice" | "video")}
            >
              <option value="text">text</option>
              <option value="voice">voice</option>
              <option value="video">video</option>
            </select>
            <Button disabled={createChannel.isPending} type="submit">
              {createChannel.isPending ? "..." : "Create"}
            </Button>
          </div>
          {createErr ? <div className="text-xs text-red-400">{createErr}</div> : null}
        </form>
      </Modal>
    </aside>

    {ctxMenu ? (
      <div
        ref={ctxMenuRef}
        style={{ position: "fixed", top: ctxMenu.y, left: ctxMenu.x, zIndex: 100 }}
        className="min-w-[140px] overflow-hidden rounded-lg border border-slate-700 bg-slate-900 py-1 shadow-xl"
      >
        <button
          type="button"
          className="w-full px-3 py-2 text-left text-sm text-slate-200 hover:bg-slate-800"
          onClick={() => {
            setEditTarget({ id: ctxMenu.channel.id, name: ctxMenu.channel.name });
            setCtxMenu(null);
          }}
        >
          Edit
        </button>
        <button
          type="button"
          className="w-full px-3 py-2 text-left text-sm text-rose-400 hover:bg-slate-800"
          onClick={() => {
            deleteChannel.mutate(ctxMenu.channel.id);
            setCtxMenu(null);
          }}
        >
          Delete
        </button>
      </div>
    ) : null}

    <Modal open={!!editTarget} title="Edit channel" onClose={() => setEditTarget(null)}>
      <form
        className="space-y-2"
        onSubmit={(e) => {
          e.preventDefault();
          if (!editTarget) return;
          updateChannel.mutate({ id: editTarget.id, name: editTarget.name.trim() });
        }}
      >
        <Input
          value={editTarget?.name ?? ""}
          onChange={(e) => setEditTarget((t) => (t ? { ...t, name: e.target.value } : t))}
          placeholder="Channel name"
        />
        <div className="flex justify-end">
          <Button disabled={updateChannel.isPending || !editTarget?.name.trim()} type="submit">
            {updateChannel.isPending ? "..." : "Save"}
          </Button>
        </div>
      </form>
    </Modal>
    </>
  );
}
