import { useEffect, useMemo, useState } from "react";
import { Room } from "livekit-client";
import { LiveKitRoom } from "@livekit/components-react";
import type { MediaConnectStatus } from "../types";
import { ConnectionBanner } from "./ConnectionBanner";

export function FluxMediaShell(props: {
  serverUrl: string;
  token: string;
  header: React.ReactNode;
  footer?: React.ReactNode;
  children: React.ReactNode;
  onEnd: () => void;
}) {
  const [connect, setConnect] = useState(true);
  const [status, setStatus] = useState<MediaConnectStatus>("connecting");
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (!connect) setStatus("ended");
  }, [connect]);

  const roomOptions = useMemo(() => ({ adaptiveStream: true, dynacast: true }), []);
  const room = useMemo(() => new Room(roomOptions), [roomOptions]);

  useEffect(() => {
    const onReconnecting = () => setStatus("reconnecting");
    const onReconnected = () => setStatus("connected");
    room.on("reconnecting", onReconnecting);
    room.on("reconnected", onReconnected);
    return () => {
      room.off("reconnecting", onReconnecting);
      room.off("reconnected", onReconnected);
    };
  }, [room]);

  return (
    <div className="rounded-xl border border-slate-800 bg-slate-900/30 p-4">
      <div className="flex items-center justify-between gap-3">
        <div className="min-w-0">{props.header}</div>
        <div className="flex items-center gap-2">
          <button
            className="rounded-md border border-slate-800 bg-slate-900 px-2 py-1 text-xs text-slate-200 hover:bg-slate-800/60"
            onClick={() => {
              setConnect(false);
              props.onEnd();
            }}
            type="button"
          >
            Back
          </button>
        </div>
      </div>

      <div className="mt-2">
        <ConnectionBanner
          status={status}
          error={error}
          onRetry={
            status === "failed"
              ? () => {
                  setError(null);
                  setStatus("connecting");
                  setConnect(true);
                }
              : undefined
          }
          onEnd={() => {
            setConnect(false);
            props.onEnd();
          }}
        />
      </div>

      <div className="mt-3 h-[70vh] overflow-hidden rounded-lg border border-slate-800 bg-black">
        <LiveKitRoom
          serverUrl={props.serverUrl}
          token={props.token}
          connect={connect}
          options={roomOptions}
          room={room}
          onConnected={() => setStatus("connected")}
          onDisconnected={() => {
            if (!connect) {
              setStatus("ended");
              return;
            }
            setStatus("reconnecting");
          }}
          onError={(e) => {
            setError(e?.message ?? String(e));
            setStatus("failed");
            setConnect(false);
          }}
        >
          <div className="relative h-full">
            {props.footer ? (
              <div className="absolute right-3 top-3 z-10">{props.footer}</div>
            ) : null}
            <div className="h-full">{props.children}</div>
          </div>
        </LiveKitRoom>
      </div>
    </div>
  );
}
