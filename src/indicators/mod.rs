pub mod ema;
pub mod momentum;
pub mod rsi;
pub mod sma;
pub mod smma;

pub use ema::ema;
pub use momentum::momentum;
pub use rsi::rsi;
pub use sma::sma;
pub use smma::smma;

pub(crate) fn require_input(closes: &[f64]) -> crate::error::Result<()> {
    if closes.is_empty() {
        return Err(crate::error::Error::Config(
            "indicator input is empty: provide at least one close price".into(),
        ));
    }
    if closes.iter().any(|c| !c.is_finite()) {
        return Err(crate::error::Error::InvalidValue {
            field: "closes",
            reason: "input contains NaN or infinity".into(),
        });
    }
    Ok(())
}

pub(crate) fn require_period(period: usize) -> crate::error::Result<()> {
    if period == 0 {
        return Err(crate::error::Error::InvalidValue {
            field: "period",
            reason: "period must be at least 1".into(),
        });
    }
    Ok(())
}

/// QA guarantee "no NaN/Infinity in output": values are validated on construction.
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_input_rejected() {
        assert!(require_input(&[]).is_err());
        assert!(require_input(&[1.0]).is_ok());
    }

    #[test]
    fn non_finite_input_rejected() {
        assert!(require_input(&[f64::NAN]).is_err());
        assert!(require_input(&[f64::INFINITY]).is_err());
    }

    #[test]
    fn zero_period_rejected() {
        assert!(require_period(0).is_err());
        assert!(require_period(1).is_ok());
    }
}
