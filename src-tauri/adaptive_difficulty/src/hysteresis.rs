//! Hysteresis rules for switching between difficulty levels.
//!
//! IMPORTANT — do NOT flip the comparison direction (`>` vs `<`) without
//! consulting the spec. The spec uses:
//! - `>` (strictly greater) for **promotion** thresholds.
//! - `<=` (less than or equal) for **demotion** thresholds.
//!
//! With these conventions:
//! - `ability == B1 + BUFFER == 220.0` exactly → no promotion (stays junior).
//! - `ability == B1 - BUFFER == 180.0` exactly → demotes (stays senior→junior).
//!
//! The asymmetry is intentional: it produces the buffer zone `[B1 - BUFFER,
//! B1 + BUFFER]` (and similarly around B2) where the level does not change.

use crate::level::Level;
use crate::params::Params;

/// Decide the new level using the **new** `ability` together with the
/// **old** level (i.e. `state.current_level` *before* it is overwritten).
///
/// No direct junior_high ↔ undergraduate transition is possible — senior_high
/// is always the intermediate step. This matches the spec.
pub fn apply_hysteresis(params: &Params, ability: f64, old_level: Level) -> Level {
    let upper_senior = params.b1 + params.buffer;
    let lower_senior = params.b1 - params.buffer;
    let upper_undergrad = params.b2 + params.buffer;
    let lower_undergrad = params.b2 - params.buffer;

    match old_level {
        Level::JuniorHigh => {
            if ability > upper_senior {
                Level::SeniorHigh
            } else {
                Level::JuniorHigh
            }
        }
        Level::SeniorHigh => {
            // Check demotion first (lower bound), then promotion (upper bound).
            // They are mutually exclusive because lower_senior < upper_undergrad.
            if ability <= lower_senior {
                Level::JuniorHigh
            } else if ability > upper_undergrad {
                Level::Undergraduate
            } else {
                Level::SeniorHigh
            }
        }
        Level::Undergraduate => {
            if ability <= lower_undergrad {
                Level::SeniorHigh
            } else {
                Level::Undergraduate
            }
        }
    }
}