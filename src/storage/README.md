# storage

Persistence layer: SQLite via SQLx.

- `db.rs` — connection pool, WAL mode, compile-time embedded migration runner
- `repository.rs` — data access for candles / quotes / signals + retention pruning
- `backup.rs` — backup create (`VACUUM INTO` snapshot) / verify / restore with safety copy

The database file lives under `data/` (default `data/aurum.db`). Migrations are versioned in `migrations/` and applied automatically at startup.