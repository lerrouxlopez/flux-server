import { useEffect, useMemo, useRef, useState } from "react";
import { useNavigate, useParams } from "react-router-dom";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { apiFetch } from "../api/client";
import type {
  ChannelsResponse,
  Channel,
  ChannelSearchResponse,
  ListMessagesResponse,
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
import { useAuthStore } from "../state/auth";
import { useBrandingStore } from "../state/branding";

type PresenceStatus = "online" | "offline";

export type ChannelEngine = ReturnType<typeof useChannelEngine>;

export function useChannelEngine() {
  const { org_slug, channel_id } = useParams();
  const nav = useNavigate();
  const qc = useQueryClient();
  const me = useAuthStore((s) => s.user);
  const meId = me?.id ?? null;
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

  useEffect(() => {
    bottomRef.current?.scrollIntoView({ block: "end" });
  }, [messages.data?.messages?.length]);

  const send = useMutation({
    mutationFn: async (input: { body?: string; attachments?: { filename: string; content_type?: string; data_url: string }[] }) =>
      apiFetch<Message>(`/channels/${channel_id}/messages`, {
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
        thread_id: null,
        attachments: (input.attachments ?? []).map((a, idx) => ({
          id: `optimistic-att-${idx}`,
          filename: a.filename,
          content_type: a.content_type ?? null,
          size_bytes: a.data_url.length,
          download_url: a.data_url,
          created_at: new Date().toISOString(),
        })),
        reactions: [],
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
        if (!prev?.messages?.length) return prev;
        const next = prev.messages.map((m) => {
          if (m.id !== input.messageId) return m;
          const reactions = m.reactions ?? [];
          const existing = reactions.find((r) => r.emoji === input.emoji);
          const updated = existing
            ? reactions.map((r) =>
                r.emoji === input.emoji ? { ...r, count: r.count + 1, reacted_by_me: true } : r,
              )
            : [...reactions, { emoji: input.emoji, count: 1, reacted_by_me: true }];
          return { ...m, reactions: updated };
        });
        return { ...prev, messages: next };
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
        if (!prev?.messages?.length) return prev;
        const next = prev.messages.map((m) => {
          if (m.id !== input.messageId) return m;
          const reactions = m.reactions ?? [];
          const updated = reactions
            .map((r) =>
              r.emoji === input.emoji ? { ...r, count: Math.max(0, r.count - 1), reacted_by_me: false } : r,
            )
            .filter((r) => r.count > 0);
          return { ...m, reactions: updated };
        });
        return { ...prev, messages: next };
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
    const withoutTrailingSmile = trimmed.replace(/\s*ðŸ™‚$/, "");
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
    return [...list].reverse();
  }, [messages.data]);

  const searchResults = useQuery({
    enabled: !!channel_id && uiMode === "work" && workPane === "search" && workSearchOpen && !!workSearch.trim(),
    queryKey: ["channel-search", channel_id, workSearch.trim()],
    queryFn: () =>
      apiFetch<ChannelSearchResponse>(`/channels/${channel_id}/search?q=${encodeURIComponent(workSearch.trim())}&limit=50`),
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
      const da = new Date(a.created_at).getTime();
      const db = new Date(b.created_at).getTime();
      if (da !== db) return da - db;
      return String(a.id).localeCompare(String(b.id));
    });
    return sorted;
  }, [orderedMessages, uiMode, workPane, workSearch, searchResults.data]);

  const createThread = useMutation({
    mutationFn: async (input: { body?: string; root_message_id?: string }) => {
      return apiFetch<any>(`/channels/${channel_id}/threads`, {
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

  return {
    // context
    org_slug,
    channel_id,
    orgs,
    org,
    channels,
    channel,
    members,
    myRole,
    canSeeAdmin,
    memberById,
    canManageChannel,
    channelTitle,
    uiMode,
    meId,

    // header edit state
    editingChannelName,
    setEditingChannelName,
    channelNameDraft,
    setChannelNameDraft,
    updateChannel,
    deleteChannel,

    // realtime + presence/typing
    connected,
    presenceByUser,
    typingText,

    // compose + reactions
    text,
    setText,
    emojiOpen,
    setEmojiOpen,
    reactionPickerFor,
    setReactionPickerFor,
    QUICK_REACTIONS,
    reactionPickerRef,
    fileInputRef,
    textAreaRef,
    pendingAttachments,
    setPendingAttachments,
    onTypingChange,
    onSubmit,
    send,
    addReaction,
    removeReaction,

    // messages
    messages,
    visibleMessages,
    bottomRef,

    // work panes
    workSearchOpen,
    setWorkSearchOpen,
    workSearch,
    setWorkSearch,
    workPane,
    setWorkPane,
    activeThreadId,
    setActiveThreadId,
    threadDraft,
    setThreadDraft,
    newThreadDraft,
    setNewThreadDraft,
    searchResults,
    pins,
    pinnedIds,
    threads,
    thread,
    createThread,
    replyToThread,
    pin,
    unpin,

    // meeting
    createMeeting,
  };
}
