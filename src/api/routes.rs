use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::{Json, routing::get};
use serde::Deserialize;

use crate::api::dto::{self, ApiError, api_status};
use crate::error::Result;
use crate::market::models::Symbol;
use crate::state::App;

pub type AppState = std::sync::Arc<App>;

/// Full router: unversioned health + versioned `/api/v1` (README §19-20).
pub fn router(state: AppState) -> axum::Router {
    axum::Router::new()
        .route("/health/live", get(health_live))
        .route("/health/ready", get(health_ready))
        .route("/api/v1/status", get(status))
        .route("/api/v1/quote/{symbol}", get(quote))
        .route("/api/v1/history/{symbol}", get(history))
        .route("/api/v1/signal/{symbol}", get(signal))
        .with_state(state)
}

async fn health_live() -> impl IntoResponse {
    Json(serde_json::json!({ "status": "ok", "version": dto::API_VERSION }))
}

async fn health_ready() -> impl IntoResponse {
    Json(serde_json::json!({ "status": "ok", "version": dto::API_VERSION }))
}

async fn status(State(state): State<AppState>) -> (StatusCode, Json<dto::StatusDto>) {
    let mode = state.config.mode.as_str().to_string();
    let provider = if state.is_demo().await {
        "replay"
    } else {
        "massive"
    };
    (
        StatusCode::OK,
        Json(dto::StatusDto {
            version: dto::API_VERSION,
            status: "ok",
            mode: mode.to_string(),
            provider: provider.to_string(),
            symbol: state.config.symbol.clone(),
        }),
    )
}

async fn quote(State(state): State<AppState>, Path(symbol_str): Path<String>) -> Response {
    let symbol = match symbol_param(&symbol_str) {
        Ok(s) => s,
        Err(e) => return error_response(&e),
    };
    match state.quote_with_stale(&symbol).await {
        Ok((q, age)) => {
            let mode = state.config.mode.as_str().to_string();
            // Docs/RESILIENCE.md: stale cache is served with 200 + stale tags.
            let dto = if age == 0 {
                dto::quote_dto(&mode, &q)
            } else {
                dto::stale_quote_dto(&mode, &q, age / 1000)
            };
            (StatusCode::OK, Json(dto)).into_response()
        }
        Err(e) => error_response(&e),
    }
}

async fn history(
    State(state): State<AppState>,
    Path(symbol_str): Path<String>,
    Query(q): Query<HistoryQuery>,
) -> Response {
    let limit = q.limit.unwrap_or(50).min(500);
    let symbol = match symbol_param(&symbol_str) {
        Ok(s) => s,
        Err(e) => return error_response(&e),
    };
    match state.history(&symbol, limit).await {
        Ok(candles) => {
            let mode = state.config.mode.as_str().to_string();
            let tf = state.config.timeframe.clone();
            (StatusCode::OK, Json(dto::history_dto(&mode, &tf, &candles))).into_response()
        }
        Err(e) => error_response(&e),
    }
}

async fn signal(State(state): State<AppState>, Path(symbol_str): Path<String>) -> Response {
    let symbol = match symbol_param(&symbol_str) {
        Ok(s) => s,
        Err(e) => return error_response(&e),
    };
    match state.compute_signal(&symbol).await {
        Ok(sig) => {
            let mode = state.config.mode.as_str().to_string();
            let source = if state.is_demo().await {
                "fixture"
            } else {
                "massive"
            };
            let _ = state
                .persist_signals(std::slice::from_ref(&sig), &symbol)
                .await;
            (
                StatusCode::OK,
                Json(dto::signal_dto(&mode, source, &symbol, &sig)),
            )
                .into_response()
        }
        Err(e) => error_response(&e),
    }
}

fn symbol_param(input: &str) -> Result<Symbol> {
    Symbol::normalize(input)
}

#[derive(Default, Deserialize)]
pub struct HistoryQuery {
    limit: Option<u32>,
}

fn error_response(e: &crate::error::Error) -> Response {
    let (code, msg) = api_status(e);
    (
        code,
        Json(ApiError {
            version: dto::API_VERSION,
            error: msg,
        }),
    )
        .into_response()
}
