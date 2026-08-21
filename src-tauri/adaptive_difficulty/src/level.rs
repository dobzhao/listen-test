//! Difficulty level enum and per-level initial ability scores.

use serde::{Deserialize, Serialize};

/// The three fixed difficulty levels supported by the peiyuan app.
///
/// Serialized as snake_case strings (`"junior_high"`, `"senior_high"`,
/// `"undergraduate"`) — this matches peiyuan's existing
/// `DifficultyConfig.level: String` byte-for-byte, so the eventual
/// integration step needs no string migration.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Level {
    /// Initial ability = 100. Roughly aligned with Chinese junior-high English.
    JuniorHigh,
    /// Initial ability = 300. Roughly aligned with Chinese senior-high English.
    SeniorHigh,
    /// Initial ability = 500. Roughly aligned with Chinese undergraduate English.
    Undergraduate,
}

impl Level {
    /// The ability score this level is initialized to when
    /// [`crate::reset_to`] is called (or when [`crate::AdaptiveState::new`]
    /// constructs a fresh state).
    ///
    /// These values are **independent state** — they are the starting points
    /// after a manual reset, not derived from `ability_score`. The hysteresis
    /// rules may then nudge `current_level` up or down on the next `update`.
    pub const fn initial_ability(self) -> f64 {
        match self {
            Level::JuniorHigh => 100.0,
            Level::SeniorHigh => 300.0,
            Level::Undergraduate => 500.0,
        }
    }

    /// Iterate over the three levels in ascending difficulty order.
    pub const fn all() -> [Level; 3] {
        [Level::JuniorHigh, Level::SeniorHigh, Level::Undergraduate]
    }

    /// The snake_case string form — identical to serde's output.
    pub const fn as_str(self) -> &'static str {
        match self {
            Level::JuniorHigh => "junior_high",
            Level::SeniorHigh => "senior_high",
            Level::Undergraduate => "undergraduate",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn initial_abilities_match_spec() {
        assert_eq!(Level::JuniorHigh.initial_ability(), 100.0);
        assert_eq!(Level::SeniorHigh.initial_ability(), 300.0);
        assert_eq!(Level::Undergraduate.initial_ability(), 500.0);
    }

    #[test]
    fn as_str_matches_serde_form() {
        assert_eq!(Level::JuniorHigh.as_str(), "junior_high");
        assert_eq!(Level::SeniorHigh.as_str(), "senior_high");
        assert_eq!(Level::Undergraduate.as_str(), "undergraduate");
    }

    #[test]
    fn serde_round_trip_via_string() {
        for level in Level::all() {
            let s = serde_json::to_string(&level).unwrap();
            assert_eq!(s, format!("\"{}\"", level.as_str()));
            let back: Level = serde_json::from_str(&s).unwrap();
            assert_eq!(back, level);
        }
    }

    #[test]
    fn unknown_level_string_fails() {
        let err = serde_json::from_str::<Level>("\"phd\"").unwrap_err();
        // Just assert there is an error — don't pin the exact message.
        assert!(err.to_string().contains("phd") || err.is_data());
    }
}