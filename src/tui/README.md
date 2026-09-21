# tui

Terminal user interface (Ratatui + Crossterm), used by `aurum watch <symbol>`.

- `app.rs` — application event loop / state
- `ui.rs` — rendering

Priorities: readability, terminal aesthetics, low CPU usage, clean refreshes. Shows symbol, mode, price, indicators, signal, strength, updated time and data age.