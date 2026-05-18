# Known Quirks (Phase 0)

This doc is intentionally blunt: it captures current rough edges so future changes can burn them down deliberately.

## Local vs production LiveKit

- `community-app/docker-compose.yml` runs LiveKit with `--dev` for convenience only. This is not production-safe (keys/TLS/TURN/etc. are not representative).
- The config crate supports a legacy single `LIVEKIT_URL`, but production should set:
  - `LIVEKIT_URL_INTERNAL` (container-to-container)
  - `LIVEKIT_URL_PUBLIC` (browser-reachable)

## WebSocket auth token transport

- `apps/web/src/realtime/ws.ts` passes the access token via query param `access_token`.
  - This is necessary because browsers can’t reliably set the `Authorization` header for WS.
  - It can leak into logs/proxies/analytics if not careful (treat as a temporary dev compromise).
- `apps/realtime-gateway/src/main.rs` accepts tokens via either header or query param.

## CORS behavior

- API CORS is permissive in local mode:
  - If `APP_ENV=local` (default) or `CORS_ALLOW_ORIGINS` is empty, it allows `Any`.
- Non-local environments should set `CORS_ALLOW_ORIGINS` explicitly (comma-separated).

## Migrations execution

- `api-server` runs SQLx migrations on startup.
- `realtime-gateway` does not run migrations (it only connects to DB).

## “Domain crates” still in progress

Several workspace crates are placeholders right now (`crates/branding`, `crates/chat`, `crates/notifications`, `crates/orgs`).
Practically, much of the feature logic currently lives in:
- `apps/api-server/src/routes_*.rs`
- `apps/worker/src/main.rs`

## Public branding resolution

- `GET /public/branding?host=...` resolves by:
  1) exact `branding_profiles.custom_domain`
  2) org slug from the first label of the host (`acme.localhost` → `acme`)
- Be explicit about what you pass as `host` in local testing.

## Repo size / committed artifacts

- `apps/web/node_modules/` and `apps/web/dist/` exist in-tree. If this repo is meant to stay lean, consider treating these as build artifacts (but that is a policy decision, not a behavior change).

