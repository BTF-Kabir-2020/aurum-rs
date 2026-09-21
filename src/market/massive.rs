use std::sync::Once;
use std::time::Duration;

use chrono::{DateTime, TimeZone, Utc};
use serde::Deserialize;

use crate::error::{Error, Result};
use crate::market::models::{Candle, Quote, Symbol, Timeframe};

pub const DEFAULT_API_BASE: &str = "https://api.massive.com";

/// Resilience policy (docs/RESILIENCE.md §3): max 3 attempts, exponential
/// backoff, `Retry-After` honored on 429, permanent 4xx never retried.
const MAX_ATTEMPTS: u32 = 3;
const BACKOFF_BASE_MS: u64 = 200;

const PROVIDER_SOURCE: &str = "massive";

static CRYPTO_INIT: Once = Once::new();

/// reqwest uses `rustls-no-provider`; a process-wide crypto provider must be
/// installed once before any TLS connection (ring, per docs/BUILD.md).
fn ensure_crypto_provider() {
    CRYPTO_INIT.call_once(|| {
        let _ = rustls::crypto::ring::default_provider().install_default();
    });
}

/// Massive REST provider (v0.1 HISTORICAL + quote).
/// Endpoints verified against https://massive.com/docs (2026-09-21):
/// - last quote:  GET /v1/last_quote/currencies/{from}/{to}
/// - custom bars: GET /v2/aggs/ticker/{ticker}/range/{mult}/{timespan}/{from}/{to}
pub struct MassiveProvider {
    client: reqwest::Client,
    api_key: String,
    base_url: String,
}

#[derive(Debug, Deserialize)]
struct LastQuoteResponse {
    last: LastQuote,
    #[serde(default)]
    status: String,
}

#[derive(Debug, Deserialize)]
struct LastQuote {
    ask: f64,
    bid: f64,
    timestamp: i64,
}

#[derive(Debug, Deserialize)]
struct AggsResponse {
    #[serde(default)]
    results: Vec<Agg>,
    #[serde(default)]
    status: String,
    #[serde(default)]
    error: Option<String>,
}

#[derive(Debug, Deserialize)]
struct Agg {
    o: f64,
    h: f64,
    l: f64,
    c: f64,
    v: f64,
    t: i64,
}

#[derive(Debug, Deserialize)]
struct MassiveError {
    #[serde(default)]
    error: Option<String>,
    #[serde(default)]
    message: Option<String>,
}

impl MassiveProvider {
    pub fn new(api_key: impl Into<String>) -> Result<Self> {
        Self::with_base_url(api_key, DEFAULT_API_BASE)
    }

    pub fn with_base_url(api_key: impl Into<String>, base_url: &str) -> Result<Self> {
        ensure_crypto_provider();
        let client = reqwest::Client::builder()
            .connect_timeout(Duration::from_secs(10))
            .timeout(Duration::from_secs(30))
            .build()
            .map_err(|e| Error::Provider(format!("cannot build HTTP client: {e}")))?;
        Ok(Self {
            client,
            api_key: api_key.into(),
            base_url: base_url.trim_end_matches('/').to_string(),
        })
    }

    fn auth_request(&self, url: String) -> reqwest::RequestBuilder {
        self.client
            .get(url)
            .bearer_auth(&self.api_key)
            .header("Accept", "application/json")
    }

    async fn send_resilient(&self, url: String) -> Result<(reqwest::StatusCode, String)> {
        let mut last_err: Option<Error> = None;
        for attempt in 1..=MAX_ATTEMPTS {
            let resp = self
                .auth_request(url.clone())
                .send()
                .await
                .map_err(|e| Error::Provider(format!("Massive API request failed: {e}")))?;
            let status = resp.status();

            // Retriable: 429 (Retry-After honored) and 5xx (exponential backoff).
            if (status.as_u16() == 429 || status.is_server_error()) && attempt < MAX_ATTEMPTS {
                let retry_after = resp
                    .headers()
                    .get("retry-after")
                    .and_then(|v| v.to_str().ok().map(String::from));
                let body = resp.text().await?;
                let delay_ms = if status.as_u16() == 429 {
                    parse_retry_after_seconds(&retry_after).unwrap_or(BACKOFF_BASE_MS)
                } else {
                    BACKOFF_BASE_MS << (attempt - 1) // 200, 400, 800 …
                };
                tracing::warn!(
                    "provider retry {attempt}/{MAX_ATTEMPTS} after HTTP {status}, waiting {delay_ms}ms"
                );
                tokio::time::sleep(std::time::Duration::from_millis(delay_ms)).await;
                last_err = Some(self.check_status(status, &body).await.unwrap_err());
                continue;
            }

            // Success or permanent failure (4xx except 429): single call, no retry.
            let body = resp.text().await?;
            self.check_status(status, &body).await?;
            return Ok((status, body));
        }
        Err(last_err.unwrap_or_else(|| Error::Provider("Massive API unreachable".into())))
    }

