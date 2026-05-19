import { useNavigate, useParams } from "react-router-dom";
import { useQuery } from "@tanstack/react-query";
import "@livekit/components-styles";
import { apiFetch } from "../api/client";
import type { MediaRoom } from "../api/types";
import { useDeviceId } from "../media/hooks/useDeviceId";
import { useMediaJoin } from "../media/hooks/useMediaJoin";
import { FluxMediaShell } from "../media/components/FluxMediaShell";
import { VoiceDock } from "../media/components/VoiceDock";
import { MeetingRoom } from "../media/components/MeetingRoom";
import { DeviceSetupDialog } from "../media/components/DeviceSetupDialog";
import { createRealtimeClient } from "../realtime/ws";
import { useEffect, useMemo, useState } from "react";
import { useMediaRoomStore } from "../media/state/mediaRoom";
import { useExperience } from "../features/experience/useExperience";

function normalizeLiveKitWsUrl(url: string) {
  const trimmed = url.trim().replace(/\/+$/, "");
  if (trimmed.startsWith("ws://") || trimmed.startsWith("wss://")) return trimmed;
  if (trimmed.startsWith("https://")) return "wss://" + trimmed.slice("https://".length);
  if (trimmed.startsWith("http://")) return "ws://" + trimmed.slice("http://".length);
  return "ws://" + trimmed;
}

export function VoiceRoomPage() {
  const { room_id, org_slug } = useParams();
  const nav = useNavigate();
  const uiMode = useExperience().rawMode;
  const deviceId = useDeviceId();

  const room = useQuery({
    enabled: !!room_id,
    queryKey: ["mediaRoom", room_id],
    queryFn: () => apiFetch<MediaRoom>(`/media/rooms/${room_id}`),
  });

  const join = useMediaJoin({ room: room.data, deviceId, uiMode, enabled: !!room.data?.id });
  const upsertParticipant = useMediaRoomStore((s) => s.upsertParticipant);
  const markLeft = useMediaRoomStore((s) => s.markLeft);
  const clearRoom = useMediaRoomStore((s) => s.clearRoom);
  const [rtConnected, setRtConnected] = useState(false);

  const rt = useMemo(() => {
    return createRealtimeClient({
      onOpen: () => setRtConnected(true),
      onClose: () => setRtConnected(false),
      onEvent: (evt) => {
        const e = evt as any;
        if (!room.data?.id) return;
        if (e?.type === "media.participant.joined" && e.room_id === room.data.id) {
          upsertParticipant(room.data.id, {
            participant_id: String(e.participant_id),
            user_id: String(e.user_id),
            device_id: String(e.device_id ?? ""),
            joined_at: String(e.joined_at ?? ""),
          });
        }
        if (e?.type === "media.participant.left" && e.room_id === room.data.id) {
          markLeft(room.data.id, String(e.participant_id), String(e.left_at ?? new Date().toISOString()));
        }
      },
    });
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [room.data?.id, upsertParticipant, markLeft]);

  useEffect(() => {
    rt.start();
    return () => rt.stop();
  }, [rt]);

  useEffect(() => {
    const roomId = room.data?.id;
    if (!roomId) return;
    rt.send({ type: "media.subscribe", room_id: roomId });
    return () => {
      rt.send({ type: "media.unsubscribe", room_id: roomId });
      clearRoom(roomId);
    };
  }, [rt, room.data?.id, clearRoom]);

  if (room.isLoading || join.isLoading) return <div className="text-slate-300">Connecting…</div>;
  if (room.isError) return <div className="flux-text-danger">{(room.error as Error).message}</div>;
  if (join.isError) return <div className="flux-text-danger">{(join.error as Error).message}</div>;
  if (!room.data || !join.data) return <div className="text-slate-300">Missing data.</div>;

  const serverUrl = normalizeLiveKitWsUrl(join.data.livekit_url);
  const backToChannel =
    room.data.channel_id && org_slug
      ? `/app/${org_slug}/channels/${room.data.channel_id}`
      : org_slug
        ? `/app/${org_slug}`
        : "/orgs";

  return (
    <FluxMediaShell
      serverUrl={serverUrl}
      token={join.data.token}
      header={<h1 className="min-w-0 truncate text-lg font-semibold">{room.data.name}</h1>}
      footer={
        <div className="flex items-center gap-2">
          <div className={`text-[11px] ${rtConnected ? "flux-text-success" : "text-slate-400"}`}>{rtConnected ? "Realtime" : "Realtime…"}</div>
          <DeviceSetupDialog />
        </div>
      }
      onEnd={() => nav(backToChannel)}
    >
      {uiMode === "play" ? <VoiceDock roomId={room.data.id} /> : <MeetingRoom />}
    </FluxMediaShell>
  );
}
