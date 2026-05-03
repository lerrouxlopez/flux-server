import { useEffect, useMemo, useRef, useState } from "react";
import { useParams, Link, useNavigate } from "react-router-dom";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { apiFetch } from "../api/client";
import type { OrgsListResponse, ChannelsResponse, ListMessagesResponse, Message, MediaRoom } from "../api/types";
import { createRealtimeClient } from "../realtime/ws";
import { Input } from "../components/Input";
import { Button } from "../components/Button";

type SendMessageResponse = Message;

export function ChannelPage() {
  const { org_slug, channel_id } = useParams();
  const nav = useNavigate();
  const qc = useQueryClient();
  const [text, setText] = useState("");
  const [connected, setConnected] = useState(false);
  const typingTimeout = useRef<number | null>(null);

  const orgs = useQuery({
    queryKey: ["orgs"],
    queryFn: () => apiFetch<OrgsListResponse>("/orgs"),
  });
  const org = orgs.data?.organizations.find((o) => o.slug === org_slug);

  const channels = useQuery({
    enabled: !!org?.id,
    queryKey: ["channels", org?.id],
    queryFn: () => apiFetch<ChannelsResponse>(`/orgs/${org!.id}/channels`),
  });

  const channel = channels.data?.channels.find((c) => c.id === channel_id);

  const messages = useQuery({
    enabled: !!channel_id,
    queryKey: ["messages", channel_id],
    queryFn: () => apiFetch<ListMessagesResponse>(`/channels/${channel_id}/messages?limit=50`),
    staleTime: 5_000,
  });

  const rt = useMemo(() => {
    return createRealtimeClient({
      onOpen: () => setConnected(true),
      onClose: () => setConnected(false),
      onEvent: (evt) => {
        const e = evt as any;
        if (e?.type === "message.created" && e.channel_id === channel_id) {
          qc.invalidateQueries({ queryKey: ["messages", channel_id] });
        }
        // typing.stopped is server-driven; we can clear UI indicators here later.
      },
    });
  }, [channel_id, qc]);

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

  return (
    <div className="grid gap-6 md:grid-cols-[260px_1fr]">
      <aside className="rounded-xl border border-slate-800 bg-slate-900/30 p-3">
        <div className="px-2 py-2 text-sm font-semibold">{org.name}</div>
        <div className="mt-2 space-y-1">
          {(channels.data?.channels ?? []).map((c) => (
            <Link
              key={c.id}
              to={`/app/${org.slug}/channels/${c.id}`}
              className="block rounded-md px-2 py-1.5 text-sm text-slate-200 hover:bg-slate-800/60"
            >
              # {c.name}
            </Link>
          ))}
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
            {(messages.data?.messages ?? []).map((m) => (
              <div key={m.id} className="text-sm">
                <div className="text-xs text-slate-400">{m.created_at}</div>
                <div className="text-slate-100">{m.body}</div>
              </div>
            ))}
          </div>
        </div>

        <form className="mt-3 flex gap-2" onSubmit={onSubmit}>
          <Input value={text} onChange={(e) => onTypingChange(e.target.value)} placeholder="Message…" />
          <Button disabled={send.isPending}>Send</Button>
        </form>
      </section>
    </div>
  );
}
