use sqlx::SqlitePool;

use crate::error::{Error, Result};
use crate::market::models::{Candle, Quote, Symbol, Timeframe};
use crate::signal::Conditions;
use crate::signal::{Action, Signal};

fn from_ms(v: i64) -> Result<chrono::DateTime<chrono::Utc>> {
    chrono::DateTime::from_timestamp_millis(v)
        .ok_or_else(|| Error::Storage(format!("stored timestamp {v} out of range")))
}

fn action_str(a: Action) -> &'static str {
    match a {
        Action::Buy => "BUY",
        Action::Sell => "SELL",
        Action::Hold => "HOLD",
    }
}

fn parse_action(s: &str) -> Result<Action> {
    match s {
        "BUY" => Ok(Action::Buy),
        "SELL" => Ok(Action::Sell),
        "HOLD" => Ok(Action::Hold),
        other => Err(Error::Storage(format!("unknown action '{other}'"))),
    }
}

/// Upsert candles (dedupe by symbol+timeframe+timestamp; README §10).
pub async fn save_candles(pool: &SqlitePool, candles: &[Candle]) -> Result<u64> {
    let mut tx = pool.begin().await?;
    let mut n = 0u64;
    for c in candles {
        sqlx::query(
            "INSERT INTO candles (symbol, timeframe, timestamp, open, high, low, close, volume, source)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
             ON CONFLICT (symbol, timeframe, timestamp) DO UPDATE SET
                open = excluded.open, high = excluded.high, low = excluded.low,
                close = excluded.close, volume = excluded.volume, source = excluded.source",
        )
        .bind(c.symbol.as_str())
        .bind(c.timeframe.as_str())
        .bind(c.timestamp.timestamp_millis())
        .bind(c.open)
        .bind(c.high)
        .bind(c.low)
        .bind(c.close)
        .bind(c.volume)
        .bind(&c.source)
        .execute(&mut *tx)
        .await?;
        n += 1;
    }
    tx.commit().await?;
    Ok(n)
}

pub async fn save_quote(pool: &SqlitePool, quote: &Quote) -> Result<()> {
    sqlx::query(
        "INSERT INTO quotes (symbol, timestamp, bid, ask, mid, source)
         VALUES (?, ?, ?, ?, ?, ?)
         ON CONFLICT (symbol, timestamp) DO UPDATE SET
            bid = excluded.bid, ask = excluded.ask, mid = excluded.mid, source = excluded.source",
    )
    .bind(quote.symbol.as_str())
    .bind(quote.timestamp.timestamp_millis())
    .bind(quote.bid)
    .bind(quote.ask)
    .bind(quote.mid)
    .bind(&quote.source)
    .execute(pool)
    .await?;
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn build_signal(
    price: f64,
    rsi14: f64,
    ema20: f64,
    momentum: f64,
    action: &str,
    strength: i64,
    max_strength: i64,
    conditions: &str,
) -> Result<Signal> {
    let cond: Conditions =
        serde_json::from_str(conditions).map_err(|e| Error::Storage(format!("conditions: {e}")))?;
    Ok(Signal {
        action: parse_action(action)?,
        strength: strength as u8,
        max_strength: max_strength as u8,
        conditions: cond,
        price,
        ema20,
        rsi14,
        momentum,
    })
}

/// Persist one signal for a symbol; row key is (symbol, now) so consecutive
/// calls append. Returns the inserted row count (1 per signal).
pub async fn save_signals(pool: &SqlitePool, signals: &[Signal], symbol: &Symbol) -> Result<u64> {
    let mut tx = pool.begin().await?;
    let mut n = 0u64;
    for s in signals {
        sqlx::query(
            "INSERT INTO signals (symbol, timestamp, price, rsi, ema, momentum, action,
                                  strength, max_strength, conditions, source)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
             ON CONFLICT (symbol, timestamp) DO UPDATE SET
                price = excluded.price, rsi = excluded.rsi, ema = excluded.ema,
                momentum = excluded.momentum, action = excluded.action,
                strength = excluded.strength, max_strength = excluded.max_strength,
                conditions = excluded.conditions, source = excluded.source",
        )
        .bind(symbol.as_str())
        .bind(chrono::Utc::now().timestamp_millis())
        .bind(s.price)
        .bind(s.rsi14)
        .bind(s.ema20)
        .bind(s.momentum)
        .bind(action_str(s.action).to_string())
        .bind(s.strength as i64)
        .bind(s.max_strength as i64)
        .bind(
            serde_json::to_string(&s.conditions)
                .map_err(|e| Error::Storage(format!("conditions serialize: {e}")))?,
        )
        .bind("engine")
        .execute(&mut *tx)
        .await?;
        n += 1;
    }
    tx.commit().await?;
    Ok(n)
}

