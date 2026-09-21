use crate::error::{Error, Result};

use super::{require_input, require_period};

/// Simple Moving Average: windowed mean.
/// Output has the same length as input; the first `period - 1` slots are None.
pub fn sma(values: &[f64], period: usize) -> Result<Vec<Option<f64>>> {
    require_input(values)?;
    require_period(period)?;
    if values.len() < period {
        return Err(Error::InvalidValue {
            field: "values",
            reason: format!(
                "need at least {period} values for SMA({period}), got {}",
                values.len()
            ),
        });
    }

    let mut out: Vec<Option<f64>> = vec![None; values.len()];
    let mut window: f64 = values[..period].iter().sum();
    out[period - 1] = Some(window / period as f64);
    for i in period..values.len() {
        window += values[i] - values[i - period];
        out[i] = Some(window / period as f64);
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
    fn known_small_case() {
        let s = sma(&[1.0, 2.0, 3.0, 4.0], 2).unwrap();
        assert_eq!(s[0], None);
        assert_close(s[1].unwrap(), 1.5);
        assert_close(s[2].unwrap(), 2.5);
        assert_close(s[3].unwrap(), 3.5);
    }

    #[test]
    fn windowed_mean_matches_math() {
        let s = sma(&[10.0, 20.0, 30.0, 40.0, 50.0], 3).unwrap();
        assert_eq!(s[0], None);
        assert_eq!(s[1], None);
        assert_close(s[2].unwrap(), 20.0);
        assert_close(s[3].unwrap(), 30.0);
        assert_close(s[4].unwrap(), 40.0);
    }

    #[test]
    fn flat_input_is_constant() {
        let s = sma(&[7.0; 5], 3).unwrap();
        assert!(s.iter().flatten().all(|v| *v == 7.0));
    }

    #[test]
    fn gates() {
        assert!(sma(&[], 3).is_err());
        assert!(sma(&[1.0], 3).is_err());
        assert!(sma(&[1.0, 2.0], 0).is_err());
        assert!(sma(&[1.0, f64::NAN], 2).is_err());
    }
}
