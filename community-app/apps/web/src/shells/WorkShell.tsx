import { Link, useNavigate } from "react-router-dom";
import { Button } from "../components/Button";
import { OrgSidebar } from "../components/OrgSidebar";
import { Avatar } from "../components/Avatar";
import { TextArea } from "../components/TextArea";
import type { ChannelEngine } from "../engines/useChannelEngine";

export function WorkShell({ e }: { e: ChannelEngine }) {
  const nav = useNavigate();

  if (e.orgs.isLoading) return <div className="text-slate-300">Loading...</div>;
  if (!e.org) return <div className="text-slate-300">Org not found.</div>;
  if (!e.channel) return <div className="text-slate-300">Channel not found.</div>;

  return (
    <div className="grid gap-6 md:grid-cols-[280px_1fr]" data-testid="work-shell">
      <OrgSidebar org={e.org} activeChannelId={e.channel_id} presenceByUser={e.presenceByUser} />

      <section className="rounded-xl border border-slate-800 bg-slate-900/30 p-4">
        <div className="flex items-center justify-between">
          <div className="flex items-center gap-2">
            {!e.editingChannelName ? (
              <div className="text-lg font-semibold">{e.channelTitle}</div>
            ) : (
              <div className="flex items-center gap-2">
                <input
                  className="w-[240px] rounded-md border border-slate-700 bg-slate-950/40 px-2 py-1 text-sm text-slate-100 outline-none focus:border-indigo-500"
                  value={e.channelNameDraft}
                  onChange={(ev) => e.setChannelNameDraft(ev.target.value)}
                  onKeyDown={(ev) => {
                    if (ev.key === "Escape") e.setEditingChannelName(false);
                    if (ev.key === "Enter") {
                      ev.preventDefault();
                      const next = e.channelNameDraft.trim();
                      if (!next) return;
                      e.updateChannel.mutate({ name: next });
                    }
                  }}
                  aria-label="Channel name"
                />
                <Button
                  className="bg-indigo-600 px-2 py-1 text-xs hover:bg-indigo-500"
                  disabled={e.updateChannel.isPending || !e.channelNameDraft.trim()}
                  onClick={() => e.updateChannel.mutate({ name: e.channelNameDraft.trim() })}
                  type="button"
                >
                  Save
                </Button>
                <Button
                  className="bg-slate-800 px-2 py-1 text-xs hover:bg-slate-700"
                  disabled={e.updateChannel.isPending}
                  onClick={() => e.setEditingChannelName(false)}
                  type="button"
                >
                  Cancel
                </Button>
              </div>
            )}

            <div className="ml-2 flex items-center gap-1">
              <button
                className={`rounded-md px-2 py-1 text-xs ${
                  e.workPane === "search" ? "bg-slate-800 text-white" : "bg-slate-900 text-slate-200 hover:bg-slate-800"
                }`}
                onClick={() => {
                  if (e.workPane === "search") {
                    e.setWorkSearch("");
                    e.setWorkPane(null);
                    e.setWorkSearchOpen(false);
                    return;
                  }
                  e.setWorkSearchOpen(true);
                  e.setWorkPane("search");
                }}
                type="button"
                title="Search"
              >
                Search
              </button>
              <button
                className={`rounded-md px-2 py-1 text-xs ${
                  e.workPane === "pins" ? "bg-slate-800 text-white" : "bg-slate-900 text-slate-200 hover:bg-slate-800"
                }`}
                onClick={() => {
                  e.setActiveThreadId(null);
                  e.setThreadDraft("");
                  e.setWorkPane((p) => (p === "pins" ? null : "pins"));
                }}
                type="button"
                title="Pins"
              >
                Pins
              </button>
              <button
                className={`rounded-md px-2 py-1 text-xs ${
                  e.workPane === "threads" ? "bg-slate-800 text-white" : "bg-slate-900 text-slate-200 hover:bg-slate-800"
                }`}
                onClick={() => {
                  e.setThreadDraft("");
                  e.setWorkPane((p) => {
                    const next = p === "threads" ? null : "threads";
                    if (next === null) e.setActiveThreadId(null);
                    return next;
                  });
                }}
                type="button"
                title="Threads"
              >
                Threads
              </button>
              <button
                className="rounded-md bg-indigo-600 px-2 py-1 text-xs font-semibold text-white hover:bg-indigo-500 disabled:opacity-50"
                disabled={e.createMeeting.isPending}
                onClick={() => e.createMeeting.mutate()}
                title="Start meeting"
                type="button"
                data-testid="meeting-control"
              >
                Start meeting
              </button>
            </div>
          </div>

          <div className="flex items-center gap-3">
            <div className="text-xs text-slate-400">{e.connected ? "realtime online" : "realtime offline"}</div>
            <Link className="text-xs text-slate-300 hover:text-white" to={`/app/${e.org!.slug}/friends`}>
              Friends
            </Link>
            {e.canSeeAdmin ? (
              <button
                className="text-xs text-slate-300 hover:text-white"
                onClick={() => nav(`/admin/${e.org!.slug}`)}
                type="button"
              >
                Admin
              </button>
            ) : null}
          </div>
        </div>

        {e.workPane === "search" && e.workSearchOpen ? (
          <div className="mt-3 flex items-center gap-2 rounded-lg border border-slate-800 bg-slate-950/30 px-3 py-2">
            <input
              className="w-full rounded-md border border-slate-800 bg-slate-900 px-3 py-2 text-sm text-slate-200 outline-none focus:border-indigo-500"
              placeholder="Search messages in this channel..."
              value={e.workSearch}
              onChange={(ev) => e.setWorkSearch(ev.target.value)}
            />
            <button
              className="rounded-md bg-slate-900 px-3 py-2 text-xs text-slate-200 hover:bg-slate-800"
              onClick={() => {
                e.setWorkSearch("");
                e.setWorkPane(null);
                e.setWorkSearchOpen(false);
              }}
              type="button"
              title="Close search"
            >
              Close
            </button>
          </div>
        ) : null}

        {e.workPane && e.workPane !== "search" ? (
          <div className="mt-3 rounded-lg border border-slate-800 bg-slate-950/30 px-3 py-2 text-sm text-slate-200">
            <div className="flex items-center justify-between">
              <div className="font-semibold capitalize">
                {e.workPane === "threads" ? (e.activeThreadId ? "Thread" : "Threads") : "Pins"}
              </div>
              <div className="flex items-center gap-2">
                {e.workPane === "threads" && e.activeThreadId ? (
                  <button
                    className="rounded-md bg-slate-900 px-3 py-2 text-xs text-slate-200 hover:bg-slate-800"
                    onClick={() => {
                      e.setActiveThreadId(null);
                      e.setThreadDraft("");
                    }}
                    type="button"
                  >
                    Back
                  </button>
                ) : null}
                <button
                  className="rounded-md bg-slate-900 px-3 py-2 text-xs text-slate-200 hover:bg-slate-800"
                  onClick={() => {
                    e.setWorkPane(null);
                    e.setActiveThreadId(null);
                    e.setThreadDraft("");
                  }}
                  type="button"
                >
                  Close
                </button>
              </div>
            </div>

            {e.workPane === "pins" ? (
              <div className="mt-2 space-y-2">
                {e.pins.isLoading ? <div className="text-xs text-slate-400">Loading pins...</div> : null}
                {e.pins.isError ? <div className="text-xs text-rose-300">Failed to load pins.</div> : null}
                {!e.pins.isLoading && !e.pins.isError && (e.pins.data?.pins ?? []).length === 0 ? (
                  <div className="text-xs text-slate-400">No pinned messages yet.</div>
                ) : null}
                {(e.pins.data?.pins ?? []).map((p) => {
                  const msg = p.message;
                  const senderName = e.memberById.get(msg.sender_id)?.display_name ?? msg.sender_id.slice(0, 8);
                  const pinnedByName = e.memberById.get(p.pinned_by)?.display_name ?? p.pinned_by.slice(0, 8);
                  return (
                    <div key={p.message_id} className="rounded-lg border border-slate-800 bg-slate-950/20 p-2">
                      <div className="flex items-start justify-between gap-3">
                        <div className="min-w-0">
                          <div className="text-xs text-slate-400">
                            {senderName} · {pinnedByName}
                          </div>
                          <div className="mt-1 truncate text-sm text-slate-200">{msg.body ?? "(no text)"}</div>
                        </div>
                        <button
                          className="shrink-0 rounded-md bg-slate-900 px-2 py-1 text-xs text-slate-200 hover:bg-slate-800"
                          onClick={() => e.unpin.mutate(p.message_id)}
                          type="button"
                          title="Unpin"
                        >
                          Unpin
                        </button>
                      </div>
                    </div>
                  );
                })}
              </div>
            ) : null}

            {e.workPane === "threads" ? (
              <div className="mt-2 space-y-3">
                {e.activeThreadId ? (
                  <>
                    {e.thread.isLoading ? <div className="text-xs text-slate-400">Loading thread...</div> : null}
                    {e.thread.isError ? <div className="text-xs text-rose-300">Failed to load thread.</div> : null}
                    {e.thread.data ? (
                      <div className="space-y-3">
                        <div className="rounded-lg border border-slate-800 bg-slate-950/20 p-2">
                          <div className="text-xs text-slate-400">Root</div>
                          <div className="mt-1 text-sm text-slate-200">{e.thread.data.root?.body ?? "(no text)"}</div>
                        </div>
                        <div className="space-y-2">
                          <div className="text-xs text-slate-400">Replies</div>
                          {(e.thread.data.replies ?? []).length === 0 ? (
                            <div className="text-xs text-slate-500">No replies yet.</div>
                          ) : null}
                          {(e.thread.data.replies ?? []).map((m) => {
                            const senderName = e.memberById.get(m.sender_id)?.display_name ?? m.sender_id.slice(0, 8);
                            return (
                              <div key={m.id} className="rounded-lg border border-slate-800 bg-slate-950/10 p-2">
                                <div className="text-xs text-slate-400">{senderName}</div>
                                <div className="mt-1 text-sm text-slate-200">{m.body ?? ""}</div>
                              </div>
                            );
                          })}
                        </div>

                        <form
                          className="flex items-end gap-2"
                          onSubmit={(ev) => {
                            ev.preventDefault();
                            const body = e.threadDraft.trim();
                            if (!body || !e.activeThreadId) return;
                            e.replyToThread.mutate({ thread_id: e.activeThreadId, body });
                          }}
                        >
                          <TextArea
                            rows={2}
                            value={e.threadDraft}
                            placeholder="Reply in thread..."
                            onChange={(ev) => e.setThreadDraft(ev.target.value)}
                            className="min-h-[60px] flex-1 resize-none rounded-md border border-slate-800 bg-slate-900 px-3 py-2 text-sm text-slate-200 outline-none focus:border-indigo-500"
                          />
                          <Button
                            className="bg-indigo-600 px-3 py-2 text-xs hover:bg-indigo-500"
                            disabled={e.replyToThread.isPending || !e.threadDraft.trim()}
                            type="submit"
                          >
                            Reply
                          </Button>
                        </form>
                      </div>
                    ) : null}
                  </>
                ) : (
                  <>
                    <form
                      className="flex items-end gap-2"
                      onSubmit={(ev) => {
                        ev.preventDefault();
                        const body = e.newThreadDraft.trim();
                        if (!body) return;
                        e.createThread.mutate({ body });
                        e.setNewThreadDraft("");
                      }}
                    >
                      <TextArea
                        rows={2}
                        value={e.newThreadDraft}
                        placeholder="Start a new thread..."
                        onChange={(ev) => e.setNewThreadDraft(ev.target.value)}
                        className="min-h-[60px] flex-1 resize-none rounded-md border border-slate-800 bg-slate-900 px-3 py-2 text-sm text-slate-200 outline-none focus:border-indigo-500"
                      />
                      <Button
                        className="bg-indigo-600 px-3 py-2 text-xs hover:bg-indigo-500"
                        disabled={e.createThread.isPending || !e.newThreadDraft.trim()}
                        type="submit"
                      >
                        Create
                      </Button>
                    </form>

                    {e.threads.isLoading ? <div className="text-xs text-slate-400">Loading threads...</div> : null}
                    {e.threads.isError ? <div className="text-xs text-rose-300">Failed to load threads.</div> : null}
                    {!e.threads.isLoading && !e.threads.isError && (e.threads.data?.threads ?? []).length === 0 ? (
                      <div className="text-xs text-slate-400">No threads yet.</div>
                    ) : null}
                    <div className="space-y-2">
                      {(e.threads.data?.threads ?? []).map((t) => {
                        const root = t.root;
                        const senderName = root
                          ? e.memberById.get(root.sender_id)?.display_name ?? root.sender_id.slice(0, 8)
                          : "Unknown";
                        return (
                          <button
                            key={t.thread.id}
                            className="w-full rounded-lg border border-slate-800 bg-slate-950/10 p-2 text-left hover:bg-slate-900/40"
                            onClick={() => e.setActiveThreadId(t.thread.id)}
                            type="button"
                          >
                            <div className="flex items-center justify-between gap-2">
                              <div className="min-w-0">
                                <div className="text-xs text-slate-400">{senderName}</div>
                                <div className="mt-1 truncate text-sm text-slate-200">{root?.body ?? "(no text)"}</div>
                              </div>
                              <div className="shrink-0 rounded-full bg-slate-900 px-2 py-0.5 text-xs text-slate-200">
                                {t.reply_count}
                              </div>
                            </div>
                          </button>
                        );
                      })}
                    </div>
                  </>
                )}
              </div>
            ) : null}
          </div>
        ) : null}

        <div className="mt-4 h-[60vh] overflow-auto rounded-lg border border-slate-800 bg-slate-950/30 p-3" data-density="comfortable">
          <MessageList e={e} />
        </div>

        {e.typingText ? <div className="mt-2 text-xs text-slate-400">{e.typingText}</div> : null}

        <Composer e={e} density="comfortable" />
      </section>
    </div>
  );
}

