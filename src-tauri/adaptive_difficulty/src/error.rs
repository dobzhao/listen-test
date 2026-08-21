//! Error types for the adaptive difficulty algorithm.

use thiserror::Error;

/// Errors that can be returned from [`crate::update`].
#[derive(Debug, Error)]
pub enum AdaptiveError {
    /// An `a`, `b`, or `c` score rate was outside `[0.0, 1.0]` or not finite
    /// (NaN / ±∞). The payload identifies which input failed.
    #[error("score rate out of [0, 1]: {0}")]
    InvalidScoreRate(String),

    /// A [`crate::Params`] field produced a non-finite intermediate result,
    /// or the step-4 denominator `(0.8 - score_floor)` collapsed to zero.
    /// The state is left unchanged when this is returned.
    #[error("params produce non-finite result: {0}")]
    NonFinite(String),
}