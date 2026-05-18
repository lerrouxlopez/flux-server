# Manual QA — Media Realtime (Two Browsers)

Goal: verify media lifecycle events are org-scoped, room-scoped, and update the VoiceDock UI without polling.

## Setup

From `community-app/`:
- Start stack: `docker compose up -d --build`
- Web: `http://localhost:5173`

Open two browsers (or one normal + one incognito):
- Browser A (User A)
- Browser B (User B)

## Happy path: participant join/leave updates

1. Browser A: register + login, create org `acme`, open the voice channel, start a media room and enter it.
2. Browser A: confirm VoiceDock shows `1 participant`.
3. Browser B: register + login, join the same org (invite or add member), enter the same media room.
4. Browser A + B: confirm VoiceDock shows `2 participants` within ~1s (no page refresh).
5. Browser B: click Back (leave media room).
6. Browser A: confirm VoiceDock returns to `1 participant` within ~1s.

## Reconnect behavior

1. Browser A: while in the media room, temporarily disable network for ~5–10 seconds.
2. Re-enable network.
3. Confirm the media page shows `Reconnecting…` then returns to `Connected`.
4. Confirm participant count does not keep increasing after reconnects.

## Tenant isolation (no cross-org leaks)

1. Browser A: stay in org `acme` media room.
2. Browser B: create a different org `beta` and enter a media room there.
3. Confirm Browser A’s VoiceDock participant count does NOT change due to Browser B’s activity.

## Debug notes

- Media events are delivered over the existing realtime gateway WebSocket. The media page displays a small “Realtime / Realtime…” label to indicate WS connectivity.

