# Configuration

Aurum is configured through `config.toml` plus environment variables for secrets.

## config.toml

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
retention_days = 90

[provider]
name = "massive"

[ui]
refresh_ms = 1000
```

- `mode`: `demo` | `historical` | `live`
- `symbol`: canonical symbol (e.g. `XAUUSD`); aliases are normalized at input
- `timeframe`: requested granularity (v0.1 target: `5m`)
- `server.host` / `server.port`: Axum bind address. Defaults to localhost for safety.
- `database.path`: SQLite file location
- `provider.name`: provider adapter to use (`massive`, `replay`)
- `ui.refresh_ms`: TUI refresh interval

## Environment / secrets

Provider credentials must come from environment/runtime secret configuration: never in `config.toml`, never committed to the repository, never logged.

Configured via e.g.:

```
MASSIVE_API_KEY=...
```

The exact variable names are defined in `config.rs`, which is the single place that reads secrets and passes them to `AlphaVantage`/`Massive` providers. Business logic never calls `std::env::var` directly.

## Environment configuration

Configuration is resolved with this **precedence (highest wins)**:

```
command-line flags
      ↑
environment variables (AURUM_*, secrets, RUST_LOG)
      ↑
config.toml
      ↑
built-in defaults
```

### Environment variable inventory

| Variable | Overrides | Default | Notes |
|---|---|---|---|
| `AURUM_CONFIG` | — | `config.toml` | path to config file |
| `AURUM_MODE` | `[app].mode` | `demo` | `demo` \| `historical` \| `live` |
| `AURUM_SYMBOL` | `[app].symbol` | `XAUUSD` | aliases normalized |
| `AURUM_TIMEFRAME` | `[app].timeframe` | `5m` | requested granularity |
| `AURUM_SERVER_HOST` | `[server].host` | `127.0.0.1` | prod/Docker: `0.0.0.0` |
| `AURUM_SERVER_PORT` | `[server].port` | `8080` | |
| `AURUM_DB_PATH` | `[database].path` | `./data/aurum.db` | Docker: `/data/aurum.db` |
| `AURUM_DB_RETENTION_DAYS` | `[database].retention_days` | `90` | candles/quotes older than this are pruned |
| `AURUM_PROVIDER` | `[provider].name` | `massive` | |
| `AURUM_UI_REFRESH_MS` | `[ui].refresh_ms` | `1000` | TUI refresh |
| `MASSIVE_API_KEY` | `[provider]` secret | — | never in config.toml, never logged |
| `MASSIVE_S3_ACCESS_KEY_ID` | flat-files S3 access | — | Massive "Flat Files" (S3) |
| `MASSIVE_S3_SECRET_ACCESS_KEY` | flat-files S3 access | — | Massive "Flat Files" (S3) |
| `MASSIVE_S3_ENDPOINT` | S3 endpoint | `https://files.massive.com` | stable constant |
| `MASSIVE_S3_BUCKET` | S3 bucket | `flatfiles` | stable constant |
| `AURUM_MASSIVE_BASE` | Massive REST base URL | `https://api.massive.com` | runtime override (tests/proxies) — no config.toml field |
| `RUST_LOG` | logging filter | `info` | tracing filter |

### dotenv loading

- Local development: if a `.env` file exists in the working directory, it is loaded (dev convenience). `.env` is gitignored.
- Production: secrets and overrides must be **injected by the environment** (Docker `environment`/`env_file`, orchestrator, process manager). Never a committed file.
- Templates: `.env.example`, `.env.production.example`.

### Runtime profiles

- **dev**: default `config.toml`, `mode = demo`, localhost bind, `.env` auto-load, `RUST_LOG=info`.
- **prod**: injected env: `AURUM_MODE=historical`, `AURUM_SERVER_HOST=0.0.0.0`, `AURUM_DB_PATH=/data/aurum.db`, `RUST_LOG=warn`, real `MASSIVE_API_KEY`. Server is reachable only if the user intentionally exposes it.

No auth users, API tokens, or subscription platform exist in v0.1. See `SECURITY.md`.

## Config commands

```
aurum config init       # create a default config.toml scaffold
aurum config show       # print effective config, REDACTED
aurum config validate   # report exactly what is missing/invalid
```

- `config show` must redact secret values.
- `config validate` must tell the user exactly what is missing or invalid (e.g. missing API key for `historical` mode), and must never print secret values.

## Validation rules

- Unknown/invalid mode → error.
- Invalid bind host/port → error.
- Unwritable database/data path → error.
- Provider required by mode is unconfigured → error (only when the requested mode actually needs it; DEMO never requires a provider credential).