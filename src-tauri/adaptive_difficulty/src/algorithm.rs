//! The 11-step adaptive update algorithm and helpers.

use crate::error::AdaptiveError;
use crate::hysteresis::apply_hysteresis;
use crate::level::Level;
use crate::params::Params;
use crate::state::AdaptiveState;
use crate::trace::UpdateTrace;

/// `update` with [`Params::default()`] injected.
///
/// Equivalent to `update(state, &Params::default(), a, b, c)`. Use this when
/// production parameters are wanted and the caller has no `Params` to pass.
pub fn update_with_defaults(
    state: &mut AdaptiveState,
    a: f64,
    b: f64,
    c: f64,
) -> Result<UpdateTrace, AdaptiveError> {
    update(state, &Params::default(), a, b, c)
}

/// Apply one 19-question test result.
///
/// - `a`: 1–14 MCQ score rate (`correct / 14`), expected in `[0, 1]`.
/// - `b`: 15–18 fill-in-the-blank score rate, expected in `[0, 1]`.
/// - `c`: Q19 retell score rate, expected in `[0, 1]`.
///
/// On error (`Err`) the state is **not** mutated. On success all three
/// fields are written atomically and `update_count` is incremented.
pub fn update(
    state: &mut AdaptiveState,
    params: &Params,
    a: f64,
    b: f64,
    c: f64,
) -> Result<UpdateTrace, AdaptiveError> {
    // Reject bad inputs BEFORE capturing snapshots, so a failed call leaves
    // state completely untouched.
    validate_score_rate("a", a)?;
    validate_score_rate("b", b)?;
    validate_score_rate("c", c)?;
    validate_params(params)?;

    let ability_before = state.ability_score;
    let trend_before = state.trend;
    let level_before = state.current_level;

    // step 1: weighted combined score rate
    let combined = params.weight_a * a + params.weight_b * b + params.weight_c * c;

    // step 2: floor-clip. Local-only — does NOT mutate `a` / `b` / `c`,
    // so the caller's inputs are preserved for display / return values.
    let combined_clamped = combined.max(params.score_floor);

    // step 3: center at 0.8 (the neutral performance line).
    let x = combined_clamped - 0.8;

    // step 4: normalize so that X_norm ∈ [-1, 1] when a, b, c ∈ [0, 1].
    let denom = 0.8 - params.score_floor;
    if !denom.is_finite() || denom.abs() < f64::EPSILON {
        return Err(AdaptiveError::NonFinite(
            "0.8 - params.score_floor must be non-zero finite".into(),
        ));
    }
    let x_norm = x / denom;
    if !x_norm.is_finite() {
        return Err(AdaptiveError::NonFinite(
            "x_norm is non-finite (check weight / score_floor params)".into(),
        ));
    }

    // step 5: EMA update of trend. Defensive clamp keeps the documented
    // [-1, 1] invariant under any future drift.
    let trend_after_unclamped = params.alpha * x_norm + (1.0 - params.alpha) * state.trend;
    if !trend_after_unclamped.is_finite() {
        return Err(AdaptiveError::NonFinite(
            "trend_after is non-finite (check alpha / params)".into(),
        ));
    }
    let trend_after = trend_after_unclamped.clamp(params.trend_min, params.trend_max);

    // step 6: magnitude ceiling. The intermediate `base + k*|trend|` must be
    // finite BEFORE applying `.min(max_magnitude)` — `f64::min(NaN, x)`
    // returns `x` (the non-NaN argument), which would silently mask NaN
    // params and corrupt ability_score. `validate_params` above catches
    // NaN params at the gate, but we still check the product explicitly
    // in case any future param (e.g. a derived field) becomes NaN later.
    let magnitude_unclamped = params.base_magnitude + params.k * trend_after.abs();
    if !magnitude_unclamped.is_finite() {
        return Err(AdaptiveError::NonFinite(
            "magnitude is non-finite (check base_magnitude / k / max_magnitude)".into(),
        ));
    }
    let magnitude = magnitude_unclamped.min(params.max_magnitude);

    // step 7: signed delta. No extra clamp — X_norm is bounded in [-1, 1]
    // and magnitude ≤ max_magnitude, so |delta| ≤ max_magnitude.
    let delta = magnitude * x_norm;

    // step 8: clamp ability into the documented range.
    let ability_after = (state.ability_score + delta)
        .clamp(params.ability_min, params.ability_max);

    // step 9: hysteresis uses the NEW ability + OLD level (state.current_level
    // is still the old value at this point — we haven't written yet).
    let level_after = apply_hysteresis(params, ability_after, state.current_level);

    // step 10: atomic write of all three fields + counter.
    state.ability_score = ability_after;
    state.trend = trend_after;
    state.current_level = level_after;
    state.update_count = state.update_count.saturating_add(1);

    Ok(UpdateTrace {
        combined,
        combined_clamped,
        x,
        x_norm,
        magnitude,
        delta,
        ability_before,
        ability_after,
        trend_before,
        trend_after,
        level_before,
        level_after,
        params_used: *params,
    })
}

