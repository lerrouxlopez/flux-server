import { Avatar } from "../../../components/Avatar";
import type { ChannelEngine } from "../../../engines/useChannelEngine";
import type { Message } from "../../../api/types";
import { AttachmentGrid } from "./AttachmentGrid";
import { ReactionBar } from "./ReactionBar";
import { MessageToolbar } from "./MessageToolbar";

export function MessageItem(props: {
  e: ChannelEngine;
  m: Message;
  density: "comfortable" | "compact";
  panelMode: "expanded" | "voice-first";
}) {
  const { e, m } = props;
  const isMe = (e.meId && m.sender_id === e.meId) || m.sender_id === "me";
  const senderName = e.memberById.get(m.sender_id)?.display_name ?? m.sender_id.slice(0, 8);
  const isPinned = e.pinnedIds.has(m.id);
  const threadId = m.thread_id ?? null;

  return (
    <div className={`group flex ${isMe ? "justify-end" : "justify-start"}`}>
      <div className={`flex max-w-[92%] items-end gap-2 ${isMe ? "flex-row-reverse" : "flex-row"}`}>
        {!isMe ? <Avatar name={senderName} size={props.density === "compact" ? 26 : 28} online={e.presenceByUser[m.sender_id] === "online"} /> : null}
        <div className="min-w-0">
          <div className={`mb-0.5 text-[11px] text-slate-500 ${isMe ? "text-right" : "text-left"}`}>{senderName}</div>

          <div className="relative">
            <div ref={e.reactionPickerFor === m.id ? e.reactionPickerRef : undefined}>
              <div className={`relative w-fit rounded-2xl ${isMe ? "ml-auto" : "mr-auto"} ${props.density === "compact" ? "px-2.5 py-1.5 text-sm leading-5" : "px-3 py-2 text-sm leading-6"} ${isMe ? "flux-bubble-me" : "flux-bubble-other"}`}>
                <MessageToolbar
                  e={e}
                  messageId={m.id}
                  isPinned={isPinned}
                  isMe={isMe}
                  threadId={threadId}
                  panelMode={props.panelMode}
                />

                {e.reactionPickerFor === m.id ? (
                  <div className={`absolute -top-12 ${isMe ? "left-0" : "right-0"} z-20 flex items-center gap-1 rounded-full border border-slate-700 bg-slate-950/95 px-2 py-1 shadow-lg backdrop-blur`}>
                    {e.QUICK_REACTIONS.map((emoji) => (
                      <button
                        key={emoji}
                        className="grid h-8 w-8 place-items-center rounded-full text-base hover:bg-slate-800/70"
                        onClick={() => {
                          e.setReactionPickerFor(null);
                          e.addReaction.mutate({ messageId: m.id, emoji });
                        }}
                        type="button"
                        title={emoji}
                      >
                        {emoji}
                      </button>
                    ))}
                  </div>
                ) : null}

                {m.body ? <div className="whitespace-pre-wrap">{m.body}</div> : null}
                {(m.attachments ?? []).length ? (
                  <div className={`${m.body ? "mt-2" : ""}`}>
                    <AttachmentGrid attachments={m.attachments ?? []} density={props.density} />
                  </div>
                ) : null}
              </div>
            </div>
          </div>

          <ReactionBar e={e} messageId={m.id} reactions={m.reactions ?? []} align={isMe ? "end" : "start"} />
        </div>
      </div>
    </div>
  );
}

