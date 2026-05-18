# Tenant isolation audit (org scoping)

## What was checked

- API routes: messages, threads/search, branding, media, org-scoped endpoints.
- Realtime gateway: subscription gating for channels and media rooms.

## Fixes applied

- Media room creation now validates `channel_id` belongs to the `org_id` in the route and that the caller can access the channel (prevents cross-org channel attachment).
- Message listing now includes an explicit `organization_id = ...` filter in SQL (defense-in-depth).

## Automated coverage added

- `apps/api-server/tests/tenant_isolation_full.rs`: creates two orgs/users and asserts cross-tenant denial for:
  - message listing and message edit
  - channel search
  - org branding read
  - media room read + participants list
  - cross-org `channel_id` injection during media room creation
- `apps/realtime-gateway/tests/tenant_isolation_ws_scoping.rs`: validates the websocket subscription guards reject cross-org channel/media room IDs.

