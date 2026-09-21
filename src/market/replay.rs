use std::path::{Path, PathBuf};

use chrono::{DateTime, Utc};

use crate::error::{Error, Result};
use crate::market::models::{Candle, Quote, Symbol, Timeframe};

/// Default fixture bundled with the repo (DEMO mode, offline, no credentials).
pub const DEFAULT_FIXTURE: &str = "fixtures/xauusd_5m.csv";

/// Same fixture embedded at compile time: `cargo install --path .` users (or
/// Docker WORKDIR changes) can run from anywhere (README §15, §30).
const EMBEDDED_FIXTURE: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/fixtures/xauusd_5m.csv"
));

const FIXTURE_SOURCE: &str = "fixture";
const DEMO_SPREAD: f64 = 0.30;

/// Offline deterministic DEMO provider (docs/PROVIDERS.md).
/// Serves candles from a bundled CSV fixture; same input → same output, ever.
#[derive(Debug)]
pub struct ReplayProvider {
    fixture_path: PathBuf,
    symbol: Symbol,
    candles: Vec<Candle>,
}

#[allow(dead_code)]
impl ReplayProvider {
    pub fn load(fixture: impl AsRef<Path>) -> Result<Self> {
        let raw = std::fs::read_to_string(fixture.as_ref()).map_err(|e| {
            Error::Provider(format!(
                "replay fixture cannot be read: {} ({e})",
                fixture.as_ref().display()
            ))
        })?;
        let candles = parse_fixture(&raw)?;
        let symbol = candles
            .first()
            .map(|c| c.symbol.clone())
            .ok_or_else(|| Error::Provider("replay fixture is empty".into()))?;
        Ok(Self {
            fixture_path: fixture.as_ref().to_path_buf(),
            symbol,
            candles,
        })
    }

    pub fn load_default() -> Result<Self> {
        match Self::load(Path::new(DEFAULT_FIXTURE)) {
            Ok(p) => Ok(p),
            // File not present in the working directory → compiled-in copy.
            Err(e) if e.to_string().contains("cannot be read") => {
                let candles = parse_fixture(EMBEDDED_FIXTURE)?;
                let symbol = candles
                    .first()
                    .map(|c| c.symbol.clone())
                    .ok_or_else(|| Error::Provider("replay fixture is empty".into()))?;
                tracing::debug!("replay: using compiled-in fixture (no file at {DEFAULT_FIXTURE})");
                Ok(Self {
                    fixture_path: PathBuf::from(DEFAULT_FIXTURE),
                    symbol,
                    candles,
                })
            }
            Err(e) => Err(e),
        }
    }

    pub fn fixture_path(&self) -> &Path {
        &self.fixture_path
    }

    pub fn candle_count(&self) -> usize {
        self.candles.len()
    }

    /// Quote derived from the latest candle close ± a fixed demo spread.
    fn latest_quote(&self) -> Result<Quote> {
        let last = self
            .candles
            .last()
            .ok_or_else(|| Error::Provider("replay fixture is empty".into()))?;
        Ok(Quote::new(
            last.symbol.clone(),
            last.timestamp,
            last.close - DEMO_SPREAD / 2.0,
            last.close + DEMO_SPREAD / 2.0,
            FIXTURE_SOURCE,
        ))
    }
}

impl crate::market::provider::MarketDataProvider for ReplayProvider {
    fn name(&self) -> &'static str {
        "replay"
    }

    async fn quote(&self, symbol: &Symbol) -> Result<Quote> {
        if symbol != &self.symbol {
            return Err(Error::Provider(format!(
                "replay fixture only covers {}; requested {symbol}",
                self.symbol
            )));
        }
        self.latest_quote()
    }

    async fn history(
        &self,
        symbol: &Symbol,
        timeframe: Timeframe,
        limit: u32,
        until: Option<DateTime<Utc>>,
    ) -> Result<Vec<Candle>> {
        if symbol != &self.symbol {
            return Err(Error::Provider(format!(
                "replay fixture only covers {}; requested {symbol}",
                self.symbol
            )));
        }
        if limit == 0 {
            return Ok(Vec::new());
        }

        let mut out: Vec<Candle> = self
            .candles
            .iter()
            .filter(|c| c.timeframe == timeframe)
            .filter(|c| until.is_none_or(|u| c.timestamp <= u))
            .cloned()
            .collect();
        if out.len() > limit as usize {
            out.drain(..out.len() - limit as usize);
        }
        Ok(out)
    }
}

/// Fixture CSV format: `timestamp,open,high,low,close,volume` (header row).
fn parse_fixture(raw: &str) -> Result<Vec<Candle>> {
    let mut candles = Vec::new();
    for (idx, line) in raw.lines().enumerate() {
        let line = line.trim_end_matches('\r');
        if line.is_empty() {
            continue;
        }
        if idx == 0 && line.starts_with("timestamp") {
            continue;
        }
        candles.push(
            parse_row(line)
                .map_err(|e| Error::Provider(format!("fixture row {}: {e}", idx + 1)))?,
        );
    }
    for c in &candles {
        c.validate().map_err(|e| {
            Error::Provider(format!(
                "fixture row for {} failed validation: {e}",
                c.timestamp
            ))
        })?;
    }
    Ok(candles)
}

