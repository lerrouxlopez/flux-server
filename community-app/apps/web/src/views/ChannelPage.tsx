import { useEffect, useMemo, useRef, useState } from "react";
import { Link, useNavigate, useParams } from "react-router-dom";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { apiFetch } from "../api/client";
import type {
  ChannelsResponse,
  ListMessagesResponse,
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

  const typingTimeout = useRef<number | null>(null);
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
  const channel = channels.data?.channels.find((c) => c.id === channel_id);

  const members = useQuery({
    enabled: !!org?.id,
    queryKey: ["members", org?.id],
    queryFn: () => apiFetch<MembersResponse>(`/orgs/${org!.id}/members`),
    staleTime: 10_000,
  });

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
    mutationFn: async (body: string) =>
      apiFetch<SendMessageResponse>(`/channels/${channel_id}/messages`, {
        method: "POST",
        body: JSON.stringify({ body }),
      }),
    onMutate: async (body) => {
      await qc.cancelQueries({ queryKey: ["messages", channel_id] });
      const prev = qc.getQueryData<ListMessagesResponse>(["messages", channel_id]);
      const optimistic: Message = {
        id: `optimistic-${Date.now()}`,
        organization_id: org?.id ?? "",
        channel_id: channel_id!,
        sender_id: "me",
        body,
        kind: "text",
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

  if (!org || !channel) {
    return <div className="text-slate-300">Channel not found.</div>;
  }

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
    if (!body) return;
    setText("");
    send.mutate(body);
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

  const onlineCount = (members.data?.members ?? []).filter((m) => presenceByUser[m.user_id] === "online").length;

  const orderedMessages = useMemo(() => {
    const list = messages.data?.messages ?? [];
    // API returns newest-first; UI should show newest at bottom.
    return [...list].reverse();
  }, [messages.data]);

  return (
    <div className="grid gap-6 md:grid-cols-[280px_1fr]">
      <aside className="rounded-xl border border-slate-800 bg-slate-900/30 p-3">
        <div className="px-2 py-2 text-sm font-semibold">{org.name}</div>

        <div className="mt-2 space-y-1">
          {(channels.data?.channels ?? []).map((c) => (
            <Link
              key={c.id}
              to={`/app/${org.slug}/channels/${c.id}`}
              className={`block rounded-md px-2 py-1.5 text-sm hover:bg-slate-800/60 ${
                c.id === channel_id ? "text-white" : "text-slate-200"
              }`}
            >
              # {c.name}
            </Link>
          ))}
        </div>

        <div className="mt-3 border-t border-slate-800 pt-3">
          <div className="px-2 text-xs font-semibold text-slate-400">
            Members {members.isLoading ? "" : `(${onlineCount} online)`}
          </div>
          <div className="mt-2 space-y-1">
            {(members.data?.members ?? []).slice(0, 12).map((m) => {
              const online = presenceByUser[m.user_id] === "online";
              return (
                <div key={m.user_id} className="flex items-center justify-between rounded-md px-2 py-1 text-sm">
                  <div className="flex items-center gap-2">
                    <span className={`h-2 w-2 rounded-full ${online ? "bg-emerald-400" : "bg-slate-600"}`} />
                    <span className="text-slate-200">{m.display_name}</span>
                  </div>
                  <span className="text-xs text-slate-500">{m.role}</span>
                </div>
              );
            })}
          </div>
          <div className="mt-2 px-2">
            <Link className="text-xs text-indigo-400 hover:underline" to={`/admin/${org.slug}`}>
              Open admin panel
            </Link>
          </div>
        </div>
      </aside>

      <section className="rounded-xl border border-slate-800 bg-slate-900/30 p-4">
        <div className="flex items-center justify-between">
          <div className="text-lg font-semibold"># {channel.name}</div>
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
            {orderedMessages.map((m) => (
              <div key={m.id} className="text-sm">
                <div className="text-xs text-slate-400">
                  {(memberById.get(m.sender_id)?.display_name ?? m.sender_id.slice(0, 8)) +
                    " . " +
                    formatDateTime(m.created_at)}
                </div>
                <div className="text-slate-100">{m.body}</div>
              </div>
            ))}
            <div ref={bottomRef} />
          </div>
        </div>

        {typingText ? <div className="mt-2 text-xs text-slate-400">{typingText}</div> : null}

        <form className="mt-3 flex gap-2" onSubmit={onSubmit}>
          <Input value={text} onChange={(e) => onTypingChange(e.target.value)} placeholder="Message…" />
          <Button disabled={send.isPending}>Send</Button>
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
