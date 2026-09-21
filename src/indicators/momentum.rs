use crate::error::{Error, Result};

use super::{require_input, require_period};

/// Momentum as signed percent change over `period` closes:
/// (close[i] - close[i - period]) / close[i - period] * 100.
/// A None slot is emitted when the base close is 0 (invalid price data);
/// no NaN/Infinity ever leaves this module.
pub fn momentum(closes: &[f64], period: usize) -> Result<Vec<Option<f64>>> {
    require_input(closes)?;
    require_period(period)?;
    if closes.len() < period + 1 {
        return Err(Error::InvalidValue {
            field: "closes",
            reason: format!(
                "need at least {} closes for momentum({}), got {}",
                period + 1,
                period,
                closes.len()
            ),
        });
    }

    let mut out = vec![None; closes.len()];
    for i in period..closes.len() {
        let base = closes[i - period];
        out[i] = if base == 0.0 {
            None
        } else {
            Some((closes[i] - base) / base * 100.0)
        };
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_close(a: f64, b: f64) {
        assert!((a - b).abs() < 1e-9, "{a} != {b}");
    }

    #[test]
    fn percent_change_matches_math() {
        let m = momentum(&[100.0, 101.0, 90.0], 1).unwrap();
        assert_eq!(m[0], None);
        assert_close(m[1].unwrap(), 1.0);
        assert_close(m[2].unwrap(), -10.891089108910892);
    }

    #[test]
    fn zero_base_close_is_none_not_nan() {
        let m = momentum(&[0.0, 100.0, 110.0], 1).unwrap();
        assert_eq!(m[1], None);
        assert!(m.iter().flatten().all(|v| v.is_finite()));
    }

    #[test]
    fn flat_input_is_zero() {
        let m = momentum(&[50.0; 6], 5).unwrap();
        assert_close(m[5].unwrap(), 0.0);
    }

    #[test]
    fn gates() {
        assert!(momentum(&[], 10).is_err());
        assert!(momentum(&[1.0, 2.0], 10).is_err());
        assert!(momentum(&[1.0, 2.0], 0).is_err());
        assert!(momentum(&[f64::NAN, 1.0], 1).is_err());
    }
}
