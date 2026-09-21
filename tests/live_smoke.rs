//! LIVE provider smoke test. Runs ONLY when explicitly enabled:
//!   $env:MASSIVE_API_KEY = "..."; $env:MASSIVE_LIVE = "1"; cargo test --test live_smoke
//! CI must not depend on this (docs/QA.md): skipped by default.

use aurum::market::provider::MarketDataProvider;
use aurum::market::{MassiveProvider, Symbol, Timeframe};

fn live_enabled() -> bool {
    std::env::var("MASSIVE_LIVE").ok().as_deref() == Some("1")
        && std::env::var("MASSIVE_API_KEY")
            .ok()
            .map(|k| !k.is_empty())
            .unwrap_or(false)
}

fn market_key() -> Option<()> {
    live_enabled().then_some(())
}

#[tokio::test]
async fn live_daily_history_returns_real_candles() {
    if market_key().is_none() {
        eprintln!("SKIP: MASSIVE_LIVE/MASSIVE_API_KEY not set");
        return;
    }
    let key = std::env::var("MASSIVE_API_KEY").unwrap();
    let provider = MassiveProvider::new(key).expect("client");
    let symbol = Symbol::normalize("XAUUSD").unwrap();

    let candles = provider.history(&symbol, Timeframe::D1, 3, None).await;
    match candles {
        Ok(c) => {
            assert!(!c.is_empty(), "expected at least one bar");
            for bar in &c {
                assert!(bar.open > 0.0 && bar.close > 0.0);
                assert!(bar.high >= bar.low);
                assert_eq!(bar.source, "massive");
            }
            let last = c.last().unwrap();
            println!(
                "LIVE_OK symbol={} bars={} last_close={} ts={}",
                symbol,
                c.len(),
                last.close,
                last.timestamp
            );
        }
        Err(e) => panic!("live history failed: {e}"),
    }
}

#[tokio::test]
async fn live_intraday_reports_clear_plan_error() {
    if market_key().is_none() {
        eprintln!("SKIP: MASSIVE_LIVE/MASSIVE_API_KEY not set");
        return;
    }
    let key = std::env::var("MASSIVE_API_KEY").unwrap();
    let provider = MassiveProvider::new(key).expect("client");
    let symbol = Symbol::normalize("XAUUSD").unwrap();

    let err = provider
        .history(&symbol, Timeframe::M5, 5, None)
        .await
        .unwrap_err();
    let msg = err.to_string();
    println!("LIVE_PLAN_LIMIT_OK: {msg}");
    assert!(!msg.is_empty());
}
