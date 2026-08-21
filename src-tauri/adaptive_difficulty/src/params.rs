//! Tunable parameters for the adaptive difficulty algorithm.
//!
//! All values default to the spec-provided constants. Inject custom `Params`
//! from tests or the GUI verifier to observe how the algorithm reacts.

use serde::{Deserialize, Serialize};

/// Knobs for [`crate::update`].
///
/// The default values are the production values from the project spec;
/// override individual fields in tests to exercise edge cases (e.g. set
/// `score_floor = 0.8` to verify the step-4 divide-by-zero guard).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Params {
    // ----------------------------------------------------------------------
    // Weighted score rate
    // ----------------------------------------------------------------------
    /// Weight of the 1–14 MCQ score rate in the combined score.
    pub weight_a: f64,
    /// Weight of the 15–18 fill-in-the-blank score rate in the combined score.
    pub weight_b: f64,
    /// Weight of the Q19 retell score rate in the combined score.
    pub weight_c: f64,

    // ----------------------------------------------------------------------
    // Score rate clipping
    // ----------------------------------------------------------------------
    /// Lower bound applied to `combined` before further calculation
    /// (step 2 of the algorithm). Spec value: `0.6`. Lower means more
    /// tolerance for poor performance; higher means harsher feedback.
    pub score_floor: f64,

    // ----------------------------------------------------------------------
    // EMA trend
    // ----------------------------------------------------------------------
    /// Smoothing coefficient for the `trend` EMA (step 5). Must be in `(0, 1)`.
    /// Spec value: `0.4`. Larger means recent results weigh more heavily.
    pub alpha: f64,

    // ----------------------------------------------------------------------
    // Magnitude / delta
    // ----------------------------------------------------------------------
    /// Minimum adjustment magnitude per update (step 6 base).
    /// Spec value: `3.0`.
    pub base_magnitude: f64,
    /// Multiplier converting `|trend|` into additional magnitude (step 6).
    /// Spec value: `10.0`.
    pub k: f64,
    /// Upper bound on magnitude per update (step 6 cap).
    /// Spec value: `8.0`.
    pub max_magnitude: f64,

    // ----------------------------------------------------------------------
    // Hysteresis thresholds
    // ----------------------------------------------------------------------
    /// Ability threshold separating junior_high and senior_high.
    /// Spec value: `200.0`.
    pub b1: f64,
    /// Ability threshold separating senior_high and undergraduate.
    /// Spec value: `400.0`.
    pub b2: f64,
    /// Width of the buffer zone around each threshold that prevents
    /// oscillation (step 9). Spec value: `20.0`.
    pub buffer: f64,

    // ----------------------------------------------------------------------
    // Output clamps (step 8)
    // ----------------------------------------------------------------------
    /// Minimum allowed `ability_score`. Spec value: `0.0`.
    pub ability_min: f64,
    /// Maximum allowed `ability_score`. Spec value: `600.0`.
    pub ability_max: f64,

    // ----------------------------------------------------------------------
    // Trend clamp (step 5 defensive guard)
    pub trend_min: f64,
    /// Upper bound on `trend`. Spec value: `1.0`.
    pub trend_max: f64,
}

impl Default for Params {
    /// Production values from the project spec.
    fn default() -> Self {
        Self {
            weight_a: 0.5,
            weight_b: 0.2,
            weight_c: 0.3,
            score_floor: 0.6,
            alpha: 0.4,
            base_magnitude: 3.0,
            k: 10.0,
            max_magnitude: 8.0,
            b1: 200.0,
            b2: 400.0,
            buffer: 20.0,
            ability_min: 0.0,
            ability_max: 600.0,
            trend_min: -1.0,
            trend_max: 1.0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_matches_spec() {
        let p = Params::default();
        assert_eq!(p.weight_a, 0.5);
        assert_eq!(p.weight_b, 0.2);
        assert_eq!(p.weight_c, 0.3);
        assert_eq!(p.score_floor, 0.6);
        assert_eq!(p.alpha, 0.4);
        assert_eq!(p.base_magnitude, 3.0);
        assert_eq!(p.k, 10.0);
        assert_eq!(p.max_magnitude, 8.0);
        assert_eq!(p.b1, 200.0);
        assert_eq!(p.b2, 400.0);
        assert_eq!(p.buffer, 20.0);
        assert_eq!(p.ability_min, 0.0);
        assert_eq!(p.ability_max, 600.0);
        assert_eq!(p.trend_min, -1.0);
        assert_eq!(p.trend_max, 1.0);
    }

    #[test]
    fn params_round_trip_json() {
        let p = Params::default();
        let s = serde_json::to_string(&p).unwrap();
        let back: Params = serde_json::from_str(&s).unwrap();
        assert_eq!(p, back);
    }
}