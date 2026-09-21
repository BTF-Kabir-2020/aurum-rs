use crate::error::{Error, Result};

use super::{require_input, require_period};

/// Exponential Moving Average, Wilder seeding: first value is the SMA of the
/// first `period` closes (produced by [`super::sma`]), then k = 2/(period+1).
/// Output has the same length as input; the first `period - 1` slots are None.
pub fn ema(closes: &[f64], period: usize) -> Result<Vec<Option<f64>>> {
    require_input(closes)?;
    require_period(period)?;
    if closes.len() < period {
        return Err(Error::InvalidValue {
            field: "closes",
            reason: format!(
                "need at least {period} closes for EMA({period}), got {}",
                closes.len()
            ),
        });
    }

    let mut out = vec![None; closes.len()];
    let mut prev = super::sma(closes, period)?[period - 1].unwrap();
    let k = 2.0 / (period as f64 + 1.0);
    out[period - 1] = Some(prev);
    for (i, close) in closes.iter().enumerate().skip(period) {
        prev = (*close - prev) * k + prev;
        out[i] = Some(prev);
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_close(a: f64, b: f64) {
        assert!((a - b).abs() < 1e-9);
    }

    #[test]
    fn flat_input_converges_to_constant() {
        let e = ema(&[100.0; 25], 20).unwrap();
        let last = e.last().unwrap().unwrap();
        assert_close(last, 100.0);
        assert!(e[..19].iter().all(|v| v.is_none()));
        assert_eq!(e.len(), 25);
    }

    #[test]
    fn known_small_case() {
        let e = ema(&[1.0, 2.0, 3.0, 4.0], 2).unwrap();
        assert_eq!(e[0], None);
        assert_close(e[1].unwrap(), 1.5);
        assert_close(e[2].unwrap(), 2.5);
        assert_close(e[3].unwrap(), 3.5);
    }

    #[test]
    fn schemas_and_gates() {
        assert!(ema(&[], 20).is_err());
        assert!(ema(&[1.0, 2.0], 20).is_err());
        assert!(ema(&[1.0, 2.0], 0).is_err());
        assert!(ema(&[f64::NAN], 1).is_err());
    }

    #[test]
    fn no_nan_in_output() {
        let e = ema(&[10.0, 12.0, 11.0, 13.5, 12.8, 14.1], 3).unwrap();
        assert!(e.iter().flatten().all(|v| v.is_finite()));
    }
}
