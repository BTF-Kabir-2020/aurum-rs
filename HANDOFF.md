# HANDOFF — Progress & State

> **Read this FIRST before doing anything.** It tells you exactly what has been done, what is left, and what decisions are frozen.
> Then read `README.md` (the master build specification) and `docs/QA.md` (the quality gates).

**Last updated:** 2026-09-21
**Current status:** `ALL PHASES 0-18 DONE (2026-09-21) — v0.1 feature-complete, all gates + smoke green, real-use verified; remaining OWNER items only: git init (§6), GitHub publish decision (§7)`

---

## 1. How to use this file (rules for whoever is working)

- When you **START** a task: change its row to 🔄 and note the date.
- When you **FINISH** a task: change it to ✅ and add **verification evidence** (commands + expected output). "It compiles" is not enough — see `docs/QA.md`.
- If a task is **blocked**: mark it ⬛ and write ONE sentence why.
- Never delete history. Append notes so the next person sees evolution.
- Touch only the rows relevant to you. Do not "clean up" completed rows.

Legend: ✅ done · 🔄 in progress · ⬛ blocked · ⬜ not started

---

## 2. Repository state today

Everything in the repo is **scaffolding + docs + CI config + a compiling skeleton**. `Cargo.toml` (full dependency set from docs/BUILD.md), `Cargo.lock` and `src/main.rs` exist and all local gates pass; real module content (Phases 3+) is the remaining build work.

```
aurum-rs/
├── README.md                  # master build specification (68 sections)
├── HANDOFF.md                 # this file — progress tracker
├── Cargo.toml + Cargo.lock    # Phase 2 skeleton, no external deps yet
├── config.example.toml, .env.example, .env.production.example
├── rust-toolchain.toml        # stable 1.98.1, edition 2024
├── Dockerfile + docker-compose.yml
├── Makefile                   # fmt / check / clippy / test / gates / smoke
├── deny.toml                  # license + advisory policy
├── LICENSE (MIT), DISCLAIMER.md, SECURITY.md, CONTRIBUTING.md
├── .gitattributes, .gitignore, .editorconfig, .dockerignore
├── .devcontainer/             # VS Code dev container (Rust 1.98.1)
├── .github/                   # CI, release, dependabot, CODEOWNERS, issue/PR templates
├── docs/                      # ARCHITECTURE, CONFIGURATION, PROVIDERS, OPERATIONS,
│                              # BACKUPS, RELEASE, RESILIENCE, QA, BUILD, HANDOFF
├── migrations/                # empty — SQL migrations not written yet
├── fixtures/, fixtures/provider/  # empty — synthetic CSV + provider payloads not written yet
├── scripts/                   # empty — demo.sh / demo.ps1 not written yet
├── tests/                     # empty
├── src/                       # src/main.rs + empty module folders (market, indicators, signal, storage, cli, api, tui)
├── data/                      # runtime SQLite lives here (gitignored)
└── backups/                   # backups live here (gitignored)
```

> `.env` (gitignored) holds the real Massive credentials; `.env.example` has matching **empty** placeholders only.

---

## 3. Frozen decisions (changing these requires a discussion)

| # | Decision | Where documented |
|---|---|---|
| 1 | Language: **Rust only** (no Python/Node/frontend in v0.1) | README §6-8 |
| 2 | Three modes: DEMO (offline) / HISTORICAL / LIVE (never simulated) | README §3 |
| 3 | Provider: **Massive**, user brings their own key/BYO; `C:XAUUSD` | docs/PROVIDERS.md |
| 4 | Storage: **SQLite (SQLx)** — no Postgres/Redis/Kafka | README §9-10 |
| 5 | Binary name **`aurum`**, edition 2024, pinned `1.98.1` | docs/BUILD.md |
| 6 | Signal engine is **rule-based** — never call it AI | README §26-27 |
| 7 | **No auth platform / no users / no subscriptions** (no AURUM_API_TOKEN) | README §35 |
| 8 | Public API is versioned `/api/v1/...`; health endpoints unversioned | README §19-20 |
| 9 | Backups: create/verify/restore with safety rules | docs/BACKUPS.md |
| 10 | Repo owner: `BTF-Kabir-2020` — **not published to GitHub yet** | README badges |
| 11 | Out of scope for v0.1: ML, blockchain, trading execution, web dashboard, microservices | README §34 |
| 12 | Provider credentials (Massive API + flat-files S3) provided by owner; live ONLY in gitignored `.env` / injected env | docs/PROVIDERS.md, docs/CONFIGURATION.md |
| 13 | **Proxy policy:** the repo is proxy-agnostic — zero proxy references in committed files. Network fixes are runtime-only, per-host (see Environment notes) | HANDOFF §4a |

