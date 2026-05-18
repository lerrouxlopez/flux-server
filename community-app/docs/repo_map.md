# Flux — Repo Map (`community-app/`)

This map is meant to be stable and “Phase 0” friendly: it lists what exists today so later refactors are deliberate.

## Top-level layout

- `apps/`
  - `api-server/` — REST API (Axum) + SQLx migrations on startup
  - `realtime-gateway/` — WebSocket gateway + NATS fanout
  - `worker/` — background durable consumers (JetStream) + periodic cleanup
  - `web/` — React + Vite frontend
- `crates/` — shared Rust libraries (domain, config, permissions, media, events, etc.)
- `deploy/` — VPS deployment assets (compose/env/nginx/livekit config)
- `docker-compose.yml` — local stack for dev
- `Dockerfile.*` — container builds for each service
- `.env.example` — baseline env var template for local runs

## Apps

### `apps/api-server` (REST)

Entry:
- `apps/api-server/src/main.rs` — loads `.env`, initializes telemetry, connects DB/Redis/NATS, runs migrations, serves Axum.
- `apps/api-server/src/app.rs` — router + middleware (request id, tracing, cors, body limit, timeout, auth context).

Routers:
- `apps/api-server/src/routes_auth.rs`
  - `POST /auth/register`
  - `POST /auth/login`
  - `POST /auth/refresh`
  - `POST /auth/logout`
  - `GET/PATCH /auth/me`
  - `POST /auth/me/avatar`
- `apps/api-server/src/routes_orgs.rs`
  - `POST/GET /orgs/`
  - `POST /orgs/join`
  - `GET /orgs/{org_id}`
  - `GET/POST /orgs/{org_id}/members`
  - `PATCH /orgs/{org_id}/members/{user_id}`
  - `POST /orgs/{org_id}/invites`
  - `GET /orgs/{org_id}/roles`
- `apps/api-server/src/routes_channels.rs`
  - `GET/POST /orgs/{org_id}/channels`
  - `GET/PATCH/DELETE /channels/{channel_id}`
- `apps/api-server/src/routes_messages.rs`
  - `GET/POST /channels/{channel_id}/messages`
  - `PATCH/DELETE /messages/{message_id}`
  - `POST /messages/{message_id}/reactions`
  - `DELETE /messages/{message_id}/reactions/{emoji}`
- `apps/api-server/src/routes_media.rs`
  - `POST /orgs/{org_id}/media/rooms`
  - `GET /media/rooms/{room_id}`
  - `POST /media/rooms/{room_id}/token`
  - `GET /media/rooms/{room_id}/participants`
- `apps/api-server/src/routes_branding.rs`
  - `GET /public/branding?host=...`
  - `GET/PATCH /orgs/{org_id}/branding`
- `apps/api-server/src/routes_friends.rs`
  - `GET /orgs/{org_id}/friends`
  - `GET/POST /orgs/{org_id}/friends/requests`
  - `POST /orgs/{org_id}/friends/requests/{request_id}/accept`
  - `POST /orgs/{org_id}/friends/requests/{request_id}/decline`
  - `POST /orgs/{org_id}/friends/requests/{request_id}/cancel`
  - `DELETE /orgs/{org_id}/friends/{user_id}`
- `apps/api-server/src/routes_dms.rs`
  - `GET /orgs/{org_id}/dms`
  - `POST /orgs/{org_id}/dms/{user_id}`
- `apps/api-server/src/routes_audit.rs`
  - `GET /orgs/{org_id}/audit-logs`

Health:
- `GET /healthz`
- `GET /readyz`

Tests:
- `apps/api-server/tests/permissions_isolation.rs` — integration test using `TEST_DATABASE_URL` (or `DATABASE_URL`)

### `apps/realtime-gateway` (WebSocket)

Entry:
- `apps/realtime-gateway/src/main.rs` — loads `.env`, init telemetry, connects DB/Redis/NATS, spawns NATS fanout runtime.

Routes:
- `GET /health`
- `GET /realtime/ws` — WebSocket upgrade; accepts access token via:
  - Authorization header (preferred when possible)
  - query params `access_token` or `token` (for browser WS constraints)

Core modules:
- `apps/realtime-gateway/src/runtime.rs` — runtime that fans out events
- `apps/realtime-gateway/src/protocol.rs` — event/protocol shaping

### `apps/worker` (background)

Entry:
- `apps/worker/src/main.rs` — connects DB/Redis/NATS, ensures JetStream streams, consumes `message.created`, periodic cleanup.

