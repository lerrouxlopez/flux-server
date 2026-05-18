import { create } from "zustand";

export type MediaParticipantView = {
  participant_id: string;
  user_id: string;
  device_id: string;
  joined_at: string;
  left_at?: string | null;
};

type MediaRoomState = {
  participantsByRoom: Record<string, Record<string, MediaParticipantView>>;
  upsertParticipant: (roomId: string, p: MediaParticipantView) => void;
  markLeft: (roomId: string, participantId: string, leftAt: string) => void;
  clearRoom: (roomId: string) => void;
};

export const useMediaRoomStore = create<MediaRoomState>((set) => ({
  participantsByRoom: {},
  upsertParticipant: (roomId, p) =>
    set((s) => {
      const room = s.participantsByRoom[roomId] ?? {};
      return {
        participantsByRoom: {
          ...s.participantsByRoom,
          [roomId]: { ...room, [p.participant_id]: p },
        },
      };
    }),
  markLeft: (roomId, participantId, leftAt) =>
    set((s) => {
      const room = s.participantsByRoom[roomId];
      if (!room || !room[participantId]) return s;
      return {
        participantsByRoom: {
          ...s.participantsByRoom,
          [roomId]: {
            ...room,
            [participantId]: { ...room[participantId], left_at: leftAt },
          },
        },
      };
    }),
  clearRoom: (roomId) =>
    set((s) => {
      const next = { ...s.participantsByRoom };
      delete next[roomId];
      return { participantsByRoom: next };
    }),
}));

