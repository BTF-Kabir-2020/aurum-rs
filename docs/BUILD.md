# Build decisions — docs/BUILD.md

Frozen decisions the implementing agent must follow when creating the Rust crate.

## Package

- Package name: `aurum-rs`
- Binary name: `[[bin]] name = "aurum"` — CLI, Dockerfile, release and scripts all expect the `aurum` binary.
- Edition: `2024` (per `rust-toolchain.toml` pin `1.98.1`).
- `Cargo.lock` is committed (application, reproducible builds).

## Cargo.toml metadata (to fill)

```toml
[package]
name = "aurum-rs"
version = "0.1.0"
edition = "2024"
description = "High-performance financial market intelligence engine built in Rust"
license = "MIT"
repository = "https://github.com/BTF-Kabir-2020/aurum-rs"
keywords = ["trading", "indicators", "market-data", "cli", "rust"]
categories = ["command-line-utilities", "finance", "science", "asynchronous"]

[[bin]]
name = "aurum"
path = "src/main.rs"

[profile.release]
lto = "thin"
codegen-units = 1
strip = true
```

## Dependencies and their purpose

| Crate | Purpose |
|---|---|
| tokio (full) | async runtime |
| reqwest (json, rustls-no-provider + ring) | HTTP client to providers; rustls for portability — `rustls-no-provider` because `aws-lc-rs` needs cmake/perl (avoided deliberately); a ring provider is installed once by the provider layer |
| serde + serde_json | JSON models and provider parsing |
| axum + tower-http | REST API |
| clap (derive) | CLI |
| ratatui + crossterm | terminal UI |
| sqlx (runtime-tokio, sqlite) | async SQLite |
| tracing + tracing-subscriber | structured logs |
| chrono (serde) | UTC timestamps |
| thiserror | typed domain/provider errors |
| anyhow | app-level context errors |

Rustls wins over native-tls where practical. Every dependency must justify its existence (`docs/QA.md` gate).

## Features / config

- Read secrets via `std::env` only inside `config.rs`; never in business logic.
- dotenv loading in dev only (`AURUM_MODE`/`config.toml`; see `docs/CONFIGURATION.md`).
- Default bind `127.0.0.1`; Docker overrides to `0.0.0.0`.

## cargo-dist

cargo-dist is initialized (2026-09-21, v0.32.0): `dist-workspace.toml` holds the
config and `.github/workflows/release.yml` is **generated** by `dist generate`
(on tag push, dist builds archives for windows-x64, linux-x64, linux-arm64,
macOS x64+arm64 and ships shell + PowerShell installers per README §16).

- To change platforms/installers: edit `dist-workspace.toml`, then `dist generate`. Never hand-edit the generated `release.yml`.
- Binary name `aurum` is what archives/installers package.
- Local pre-flight: `dist plan` (dry look) — publishing needs the GitHub repo + a tag.