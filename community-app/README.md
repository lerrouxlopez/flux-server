# community-app

Centralized, multi-tenant collaboration platform (Discord/Slack/Zoom-style).

## Stack
- Rust (Tokio), Axum
- PostgreSQL (source of truth) via SQLx
- Redis (ephemeral state only)
- NATS + JetStream (durable internal events)
- LiveKit (voice/video/screen share; backend issues tokens)

## Dev quickstart (WSL / Docker)
1. Start dependencies:
   - `docker compose up -d`
2. Run API server:
   - `cp .env.example .env`
   - `cargo run -p api-server`
3. Run realtime gateway (WebSocket):
   - `cargo run -p realtime-gateway`
4. Run worker:
   - `cargo run -p worker`

Health checks:
- API: `curl http://localhost:3000/health`
- Realtime: `curl http://localhost:3001/health`

### Windows + WSL notes
- Recommended: run `cargo` inside WSL (Linux toolchain), and run `docker compose` either inside WSL or via Docker Desktop WSL integration.
- Ports are published to `localhost` by default via `docker-compose.yml`.
