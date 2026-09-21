# Quality Assurance — docs/QA.md

QA for Aurum v0.1. Every claim we ship must be verified; this doc says how.

## 1. Quality gates (machine-enforced)

Run locally and in CI before any release/PR merge:

| Gate | Command |
|---|---|
| Format | `cargo fmt --all -- --check` |
| Compile | `cargo check --all-targets --all-features` |
| Lint | `cargo clippy --all-targets --all-features -- -D warnings` |
| Unit + integration | `cargo test --all-targets --all-features` |
| DB migrations | automatic on startup; verified by integration test |
| Docker | `docker compose up -d` + health + API + persistence |

CI (`ci.yml`) also builds with `-D warnings` via a RUSTFLAGS policy. See README §37 for the canonical command set.

## 2. Test layers

| Layer | Scope | Runs on |
|---|---|---|
| Unit | indicator math, signal rules, symbol normalization, config validation | `cargo test` |
| Integration | provider parsing (fixtures), DB + migrations, backup validation, API DTOs | `cargo test` |
| Smoke (native) | demo offline, doctor, server + endpoints, backup create/verify/restore | manual script + CI Linux/macOS/Windows |
| Container | build, health, API, restart persistence, non-root | CI `docker` job |

Provider-dependent paths run against `fixtures/provider/*` only. CI never needs a personal API key.

## 3. Required smoke tests (README §37)

- [ ] `aurum demo` works offline (no network, no key)
- [ ] `aurum doctor` produces understandable diagnostics
- [ ] clean DB init + migrations applied
- [ ] server: `/health/live`, `/health/ready`, `/api/v1/status`, `/api/v1/signal/XAUUSD`
- [ ] backup: create then verify
- [ ] restore: into isolated env → integrity + migrations valid
- [ ] docker: `up -d` → healthy + API → `down` → `up -d` → data persists

## 4. Platform matrix

| Platform | Native | Docker |
|---|---|---|
| Windows x86_64 | CI test job | — |
| macOS (x86_64, arm64) | CI test job | — |
| Linux x86_64 | CI test job | CI build + smoke |
| Linux ARM64 | release build | optional |

## 5. Anti-fake-claim verification

- Rule-based signals must not use the words AI/ML.
- No predicted accuracy/win-rate numbers without a reproducible backtest.
- No invented performance numbers (latency/uptime/throughput).
- DEMO data always labeled `MODE: DEMO / DATA: BUNDLED FIXTURE`.
- Stale cache must never be presented as live.

## 6. Definition of done checklist

Mirror of README §59. A task is done only when the full checklist passes, not merely when `cargo check` passes.

## 7. Test inventory (target)

- `indicators`: EMA/RSI/momentum: empty input, too few candles, zero denominator, flat data, invalid period, no NaN/Infinity in output
- `signal`: each BUY/SELL/HOLD condition and strength scoring in isolation
- `market`: provider parsing from JSON fixtures, 429/500 error mapping, symbol aliases → canonical
- `storage`: migration up, WAL mode, candle/quote/signal persistence tagging by source, retention pruning respects `retention_days`
- `backup`: create/verify/restore integrity, no silent overwrite, rejection of invalid files/path traversal
- `api`: route availability, versioning, DTO shape, ISO 8601 timestamps
- `cli`: command surface + config show redaction
- `resilience`: provider 500/429 mapping, stale-cache exposure, DB-down degraded path, demo-without-network, signal endpoint while provider is down (see `docs/RESILIENCE.md`)

## 8. Release gate

Before a tag ships (`docs/RELEASE.md`): all gates above pass, `aurum version` matches the tag, smoke tests green, no secrets committed, docs updated.