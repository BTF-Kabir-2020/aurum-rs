//! Phase 11 smoke: full HTTP API path over an in-process axum server.
//! Offline only (DEMO / replay fixture, temp DB). No network, no credentials.

use std::net::SocketAddr;

#[tokio::test]
async fn api_endpoints_smoke() {
    // reqwest uses rustls-no-provider; install ring (same as MassiveProvider).
    let _ = rustls::crypto::ring::default_provider().install_default();

    let dir = std::env::temp_dir().join(format!("aurum-api-test-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let mut config = aurum::config::Config::load_with_env(|_| None).unwrap();
    config.mode = aurum::config::Mode::Demo;
    config.db_path = dir.join("api.db");

    let app: std::sync::Arc<aurum::state::App> =
        std::sync::Arc::new(aurum::state::App::from_config(config).await.unwrap());
    let router = aurum::api::routes::router(app.clone());

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr: SocketAddr = listener.local_addr().unwrap();
    tokio::spawn(async move { axum::serve(listener, router).await.unwrap() });

    let base = format!("http://{addr}");
    let client = reqwest::Client::new();

    // health (unversioned) — include version (README §19)
    let r = client
        .get(format!("{base}/health/live"))
        .send()
        .await
        .unwrap();
    assert_eq!(r.status(), 200);
    let j: serde_json::Value = r.json().await.unwrap();
    assert_eq!(j["status"], "ok");

    // status (versioned)
    let r = client
        .get(format!("{base}/api/v1/status"))
        .send()
        .await
        .unwrap();
    assert_eq!(r.status(), 200);
    let j: serde_json::Value = r.json().await.unwrap();
    assert_eq!(j["mode"], "demo");
    assert_eq!(j["provider"], "replay");
    assert_eq!(j["status"], "ok");
    assert!(j["version"].as_str().is_some());

    // quote — demo replay provider, display form (README §51)
    let r = client
        .get(format!("{base}/api/v1/quote/XAUUSD"))
        .send()
        .await
        .unwrap();
    assert_eq!(r.status(), 200);
    let j: serde_json::Value = r.json().await.unwrap();
    assert_eq!(j["symbol"], "XAU/USD");
    assert_eq!(j["canonical_symbol"], "XAUUSD");
    assert_eq!(j["source"], "fixture");
    assert!(j["bid"].as_f64().unwrap() > 0.0);

    // quote with alias normalization
    let r = client
        .get(format!("{base}/api/v1/quote/xau%2Fusd"))
        .send()
        .await
        .unwrap();
    assert_eq!(r.status(), 200);

    // history with limit
    let r = client
        .get(format!("{base}/api/v1/history/XAUUSD?limit=10"))
        .send()
        .await
        .unwrap();
    assert_eq!(r.status(), 200);
    let j: serde_json::Value = r.json().await.unwrap();
    assert_eq!(j["count"], 10);
    assert_eq!(j["timeframe"], "5m");
    assert!(j["candles"].as_array().unwrap().len() == 10);

    // signal — README §50 shape
    let r = client
        .get(format!("{base}/api/v1/signal/XAUUSD"))
        .send()
        .await
        .unwrap();
    assert_eq!(r.status(), 200);
    let j: serde_json::Value = r.json().await.unwrap();
    assert_eq!(j["mode"], "demo");
    assert!(j["indicators"]["ema20"].as_f64().unwrap().is_finite());
    assert!(
        j["indicators"]["rsi14"].as_f64().unwrap() >= 0.0
            && j["indicators"]["rsi14"].as_f64().unwrap() <= 100.0
    );
    assert!(j["signal"]["max_strength"].as_u64().unwrap() == 3);
    assert!(j["version"].is_string());
    assert_eq!(j["data"]["source"], "fixture");

    // invalid symbol → 400 with actionable error (README §21)
    let r = client
        .get(format!("{base}/api/v1/quote/GOLD_BAR"))
        .send()
        .await
        .unwrap();
    assert_eq!(r.status(), 400);
    let j: serde_json::Value = r.json().await.unwrap();
    assert!(j["error"].as_str().unwrap().contains("not a valid symbol"));

    // cleanup
    app.close_db().await;
}
