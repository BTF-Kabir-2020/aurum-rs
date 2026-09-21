use serde::Serialize;

use crate::error::Error;
use crate::market::models::{Candle, Quote};
use crate::signal::engine::MAX_STRENGTH;
use crate::signal::{Action, Conditions, Signal};

pub const API_VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Debug, Serialize)]
pub struct StatusDto {
    pub version: &'static str,
    pub status: &'static str,
    pub mode: String,
    pub provider: String,
    pub symbol: String,
}

#[derive(Debug, Serialize)]
pub struct QuoteDto {
    pub version: &'static str,
    pub symbol: String,
    pub canonical_symbol: String,
    pub timestamp: String,
    pub bid: f64,
    pub ask: f64,
    pub mid: f64,
    pub source: String,
    pub stale: bool,
    pub age_seconds: u64,
}

#[derive(Debug, Serialize)]
pub struct CandleDto {
    pub timestamp: String,
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    pub volume: f64,
    pub source: String,
}

#[derive(Debug, Serialize)]
pub struct HistoryDto {
    pub version: &'static str,
    pub symbol: String,
    pub mode: String,
    pub timeframe: String,
    pub count: usize,
    pub candles: Vec<CandleDto>,
}

#[derive(Debug, Serialize)]
pub struct IndicatorsDto {
    pub ema20: f64,
    pub rsi14: f64,
    pub momentum: f64,
}

#[derive(Debug, Serialize)]
pub struct SignalRuleDto {
    pub action: String,
    pub strength: u8,
    pub max_strength: u8,
}

#[derive(Debug, Serialize)]
pub struct SignalDto {
    pub version: &'static str,
    pub symbol: String,
    pub mode: String,
    pub timestamp: String,
    pub price: f64,
    pub indicators: IndicatorsDto,
    pub signal: SignalRuleDto,
    pub conditions: Conditions,
    pub data: DataTagDto,
}

#[derive(Debug, Serialize)]
pub struct DataTagDto {
    pub source: String,
    pub stale: bool,
}

#[derive(Debug, Serialize)]
pub struct ApiError {
    pub version: &'static str,
    pub error: String,
}

impl From<Action> for String {
    fn from(a: Action) -> Self {
        match a {
            Action::Buy => "BUY".into(),
            Action::Sell => "SELL".into(),
            Action::Hold => "HOLD".into(),
        }
    }
}

fn iso(dt: chrono::DateTime<chrono::Utc>) -> String {
    dt.to_rfc3339_opts(chrono::SecondsFormat::Secs, true)
}

pub fn quote_dto(_mode: &str, q: &Quote) -> QuoteDto {
    QuoteDto {
        version: API_VERSION,
        symbol: q.symbol.display(),
        canonical_symbol: q.symbol.as_str().to_string(),
        timestamp: q.timestamp.to_rfc3339(),
        bid: q.bid,
        ask: q.ask,
        mid: q.mid,
        source: q.source.clone(),
        stale: false,
        age_seconds: 0,
    }
}

pub fn stale_quote_dto(_mode: &str, q: &Quote, age_seconds: u64) -> QuoteDto {
    QuoteDto {
        stale: true,
        age_seconds,
        ..quote_dto(_mode, q)
    }
}

pub fn history_dto(mode: &str, timeframe: &str, candles: &[Candle]) -> HistoryDto {
    HistoryDto {
        version: API_VERSION,
        symbol: candles
            .first()
            .map(|c| c.symbol.display())
            .unwrap_or_default(),
        mode: mode.to_string(),
        timeframe: timeframe.to_string(),
        count: candles.len(),
        candles: candles
            .iter()
            .map(|c: &Candle| CandleDto {
                timestamp: iso(c.timestamp),
                open: c.open,
                high: c.high,
                low: c.low,
                close: c.close,
                volume: c.volume,
                source: c.source.clone(),
            })
            .collect(),
    }
}

