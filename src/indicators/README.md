# indicators

Pure/domain-level technical-analysis math, independent of HTTP, storage, and UI.

- `sma.rs` — SMA, the shared windowed-mean foundation
- `smma.rs` — Wilder's Smoothed MA (aka RMA / SMMA, α = 1/period)
- `ema.rs` — EMA (default period 20); seeds from `sma()`
- `rsi.rs` — RSI (default period 14); Wilder smoothing delegated to `smma()` on the gain/loss series
- `momentum.rs` — percent change

Functions are deterministic and unit-tested, including edge cases (empty input, too few candles, zero denominator, flat data, invalid period). No NaN/Infinity leaks into public responses.