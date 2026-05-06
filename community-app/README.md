# Flux

Centralized, multi-tenant collaboration platform (Discord/Slack/Zoom-style).

## Stack
- Rust (Tokio), Axum
- PostgreSQL (source of truth) via SQLx
- Redis (ephemeral state only)
- NATS + JetStream (durable internal events)
- LiveKit (voice/video/screen share; backend issues tokens)

## Dev quickstart (WSL / Docker)
1. Start the full stack (deps + backend + web UI):
   - `docker compose up -d --build`
2. Open the web app:
   - `http://localhost:5173`

Health checks:
- API: `curl http://localhost:8080/healthz`
- Realtime: `curl http://localhost:8081/health`

## Vertical slices (UI-first)

### Slice 1: Chat (realtime)
1. Register (`/register`) → redirects to `/orgs`
2. Create org on `/orgs` → redirects to `/app/<org_slug>`
3. Create a channel in the org sidebar
4. Open the channel → send a message
5. Observe the message arriving via WebSocket (`message.created`) without a manual refresh

### Slice 2: Media (LiveKit)
1. In a channel, click “Start meeting”
2. The app creates a media room → requests a LiveKit token
3. You join the LiveKit room (voice/video + screen share via LiveKit UI)

### Slice 3: Branding
1. Open admin panel (`/admin/<org_slug>`)
2. Set branding (app name, logo URL, colors) and save
3. Reload the app: login + shell should render branded, and `/public/branding?host=...` serves the public profile

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
