import type { ChannelEngine } from "../../../engines/useChannelEngine";
import type { MessageReaction } from "../../../api/types";

export function ReactionBar(props: {
  e: ChannelEngine;
  messageId: string;
  reactions: MessageReaction[];
  align: "start" | "end";
}) {
  const reactions = props.reactions ?? [];
  if (!reactions.length) return null;

  return (
    <div
      className={`mt-1 flex flex-wrap gap-1 ${props.align === "end" ? "justify-end" : "justify-start"}`}
      aria-label="Reactions"
    >
      {reactions.map((r) => (
        <button
          key={r.emoji}
          className={`rounded-full border px-2 py-0.5 text-xs ${
            r.reacted_by_me
              ? "flux-chip-active"
              : "border-slate-700 bg-slate-900/40 text-slate-200 hover:bg-slate-800/60"
          }`}
          onClick={() => {
            if (r.reacted_by_me) props.e.removeReaction.mutate({ messageId: props.messageId, emoji: r.emoji });
            else props.e.addReaction.mutate({ messageId: props.messageId, emoji: r.emoji });
          }}
          type="button"
        >
          {r.emoji} {r.count}
        </button>
      ))}
    </div>
  );
}

