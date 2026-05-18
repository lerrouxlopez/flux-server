# Manual QA: Messaging (Threads, Pins, Search)

## Prereqs
- `community-app` backend + realtime gateway + web running locally.
- Two test users (two separate browser profiles or two browsers).

## Scenario A: Threads (same org + same channel)
1. Browser A: create an org + a text channel, open the channel page.
2. Browser B: register/login and join the same org, open the same channel.
3. Browser A: send a message ("thread root").
4. Browser A (hover message): click the thread icon to create/open a thread.
5. In the thread panel: post a reply.
6. Browser B: open the thread panel and verify the reply appears without a full page refresh.
7. Browser A: verify the Threads pane shows the thread and the reply count increments.

## Scenario B: Pins (per-channel, realtime)
1. Browser A: pin a message in the channel.
2. Browser B: open Pins pane and verify the pinned message appears.
3. Browser B: unpin the same message.
4. Browser A: open Pins pane and verify the pinned message is removed.

## Scenario C: Channel search (server-backed)
1. Browser A: send a message with a distinctive keyword (e.g., "unicorn-123").
2. Browser A: open Search and query `unicorn-123`.
3. Verify results include the message and only include messages from the current channel.

## Scenario D: Tenant isolation (cross-org)
1. Browser A: create Org 1 + Channel 1.
2. Browser B: create Org 2 + Channel 2.
3. Browser A: attempt to navigate directly to Channel 2 by URL.
4. Verify the UI does not load messages/pins/threads for a channel in another org (server should return 403).

## Notes
- Pin limit is enforced per channel (current default: 50).
