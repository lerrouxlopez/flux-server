import type { ChannelEngine } from "../../../engines/useChannelEngine";
import { MessageList } from "./MessageList";

export function MessageViewport(props: {
  e: ChannelEngine;
  density: "comfortable" | "compact";
  panelMode: "expanded" | "voice-first";
  className?: string;
}) {
  return (
    <div className={props.className ?? ""} data-density={props.density}>
      <MessageList e={props.e} density={props.density} panelMode={props.panelMode} />
    </div>
  );
}

