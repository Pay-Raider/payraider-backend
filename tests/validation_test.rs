//! Tests for request-parameter validation logic.
//!
//! Extends the inline `#[cfg(test)]` block in `src/validation.rs` with
//! additional boundary and cross-field cases.

use stellar_insights_backend::validation::validate_corridor_filters;

// ── individual parameter validation ─────────────────────────────────────────

#[test]
fn test_all_none_filters_are_valid() {
    assert!(validate_corridor_filters(None, None, None, None).is_ok());
}

#[test]
fn test_exact_boundary_success_rate_zero_and_100() {
    assert!(validate_corridor_filters(Some(0.0), Some(100.0), None, None).is_ok());
}

#[test]
fn test_success_rate_min_above_100_is_invalid() {
    assert!(validate_corridor_filters(Some(100.1), None, None, None).is_err());
}

#[test]
fn test_success_rate_max_above_100_is_invalid() {
    assert!(validate_corridor_filters(None, Some(101.0), None, None).is_err());
}

#[test]
fn test_success_rate_below_zero_is_invalid() {
    assert!(validate_corridor_filters(Some(-0.1), None, None, None).is_err());
}

#[test]
fn test_success_rate_nan_is_invalid() {
    assert!(validate_corridor_filters(Some(f64::NAN), None, None, None).is_err());
    assert!(validate_corridor_filters(None, Some(f64::NAN), None, None).is_err());
}

#[test]
fn test_success_rate_infinity_is_invalid() {
    assert!(validate_corridor_filters(Some(f64::INFINITY), None, None, None).is_err());
    assert!(validate_corridor_filters(Some(f64::NEG_INFINITY), None, None, None).is_err());
}

#[test]
fn test_volume_negative_is_invalid() {
    assert!(validate_corridor_filters(None, None, Some(-1.0), None).is_err());
    assert!(validate_corridor_filters(None, None, None, Some(-0.01)).is_err());
}

#[test]
fn test_volume_zero_is_valid() {
    assert!(validate_corridor_filters(None, None, Some(0.0), None).is_ok());
}

#[test]
fn test_volume_nan_is_invalid() {
    assert!(validate_corridor_filters(None, None, Some(f64::NAN), None).is_err());
}

#[test]
fn test_volume_infinity_is_invalid() {
    assert!(validate_corridor_filters(None, None, Some(f64::INFINITY), None).is_err());
}

#[test]
fn test_volume_extremely_large_is_invalid() {
    // Anything above 1e18 should be rejected
    assert!(validate_corridor_filters(None, None, None, Some(1e19)).is_err());
}

#[test]
fn test_volume_just_under_cap_is_valid() {
    assert!(validate_corridor_filters(None, None, Some(0.0), Some(1e18)).is_ok());
}

// ── cross-field range checks ─────────────────────────────────────────────────

#[test]
fn test_success_rate_min_greater_than_max_is_invalid() {
    assert!(validate_corridor_filters(Some(80.0), Some(70.0), None, None).is_err());
}

#[test]
fn test_success_rate_min_equal_to_max_is_valid() {
    assert!(validate_corridor_filters(Some(50.0), Some(50.0), None, None).is_ok());
}

#[test]
fn test_volume_min_greater_than_max_is_invalid() {
    assert!(validate_corridor_filters(None, None, Some(1000.0), Some(500.0)).is_err());
}

#[test]
fn test_volume_min_equal_to_max_is_valid() {
    assert!(validate_corridor_filters(None, None, Some(250.0), Some(250.0)).is_ok());
}

#[test]
fn test_valid_combined_constraints() {
    assert!(validate_corridor_filters(
        Some(90.0),
        Some(99.9),
        Some(1_000.0),
        Some(10_000_000.0)
    )
    .is_ok());
}

#[test]
fn test_only_min_success_rate_provided_is_valid() {
    assert!(validate_corridor_filters(Some(50.0), None, None, None).is_ok());
}

#[test]
fn test_only_max_success_rate_provided_is_valid() {
    assert!(validate_corridor_filters(None, Some(50.0), None, None).is_ok());
}

#[test]
fn test_only_min_volume_provided_is_valid() {
    assert!(validate_corridor_filters(None, None, Some(100.0), None).is_ok());
}

#[test]
fn test_only_max_volume_provided_is_valid() {
    assert!(validate_corridor_filters(None, None, None, Some(100.0)).is_ok());
}
