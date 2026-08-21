//! Intermediate values from a single [`crate::update`] call.
//!
//! Returned by `update` so the GUI verifier and tests can inspect each
//! step of the algorithm without re-deriving it from inputs.

use serde::{Deserialize, Serialize};

use crate::level::Level;
use crate::params::Params;

/// Per-update trace returned by [`crate::update`].
///
/// `combined` is the raw `W_A*a + W_B*b + W_C*c` (before floor clipping),
/// while `combined_clamped` is the value actually used downstream. Both
/// are exposed so the GUI can display them side-by-side; only
/// `combined_clamped` affects `ability_score`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UpdateTrace {
    /// Step 1: raw weighted score rate (before step 2 floor).
    pub combined: f64,
    /// Step 2: `combined.max(score_floor)`.
    pub combined_clamped: f64,
    /// Step 3: `combined_clamped - 0.8`.
    pub x: f64,
    /// Step 4: `x / (0.8 - score_floor)` — normalized into roughly `[-1, 1]`.
    pub x_norm: f64,
    /// Step 6: `min(max_magnitude, base_magnitude + k * |trend_after|)`.
    pub magnitude: f64,
    /// Step 7: `magnitude * x_norm`. Bounded by step 8 clamp.
    pub delta: f64,

    pub ability_before: f64,
    pub ability_after: f64,
    pub trend_before: f64,
    pub trend_after: f64,

    pub level_before: Level,
    pub level_after: Level,

    /// Snapshot of the `Params` that produced this trace, so the GUI can
    /// annotate each history row with the parameters in force at the time.
    pub params_used: Params,
}