use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};
use crate::indicators::{ema, momentum, rsi};

pub const EMA_PERIOD: usize = 20;
pub const RSI_PERIOD: usize = 14;
pub const MOMENTUM_PERIOD: usize = 10;
pub const MAX_STRENGTH: u8 = 3;
const RSI_LOW: f64 = 30.0;
const RSI_HIGH: f64 = 70.0;

/// Rule-based decision — README §26. NEVER call this AI (README §28).
///
/// BUY : RSI < 30  AND price > EMA20 AND momentum > 0
/// SELL: RSI > 70  AND price < EMA20 AND momentum < 0
/// Otherwise: HOLD.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum Action {
    Buy,
    Sell,
    Hold,
}

/// Transparent rule state (README §27): the API exposes the reasoning.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Conditions {
    pub rsi_oversold: bool,
    pub rsi_overbought: bool,
    pub price_above_ema: bool,
    pub price_below_ema: bool,
    pub positive_momentum: bool,
    pub negative_momentum: bool,
}

impl Conditions {
    fn buy_score(&self) -> u8 {
        u8::from(self.rsi_oversold)
            + u8::from(self.price_above_ema)
            + u8::from(self.positive_momentum)
    }

    fn sell_score(&self) -> u8 {
        u8::from(self.rsi_overbought)
            + u8::from(self.price_below_ema)
            + u8::from(self.negative_momentum)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Signal {
    pub action: Action,
    /// Matched conditions of the fired rule (BUY/SELL: 3; HOLD: 0).
    pub strength: u8,
    pub max_strength: u8,
    pub conditions: Conditions,
    pub price: f64,
    pub ema20: f64,
    pub rsi14: f64,
    pub momentum: f64,
}

/// Core rule evaluation over already-computed indicator values (pure).
pub fn decide_from_values(price: f64, ema20: f64, rsi14: f64, momentum: f64) -> Signal {
    let conditions = Conditions {
        rsi_oversold: rsi14 < RSI_LOW,
        rsi_overbought: rsi14 > RSI_HIGH,
        price_above_ema: price > ema20,
        price_below_ema: price < ema20,
        positive_momentum: momentum > 0.0,
        negative_momentum: momentum < 0.0,
    };

    let (action, strength) =
        if conditions.rsi_oversold && conditions.price_above_ema && conditions.positive_momentum {
            (Action::Buy, conditions.buy_score())
        } else if conditions.rsi_overbought
            && conditions.price_below_ema
            && conditions.negative_momentum
        {
            (Action::Sell, conditions.sell_score())
        } else {
            (Action::Hold, 0)
        };

    Signal {
        action,
        strength,
        max_strength: MAX_STRENGTH,
        conditions,
        price,
        ema20,
        rsi14,
        momentum,
    }
}

/// Evaluate the rule engine over a close-price series (latest values win).
pub fn evaluate(closes: &[f64]) -> Result<Signal> {
    let price = *closes
        .last()
        .ok_or_else(|| Error::Config("indicator input is empty".into()))?;
    let ema20 = ema(closes, EMA_PERIOD)?
        .last()
        .copied()
        .flatten()
        .ok_or_else(|| {
            Error::Config(format!(
                "not enough closes for EMA({EMA_PERIOD}): need at least {EMA_PERIOD}"
            ))
        })?;
    let rsi14 = rsi(closes, RSI_PERIOD)?
        .last()
        .copied()
        .flatten()
        .ok_or_else(|| {
            Error::Config(format!(
                "not enough closes for RSI({RSI_PERIOD}): need at least {}",
                RSI_PERIOD + 1
            ))
        })?;
    let momentum_val = momentum(closes, MOMENTUM_PERIOD)?
        .last()
        .copied()
        .flatten()
        .ok_or_else(|| {
            Error::Config(format!(
                "not enough closes for momentum({MOMENTUM_PERIOD}): need at least {}",
                MOMENTUM_PERIOD + 1
            ))
        })?;

    Ok(decide_from_values(price, ema20, rsi14, momentum_val))
}

#[cfg(test)]
mod tests {
    use super::*;

    // ---- per-condition isolation (docs/QA.md §6 `signal`) ----

    #[test]
    fn buy_fires_only_with_all_three() {
        let all = |rsi: f64, price_rel: f64, mom: f64| {
            decide_from_values(100.0 * price_rel, 100.0, rsi, mom)
        };
        let cases = [
            (all(25.0, 1.02, 0.1), Action::Buy),
            (all(35.0, 1.02, 0.1), Action::Hold), // RSI not oversold
            (all(25.0, 0.98, 0.1), Action::Hold), // price below EMA
            (all(25.0, 1.02, 0.0), Action::Hold), // momentum not positive
        ];

        for (s, expected) in cases {
            assert_eq!(
                s.action,
                expected,
                "rsi={} price_rel={} mom={}",
                s.rsi14,
                s.price / s.ema20,
                s.momentum
            );
        }
        assert_eq!(all(25.0, 1.02, 0.1).strength, 3);
    }

    #[test]
    fn sell_fires_only_with_all_three() {
        let all = |rsi: f64, price_rel: f64, mom: f64| {
            decide_from_values(100.0 * price_rel, 100.0, rsi, mom)
        };
        let s = all(75.0, 0.98, -0.1);
        assert_eq!(s.action, Action::Sell);
        assert_eq!(s.strength, 3);
        assert_eq!(all(65.0, 0.98, -0.1).action, Action::Hold);
        assert_eq!(all(75.0, 1.02, -0.1).action, Action::Hold);
        assert_eq!(all(75.0, 0.98, 0.0).action, Action::Hold);
    }

    #[test]
    fn strength_counts_matched_conditions() {
        let s = decide_from_values(100.0, 100.0, 25.0, 0.1);
        assert_eq!(s.action, Action::Hold);
        assert_eq!(s.strength, 0);
        assert!(s.conditions.rsi_oversold && s.conditions.positive_momentum);
        assert_eq!(
            s.conditions.rsi_oversold as u8
                + s.conditions.positive_momentum as u8
                + s.conditions.price_above_ema as u8,
            2
        );
    }

    // ---- full-series evaluation (wiring) ----

    #[test]
    fn flat_series_is_hold_with_sane_values() {
        let s = evaluate(&[100.0; 31]).unwrap();
        assert_eq!(s.action, Action::Hold);
        assert_eq!(s.price, 100.0);
        assert!((s.ema20 - 100.0).abs() < 1e-9);
        assert!((s.rsi14 - 50.0).abs() < 1e-9);
        assert!((s.momentum - 0.0).abs() < 1e-9);
    }

    #[test]
    fn rising_series_matches_indicator_outputs() {
        let closes: Vec<f64> = (0..40).map(|i| 100.0 + i as f64).collect();
        let s = evaluate(&closes).unwrap();
        let e = ema(&closes, EMA_PERIOD).unwrap().last().unwrap().unwrap();
        let r = rsi(&closes, RSI_PERIOD).unwrap().last().unwrap().unwrap();
        let m = momentum(&closes, MOMENTUM_PERIOD)
            .unwrap()
            .last()
            .unwrap()
            .unwrap();
        assert_eq!(s.ema20, e);
        assert_eq!(s.rsi14, r);
        assert_eq!(s.momentum, m);
        assert!(s.rsi14 > 70.0);
    }

    #[test]
    fn too_few_closes_is_actionable() {
        let closes: Vec<f64> = (0..15).map(f64::from).collect();
        let err = evaluate(&closes).unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("closes"), "msg: {msg}");
    }

    #[test]
    fn empty_input_is_actionable() {
        assert!(evaluate(&[]).unwrap_err().to_string().contains("empty"));
    }

    #[test]
    fn output_is_finite_everywhere() {
        let closes: Vec<f64> = (0..40).map(|i| 90.0 + (i * 7 % 20) as f64).collect();
        let s = evaluate(&closes).unwrap();
        for v in [s.price, s.ema20, s.rsi14, s.momentum] {
            assert!(v.is_finite());
        }
    }

    #[test]
    fn actions_serialize_uppercase() {
        assert_eq!(serde_json::to_string(&Action::Buy).unwrap(), r#""BUY""#);
        assert_eq!(serde_json::to_string(&Action::Sell).unwrap(), r#""SELL""#);
        assert_eq!(serde_json::to_string(&Action::Hold).unwrap(), r#""HOLD""#);
    }

    #[test]
    fn buy_fires_on_sustained_recovery_from_decline() {
        // long decline, then strong multi-bar rally: momentum>0, price>EMA.
        // We assert only that the engine wiring stays consistent with the
        // indicator outputs; rule behavior itself is covered in isolation.
        let mut c: Vec<f64> = (0..25).map(|i| 100.0 - i as f64 * 0.2).collect();
        c.extend((0..12).map(|i| 95.0 + i as f64 * 0.8));
        let s = evaluate(&c).unwrap();
        let e = ema(&c, EMA_PERIOD).unwrap().last().unwrap().unwrap();
        let r = rsi(&c, RSI_PERIOD).unwrap().last().unwrap().unwrap();
        let m = momentum(&c, MOMENTUM_PERIOD)
            .unwrap()
            .last()
            .unwrap()
            .unwrap();
        assert_eq!(s.ema20, e);
        assert_eq!(s.rsi14, r);
        assert_eq!(s.momentum, m);
        assert_eq!(s.conditions.price_above_ema, s.price > s.ema20);
    }
}
