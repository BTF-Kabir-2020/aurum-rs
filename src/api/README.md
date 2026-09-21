# api

Axum REST API.

- `routes.rs` — `/health/live`, `/health/ready`, `/api/v1/status`, `/api/v1/quote/:symbol`, `/api/v1/history/:symbol`, `/api/v1/signal/:symbol`
- `dto.rs` — stable response DTOs (ISO 8601 timestamps, source/stale fields)

Public routes (except health) are versioned under `/api/v1/`.