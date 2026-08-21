//! Persistent adaptive-difficulty state.

use serde::{Deserialize, Serialize};

use crate::level::Level;

/// All fields the caller must persist between tests.
///
/// `current_level` is **independent state** — it is **not** derived from
/// `ability_score`. The hysteresis rules require remembering the previous
/// level to decide whether the new ability warrants a switch.
///
/// `update_count` is included so the GUI history list can number entries
/// stably across reloads. It is `#[serde(default)]` so old JSON files
/// without this field still deserialize.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AdaptiveState {
    /// User ability score. Bounded to `[ability_min, ability_max]` (default
    /// `[0.0, 600.0]`) by [`crate::update`].
    pub ability_score: f64,
    /// EMA of recent performance, normalized to roughly `[-1, 1]`. Used by
    /// step 6 of the algorithm to amplify the per-update delta when recent
    /// results trend consistently good or bad.
    pub trend: f64,
    /// Current effective difficulty level. Independent of `ability_score` —
    /// see module docs.
    pub current_level: Level,
    /// Monotonic counter incremented by [`crate::update`] (but **not** by
    /// [`crate::reset_to`]). `#[serde(default)]` keeps old payloads
    /// backwards-compatible.
    #[serde(default)]
    pub update_count: u64,
}

impl AdaptiveState {
    /// Construct a fresh state at the given level.
    ///
    /// `ability_score` is set to [`Level::initial_ability`], `trend` is
    /// zeroed, and `update_count` starts at zero.
    pub fn new(level: Level) -> Self {
        Self {
            ability_score: level.initial_ability(),
            trend: 0.0,
            current_level: level,
            update_count: 0,
        }
    }

    /// Convenience: state at [`Level::JuniorHigh`].
    pub fn junior_high() -> Self {
        Self::new(Level::JuniorHigh)
    }

    /// Convenience: state at [`Level::SeniorHigh`].
    pub fn senior_high() -> Self {
        Self::new(Level::SeniorHigh)
    }

    /// Convenience: state at [`Level::Undergraduate`].
    pub fn undergraduate() -> Self {
        Self::new(Level::Undergraduate)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_sets_initial_ability_and_zero_trend() {
        let s = AdaptiveState::new(Level::SeniorHigh);
        assert_eq!(s.ability_score, 300.0);
        assert_eq!(s.trend, 0.0);
        assert_eq!(s.current_level, Level::SeniorHigh);
        assert_eq!(s.update_count, 0);
    }

    #[test]
    fn convenience_constructors() {
        assert_eq!(AdaptiveState::junior_high().ability_score, 100.0);
        assert_eq!(AdaptiveState::senior_high().ability_score, 300.0);
        assert_eq!(AdaptiveState::undergraduate().ability_score, 500.0);
    }

    #[test]
    fn deserialize_without_update_count_defaults_to_zero() {
        let json = r#"{"ability_score":150.0,"trend":0.1,"current_level":"senior_high"}"#;
        let s: AdaptiveState = serde_json::from_str(json).unwrap();
        assert_eq!(s.ability_score, 150.0);
        assert_eq!(s.trend, 0.1);
        assert_eq!(s.current_level, Level::SeniorHigh);
        assert_eq!(s.update_count, 0);
    }
}