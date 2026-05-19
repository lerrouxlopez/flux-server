import type { ChannelEngine } from "../../../engines/useChannelEngine";
import { MessageItem } from "./MessageItem";

export function MessageList(props: {
  e: ChannelEngine;
  density: "comfortable" | "compact";
  panelMode: "expanded" | "voice-first";
}) {
  if (props.e.messages.isLoading) {
    return (
      <div className="space-y-2" aria-label="Loading messages">
        {Array.from({ length: 6 }).map((_, i) => (
          <div key={i} className="h-10 rounded-lg border border-slate-800 bg-slate-950/20" />
        ))}
      </div>
    );
  }

  if (props.e.messages.isError) {
    return <div className="text-sm text-rose-300">Failed to load messages.</div>;
  }

  return (
    <div className="space-y-2">
      {props.e.visibleMessages.map((m) => (
        <MessageItem key={m.id} e={props.e} m={m} density={props.density} panelMode={props.panelMode} />
      ))}
      <div ref={props.e.bottomRef} />
    </div>
  );
}