function MessageList({ e }: { e: ChannelEngine }) {
  if (e.messages.isLoading) {
    return (
      <div className="space-y-3" aria-label="Loading messages">
        {Array.from({ length: 6 }).map((_, i) => (
          <div key={i} className="h-10 rounded-lg border border-slate-800 bg-slate-950/20" />
        ))}
      </div>
    );
  }
  if (e.messages.isError) {
    return <div className="text-sm text-rose-300">Failed to load messages.</div>;
  }

  return (
    <div className="space-y-3">
      {e.visibleMessages.map((m) => {
        const isMe = (e.meId && m.sender_id === e.meId) || m.sender_id === "me";
        const senderName = e.memberById.get(m.sender_id)?.display_name ?? m.sender_id.slice(0, 8);
        const isPinned = e.pinnedIds.has(m.id);
        const threadId = m.thread_id ?? null;
        return (
          <div key={m.id} className={`group flex ${isMe ? "justify-end" : "justify-start"}`}>
            <div className={`flex max-w-[88%] items-end gap-2 ${isMe ? "flex-row-reverse" : "flex-row"}`}>
              {!isMe ? <Avatar name={senderName} size={28} online={e.presenceByUser[m.sender_id] === "online"} /> : null}
              <div className="min-w-0">
                <div className={`mb-1 text-xs text-slate-500 ${isMe ? "text-right" : "text-left"}`}>{senderName}</div>
                <div
                  className={`relative w-fit rounded-2xl px-3 py-2 text-sm leading-6 ${isMe ? "ml-auto" : "mr-auto"}`}
                  style={{
                    backgroundColor: isMe ? "var(--chat-bubble-me-bg, #4f46e5)" : "var(--chat-bubble-other-bg, #1f2937)",
                    color: isMe ? "var(--chat-bubble-me-text, #ffffff)" : "var(--chat-bubble-other-text, #e2e8f0)",
                  }}
                >
                  <div ref={e.reactionPickerFor === m.id ? e.reactionPickerRef : undefined}>
                    <div
                      className={`absolute -top-3 ${isMe ? "left-0 -translate-x-1/2" : "right-0 translate-x-1/2"} z-20 flex items-center gap-1 rounded-full border border-slate-700 bg-slate-950/90 px-1 py-1 text-xs text-slate-200 opacity-0 shadow-sm backdrop-blur transition-opacity group-hover:opacity-100`}
                    >
                      <button
                        className="grid h-7 w-7 place-items-center rounded-full hover:bg-slate-900"
                        onClick={() => e.setReactionPickerFor((cur) => (cur === m.id ? null : m.id))}
                        type="button"
                        aria-label="Add reaction"
                        title="React"
                      >
                        {"\u{1F642}"}
                      </button>
                      <button
                        className={`grid h-7 w-7 place-items-center rounded-full ${
                          isPinned ? "bg-indigo-500/20 text-indigo-100" : "hover:bg-slate-900"
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
                              <a className="text-sm text-indigo-200 underline" download={a.filename} href={a.download_url}>
                                {a.filename}
                              </a>
                            )}
                          </div>
                        );
                      })}
                    </div>
                  ) : null}
                </div>
              </div>
            </div>
          </div>
        );
      })}
      <div ref={e.bottomRef} />
    </div>
  );
}

function Composer({ e, density }: { e: ChannelEngine; density: "comfortable" | "compact" }) {
  return (
    <form className="mt-4" onSubmit={e.onSubmit}>
      <div
        className={`flex items-end gap-2 rounded-xl border border-slate-800 bg-slate-950/30 px-3 py-2 focus-within:border-indigo-500/70 focus-within:ring-1 focus-within:ring-indigo-500/30 ${
          density === "compact" ? "text-sm" : "text-sm"
        }`}
      >
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
            el.style.height = Math.min(el.scrollHeight, 180) + "px";
          }}
          className="max-h-[180px] flex-1 resize-none border-0 bg-transparent px-0 py-0 text-sm leading-6 focus:border-0"
        />
        <button
          className="grid h-9 w-9 place-items-center rounded-md text-sm text-slate-300 hover:bg-slate-800/60 hover:text-slate-100"
          onClick={() => e.fileInputRef.current?.click()}
          type="button"
          title="Attach"
        >
          {"\u{1F4CE}"}
        </button>
        <div className="mx-1 h-6 w-px bg-slate-800" />
        <button
          className={`grid h-9 w-9 place-items-center rounded-md text-sm ${
            e.send.isPending ? "bg-slate-800 text-slate-400" : "bg-indigo-600 text-white hover:bg-indigo-500"
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
  );
}
