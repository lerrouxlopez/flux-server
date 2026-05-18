# Observability (Production)

This repo uses:

- **Structured logs** via `tracing` (set `LOG_FORMAT=json` in production).
- **Prometheus metrics** via the `metrics` crate + `metrics-exporter-prometheus`.
  - API Server exposes metrics at `GET /metrics`.

## Logging

Recommended env:

- `LOG_FORMAT=json`
- `RUST_LOG=info` (or `debug` temporarily)

Safety:

- Do not log JWTs / LiveKit tokens.
- Prefer identifiers (`org_id`, `room_id`, `session_id`, `participant_id`) over payloads.

## Metrics

### API Server (`apps/api-server`)

Endpoint:

- `GET /metrics` (Prometheus text format)

Core metrics:

- `http_server_requests_total{status="..."}`: total HTTP responses by status code.
- `http_server_request_duration_seconds`: histogram of request latency.

Media metrics:

- `media_join_requests_total{intent="...",room_kind="..."}`: join attempts
- `media_join_success_total`: join successes
- `media_join_denied_total`: permission denied joins
- `media_join_errors_total`: other join errors
- `media_reconnect_reuse_total`: reconnects that reuse an existing participant row
- `livekit_tokens_issued_total`: successfully issued LiveKit tokens
- `livekit_token_issuance_errors_total`: failures generating LiveKit JWTs

NATS metrics:

- `nats_publish_attempts_total`: attempted publishes
- `nats_publish_failures_total`: failed publishes

### Realtime Gateway (`apps/realtime-gateway`)

Core metrics:

- `ws_connections_active`: gauge of current websocket connections
- `ws_connections_accepted_total`: connections accepted
- `ws_sessions_error_total`: websocket sessions ending with error

## Suggested dashboards (safe defaults)

### API health

- HTTP status: `sum by (status) (rate(http_server_requests_total[5m]))`
- Latency (p95): `histogram_quantile(0.95, sum(rate(http_server_request_duration_seconds_bucket[5m])) by (le))`
- Error rate: `sum(rate(http_server_requests_total{status=~"5.."}[5m])) / sum(rate(http_server_requests_total[5m]))`

### Media

- Join attempts/success: `rate(media_join_requests_total[5m])`, `rate(media_join_success_total[5m])`
- Reconnect reuse: `rate(media_reconnect_reuse_total[5m])`
- Token issuance errors: `rate(livekit_token_issuance_errors_total[5m])`

### NATS

- Publish failure rate: `rate(nats_publish_failures_total[5m]) / rate(nats_publish_attempts_total[5m])`

### Realtime (WS)

- Active connections: `ws_connections_active`
- New connections: `rate(ws_connections_accepted_total[5m])`
- Session errors: `rate(ws_sessions_error_total[5m])`

