import { useEffect, useMemo, useRef, useState } from "react";
import { Link, useNavigate, useParams } from "react-router-dom";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { apiFetch } from "../api/client";
import type {
  ChannelsResponse,
  Channel,
  ListMessagesResponse,
  MessageAttachment,
  MessageReaction,
  MediaRoom,
  Member,
  MembersResponse,
  Message,
  OrgsListResponse,
} from "../api/types";
import { createRealtimeClient } from "../realtime/ws";
import { Input } from "../components/Input";
import { Button } from "../components/Button";
import { useAuthStore } from "../state/auth";
import { useBrandingStore } from "../state/branding";
import { OrgSidebar } from "../components/OrgSidebar";
import { Avatar } from "../components/Avatar";

type SendMessageResponse = Message;

type PresenceStatus = "online" | "offline";

export function ChannelPage() {
  const { org_slug, channel_id } = useParams();
  const nav = useNavigate();
  const qc = useQueryClient();
  const me = useAuthStore((s) => s.user);
  const loadOrgBranding = useBrandingStore((s) => s.loadOrgBranding);

  const [text, setText] = useState("");
  const [connected, setConnected] = useState(false);
  const [emojiOpen, setEmojiOpen] = useState(false);
  const [pendingAttachments, setPendingAttachments] = useState<
    { filename: string; content_type?: string; data_url: string }[]
  >([]);

  const typingTimeout = useRef<number | null>(null);
  const fileInputRef = useRef<HTMLInputElement | null>(null);
  const [typingByUser, setTypingByUser] = useState<Record<string, number>>({});
  const [presenceByUser, setPresenceByUser] = useState<Record<string, PresenceStatus>>({});

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
          data_url: a.data_url,
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

  async function onSubmit(e: React.FormEvent) {
    e.preventDefault();
    const body = text.trim();
    if (!body && pendingAttachments.length === 0) return;
    setText("");
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

  const channelTitle = channel?.kind === "dm" ? `Direct message` : `# ${channel?.name ?? ""}`;

  if (orgs.isLoading) return <div className="text-slate-300">Loading...</div>;
  if (!org) return <div className="text-slate-300">Org not found.</div>;
  if (!channel) return <div className="text-slate-300">Channel not found.</div>;

  return (
    <div className="grid gap-6 md:grid-cols-[280px_1fr]">
      <OrgSidebar org={org} activeChannelId={channel_id} presenceByUser={presenceByUser} />

      <section className="rounded-xl border border-slate-800 bg-slate-900/30 p-4">
        <div className="flex items-center justify-between">
          <div className="text-lg font-semibold">{channelTitle}</div>
          <div className="text-xs text-slate-400">{connected ? "realtime online" : "realtime offline"}</div>
        </div>

        <div className="mt-3 flex gap-2">
          <Button
            className="bg-emerald-600 hover:bg-emerald-500"
            disabled={createMeeting.isPending}
            onClick={() => createMeeting.mutate()}
            type="button"
          >
            {createMeeting.isPending ? "Creating…" : "Start meeting"}
          </Button>
        </div>

        <div className="mt-4 h-[60vh] overflow-auto rounded-lg border border-slate-800 bg-slate-950/30 p-3">
          <div className="space-y-3">
            {orderedMessages.map((m) => {
              const isMe = m.sender_id === (me?.id ?? "me") || m.sender_id === "me";
              const senderName = memberById.get(m.sender_id)?.display_name ?? m.sender_id.slice(0, 8);
              return (
                <div key={m.id} className={`flex ${isMe ? "justify-end" : "justify-start"}`}>
                  <div className={`flex max-w-[88%] items-end gap-2 ${isMe ? "flex-row-reverse" : "flex-row"}`}>
                    {!isMe ? <Avatar name={senderName} size={28} online={presenceByUser[m.sender_id] === "online"} /> : null}
                    <div className="min-w-0">
                      <div className={`mb-1 text-xs text-slate-500 ${isMe ? "text-right" : "text-left"}`}>
                        {senderName + " · " + formatDateTime(m.created_at)}
                      </div>
                      <div
                        className={`w-fit rounded-2xl px-3 py-2 text-sm leading-snug ${
                          isMe ? "ml-auto bg-indigo-600 text-white" : "mr-auto bg-slate-800 text-slate-100"
                        }`}
                      >
                        {m.body ? <div className="whitespace-pre-wrap">{m.body}</div> : null}
                        {(m.attachments ?? []).length ? (
                          <div className={`${m.body ? "mt-2" : ""} space-y-2`}>
                            {(m.attachments ?? []).map((a) => {
                              const isImage = (a.content_type ?? "").startsWith("image/") || a.data_url.startsWith("data:image/");
                              return (
                                <div key={a.id} className="rounded-xl border border-white/10 bg-black/10 p-2">
                                  {isImage ? (
                                    <img
                                      alt={a.filename}
                                      className="max-h-64 w-auto rounded-lg"
                                      src={a.data_url}
                                    />
                                  ) : (
                                    <a
                                      className="text-sm text-indigo-200 underline"
                                      download={a.filename}
                                      href={a.data_url}
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
                        <button
                          className="rounded-full border border-slate-700 bg-slate-900/40 px-2 py-0.5 text-xs text-slate-200 hover:bg-slate-800/60"
                          onClick={() => {
                            const emoji = prompt("React with emoji:");
                            if (!emoji) return;
                            addReaction.mutate({ messageId: m.id, emoji });
                          }}
                          type="button"
                        >
                          🙂
                        </button>
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
            <Button className="bg-slate-900 hover:bg-slate-800" onClick={() => setEmojiOpen((v) => !v)} type="button">
              🙂
            </Button>
            <Button className="bg-slate-900 hover:bg-slate-800" onClick={() => fileInputRef.current?.click()} type="button">
              📎
            </Button>
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
          <Input value={text} onChange={(e) => onTypingChange(e.target.value)} placeholder="Message…" />
          <Button className="bg-indigo-600 hover:bg-indigo-500" disabled={send.isPending} type="submit">
            Send
          </Button>
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
