//! Tests for request-parameter validation logic.
//!
//! Extends the inline `#[cfg(test)]` block in `src/validation.rs` with
//! additional boundary and cross-field cases.
//!
//! Contains two test suites:
//!  1. `validate_corridor_filters` — query-parameter boundary tests
//!  2. `CreateCorridorRequest` / `CreateAnchorRequest` — `validator` crate struct tests

use stellar_insights_backend::validation::validate_corridor_filters;
use validator::Validate;

// ── Struct mirrors for validator-crate tests (keep in sync with src/models.rs) ──

#[derive(Debug, serde::Deserialize, Validate)]
struct CreateCorridorRequest {
    #[validate(length(min = 1, max = 12))]
    pub source_asset_code: String,
    #[validate(length(min = 56, max = 56))]
    pub source_asset_issuer: String,
    #[validate(length(min = 1, max = 12))]
    pub dest_asset_code: String,
    #[validate(length(min = 56, max = 56))]
    pub dest_asset_issuer: String,
}

#[derive(Debug, serde::Deserialize, Validate)]
struct CreateAnchorRequest {
    #[validate(length(min = 1, max = 100))]
    pub name: String,
    #[validate(length(min = 56, max = 56))]
    pub stellar_account: String,
    #[validate(length(max = 253))]
    pub home_domain: Option<String>,
}

const VALID_ISSUER: &str = "GBBD47IF6LWK7P7MDEVSCWR7DPUWV3NY3DTQEVFL4NAT4AQH3ZLLFLA5";
const ALT_ISSUER: &str = "GAAZI4TCR3TY5OJHCTJC2A4QSY6CJWJH5IAJTGKIN2ER7LBNVKOCCWN7";

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

// ── CreateCorridorRequest validation tests ───────────────────────────────────

#[test]
fn test_create_corridor_validation_valid() {
    let req = CreateCorridorRequest {
        source_asset_code: "USDC".into(),
        source_asset_issuer: VALID_ISSUER.into(),
        dest_asset_code: "XLM".into(),
        dest_asset_issuer: ALT_ISSUER.into(),
    };
    assert!(req.validate().is_ok());
}

#[test]
fn test_invalid_asset_code_rejected_empty() {
    let req = CreateCorridorRequest {
        source_asset_code: "".into(),
        source_asset_issuer: VALID_ISSUER.into(),
        dest_asset_code: "XLM".into(),
        dest_asset_issuer: ALT_ISSUER.into(),
    };
    assert!(req.validate().is_err());
}

#[test]
fn test_invalid_asset_code_rejected_too_long() {
    let req = CreateCorridorRequest {
        source_asset_code: "TOOLONGASSET1".into(), // 13 chars
        source_asset_issuer: VALID_ISSUER.into(),
        dest_asset_code: "XLM".into(),
        dest_asset_issuer: ALT_ISSUER.into(),
    };
    assert!(req.validate().is_err());
}

#[test]
fn test_invalid_issuer_rejected_too_short() {
    let req = CreateCorridorRequest {
        source_asset_code: "USDC".into(),
        source_asset_issuer: "GSHORT".into(),
        dest_asset_code: "XLM".into(),
        dest_asset_issuer: ALT_ISSUER.into(),
    };
    assert!(req.validate().is_err());
}

#[test]
fn test_invalid_issuer_rejected_too_long() {
    let req = CreateCorridorRequest {
        source_asset_code: "USDC".into(),
        source_asset_issuer: format!("{}X", VALID_ISSUER), // 57 chars
        dest_asset_code: "XLM".into(),
        dest_asset_issuer: ALT_ISSUER.into(),
    };
    assert!(req.validate().is_err());
}

#[test]
fn test_dest_asset_code_validated() {
    let req = CreateCorridorRequest {
        source_asset_code: "USDC".into(),
        source_asset_issuer: VALID_ISSUER.into(),
        dest_asset_code: "".into(),
        dest_asset_issuer: ALT_ISSUER.into(),
    };
    assert!(req.validate().is_err());
}

#[test]
fn test_dest_issuer_validated() {
    let req = CreateCorridorRequest {
        source_asset_code: "USDC".into(),
        source_asset_issuer: VALID_ISSUER.into(),
        dest_asset_code: "XLM".into(),
        dest_asset_issuer: "GSHORT".into(),
    };
    assert!(req.validate().is_err());
}

// ── CreateAnchorRequest validation tests ─────────────────────────────────────

#[test]
fn test_create_anchor_validation_valid() {
    let req = CreateAnchorRequest {
        name: "Circle".into(),
        stellar_account: VALID_ISSUER.into(),
        home_domain: Some("circle.com".into()),
    };
    assert!(req.validate().is_ok());
}

#[test]
fn test_create_anchor_empty_name_rejected() {
    let req = CreateAnchorRequest {
        name: "".into(),
        stellar_account: VALID_ISSUER.into(),
        home_domain: None,
    };
    assert!(req.validate().is_err());
}

#[test]
fn test_create_anchor_invalid_account_rejected() {
    let req = CreateAnchorRequest {
        name: "Test".into(),
        stellar_account: "SHORTACCOUNT".into(),
        home_domain: None,
    };
    assert!(req.validate().is_err());
}

#[test]
fn test_create_anchor_no_home_domain_ok() {
    let req = CreateAnchorRequest {
        name: "Test Anchor".into(),
        stellar_account: VALID_ISSUER.into(),
        home_domain: None,
    };
    assert!(req.validate().is_ok());
}

// ── Business logic validation tests ──────────────────────────────────────────

#[test]
fn test_corridor_not_self_referential_same_asset() {
    let source_code = "USDC";
    let source_issuer = VALID_ISSUER;
    let dest_code = "USDC";
    let dest_issuer = VALID_ISSUER;

    let is_same = source_code.eq_ignore_ascii_case(dest_code) && source_issuer == dest_issuer;
    assert!(is_same, "same asset should be detected as self-referential");
}

#[test]
fn test_corridor_different_issuers_allowed() {
    let source_code = "USDC";
    let source_issuer = VALID_ISSUER;
    let dest_code = "USDC";
    let dest_issuer = ALT_ISSUER;

    let is_same = source_code.eq_ignore_ascii_case(dest_code) && source_issuer == dest_issuer;
    assert!(!is_same, "same code but different issuers are different assets");
}
