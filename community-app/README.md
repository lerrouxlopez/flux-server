# community-app

Rust + PostgreSQL backend scaffold for an org-centric realtime app.

## Quickstart
- `cd community-app`
- Start dependencies:
  - `docker compose up -d`
- Run binaries:
  - `cp .env.example .env`
  - set `JWT_ACCESS_SECRET` + `JWT_REFRESH_SECRET` in `.env`
  - API: `cargo run -p api-server`
  - Gateway: `cargo run -p realtime-gateway`
  - Worker: `cargo run -p worker`

## Auth endpoints
- `POST /auth/register`
- `POST /auth/login`
- `POST /auth/refresh`
- `POST /auth/logout` (requires `Authorization: Bearer <access_token>`)

## Org + channel endpoints
- `POST /orgs` (requires `Authorization: Bearer <access_token>`)
- `GET /orgs/current` (requires `Authorization: Bearer <access_token>`)
- `GET /channels` (requires `Authorization: Bearer <access_token>`)
- `POST /channels` (requires `Authorization: Bearer <access_token>`)
- Optional header for the above org-scoped routes: `x-organization-id: <uuid>`

## Layout
- `apps/api-server` Axum HTTP API (MVC + Service + Repository)
  - `controllers/` HTTP handlers + routing
  - `services/` business logic
  - `repositories/` DB access layer
  - `models/` request/response models
- `apps/realtime-gateway` WebSocket gateway scaffold (`/ws`, `/health`)
- `apps/worker` Background worker scaffold (tick loop for now)
- `crates/db` Postgres pool + SQLx migrations (`crates/db/migrations/`)
- `crates/*` internal modules intended to absorb domain logic over time
