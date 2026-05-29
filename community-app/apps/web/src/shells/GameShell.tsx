import { OrgSidebar } from "../components/OrgSidebar";
import type { ChannelEngine } from "../engines/useChannelEngine";
import { MessageViewport } from "../features/messaging/components/MessageViewport";
import { TypingIndicator } from "../features/messaging/components/TypingIndicator";
import { Composer } from "../features/messaging/components/Composer";

const EMOJI_PALETTE = ["😀", "😂", "❤️", "👍", "🎉", "🙏", "🔥", "😮", "😢", "😡", "✅", "👀"];

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

          {e.channel?.kind !== "voice" && e.channel?.kind !== "video" ? (
            <div className="flex items-center gap-2">
              <button
                className="flux-btn-primary rounded-md px-3 py-2 text-xs font-semibold disabled:opacity-50"
                disabled={e.createMeeting.isPending}
                onClick={() => e.createMeeting.mutate(undefined)}
                type="button"
                title="Jump into a voice room"
              >
                Voice
              </button>
            </div>
          ) : null}
        </div>

        <MessageViewport
          e={e}
          density="compact"
          panelMode="voice-first"
          className="mt-4 h-[60vh] overflow-auto rounded-lg border border-slate-800 bg-slate-950/30 p-2"
        />

        <TypingIndicator text={e.typingText} />

        <Composer e={e} density="compact" className="mt-3" />
      </section>
    </div>
  );
}
