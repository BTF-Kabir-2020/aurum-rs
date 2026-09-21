# src

Rust application source. Entry point `main.rs` plus these modules:

- `market/` — provider abstraction and data models (Massive, Replay)
- `indicators/` — pure technical-analysis primitives (SMA, SMMA/RMA) and calculations (EMA, RSI, Momentum)
- `signal/` — rule-based BUY/SELL/HOLD engine
- `storage/` — SQLite (SQLx), migrations, repositories, backup
- `state.rs` — application state: provider selection + storage wiring
- `cli/` — Clap commands (demo, doctor, config, market, backup)
- `api/` — Axum REST routes and DTOs
- `tui/` — Ratatui/Crossterm live screen

Also at the root: `config.rs`, `error.rs`, `state.rs`.

See `docs/ARCHITECTURE.md` for the full picture.