Current durable consumer:
- Subject filter: `org.*.channel.*.message.created`
- Behavior:
  - writes an audit log entry
  - creates offline notification rows (skips if presence key exists in Redis)

### `apps/web` (frontend)

Framework:
- React + React Router + TanStack Query + Zustand
- Vite dev proxy for `/auth`, `/orgs`, `/channels`, `/messages`, `/media`, `/public`, and `/realtime` (WS enabled).

Key files:
- `apps/web/src/main.tsx` — app entry
- `apps/web/src/router.tsx` — routes:
  - `/login`, `/register`, `/orgs`
  - `/app/:org_slug` (org shell)
  - `/app/:org_slug/channels/:channel_id`
  - `/app/:org_slug/voice/:room_id`
  - `/app/:org_slug/friends`
  - `/profile`
  - `/admin/:org_slug`
- `apps/web/src/api/client.ts` — fetch wrapper (Bearer token + JSON)
- `apps/web/src/realtime/ws.ts` — WS client (query-param token, reconnect backoff)
- `apps/web/src/state/*` — auth + branding stores
- `apps/web/src/views/*` — page-level features (admin, channel, friends, voice room, etc.)
- `apps/web/src/components/*` — UI building blocks and shells

## Crates (`crates/`)

The workspace lists these crates in `community-app/Cargo.toml`:

- `api` — typed API error envelopes + HTTP mapping
- `auth` — password hashing (argon2id), JWT issue/verify, refresh token hashing
- `branding` — placeholder (branding logic currently lives in `apps/api-server/src/routes_branding.rs`)
- `chat` — placeholder (chat logic currently lives in `apps/api-server/src/routes_channels.rs` + `routes_messages.rs`)
- `config` — env loading (`AppConfig::from_env()`), LiveKit URL fallback rules
- `db` — PgPool connect + `sqlx::migrate!("./migrations")`
- `domain` — shared domain types (orgs/users/channels/messages/media rooms/branding profile)
- `events` — NATS connect + event envelope/subjects/JetStream helpers
- `media` — LiveKit room + token issuing + participants listing (RoomService API)
- `notifications` — placeholder (some notification behavior implemented in `apps/worker`)
- `orgs` — placeholder (org logic currently implemented in `apps/api-server/src/routes_orgs.rs`)
- `permissions` — permission bitset + helpers
- `realtime` — broadcast hub + ws module (used by realtime gateway)
- `telemetry` — tracing subscriber init

## Database migrations

SQLx migrations live at:
- `crates/db/migrations/`

Current migration files:
- `202604280001_init.sql`
- `202604280002_required_tables.sql`
- `202604280003_org_invites.sql`
- `202605030001_media_rooms_name.sql`
- `202605030002_notifications.sql`
- `202605060001_branding_theme.sql`
- `202605070001_friends_dms.sql`
- `202605070002_user_profile.sql`
- `202605090001_channel_created_by.sql`
- `202605100001_branding_modes_theme.sql`
- `202605100002_branding_more_tokens.sql`

## Docker services (local)

Local `docker-compose.yml` defines:
- `postgres` (5432)
- `redis` (6379)
- `adminer` (8082)
- `nats` (4222, 8222)
- `livekit` (7880/7881 TCP, 7882 UDP; runs `--dev` for local convenience)
- `api-server` (8080)
- `realtime-gateway` (8081)
- `worker`
- `web` (5173)

## Environment variables (overview)

Required by Rust services (see `crates/config/src/lib.rs`):
- `DATABASE_URL`, `REDIS_URL`, `NATS_URL`
- `JWT_ACCESS_SECRET`, `JWT_REFRESH_SECRET`
- LiveKit: `LIVEKIT_URL_INTERNAL` + `LIVEKIT_URL_PUBLIC` (or legacy `LIVEKIT_URL`), plus `LIVEKIT_API_KEY` + `LIVEKIT_API_SECRET`

Optional:
- `APP_ENV`, `HTTP_ADDR`, `WS_ADDR`
- `ACCESS_TOKEN_TTL_SECONDS`, `REFRESH_TOKEN_TTL_SECONDS`
- `CORS_ALLOW_ORIGINS` (API server)

Frontend:
- `VITE_API_TARGET`, `VITE_REALTIME_TARGET` (Vite dev proxy)
- `VITE_BACKEND_ORIGIN` (built client absolute origin override)

