# Operations

A user should be able to operate and recover this app without reading the source code.

## Native startup

Install locally:

```
cargo install --path .
aurum version
```

Run the offline demo (no API key, no network). `--speed 1..10` scales the
deterministic replay playback (README §30):

```
aurum demo
aurum demo --speed 10
```

One-shot alternatives (each fetches + persists and prints its result):

```
aurum quote XAUUSD --offline
aurum history XAUUSD --offline --limit 10
aurum signal XAUUSD --offline     # JSON per README §50 shape
```

Manage configuration:

```
aurum init                        # create ./config.toml scaffold (never overwrites)
aurum config show                 # effective config, secrets redacted
aurum config validate
```

Backup:

```
aurum backup create               # default ./backups/backup.db; --output overrides
aurum backup verify backups/backup.db
aurum backup restore backups/backup.db   # keeps a safety copy of the live DB
```

Run the server:

```
aurum server          # uses config, default mode = demo
```

Watch a symbol in the TUI:

```
aurum watch XAUUSD
```

## Docker startup

```
docker compose up -d
docker compose logs -f
```

The container:

- starts cleanly, healthcheck becomes healthy, API responds
- creates the SQLite database automatically on first start
- mounts `./data:/data` and `./backups:/backups` (persistent)
- uses `restart: unless-stopped`
- runs as a non-root user
- needs no manual shell access for normal operation

The mounted directories carry all important state across container replacement.

> **Windows Docker Desktop bind-mount quirk (verified live):** bind-mounted
> dirs from a Windows host show as `root:root 755` inside the container, so the
> non-root user cannot create NEW files in them (existing files stay writable —
> DB flows work). Effects + workaround:
> - `aurum doctor` reports a `[WARN]` (not a failure) for the data directory.
> - `aurum backup create` inside the container must write to a writable path
>   (e.g. `/tmp/...`); to keep the backup in `./backups`, run it on the host.
> - On Linux hosts, ensure `./data` and `./backups` are owned by the container
>   user (`chown -R 10001:10001 ./data ./backups`) and this cannot happen.

## Shutdown

- Native server: `Ctrl+C` → graceful shutdown (stops accepting requests, closes the DB, exit 0).
- Docker: `docker compose down` (data persists in `./data`).
- TUI: `q` or `Ctrl+C` (terminal raw mode restored on all exit paths).

## Logs

Structured `tracing` logs. Useful context: startup, mode, provider, symbol, request failures, retry events, database startup, migration status, backup status. Secrets are never logged. The TUI does not spam logs every second.

## Configuration sources

Precedence (highest wins): **CLI flags → environment variables → `config.toml` → defaults**.

- Dev: `.env` is auto-loaded from the working directory if present (gitignored). See `.env.example`.
- Prod: inject real values via the environment (Docker `env_file`/`environment`, process manager), never a committed file.
- Docker: compose passes the host `.env` (optional) and sets `/data`, `/backups`, `0.0.0.0` defaults.

Full inventory and runtime profiles: `docs/CONFIGURATION.md` → "Environment configuration".

## Health & API

```
GET /health/live                  # process is running
GET /health/ready                 # application can perform required operations
GET /api/v1/status                # version, mode, provider
GET /api/v1/quote/XAUUSD          # fresh quote, or stale cache tagged stale:true + age_seconds
GET /api/v1/history/XAUUSD?limit=50
GET /api/v1/signal/XAUUSD         # README §50 response shape
```

All responses include `version`. The app is NOT reported unhealthy merely
because an optional live provider is unavailable in DEMO mode. If the provider
is down but a cached quote exists, the API answers `200` with `stale: true`
(docs/RESILIENCE.md §3) — it is never presented as live.

## Database

- Location: `./data/aurum.db` (configurable via `[database].path`).
- SQLite with WAL mode.
- Versioned migrations in `migrations/`, applied automatically at startup.
- Runtime database files must not be committed.

## Data retention

The database grows as candles/quotes accumulate. Bounded by design:

- `[database].retention_days` (default `90`, env `AURUM_DB_RETENTION_DAYS`) is the window kept for candles and quotes.
- Pruning runs after every ingestion (quote / history / signal persistence) and deletes rows older than the window.
- If you need to keep history longer, raise `retention_days` or change it before ingestion; pruning never rewrites raw data it may still need (the newest rows always survive).
- Take a backup before lowering `retention_days` if you might need old data later.

## Backup / restore

```
aurum backup create   # e.g. --output ./backups/my-backup.db
aurum backup verify <file>
aurum backup restore <file>
```

See `docs/BACKUPS.md` for details.

## Upgrade

1. Recommended: `aurum backup create` before upgrading.
2. Start the new version: it loads the DB, runs pending migrations, and fails safely if a migration fails.
3. A migration failure must never silently destroy data.

See the documented rollback strategy in `docs/BACKUPS.md` / release notes. Manual SQL is not required for normal upgrades.

## Provider troubleshooting

- Run `aurum doctor` for a full diagnostic.
- Common causes:
  - `Live provider access unavailable`: plan/account does not permit live access; use `historical` or `demo`.
  - HTTP 429: rate limit; retries/backoff handle it, check plan limits.
  - Missing API key: configure the secret via environment; DEMO mode never needs one.
  - HTTP 5xx: transient provider issue; the client retries up to 3 times.