import type { ChannelEngine } from "../../../engines/useChannelEngine";

export function MessageToolbar(props: {
  e: ChannelEngine;
  messageId: string;
  isPinned: boolean;
  isMe: boolean;
  threadId: string | null;
  panelMode: "expanded" | "voice-first";
}) {
  // panelMode currently only affects placement/visibility; functionality remains identical.
  const { e } = props;

  return (
    <div
      className={`absolute -top-3 ${
        props.isMe ? "left-0 -translate-x-1/2" : "right-0 translate-x-1/2"
      } z-20 flex items-center gap-1 rounded-full border border-slate-700 bg-slate-950/90 px-1 py-1 text-xs text-slate-200 opacity-0 shadow-sm backdrop-blur transition-opacity group-hover:opacity-100`}
    >
      <button
        className="grid h-7 w-7 place-items-center rounded-full hover:bg-slate-900"
        onClick={() => e.setReactionPickerFor((cur) => (cur === props.messageId ? null : props.messageId))}
        type="button"
        aria-label="Add reaction"
        title="React"
      >
        {"\u{1F642}"}
      </button>
      <button
        className={`grid h-7 w-7 place-items-center rounded-full ${props.isPinned ? "flux-chip-active" : "hover:bg-slate-900"}`}
        onClick={() => {
          if (props.isPinned) e.unpin.mutate(props.messageId);
          else e.pin.mutate(props.messageId);
        }}
        type="button"
        aria-label="Pin message"
        title={props.isPinned ? "Unpin" : "Pin"}
      >
        {"\u{1F4CC}"}
      </button>
      <button
        className="grid h-7 w-7 place-items-center rounded-full hover:bg-slate-900"
        onClick={() => {
          e.setWorkPane("threads");
          if (props.threadId) e.setActiveThreadId(props.threadId);
          else e.createThread.mutate({ root_message_id: props.messageId });
        }}
        type="button"
        aria-label="Open thread"
        title="Thread"
      >
        {"\u{1F9F5}"}
      </button>
    </div>
  );
}

