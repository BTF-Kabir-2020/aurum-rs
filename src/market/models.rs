use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};

/// Canonical trading symbol. Internally always the bare form, e.g. `XAUUSD`.
/// Display form is `XAU/USD`; provider form is `C:XAUUSD` (README §51).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Symbol(String);

impl Symbol {
    /// One canonical normalization function (README §51).
    /// Accepts `XAUUSD`, `XAU/USD`, `C:XAUUSD`, `xauusd` → `XAUUSD`.
    pub fn normalize(input: &str) -> Result<Self> {
        let raw = input.trim();
        if raw.is_empty() {
            return Err(Error::Symbol("empty symbol".into()));
        }

        let mut s = raw.to_ascii_uppercase();
        if let Some(rest) = s.strip_prefix("C:") {
            s = rest.to_string();
        }
        s.retain(|c| c != '/' && c != ' ' && c != '-' && c != '_');

        if s.len() != 6 || !s.chars().all(|c| c.is_ascii_alphabetic()) {
            return Err(Error::Symbol(format!(
                "'{raw}' is not a valid symbol (expected base/quote like XAUUSD)"
            )));
        }

        Ok(Self(s))
    }

    /// Loose validity check used by config validation before a `Symbol` exists.
    pub fn is_valid_str(input: &str) -> bool {
        Self::normalize(input).is_ok()
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn base(&self) -> &str {
        &self.0[..3]
    }

    pub fn quote(&self) -> &str {
        &self.0[3..]
    }

    /// `XAUUSD` → `XAU/USD` (UI display form).
    pub fn display(&self) -> String {
        format!("{}/{}", self.base(), self.quote())
    }

    /// `XAUUSD` → `C:XAUUSD` (provider form, Massive).
    pub fn provider_form(&self) -> String {
        format!("C:{}", self.0)
    }
}

impl std::fmt::Display for Symbol {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::str::FromStr for Symbol {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self> {
        Self::normalize(s)
    }
}

/// Supported timeframe granularities (v0.1 targets 5m).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Timeframe {
    M1,
    M5,
    M15,
    H1,
    H4,
    D1,
}

impl Timeframe {
    pub fn as_str(self) -> &'static str {
        match self {
            Timeframe::M1 => "1m",
            Timeframe::M5 => "5m",
            Timeframe::M15 => "15m",
            Timeframe::H1 => "1h",
            Timeframe::H4 => "4h",
            Timeframe::D1 => "1d",
        }
    }
}

impl std::str::FromStr for Timeframe {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self> {
        match s.trim() {
            "1m" => Ok(Timeframe::M1),
            "5m" => Ok(Timeframe::M5),
            "15m" => Ok(Timeframe::M15),
            "1h" => Ok(Timeframe::H1),
            "4h" => Ok(Timeframe::H4),
            "1d" => Ok(Timeframe::D1),
            other => Err(Error::Timeframe(other.to_string())),
        }
    }
}

/// OHLCV candle, UTC timestamps everywhere (README §52).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Candle {
    pub symbol: Symbol,
    pub timestamp: DateTime<Utc>,
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    pub volume: f64,
    pub timeframe: Timeframe,
    pub source: String,
}

impl Candle {
    pub fn validate(&self) -> Result<()> {
        if !(self.high >= self.low
            && self.high >= self.open
            && self.high >= self.close
            && self.low <= self.open
            && self.low <= self.close)
        {
            return Err(Error::InvalidValue {
                field: "candle",
                reason: format!(
                    "inconsistent OHLC at {}: H={} L={} O={} C={}",
                    self.timestamp, self.high, self.low, self.open, self.close
                ),
            });
        }
        if self.volume < 0.0 {
            return Err(Error::InvalidValue {
                field: "candle",
                reason: "negative volume".into(),
            });
        }
        Ok(())
    }
}

/// Top-of-book quote; `mid` is derived, `source` marks demo/historical/live.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Quote {
    pub symbol: Symbol,
    pub timestamp: DateTime<Utc>,
    pub bid: f64,
    pub ask: f64,
    pub mid: f64,
    pub source: String,
}

impl Quote {
    pub fn new(
        symbol: Symbol,
        timestamp: DateTime<Utc>,
        bid: f64,
        ask: f64,
        source: impl Into<String>,
    ) -> Self {
        Self {
            symbol,
            timestamp,
            bid,
            ask,
            mid: (bid + ask) / 2.0,
            source: source.into(),
        }
    }

    pub fn validate(&self) -> Result<()> {
        if self.bid <= 0.0 || self.ask <= 0.0 {
            return Err(Error::InvalidValue {
                field: "quote",
                reason: "bid/ask must be positive".into(),
            });
        }
        if self.ask < self.bid {
            return Err(Error::InvalidValue {
                field: "quote",
                reason: "ask below bid (crossed book)".into(),
            });
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    #[test]
    fn normalize_all_documented_aliases() {
        for input in ["XAUUSD", "XAU/USD", "C:XAUUSD", "xauusd", " xau / usd "] {
            let s = Symbol::normalize(input).unwrap();
            assert_eq!(s, Symbol("XAUUSD".into()), "input: {input}");
        }
    }

    #[test]
    fn display_and_provider_forms() {
        let s = Symbol::normalize("xau/usd").unwrap();
        assert_eq!(s.display(), "XAU/USD");
        assert_eq!(s.provider_form(), "C:XAUUSD");
        assert_eq!(s.base(), "XAU");
        assert_eq!(s.quote(), "USD");
    }

    #[test]
    fn invalid_symbols_rejected() {
        for bad in ["", "XAU", "123456", "XAU/USDTT!!", "GOLD BAR"] {
            assert!(Symbol::normalize(bad).is_err(), "input: {bad}");
        }
    }

    #[test]
    fn timeframe_roundtrip() {
        for tf in ["1m", "5m", "15m", "1h", "4h", "1d"] {
            let parsed: Timeframe = tf.parse().unwrap();
            assert_eq!(parsed.as_str(), tf);
        }
        assert!("7s".parse::<Timeframe>().is_err());
    }

    #[test]
    fn candle_validation_catches_bad_ohlc() {
        let ts = Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap();
        let ok = Candle {
            symbol: Symbol::normalize("XAUUSD").unwrap(),
            timestamp: ts,
            open: 100.0,
            high: 110.0,
            low: 95.0,
            close: 105.0,
            volume: 12.0,
            timeframe: Timeframe::M5,
            source: "fixture".into(),
        };
        assert!(ok.validate().is_ok());

        let bad = Candle { high: 90.0, ..ok };
        assert!(bad.validate().is_err());
    }

    #[test]
    fn quote_mid_and_validation() {
        let ts = Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap();
        let q = Quote::new(
            Symbol::normalize("XAUUSD").unwrap(),
            ts,
            3680.10,
            3681.00,
            "demo",
        );
        assert!((q.mid - 3680.55).abs() < 1e-9);
        assert!(q.validate().is_ok());

        let crossed = Quote::new(q.symbol.clone(), ts, 3682.0, 3681.0, "demo");
        assert!(crossed.validate().is_err());
    }
}
