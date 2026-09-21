-- Aurum v0.1 initial schema (README §10).
-- Timestamps are UTC epoch milliseconds; source tags demo/historical/live.

CREATE TABLE IF NOT EXISTS candles (
    symbol     TEXT    NOT NULL,
    timeframe  TEXT    NOT NULL,
    timestamp  INTEGER NOT NULL, -- epoch ms (UTC)
    open       REAL    NOT NULL,
    high       REAL    NOT NULL,
    low        REAL    NOT NULL,
    close      REAL    NOT NULL,
    volume     REAL    NOT NULL,
    source     TEXT    NOT NULL,
    PRIMARY KEY (symbol, timeframe, timestamp)
);

CREATE TABLE IF NOT EXISTS quotes (
    symbol     TEXT    NOT NULL,
    timestamp  INTEGER NOT NULL, -- epoch ms (UTC)
    bid        REAL    NOT NULL,
    ask        REAL    NOT NULL,
    mid        REAL    NOT NULL,
    source     TEXT    NOT NULL,
    PRIMARY KEY (symbol, timestamp)
);

CREATE TABLE IF NOT EXISTS signals (
    symbol      TEXT    NOT NULL,
    timestamp   INTEGER NOT NULL, -- epoch ms (UTC)
    price       REAL    NOT NULL,
    rsi         REAL    NOT NULL,
    ema         REAL    NOT NULL,
    momentum    REAL    NOT NULL,
    action      TEXT    NOT NULL CHECK (action IN ('BUY', 'SELL', 'HOLD')),
    strength    INTEGER NOT NULL,
    max_strength INTEGER NOT NULL,
    conditions  TEXT    NOT NULL, -- JSON (README §27: exposed reasoning)
    source      TEXT    NOT NULL,
    PRIMARY KEY (symbol, timestamp)
);

CREATE TABLE IF NOT EXISTS app_metadata (
    key   TEXT PRIMARY KEY,
    value TEXT NOT NULL
);