pub async fn latest_quote(pool: &SqlitePool, symbol: &Symbol) -> Result<Option<Quote>> {
    let row = sqlx::query_as::<_, (i64, f64, f64, f64, String)>(
        "SELECT timestamp, bid, ask, mid, source FROM quotes
         WHERE symbol = ? ORDER BY timestamp DESC LIMIT 1",
    )
    .bind(symbol.as_str())
    .fetch_optional(pool)
    .await?;
    row.map(|(ts, bid, ask, mid, source)| {
        Ok(Quote {
            symbol: symbol.clone(),
            timestamp: from_ms(ts)?,
            bid,
            ask,
            mid,
            source,
        })
    })
    .transpose()
}

pub async fn latest_signal(pool: &SqlitePool, symbol: &Symbol) -> Result<Option<Signal>> {
    let row = sqlx::query_as::<_, (f64, f64, f64, f64, String, i64, i64, String)>(
        "SELECT price, rsi, ema, momentum, action, strength, max_strength, conditions
         FROM signals WHERE symbol = ? ORDER BY timestamp DESC LIMIT 1",
    )
    .bind(symbol.as_str())
    .fetch_optional(pool)
    .await?;
    row.map(
        |(price, rsi14, ema20, momentum, action, strength, max_strength, conditions)| {
            build_signal(
                price,
                rsi14,
                ema20,
                momentum,
                &action,
                strength,
                max_strength,
                &conditions,
            )
        },
    )
    .transpose()
}

/// Historic signals for a symbol, oldest first.
pub async fn load_signals(pool: &SqlitePool, symbol: &Symbol, limit: u32) -> Result<Vec<Signal>> {
    let rows = sqlx::query_as::<_, (f64, f64, f64, f64, String, i64, i64, String)>(
        "SELECT price, rsi, ema, momentum, action, strength, max_strength, conditions
         FROM signals WHERE symbol = ? ORDER BY timestamp DESC LIMIT ?",
    )
    .bind(symbol.as_str())
    .bind(limit)
    .fetch_all(pool)
    .await?;
    let mut out = Vec::with_capacity(rows.len());
    for (price, rsi14, ema20, momentum, action, strength, max_strength, conditions) in rows {
        out.push(build_signal(
            price,
            rsi14,
            ema20,
            momentum,
            &action,
            strength,
            max_strength,
            &conditions,
        )?);
    }
    out.reverse();
    Ok(out)
}

pub async fn load_candles(
    pool: &SqlitePool,
    symbol: &Symbol,
    timeframe: Timeframe,
    limit: u32,
) -> Result<Vec<Candle>> {
    let rows = sqlx::query_as::<_, (i64, f64, f64, f64, f64, f64, String)>(
        "SELECT timestamp, open, high, low, close, volume, source FROM candles
         WHERE symbol = ? AND timeframe = ?
         ORDER BY timestamp DESC LIMIT ?",
    )
    .bind(symbol.as_str())
    .bind(timeframe.as_str())
    .bind(limit)
    .fetch_all(pool)
    .await?;

    let mut out = Vec::with_capacity(rows.len());
    for (ts, open, high, low, close, volume, source) in rows {
        out.push(Candle {
            symbol: symbol.clone(),
            timestamp: from_ms(ts)?,
            open,
            high,
            low,
            close,
            volume,
            timeframe,
            source,
        });
    }
    out.reverse(); // oldest first, provider-compatible
    Ok(out)
}

/// Retention pruning (docs/CONFIGURATION.md): candles/quotes/signals older
/// than `older_than_ms` are pruned. Returns rows deleted per table.
pub async fn prune_older_than(pool: &SqlitePool, older_than_ms: i64) -> Result<(u64, u64, u64)> {
    let c = sqlx::query("DELETE FROM candles WHERE timestamp < ?")
        .bind(older_than_ms)
        .execute(pool)
        .await?;
    let q = sqlx::query("DELETE FROM quotes WHERE timestamp < ?")
        .bind(older_than_ms)
        .execute(pool)
        .await?;
    let s = sqlx::query("DELETE FROM signals WHERE timestamp < ?")
        .bind(older_than_ms)
        .execute(pool)
        .await?;
    Ok((c.rows_affected(), q.rows_affected(), s.rows_affected()))
}

