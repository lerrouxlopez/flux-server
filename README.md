# flux-server

Rust + PostgreSQL backend scaffold for an org-centric realtime app.

## Quickstart
- Start dependencies:
  - `docker compose up -d`
- Run migrations + API:
  - `cp .env.example .env`
  - set `JWT_SECRET` in `.env`
  - `cargo run -p api-server`
- Check health:
  - `curl http://localhost:3000/health`

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
- `crates/db` Postgres pool + SQLx migrations
- `migrations/` SQL schema migrations (SQLx)
