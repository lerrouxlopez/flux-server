# Alerts (Suggested)

These alert ideas are intended to be **safe by default** and avoid noisy paging.

## API Server

### Elevated 5xx error rate (page)

Trigger when:

- `5xx_rate > 1%` for 10m **and**
- request volume is non-trivial.

PromQL (example):

- `sum(rate(http_server_requests_total{status=~"5.."}[10m])) / sum(rate(http_server_requests_total[10m])) > 0.01`

### High latency p95 (ticket/page depending on SLA)

- `histogram_quantile(0.95, sum(rate(http_server_request_duration_seconds_bucket[10m])) by (le)) > 1.0`

### Dependency readiness failing (ticket)

Use `/readyz` in your orchestrator health checks and alert if failing > N minutes.

## Media / LiveKit

### Token issuance errors (ticket)

- `rate(livekit_token_issuance_errors_total[10m]) > 0`

### Join errors spikes (ticket)

- `rate(media_join_errors_total[10m])` above baseline

## NATS

### Publish failure rate elevated (ticket)

- `rate(nats_publish_failures_total[10m]) / rate(nats_publish_attempts_total[10m]) > 0.01`

## Realtime Gateway (WebSocket)

### Active connections drop to zero unexpectedly (ticket/page depending on expected traffic)

- `ws_connections_active == 0` for 10m (only if you normally have steady traffic)

### Session error rate elevated (ticket)

- `rate(ws_sessions_error_total[10m])` above baseline