pub async fn count_candles(pool: &SqlitePool) -> Result<i64> {
    let (n,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM candles")
        .fetch_one(pool)
        .await?;
    Ok(n)
}

pub async fn count_quotes(pool: &SqlitePool) -> Result<i64> {
    let (n,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM quotes")
        .fetch_one(pool)
        .await?;
    Ok(n)
}

pub async fn count_signals(pool: &SqlitePool) -> Result<i64> {
    let (n,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM signals")
        .fetch_one(pool)
        .await?;
    Ok(n)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::signal::engine::MAX_STRENGTH;

    async fn setup() -> SqlitePool {
        static N: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let n = N.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!("aurum-repo-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join(format!("repo-{n}.db"));
        let _ = std::fs::remove_file(&path);
        crate::storage::db::open(&path).await.unwrap()
    }

    fn candle(ts_ms: i64, tf: Timeframe) -> Candle {
        Candle {
            symbol: Symbol::normalize("XAUUSD").unwrap(),
            timestamp: chrono::DateTime::from_timestamp_millis(ts_ms).unwrap(),
            open: 10.0,
            high: 12.0,
            low: 9.0,
            close: 11.0,
            volume: 5.0,
            timeframe: tf,
            source: "fixture".into(),
        }
    }

    fn sample_signal(action: Action) -> Signal {
        Signal {
            action,
            strength: 3,
            max_strength: MAX_STRENGTH,
            conditions: Conditions {
                rsi_oversold: true,
                rsi_overbought: false,
                price_above_ema: true,
                price_below_ema: false,
                positive_momentum: true,
                negative_momentum: false,
            },
            price: 3700.0,
            ema20: 3680.0,
            rsi14: 28.0,
            momentum: 0.5,
        }
    }

    #[tokio::test]
    async fn candle_roundtrip_and_upsert_dedupes() {
        let pool = setup().await;
        let symbol = Symbol::normalize("XAUUSD").unwrap();
        let tf = Timeframe::M5;
        save_candles(&pool, &[candle(1000, tf), candle(6000, tf)])
            .await
            .unwrap();
        save_candles(
            &pool,
            &[Candle {
                close: 99.0,
                ..candle(1000, tf)
            }],
        )
        .await
        .unwrap();
        let loaded = load_candles(&pool, &symbol, tf, 10).await.unwrap();
        assert_eq!(loaded.len(), 2, "dedupe by PK");
        assert!((loaded[0].close - 99.0).abs() < 1e-9);
        assert_eq!(loaded[0].timestamp.timestamp_millis(), 1000);
        assert_eq!(count_candles(&pool).await.unwrap(), 2);
    }

    #[tokio::test]
    async fn quote_roundtrip() {
        let pool = setup().await;
        let symbol = Symbol::normalize("XAUUSD").unwrap();
        let ts = chrono::DateTime::from_timestamp_millis(12345).unwrap();
        save_quote(&pool, &Quote::new(symbol.clone(), ts, 10.0, 10.3, "demo"))
            .await
            .unwrap();
        let q = latest_quote(&pool, &symbol).await.unwrap().unwrap();
        assert_eq!(q.bid, 10.0);
        assert!((q.mid - 10.15).abs() < 1e-9);
        assert_eq!(q.source, "demo");
        assert_eq!(count_quotes(&pool).await.unwrap(), 1);
    }

    #[tokio::test]
    async fn signal_roundtrip_preserves_conditions_and_action() {
        let pool = setup().await;
        let symbol = Symbol::normalize("XAUUSD").unwrap();
        save_signals(&pool, &[sample_signal(Action::Buy)], &symbol)
            .await
            .unwrap();
        let loaded = latest_signal(&pool, &symbol).await.unwrap().unwrap();
        assert_eq!(loaded.action, Action::Buy);
        assert_eq!(loaded.strength, 3);
        assert_eq!(loaded.max_strength, 3);
        assert_eq!(loaded.conditions, sample_signal(Action::Buy).conditions);
        assert_eq!(loaded.rsi14, 28.0);
        assert_eq!(count_signals(&pool).await.unwrap(), 1);
    }

    #[tokio::test]
    async fn signals_load_history_oldest_first() {
        let pool = setup().await;
        let symbol = Symbol::normalize("XAUUSD").unwrap();
        save_signals(&pool, &[sample_signal(Action::Buy)], &symbol)
            .await
            .unwrap();
        std::thread::sleep(std::time::Duration::from_millis(2));
        save_signals(&pool, &[sample_signal(Action::Sell)], &symbol)
            .await
            .unwrap();
        // Distinct rotation through upsert conflict is possible in same ms;
        // two rows expected given 2ms apart timestamps.
        let all = load_signals(&pool, &symbol, 10).await.unwrap();
        assert_eq!(all.len(), 2);
        assert_eq!(all[0].action, Action::Buy);
        assert_eq!(all[1].action, Action::Sell);
    }

    #[tokio::test]
    async fn retention_prune_removes_old_rows() {
        let pool = setup().await;
        let tf = Timeframe::M5;
        save_candles(&pool, &[candle(100_000, tf), candle(200_000, tf)])
            .await
            .unwrap();
        let ts = chrono::DateTime::from_timestamp_millis(300_000).unwrap();
        save_quote(
            &pool,
            &Quote::new(Symbol::normalize("XAUUSD").unwrap(), ts, 1.0, 1.1, "demo"),
        )
        .await
        .unwrap();
        let (c, q, s) = prune_older_than(&pool, 150_000).await.unwrap();
        assert_eq!(c, 1);
        assert_eq!(q, 0);
        assert_eq!(s, 0);
        assert_eq!(count_candles(&pool).await.unwrap(), 1);
    }

    #[tokio::test]
    async fn empty_queries_return_none_none() {
        let pool = setup().await;
        let symbol = Symbol::normalize("XAUUSD").unwrap();
        assert!(latest_quote(&pool, &symbol).await.unwrap().is_none());
        assert!(latest_signal(&pool, &symbol).await.unwrap().is_none());
    }
}
