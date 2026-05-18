import { OrgSidebar } from "../components/OrgSidebar";
import { Avatar } from "../components/Avatar";
import { TextArea } from "../components/TextArea";
import type { ChannelEngine } from "../engines/useChannelEngine";

export function GameShell({ e }: { e: ChannelEngine }) {
  if (e.orgs.isLoading) return <div className="text-slate-300">Loading...</div>;
  if (!e.org) return <div className="text-slate-300">Org not found.</div>;
  if (!e.channel) return <div className="text-slate-300">Channel not found.</div>;

  const onlineCount = Object.values(e.presenceByUser).filter((s) => s === "online").length;

  return (
    <div className="grid gap-6 md:grid-cols-[280px_1fr]" data-testid="game-shell">
      <OrgSidebar org={e.org} activeChannelId={e.channel_id} presenceByUser={e.presenceByUser} />

      <section className="rounded-xl border border-slate-800 bg-slate-900/30 p-4">
        <div className="flex items-center justify-between">
          <div className="min-w-0">
            <div className="truncate text-lg font-semibold">{e.channelTitle}</div>
            <div className="mt-0.5 text-xs text-slate-400">
              {e.connected ? "realtime online" : "realtime offline"} · {onlineCount} online
            </div>
          </div>

          <div className="flex items-center gap-2">
            <button
              className="flux-btn-primary rounded-md px-3 py-2 text-xs font-semibold disabled:opacity-50"
              disabled={e.createMeeting.isPending}
              onClick={() => e.createMeeting.mutate()}
              type="button"
              title="Jump into a voice room"
            >
              Voice
            </button>
            <button
              className={`rounded-md px-3 py-2 text-xs ${
                e.emojiOpen ? "bg-slate-800 text-white" : "bg-slate-900 text-slate-200 hover:bg-slate-800"
              }`}
              onClick={() => e.setEmojiOpen((v) => !v)}
              type="button"
              title="Reactions"
            >
              Reactions
            </button>
          </div>
        </div>

        {e.emojiOpen ? (
          <div className="mt-3 flex flex-wrap gap-1 rounded-lg border border-slate-800 bg-slate-950/30 p-2">
            {["😀", "😂", "❤️", "👍", "🎉", "🙏", "🔥", "😮", "😢", "😡", "✅", "👀"].map((emoji) => (
              <button
                key={emoji}
                className="grid h-9 w-9 place-items-center rounded-md text-base hover:bg-slate-800/60"
                onClick={() => {
                  e.setText((t) => (t ? t + " " + emoji : emoji));
                }}
                type="button"
                title={emoji}
              >
                {emoji}
              </button>
            ))}
          </div>
        ) : null}

        <div
          className="mt-4 h-[60vh] overflow-auto rounded-lg border border-slate-800 bg-slate-950/30 p-2"
          data-density="compact"
        >
          <div className="space-y-2">
            {e.messages.isLoading ? (
              <div className="space-y-2" aria-label="Loading messages">
                {Array.from({ length: 6 }).map((_, i) => (
                  <div key={i} className="h-10 rounded-lg border border-slate-800 bg-slate-950/20" />
                ))}
              </div>
            ) : null}
            {e.messages.isError ? <div className="text-sm text-rose-300">Failed to load messages.</div> : null}
            {!e.messages.isLoading && !e.messages.isError
              ? e.visibleMessages.map((m) => {
              const isMe = (e.meId && m.sender_id === e.meId) || m.sender_id === "me";
              const senderName = e.memberById.get(m.sender_id)?.display_name ?? m.sender_id.slice(0, 8);
              const isPinned = e.pinnedIds.has(m.id);
              const threadId = m.thread_id ?? null;
              return (
                <div key={m.id} className={`group flex ${isMe ? "justify-end" : "justify-start"}`}>
                  <div className={`flex max-w-[92%] items-end gap-2 ${isMe ? "flex-row-reverse" : "flex-row"}`}>
                    {!isMe ? <Avatar name={senderName} size={26} online={e.presenceByUser[m.sender_id] === "online"} /> : null}
                    <div className="min-w-0">
                      <div className={`mb-0.5 text-[11px] text-slate-500 ${isMe ? "text-right" : "text-left"}`}>
                        {senderName}
                      </div>
                      <div
                        className={`relative w-fit rounded-2xl px-2.5 py-1.5 text-sm leading-5 ${isMe ? "ml-auto" : "mr-auto"}`}
                        style={{
                          backgroundColor: isMe ? "var(--chat-bubble-me-bg, #4f46e5)" : "var(--chat-bubble-other-bg, #1f2937)",
                          color: isMe ? "var(--chat-bubble-me-text, #ffffff)" : "var(--chat-bubble-other-text, #e2e8f0)",
                        }}
                      >
                        <div className="absolute -top-3 right-2 z-20 flex items-center gap-1 rounded-full border border-slate-700 bg-slate-950/90 px-1 py-1 text-xs text-slate-200 opacity-0 shadow-sm backdrop-blur transition-opacity group-hover:opacity-100">
                          {e.QUICK_REACTIONS.slice(0, 3).map((emoji) => (
                            <button
                              key={emoji}
                              className="grid h-7 w-7 place-items-center rounded-full text-base hover:bg-slate-900"
                              onClick={() => e.addReaction.mutate({ messageId: m.id, emoji })}
                              type="button"
                              title={emoji}
                            >
                              {emoji}
                            </button>
                          ))}
                          <button
                            className={`grid h-7 w-7 place-items-center rounded-full ${
                              isPinned ? "flux-chip-active" : "hover:bg-slate-900"
                            }`}
                            onClick={() => {
                              if (isPinned) e.unpin.mutate(m.id);
                              else e.pin.mutate(m.id);
                            }}
                            type="button"
                            aria-label="Pin message"
                            title={isPinned ? "Unpin" : "Pin"}
                          >
                            {"\u{1F4CC}"}
                          </button>
                          <button
                            className="grid h-7 w-7 place-items-center rounded-full hover:bg-slate-900"
                            onClick={() => {
                              e.setWorkPane("threads");
                              if (threadId) e.setActiveThreadId(threadId);
                              else e.createThread.mutate({ root_message_id: m.id });
                            }}
                            type="button"
                            aria-label="Open thread"
                            title="Thread"
                          >
                            {"\u{1F9F5}"}
                          </button>
                        </div>

                        {m.body ? <div className="whitespace-pre-wrap">{m.body}</div> : null}
                        {(m.attachments ?? []).length ? (
                          <div className={`${m.body ? "mt-2" : ""} space-y-2`}>
                            {(m.attachments ?? []).map((a) => {
                              const isImage = (a.content_type ?? "").startsWith("image/");
                              return (
                                <div key={a.id} className="rounded-xl border border-white/10 bg-black/10 p-2">
                                  {isImage ? (
                                    <img alt={a.filename} className="max-h-64 w-auto rounded-lg" src={a.download_url} />
                                  ) : (
                                  <a className="flux-link text-sm" download={a.filename} href={a.download_url}>
                                    {a.filename}
                                  </a>
                                )}
                                </div>
                              );
                            })}
                          </div>
                        ) : null}
                      </div>
                      <div className={`mt-1 flex flex-wrap gap-1 ${isMe ? "justify-end" : "justify-start"}`}>
                        {(m.reactions ?? []).map((r) => (
                          <button
                            key={r.emoji}
                            className={`rounded-full border px-2 py-0.5 text-xs ${
                              r.reacted_by_me
                                ? "flux-chip-active"
                                : "border-slate-700 bg-slate-900/40 text-slate-200 hover:bg-slate-800/60"
                            }`}
                            onClick={() => {
                              if (r.reacted_by_me) e.removeReaction.mutate({ messageId: m.id, emoji: r.emoji });
                              else e.addReaction.mutate({ messageId: m.id, emoji: r.emoji });
                            }}
                            type="button"
                          >
                            {r.emoji} {r.count}
                          </button>
                        ))}
                      </div>
                    </div>
                  </div>
                </div>
              );
              })
              : null}
            <div ref={e.bottomRef} />
          </div>
        </div>

        {e.typingText ? <div className="mt-2 text-xs text-slate-400">{e.typingText}</div> : null}

        <form className="mt-3" onSubmit={e.onSubmit}>
          <div className="flux-focus-within flex items-end gap-2 rounded-xl border border-slate-800 bg-slate-950/30 px-3 py-2">
            <TextArea
              ref={e.textAreaRef}
              rows={1}
              value={e.text}
              placeholder="Type a message"
              onChange={(ev) => e.onTypingChange(ev.target.value)}
              onKeyDown={(ev) => {
                if (ev.key === "Enter" && !ev.shiftKey) {
                  ev.preventDefault();
                  (ev.currentTarget.form as HTMLFormElement | null)?.requestSubmit();
                }
              }}
              onInput={(ev) => {
                const el = ev.currentTarget;
                el.style.height = "";
                el.style.height = Math.min(el.scrollHeight, 160) + "px";
              }}
              className="max-h-[160px] flex-1 resize-none border-0 bg-transparent px-0 py-0 text-sm leading-6 focus:border-0"
            />
            <button
              className="grid h-9 w-9 place-items-center rounded-md text-sm text-slate-300 hover:bg-slate-800/60 hover:text-slate-100"
              onClick={() => e.fileInputRef.current?.click()}
              type="button"
              title="Attach"
            >
              {"\u{1F4CE}"}
            </button>
            <button
              className={`grid h-9 w-9 place-items-center rounded-md text-sm ${
                e.send.isPending ? "bg-slate-800 text-slate-400" : "flux-btn-primary"
              }`}
              disabled={e.send.isPending}
              type="submit"
              title="Send"
            >
              {"\u{27A4}"}
            </button>
          </div>
          <input
            accept="image/*,application/pdf,text/plain"
            className="hidden"
            multiple
            onChange={(ev) => {
              const files = Array.from(ev.target.files ?? []);
              if (!files.length) return;
              Promise.all(
                files.map(
                  (f) =>
                    new Promise<{ filename: string; content_type?: string; data_url: string }>((resolve, reject) => {
                      const fr = new FileReader();
                      fr.onerror = () => reject(new Error("read failed"));
                      fr.onload = () =>
                        resolve({
                          filename: f.name,
                          content_type: f.type || undefined,
                          data_url: typeof fr.result === "string" ? fr.result : "",
                        });
                      fr.readAsDataURL(f);
                    }),
                ),
              )
                .then((atts) => e.setPendingAttachments((prev) => [...prev, ...atts.filter((a) => a.data_url)]))
                .catch(() => {});
              ev.target.value = "";
            }}
            ref={e.fileInputRef}
            type="file"
          />
        </form>
      </section>
    </div>
  );
}