pub fn signal_dto(
    mode: &str,
    source: &str,
    symbol: &crate::market::Symbol,
    s: &Signal,
) -> SignalDto {
    SignalDto {
        version: API_VERSION,
        symbol: symbol.display(),
        mode: mode.to_string(),
        timestamp: chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
        price: s.price,
        indicators: IndicatorsDto {
            ema20: s.ema20,
            rsi14: s.rsi14,
            momentum: s.momentum,
        },
        signal: SignalRuleDto {
            action: String::from(s.action),
            strength: s.strength,
            max_strength: MAX_STRENGTH,
        },
        conditions: s.conditions,
        data: DataTagDto {
            source: source.to_string(),
            stale: false,
        },
    }
}

/// HTTP status + actionable message for domain errors (README §21).
pub fn api_status(e: &Error) -> (axum::http::StatusCode, String) {
    let msg = e.to_string();
    let code = match e {
        Error::Symbol(_) | Error::Config(_) | Error::InvalidValue { .. } => {
            axum::http::StatusCode::BAD_REQUEST
        }
        Error::MissingCredential(_) => axum::http::StatusCode::SERVICE_UNAVAILABLE,
        Error::Provider(_) => axum::http::StatusCode::BAD_GATEWAY,
        Error::Storage(_) | Error::Database(_) | Error::Io(_) => {
            axum::http::StatusCode::INTERNAL_SERVER_ERROR
        }
        Error::Http(_) => axum::http::StatusCode::BAD_GATEWAY,
        _ => axum::http::StatusCode::INTERNAL_SERVER_ERROR,
    };
    (code, msg)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::signal::Conditions;
    use crate::signal::Signal;

    fn sample_signal() -> Signal {
        Signal {
            action: crate::signal::Action::Buy,
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
            price: 3692.39,
            ema20: 3689.04,
            rsi14: 28.31,
            momentum: 0.10,
        }
    }

    fn symbol() -> crate::market::Symbol {
        crate::market::Symbol::normalize("XAUUSD").unwrap()
    }

    #[test]
    fn signal_dto_matches_readme_shape() {
        let dto = signal_dto("demo", "fixture", &symbol(), &sample_signal());
        let json = serde_json::to_value(&dto).unwrap();
        assert_eq!(json["symbol"], "XAU/USD");
        assert_eq!(json["mode"], "demo");
        assert_eq!(json["indicators"]["ema20"], serde_json::json!(3689.04));
        assert_eq!(json["signal"]["action"], "BUY");
        assert_eq!(json["signal"]["max_strength"], 3);
        assert_eq!(json["data"]["source"], "fixture");
        assert_eq!(json["version"], API_VERSION);
    }

    #[test]
    fn quote_dto_display_and_canonical() {
        let q = Quote::new(symbol(), chrono::Utc::now(), 3692.20, 3692.60, "fixture");
        let json = serde_json::to_value(quote_dto("demo", &q)).unwrap();
        assert_eq!(json["symbol"], "XAU/USD");
        assert_eq!(json["canonical_symbol"], "XAUUSD");
        assert!(json["timestamp"].as_str().unwrap().contains('T'));
    }

    #[test]
    fn error_mapping_pickable() {
        let (code, _) = api_status(&Error::Symbol("bad".into()));
        assert_eq!(code, axum::http::StatusCode::BAD_REQUEST);
        let (code, msg) = api_status(&Error::Provider("HTTP 429 rate limit exceeded".into()));
        assert_eq!(code, axum::http::StatusCode::BAD_GATEWAY);
        assert!(msg.contains("429"));
        let (code, _) = api_status(&Error::missing_credential("MASSIVE_API_KEY"));
        assert_eq!(code, axum::http::StatusCode::SERVICE_UNAVAILABLE);
    }

    #[test]
    fn history_dto_counts_candles() {
        use chrono::{TimeZone, Utc};
        let ts = Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap();
        let c = Candle {
            symbol: symbol(),
            timestamp: ts,
            open: 100.0,
            high: 110.0,
            low: 95.0,
            close: 105.0,
            volume: 3.0,
            timeframe: crate::market::Timeframe::M5,
            source: "replay".into(),
        };
        let dto = history_dto("demo", "5m", &[c]);
        assert_eq!(dto.count, 1);
        let json = serde_json::to_value(&dto).unwrap();
        assert_eq!(json["candles"][0]["close"], serde_json::json!(105.0));
        assert!(
            json["candles"][0]["timestamp"]
                .as_str()
                .unwrap()
                .ends_with('Z')
        );
    }
}
