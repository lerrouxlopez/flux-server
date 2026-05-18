# Flux (community-app) — Developer Notes

This file is the “Phase 0” dev handoff: how to run the stack locally, what to configure, and what commands exist today.

## Quickstart (recommended): Docker Compose

From `community-app/`:

1. Start the full stack:
   - `docker compose up -d --build`
2. Open the web app:
   - `http://localhost:5173`

Health checks:
- API: `http://localhost:8080/healthz`
- Realtime gateway: `http://localhost:8081/health`

Local DB admin (Adminer):
- `http://localhost:8082`
- System: `PostgreSQL`
- Server: `postgres` (from another container) or `localhost` (from your host)
- Username: `app`
- Password: `app`
- Database: `community_app`

## Running services without Docker (optional)

The repo’s `community-app/.env.example` documents the minimum env vars expected by the Rust services.

From `community-app/`:

- API server:
  - `cargo run -p api-server`
- Realtime gateway:
  - `cargo run -p realtime-gateway`
- Worker:
  - `cargo run -p worker`
- Web dev server:
  - `cd apps/web`
  - `npm run dev`

Notes:
- `api-server` runs SQLx migrations on startup.
- `realtime-gateway` does not run migrations (it only connects to the DB).

## Environment variables

Rust services read configuration via `crates/config::AppConfig::from_env()`:

- `APP_ENV` (default `local`)
- `HTTP_ADDR` (default `0.0.0.0:8080`)
- `WS_ADDR` (default `0.0.0.0:8081`)
- `DATABASE_URL`
- `REDIS_URL`
- `NATS_URL`
- `JWT_ACCESS_SECRET`
- `JWT_REFRESH_SECRET`
- `ACCESS_TOKEN_TTL_SECONDS` (default `900`)
- `REFRESH_TOKEN_TTL_SECONDS` (default `2592000`)
- LiveKit:
  - `LIVEKIT_URL_INTERNAL` (or legacy `LIVEKIT_URL`)
  - `LIVEKIT_URL_PUBLIC` (or legacy `LIVEKIT_URL`)
  - `LIVEKIT_API_KEY`
  - `LIVEKIT_API_SECRET`

API server CORS:
- `CORS_ALLOW_ORIGINS` (comma-separated). If unset/empty or `APP_ENV=local`, CORS allows any origin.

Web app (Vite):
- `VITE_API_TARGET` (dev proxy target for REST, default `http://localhost:8080`)
- `VITE_REALTIME_TARGET` (dev proxy target for WS, default `http://localhost:8081`)
- `VITE_BACKEND_ORIGIN` (optional absolute origin used by the built client fetch/WS helpers; if unset it uses relative URLs / `window.location.origin`)

Message attachments (local filesystem backend):
- `ATTACHMENTS_MAX_BYTES` (default `5242880`)
- `ATTACHMENTS_DIR` (default `.local/attachments`)

Experience mode resolver:
- `GET /experience/context?org_id=...&channel_id=...` resolves Work/Play using: user preference > channel hint > org default > preset.
- `PATCH /experience/preferences` with `{ "mode_preference": "work" | "play" | null }` updates the user’s global preference.

## Current commands (do not run here; run locally)

From `community-app/`:
- Format: `cargo fmt`
- Lint: `cargo clippy --all-targets --all-features -- -D warnings`
- Tests: `cargo test`

From `community-app/apps/web/`:
- Dev: `npm run dev`
- Lint: `npm run lint`
- Build: `npm run build`
- Preview: `npm run preview`

## Integration tests

There is an API integration test at `apps/api-server/tests/permissions_isolation.rs` which uses:
- `TEST_DATABASE_URL` (preferred) or `DATABASE_URL`
- plus the standard service env vars loaded by `config::AppConfig::from_env()` (Redis/NATS/LiveKit + JWT secrets)

## Production deployment (repo-level)

The repo root contains a GitHub Actions workflow that builds/pushes images and SSH-deploys:
- `.github/workflows/deploy-flux.yml`

Server compose/env examples live under:
- `community-app/deploy/flux/`
