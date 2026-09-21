#![allow(dead_code)] // consumed phase-by-phase (README §60); clippy gate must stay green
#![allow(unused_imports)]

pub mod massive;
pub mod models;
pub mod provider;
pub mod replay;

pub use massive::MassiveProvider;
pub use models::{Candle, Quote, Symbol, Timeframe};
pub use provider::MarketDataProvider;
pub use replay::ReplayProvider;