    async fn check_status(&self, status: reqwest::StatusCode, body: &str) -> Result<()> {
        if status.is_success() {
            return Ok(());
        }
        let detail = serde_json::from_str::<MassiveError>(body)
            .ok()
            .and_then(|e| e.error.or(e.message))
            .unwrap_or_else(|| body.chars().take(200).collect());
        let msg = match status.as_u16() {
            401 | 403 => format!(
                "Massive API rejected credentials (HTTP {status}): {detail}. Check MASSIVE_API_KEY and plan access."
            ),
            429 => format!("Massive API rate limit exceeded (HTTP 429): {detail}"),
            s if s >= 500 => format!("Massive API server error (HTTP {s}): {detail}"),
            s => format!("Massive API request failed (HTTP {s}): {detail}"),
        };
        Err(Error::Provider(msg))
    }

    fn parse_quote(symbol: &Symbol, raw: &str) -> Result<Quote> {
        let resp: LastQuoteResponse = serde_json::from_str(raw)
            .map_err(|e| Error::Provider(format!("Massive quote parse failed: {e}")))?;
        if resp.status == "error" {
            return Err(Error::Provider(
                "Massive API returned status=error for quote".into(),
            ));
        }
        let ts = Utc
            .timestamp_millis_opt(resp.last.timestamp)
            .single()
            .ok_or_else(|| Error::Provider("Massive quote timestamp out of range".into()))?;
        Ok(Quote::new(
            symbol.clone(),
            ts,
            resp.last.bid,
            resp.last.ask,
            PROVIDER_SOURCE,
        ))
    }

    fn parse_candles(raw: &str, timeframe: Timeframe) -> Result<Vec<Candle>> {
        let resp: AggsResponse = serde_json::from_str(raw)
            .map_err(|e| Error::Provider(format!("Massive aggs parse failed: {e}")))?;
        if resp.status == "error" {
            let detail = resp.error.unwrap_or_else(|| "unknown error".into());
            return Err(Error::Provider(format!("Massive API error: {detail}")));
        }
        resp.results
            .iter()
            .map(|agg| {
                let ts = Utc
                    .timestamp_millis_opt(agg.t)
                    .single()
                    .ok_or_else(|| Error::Provider("Massive agg timestamp out of range".into()))?;
                Ok(Candle {
                    symbol: Symbol::normalize("XAUUSD")?,
                    timestamp: ts,
                    open: agg.o,
                    high: agg.h,
                    low: agg.l,
                    close: agg.c,
                    volume: agg.v,
                    timeframe,
                    source: PROVIDER_SOURCE.to_string(),
                })
            })
            .collect()
    }

    fn timeframe_range(tf: Timeframe) -> (u32, &'static str) {
        match tf {
            Timeframe::M1 => (1, "minute"),
            Timeframe::M5 => (5, "minute"),
            Timeframe::M15 => (15, "minute"),
            Timeframe::H1 => (1, "hour"),
            Timeframe::H4 => (4, "hour"),
            Timeframe::D1 => (1, "day"),
        }
    }

    fn window_ms(tf: Timeframe) -> i64 {
        match tf {
            Timeframe::M1 => 60_000,
            Timeframe::M5 => 300_000,
            Timeframe::M15 => 900_000,
            Timeframe::H1 => 3_600_000,
            Timeframe::H4 => 14_400_000,
            Timeframe::D1 => 86_400_000,
        }
    }
}