/// Manual reset — user changed difficulty in the settings UI.
///
/// `trend` is zeroed, `ability_score` is set to the level's initial value,
/// `current_level` is set to `level`. `update_count` is **not** incremented
/// (a manual reset is not a test result).
pub fn reset_to(state: &mut AdaptiveState, level: Level) {
    state.ability_score = level.initial_ability();
    state.trend = 0.0;
    state.current_level = level;
    // update_count intentionally preserved.
}

/// Validate a state loaded from external JSON.
///
/// Clamps out-of-range `ability_score` and `trend` back into their legal
/// intervals. Does **not** mutate `current_level` — an unknown level
/// string will already have failed at `serde_json::from_str` and is
/// surfaced by the caller. This function is intentionally idempotent and
/// cheap so callers can apply it before every load.
pub fn validate_loaded(state: AdaptiveState) -> AdaptiveState {
    // Re-derive the [min, max] from the production defaults — we don't want
    // to expose these in the public state and the spec values are fixed.
    let ability_min = 0.0_f64;
    let ability_max = 600.0_f64;
    let trend_min = -1.0_f64;
    let trend_max = 1.0_f64;

    AdaptiveState {
        ability_score: state.ability_score.clamp(ability_min, ability_max),
        trend: state.trend.clamp(trend_min, trend_max),
        current_level: state.current_level,
        update_count: state.update_count,
    }
}

fn validate_score_rate(name: &str, value: f64) -> Result<(), AdaptiveError> {
    if !value.is_finite() || !(0.0..=1.0).contains(&value) {
        return Err(AdaptiveError::InvalidScoreRate(name.to_string()));
    }
    Ok(())
}

/// Reject any `Params` field that is non-finite (NaN / ±∞). Without this
/// guard, NaN values in `k` or `base_magnitude` would propagate through
/// `magnitude` and corrupt `ability_score` permanently — because Rust's
/// `f64::min(NaN, x)` returns `x`, silently masking the NaN.
fn validate_params(p: &Params) -> Result<(), AdaptiveError> {
    let fields: &[(&str, f64)] = &[
        ("weight_a", p.weight_a),
        ("weight_b", p.weight_b),
        ("weight_c", p.weight_c),
        ("score_floor", p.score_floor),
        ("alpha", p.alpha),
        ("base_magnitude", p.base_magnitude),
        ("k", p.k),
        ("max_magnitude", p.max_magnitude),
        ("b1", p.b1),
        ("b2", p.b2),
        ("buffer", p.buffer),
        ("ability_min", p.ability_min),
        ("ability_max", p.ability_max),
        ("trend_min", p.trend_min),
        ("trend_max", p.trend_max),
    ];
    for (name, value) in fields {
        if !value.is_finite() {
            return Err(AdaptiveError::NonFinite(format!("params.{}", name)));
        }
    }
    Ok(())
}