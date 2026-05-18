import { useEffect, useMemo, useRef, useState } from "react";
import { Link, useNavigate, useParams } from "react-router-dom";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { apiFetch } from "../api/client";
import type {
  ChannelsResponse,
  Channel,
  ChannelSearchResponse,
  ListMessagesResponse,
  MessageAttachment,
  MessageReaction,
  MediaRoom,
  Member,
  MembersResponse,
  Message,
  OrgsListResponse,
  PinsResponse,
  ThreadWithMessagesResponse,
  ThreadsListResponse,
} from "../api/types";
import { createRealtimeClient } from "../realtime/ws";
import { Button } from "../components/Button";
import { useAuthStore } from "../state/auth";
import { useBrandingStore } from "../state/branding";
import { OrgSidebar } from "../components/OrgSidebar";
import { Avatar } from "../components/Avatar";
import { TextArea } from "../components/TextArea";

type SendMessageResponse = Message;

type PresenceStatus = "online" | "offline";

export function ChannelPage() {
  const { org_slug, channel_id } = useParams();
  const nav = useNavigate();
  const qc = useQueryClient();
  const me = useAuthStore((s) => s.user);
  const loadOrgBranding = useBrandingStore((s) => s.loadOrgBranding);
  const uiMode = useBrandingStore((s) => s.branding?.ui_mode ?? "work");

  const [text, setText] = useState("");
  const [connected, setConnected] = useState(false);
  const [emojiOpen, setEmojiOpen] = useState(false);
  const [reactionPickerFor, setReactionPickerFor] = useState<string | null>(null);
  const [pendingAttachments, setPendingAttachments] = useState<
    { filename: string; content_type?: string; data_url: string }[]
  >([]);

  const typingTimeout = useRef<number | null>(null);
  const reactionPickerRef = useRef<HTMLDivElement | null>(null);
  const fileInputRef = useRef<HTMLInputElement | null>(null);
  const textAreaRef = useRef<HTMLTextAreaElement | null>(null);
  const [typingByUser, setTypingByUser] = useState<Record<string, number>>({});
  const [presenceByUser, setPresenceByUser] = useState<Record<string, PresenceStatus>>({});
  const [workSearchOpen, setWorkSearchOpen] = useState(false);
  const [workSearch, setWorkSearch] = useState("");
  const [workPane, setWorkPane] = useState<"search" | "pins" | "threads" | null>(null);
  const [activeThreadId, setActiveThreadId] = useState<string | null>(null);
  const [threadDraft, setThreadDraft] = useState("");
  const [newThreadDraft, setNewThreadDraft] = useState("");

  const QUICK_REACTIONS = useMemo(
    () => ["\u{1F44D}", "\u{2764}\u{FE0F}", "\u{1F602}", "\u{1F62E}", "\u{1F622}", "\u{1F621}"],
    [],
  );

  useEffect(() => {
    if (!reactionPickerFor) return;
    function onDown(e: MouseEvent) {
      const el = reactionPickerRef.current;
      if (!el) return;
      if (e.target instanceof Node && el.contains(e.target)) return;
      setReactionPickerFor(null);
    }
    function onKey(e: KeyboardEvent) {
      if (e.key === "Escape") setReactionPickerFor(null);
    }
    document.addEventListener("mousedown", onDown);
    document.addEventListener("keydown", onKey);
    return () => {
      document.removeEventListener("mousedown", onDown);
      document.removeEventListener("keydown", onKey);
    };
  }, [reactionPickerFor]);

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
  const channelFromList = channels.data?.channels.find((c) => c.id === channel_id);

  const channelInfo = useQuery({
    enabled: !!channel_id,
    queryKey: ["channel", channel_id],
    queryFn: () => apiFetch<Channel>(`/channels/${channel_id}`),
    staleTime: 10_000,
  });

  const channel = channelFromList ?? channelInfo.data ?? null;

  const members = useQuery({
    enabled: !!org?.id,
    queryKey: ["members", org?.id],
    queryFn: () => apiFetch<MembersResponse>(`/orgs/${org!.id}/members`),
    staleTime: 10_000,
  });

  const myRole = members.data?.members.find((m) => m.user_id === me?.id)?.role ?? null;
  const canSeeAdmin = myRole === "owner" || myRole === "admin";

  const memberById = useMemo(() => {
    const map = new Map<string, Member>();
    for (const m of members.data?.members ?? []) map.set(m.user_id, m);
    return map;
  }, [members.data]);

  const channelCreatorId = (channel as unknown as { created_by?: string | null } | null)?.created_by ?? null;
  const canManageChannel =
    !!channel_id &&
    channel?.kind !== "dm" &&
    (myRole === "owner" || (!!me?.id && !!channelCreatorId && channelCreatorId === me.id));

  const [editingChannelName, setEditingChannelName] = useState(false);
  const [channelNameDraft, setChannelNameDraft] = useState("");

  useEffect(() => {
    setEditingChannelName(false);
    setChannelNameDraft(channel?.name ?? "");
  }, [channel?.id, channel?.name]);

  const updateChannel = useMutation({
    mutationFn: async (input: { name: string }) => {
      return apiFetch<{ status: string }>(`/channels/${channel_id}`, {
        method: "PATCH",
        body: JSON.stringify({ name: input.name }),
      });
    },
    onSuccess: async () => {
      setEditingChannelName(false);
      await qc.invalidateQueries({ queryKey: ["channels", org?.id] });
      await qc.invalidateQueries({ queryKey: ["channel", channel_id] });
    },
  });

  const deleteChannel = useMutation({
    mutationFn: async () => {
      return apiFetch<{ status: string }>(`/channels/${channel_id}`, { method: "DELETE" });
    },
    onSuccess: async () => {
      await qc.invalidateQueries({ queryKey: ["channels", org?.id] });
      nav(`/app/${org!.slug}`);
    },
  });

  const messages = useQuery({
    enabled: !!channel_id,
    queryKey: ["messages", channel_id],
    queryFn: () => apiFetch<ListMessagesResponse>(`/channels/${channel_id}/messages?limit=50`),
    staleTime: 5_000,
  });
  const bottomRef = useRef<HTMLDivElement | null>(null);

  const rt = useMemo(() => {
    return createRealtimeClient({
      onOpen: () => setConnected(true),
      onClose: () => setConnected(false),
      onEvent: (evt) => {
        const e = evt as any;

        if (e?.type === "message.created" && e.channel_id === channel_id) {
          const msg = e.message as Message | undefined;
          if (msg && typeof msg.id === "string") {
            qc.setQueryData<ListMessagesResponse>(["messages", channel_id], (prev) => {
              const existing = prev?.messages ?? [];
              if (existing.some((m) => m.id === msg.id)) return prev ?? { messages: [msg], next_cursor: null };
              // Replace optimistic echo from the local sender if possible.
              const filtered =
                me?.id && msg.sender_id === me.id
                  ? existing.filter((m) => !(m.id.startsWith("optimistic-") && m.body === msg.body))
                  : existing;
              return { ...(prev ?? {}), messages: [msg, ...filtered] };
            });
          } else {
            qc.invalidateQueries({ queryKey: ["messages", channel_id] });
          }
        }

        if (e?.type === "typing.started" && e.channel_id === channel_id) {
          const userId = String(e.user_id ?? "");
          if (!userId) return;
          setTypingByUser((prev) => ({ ...prev, [userId]: Date.now() }));
          return;
        }

        if (e?.type === "typing.stopped" && e.channel_id === channel_id) {
          const userId = String(e.user_id ?? "");
          if (!userId) return;
          setTypingByUser((prev) => {
            const next = { ...prev };
            delete next[userId];
            return next;
          });
          return;
        }

        if (e?.type === "presence.changed" && org?.id && e.organization_id === org.id) {
          const userId = String(e.user_id ?? "");
          const status = String(e.status ?? "") as PresenceStatus;
          if (!userId || (status !== "online" && status !== "offline")) return;
          setPresenceByUser((prev) => ({ ...prev, [userId]: status }));
        }

        if (e?.type === "thread.reply.created" && e.channel_id === channel_id) {
          qc.invalidateQueries({ queryKey: ["threads", channel_id] });
          qc.invalidateQueries({ queryKey: ["thread", String(e.thread_id ?? "")] });
          return;
        }

        if (e?.type === "channel.pins.changed" && e.channel_id === channel_id) {
          qc.invalidateQueries({ queryKey: ["pins", channel_id] });
          return;
        }
      },
    });
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [channel_id, qc, org?.id, me?.id]);

  useEffect(() => {
    rt.start();
    return () => rt.stop();
  }, [rt]);

  useEffect(() => {
    if (!channel_id) return;
    rt.send({ type: "channel.subscribe", channel_id });
    return () => {
      rt.send({ type: "channel.unsubscribe", channel_id });
    };
  }, [channel_id, rt]);

  // Periodically clean typing indicators (server TTL is short, but we keep UI resilient).
  useEffect(() => {
    const id = window.setInterval(() => {
      const cutoff = Date.now() - 7_000;
      setTypingByUser((prev) => {
        const next: Record<string, number> = {};
        for (const [k, ts] of Object.entries(prev)) {
          if (ts >= cutoff) next[k] = ts;
        }
        return next;
      });
    }, 1500);
    return () => window.clearInterval(id);
  }, []);

  // Keep viewport pinned to newest messages.
  useEffect(() => {
    bottomRef.current?.scrollIntoView({ block: "end" });
  }, [messages.data?.messages?.length]);

  const send = useMutation({
    mutationFn: async (input: { body?: string; attachments?: { filename: string; content_type?: string; data_url: string }[] }) =>
      apiFetch<SendMessageResponse>(`/channels/${channel_id}/messages`, {
        method: "POST",
        body: JSON.stringify(input),
      }),
    onMutate: async (input) => {
      await qc.cancelQueries({ queryKey: ["messages", channel_id] });
      const prev = qc.getQueryData<ListMessagesResponse>(["messages", channel_id]);
      const optimistic: Message = {
        id: `optimistic-${Date.now()}`,
        organization_id: org?.id ?? "",
        channel_id: channel_id!,
        sender_id: "me",
        body: input.body ?? null,
        kind: input.attachments?.length ? "attachment" : "text",
        attachments: (input.attachments ?? []).map((a, idx) => ({
          id: `optimistic-att-${idx}`,
          filename: a.filename,
          content_type: a.content_type ?? null,
          size_bytes: a.data_url.length,
          download_url: a.data_url,
          created_at: new Date().toISOString(),
        })) as MessageAttachment[],
        reactions: [] as MessageReaction[],
        created_at: new Date().toISOString(),
      };
      qc.setQueryData<ListMessagesResponse>(["messages", channel_id], {
        messages: [optimistic, ...(prev?.messages ?? [])],
        next_cursor: prev?.next_cursor ?? null,
      });
      return { prev };
    },
    onError: (_err, _body, ctx) => {
      if (ctx?.prev) qc.setQueryData(["messages", channel_id], ctx.prev);
    },
    onSettled: () => {
      qc.invalidateQueries({ queryKey: ["messages", channel_id] });
    },
  });

  const addReaction = useMutation({
    mutationFn: async (input: { messageId: string; emoji: string }) =>
      apiFetch<{ status: string }>(`/messages/${input.messageId}/reactions`, {
        method: "POST",
        body: JSON.stringify({ emoji: input.emoji }),
      }),
    onSuccess: (_res, input) => {
      qc.setQueryData<ListMessagesResponse>(["messages", channel_id], (prev) => {
        const msgs = prev?.messages ?? [];
        return {
          ...(prev ?? { next_cursor: null }),
          messages: msgs.map((m) => {
            if (m.id !== input.messageId) return m;
            const existing = m.reactions ?? [];
            const hit = existing.find((r) => r.emoji === input.emoji);
            const next = hit
              ? existing.map((r) => (r.emoji === input.emoji ? { ...r, count: r.count + 1, reacted_by_me: true } : r))
              : [...existing, { emoji: input.emoji, count: 1, reacted_by_me: true }];
            return { ...m, reactions: next };
          }),
        };
      });
    },
  });

  const removeReaction = useMutation({
    mutationFn: async (input: { messageId: string; emoji: string }) =>
      apiFetch<{ status: string }>(`/messages/${input.messageId}/reactions/${encodeURIComponent(input.emoji)}`, {
        method: "DELETE",
      }),
    onSuccess: (_res, input) => {
      qc.setQueryData<ListMessagesResponse>(["messages", channel_id], (prev) => {
        const msgs = prev?.messages ?? [];
        return {
          ...(prev ?? { next_cursor: null }),
          messages: msgs.map((m) => {
            if (m.id !== input.messageId) return m;
            const existing = m.reactions ?? [];
            const next = existing
              .map((r) =>
                r.emoji === input.emoji ? { ...r, count: Math.max(0, r.count - 1), reacted_by_me: false } : r,
              )
              .filter((r) => r.count > 0);
            return { ...m, reactions: next };
          }),
        };
      });
    },
  });

  const createMeeting = useMutation({
    mutationFn: async () => {
      return apiFetch<MediaRoom>(`/orgs/${org!.id}/media/rooms`, {
        method: "POST",
        body: JSON.stringify({
          kind: "meeting",
          channel_id: channel_id,
          name: `Meeting - ${channel?.name ?? "channel"}`,
        }),
      });
    },
    onSuccess: (room) => {
      nav(`/app/${org!.slug}/voice/${room.id}`);
    },
  });

  function onTypingChange(v: string) {
    setText(v);
    if (!channel_id) return;
    rt.send({ type: "typing.start", channel_id });
    if (typingTimeout.current) window.clearTimeout(typingTimeout.current);
    typingTimeout.current = window.setTimeout(() => {
      rt.send({ type: "typing.stop", channel_id });
    }, 1200);
  }

  function sanitizeOutgoingBody(raw: string) {
    const trimmed = raw.trim();
    // Work around an observed UI bug where a trailing 🙂 is appended on send.
    // Keep a message that is exactly "🙂" intact.
    const withoutTrailingSmile = trimmed.replace(/\s*🙂$/, "");
    if (withoutTrailingSmile !== trimmed && withoutTrailingSmile.trim().length > 0) return withoutTrailingSmile;
    return trimmed;
  }

  async function onSubmit(e: React.FormEvent) {
    e.preventDefault();
    const body = sanitizeOutgoingBody(text);
    if (!body && pendingAttachments.length === 0) return;
    setText("");
    if (textAreaRef.current) textAreaRef.current.style.height = "";
    const attachments = pendingAttachments.length ? pendingAttachments : undefined;
    setPendingAttachments([]);
    setEmojiOpen(false);
    send.mutate({ body: body || undefined, attachments });
  }

  const typingUsers = Object.keys(typingByUser)
    .filter((id) => id !== "me")
    .slice(0, 3);
  const typingText =
    typingUsers.length === 0
      ? null
      : typingUsers
          .map((id) => memberById.get(id)?.display_name ?? id.slice(0, 8))
          .join(", ") + (typingUsers.length === 1 ? " is typing…" : " are typing…");

  const orderedMessages = useMemo(() => {
    const list = messages.data?.messages ?? [];
    // API returns newest-first; UI should show newest at bottom.
    return [...list].reverse();
  }, [messages.data]);

  const searchResults = useQuery({
    enabled: !!channel_id && uiMode === "work" && workPane === "search" && workSearchOpen && !!workSearch.trim(),
    queryKey: ["channel-search", channel_id, workSearch.trim()],
    queryFn: () => apiFetch<ChannelSearchResponse>(`/channels/${channel_id}/search?q=${encodeURIComponent(workSearch.trim())}&limit=50`),
    staleTime: 2_000,
  });

  const pins = useQuery({
    enabled: !!channel_id && uiMode === "work",
    queryKey: ["pins", channel_id],
    queryFn: () => apiFetch<PinsResponse>(`/channels/${channel_id}/pins`),
    staleTime: 2_000,
  });

  const threads = useQuery({
    enabled: !!channel_id && uiMode === "work" && workPane === "threads" && !activeThreadId,
    queryKey: ["threads", channel_id],
    queryFn: () => apiFetch<ThreadsListResponse>(`/channels/${channel_id}/threads`),
    staleTime: 2_000,
  });

  const thread = useQuery({
    enabled: !!activeThreadId,
    queryKey: ["thread", activeThreadId],
    queryFn: () => apiFetch<ThreadWithMessagesResponse>(`/threads/${activeThreadId}`),
    staleTime: 2_000,
  });

  const pinnedIds = useMemo(() => {
    const set = new Set<string>();
    for (const p of pins.data?.pins ?? []) set.add(p.message_id);
    return set;
  }, [pins.data]);

  const visibleMessages = useMemo(() => {
    if (uiMode !== "work") return orderedMessages;
    if (workPane !== "search") return orderedMessages;
    const q = workSearch.trim().toLowerCase();
    if (!q) return orderedMessages;

    const fromApi = searchResults.data?.messages ?? null;
    if (!fromApi) return orderedMessages;

    const sorted = [...fromApi].sort((a, b) => {
      const da = parseApiOffsetDateTime(a.created_at)?.getTime() ?? 0;
      const db = parseApiOffsetDateTime(b.created_at)?.getTime() ?? 0;
      if (da !== db) return da - db;
      return String(a.id).localeCompare(String(b.id));
    });
    return sorted;
  }, [orderedMessages, uiMode, workPane, workSearch, searchResults.data]);

  const createThread = useMutation({
    mutationFn: async (input: { body?: string; root_message_id?: string }) => {
      return apiFetch<{ id: string } | any>(`/channels/${channel_id}/threads`, {
        method: "POST",
        body: JSON.stringify(input),
      });
    },
    onSuccess: async (res: any) => {
      const tid = String(res?.id ?? "");
      if (tid) {
        setWorkPane("threads");
        setActiveThreadId(tid);
        setThreadDraft("");
        await qc.invalidateQueries({ queryKey: ["threads", channel_id] });
      }
    },
  });

  const replyToThread = useMutation({
    mutationFn: async (input: { thread_id: string; body: string }) => {
      return apiFetch<{ message_id: string }>(`/threads/${input.thread_id}/replies`, {
        method: "POST",
        body: JSON.stringify({ body: input.body }),
      });
    },
    onSuccess: async (_res, vars) => {
      setThreadDraft("");
      await qc.invalidateQueries({ queryKey: ["thread", vars.thread_id] });
      await qc.invalidateQueries({ queryKey: ["threads", channel_id] });
    },
  });

  const pin = useMutation({
    mutationFn: async (messageId: string) => {
      return apiFetch<{ status: string }>(`/channels/${channel_id}/pins`, {
        method: "POST",
        body: JSON.stringify({ message_id: messageId }),
      });
    },
    onSuccess: async () => {
      await qc.invalidateQueries({ queryKey: ["pins", channel_id] });
    },
  });

  const unpin = useMutation({
    mutationFn: async (messageId: string) => {
      return apiFetch<{ status: string }>(`/channels/${channel_id}/pins/${messageId}`, { method: "DELETE" });
    },
    onSuccess: async () => {
      await qc.invalidateQueries({ queryKey: ["pins", channel_id] });
    },
  });

  const channelTitle = channel?.kind === "dm" ? `Direct message` : `# ${channel?.name ?? ""}`;

  if (orgs.isLoading) return <div className="text-slate-300">Loading...</div>;
  if (!org) return <div className="text-slate-300">Org not found.</div>;
  if (!channel) return <div className="text-slate-300">Channel not found.</div>;

  return (
    <div className="grid gap-6 md:grid-cols-[280px_1fr]">
      <OrgSidebar org={org} activeChannelId={channel_id} presenceByUser={presenceByUser} />

      <section className="rounded-xl border border-slate-800 bg-slate-900/30 p-4">
        <div className="flex items-center justify-between">
          <div className="flex items-center gap-2">
            {!editingChannelName ? (
              <div className="text-lg font-semibold">{channelTitle}</div>
            ) : (
              <div className="flex items-center gap-2">
                <input
                  className="w-[240px] rounded-md border border-slate-700 bg-slate-950/40 px-2 py-1 text-sm text-slate-100 outline-none focus:border-indigo-500"
                  value={channelNameDraft}
                  onChange={(e) => setChannelNameDraft(e.target.value)}
                  onKeyDown={(e) => {
                    if (e.key === "Escape") setEditingChannelName(false);
                    if (e.key === "Enter") {
                      e.preventDefault();
                      const next = channelNameDraft.trim();
                      if (!next) return;
                      updateChannel.mutate({ name: next });
                    }
                  }}
                  aria-label="Channel name"
                />
                <Button
                  className="bg-indigo-600 px-2 py-1 text-xs hover:bg-indigo-500"
                  disabled={updateChannel.isPending || !channelNameDraft.trim()}
                  onClick={() => updateChannel.mutate({ name: channelNameDraft.trim() })}
                  type="button"
                >
                  Save
                </Button>
                <Button
                  className="bg-slate-800 px-2 py-1 text-xs hover:bg-slate-700"
                  disabled={updateChannel.isPending}
                  onClick={() => setEditingChannelName(false)}
                  type="button"
                >
                  Cancel
                </Button>
              </div>
            )}
            {uiMode === "work" ? (
              <div className="ml-2 flex items-center gap-1">
                <button
                  className={`rounded-md px-2 py-1 text-xs ${
                    workPane === "search" ? "bg-slate-800 text-white" : "bg-slate-900 text-slate-200 hover:bg-slate-800"
                  }`}
                  onClick={() => {
                    if (workPane === "search") {
                      setWorkSearch("");
                      setWorkPane(null);
                      setWorkSearchOpen(false);
                      return;
                    }
                    setWorkSearchOpen(true);
                    setWorkPane("search");
                  }}
                  type="button"
                  title="Search"
                >
                  Search
                </button>
                <button
                  className={`rounded-md px-2 py-1 text-xs ${
                    workPane === "pins" ? "bg-slate-800 text-white" : "bg-slate-900 text-slate-200 hover:bg-slate-800"
                  }`}
                  onClick={() => {
                    setActiveThreadId(null);
                    setThreadDraft("");
                    setWorkPane((p) => (p === "pins" ? null : "pins"));
                  }}
                  type="button"
                  title="Pins"
                >
                  Pins
                </button>
                <button
                  className={`rounded-md px-2 py-1 text-xs ${
                    workPane === "threads"
                      ? "bg-slate-800 text-white"
                      : "bg-slate-900 text-slate-200 hover:bg-slate-800"
                  }`}
                  onClick={() => {
                    setThreadDraft("");
                    setWorkPane((p) => {
                      const next = p === "threads" ? null : "threads";
                      if (next === null) setActiveThreadId(null);
                      return next;
                    });
                  }}
                  type="button"
                  title="Threads"
                >
                  Threads
                </button>
              </div>
            ) : null}
            {uiMode === "play" ? (
              <button
                className="ml-2 inline-flex items-center gap-2 rounded-md bg-indigo-600 px-3 py-2 text-xs font-semibold text-white hover:bg-indigo-500 disabled:opacity-50"
                disabled={createMeeting.isPending}
                onClick={() => createMeeting.mutate()}
                title="Start meeting"
                type="button"
              >
                ðŸ“¹ <span>Start meeting</span>
              </button>
            ) : null}
            <button
              className={`grid h-8 w-8 place-items-center rounded-md bg-slate-900 text-slate-200 hover:bg-slate-800 disabled:opacity-50 ${
                uiMode === "play" ? "hidden" : ""
              }`}
              disabled={createMeeting.isPending}
              onClick={() => createMeeting.mutate()}
              title="Start meeting"
              type="button"
            >
              📹
            </button>
            {canManageChannel && !editingChannelName ? (
              <>
                <button
                  className="grid h-8 w-8 place-items-center rounded-md bg-slate-900 text-slate-200 hover:bg-slate-800"
                  onClick={() => setEditingChannelName(true)}
                  title="Rename channel"
                  type="button"
                >
                  {"\u{270F}\u{FE0F}"}
                </button>
                <button
                  className="grid h-8 w-8 place-items-center rounded-md bg-slate-900 text-red-300 hover:bg-red-950/40 hover:text-red-200 disabled:opacity-50"
                  disabled={deleteChannel.isPending}
                  onClick={() => {
                    const ok = confirm(`Delete channel "${channel?.name ?? ""}"? This cannot be undone.`);
                    if (!ok) return;
                    deleteChannel.mutate();
                  }}
                  title="Delete channel"
                  type="button"
                >
                  {"\u{1F5D1}\u{FE0F}"}
                </button>
              </>
            ) : null}
          </div>
          <div className="text-xs text-slate-400">{connected ? "realtime online" : "realtime offline"}</div>
        </div>

        {uiMode === "work" && workPane === "search" && workSearchOpen ? (
          <div className="mt-3 flex items-center gap-2 rounded-lg border border-slate-800 bg-slate-950/30 px-3 py-2">
            <input
              className="w-full rounded-md border border-slate-800 bg-slate-900 px-3 py-2 text-sm text-slate-200 outline-none focus:border-indigo-500"
              placeholder="Search messages in this channel..."
              value={workSearch}
              onChange={(e) => setWorkSearch(e.target.value)}
            />
            <button
              className="rounded-md bg-slate-900 px-3 py-2 text-xs text-slate-200 hover:bg-slate-800"
              onClick={() => {
                setWorkSearch("");
                setWorkPane(null);
                setWorkSearchOpen(false);
              }}
              type="button"
              title="Close search"
            >
              Close
            </button>
          </div>
        ) : null}

        {uiMode === "work" && workPane && workPane !== "search" ? (
          <div className="mt-3 rounded-lg border border-slate-800 bg-slate-950/30 px-3 py-2 text-sm text-slate-200">
            <div className="flex items-center justify-between">
              <div className="font-semibold capitalize">
                {workPane === "threads" ? (activeThreadId ? "Thread" : "Threads") : "Pins"}
              </div>
              <div className="flex items-center gap-2">
                {workPane === "threads" && activeThreadId ? (
                  <button
                    className="rounded-md bg-slate-900 px-3 py-2 text-xs text-slate-200 hover:bg-slate-800"
                    onClick={() => {
                      setActiveThreadId(null);
                      setThreadDraft("");
                    }}
                    type="button"
                  >
                    Back
                  </button>
                ) : null}
                <button
                  className="rounded-md bg-slate-900 px-3 py-2 text-xs text-slate-200 hover:bg-slate-800"
                  onClick={() => {
                    setWorkPane(null);
                    setActiveThreadId(null);
                    setThreadDraft("");
                  }}
                  type="button"
                >
                  Close
                </button>
              </div>
            </div>

            {workPane === "pins" ? (
              <div className="mt-2 space-y-2">
                {pins.isLoading ? <div className="text-xs text-slate-400">Loading pins...</div> : null}
                {pins.isError ? <div className="text-xs text-rose-300">Failed to load pins.</div> : null}
                {!pins.isLoading && !pins.isError && (pins.data?.pins ?? []).length === 0 ? (
                  <div className="text-xs text-slate-400">No pinned messages yet.</div>
                ) : null}
                {(pins.data?.pins ?? []).map((p) => {
                  const msg = p.message;
                  const senderName = memberById.get(msg.sender_id)?.display_name ?? msg.sender_id.slice(0, 8);
                  const pinnedByName = memberById.get(p.pinned_by)?.display_name ?? p.pinned_by.slice(0, 8);
                  return (
                    <div key={p.message_id} className="rounded-lg border border-slate-800 bg-slate-950/20 p-2">
                      <div className="flex items-start justify-between gap-3">
                        <div className="min-w-0">
                          <div className="text-xs text-slate-400">
                            {senderName} · {formatDateTime(msg.created_at)} · pinned by {pinnedByName}
                          </div>
                          <div className="mt-1 truncate text-sm text-slate-200">{msg.body ?? "(no text)"}</div>
                        </div>
                        <button
                          className="shrink-0 rounded-md bg-slate-900 px-2 py-1 text-xs text-slate-200 hover:bg-slate-800"
                          onClick={() => unpin.mutate(p.message_id)}
                          type="button"
                          title="Unpin"
                        >
                          Unpin
                        </button>
                      </div>
                    </div>
                  );
                })}
              </div>
            ) : null}

            {workPane === "threads" ? (
              <div className="mt-2 space-y-3">
                {activeThreadId ? (
                  <>
                    {thread.isLoading ? <div className="text-xs text-slate-400">Loading thread...</div> : null}
                    {thread.isError ? <div className="text-xs text-rose-300">Failed to load thread.</div> : null}
                    {thread.data ? (
                      <div className="space-y-3">
                        <div className="rounded-lg border border-slate-800 bg-slate-950/20 p-2">
                          <div className="text-xs text-slate-400">Root</div>
                          <div className="mt-1 text-sm text-slate-200">{thread.data.root?.body ?? "(no text)"}</div>
                        </div>
                        <div className="space-y-2">
                          <div className="text-xs text-slate-400">Replies</div>
                          {(thread.data.replies ?? []).length === 0 ? (
                            <div className="text-xs text-slate-500">No replies yet.</div>
                          ) : null}
                          {(thread.data.replies ?? []).map((m) => {
                            const senderName = memberById.get(m.sender_id)?.display_name ?? m.sender_id.slice(0, 8);
                            return (
                              <div key={m.id} className="rounded-lg border border-slate-800 bg-slate-950/10 p-2">
                                <div className="text-xs text-slate-400">
                                  {senderName} · {formatDateTime(m.created_at)}
                                </div>
                                <div className="mt-1 text-sm text-slate-200">{m.body ?? ""}</div>
                              </div>
                            );
                          })}
                        </div>

                        <form
                          className="flex items-end gap-2"
                          onSubmit={(e) => {
                            e.preventDefault();
                            const body = threadDraft.trim();
                            if (!body || !activeThreadId) return;
                            replyToThread.mutate({ thread_id: activeThreadId, body });
                          }}
                        >
                          <TextArea
                            rows={2}
                            value={threadDraft}
                            placeholder="Reply in thread..."
                            onChange={(e) => setThreadDraft(e.target.value)}
                            className="min-h-[60px] flex-1 resize-none rounded-md border border-slate-800 bg-slate-900 px-3 py-2 text-sm text-slate-200 outline-none focus:border-indigo-500"
                          />
                          <Button
                            className="bg-indigo-600 px-3 py-2 text-xs hover:bg-indigo-500"
                            disabled={replyToThread.isPending || !threadDraft.trim()}
                            type="submit"
                          >
                            Reply
                          </Button>
                        </form>
                      </div>
                    ) : null}
                  </>
                ) : (
                  <>
                    <form
                      className="flex items-end gap-2"
                      onSubmit={(e) => {
                        e.preventDefault();
                        const body = newThreadDraft.trim();
                        if (!body) return;
                        createThread.mutate({ body });
                        setNewThreadDraft("");
                      }}
                    >
                      <TextArea
                        rows={2}
                        value={newThreadDraft}
                        placeholder="Start a new thread..."
                        onChange={(e) => setNewThreadDraft(e.target.value)}
                        className="min-h-[60px] flex-1 resize-none rounded-md border border-slate-800 bg-slate-900 px-3 py-2 text-sm text-slate-200 outline-none focus:border-indigo-500"
                      />
                      <Button
                        className="bg-indigo-600 px-3 py-2 text-xs hover:bg-indigo-500"
                        disabled={createThread.isPending || !newThreadDraft.trim()}
                        type="submit"
                      >
                        Create
                      </Button>
                    </form>

                    {threads.isLoading ? <div className="text-xs text-slate-400">Loading threads...</div> : null}
                    {threads.isError ? <div className="text-xs text-rose-300">Failed to load threads.</div> : null}
                    {!threads.isLoading && !threads.isError && (threads.data?.threads ?? []).length === 0 ? (
                      <div className="text-xs text-slate-400">No threads yet.</div>
                    ) : null}
                    <div className="space-y-2">
                      {(threads.data?.threads ?? []).map((t) => {
                        const root = t.root;
                        const senderName = root ? memberById.get(root.sender_id)?.display_name ?? root.sender_id.slice(0, 8) : "Unknown";
                        return (
                          <button
                            key={t.thread.id}
                            className="w-full rounded-lg border border-slate-800 bg-slate-950/10 p-2 text-left hover:bg-slate-900/40"
                            onClick={() => setActiveThreadId(t.thread.id)}
                            type="button"
                          >
                            <div className="flex items-center justify-between gap-2">
                              <div className="min-w-0">
                                <div className="text-xs text-slate-400">
                                  {senderName} · {root ? formatDateTime(root.created_at) : ""}
                                </div>
                                <div className="mt-1 truncate text-sm text-slate-200">{root?.body ?? "(no text)"}</div>
                              </div>
                              <div className="shrink-0 rounded-full bg-slate-900 px-2 py-0.5 text-xs text-slate-200">
                                {t.reply_count}
                              </div>
                            </div>
                          </button>
                        );
                      })}
                    </div>
                  </>
                )}
              </div>
            ) : null}
          </div>
        ) : null}

        <div className="mt-4 h-[60vh] overflow-auto rounded-lg border border-slate-800 bg-slate-950/30 p-3">
          <div className="space-y-3">
            {visibleMessages.map((m) => {
              const isMe = m.sender_id === (me?.id ?? "me") || m.sender_id === "me";
              const senderName = memberById.get(m.sender_id)?.display_name ?? m.sender_id.slice(0, 8);
              const isPinned = uiMode === "work" && pinnedIds.has(m.id);
              const threadId = m.thread_id ?? null;
              return (
                <div key={m.id} className={`group flex ${isMe ? "justify-end" : "justify-start"}`}>
                  <div className={`flex max-w-[88%] items-end gap-2 ${isMe ? "flex-row-reverse" : "flex-row"}`}>
                    {!isMe ? <Avatar name={senderName} size={28} online={presenceByUser[m.sender_id] === "online"} /> : null}
                    <div className="min-w-0">
                      <div className={`mb-1 text-xs text-slate-500 ${isMe ? "text-right" : "text-left"}`}>
                        {senderName + " · " + formatDateTime(m.created_at)}
                      </div>
                      <div
                        className={`relative w-fit rounded-2xl px-3 py-2 text-sm leading-snug ${
                          isMe ? "ml-auto" : "mr-auto"
                        }`}
                        style={{
                          backgroundColor: isMe ? "var(--chat-bubble-me-bg, #4f46e5)" : "var(--chat-bubble-other-bg, #1f2937)",
                          color: isMe ? "var(--chat-bubble-me-text, #ffffff)" : "var(--chat-bubble-other-text, #e2e8f0)",
                        }}
                      >
                        <div ref={reactionPickerFor === m.id ? reactionPickerRef : undefined}>
                          {uiMode === "play" ? (
                            <div
                              className={`absolute -top-4 ${
                                isMe ? "left-0 -translate-x-1/2" : "right-0 translate-x-1/2"
                              } z-20 flex items-center gap-1 rounded-full border border-slate-700 bg-slate-950/95 px-2 py-1 text-xs text-slate-200 opacity-0 shadow-lg backdrop-blur transition-opacity group-hover:opacity-100`}
                            >
                              {QUICK_REACTIONS.slice(0, 3).map((e) => (
                                <button
                                  key={e}
                                  className="grid h-7 w-7 place-items-center rounded-full text-base hover:bg-slate-800/70"
                                  onClick={() => addReaction.mutate({ messageId: m.id, emoji: e })}
                                  type="button"
                                  title={e}
                                >
                                  {e}
                                </button>
                              ))}
                              <button
                                className="grid h-7 w-7 place-items-center rounded-full hover:bg-slate-800/70"
                                onClick={() => setReactionPickerFor((cur) => (cur === m.id ? null : m.id))}
                                type="button"
                                aria-label="More reactions"
                                title="More"
                              >
                                +
                              </button>
                            </div>
                          ) : (
                            <div
                              className={`absolute -top-3 ${
                                isMe ? "left-0 -translate-x-1/2" : "right-0 translate-x-1/2"
                              } z-20 flex items-center gap-1 rounded-full border border-slate-700 bg-slate-950/90 px-1 py-1 text-xs text-slate-200 opacity-0 shadow-sm backdrop-blur transition-opacity group-hover:opacity-100`}
                            >
                              <button
                                className="grid h-7 w-7 place-items-center rounded-full hover:bg-slate-900"
                                onClick={() => setReactionPickerFor((cur) => (cur === m.id ? null : m.id))}
                                type="button"
                                aria-label="Add reaction"
                                title="React"
                              >
                                {"\u{1F642}"}
                              </button>
                              <button
                                className={`grid h-7 w-7 place-items-center rounded-full ${
                                  isPinned ? "bg-indigo-500/20 text-indigo-100" : "hover:bg-slate-900"
                                }`}
                                onClick={() => {
                                  if (isPinned) unpin.mutate(m.id);
                                  else pin.mutate(m.id);
                                }}
                                type="button"
                                aria-label="Pin message"
                                title={isPinned ? "Unpin" : "Pin"}
                              >
                                {"\u{1F4CC}"}
                              </button>
                              <button
                                className="grid h-7 w-7 place-items-center rounded-full hover:bg-slate-900"
                                onClick={() => {
                                  setWorkPane("threads");
                                  if (threadId) setActiveThreadId(threadId);
                                  else createThread.mutate({ root_message_id: m.id });
                                }}
                                type="button"
                                aria-label="Open thread"
                                title="Thread"
                              >
                                {"\u{1F9F5}"}
                              </button>
                            </div>
                          )}
                          {reactionPickerFor === m.id ? (
                            <div
                              className={`absolute -top-12 ${isMe ? "left-0" : "right-0"} z-20 flex items-center gap-1 rounded-full border border-slate-700 bg-slate-950/95 px-2 py-1 shadow-lg backdrop-blur`}
                            >
                              {QUICK_REACTIONS.map((e) => (
                                <button
                                  key={e}
                                  className="grid h-8 w-8 place-items-center rounded-full text-base hover:bg-slate-800/70"
                                  onClick={() => {
                                    setReactionPickerFor(null);
                                    addReaction.mutate({ messageId: m.id, emoji: e });
                                  }}
                                  type="button"
                                  title={e}
                                >
                                  {e}
                                </button>
                              ))}
                            </div>
                          ) : null}
                        </div>
                        {m.body ? <div className="whitespace-pre-wrap">{m.body}</div> : null}
                        {(m.attachments ?? []).length ? (
                          <div className={`${m.body ? "mt-2" : ""} space-y-2`}>
                            {(m.attachments ?? []).map((a) => {
                              const isImage = (a.content_type ?? "").startsWith("image/");
                              return (
                                <div key={a.id} className="rounded-xl border border-white/10 bg-black/10 p-2">
                                  {isImage ? (
                                    <img
                                      alt={a.filename}
                                      className="max-h-64 w-auto rounded-lg"
                                      src={a.download_url}
                                    />
                                  ) : (
                                    <a
                                      className="text-sm text-indigo-200 underline"
                                      download={a.filename}
                                      href={a.download_url}
                                    >
                                      {a.filename}
                                    </a>
                                  )}
                                </div>
                              );
                            })}
                          </div>
                        ) : null}
                      </div>
                      <div className={`mt-2 flex flex-wrap gap-1 ${isMe ? "justify-end" : "justify-start"}`}>
                        {(m.reactions ?? []).map((r) => (
                          <button
                            key={r.emoji}
                            className={`rounded-full border px-2 py-0.5 text-xs ${
                              r.reacted_by_me
                                ? "border-indigo-500/60 bg-indigo-500/20 text-indigo-100"
                                : "border-slate-700 bg-slate-900/40 text-slate-200 hover:bg-slate-800/60"
                            }`}
                            onClick={() => {
                              if (r.reacted_by_me) removeReaction.mutate({ messageId: m.id, emoji: r.emoji });
                              else addReaction.mutate({ messageId: m.id, emoji: r.emoji });
                            }}
                            type="button"
                          >
                            {r.emoji} {r.count}
                          </button>
                        ))}
                      </div>
                    </div>
                  </div>
                </div>
              );
            })}
            <div ref={bottomRef} />
          </div>
        </div>

        {typingText ? <div className="mt-2 text-xs text-slate-400">{typingText}</div> : null}

        {pendingAttachments.length ? (
          <div className="mt-3 flex flex-wrap gap-2">
            {pendingAttachments.map((a, idx) => (
              <div
                key={`${a.filename}-${idx}`}
                className="flex items-center gap-2 rounded-lg border border-slate-800 bg-slate-950/30 px-2 py-1"
              >
                <span className="max-w-[220px] truncate text-xs text-slate-200">{a.filename}</span>
                <button
                  className="text-xs text-slate-400 hover:text-slate-200"
                  onClick={() => setPendingAttachments((prev) => prev.filter((_, i) => i !== idx))}
                  type="button"
                >
                  ✕
                </button>
              </div>
            ))}
          </div>
        ) : null}

        {emojiOpen ? (
          <div className="mt-2 flex flex-wrap gap-1 rounded-lg border border-slate-800 bg-slate-950/30 p-2">
            {["😀", "😂", "❤️", "👍", "🎉", "🙏", "🔥", "😮", "😢", "😡", "✅", "👀"].map((e) => (
              <button
                key={e}
                className="rounded-md px-2 py-1 text-sm hover:bg-slate-800/60"
                onClick={() => setText((t) => (t ? `${t} ${e}` : e))}
                type="button"
              >
                {e}
              </button>
            ))}
          </div>
        ) : null}

        <form className="mt-3 space-y-2" onSubmit={onSubmit}>
          <div className="flex gap-2">
            <input
              className="hidden"
              multiple
              onChange={(e) => {
                const files = Array.from(e.target.files ?? []);
                if (!files.length) return;
                const readers = files.slice(0, 3).map(
                  (f) =>
                    new Promise<{ filename: string; content_type?: string; data_url: string }>((resolve, reject) => {
                      if (f.size > 2_000_000) return reject(new Error("File too large (max 2MB each for now)."));
                      const fr = new FileReader();
                      fr.onload = () =>
                        resolve({
                          filename: f.name,
                          content_type: f.type || undefined,
                          data_url: typeof fr.result === "string" ? fr.result : "",
                        });
                      fr.onerror = () => reject(new Error("Failed to read file."));
                      fr.readAsDataURL(f);
                    }),
                );
                Promise.all(readers)
                  .then((atts) => setPendingAttachments((prev) => [...prev, ...atts.filter((a) => a.data_url)]))
                  .catch((err) => alert((err as Error).message));
                e.target.value = "";
              }}
              ref={fileInputRef}
              type="file"
            />
            <div className="flex-1" />
          </div>
          <div className="flex items-end gap-2 rounded-xl border border-slate-800 bg-slate-950/30 px-3 py-2 focus-within:border-indigo-500/70 focus-within:ring-1 focus-within:ring-indigo-500/30">
            <TextArea
              ref={textAreaRef}
              rows={1}
              value={text}
              placeholder="Type a message"
              onChange={(e) => onTypingChange(e.target.value)}
              onKeyDown={(e) => {
                if (e.key === "Enter" && !e.shiftKey) {
                  e.preventDefault();
                  (e.currentTarget.form as HTMLFormElement | null)?.requestSubmit();
                }
              }}
              onInput={(e) => {
                const el = e.currentTarget;
                el.style.height = "";
                el.style.height = Math.min(el.scrollHeight, 180) + "px";
              }}
              className="max-h-[180px] flex-1 resize-none border-0 bg-transparent px-0 py-0 text-sm leading-6 focus:border-0"
            />
            <div className="flex items-center gap-1 pb-0.5">
              <button
                className={`grid h-9 w-9 place-items-center rounded-md text-sm text-slate-300 hover:bg-slate-800/60 hover:text-slate-100 ${
                  uiMode === "work" ? "hidden" : ""
                }`}
                onClick={() => setEmojiOpen((v) => !v)}
                type="button"
                title="Emoji"
              >
                🙂
              </button>
              <button
                className="grid h-9 w-9 place-items-center rounded-md text-sm text-slate-300 hover:bg-slate-800/60 hover:text-slate-100"
                onClick={() => fileInputRef.current?.click()}
                type="button"
                title="Attach"
              >
                📎
              </button>
              <div className="mx-1 h-6 w-px bg-slate-800" />
              <button
                className={`grid h-9 w-9 place-items-center rounded-md text-sm ${
                  send.isPending ? "bg-slate-800 text-slate-400" : "bg-indigo-600 text-white hover:bg-indigo-500"
                }`}
                disabled={send.isPending}
                type="submit"
                title="Send"
              >
                ➤
              </button>
            </div>
          </div>
        </form>
      </section>
    </div>
  );
}

function formatDateTime(input: unknown): string {
  const d = parseApiOffsetDateTime(input);
  if (!d) return String(input ?? "");
  const yyyy = d.getFullYear();
  const mm = String(d.getMonth() + 1).padStart(2, "0");
  const dd = String(d.getDate()).padStart(2, "0");

  let hours = d.getHours();
  const ampm = hours >= 12 ? "PM" : "AM";
  hours = hours % 12;
  if (hours === 0) hours = 12;
  const hh = String(hours).padStart(2, "0");
  const mi = String(d.getMinutes()).padStart(2, "0");
  const ss = String(d.getSeconds()).padStart(2, "0");

  return `${yyyy}/${mm}/${dd} ${hh}:${mi}:${ss} ${ampm}`;
}

function parseApiOffsetDateTime(input: unknown): Date | null {
  if (!input) return null;
  if (typeof input === "string") {
    const d = new Date(input);
    return Number.isNaN(d.getTime()) ? null : d;
  }
  if (Array.isArray(input) && input.length >= 6 && input.every((x) => typeof x === "number")) {
    const year = input[0] as number;
    const ordinal = input[1] as number; // 1..366
    const hour = input[2] as number;
    const minute = input[3] as number;
    const second = input[4] as number;
    const nanos = input[5] as number;
    const offsetH = (input[6] as number | undefined) ?? 0;
    const offsetM = (input[7] as number | undefined) ?? 0;
    const offsetS = (input[8] as number | undefined) ?? 0;

    const md = monthDayFromOrdinal(year, ordinal);
    if (!md) return null;
    const ms = Math.floor(nanos / 1_000_000);
    const localMillis = Date.UTC(year, md.month - 1, md.day, hour, minute, second, ms);
    const offsetTotalSeconds = offsetH * 3600 + offsetM * 60 + offsetS;
    return new Date(localMillis - offsetTotalSeconds * 1000);
  }
  return null;
}

function monthDayFromOrdinal(year: number, ordinal: number): { month: number; day: number } | null {
  if (!Number.isFinite(year) || !Number.isFinite(ordinal)) return null;
  if (ordinal < 1 || ordinal > 366) return null;
  const leap = year % 4 === 0 && (year % 100 !== 0 || year % 400 === 0);
  const dim = [31, leap ? 29 : 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
  let dayOfYear = Math.floor(ordinal);
  for (let i = 0; i < dim.length; i++) {
    const days = dim[i];
    if (dayOfYear <= days) return { month: i + 1, day: dayOfYear };
    dayOfYear -= days;
  }
  return null;
}
