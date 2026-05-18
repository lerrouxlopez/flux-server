import { useChannelEngine } from "../engines/useChannelEngine";
import { GameShell } from "../shells/GameShell";
import { WorkShell } from "../shells/WorkShell";

export function ChannelPage() {
  const e = useChannelEngine();
  return e.uiMode === "play" ? <GameShell e={e} /> : <WorkShell e={e} />;
}

