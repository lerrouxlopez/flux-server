import { useQuery } from "@tanstack/react-query";
import type { JoinResponse, MediaRoom } from "../../api/types";
import { apiFetch } from "../../api/client";
import type { ExperienceContextValue } from "../../features/experience/ExperienceProvider";

export type MediaJoinIntent =
  | "voice_only"
  | "video"
  | "screen_share"
  | "stage_viewer"
  | "stage_speaker";

export function defaultIntent(params: {
  mediaDefaults: ExperienceContextValue["mediaDefaults"];
  roomKind: string;
}): MediaJoinIntent {
  const kind = params.roomKind;
  if (kind === "stage")
    return params.mediaDefaults.room_kind_preference === "meeting" ? "stage_speaker" : "stage_viewer";
  if (kind === "voice") return "voice_only";
  // meeting/default rooms
  return params.mediaDefaults.join_intent;
}

export function useMediaJoin(params: {
  room: MediaRoom | undefined;
  deviceId: string;
  mediaDefaults: ExperienceContextValue["mediaDefaults"];
  intent?: MediaJoinIntent;
  enabled?: boolean;
}) {
  const roomId = params.room?.id;
  const intent =
    params.intent ??
    (params.room ? defaultIntent({ mediaDefaults: params.mediaDefaults, roomKind: params.room.kind }) : undefined);

  return useQuery({
    enabled: (params.enabled ?? true) && !!roomId && !!intent,
    queryKey: ["mediaJoin", roomId, params.deviceId, intent],
    queryFn: () =>
      apiFetch<JoinResponse>(`/media/rooms/${roomId}/join`, {
        method: "POST",
        body: JSON.stringify({ intent, device_id: params.deviceId }),
      }),
    staleTime: 10_000,
  });
}
