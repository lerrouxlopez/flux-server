import { RoomAudioRenderer, useRoomContext } from "@livekit/components-react";
import { useEffect, useState } from "react";

export function VoiceDock() {
  const room = useRoomContext();
  const [muted, setMuted] = useState<boolean>(false);

  useEffect(() => {
    let alive = true;
    (async () => {
      try {
        const enabled = room.localParticipant.isMicrophoneEnabled;
        if (alive) setMuted(!enabled);
      } catch {
        // ignore
      }
    })();
    return () => {
      alive = false;
    };
  }, [room]);

  return (
    <div className="flex h-full flex-col">
      <div className="flex items-center justify-between border-b border-slate-800 bg-slate-950/30 px-3 py-2">
        <div className="text-xs font-semibold text-slate-200">VoiceDock</div>
        <button
          className="rounded-md border border-slate-800 bg-slate-900 px-2 py-1 text-xs text-slate-200 hover:bg-slate-800/60"
          type="button"
          onClick={async () => {
            const next = !muted;
            setMuted(next);
            try {
              await room.localParticipant.setMicrophoneEnabled(!next);
            } catch {
              // ignore
            }
          }}
        >
          {muted ? "Unmute" : "Mute"}
        </button>
      </div>

      <div className="flex-1 p-3">
        <div className="rounded-lg border border-slate-800 bg-slate-950/30 p-3 text-sm text-slate-200">
          <div className="font-semibold">Audio-first mode</div>
          <div className="mt-1 text-xs text-slate-300">
            FLUX keeps this minimal for Game Mode. Video UI is intentionally hidden here.
          </div>
        </div>
      </div>

      <RoomAudioRenderer />
    </div>
  );
}

