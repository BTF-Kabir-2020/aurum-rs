# Aurum — Pitch Document

> One minute. One terminal. Real market data, real math, zero secrets.

## The 30-second pitch

> **Aurum is an open-source financial analysis engine for gold (XAU/USD) that runs on your own machine.**
> It pulls real market data with your own API key, computes the classic indicators (EMA, RSI, Momentum)
> entirely in Rust, and produces an explicit **BUY / SELL / HOLD** signal together with the exact conditions
> that produced it. Everything is transparent, everything is local — no accounts, no subscriptions, and
> no "black-box AI" claims. The rules are fixed, documented, and readable by anyone in one file.

## Why it matters

| Problem with typical tools | What Aurum does instead |
|---|---|
| Signal-seller bots: hidden logic, "80% win rate" claims | Rules are printed every time (`conditions` in every response), and the codebase states in writing it will **never** be called AI |
| Data lives on someone else's server | All quotes, candles and signals persist in your own SQLite file — `aurum backup` moves it anywhere |
| Desktop lock-in (TradingView plans, plugins) | One small native binary + REST API + terminal UI; runs natively, in Docker, headless |
| Trial accounts and expiring datasets | Standard data provider contract — BYO Massive key, swap providers by editing one config line |

## Show it in 10 seconds

```powershell
git clone https://github.com/BTF-Kabir-2020/aurum-rs
cd aurum-rs
.\scripts\demo.ps1        # or: ./scripts/demo.sh — offline demo, no key, no network
```

What the audience sees:

1. A deterministic replay of 240 gold candles (offline fixture — always works, no internet needed).
2. Each frame shows price, EMA(20), RSI(14), momentum and the resulting rule decision.
3. A doctor report proving configuration, database, migrations and provider setup are all green.

## What the engine actually does

- Ships with **two providers**: a Massive (api.massive.com) adapter for real daily bars — verified live —
  and a deterministic offline replay provider for demos and tests.
- Persists every quote/candle/signal in SQLite (WAL, deduped, retention-pruned) — the data belongs to the user.
- Exposes everything twice: a polished CLI plus a versioned REST API (`/api/v1/quote|history|signal` with
  RFC/ISO-8601 timestamps, redacted config, stale-cache tagging when a provider is down).
- Never hides its reasoning: HTTP responses include the raw condition matrix
  (`rsi_oversold`, `price_above_ema`, `positive_momentum`, ...). “Signal strength” is a rule score, not AI
  confidence — the word AI is explicitly forbidden in the product vocabulary.

## Engineering proof (for the technical audience)

- 106 tests (unit + integration, network-free): indicator math, rule isolation, provider contracts, storage, API.
- Gates green end-to-end: fmt / check / clippy `-D warnings` / test / `cargo audit` (0 vulnerabilities) / `cargo deny`.
- Verified live: real Massive daily gold data pulled and signaled through the resilient client
  (bounded retries, exponential backoff, Retry-After honoured), plus a full Docker smoke
  (`compose build`, healthcheck, persistence across restart).
- Release-ready via **cargo-dist**: a tag push ships static archives and `install.sh`/`install.ps1`
  for Windows, Linux, macOS.

## Where it is deliberately limited (honesty slide)

- Today it is a **research/analysis tool**, not an execution engine — it will never place a trade.
- Signals come from three classical indicators; there is no machine learning and no backtest engine in v0.1.
- Intraday data quality depends on the user's Massive plan (the free tier is daily).
- Live WebSocket streaming and multi-symbol support are on the public roadmap (v0.2 → v0.4), not in this release.

## Roadmap teaser

- v0.2 — Live WebSocket ingestion
- v0.3 — Multi-symbol (BTC, ETH, EUR/USD, GBP/USD)
- v0.4 — Backtesting and rule scoring history

---

*Aurum — Financial Market Intelligence Engine. Your data. Your machine. Transparent rules.*
