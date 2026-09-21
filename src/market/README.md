# market

Everything that fetches market data.

- `mod.rs` — module surface
- `models.rs` — internal `Candle`, `Quote`, canonical `Symbol`
- `provider.rs` — `MarketDataProvider` trait (the contract)
- `massive.rs` — Massive provider adapter (HTTP/auth, provider-specific)
- `replay.rs` — offline deterministic DEMO provider

The rest of the app never couples to a specific provider. See `docs/PROVIDERS.md`.