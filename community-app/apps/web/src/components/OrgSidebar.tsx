import { useMemo } from "react";
import { Link } from "react-router-dom";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { apiFetch } from "../api/client";
import type {
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

type PresenceStatus = "online" | "offline";

export function OrgSidebar(props: {
  org: Org;
  activeChannelId?: string | null;
  presenceByUser?: Record<string, PresenceStatus>;
  onCreateRoomClick?: () => void;
}) {
  const qc = useQueryClient();
  const me = useAuthStore((s) => s.user);

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

  const myRole = members.data?.members.find((m) => m.user_id === me?.id)?.role ?? null;
  const canSeeAdmin = myRole === "owner" || myRole === "admin";

  const onlineCount = useMemo(() => {
    const presence = props.presenceByUser ?? {};
    return (members.data?.members ?? []).filter((m) => presence[m.user_id] === "online").length;
  }, [members.data, props.presenceByUser]);

  return (
    <aside className="rounded-xl border border-slate-800 bg-slate-900/30 p-3">
      <div className="flex items-center justify-between gap-2 px-2 py-2">
        <div className="min-w-0 text-sm font-semibold">{props.org.name}</div>
        {props.onCreateRoomClick ? (
          <button
            aria-label="Create room"
            className="grid h-8 w-8 place-items-center rounded-md border border-slate-800 bg-slate-900 text-slate-200 hover:bg-slate-800/60"
            onClick={props.onCreateRoomClick}
            type="button"
          >
            +
          </button>
        ) : null}
      </div>

      <div className="mt-1 flex gap-3 px-2 text-xs">
        <Link className="text-slate-300 hover:text-white" to={`/app/${props.org.slug}`}>
          Channels
        </Link>
        <Link className="text-slate-300 hover:text-white" to={`/app/${props.org.slug}/friends`}>
          Friends
        </Link>
      </div>

      <div className="mt-3">
        <div className="px-2 text-xs font-semibold text-slate-400">Channels</div>
        <div className="mt-2 space-y-1">
          {(channels.data?.channels ?? []).map((c) => {
            const active = c.id === props.activeChannelId;
            return (
              <Link
                key={c.id}
                to={`/app/${props.org.slug}/channels/${c.id}`}
                className={`flex items-center gap-2 rounded-md px-2 py-2 text-sm hover:bg-slate-800/60 ${
                  active ? "bg-slate-800/60 text-white" : "text-slate-200"
                }`}
              >
                <span className="grid h-8 w-8 place-items-center rounded-lg bg-slate-900 text-slate-200">#</span>
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
          {(dms.data?.dms ?? []).map((d) => {
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
          {!(dms.data?.dms ?? []).length ? <div className="px-2 text-sm text-slate-400">No chats yet.</div> : null}
        </div>
      </div>

      <div className="mt-3 border-t border-slate-800 pt-3">
        <div className="px-2 text-xs font-semibold text-slate-400">
          Members {props.presenceByUser ? (members.isLoading ? "" : `(${onlineCount} online)`) : ""}
        </div>
        <div className="mt-2 space-y-1">
          {(members.data?.members ?? []).slice(0, 12).map((m) => {
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
                    <span className={`h-2 w-2 rounded-full ${online ? "bg-emerald-400" : "bg-slate-600"}`} />
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
                        className="bg-indigo-600 px-2 py-1 text-xs hover:bg-indigo-500"
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
        {openDm.data?.channel_id ? (
          <div className="mt-2 px-2">
            <Link className="text-xs text-indigo-400 hover:underline" to={`/app/${props.org.slug}/channels/${openDm.data.channel_id}`}>
              Open DM
            </Link>
          </div>
        ) : null}
        <div className="mt-2 px-2">
          {canSeeAdmin ? (
            <Link className="text-xs text-indigo-400 hover:underline" to={`/admin/${props.org.slug}`}>
              Open admin panel
            </Link>
          ) : null}
        </div>
      </div>
    </aside>
  );
}
