# Providers

Aurum runs on a **BYO-provider-access** model. The repository never redistributes commercial market data. DEMO mode uses bundled synthetic fixtures; HISTORICAL and LIVE modes use the user's own provider credentials.

## v0.1 provider: Massive

- Provider: **Massive** (Currencies API) — docs verified 2026-09-21 (massive.com/docs)
- REST endpoints in use:
  - last quote: `GET /v1/last_quote/currencies/{from}/{to}`
  - bars: `GET /v2/aggs/ticker/{ticker}/range/{mult}/{timespan}/{from}/{to}` (sort=asc)
  - auth: `Authorization: Bearer MASSIVE_API_KEY`
- Symbol mapping: user-facing `XAU/USD` → internal canonical `XAUUSD` → provider `C:XAUUSD`
- Currency: gold (`C:XAUUSD`)
- Historical / EOD data via the aggregates endpoint above (all Currencies plans).
- Live data via Massive Forex WebSocket (requires an account/plan that permits it; v0.2+). Live must never be simulated.
- Default timeframe: `5m` candles when the plan supports them; otherwise explain the limitation gracefully. Never relabel daily data as 5-minute data.

> **Owner plan reality (verified live 2026-09-21):** the current key returns
> daily bars (`status=DELAYED`, end-of-day) but HTTP 403 for intraday 5m bars
> and `/v1/last_quote` ("plan doesn't include…"). Until the plan is upgraded,
> intraday/quote come from the DEMO replay provider only — daily data is never
> relabeled as 5-minute data.

## Provider abstraction

All providers implement the internal `MarketDataProvider` trait (see `market/provider.rs`).

```
MarketDataProvider
    ├── MassiveProvider   (v0.1)
    ├── ReplayProvider    (v0.1, DEMO)
    └── future: TwelveData, Alpha Vantage, Other...
```

The indicator engine and signal engine never depend on provider details.

## Resilience

The provider client must implement:

- connect timeout and total request timeout
- controlled retries with exponential backoff
- HTTP 429 handling, retry with `Retry-After` when provided
- 5xx retry; no pointless retry of permanent 4xx errors
- structured, actionable provider errors (e.g. `Massive API request failed: HTTP 429 rate limit exceeded`)
- default retry limit: 3, never infinite

## Staleness handling

On provider failure:

```
provider error
      ↓
cached data available?
      ↓
YES → expose stale data + timestamp (tagged stale)
NO  → clear actionable error
```

Cached/stale data is never labeled live.

## Provider fixtures for tests

`fixtures/provider/`:

- `massive_quote.json`
- `massive_history.json`
- `massive_error_429.json`
- `massive_error_500.json`

These keep parsing and integration tests deterministic: no personal API key, no dependence on live provider behavior.

## Data source / licensing notes

- Aurum **software** is distinct from **third-party market data**.
- Aurum does not own or redistribute third-party market data.
- Future commercial/hosted offerings must be designed around properly licensed data.

## Flat files (S3)

The account also has Massive "Flat Files" delivery via S3-compatible storage:

- Endpoint: `https://files.massive.com` (constant)
- Bucket: `flatfiles`
- Credentials: access key id + secret (user-provided, see `MASSIVE_S3_*` in `docs/CONFIGURATION.md`)

These carry bulk/daily files rather than live ticks. v0.1 uses the HTTP API; the S3 access is stored so bulk-history / fixture regeneration can use it later without a key change.

Secrets rule: all four `MASSIVE_*` values live only in runtime secret config (gitignored `.env` / injected environment). Never in `config.toml`, never committed, never logged.

## Alternatives considered

- **Alpha Vantage**: has Gold/Silver endpoints but not the intraday granularity wanted for the market engine; the trait makes it swappable later.
- **MetalpriceAPI**: supports XAU/USD with latest/hourly/OHLC/minutely, but minutely is Enterprise-only and free live data is delayed; avoided for a project claiming real-time. Revisit only under a suitable plan.