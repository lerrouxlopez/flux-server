# flux-server

Rust + PostgreSQL backend scaffold for an org-centric realtime app.

## Quickstart
- Start dependencies:
  - `docker compose up -d`
- Run migrations + API:
  - `cp .env.example .env`
  - `cargo run -p api-server`
- Check health:
  - `curl http://localhost:3000/health`

## Layout
- `apps/api-server` Axum HTTP API (MVC + Service + Repository)
  - `controllers/` HTTP handlers + routing
  - `services/` business logic
  - `repositories/` DB access layer
  - `models/` request/response models
- `crates/db` Postgres pool + SQLx migrations
- `migrations/` SQL schema migrations (SQLx)
