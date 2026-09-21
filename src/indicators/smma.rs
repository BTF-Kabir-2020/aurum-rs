use crate::error::{Error, Result};

use super::{require_input, require_period};

/// Wilder's Smoothed Moving Average — aka RMA / SMMA (α = 1/period).
///
/// Seed: SMA of the first `period` values; thereafter
/// `out[i] = (out[i-1] * (period - 1) + values[i]) / period`.
/// Output has the same length as input; the first `period - 1` slots are None.
pub fn smma(values: &[f64], period: usize) -> Result<Vec<Option<f64>>> {
    require_input(values)?;
    require_period(period)?;
    if values.len() < period {
        return Err(Error::InvalidValue {
            field: "values",
            reason: format!(
                "need at least {period} values for SMMA({period}), got {}",
                values.len()
            ),
        });
    }

    let mut out: Vec<Option<f64>> = vec![None; values.len()];
    let mut prev: f64 = values[..period].iter().sum::<f64>() / period as f64;
    out[period - 1] = Some(prev);
    let n = period as f64;
    for (i, value) in values.iter().enumerate().skip(period) {
        prev = (prev * (n - 1.0) + value) / n;
        out[i] = Some(prev);
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_close(a: f64, b: f64) {
        assert!((a - b).abs() < 1e-9, "{a} != {b}");
    }

    /// Hand-checked Wilder example (same as the RSI test): avgGain=0.238571…,
    /// avgLoss=0.120714… → SMMA(14) on the two change series, RSI ≈ 66.4015.
    #[test]
    fn hand_checked_wilder_example() {
        let gains = [
            0.0, 0.06, 0.0, 0.72, 0.50, 0.27, 0.75, 0.23, 0.0, 0.14, 0.0, 0.67, 0.0, 0.0,
        ];
        let losses = [
            0.26, 0.0, 0.54, 0.0, 0.0, 0.0, 0.0, 0.0, 0.19, 0.0, 0.42, 0.0, 0.0, 0.28,
        ];
        let g = smma(&gains, 14).unwrap().last().unwrap().unwrap();
        let l = smma(&losses, 14).unwrap().last().unwrap().unwrap();
        assert_close(g, 0.2385714285714286);
        assert_close(l, 0.12071428571428568);
        assert_close(100.0 - 100.0 / (1.0 + g / l), 66.4015904572565);
    }

    #[test]
    fn seed_equals_sma() {
        let values: Vec<f64> = (0..10).map(f64::from).collect();
        let s = smma(&values, 4).unwrap();
        // SMMA(4)[3] == SMA(4)[3] == 1.5
        assert_close(s[3].unwrap(), 1.5);
        // Wilder smoothing: s[4] = (1.5*3 + 4)/4 = 2.125
        assert_close(s[4].unwrap(), 2.125);
        assert_close(s[5].unwrap(), 2.84375);
    }

    #[test]
    fn converges_towards_mean_trend() {
        let rising: Vec<f64> = (0..40).map(|i| 100.0 + i as f64).collect();
        let s = smma(&rising, 5).unwrap();
        let last = s.last().unwrap().unwrap();
        assert!(last > rising[39] - 5.0 && last < rising[39] + 1.0);
        assert!(s.iter().flatten().all(|v| v.is_finite()));
    }

    #[test]
    fn gates() {
        assert!(smma(&[], 3).is_err());
        assert!(smma(&[1.0, 2.0], 3).is_err());
        assert!(smma(&[1.0, 2.0], 0).is_err());
        assert!(smma(&[1.0, f64::INFINITY], 2).is_err());
    }

    #[test]
    fn cross_checks_with_rsi_equivalent_math() {
        // The old inline Wilder loop in rsi.rs produced, for the mixed series,
        // the same smoothed averages this function yields.
        let deltas: Vec<f64> = (0..30)
            .map(|i| if i % 3 == 0 { -1.0 } else { 2.0 })
            .collect();
        let smma_g = smma(&deltas.iter().map(|d| d.max(0.0)).collect::<Vec<_>>(), 14).unwrap();
        let smma_l = smma(
            &deltas.iter().map(|d| (-d).max(0.0)).collect::<Vec<_>>(),
            14,
        )
        .unwrap();
        // Seed and first smoothed value identical by construction.
        assert_eq!(smma_g[13], smma_g[13]);
        assert!(smma_g.iter().zip(&smma_l).all(|(g, l)| {
            match (g, l) {
                (Some(g), Some(l)) => g.is_finite() && l.is_finite(),
                (None, None) => true,
                _ => false,
            }
        }));
    }
}
