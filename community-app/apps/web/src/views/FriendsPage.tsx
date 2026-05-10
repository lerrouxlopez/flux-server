import { useMemo, useState } from "react";
import { useNavigate, useParams } from "react-router-dom";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { apiFetch } from "../api/client";
import type { FriendRequestsResponse, FriendsResponse, MembersResponse, OrgsListResponse } from "../api/types";
import { Button } from "../components/Button";
import { Input } from "../components/Input";
import { OrgSidebar } from "../components/OrgSidebar";
import { useAuthStore } from "../state/auth";

export function FriendsPage() {
  const { org_slug } = useParams();
  const nav = useNavigate();
  const qc = useQueryClient();
  const me = useAuthStore((s) => s.user);

  const [q, setQ] = useState("");

  const orgs = useQuery({
    queryKey: ["orgs"],
    queryFn: () => apiFetch<OrgsListResponse>("/orgs"),
  });
  const org = orgs.data?.organizations.find((o) => o.slug === org_slug);

  const members = useQuery({
    enabled: !!org?.id,
    queryKey: ["members", org?.id],
    queryFn: () => apiFetch<MembersResponse>(`/orgs/${org!.id}/members`),
    staleTime: 10_000,
  });

  const friends = useQuery({
    enabled: !!org?.id,
    queryKey: ["friends", org?.id],
    queryFn: () => apiFetch<FriendsResponse>(`/orgs/${org!.id}/friends`),
    staleTime: 10_000,
  });

  const friendRequests = useQuery({
    enabled: !!org?.id,
    queryKey: ["friendRequests", org?.id],
    queryFn: () => apiFetch<FriendRequestsResponse>(`/orgs/${org!.id}/friends/requests`),
    staleTime: 5_000,
  });

  const sendFriendRequest = useMutation({
    mutationFn: async (userId: string) =>
      apiFetch<{ status: string; request_id: string }>(`/orgs/${org!.id}/friends/requests`, {
        method: "POST",
        body: JSON.stringify({ user_id: userId }),
      }),
    onSuccess: async () => {
      await qc.invalidateQueries({ queryKey: ["friendRequests", org?.id] });
      await qc.invalidateQueries({ queryKey: ["friends", org?.id] });
    },
  });

  const acceptFriendRequest = useMutation({
    mutationFn: async (requestId: string) =>
      apiFetch<{ status: string }>(`/orgs/${org!.id}/friends/requests/${requestId}/accept`, { method: "POST" }),
    onSuccess: async () => {
      await qc.invalidateQueries({ queryKey: ["friendRequests", org?.id] });
      await qc.invalidateQueries({ queryKey: ["friends", org?.id] });
    },
  });

  const declineFriendRequest = useMutation({
    mutationFn: async (requestId: string) =>
      apiFetch<{ status: string }>(`/orgs/${org!.id}/friends/requests/${requestId}/decline`, { method: "POST" }),
    onSuccess: async () => {
      await qc.invalidateQueries({ queryKey: ["friendRequests", org?.id] });
    },
  });

  const cancelFriendRequest = useMutation({
    mutationFn: async (requestId: string) =>
      apiFetch<{ status: string }>(`/orgs/${org!.id}/friends/requests/${requestId}/cancel`, { method: "POST" }),
    onSuccess: async () => {
      await qc.invalidateQueries({ queryKey: ["friendRequests", org?.id] });
    },
  });

  const removeFriend = useMutation({
    mutationFn: async (userId: string) =>
      apiFetch<{ status: string }>(`/orgs/${org!.id}/friends/${userId}`, { method: "DELETE" }),
    onSuccess: async () => {
      await qc.invalidateQueries({ queryKey: ["friendRequests", org?.id] });
      await qc.invalidateQueries({ queryKey: ["friends", org?.id] });
      await qc.invalidateQueries({ queryKey: ["dms", org?.id] });
    },
  });

  const openDm = useMutation({
    mutationFn: async (userId: string) =>
      apiFetch<{ channel_id: string }>(`/orgs/${org!.id}/dms/${userId}`, { method: "POST" }),
    onSuccess: async (res) => {
      await qc.invalidateQueries({ queryKey: ["dms", org?.id] });
      nav(`/app/${org!.slug}/channels/${res.channel_id}`);
    },
  });

  const friendById = useMemo(() => {
    const set = new Set((friends.data?.friends ?? []).map((f) => f.id));
    return set;
  }, [friends.data]);

  const incoming = useMemo(() => {
    return (friendRequests.data?.requests ?? []).filter((r) => r.addressee.id === me?.id);
  }, [friendRequests.data, me?.id]);

  const outgoing = useMemo(() => {
    return (friendRequests.data?.requests ?? []).filter((r) => r.requester.id === me?.id);
  }, [friendRequests.data, me?.id]);

  const filteredMembers = useMemo(() => {
    const raw = members.data?.members ?? [];
    const query = q.trim().toLowerCase();
    const list = raw
      .filter((m) => m.user_id !== me?.id)
      .filter((m) => {
        if (!query) return true;
        return (
          m.display_name.toLowerCase().includes(query) ||
          m.email.toLowerCase().includes(query) ||
          m.role.toLowerCase().includes(query)
        );
      });
    list.sort((a, b) => a.display_name.localeCompare(b.display_name));
    return list;
  }, [members.data, q, me?.id]);

  if (orgs.isLoading) return <div className="text-slate-300">Loading...</div>;
  if (!org) return <div className="text-slate-300">Org not found.</div>;

  return (
    <div className="grid gap-6 md:grid-cols-[280px_1fr]">
      <OrgSidebar org={org} />

      <section className="rounded-xl border border-slate-800 bg-slate-900/30 p-4">
        <div className="flex items-center justify-between gap-4">
          <div className="text-lg font-semibold">Friends</div>
          <div className="text-xs text-slate-400">{friends.data?.friends?.length ?? 0} friends</div>
        </div>

        <div className="mt-3 grid gap-4 md:grid-cols-2">
          <div className="rounded-lg border border-slate-800 bg-slate-950/20 p-3">
            <div className="text-xs font-semibold text-slate-400">Incoming requests</div>
            <div className="mt-2 space-y-2">
              {incoming.length ? (
                incoming.map((r) => (
                  <div key={r.id} className="flex items-center justify-between gap-2 rounded-md px-2 py-2">
                    <div className="min-w-0">
                      <div className="truncate text-sm text-slate-200">{r.requester.display_name}</div>
                      <div className="truncate text-xs text-slate-500">{r.requester.email}</div>
                    </div>
                    <div className="flex shrink-0 gap-2">
                      <Button
                        className="bg-indigo-600 hover:bg-indigo-500"
                        disabled={acceptFriendRequest.isPending}
                        onClick={() => acceptFriendRequest.mutate(r.id)}
                        type="button"
                      >
                        Accept
                      </Button>
                      <Button
                        className="bg-slate-800 hover:bg-slate-700"
                        disabled={declineFriendRequest.isPending}
                        onClick={() => declineFriendRequest.mutate(r.id)}
                        type="button"
                      >
                        Decline
                      </Button>
                    </div>
                  </div>
                ))
              ) : (
                <div className="text-sm text-slate-400">No incoming requests.</div>
              )}
            </div>
          </div>

          <div className="rounded-lg border border-slate-800 bg-slate-950/20 p-3">
            <div className="text-xs font-semibold text-slate-400">Outgoing requests</div>
            <div className="mt-2 space-y-2">
              {outgoing.length ? (
                outgoing.map((r) => (
                  <div key={r.id} className="flex items-center justify-between gap-2 rounded-md px-2 py-2">
                    <div className="min-w-0">
                      <div className="truncate text-sm text-slate-200">{r.addressee.display_name}</div>
                      <div className="truncate text-xs text-slate-500">{r.addressee.email}</div>
                    </div>
                    <Button
                      className="bg-slate-800 hover:bg-slate-700"
                      disabled={cancelFriendRequest.isPending}
                      onClick={() => cancelFriendRequest.mutate(r.id)}
                      type="button"
                    >
                      Cancel
                    </Button>
                  </div>
                ))
              ) : (
                <div className="text-sm text-slate-400">No outgoing requests.</div>
              )}
            </div>
          </div>
        </div>

        <div className="mt-5 rounded-lg border border-slate-800 bg-slate-950/20 p-3">
          <div className="text-xs font-semibold text-slate-400">Find people</div>
          <div className="mt-2">
            <Input value={q} onChange={(e) => setQ(e.target.value)} placeholder="Search by name or email..." />
          </div>

          <div className="mt-3 space-y-2">
            {filteredMembers.map((m) => {
              const isFriend = friendById.has(m.user_id);
              const incomingReq = incoming.find((r) => r.requester.id === m.user_id);
              const outgoingReq = outgoing.find((r) => r.addressee.id === m.user_id);

              return (
                <div key={m.user_id} className="flex items-center justify-between gap-3 rounded-md px-2 py-2">
                  <div className="min-w-0">
                    <div className="truncate text-sm text-slate-200">{m.display_name}</div>
                    <div className="truncate text-xs text-slate-500">{m.email}</div>
                  </div>
                  <div className="flex shrink-0 items-center gap-2">
                    {isFriend ? (
                      <>
                        <Button
                          className="bg-slate-800 hover:bg-slate-700"
                          disabled={openDm.isPending}
                          onClick={() => openDm.mutate(m.user_id)}
                          type="button"
                        >
                          Message
                        </Button>
                        <Button
                          className="bg-red-600 hover:bg-red-500"
                          disabled={removeFriend.isPending}
                          onClick={() => removeFriend.mutate(m.user_id)}
                          type="button"
                        >
                          Remove
                        </Button>
                      </>
                    ) : incomingReq ? (
                      <>
                        <Button
                          className="bg-indigo-600 hover:bg-indigo-500"
                          disabled={acceptFriendRequest.isPending}
                          onClick={() => acceptFriendRequest.mutate(incomingReq.id)}
                          type="button"
                        >
                          Accept
                        </Button>
                        <Button
                          className="bg-slate-800 hover:bg-slate-700"
                          disabled={declineFriendRequest.isPending}
                          onClick={() => declineFriendRequest.mutate(incomingReq.id)}
                          type="button"
                        >
                          Decline
                        </Button>
                      </>
                    ) : outgoingReq ? (
                      <Button
                        className="bg-slate-800 hover:bg-slate-700"
                        disabled={cancelFriendRequest.isPending}
                        onClick={() => cancelFriendRequest.mutate(outgoingReq.id)}
                        type="button"
                      >
                        Cancel
                      </Button>
                    ) : (
                      <Button
                        className="bg-slate-800 hover:bg-slate-700"
                        disabled={sendFriendRequest.isPending}
                        onClick={() => sendFriendRequest.mutate(m.user_id)}
                        type="button"
                      >
                        Add
                      </Button>
                    )}
                  </div>
                </div>
              );
            })}

            {members.isLoading ? <div className="text-sm text-slate-400">Loading members...</div> : null}
          </div>
        </div>
      </section>
    </div>
  );
}
