use sqlx::SqlitePool;
use tokio::sync::RwLock;

use crate::config::Config;
use crate::error::{Error, Result};
use crate::market::models::{Candle, Quote, Symbol};
use crate::market::provider::MarketDataProvider;
use crate::market::{MassiveProvider, ReplayProvider};
use crate::signal::{Signal, evaluate};

/// Active provider behind an enum (no dyn dispatch; native async traits).
pub enum Active {
    Replay(ReplayProvider),
    Massive(MassiveProvider),
}

/// Clean application state (README §33): provider + storage wiring.
/// Business logic never reads env vars directly (docs/BUILD.md).
pub struct App {
    pub config: Config,
    db: SqlitePool,
    provider: RwLock<Active>,
}

impl App {
    pub async fn from_config(config: Config) -> Result<Self> {
        let provider = match config.mode {
            crate::config::Mode::Demo => Active::Replay(ReplayProvider::load_default()?),
            _ => {
                let key = config
                    .massive_api_key()
                    .ok_or(Error::missing_credential("MASSIVE_API_KEY"))?;
                Active::Massive(MassiveProvider::with_base_url(
                    &key,
                    &config.massive_api_base,
                )?)
            }
        };
        let db = crate::storage::open(&config.db_path).await?;
        Ok(Self {
            config,
            db,
            provider: RwLock::new(provider),
        })
    }

    pub async fn is_demo(&self) -> bool {
        matches!(*self.provider.read().await, Active::Replay(_))
    }

    pub async fn db(&self) -> &SqlitePool {
        &self.db
    }

    fn timeframe(&self) -> Result<crate::market::Timeframe> {
        self.config.timeframe.parse().map_err(|_| {
            Error::Config(format!(
                "invalid configured timeframe: {}",
                self.config.timeframe
            ))
        })
    }

    /// Latest quote: fetch → persist → retention prune.
    pub async fn quote(&self, symbol: &Symbol) -> Result<Quote> {
        let q = match &*self.provider.read().await {
            Active::Replay(p) => p.quote(symbol).await?,
            Active::Massive(p) => p.quote(symbol).await?,
        };
        crate::storage::save_quote(&self.db, &q).await?;
        self.prune().await?;
        Ok(q)
    }

    /// Quote with degraded-mode fallback (docs/RESILIENCE.md §2):
    /// provider error → latest cached row, tagged `stale: true` with age;
    /// never presented as live. Cache miss → the original error.
    pub async fn quote_with_stale(&self, symbol: &Symbol) -> Result<(Quote, u64)> {
        match self.quote(symbol).await {
            Ok(q) => Ok((q, 0)),
            Err(provider_err) => {
                let cached = crate::storage::repository::latest_quote(&self.db, symbol).await?;
                match cached {
                    Some(q) => {
                        let age_ms = (chrono::Utc::now().timestamp_millis()
                            - q.timestamp.timestamp_millis())
                        .max(0) as u64;
                        tracing::warn!(
                            "provider down; serving stale quote from cache (age {}ms): {}",
                            age_ms,
                            provider_err
                        );
                        Ok((q, age_ms))
                    }
                    None => Err(provider_err),
                }
            }
        }
    }

    /// Candle history: fetch → persist (dedupe) → prune.
    pub async fn history(&self, symbol: &Symbol, limit: u32) -> Result<Vec<Candle>> {
        let tf = self.timeframe()?;
        let candles = match &*self.provider.read().await {
            Active::Replay(p) => p.history(symbol, tf, limit, None).await?,
            Active::Massive(p) => p.history(symbol, tf, limit, None).await?,
        };
        crate::storage::save_candles(&self.db, &candles).await?;
        self.prune().await?;
        Ok(candles)
    }

    /// Historical candles straight from storage (oldest first).
    pub async fn stored_candles(&self, symbol: &Symbol, limit: u32) -> Result<Vec<Candle>> {
        let tf = self.timeframe()?;
        crate::storage::repository::load_candles(&self.db, symbol, tf, limit).await
    }

    /// Flow: provider history → indicators → rule engine.
    /// For daily bars the request window is widened: ~35 evaluation bars need
    /// more calendar days because weekends produce no candles.
    pub async fn compute_signal(&self, symbol: &Symbol) -> Result<Signal> {
        let tf = self.timeframe()?;
        let need = 35u32;
        let fetch = match tf {
            crate::market::Timeframe::D1 => 90,
            crate::market::Timeframe::H4 => 60,
            _ => 45,
        };
        let mut candles = match &*self.provider.read().await {
            Active::Replay(p) => p.history(symbol, tf, fetch, None).await?,
            Active::Massive(p) => p.history(symbol, tf, fetch, None).await?,
        };
        if candles.len() < need as usize {
            return Err(Error::Config(format!(
                "not enough data for a signal on {symbol}: got {} bars of {} timeframe, need {need} — widen the window or use demo mode",
                candles.len(),
                tf.as_str()
            )));
        }
        let closes: Vec<f64> = candles.iter().map(|c| c.close).collect();
        for c in &mut candles {
            c.timeframe = tf;
        }
        crate::storage::save_candles(&self.db, &candles).await?;
        evaluate(&closes)
    }

