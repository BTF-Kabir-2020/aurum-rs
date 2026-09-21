# fixtures

Bundled, deterministic data for DEMO mode and tests.

- `xauusd_5m.csv` — realistic XAU/USD 5-minute candles (synthetic replay, not real market data)
- `provider/` — recorded provider responses for integration tests:

  - `massive_quote.json`
  - `massive_history.json`
  - `massive_error_429.json`
  - `massive_error_500.json`

Because fixtures are bundled, tests stay deterministic: provider API changes can't silently break the core suite.