---

## 4a. Environment notes (this dev container)

- `PATH` is not persisted in new shells: prefix `export PATH="$HOME/.cargo/bin:$PATH"`. This dev container is **recreated often (apt packages reset)**; after a rebuild, reinstall `build-essential` for `cc` before compiling here:
  ```
  echo 'Acquire::http::proxy "http://host.docker.internal:10808";
  Acquire::https::proxy "http://host.docker.internal:10808";' > /etc/apt/apt.conf.d/99aurum-proxy
  apt-get update && apt-get install -y --no-install-recommends build-essential
  ```
- Rust 1.98.1 exact toolchain installed at `~/.cargo/bin` with `rustfmt` + `clippy` survives in the container home. In the project dir (`rust-toolchain.toml` pin) everything selects automatically — no `RUSTUP_TOOLCHAIN` needed.
- Verified locally (2026-09-21): `cargo check` ✅ · `cargo fmt --check` ✅ · `cargo clippy --all-targets` ✅ · `cargo run` prints `Aurum — Financial Market Intelligence Engine` and exits 0.
- Network: default `HTTPS_PROXY=http://127.0.0.1:17890` works for `crates.io` + `static.rust-lang.org` (deps/toolchain OK). Tor is up via `socks5://tor:9050`.
- **When something is blocked by the network**, the owner's Windows host runs a **mixed** proxy (HTTP + SOCKS5) on port **10808**, reachable from the container as `host.docker.internal:10808` — use it for the JAMMED call only (e.g. Debian/apt gave 502 on the default route).
- That host proxy is a per-host contingency (Iran filtering); future hosts may not have it. Runtime tweaks: `/etc/apt/apt.conf.d/99aurum-proxy`, `.env` (gitignored), shell env — all ephemeral, never committed.
- `git` is NOT installed in this container; `git init -b main` must run on the Windows host (see §6).
- Docker verification was done on the Windows host (2026-09-21): release build via `rust:1.98.1-bookworm` works with `rustls-no-provider`/`ring` (no cmake needed anywhere), slim runtime runs `aurum server`, health endpoint answers. `docker compose up -d` + `docker compose ps` are the smoke path.

---

## 4. Phase checklist (from README §60)

Verification standards for every phase: `docs/QA.md` §3 + §8.