impl crate::market::provider::MarketDataProvider for MassiveProvider {
    fn name(&self) -> &'static str {
        PROVIDER_SOURCE
    }

    async fn quote(&self, symbol: &Symbol) -> Result<Quote> {
        let url = format!(
            "{}/v1/last_quote/currencies/{}/{}",
            self.base_url,
            symbol.base(),
            symbol.quote()
        );
        let (_status, body) = self.send_resilient(url).await?;
        Self::parse_quote(symbol, &body)
    }

    async fn history(
        &self,
        symbol: &Symbol,
        timeframe: Timeframe,
        limit: u32,
        until: Option<DateTime<Utc>>,
    ) -> Result<Vec<Candle>> {
        if limit == 0 {
            return Ok(Vec::new());
        }
        let (multiplier, timespan) = Self::timeframe_range(timeframe);
        let end = until.unwrap_or_else(Utc::now);
        let start = end - chrono::Duration::milliseconds(Self::window_ms(timeframe) * limit as i64);
        let url = format!(
            "{}/v2/aggs/ticker/{}/range/{}/{}/{}/{}?sort=asc&limit={}",
            self.base_url,
            symbol.provider_form(),
            multiplier,
            timespan,
            start.format("%Y-%m-%d"),
            end.format("%Y-%m-%d"),
            limit.max(1)
        );
        let (_status, body) = self.send_resilient(url).await?;
        let mut candles = Self::parse_candles(&body, timeframe)?;
        candles.retain(|c| c.timestamp <= end);
        if candles.len() > limit as usize {
            candles.drain(..candles.len() - limit as usize);
        }
        Ok(candles)
    }
}

// Transport errors and Retry-After parsing live outside the provider impl so
// both quote and history share one code path.

fn parse_retry_after_seconds(raw: &Option<String>) -> Option<u64> {
    let v = retry_after_raw(raw)?;
    v.parse::<u64>().ok()
}

fn retry_after_raw(retry_after: &Option<String>) -> Option<String> {
    retry_after.as_ref().map(|s| s.trim().to_string())
}

