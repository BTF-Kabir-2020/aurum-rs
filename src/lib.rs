// Modules shared by the `aurum` binary, integration tests and future consumers.
// Phase-wise consumption tracked in HANDOFF.md (README §60).
// (no allow(dead_code): everything wired)

pub mod api;
pub mod cli;
pub mod config;
pub mod error;
pub mod indicators;
pub mod market;
pub mod signal;
pub mod state;
pub mod storage;
pub mod tui;

#[allow(unused_imports)]
pub use market::{Candle, MassiveProvider, Quote, ReplayProvider, Symbol, Timeframe};
#[allow(unused_imports)]
pub use signal::{Action, Signal, evaluate};