| Phase | Task | Status | Notes / evidence |
|---|---|---|---|
| 0 | Scaffold repo (folders, docs, CI, env, Docker, Makefile, handoff) | ✅ | 2026-09-21 — all files exist; no code |
| 1 | Inspect environment (rustc/cargo/docker/git available?) | ✅ | 1.98.1 toolchain installed + `cc` + rustfmt/clippy; full gate chain green locally (see §4a). `git` absent here — init on Windows host. |
| 2 | Project skeleton: `Cargo.toml` (per docs/BUILD.md), deps, toolchain | ✅ | All deps from docs/BUILD.md added and `cargo check --all-targets` + `cargo clippy --all-targets -- -D warnings` + `cargo fmt --check` green on Windows host (2026-09-21). **reqwest uses `rustls-no-provider` + `ring`** (the `rustls` feature pulls `aws-lc-rs` which needs cmake/perl — avoided on purpose). `main.rs` now has `version` and `server` (axum `/health/live` + `/health/ready`) subcommands. |
| 3 | config.rs, error.rs, models, symbol normalization + compile | ✅ | 2026-09-21: `src/config.rs` (env precedence per docs/CONFIGURATION.md, `load_with_env` for tests, redacted summary, `toml` dep added for config.toml parsing), `src/error.rs` (thiserror + `Result` alias), `src/market/models.rs` (`Symbol::normalize` — one canonical fn, `Candle`, `Quote`, `Timeframe`), `src/market/provider.rs` (`MarketDataProvider` trait, native async-in-trait). 16 unit tests. Gates green on Windows host: `cargo fmt --check` / `cargo check --all-targets` / `cargo clippy -D warnings` / `cargo test` (16 passed) / `cargo run` banner OK. |
| 4 | SQLite: storage, migrations, repositories + tests | ✅ | 2026-09-21: `migrations/0001_init.sql` (candles/quotes/signals/app_metadata, PK-dedup, epoch-ms UTC timestamps) EMBEDDED at compile time in `src/storage/db.rs` (cwd-independent Migrator; sqlx `migrate` feature without proc-macros). `storage/open()` = WAL + busy_timeout + migrations; `repository.rs` upserts (candles by symbol+timeframe+timestamp, quotes by symbol+timestamp), `prune_older_than` (retention), counts/loaders. 10 tests incl. dedupe, roundtrip, prune, empty queries. |
| 5 | Replay/Demo provider (offline) + full flow fixture→signal→TUI | 🔄 | 2026-09-21: provider + fixture DONE — `fixtures/xauusd_5m.csv` (240 deterministic synthetic 5m candles), `src/market/replay.rs` (`ReplayProvider`: CSV parse+validate, deterministic quote w/ demo spread, history w/ limit/timeframe/until, actionable errors). 10 tests (26 total). Gates green on Windows host: fmt/check/clippy `-D warnings`/test. Fixture→candle→indicators→signal flow VERIFIED via `tests/offline_flow.rs` (TUI leg lands with Phase 12). |
| 6 | Massive provider (verify current official docs) | ✅ | 2026-09-21: docs verified live (massive.com/docs). Endpoints: last quote `GET /v1/last_quote/currencies/{from}/{to}`; bars `GET /v2/aggs/ticker/{ticker}/range/{mult}/{timespan}/{from}/{to}`, auth `Authorization: Bearer`. `src/market/massive.rs` (quote/history, error mapping 401/403/429/5xx, `rustls`+`ring` provider install) + 4 fixtures in `fixtures/provider/`. 12 tests. Gates green. **LIVE verified** with owner key from `.env`: daily aggs HTTP 200 (`status=DELAYED`, O/H/L/C sane); ⚠️ **plan limitation**: intraday 5m bars and `last_quote` NOT included in owner's plan (HTTP 403 "plan doesn't include…"); 5m only possible via replay/demo or plan upgrade — never relabel daily as 5m. Config now enforces MASSIVE_API_KEY for historical/live modes. **End-to-end live smoke** (`tests/live_smoke.rs`, gated by `MASSIVE_LIVE=1`, CI-safe): 2/2 passed — live daily history through `MassiveProvider` returned real candles (last_close=4372.45, ts=2026-09-20); intraday 5m → clean actionable plan error. Also: `src/lib.rs` added (lib target) so integration tests can use the crate; binary `version`, banner, and real HTTP server smoke (`/health/live`, `/health/ready` = `status=ok`) re-verified on Windows host. **Gap-closure pass (2026-09-21):** (1) `parse_candles` bug fixed — candles were hardcoded as 5m regardless of request; now tagged with requested timeframe + regression tests. (2) `Config` wired into `main.rs server` — bind follows config/env precedence (`AURUM_SERVER_PORT=8155` live-verified), `--bind` is an explicit override. (3) clippy `async_fn_in_trait` resolved with explicit allow + rationale comment. |
| 7 | Indicators: EMA, RSI, momentum + unit tests | ✅ | 2026-09-21: `src/indicators/{ema,rsi,momentum}.rs` — EMA(20, SMA-seeded, None until seeded), RSI(14, Wilder; flat=50, pure-gain=100, pure-loss=0 documented), momentum = signed % change over period (0-base → None, never NaN/Inf). QA edges covered (empty input, too few closes, zero period, zero denominator, flat data, non-finite input). **Refactor:** `sma.rs` + `smma.rs` extracted as shared primitives — EMA seeds via `sma()`, RSI smooths via `smma()` on gain/loss series; all numeric behavior identical (all pre-existing tests pass unchanged), hand-checked Wilder example (RSI≈66.402) re-verified directly on SMMA. SMA/SMMA are math primitives, NOT standalone signal indicators (docs/ARCHITECTURE dataflow note). 18 tests. |
| 8 | Signal engine + rule tests | ✅ | 2026-09-21: `src/signal/engine.rs` — `decide_from_values` (pure, per-condition isolation tests per docs/QA.md §6) + `evaluate(closes)` (EMA20/RSI14/momentum-10 runner). Rules per README §26 strict AND (BUYреса = all 3, SELL = all 3), strength = matched conditions (3/3/0), conditions dict exposed (§27), `Action` serializes BUY/SELL/HOLD, never "AI" (§28). Note: README §50 example shows BUY strength 2 — inconsistent with §26 strict-AND; implemented §26, strength 3 when BUY/SELL fires, HOLD→0. Owner may flip to "≥2 rules fired" later if §50 style is preferred. 10 tests + `tests/offline_flow.rs` closes the Phase-5 full-flow (fixture→candles→indicators→signal, wiring assertions). Gates green: 64 lib + 3 integration tests, fmt/clippy clean. |
| 9 | Storage integration (persist quotes/candles/signals) | ✅ | 2026-09-21: `src/state.rs` `App` — provider enum dispatch (Replay|Massive), quote/history/signal paths persist + retention-prune. Demo signal flow (`aurum demo` / `signal --offline`) verified live: candles=35, quotes=1, signals persisted (doctor counts). Source tags keep demo/historical separable (README §10). |
| 10 | CLI: demo, doctor, init, config, quote, history, signal, watch, server, backup, version | ✅ | 2026-09-21: all commands live and smoke-verified on Windows host — `init` (scaffold, no overwrite), `config show` (secrets redacted)/`validate`, `demo` (offline full flow + persistence), `doctor` (5 checks + counts, exit-code semantics), `quote/history/signal/watch` (--offline forces demo; JSON per README §50 shape; watch polls text-based until Phase 12 TUI), `backup create/verify` (VACUUM INTO, 45KB verified), `server` (health). `watch` = text loop (Ratatui pending Phase 12); `server` still health-only (API routes Phase 11). |
| 11 | Axum API (routes + DTOs + health) | ✅ | 2026-09-21: `src/api/{routes,dto}.rs` — health (unversioned) + `/api/v1/status`, `/api/v1/quote/{symbol}`, `/api/v1/history/{symbol}?limit=` (cap 500), `/api/v1/signal/{symbol}` (computes + persists). DTOs per README §50 shape (ISO 8601 timestamps, source/stale tags, version on every response, conditions dict exposed per §27). Error mapping: 400 invalid symbol/config, 502 provider/upstream, 503 missing credential (README §21 actionable messages). server:demo verified live on Windows host (status/quote/history/signal all 200); alias normalization works (`/quote/xau%2Fusd`). `tests/api_smoke.rs` (in-process axum + reqwest: 10 assertions incl. 400 path). Health NEVER unhealthy when optional live provider unavailable in DEMO (§19). CORS/trace middleware optional later (tower-http in deps). |
| 12 | Ratatui TUI | ✅ | 2026-09-21: `src/tui/{app,ui}.rs` — `aurum watch <symbol>` runs the Ratatui/Crossterm TUI: bordered screen per README §29 (Symbol/Mode/Price/EMA/RSI/Momentum/SIGNAL/STRENGTH `n/3 (rule score, not AI)`/Data source/Updated/Data age), poll-driven refresh with `ui.refresh_ms` (min 250ms, low CPU), 'q'/Ctrl+C clean exit with terminal restore on all paths, per-tick signal computation + persistence. `ui.rs` pure `screen_lines`/`preview_text` unit-tested (README example fields asserted, never says AI-confidence). Signal computation shared with API/CLI; error state renders actionable message. |
| 13 | Resilience: timeouts, retries, rate limits, stale cache | ✅ | 2026-09-21: `send_resilient` in massive.rs (MAX 3 attempts, backoff 200ms×2^k, `Retry-After` honored on 429, permanent 4xx never retried — verified with counting test servers: 429→2 reqs, 500,500,ok→3 reqs ok, 401→1 req only, 3×500→error after exactly 3). Stale cache: `state::quote_with_stale` — provider down → cached row served with `stale:true` + `age_seconds` (API 200, never presented as live), cache miss → original actionable error (state tests: cached-hit/miss/fresh-age-0). Connect+total timeouts already on client (10s/30s). Graceful shutdown: `ctrl_c` + `with_graceful_shutdown`. Doctor [WARN] for missing optional key in DEMO (degrades, not fails). Tests 102 lib + 4 integration green. |
| 14 | Docker smoke tests (health, API, persistence) | ✅ | 2026-09-21 (Windows host): `docker compose build` ok, `up -d` → healthy; health/live + /api/v1/status + /api/v1/signal all verified IN-CONTAINER; `/data/aurum.db` created, `down` → `up -d` restart retained candles=240/signals=7 (persistence verified); `aurum doctor` inside container → all checks passed. **Found & fixed real bug via doctor:** Windows Docker Desktop bind mounts deny new-file creation (`root:root 755`) → doctor now degrades to `[WARN]` with actionable host-backup guidance; in-container backup to `/tmp` verified working; host-side `aurum backup create` unaffected. Graceful shutdown logs verified. |
| 15 | Installers / cargo-dist init | ✅ | 2026-09-21: `dist` (cargo-dist 0.32.0) installed; `dist init --yes --hosting github` → `dist-workspace.toml` + `[profile.dist]` in Cargo.toml + GENERATED `.github/workflows/release.yml` (tag v* → builds windows/linux/macOS archives + install.sh + install.ps1 per README §16, env `production` required before first tag). `dist generate` after enabling shell+powershell installers; `dist plan` dry-run verified artifact plan (aurum-rs-{targets}.tar.xz + installers + sha256). Docs synced: docs/BUILD.md + docs/RELEASE.md now describe the cargo-dist reality. Offline owner note: publish needs the GitHub repo (still not created, owner choice). |
| 16 | Full test suite + CI green | ✅ | 2026-09-21 local: full gate chain green — fmt/check/clippy `-D warnings`/`cargo test` (102 lib + 4 integration) + `cargo audit` exit 0 (1256 advisories, no vulns) + `cargo deny check` AFTER config fixes (deny.toml had invalid SPDX `Unicode-OF-16`, removed deprecated keys copyleft/allow-osi-fsf-free/default per cargo-deny 0.20 migration, added Zlib/CDLA-Permissive-2.0 allow, documented transitive duplicates in bans skip with reason ⇒ all four categories: advisories/bans/licenses/sources ok). ci.yml steps match reality (fmt/check/clippy/test/audit/deny/docker). CI has been run on the owner's host locally; GitHub CI runs green once repo published (git init still pending owner action). |
| 17 | README product milestone + docs polish | ✅ | 2026-09-21: README §0 PRODUCT STATUS added (verified-live capability list, known limitation, what's deliberately not built + roadmap pointer). Docs synced with code reality across the session: OPERATIONS (endpoints/CLI/prune/shutdown/Windows-bind-mount note), BUILD (reqwest rustls-no-provider + dist), RELEASE (cargo-dist active), CONFIGURATION (AURUM_MASSIVE_BASE), PROVIDERS (verified endpoints + plan 403 reality), ARCHITECTURE (SMA/SMMA note, backup row), README §24/§32 trees. |
| 18 | Final validation (all gates + smoke) | ✅ | 2026-09-21: `make gates` equivalent all green — fmt --check / check --all-targets --all-features / clippy -D warnings / test (102 lib + 4 integration, 0 failed) / audit exit 0 / deny all-ok. Smoke: `aurum version`=0.1.0 · demo offline full-flow · doctor all-passed · backup create+verify (77824B) · server with ALL 6 endpoints HTTP 200. Release binary used from arbitrary cwd; real Massive daily signal verified. README §0 status lives. Open items for owner only: git init + first commit (§6), GitHub publish decision (§7), GitHub `production` environment before first tag. |

---

## 5. Quality gates (must pass at every milestone)

```
cargo fmt --all -- --check
cargo check --all-targets --all-features
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-targets --all-features
cargo audit          # security
cargo deny check     # licenses/bans
```

Shortcut when available: `make gates` then `make smoke`.

---

## 6. Agenda for the next session (Phases 3→18)

Frozen decisions: §3. Build kit: §4a. Quality gates: §5 + `docs/QA.md`. Follow README §60 plan.

1. Phase 3 — `config.rs` (env precedence per docs/CONFIGURATION.md), `error.rs` (thiserror), models, symbol normalization → compile green.
2. Phase 5 — DEMO/replay provider + `fixtures/xauusd_5m.csv` (offline flow) → Phase 6 — Massive provider with `fixtures/provider/*` parsing tests (verify current Massive docs).
3. Phase 7 indicators (EMA/RSI/momentum) → Phase 8 signal engine (rule-based, never "AI") → Phase 4 SQLite (sqlx migrations + repos + tests) → Phase 9 persist integration.
4. Phase 10 full CLI (demo, doctor, init, config, quote, history, signal, watch, server, backup, version) → Phase 11 API routes + DTOs → Phase 12 TUI.
5. Phase 13 resilience → Phase 14 full Docker smoke → Phase 15 `cargo dist init` → 16-18 test suite + CI green, README milestone, final `make gates` + `make smoke`.
6. Housekeeping: owner must run `git init -b main` + first commit on the Windows host (below) and re-verify `.env` stays untracked.

On the Windows host (one-time):
```powershell
cd "C:\Users\GH201190\Desktop\AspNetCORE8-FirstProject-master\New folder (49)\aurum-rs"
git init -b main; git add -A; git commit -m "chore: scaffold Aurum repo (spec, docs, CI, config, skeleton)"
```

## 7. Known open questions for the owner (BTF-Kabir-2020)

- [ ] Publish repo to GitHub (`aurum-rs`, public) — owner said "not yet"; wait for instruction.
- [ ] Create GitHub Environment `production` before first tagged release (docs/RELEASE.md).
- [ ] Legal name for LICENSE if the handle is not desired.
- [x] Massive API key + flat-files (S3) credentials provided by owner — stored in gitignored `.env` only, never committed (2026-09-21).
- [ ] Owner to verify `.env` is not listed in `git status` after the first commit.

## 8. Where everything lives (map)

| You need... | Go to |
|---|---|
| What to build | `README.md` (master spec) |
| What's done / next | `HANDOFF.md` (this file) |
| Quality gates & test matrix | `docs/QA.md` |
| Crate settings & dependency rationale | `docs/BUILD.md` |
| Architecture & data flow | `docs/ARCHITECTURE.md` |
| Config & env vars | `docs/CONFIGURATION.md` |
| Provider (Massive) specifics | `docs/PROVIDERS.md` |
| Run / deploy / troubleshoot | `docs/OPERATIONS.md` |
| Backup / restore details | `docs/BACKUPS.md` |
| Fault isolation / degradation | `docs/RESILIENCE.md` |
| Releases / cargo-dist | `docs/RELEASE.md` |