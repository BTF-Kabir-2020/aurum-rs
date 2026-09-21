# AURUM · MASTER BUILD SPECIFICATION

> **Aurum · Financial Market Intelligence Engine**
> Rust-based financial market intelligence engine for gold (XAU/USD) market analysis.
> This is the single source of truth for the build agent. Build it as a **real product**, not a mockup.

[![CI](https://github.com/BTF-Kabir-2020/aurum-rs/actions/workflows/ci.yml/badge.svg)](https://github.com/BTF-Kabir-2020/aurum-rs/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)
[![Rust 1.98](https://img.shields.io/badge/Rust-1.98-darkgreen.svg)](rust-toolchain.toml)

Badges target `BTF-Kabir-2020/aurum-rs`; they go live once the repository is public.

> **Taking over? Read [`HANDOFF.md`](HANDOFF.md) first.** It tracks what is done and what remains, phase by phase.

## 0. PRODUCT STATUS — v0.1 (verified live 2026-09-21)

The v0.1 build is **real and used end-to-end** (see `HANDOFF.md` phase table for evidence per claim):

- ✅ Offline DEMO: `aurum demo --speed N` — fixture replay → indicators → rule signal, zero setup.
- ✅ HISTORICAL: real XAU/USD daily bars via the user's own Massive key, persisted with retention.
- ✅ Signal engine: EMA(20) + RSI(14) + Momentum(10), rule-based BUY/SELL/HOLD with transparent conditions (never called "AI").
- ✅ Full CLI: init/config/doctor/quote/history/signal/watch/server/backup/version.
- ✅ REST API v1 (`/health/*`, `/api/v1/status|quote|history|signal`) with versioned DTOs, stale-cache tagging.
- ✅ TUI watch screen (Ratatui), SQLite (WAL) storage + backup create/verify/restore.
- ✅ Rust resilience: bounded retries with backoff + Retry-After, timeouts, configurable base URL.
- ✅ Docker: non-root slim runtime, healthcheck, persistent `./data` (verified `down`/`up` retention).
- ✅ Release packaging: cargo-dist pipeline (`dist plan` verified) → tag `v*` ships installers.

**Known limitation:** intraday (5m) bars and last_quote need a higher Massive plan (HTTP 403 on the current key); daily bars work today. 5-minute data comes from the offline fixture until the plan is upgraded (never relabeled).

**Not built (by spec, v0.1):** auth/users, live WebSocket (v0.2), multi-symbol (v0.3), backtesting (v0.4) — roadmap in §54.

---

## 1. ROLES

You are the **lead Rust engineer, software architect, QA engineer, DevOps engineer, release engineer, and product engineer** for this project.

Your job is to build a complete, runnable, maintainable, release-ready MVP of Aurum — not a code prototype.

- Do not ask unnecessary questions.
- Do not leave architectural decisions to the reader.
- Do not create fake functionality.
- Do not claim features exist unless they actually work.
- Make sensible engineering decisions according to this specification.
- When something cannot be implemented exactly as described because an external provider/API has changed, verify the current official documentation and adapt the implementation **without changing the product architecture**.

The final result must be easy for:

- a developer to run locally,
- a non-developer to run with Docker,
- a contributor to understand,
- a GitHub visitor to demo immediately,
- future versions to extend.

## 2. PRODUCT

| Field | Value |
|---|---|
| Project name | Aurum |
| Repository name | `aurum-rs` |
| Description | High-performance financial market intelligence engine built in Rust |
| Initial market | XAU/USD |
| Internal/provider symbol | `C:XAUUSD` |
| User-facing symbol | `XAU/USD` |

Aurum is an **analytics and market-signal engine**.

- It does NOT execute trades.
- It does NOT place broker orders.
- It does NOT manage real money.
- It is a market analysis / research / signal demonstrator.

Do not describe rule-based calculations as AI or machine learning.

## 3. CORE PRODUCT MODES

The application MUST support three modes.

### Mode A: DEMO

- Requires **no API key**, no external network, no database setup, no Docker, no provider account.
- Uses deterministic bundled fixture data.
- Run: `cargo run -- demo` or `aurum demo`.
- Fixture stored in the repo: `fixtures/xauusd_5m.csv` (enough deterministic candles for meaningful EMA/RSI/momentum).
- Display clearly: `MODE: DEMO` and `DATA: BUNDLED FIXTURE`.
- Never pretend demo data is real market data.

### Mode B: HISTORICAL

- Uses Massive's Currencies API.
- Provider symbol: `C:XAUUSD`.
- Provider implementation isolated behind an internal abstraction.
- Retrieves historical aggregate bars and converts them into the internal `Candle` model.
- Default timeframe: 5-minute candles when supported by the provider/plan; otherwise gracefully explain the limitation instead of fabricating data.
- Do not silently substitute daily data while labeling it 5-minute data.

### Mode C: LIVE

- Uses Massive's Forex WebSocket infrastructure when the user's account/plan permits.
- Live data must NOT be simulated.
- If live access is unavailable: show a clear provider/plan error, preserve the application, allow DEMO and HISTORICAL to keep working.
- Distinguish: `LIVE`, `HISTORICAL`, `DEMO`, `STALE`, `ERROR`.
- Never display stale data as live.
- Every live/historical result exposes the data timestamp and age where relevant.

## 4. DATA PROVIDER ARCHITECTURE

Do NOT couple the rest of the application directly to Massive.

```
src/market/
├── mod.rs
├── models.rs
├── provider.rs
└── massive.rs
```

Define a provider abstraction, conceptually:

```rust
trait MarketDataProvider {
    async fn latest_quote(...);
    async fn history(...);
    async fn stream(...);
    async fn health(...);
}
```

It must support at minimum: latest quote, historical candles, live stream where supported, provider health/status.

The signal engine must not know:

- Massive URL formats
- HTTP query parameters
- provider JSON field names
- provider authentication details

Provider-specific logic stays inside `market/massive.rs`. This is mandatory because later we may add `TwelveDataProvider`, `OtherProvider`, `ReplayProvider` without rewriting the indicator or signal engine.

## 5. DATA SOURCE / LICENSING SAFETY

- Design the public project around **BYO-provider-access**.
- The repo must NOT redistribute commercial provider data.
- Public DEMO mode uses bundled synthetic/replay fixture data.
- Historical/live modes require the user's own provider credentials.
- Docs must clearly distinguish **Aurum software** from **third-party market data**.
- Do not imply Aurum owns or redistributes third-party market data.
- Do not build a paid hosted market-data redistribution product into this MVP.

## 6. PROGRAMMING LANGUAGE

- Primary language: **Rust**, channel **stable**.
- Pin the toolchain used for the project.
- At project creation use `1.98.1` in `rust-toolchain.toml` unless the environment has a newer stable patch release verified as appropriate.
- Edition: **2024**.
- No Python in the initial application. No Python sidecar. No Node.js runtime deps. No frontend. Pure Rust core.

## 7. WHY NO PYTHON

Do not introduce Python for: indicators, market ingestion, API calls, signal logic, database, CLI, WebSocket processing.

Future research/ML may use Python in a completely separate component later. The MVP is Rust-first.

## 8. CORE TECH STACK

Use current stable compatible versions:

Rust, Tokio, Reqwest, Serde, Serde JSON, Axum, Tower HTTP, Clap, Ratatui, Crossterm, SQLx + SQLite, Tracing, Chrono, Thiserror, Anyhow.

- Use Rustls for HTTP/TLS where practical for portability.
- Do not add libraries merely because they are popular. Every dependency must have a concrete purpose. Keep the dependency tree reasonable.

## 9. DATABASE

- Use **SQLite** (`data/aurum.db`).
- Do NOT use PostgreSQL, Redis, Kafka, RabbitMQ, NATS, MongoDB for the MVP.
- Reason: Aurum needs persistence but not a DB server. SQLite gives zero external setup, native installation, Docker installation, easy backup, portability, simple deployment, future migration path.
- Use SQLx + SQLite. Enable WAL mode where appropriate.

## 10. DATABASE CONTENT

Persist only useful application data. Initial tables:

- `candles`: symbol, timestamp, open, high, low, close, volume, source, timeframe
- `quotes`: symbol, timestamp, bid, ask, mid, source
- `signals`: symbol, timestamp, price, rsi, ema, momentum, action, strength, source
- `app_metadata`

Store enough metadata to distinguish demo, historical and live data. Do NOT continuously store useless duplicate data.

## 11. MIGRATIONS

- Create `migrations/` with versioned SQL migrations.
- The application must automatically apply pending migrations at startup.
- Never require the user to manually edit SQLite tables.

## 12. BACKUP SYSTEM

Aurum MUST have a real backup feature.

- `aurum backup create` (with `--output ./backups/my-backup.db`)
- `aurum backup verify <file>`
- `aurum backup restore <file>`

Requirements:

- consistent SQLite backup, no corruption
- atomic destination creation where practical
- verify the backup is readable
- never silently overwrite an existing backup
- print backup path, size, database timestamp/version

Restore safety: validate backup → validate SQLite integrity → create safety copy of current DB → stop/lock application operations if necessary → restore → reopen → validate migrations/schema → report success/failure. The restore operation must not destroy the current DB before the backup has been validated.

## 13. CONFIGURATION

Support `config.toml`:

```toml
[app]
mode = "demo"
symbol = "XAUUSD"
timeframe = "5m"

[server]
host = "127.0.0.1"
port = 8080

[database]
path = "./data/aurum.db"

[provider]
name = "massive"

[ui]
refresh_ms = 1000
```

- Secrets MUST NOT be committed. Provider credentials via environment/runtime secret configuration.
- Never log secrets. Support configuration validation.
- Config commands: `aurum config show`, `aurum config validate`, `aurum config init`.
- `config show` must redact secrets; `config validate` must tell the user exactly what is missing/invalid.

## 14. DOCKER SUPPORT

- Docker is required, but Docker must NOT be required.
- Two first-class deployment modes: **NATIVE** and **DOCKER**; both run the same binary.
- Create `Dockerfile`, `docker-compose.yml`, `.dockerignore`.
- Dockerfile: multi-stage build, small runtime image, no compiler in runtime image, non-root user, expose app port, healthcheck support, persist data externally.
- Compose mounts `./data:/data` and `./backups:/backups`. `restart: unless-stopped`. No PostgreSQL, no Redis, no unnecessary services. Single application container.
- UX: `docker compose up -d` then `docker compose logs -f`. Healthcheck becomes healthy, API responds, DB created automatically, no manual shell access required.

## 15. NATIVE USER EXPERIENCE

- `cargo run -- demo` works and shows the product.
- `cargo install --path .` then `aurum demo`.
- No external services required for DEMO mode.

## 16. INSTALLERS / RELEASES

- Use **cargo-dist** for release packaging.
- Targets: Windows x86_64, Linux x86_64, Linux ARM64, macOS x86_64, macOS ARM64.
- Produce installable artifacts for Windows and Linux at minimum; generate `install.ps1` and `install.sh`.
- Also support `cargo install --path .`.
- Do not make users compile Rust to use a published release.

## 17. CLI

Use Clap:

```
aurum
├── demo
├── doctor
├── init
├── config (show | validate | init)
├── quote <symbol>
├── signal <symbol>
├── history <symbol>
├── watch <symbol>
├── server
├── backup (create | verify | restore)
└── version
```

Symbol aliases must be normalized. Accept `XAUUSD`, `XAU/USD`, `C:XAUUSD`, `xauusd` → canonical `XAUUSD`.

## 18. DOCTOR COMMAND

Implement `aurum doctor` to check:

- Rust/runtime compatibility where relevant
- configuration
- database + migrations
- provider configuration + connectivity
- write permissions
- data directory, backup directory

Example output:

```
AURUM DOCTOR

[OK]   Configuration
[OK]   Database
[OK]   Migrations
[OK]   Data directory
[OK]   Provider configuration
[WARN] Live provider access unavailable
[OK]   Demo mode

Result: READY
```

Warnings must not be fatal unless the requested mode actually requires the missing capability.

## 19. HEALTH ENDPOINTS (Axum)

- `GET /health/live`: process is running
- `GET /health/ready`: application can perform required operations
- `GET /api/v1/status`
- `GET /api/v1/quote/:symbol`
- `GET /api/v1/history/:symbol`
- `GET /api/v1/signal/:symbol`

All responses JSON, include application version. Do not report unhealthy just because optional live provider is unavailable in DEMO mode.

## 20. API VERSIONING

All public API routes except health are versioned: `/api/v1/...`. Do not expose unstable internal structures directly. Use response DTOs.

## 21. ERROR HANDLING

- `thiserror` for typed domain/provider errors.
- `anyhow` for application-level context where appropriate.
- Never `unwrap()`, `expect()`, `panic!()` in normal runtime paths unless a justified impossible invariant. Tests may unwrap.
- Error messages must be actionable.

Bad: `request failed`

Good: `Massive API request failed: HTTP 429 rate limit exceeded`

## 22. HTTP CLIENT RESILIENCE

Provider client must implement:

- connect timeout
- total request timeout
- controlled retries
- exponential backoff
- 429 handling
- 5xx retry
- Retry-After handling when provided
- no pointless retry of permanent 4xx errors
- structured provider errors

No infinite retry loops. Default retry limit: **3**.

## 23. CACHING / RESILIENCE

Retain the most recent successful market response where practical.

```
provider error
      ↓
cached data available?
      ↓
YES → expose stale data + timestamp
NO  → clear actionable error
```

Never pretend cached data is live. Expose `source`, `timestamp`, `age`, `stale` where appropriate.

## 24. INDICATOR ENGINE

```
src/indicators/
├── mod.rs
├── sma.rs          # SMA — shared windowed-mean foundation
├── smma.rs         # Wilder Smoothed MA (RMA/SMMA) — base of RSI
├── ema.rs          # EMA(20), seeds from SMA
├── rsi.rs          # RSI(14), Wilder smoothing via SMMA
└── momentum.rs
```

- All indicator functions are pure/domain-level calculations.
- They must not know about HTTP, SQLite, Axum, Ratatui, or provider authentication.
- Initial indicators: EMA(20), RSI(14), Momentum. SMA/SMMA are shared math primitives backing EMA/RSI — they are NOT standalone signal indicators (signal rules stay fixed, README §26).
- Independently testable.

## 25. INDICATOR CORRECTNESS

- Do not invent formulas. Use established technical-analysis definitions.
- Document the exact calculation convention in code comments or docs.
- Deterministic unit tests.
- Edge cases: empty input, too few candles, zero denominator, flat data, negative/invalid period.
- No NaN/Infinity leaks into public API responses.

## 26. SIGNAL ENGINE

```
src/signal/
├── mod.rs
└── engine.rs
```

Signal actions: `BUY`, `SELL`, `HOLD`.

Initial rule engine:

- **BUY:** RSI < 30 AND price > EMA20 AND momentum > 0
- **SELL:** RSI > 70 AND price < EMA20 AND momentum < 0
- **Otherwise:** HOLD

Do not call this AI. Do not claim predictive accuracy.

## 27. SIGNAL STRENGTH

Do not call it "AI confidence". Use **signal strength** or **rule score**, calculated transparently from matched conditions. The API must expose underlying conditions:

```json
{
  "action": "BUY",
  "strength": 2,
  "conditions": {
    "rsi_oversold": true,
    "price_above_ema": true,
    "positive_momentum": false
  }
}
```

Do not hide the reasoning.

## 28. IMPORTANT FINANCIAL PRODUCT RULE

Aurum is an analytics/research tool. Distinguish `signal` from `guaranteed profitable trade`. No claims such as `80% win rate`, `guaranteed returns`, `AI predicts gold`, unless objectively measured by a real, reproducible backtest and clearly labeled as historical testing.

## 29. RATATUI TUI

Command: `aurum watch XAUUSD`. Interface example:

```
┌───────────────────────────────────────────────┐
│ AURUM                                         │
│ Financial Market Intelligence Engine           │
├───────────────────────────────────────────────┤
│ Symbol        XAU/USD                         │
│ Mode          DEMO / HISTORICAL / LIVE        │
│ Price         3681.42                         │
│                                                │
│ EMA(20)       3694.12                         │
│ RSI(14)       28.31                           │
│ Momentum      -1.21%                          │
│                                                │
│ SIGNAL        BUY                             │
│ STRENGTH      2/3                             │
│                                                │
│ Updated       22:41:08                        │
│ Data age     00:00:01                        │
└───────────────────────────────────────────────┘
```

Keep it readable, useful, low on CPU, and clean on refresh.

## 30. DEMO EXPERIENCE

`aurum demo` should immediately show a usable experience. Prefer a small deterministic replay loop. Allow `--speed 1 | 2 | 10` (or equivalent). Demo must be screenshot-friendly.

## 31. STORAGE LAYOUT

```
aurum-rs/
├── src/
├── migrations/
├── fixtures/
├── tests/
├── scripts/
├── docs/
├── .github/
├── data/
├── backups/
├── Cargo.toml
├── Cargo.lock
├── rust-toolchain.toml
├── Dockerfile
├── docker-compose.yml
├── README.md
├── LICENSE
└── .gitignore
```

Do NOT commit: runtime database files, secrets, generated logs, local backups.

## 32. SOURCE TREE

```
src/
├── lib.rs          # shared modules (binary, integration tests, docs examples)
├── main.rs
├── config.rs
├── error.rs
├── state.rs        # application state: provider selection + storage wiring
├── market/
│   ├── mod.rs
│   ├── models.rs
│   ├── provider.rs
│   ├── massive.rs
│   └── replay.rs
├── indicators/
│   ├── mod.rs
│   ├── sma.rs      # shared SMA primitive (backs EMA)
│   ├── smma.rs     # Wilder SMMA/RMA (bases RSI)
│   ├── ema.rs
│   ├── rsi.rs
│   └── momentum.rs
├── signal/
│   ├── mod.rs
│   └── engine.rs
├── storage/
│   ├── mod.rs
│   ├── db.rs
│   ├── repository.rs
│   └── backup.rs
├── cli/
│   ├── mod.rs
│   ├── demo.rs
│   ├── doctor.rs
│   ├── config.rs
│   ├── market.rs
│   └── backup.rs
├── api/
│   ├── mod.rs
│   ├── routes.rs
│   └── dto.rs
├── tui/
│   ├── mod.rs
│   ├── app.rs
│   └── ui.rs
```

> Tree above fixes the target names: `aurum config` = `cli/config_cmd.rs`,
> `aurum demo`/`aurum quote|history|signal` = `cli/market.rs`, doctor/backup
> in their own files.

No empty abstractions just for appearance. Every module has a real purpose.

## 33. APPLICATION STATE

Clean application state layer. Provider, storage, signal engine, and API communicate through explicit state/services. main.rs should primarily: parse CLI → load config → init tracing → init storage → init provider → construct state → run requested command/server/TUI.

## 34. NO OVERENGINEERING

Do NOT add to v0.1: microservices, Kubernetes, service mesh, Redis, message queue, GraphQL, frontend, authentication platform, user management, payment processing, ML, blockchain, smart contracts. Leave room without building them now.

## 35. SECURITY

- Never log provider API keys.
- Redact secrets in config output.
- Validate external input.
- URL/query construction must not permit SSRF through user input.
- Default server bind: localhost.
- Avoid permissive CORS by default.
- Secure TLS configuration.
- Docker runtime non-root.
- Do not commit secrets.
- Sanitize filenames in backup/restore; reject path traversal.
- Validate imported backup files.
- No API authentication / user accounts in v0.1 (not an auth platform). Security relies on the localhost-only default bind; if the server is later exposed beyond localhost, re-evaluate auth then.

## 36. LOGGING

Use Tracing. Log useful context: startup, mode, provider, symbol, request failures, retry, DB startup, migration status, backup status. Never log secrets. Do not spam logs every second in the TUI. Structured and readable.

## 37. TESTING REQUIREMENTS

Before claiming completion run:

```
cargo fmt --all -- --check
cargo check --all-targets --all-features
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-targets --all-features
```

Real smoke tests required:

- **Demo**: `aurum demo` offline
- **Doctor**: `aurum doctor` understandable diagnostics
- **Database**: clean init + migration verification
- **API**: server tests: `/health/live`, `/health/ready`, `/api/v1/status`, `/api/v1/signal/XAUUSD`
- **Backup**: create and verify
- **Restore**: restore into a test environment and verify integrity
- **Docker**: `docker compose up -d`, verify healthcheck + API; `down`/`up` and verify persistence

## 38. INTEGRATION TESTS

Cover: provider parsing, symbol normalization, indicator pipeline, signal engine, DB migrations, backup validation, API responses. For external provider tests, do not make CI depend on a personal API key. Use recorded/mock provider responses.

## 39. PROVIDER FIXTURES

Keep representative responses under `fixtures/provider/`:

- `massive_quote.json`
- `massive_history.json`
- `massive_error_429.json`
- `massive_error_500.json`

Use them in parsing/integration tests so external API changes don't silently break core tests.

## 40. CI/CD

- `.github/workflows/ci.yml`: fmt, check, clippy, test
- `.github/workflows/release.yml`: build binaries for supported targets and publish GitHub release artifacts via cargo-dist
- Do not require Docker for the native release artifact.

## 41. VERSIONING

Start at `0.1.0`, semantic versioning. Expose `aurum version` and an API version field.

## 42. README

README is part of the product. Must contain: name + tagline; What it is; Features; Architecture; Quick Start; Demo; Historical Mode; Live Mode; Docker; Native Installation; CLI Examples; API Examples; Configuration; Backup / Restore; Testing; Roadmap; Limitations; Data Provider / Licensing Notes; Disclaimer. No unsupported claims.

The disclaimer also lives standalone in `DISCLAIMER.md` and is linked from the README (product milestone).

## 43. README QUICK START

First thing a visitor sees:

```
git clone <repo>
cd aurum-rs
cargo run -- demo
```

Then `docker compose up -d`. Then a provider configuration example. The demo path requires no API key.

## 44. PROJECT DOCUMENTATION

- `docs/ARCHITECTURE.md`
- `docs/CONFIGURATION.md`
- `docs/PROVIDERS.md`
- `docs/OPERATIONS.md`
- `docs/BACKUPS.md`
- `docs/RELEASE.md`

Keep documentation concise and accurate.

## 45. OPERATIONS

Document: native startup, docker startup, shutdown, logs, health, database location, backup, restore, upgrade, provider troubleshooting. A user should recover the application without reading source code.

## 46. UPGRADE SAFETY

On new version start: load DB → run migrations → fail safely if migration fails → never silently destroy DB data. Document rollback strategy. No manual SQL for normal upgrades.

## 47. BACKUP POLICY

Recommend backup before upgrade, before restore, before changing provider/storage configuration. Provide `aurum backup create` as a pre-upgrade operation.

## 48. DOCKER DATA PERSISTENCE

All important state must survive container replacement via mounted directories: at minimum `/data` and `/backups` are persistent. No important state only inside the writable container filesystem.

## 49. PERFORMANCE

- Avoid unnecessary allocations.
- Don't rebuild the entire indicator history every screen refresh.
- Don't block the async runtime.
- Avoid repeated DB queries when a cache is enough.
- No unbounded memory growth.
- TUI lightweight; ingestion asynchronous; blocking DB/filesystem work handled appropriately.

## 50. API RESPONSE RULES

Stable, explicit responses. Example signal response:

```json
{
  "symbol": "XAU/USD",
  "mode": "demo",
  "timestamp": "2026-09-20T00:00:00Z",
  "price": 3681.42,
  "indicators": {
    "ema20": 3694.12,
    "rsi14": 28.31,
    "momentum": -1.21
  },
  "signal": {
    "action": "BUY",
    "strength": 2,
    "max_strength": 3
  },
  "data": {
    "source": "fixture",
    "stale": false
  }
}
```

Use ISO 8601 timestamps.

## 51. SYMBOL NORMALIZATION

One canonical normalization function. Inputs `XAUUSD`, `XAU/USD`, `C:XAUUSD`, `xauusd` → canonical `XAUUSD`. Provider adapter converts to `C:XAUUSD`. UI displays `XAU/USD`. Do not duplicate conversion logic.

## 52. TIME HANDLING

Use UTC internally. Convert only for presentation. Document provider timezone behavior. Do not mix local timestamps with UTC internally.

## 53. FUTURE EXTENSIBILITY

Design extension points for BTCUSD, ETHUSD, EURUSD, GBPUSD, USDJPY, but implement only XAU/USD in the MVP. No empty implementations for all symbols.

## 54. FUTURE ROADMAP

README roadmap may mention:

- v0.2: live WebSocket ingestion
- v0.3: multi-symbol support
- v0.4: backtesting
- v0.5: advanced risk analytics
- v0.6: signed signal attestations
- v1.x: optional web dashboard / SaaS infrastructure

Do not implement these unless required by v0.1.

## 55. MARKETING / PRODUCT POSITIONING

Preferred wording: **"Rust-based financial market intelligence engine."**

Avoid: "AI trading bot", "guaranteed profitable signals", "institutional-grade" (without objective evidence).

GitHub presentation emphasizes: Rust, Async, market data ingestion, technical-analysis engine, SQLite, REST API, terminal UI, Docker, cross-platform binaries, backup/restore, provider abstraction.

## 56. PRODUCT-LED OPEN SOURCE STRATEGY

README must enable: Clone → Demo → See TUI → Read architecture → Try API → Connect provider → Star repo. No account creation required to see the project working.

## 57. DEMO AS PRIMARY MARKETING FEATURE

Create `scripts/demo.ps1` and `scripts/demo.sh` launching the easiest demo workflow. No internet, no credentials.

## 58. NO FAKE BENCHMARKS

Do not invent `0.4ms latency`, `99.99% uptime`, `10M events/sec`, `80% accuracy`. If benchmarking is added, measure it. Otherwise omit performance numbers.

## 59. DEFINITION OF DONE

The task is NOT complete merely because the code compiles. Completion requires all of:

- [ ] Native build works
- [ ] Demo works offline
- [ ] TUI works
- [ ] REST API works
- [ ] SQLite works
- [ ] Migrations work
- [ ] Backup works
- [ ] Restore works
- [ ] Doctor works
- [ ] Docker works
- [ ] Docker persistence works
- [ ] Provider adapter works
- [ ] Provider errors are handled
- [ ] Indicators are tested
- [ ] Signal engine is tested
- [ ] CI passes
- [ ] Release configuration works
- [ ] README complete
- [ ] Documentation complete
- [ ] No secrets committed
- [ ] No fake claims
- [ ] No dead/unused architecture

## 60. AGENT EXECUTION ORDER

Build in this order, with a testable output each stage:

- **PHASE 1**: Inspect environment and repository (rustc, cargo, docker, git). Do not destroy existing work.
- **PHASE 2**: Project skeleton + toolchain pin (`Cargo.toml`, `rust-toolchain.toml`, `src/`, `migrations/`, `fixtures/`, `tests/`, `docs/`, `scripts/`, `.github/`).
- **PHASE 3**: config, error, models, symbol normalization. Compile.
- **PHASE 4**: SQLite: storage, migrations, repositories. Run tests.
- **PHASE 5**: Replay/Demo provider (offline). Full internal flow: fixture → Candle → indicators → signal → TUI. Verify.
- **PHASE 6**: Massive provider. Verify current official API docs first. historical + quote + health. No hardcoded undocumented assumptions.
- **PHASE 7**: Indicators. Test each independently.
- **PHASE 8**: Signal engine. Test each rule independently.
- **PHASE 9**: Storage integration: persist quotes, candles, signals where useful.
- **PHASE 10**: CLI: demo, doctor, init, config, quote, history, signal, watch, server, backup, version.
- **PHASE 11**: Axum API.
- **PHASE 12**: Ratatui TUI.
- **PHASE 13**: Resilience: timeouts, retries, rate limits, stale cache, provider errors.
- **PHASE 14**: Docker. Run real Docker smoke tests.
- **PHASE 15**: Installers / release automation.
- **PHASE 16**: Full test suite + CI.
- **PHASE 17**: README / docs.
- **PHASE 18**: Final validation. Do not stop at "cargo check passed".

## 61. FINAL AGENT BEHAVIOR

After implementation: run formatting, check, clippy, tests, demo, API smoke tests, backup/restore tests, Docker smoke tests (if Docker available); fix all discovered issues; repeat validation after fixes. Do not leave compile errors, failing tests, or broken doc examples. Do not say "should work"; verify it.

If an external provider cannot be tested (no credentials), test the integration with deterministic fixtures and clearly report exactly what could and could not be externally verified.

## 62. FINAL OUTPUT TO USER

When finished provide: What was built; Final folder structure; How to run DEMO; How to run NATIVE; How to run DOCKER; How to configure provider; API endpoints; Backup/restore commands; Test results; Known limitations. Do not dump hundreds of lines of source code into the final response. The repository itself is the deliverable.

## 63. MOST IMPORTANT PRODUCT PRINCIPLE

Aurum v0.1 must be: easy to run, easy to understand, easy to demo, easy to deploy, easy to backup, easy to restore, easy to extend, hard to misunderstand.

Complexity belongs in the implementation, not in the user's setup experience.

```
clone → demo → wow → connect provider → use product
```

not

```
clone → install 12 services → create database → configure 30 variables → fight Docker → fight provider → fight migrations
```

Keep the product shell polished while keeping the core technically honest.

---

## Product name decision

| Candidate | Status |
|---|---|
| `aurum-rs` | **Frozen for v0.1** (repo + project name) |
| `marketforge-rs` | Not chosen; revisit only if scope grows beyond gold before repo creation |

## Declared out of scope for v0.1

- Python ❌
- Next.js ❌
- Postgres ❌
- Redis ❌
- Kubernetes ❌
- Blockchain ❌
- ML ❌
- Trading / order execution ❌
- Payments ❌
- Auth platform ❌