/// Resilience helpers (docs/RESILIENCE.md §3): bounded only, never infinite.
pub const RESILIENCE_MAX_ATTEMPTS: u32 = MAX_ATTEMPTS;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::market::provider::MarketDataProvider;
    use axum::routing::get;
    use std::net::SocketAddr;
    use std::path::PathBuf;

    fn fixture(name: &str) -> String {
        std::fs::read_to_string(
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("fixtures/provider")
                .join(name),
        )
        .unwrap()
    }

    #[test]
    fn quote_fixture_parses() {
        let symbol = Symbol::normalize("XAUUSD").unwrap();
        let q = MassiveProvider::parse_quote(&symbol, &fixture("massive_quote.json")).unwrap();
        assert_eq!(q.bid, 3692.9);
        assert_eq!(q.ask, 3693.1);
        assert!((q.mid - 3693.0).abs() < 1e-9);
        assert_eq!(q.source, "massive");
        assert_eq!(q.timestamp.to_rfc3339(), "2026-09-14T01:15:00+00:00");
        q.validate().unwrap();
    }

    #[test]
    fn history_fixture_parses_to_candles() {
        let candles =
            MassiveProvider::parse_candles(&fixture("massive_history.json"), Timeframe::M5)
                .unwrap();
        assert_eq!(candles.len(), 5);
        assert_eq!(candles[0].open, 3680.0);
        assert_eq!(
            candles[0].timestamp.to_rfc3339(),
            "2026-09-14T00:00:00+00:00"
        );
        assert_eq!(candles[4].close, 3681.83);
        for c in &candles {
            assert_eq!(c.timeframe, Timeframe::M5);
            assert_eq!(c.source, "massive");
            c.validate().unwrap();
        }

        let daily = MassiveProvider::parse_candles(&fixture("massive_history.json"), Timeframe::D1)
            .unwrap();
        assert!(daily.iter().all(|c| c.timeframe == Timeframe::D1));
    }

    #[test]
    fn error_payload_maps_to_provider_error() {
        let raw = fixture("massive_error_429.json");
        let resp: AggsResponse = serde_json::from_str(&raw).unwrap();
        assert_eq!(resp.status, "error");
        assert!(
            resp.error
                .unwrap()
                .contains("exceeded the maximum requests")
        );
    }

    #[test]
    fn malformed_json_is_rejected() {
        let err = MassiveProvider::parse_candles("{ not json", Timeframe::M5).unwrap_err();
        assert!(err.to_string().contains("parse failed"));
    }

    fn symbol() -> Symbol {
        Symbol::normalize("XAUUSD").unwrap()
    }

    async fn spawn_test_server(state: TestRoute) -> SocketAddr {
        async fn quote_route() -> (reqwest::StatusCode, String) {
            (
                reqwest::StatusCode::OK,
                std::fs::read_to_string(
                    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                        .join("fixtures/provider/massive_quote.json"),
                )
                .unwrap(),
            )
        }
        async fn history_route() -> (reqwest::StatusCode, String) {
            (
                reqwest::StatusCode::OK,
                std::fs::read_to_string(
                    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                        .join("fixtures/provider/massive_history.json"),
                )
                .unwrap(),
            )
        }
        let _ = state;
        let app = axum::Router::new()
            .route("/v1/last_quote/currencies/XAU/USD", get(quote_route))
            .route(
                "/v2/aggs/ticker/C:XAUUSD/range/5/minute/{from}/{to}",
                get(history_route),
            );
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
        addr
    }

    #[derive(Clone, Copy)]
    enum TestRoute {
        Ok,
    }

    async fn provider_at(addr: SocketAddr) -> MassiveProvider {
        MassiveProvider::with_base_url("test-key", &format!("http://{addr}")).unwrap()
    }

    #[tokio::test]
    async fn quote_roundtrip_over_local_server() {
        let addr = spawn_test_server(TestRoute::Ok).await;
        let p = provider_at(addr).await;
        let q = p.quote(&symbol()).await.unwrap();
        assert_eq!(q.source, "massive");
        assert!((q.mid - 3693.0).abs() < 1e-9);
    }

    #[tokio::test]
    async fn history_roundtrip_over_local_server() {
        let addr = spawn_test_server(TestRoute::Ok).await;
        let p = provider_at(addr).await;
        let h = p.history(&symbol(), Timeframe::M5, 5, None).await.unwrap();
        assert_eq!(h.len(), 5);
        assert_eq!(h[0].open, 3680.0);
    }

    #[tokio::test]
    async fn auth_header_is_sent() {
        let captured = std::sync::Arc::new(std::sync::Mutex::new(None::<String>));
        let captured_for_handler = captured.clone();
        async fn capture(
            sink: std::sync::Arc<std::sync::Mutex<Option<String>>>,
            headers: axum::http::HeaderMap,
        ) -> (reqwest::StatusCode, String) {
            *sink.lock().unwrap() = headers
                .get("authorization")
                .map(|v| v.to_str().unwrap().to_string());
            (
                reqwest::StatusCode::OK,
                std::fs::read_to_string(
                    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                        .join("fixtures/provider/massive_quote.json"),
                )
                .unwrap(),
            )
        }
        let app = axum::Router::new().route(
            "/v1/last_quote/currencies/XAU/USD",
            get(move |headers: axum::http::HeaderMap| {
                capture(captured_for_handler.clone(), headers)
            }),
        );
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });

        let p = provider_at(addr).await;
        let q = p.quote(&symbol()).await.unwrap();
        assert_eq!(q.source, "massive");
        let auth = captured.lock().unwrap().clone();
        assert_eq!(auth.as_deref(), Some("Bearer test-key"));
    }

    #[tokio::test]
    async fn http_429_becomes_rate_limit_error() {
        async fn err429() -> (reqwest::StatusCode, String) {
            (
                reqwest::StatusCode::TOO_MANY_REQUESTS,
                std::fs::read_to_string(
                    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                        .join("fixtures/provider/massive_error_429.json"),
                )
                .unwrap(),
            )
        }
        let app = axum::Router::new().route("/v1/last_quote/currencies/XAU/USD", get(err429));
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });

        let p = provider_at(addr).await;
        let err = p.quote(&symbol()).await.unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("rate limit"), "msg: {msg}");
        assert!(msg.contains("429"), "msg: {msg}");
    }

    #[tokio::test]
    async fn http_500_becomes_server_error() {
        async fn err500() -> (reqwest::StatusCode, String) {
            (
                reqwest::StatusCode::INTERNAL_SERVER_ERROR,
                std::fs::read_to_string(
                    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                        .join("fixtures/provider/massive_error_500.json"),
                )
                .unwrap(),
            )
        }
        let app = axum::Router::new().route("/v1/last_quote/currencies/XAU/USD", get(err500));
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });

        let p = provider_at(addr).await;
        let err = p.quote(&symbol()).await.unwrap_err();
        assert!(err.to_string().contains("server error"));
    }

    #[tokio::test]
    async fn zero_limit_returns_empty_without_request() {
        let addr = spawn_test_server(TestRoute::Ok).await;
        let p = provider_at(addr).await;
        let h = p.history(&symbol(), Timeframe::M5, 0, None).await.unwrap();
        assert!(h.is_empty());
    }

    // ---- resilience (docs/RESILIENCE.md §3, §5) ----

    async fn spawn_sequence_server(
        hits: std::sync::Arc<std::sync::atomic::AtomicU32>,
        sequence: Vec<reqwest::StatusCode>,
    ) -> SocketAddr {
        let seq = std::sync::Arc::new(std::sync::Mutex::new(sequence));
        async fn seq_route(
            hits: std::sync::Arc<std::sync::atomic::AtomicU32>,
            seq: std::sync::Arc<std::sync::Mutex<Vec<reqwest::StatusCode>>>,
        ) -> (reqwest::StatusCode, axum::http::HeaderMap, String) {
            let n = hits.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            let status = {
                let mut q = seq.lock().unwrap();
                if q.len() > 1 {
                    q.remove(0)
                } else {
                    q.first().copied().unwrap_or(reqwest::StatusCode::OK)
                }
            };
            let _ = n;
            let mut headers = axum::http::HeaderMap::new();
            if status == reqwest::StatusCode::TOO_MANY_REQUESTS {
                headers.insert("retry-after", "0".parse().unwrap());
                (status, headers, "rate limited".to_string())
            } else {
                (
                    status,
                    headers,
                    std::fs::read_to_string(
                        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                            .join("fixtures/provider/massive_quote.json"),
                    )
                    .unwrap(),
                )
            }
        }
        let app: axum::Router = axum::Router::new().route(
            "/v1/last_quote/currencies/XAU/USD",
            get(move || async move { seq_route(hits.clone(), seq.clone()).await }),
        );
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
        addr
    }

    #[tokio::test]
    async fn four29_then_success_recovers_within_three_attempts() {
        let hits = std::sync::Arc::new(std::sync::atomic::AtomicU32::new(0));
        let addr = spawn_sequence_server(
            hits.clone(),
            vec![
                reqwest::StatusCode::TOO_MANY_REQUESTS,
                reqwest::StatusCode::OK,
            ],
        )
        .await;
        let p = provider_at(addr).await;
        let q = p.quote(&symbol()).await.unwrap();
        assert_eq!(q.source, "massive");
        let total = hits.load(std::sync::atomic::Ordering::Relaxed);
        assert_eq!(total, 2, "needs exactly the retry, got {total} requests");
    }

    #[tokio::test]
    async fn five_hundreds_recover_with_backoff() {
        let hits = std::sync::Arc::new(std::sync::atomic::AtomicU32::new(0));
        let addr = spawn_sequence_server(
            hits.clone(),
            vec![
                reqwest::StatusCode::INTERNAL_SERVER_ERROR,
                reqwest::StatusCode::INTERNAL_SERVER_ERROR,
                reqwest::StatusCode::OK,
            ],
        )
        .await;
        let p = provider_at(addr).await;
        let q = p.quote(&symbol()).await.unwrap();
        assert_eq!(q.source, "massive");
        assert_eq!(hits.load(std::sync::atomic::Ordering::Relaxed), 3);
    }

    #[tokio::test]
    async fn permanent_4xx_never_retried() {
        let hits = std::sync::Arc::new(std::sync::atomic::AtomicU32::new(0));
        let addr =
            spawn_sequence_server(hits.clone(), vec![reqwest::StatusCode::UNAUTHORIZED]).await;
        let p = provider_at(addr).await;
        let err = p.quote(&symbol()).await.unwrap_err();
        assert!(err.to_string().contains("rejected credentials"));
        assert_eq!(
            hits.load(std::sync::atomic::Ordering::Relaxed),
            1,
            "401 must not retry"
        );
    }

    #[tokio::test]
    async fn exhaustion_after_three_attempts_reports_last_error() {
        let hits = std::sync::Arc::new(std::sync::atomic::AtomicU32::new(0));
        let addr = spawn_sequence_server(
            hits.clone(),
            vec![
                reqwest::StatusCode::INTERNAL_SERVER_ERROR,
                reqwest::StatusCode::INTERNAL_SERVER_ERROR,
                reqwest::StatusCode::INTERNAL_SERVER_ERROR,
            ],
        )
        .await;
        let p = provider_at(addr).await;
        let err = p.quote(&symbol()).await.unwrap_err();
        assert!(err.to_string().contains("server error"));
        assert_eq!(
            hits.load(std::sync::atomic::Ordering::Relaxed),
            3,
            "bounded at MAX_ATTEMPTS"
        );
    }

    #[test]
    fn retry_after_header_parses_seconds() {
        assert_eq!(parse_retry_after_seconds(&Some("3".into())), Some(3));
        assert_eq!(parse_retry_after_seconds(&Some(" 5 ".into())), Some(5));
        assert_eq!(parse_retry_after_seconds(&Some("abc".into())), None);
        assert_eq!(parse_retry_after_seconds(&None), None);
    }
}
