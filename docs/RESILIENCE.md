# Resilience & Fault Isolation — docs/RESILIENCE.md

Design principle: **one component failing must never paralyze the whole engine.**

## 1. Fault isolation model

Each capability owns its failure and degrades independently:

| Component | On failure the rest of the system... |
|---|---|
| Provider (Massive) | stays up — serves stale cache or a clear error; DEMO unaffected |
| Live stream | DEGRADES to HISTORICAL/DEMO; app keeps running; `doctor` warns only |
| SQLite | runs DEGRADED (in-memory calculations); `/health/live` OK, `/health/ready` reports not-ready |
| Indicator / signal math | impossible-invariant errors only — must not panic at runtime |
| TUI | keeps rendering "last known" frames and error row; never freezes |
| One CLI command | never affects another command (no shared mutable global state) |
| Server vs TUI | separate processes — the API server dying does not kill the TUI and vice versa |

## 2. Degradation matrix (what you can still do when X is down)

| X fails | DEMO | HISTORICAL | LIVE | Server | TUI | Backup/Restore |
|---|---|---|---|---|---|---|
| Provider | ✔ | ✗ (error) | ✗ (error) | ✔ (stale cache or error) | ✔ | ✔ |
| SQLite | ✔ | ✔* | ✔* | degraded | ✔* | ✗ (reports DB down) |
| Network | ✔ | ✗ | ✗ | ✔ cache-only | ✔ | ✔ |
| Everything fails | ✔ demo always works | ✗ | ✗ | responds /health/live | shows error state | ✗ |

`*` = calculations continue in memory; persistence is skipped with a logged warning. Data is never silently dropped without a warning.

## 3. Rules enforced in code

- Timeouts everywhere: connect + total request on HTTP client, per-operation deadlines in server handlers.
- Bounded retries only: max 3 with exponential backoff; never infinite.
- 429 handled via `Retry-After`; permanent 4xx never retried.
- No `unwrap()` / `expect()` / `panic!()` in normal runtime paths: use typed errors (`thiserror`) and app-level context (`anyhow`).
- Provider error on last resort → stale cache **tagged `stale: true` with timestamp and age**, never presented as live.
- On startup, `doctor` reports degraded optional capabilities as `[WARN]`, not fatal failure.
- `/health/live` = process running. `/health/ready` = can perform required operations. A missing optional live provider in DEMO mode must not make the app unhealthy.
- SQLite unavailable at startup → the app may start in memory mode for DEMO/analysis and report readiness `false`; it must not hang.

## 4. Graceful shutdown

- Handle SIGINT/SIGTERM: stop ingestion pollers, flush pending writes to SQLite, close backups cleanly, exit with code 0.
- TUI exit (Esc/q) cleans up the raw mode terminal and stops the event loop.
- Docker `restart: unless-stopped` plus clean exit means crashes recover automatically; data survives in mounted volumes.

## 5. Tested in QA

- resilience cases are part of the test inventory (`docs/QA.md`): provider 500/429 mapping, stale-cache exposure, DB-down degraded path, demo-without-network, signal endpoint while provider is down.