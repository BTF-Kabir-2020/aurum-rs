use crate::error::Result;
use crate::market::models::{Candle, Quote, Symbol, Timeframe};

/// Provider contract (docs/ARCHITECTURE.md). Nothing outside this layer
/// talks to Massive; every source implements this trait.
///
/// Native async-in-trait (Rust 1.98); dispatch happens via generics/enum,
/// no extra `async-trait` dependency. Auto-trait bounds are irrelevant here:
/// providers never go behind `dyn` (enum dispatch in state layer).
#[allow(async_fn_in_trait)]
pub trait MarketDataProvider: Send + Sync {
    /// Short identifier: "massive", "replay", ...
    fn name(&self) -> &'static str;

    /// Latest top-of-book quote for the symbol.
    async fn quote(&self, symbol: &Symbol) -> Result<Quote>;

    /// Historical candles ending at or before `until` (None = now).
    async fn history(
        &self,
        symbol: &Symbol,
        timeframe: Timeframe,
        limit: u32,
        until: Option<chrono::DateTime<chrono::Utc>>,
    ) -> Result<Vec<Candle>>;
}
