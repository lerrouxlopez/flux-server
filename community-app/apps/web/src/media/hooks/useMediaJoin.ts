import { useQuery } from "@tanstack/react-query";
import type { JoinResponse, MediaRoom } from "../../api/types";
import { apiFetch } from "../../api/client";

export type MediaJoinIntent =
  | "voice_only"
  | "video"
  | "screen_share"
  | "stage_viewer"
  | "stage_speaker";

export function defaultIntent(params: {
  uiMode: "work" | "play";
  roomKind: string;
}): MediaJoinIntent {
  const kind = params.roomKind;
  if (kind === "stage") return params.uiMode === "work" ? "stage_speaker" : "stage_viewer";
  if (kind === "voice") return "voice_only";
  return params.uiMode === "work" ? "video" : "voice_only";
}

export function useMediaJoin(params: {
  room: MediaRoom | undefined;
  deviceId: string;
  uiMode: "work" | "play";
  intent?: MediaJoinIntent;
  enabled?: boolean;
}) {
  const roomId = params.room?.id;
  const intent = params.intent ?? (params.room ? defaultIntent({ uiMode: params.uiMode, roomKind: params.room.kind }) : undefined);

  return useQuery({
    enabled: (params.enabled ?? true) && !!roomId && !!intent,
    queryKey: ["mediaJoin", roomId, params.deviceId, params.uiMode, intent],
    queryFn: () =>
      apiFetch<JoinResponse>(`/media/rooms/${roomId}/join`, {
        method: "POST",
        body: JSON.stringify({ intent, device_id: params.deviceId }),
      }),
    staleTime: 10_000,
  });
}