    /// Persist a computed signal (timestamps inside the repository use now).
    pub async fn persist_signals(&self, signals: &[Signal], symbol: &Symbol) -> Result<()> {
        crate::storage::save_signals(&self.db, signals, symbol).await?;
        self.prune().await?;
        Ok(())
    }

    pub async fn prune(&self) -> Result<()> {
        let cutoff_days = self.config.db_retention_days as i64;
        let cutoff_ms = chrono::Utc::now().timestamp_millis() - cutoff_days * 86_400_000;
        crate::storage::repository::prune_older_than(&self.db, cutoff_ms).await?;
        Ok(())
    }

    pub async fn close_db(&self) {
        self.db.close().await;
    }
}

impl App {
    /// Explicit-wiring constructor (used by tests with a failing provider).
    pub async fn with_parts(config: Config, db: SqlitePool, provider: Active) -> Result<Self> {
        Ok(Self {
            config,
            db,
            provider: RwLock::new(provider),
        })
    }
}

/// Load candles already in the DB (helper for API/storage paths).
pub async fn latest_db_quote(pool: &SqlitePool, symbol: &Symbol) -> Result<Option<Quote>> {
    crate::storage::repository::latest_quote(pool, symbol).await
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn app_with(closed_port_massive: bool) -> App {
        static N: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let n = N.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!("aurum-state-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join(format!("state-{n}.db"));
        let mut config = Config::load_with_env(|_| None).unwrap();
        config.db_path = path.clone();
        config.db_retention_days = 90;
        let db = crate::storage::open(&path).await.unwrap();
        let provider = if closed_port_massive {
            // Port 1 on localhost: connection refused instantly, deterministic failure.
            Active::Massive(MassiveProvider::with_base_url("k", "http://127.0.0.1:1").unwrap())
        } else {
            Active::Replay(ReplayProvider::load_default().unwrap())
        };
        App::with_parts(config, db, provider).await.unwrap()
    }

    fn cached_quote(minute_ago: i64) -> Quote {
        Quote::new(
            Symbol::normalize("XAUUSD").unwrap(),
            chrono::Utc::now() - chrono::Duration::seconds(minute_ago),
            3692.20,
            3692.60,
            "historical",
        )
    }

    #[tokio::test]
    async fn provider_down_serves_cached_quote_tagged_stale() {
        let app = app_with(true).await;
        crate::storage::save_quote(app.db().await, &cached_quote(300))
            .await
            .unwrap();

        let (q, age_ms) = app
            .quote_with_stale(&Symbol::normalize("XAUUSD").unwrap())
            .await
            .unwrap();
        assert_eq!(q.source, "historical");
        assert!((q.mid - 3692.4).abs() < 1e-9);
        assert!(
            age_ms >= 299_000,
            "age must reflect cache age, got {age_ms}"
        );
        app.close_db().await;
    }

    #[tokio::test]
    async fn provider_down_without_cache_is_clear_error() {
        let app = app_with(true).await;
        let err = app
            .quote_with_stale(&Symbol::normalize("EURUSD").unwrap())
            .await
            .unwrap_err();
        let msg = err.to_string();
        assert!(!msg.is_empty());
        assert!(msg.to_lowercase().contains("massive") || msg.contains("request failed"));
        app.close_db().await;
    }

    #[tokio::test]
    async fn fresh_quote_has_zero_age() {
        let app = app_with(false).await;
        let symbol = Symbol::normalize("XAUUSD").unwrap();
        let (q, age_ms) = app.quote_with_stale(&symbol).await.unwrap();
        assert_eq!(age_ms, 0);
        assert_eq!(q.source, "fixture");
        app.close_db().await;
    }

    #[tokio::test]
    async fn demo_computes_and_persists_signal() {
        let app = app_with(false).await;
        let symbol = Symbol::normalize("XAUUSD").unwrap();
        let sig = app.compute_signal(&symbol).await.unwrap();
        app.persist_signals(std::slice::from_ref(&sig), &symbol)
            .await
            .unwrap();
        assert_eq!(
            crate::storage::count_signals(app.db().await).await.unwrap(),
            1
        );
        app.close_db().await;
    }
}
