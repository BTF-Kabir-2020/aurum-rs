//! Offline full-flow smoke: fixture → candles → indicators → signal (README §60 Phase 5 flow).
//! Offline only — no network, no credentials.

use aurum::indicators::{ema, momentum, rsi};
use aurum::market::provider::MarketDataProvider;
use aurum::market::{ReplayProvider, Symbol, Timeframe};
use aurum::signal::evaluate;

#[tokio::test]
async fn fixture_to_signal_flow_is_consistent() {
    let provider = ReplayProvider::load(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("fixtures/xauusd_5m.csv"),
    )
    .expect("fixture loads");
    let symbol = Symbol::normalize("XAUUSD").unwrap();

    let candles = provider
        .history(&symbol, Timeframe::M5, 40, None)
        .await
        .expect("history");
    assert_eq!(candles.len(), 40);

    let closes: Vec<f64> = candles.iter().map(|c| c.close).collect();
    let signal = evaluate(&closes).expect("evaluate");

    // Wiring: signal values must equal raw indicator outputs on the same data.
    let e = ema(&closes, 20).unwrap().last().unwrap().unwrap();
    let r = rsi(&closes, 14).unwrap().last().unwrap().unwrap();
    let m = momentum(&closes, 10).unwrap().last().unwrap().unwrap();
    assert_eq!(signal.ema20, e);
    assert_eq!(signal.rsi14, r);
    assert_eq!(signal.momentum, m);

    // Outputs sane and finite (docs/QA.md).
    assert!(signal.price.is_finite());
    assert!((0.0..=100.0).contains(&signal.rsi14));
    assert!(signal.strength <= signal.max_strength);

    println!(
        "OFFLINE_FLOW_OK action={:?} strength={}/{} rsi={:.2} ema={:.2} mom={:.2} price={:.2}",
        signal.action,
        signal.strength,
        signal.max_strength,
        signal.rsi14,
        signal.ema20,
        signal.momentum,
        signal.price
    );
}
