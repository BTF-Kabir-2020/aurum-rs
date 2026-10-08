# Changelog

All notable changes to aurum-rs are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.0] - 2026-09-21

First public release. Market intelligence engine for XAU/USD with a
transparent, rule-based signal model.

### Added

- **Market data providers** with a pluggable provider architecture and three run
  modes: `DEMO` (offline), `HISTORICAL`, and `LIVE`.
- **Indicator engine** — EMA20 and RSI with an SMMA-based smoothing refactor and
  correctness-focused unit tests.
- **Signal engine** — rule-based BUY / SELL / HOLD with momentum confirmation and
  a signal-strength score:
  - `BUY`: RSI < 30 AND price > EMA20 AND momentum > 0
  - `SELL`: RSI > 70 AND price < EMA20 AND momentum < 0
  - otherwise `HOLD`
- **Persistence** — SQLite storage with schema migrations, a backup system, and a
  verified restore path.
- **CLI** (`aurum`) built with Clap, including a `demo` showcase command and a
  `doctor` diagnostics command.
- **REST API** (Axum) with versioned routes and health endpoints:
  `/health/live`, `/health/ready`, `/api/v1/status`, `/api/v1/signal/XAUUSD`.
- **Terminal UI** (Ratatui) for live signal inspection.
- **HTTP client resilience** — retries, timeouts, and caching around upstream data.
- **Docker support** — `docker compose` stack with healthcheck and persistent volume.
- **Distribution** — `cargo-dist` installers for macOS (Apple Silicon / Intel),
  Linux (x64 / ARM64), and Windows (x64), with per-artifact checksums.
- **Test suite** — 106 tests covering indicators, signals, database, API, backup,
  and restore.

### Documentation

- Build specification (`README.md`), product pitch (`PITCH.md`), handoff and
  definition-of-done checklists (`HANDOFF.md`), security (`SECURITY.md`), and
  contributing guide (`CONTRIBUTING.md`).

[Unreleased]: https://github.com/BTF-Kabir-2020/aurum-rs/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/BTF-Kabir-2020/aurum-rs/releases/tag/v0.1.0
