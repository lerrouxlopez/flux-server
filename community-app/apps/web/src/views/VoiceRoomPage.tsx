import { useNavigate, useParams } from "react-router-dom";
import { useQuery } from "@tanstack/react-query";
import { LiveKitRoom, VideoConference } from "@livekit/components-react";
import "@livekit/components-styles";
import { useState } from "react";
import { apiFetch } from "../api/client";
import type { TokenResponse, MediaRoom } from "../api/types";

function normalizeLiveKitWsUrl(url: string) {
  const trimmed = url.trim().replace(/\/+$/, "");
  if (trimmed.startsWith("ws://") || trimmed.startsWith("wss://")) return trimmed;
  if (trimmed.startsWith("https://")) return "wss://" + trimmed.slice("https://".length);
  if (trimmed.startsWith("http://")) return "ws://" + trimmed.slice("http://".length);
  // Fallback: assume caller provided host:port.
  return "ws://" + trimmed;
}

export function VoiceRoomPage() {
  const { room_id, org_slug } = useParams();
  const nav = useNavigate();
  const [shouldConnect, setShouldConnect] = useState(true);
  const [lkError, setLkError] = useState<string | null>(null);
  const [disconnectReason, setDisconnectReason] = useState<string | null>(null);

  const room = useQuery({
    enabled: !!room_id,
    queryKey: ["mediaRoom", room_id],
    queryFn: () => apiFetch<MediaRoom>(`/media/rooms/${room_id}`),
  });

  const token = useQuery({
    enabled: !!room.data?.id,
    queryKey: ["livekitToken", room_id],
    queryFn: () =>
      apiFetch<TokenResponse>(`/media/rooms/${room_id}/token`, {
        method: "POST",
        body: JSON.stringify({ can_publish: true, can_subscribe: true, can_publish_data: true }),
      }),
  });

  if (room.isLoading || token.isLoading) return <div className="text-slate-300">Connecting…</div>;
  if (room.isError) return <div className="text-red-400">{(room.error as Error).message}</div>;
  if (token.isError) return <div className="text-red-400">{(token.error as Error).message}</div>;
  if (!room.data || !token.data) return <div className="text-slate-300">Missing data.</div>;

  const serverUrl = normalizeLiveKitWsUrl(token.data.livekit_url);
  const backToChannel =
    room.data.channel_id && org_slug ? `/app/${org_slug}/channels/${room.data.channel_id}` : org_slug ? `/app/${org_slug}` : "/orgs";

  return (
    <div className="rounded-xl border border-slate-800 bg-slate-900/30 p-4">
      <div className="flex items-center justify-between gap-3">
        <h1 className="min-w-0 truncate text-lg font-semibold">{room.data.name}</h1>
        <div className="flex items-center gap-2">
          {!shouldConnect ? (
            <button
              className="rounded-md border border-slate-800 bg-slate-900 px-2 py-1 text-xs text-slate-200 hover:bg-slate-800/60"
              onClick={() => {
                setLkError(null);
                setDisconnectReason(null);
                setShouldConnect(true);
              }}
              type="button"
            >
              Retry
            </button>
          ) : null}
          <button
            className="rounded-md border border-slate-800 bg-slate-900 px-2 py-1 text-xs text-slate-200 hover:bg-slate-800/60"
            onClick={() => {
              setShouldConnect(false);
              nav(backToChannel);
            }}
            type="button"
          >
            Back
          </button>
        </div>
      </div>
      {lkError || disconnectReason ? (
        <div className="mt-2 rounded-lg border border-slate-800 bg-slate-950/30 px-3 py-2 text-xs text-slate-200">
          <div className="font-semibold">LiveKit status</div>
          <div className="mt-1 text-slate-300">
            {lkError ? `Error: ${lkError}` : null}
            {lkError && disconnectReason ? " • " : null}
            {disconnectReason ? `Disconnected: ${disconnectReason}` : null}
          </div>
          <div className="mt-1 text-slate-500">{`serverUrl: ${serverUrl}`}</div>
        </div>
      ) : null}
      <div className="mt-3 h-[70vh] overflow-hidden rounded-lg border border-slate-800 bg-black">
        <LiveKitRoom
          serverUrl={serverUrl}
          token={token.data.token}
          connect={shouldConnect}
          onError={(e) => {
            setLkError(e?.message ?? String(e));
            // eslint-disable-next-line no-console
            console.error("LiveKit error", e);
          }}
          onDisconnected={() => {
            setShouldConnect(false);
            setDisconnectReason("disconnected");
          }}
        >
          <VideoConference />
        </LiveKitRoom>
      </div>
    </div>
  );
}
