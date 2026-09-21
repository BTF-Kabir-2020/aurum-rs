use crate::error::{Error, Result};

use super::{require_input, require_period};

/// RSI (Wilder). Wilder's smoothing is delegated to [`super::smma`] on the
/// gain/loss series; the same hand-checked example lives in `smma.rs`.
/// Conventions (documented): avg_loss == 0 && avg_gain == 0 (flat data) → 50;
/// avg_loss == 0 && avg_gain > 0 → 100. Never NaN/Inf.
pub fn rsi(closes: &[f64], period: usize) -> Result<Vec<Option<f64>>> {
    require_input(closes)?;
    require_period(period)?;
    if closes.len() < period + 1 {
        return Err(Error::InvalidValue {
            field: "closes",
            reason: format!(
                "need at least {} closes for RSI({}), got {}",
                period + 1,
                period,
                closes.len()
            ),
        });
    }

    let gains: Vec<f64> = closes[..]
        .windows(2)
        .map(|pair| (pair[1] - pair[0]).max(0.0))
        .collect();
    let losses: Vec<f64> = closes[..]
        .windows(2)
        .map(|pair| (pair[0] - pair[1]).max(0.0))
        .collect();

    let mut out = vec![None; closes.len()];
    for (changes_idx, (g, l)) in super::smma(&gains, period)?
        .into_iter()
        .zip(super::smma(&losses, period)?)
        .enumerate()
    {
        if let (Some(g), Some(l)) = (g, l) {
            // smma index i on changes maps to closes index i + 1
            out[changes_idx + 1] = Some(scale(g, l));
        }
    }
    Ok(out)
}

fn scale(avg_gain: f64, avg_loss: f64) -> f64 {
    if avg_loss == 0.0 {
        if avg_gain == 0.0 {
            return 50.0;
        }
        return 100.0;
    }
    let rs = avg_gain / avg_loss;
    100.0 - (100.0 / (1.0 + rs))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rising(n: usize) -> Vec<f64> {
        (0..n).map(|i| i as f64).collect()
    }

    #[test]
    fn pure_rising_reaches_100() {
        let r = rsi(&rising(30), 14).unwrap();
        assert!(r[..14].iter().all(|v| v.is_none()));
        assert_close(r[14].unwrap(), 100.0);
        assert_eq!(r.len(), 30);
    }

    #[test]
    fn flat_input_is_50_by_convention() {
        let r = rsi(&[100.0; 20], 14).unwrap();
        assert_close(r[14].unwrap(), 50.0);
        assert!(r.iter().flatten().all(|v| v.is_finite()));
    }

    #[test]
    fn pure_falling_reaches_0() {
        let data: Vec<f64> = (0..30).rev().map(|i| i as f64).collect();
        let r = rsi(&data, 14).unwrap();
        assert_close(r[14].unwrap(), 0.0);
    }

    #[test]
    fn known_mixed_pattern() {
        // Hand-computed Wilder example: avgGain=0.238571, avgLoss=0.120714 → RSI≈66.40
        let data = [
            44.35, 44.09, 44.15, 43.61, 44.33, 44.83, 45.10, 45.85, 46.08, 45.89, 46.03, 45.61,
            46.28, 46.28, 46.00,
        ];
        let r = rsi(&data, 14).unwrap();
        let v = r[14].unwrap();
        assert!(
            (v - 66.4015904572565).abs() < 1e-6,
            "expected ≈66.40, got {v}"
        );
    }

    #[test]
    fn gates() {
        assert!(rsi(&[], 14).is_err());
        assert!(rsi(&[1.0; 15], 0).is_err());
        assert!(rsi(&[1.0; 14], 14).is_err());
        assert!(rsi(&[f64::INFINITY; 16], 14).is_err());
    }

    fn assert_close(a: f64, b: f64) {
        assert!((a - b).abs() < 1e-9, "{a} != {b}");
    }
}
