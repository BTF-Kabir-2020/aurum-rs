# migrations

Versioned SQL migrations for SQLite. Applied automatically at startup. Never edit them by hand.

Each file is named `NNNN_name.sql` (e.g. `0001_init.sql`).

Initial tables:

- `candles`: symbol, timestamp, open, high, low, close, volume, source, timeframe
- `quotes`: symbol, timestamp, bid, ask, mid, source
- `signals`: symbol, timestamp, price, rsi, ema, momentum, action, strength, source
- `app_metadata`