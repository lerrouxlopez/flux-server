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

DB Admin (Adminer):
- Open: `http://localhost:8082`
- System: `PostgreSQL`
- Server: `postgres` (or `localhost` if connecting from your host machine)
- Username: `app`
- Password: `app`
- Database: `community_app`

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

## Deployment (same scheme as Kinetic)
This repo includes a GitHub Actions workflow (`community-app/.github/workflows/deploy.yml`) that builds/pushes images to GHCR and SSHes into your VPS to `docker compose pull && docker compose up -d`.

### VPS layout (one-time)
Create `/opt/flux` with persistent data mounts (rebuilds won’t wipe data):

- `/opt/flux/docker-compose.yml` (copy from `community-app/deploy/flux/docker-compose.yml`)
- `/opt/flux/.env` (copy from `community-app/deploy/flux/.env.example` and set real secrets)
- `/opt/flux/data/postgres/` (Postgres data dir)
- `/opt/flux/livekit/livekit.yaml` (copy from `community-app/deploy/flux/livekit.yaml` and set your LiveKit key/secret)

The compose uses these loopback ports (non-conflicting with existing apps on the VPS):
- API: `127.0.0.1:8010`
- Realtime WS: `127.0.0.1:8011`
- Web UI: `127.0.0.1:8012`
- LiveKit signal: `127.0.0.1:7880` (proxied by nginx), LiveKit media: `7882/udp` (public)

### Nginx vhosts
Copy:
- `community-app/deploy/flux/flux.nginx.conf` -> `/etc/nginx/sites-available/flux`
- `community-app/deploy/flux/fluxserver.nginx.conf` -> `/etc/nginx/sites-available/fluxserver`

Enable:
- `ln -s /etc/nginx/sites-available/flux /etc/nginx/sites-enabled/flux`
- `ln -s /etc/nginx/sites-available/fluxserver /etc/nginx/sites-enabled/fluxserver`
- `nginx -t && systemctl reload nginx`

Then issue certs (Certbot-managed, same as existing sites):
- `certbot --nginx -d flux.kineticapp.online`
- `certbot --nginx -d fluxserver.kineticapp.online`

### Cloudflare note (LiveKit)
LiveKit needs UDP (`7882/udp`) for best reliability. If Cloudflare proxying blocks UDP, set `fluxserver.kineticapp.online` to “DNS only” (grey cloud) or configure a TURN setup that works over 443/TCP.
