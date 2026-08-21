//! Ability-based adaptive difficulty algorithm for the peiyuan English
//! listening practice app.
//!
//! After each 19-question test, call [`update`] with the three score rates
//! (`a`, `b`, `c`) to evolve [`AdaptiveState`]. Hysteresis decides whether
//! the next test runs at a different [`Level`].
//!
//! ```rust
//! use adaptive_difficulty::{AdaptiveState, Level, Params, update};
//!
//! let mut state = AdaptiveState::junior_high();
//! let trace = update(&mut state, &Params::default(), 0.85, 0.67, 0.90).unwrap();
//! assert_eq!(trace.level_before, Level::JuniorHigh);
//! ```

#![forbid(unsafe_code)]
#![warn(missing_debug_implementations)]

pub mod algorithm;
pub mod error;
pub mod hysteresis;
pub mod level;
pub mod params;
pub mod state;
pub mod trace;

pub use crate::algorithm::{reset_to, update, update_with_defaults, validate_loaded};
pub use crate::error::AdaptiveError;
pub use crate::level::Level;
pub use crate::params::Params;
pub use crate::state::AdaptiveState;
pub use crate::trace::UpdateTrace;