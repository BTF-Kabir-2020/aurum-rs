use serde_json::json;

use crate::config::{Config, Mode};
use crate::error::Result;
use crate::market::models::Symbol;
use crate::state::App;

fn print_quote(q: &crate::market::Quote) {
    println!(
        "{}  bid={:.5} ask={:.5} mid={:.5}  ts={} (source={})",
        q.symbol.display(),
        q.bid,
        q.ask,
        q.mid,
        q.timestamp.to_rfc3339(),
        q.source
    );
}

fn print_signal(
    s: &crate::signal::Signal,
    symbol: &crate::market::Symbol,
    mode: &str,
    source: &str,
) {
    let payload = json!({
        "symbol": symbol.display(),
        "mode": mode,
        "timestamp": chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
        "price": s.price,
        "indicators": {
            "ema20": s.ema20,
            "rsi14": s.rsi14,
            "momentum": s.momentum
        },
        "signal": {
            "action": s.action,
            "strength": s.strength,
            "max_strength": s.max_strength
        },
        "conditions": s.conditions,
        "data": {
            "source": source,
            "stale": false
        }
    });
    println!("{}", serde_json::to_string_pretty(&payload).unwrap());
}

/// `aurum quote <symbol>` — provider fetch + persistence (HISTORICAL/DEMO).
pub async fn run_quote(config: Config, symbol_str: &str, offline: bool) -> Result<()> {
    let symbol = Symbol::normalize(symbol_str)?;
    let app = App::from_config(force_demo(config, offline)).await?;
    let q = app.quote(&symbol).await?;
    print_quote(&q);
    app.close_db().await;
    Ok(())
}

/// `aurum history <symbol>` — fetch → persist → print summary.
pub async fn run_history(
    config: Config,
    symbol_str: &str,
    limit: u32,
    offline: bool,
) -> Result<()> {
    let symbol = Symbol::normalize(symbol_str)?;
    let app = App::from_config(force_demo(config, offline)).await?;
    let candles = app.history(&symbol, limit).await?;
    println!(
        "{}  bars={} timeframe={} first={}",
        symbol.display(),
        candles.len(),
        app.config.timeframe,
        candles
            .first()
            .map(|c| c.timestamp.to_rfc3339())
            .unwrap_or_else(|| "-".into())
    );
    if let Some(last) = candles.last() {
        println!(
            "last: o={} h={} l={} c={} v={} ts={}",
            last.open,
            last.high,
            last.low,
            last.close,
            last.volume,
            last.timestamp.to_rfc3339()
        );
    }
    app.close_db().await;
    Ok(())
}

/// `aurum signal <symbol>` — history → indicators → rules → persist → JSON.
pub async fn run_signal(config: Config, symbol_str: &str, offline: bool) -> Result<()> {
    let symbol = Symbol::normalize(symbol_str)?;
    let config = force_demo(config, offline);
    let app = App::from_config(config).await?;
    let mode = app.config.mode.as_str().to_string();
    let signal = app.compute_signal(&symbol).await?;
    app.persist_signals(std::slice::from_ref(&signal), &symbol)
        .await?;
    let source = if app.is_demo().await {
        "fixture"
    } else {
        "massive"
    };
    print_signal(&signal, &symbol, &mode, source);
    app.close_db().await;
    Ok(())
}

/// DEMO mode never needs credentials nor network.
fn force_demo(config: Config, offline: bool) -> Config {
    if !offline {
        return config;
    }
    let mut c = config;
    c.mode = Mode::Demo;
    c
}

/// `aurum demo --speed N` (README §30): small deterministic replay over the
/// bundled fixture — computes the rule engine on growing windows, persists
/// only the final signal. speed 1..=10 scales the inter-frame delay.
pub async fn run_demo(config: Config, speed: u8) -> Result<()> {
    let speed = speed.clamp(1, 10);
    let symbol = Symbol::normalize(&config.symbol.clone())?;
    let app = App::from_config(force_demo(config, true)).await?;
    let candles = app.history(&symbol, 240).await?;
    if candles.len() < 35 {
        return Err(crate::error::Error::Config(
            "fixture too small for a demo replay".into(),
        ));
    }

    let edges = [20usize, 24, 28, 32, 36, 42, 48];
    let delay = std::time::Duration::from_millis(800 / u64::from(speed.max(2)));
    for window in edges {
        let closes: Vec<f64> = candles
            .iter()
            .rev()
            .take(window.max(1))
            .map(|c| c.close)
            .collect();
        let sig = crate::signal::evaluate(&closes)?;
        println!(
            "[demo replay {window:3} bars] price={:>9.2}  ema20={:>9.2}  rsi14={:>6.1}  mom={:>6.2}%  => {:>10} ({}/{} matched)",
            sig.price,
            sig.ema20,
            sig.rsi14,
            sig.momentum,
            string_action(sig.action),
            sig.strength,
            sig.max_strength
        );
        tokio::time::sleep(delay).await; // deterministic playback speed
    }

    let final_sig = crate::signal::evaluate(&candles.iter().map(|c| c.close).collect::<Vec<_>>())?;
    app.persist_signals(std::slice::from_ref(&final_sig), &symbol)
        .await?;
    println!(
        "FINAL — action={:?} strength={}/{} | source=fixture (offline), persisted to {}",
        final_sig.action,
        final_sig.strength,
        final_sig.max_strength,
        app.config.db_path.display()
    );
    app.close_db().await;
    Ok(())
}

fn string_action(a: crate::signal::Action) -> &'static str {
    match a {
        crate::signal::Action::Buy => "BUY",
        crate::signal::Action::Sell => "SELL",
        crate::signal::Action::Hold => "HOLD",
    }
}