fn parse_row(line: &str) -> Result<Candle> {
    let parts: Vec<&str> = line.split(',').map(str::trim).collect();
    if parts.len() != 6 {
        return Err(Error::Provider(format!(
            "expected 6 columns, got {}",
            parts.len()
        )));
    }
    let parse_num = |s: &str, col: &str| -> Result<f64> {
        s.parse::<f64>()
            .map_err(|e| Error::Provider(format!("cannot parse {col} '{s}': {e}")))
    };
    let timestamp = DateTime::parse_from_rfc3339(parts[0])
        .map_err(|e| Error::Provider(format!("cannot parse timestamp '{}': {e}", parts[0])))?
        .with_timezone(&Utc);

    Ok(Candle {
        symbol: Symbol::normalize("XAUUSD")?,
        timestamp,
        open: parse_num(parts[1], "open")?,
        high: parse_num(parts[2], "high")?,
        low: parse_num(parts[3], "low")?,
        close: parse_num(parts[4], "close")?,
        volume: parse_num(parts[5], "volume")?,
        timeframe: Timeframe::M5,
        source: FIXTURE_SOURCE.to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::market::provider::MarketDataProvider;

    fn manifest_fixture() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("fixtures/xauusd_5m.csv")
    }

    fn fixture() -> ReplayProvider {
        ReplayProvider::load(manifest_fixture()).unwrap()
    }

    #[tokio::test]
    async fn fixture_loads_with_240_candles() {
        let p = fixture();
        assert_eq!(p.candle_count(), 240);
        assert_eq!(p.name(), "replay");
        assert_eq!(p.symbol, Symbol::normalize("XAUUSD").unwrap());
        assert!(p.fixture_path().ends_with("xauusd_5m.csv"));
    }

    #[tokio::test]
    async fn quote_is_deterministic_around_last_close() {
        let p = fixture();
        let symbol = p.symbol.clone();
        let q1 = p.quote(&symbol).await.unwrap();
        let q2 = p.quote(&symbol).await.unwrap();
        assert_eq!(q1, q2);
        assert_eq!(q1.source, "fixture");
        assert_eq!(q1.symbol, symbol);
        let last_close = 3692.39;
        assert!((q1.mid - last_close).abs() < 1e-9);
        assert!(q1.validate().is_ok());
    }

    #[tokio::test]
    async fn quote_rejects_foreign_symbol() {
        let p = fixture();
        let eur = Symbol::normalize("EURUSD").unwrap();
        let err = p.quote(&eur).await.unwrap_err();
        assert!(err.to_string().contains("only covers"));
    }

    #[tokio::test]
    async fn history_respects_limit_and_timeframe() {
        let p = fixture();
        let symbol = p.symbol.clone();
        let h = p.history(&symbol, Timeframe::M5, 20, None).await.unwrap();
        assert_eq!(h.len(), 20);
        assert_eq!(
            h.last().unwrap().timestamp.to_rfc3339(),
            "2026-09-14T19:55:00+00:00"
        );
        assert!(h.windows(2).all(|w| w[0].timestamp < w[1].timestamp));

        let daily = p.history(&symbol, Timeframe::D1, 20, None).await.unwrap();
        assert!(daily.is_empty());
    }

    #[tokio::test]
    async fn history_until_filters_future_bars() {
        let p = fixture();
        let symbol = p.symbol.clone();
        let cut = DateTime::parse_from_rfc3339("2026-09-14T01:00:00Z")
            .unwrap()
            .with_timezone(&Utc);
        let h = p
            .history(&symbol, Timeframe::M5, 240, Some(cut))
            .await
            .unwrap();
        assert_eq!(h.len(), 13);
        assert!(h.iter().all(|c| c.timestamp <= cut));
    }

    #[tokio::test]
    async fn zero_limit_is_empty_not_error() {
        let p = fixture();
        let symbol = p.symbol.clone();
        let h = p.history(&symbol, Timeframe::M5, 0, None).await.unwrap();
        assert!(h.is_empty());
    }

    #[test]
    fn missing_fixture_is_actionable() {
        let err = ReplayProvider::load("does/not/exist.csv").unwrap_err();
        assert!(err.to_string().contains("cannot be read"));
    }

    #[test]
    fn malformed_rows_are_rejected_with_row_number() {
        let raw = "timestamp,open,high,low,close,volume\n2026-09-14T00:00:00Z,1,2,3\n";
        let err = parse_fixture(raw).unwrap_err();
        assert!(err.to_string().contains("row 2"));
    }

    #[test]
    fn inconsistent_ohlc_in_fixture_is_rejected() {
        let raw = concat!(
            "timestamp,open,high,low,close,volume\n",
            "2026-09-14T00:00:00Z,100.00,90.00,80.00,95.00,10\n"
        );
        let err = parse_fixture(raw).unwrap_err();
        assert!(err.to_string().contains("failed validation"));
    }

    #[test]
    fn empty_fixture_rejected() {
        let dir = std::env::temp_dir().join(format!("aurum-replay-empty-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("empty.csv");
        std::fs::write(&path, "").unwrap();
        let err = ReplayProvider::load(&path).unwrap_err();
        assert!(err.to_string().contains("empty"));
        std::fs::remove_dir_all(&dir).ok();
    }
}
