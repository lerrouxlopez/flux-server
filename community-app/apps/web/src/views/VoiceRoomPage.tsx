import { useParams } from "react-router-dom";
import { useQuery } from "@tanstack/react-query";
import { LiveKitRoom, VideoConference } from "@livekit/components-react";
import "@livekit/components-styles";
import { apiFetch } from "../api/client";
import type { TokenResponse, MediaRoom } from "../api/types";

export function VoiceRoomPage() {
  const { room_id } = useParams();

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

  return (
    <div className="rounded-xl border border-slate-800 bg-slate-900/30 p-4">
      <h1 className="text-lg font-semibold">{room.data.name}</h1>
      <div className="mt-3 h-[70vh] overflow-hidden rounded-lg border border-slate-800 bg-black">
        <LiveKitRoom serverUrl={token.data.livekit_url} token={token.data.token} connect={true}>
          <VideoConference />
        </LiveKitRoom>
      </div>
    </div>
  );
}
