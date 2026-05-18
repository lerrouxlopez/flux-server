import { useNavigate, useParams } from "react-router-dom";
import { useQuery } from "@tanstack/react-query";
import "@livekit/components-styles";
import { apiFetch } from "../api/client";
import type { MediaRoom } from "../api/types";
import { useBrandingStore } from "../state/branding";
import { useDeviceId } from "../media/hooks/useDeviceId";
import { useMediaJoin } from "../media/hooks/useMediaJoin";
import { FluxMediaShell } from "../media/components/FluxMediaShell";
import { VoiceDock } from "../media/components/VoiceDock";
import { MeetingRoom } from "../media/components/MeetingRoom";
import { DeviceSetupDialog } from "../media/components/DeviceSetupDialog";

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
  const uiMode = useBrandingStore((s) => s.branding?.ui_mode ?? "work");
  const deviceId = useDeviceId();

  const room = useQuery({
    enabled: !!room_id,
    queryKey: ["mediaRoom", room_id],
    queryFn: () => apiFetch<MediaRoom>(`/media/rooms/${room_id}`),
  });

  const join = useMediaJoin({ room: room.data, deviceId, uiMode, enabled: !!room.data?.id });

  if (room.isLoading || join.isLoading) return <div className="text-slate-300">Connecting…</div>;
  if (room.isError) return <div className="text-red-400">{(room.error as Error).message}</div>;
  if (join.isError) return <div className="text-red-400">{(join.error as Error).message}</div>;
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
      footer={<DeviceSetupDialog />}
      onEnd={() => nav(backToChannel)}
    >
      {uiMode === "play" ? <VoiceDock /> : <MeetingRoom />}
    </FluxMediaShell>
  );
}

