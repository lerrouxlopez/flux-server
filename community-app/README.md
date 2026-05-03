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
- API: `curl http://localhost:8080/healthz`
- Realtime: `curl http://localhost:8081/health`

Branding:
- Public (pre-login): `GET /public/branding?host=<hostname-or-org-slug>`
  - Resolves by `branding_profiles.custom_domain` or by org slug (first label of the host).
  - Example: `curl -sS "http://localhost:8080/public/branding?host=acme.localhost"`
  - Example (slug): `curl -sS "http://localhost:8080/public/branding?host=acme"`

Auth + org smoke test:
- Register:
  - `curl -sS -X POST http://localhost:8080/auth/register -H "content-type: application/json" -d "{\"email\":\"me@example.com\",\"display_name\":\"Me\",\"password\":\"password123\"}"`
- Login:
  - `curl -sS -X POST http://localhost:8080/auth/login -H "content-type: application/json" -d "{\"email\":\"me@example.com\",\"password\":\"password123\"}"`
- Create org (replace `$ACCESS_TOKEN`):
  - `curl -sS -X POST http://localhost:8080/orgs -H "content-type: application/json" -H "authorization: Bearer $ACCESS_TOKEN" -d "{\"name\":\"Acme\",\"slug\":\"acme\"}"`

### LiveKit local vs prod
- Local: `docker compose` runs LiveKit with `--dev` for convenience only.
- Prod: use proper keys, TLS, TURN/STUN, and an explicit LiveKit config (do not use `--dev`).

### Windows + WSL notes
- Recommended: run `cargo` inside WSL (Linux toolchain), and run `docker compose` either inside WSL or via Docker Desktop WSL integration.
- Ports are published to `localhost` by default via `docker-compose.yml`.
