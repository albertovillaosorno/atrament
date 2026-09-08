// Copyright:
//   - Copyright © 2026 Alberto Villa Osorno.
// SPDX-License-Identifier:
//   - MIT
// Confidential:
//   - false
// License-File:
//   - LICENSE-MIT
//
// Boundary-Contract:
// - Owns:
//   - Regression evidence for bounded handwriting-variation domain invariants.
// - Must-Not:
//   - Choose distributions, correlations, context semantics, RNG behavior,
//     parameter vocabularies, units, or sampling algorithms.
// - Allows:
//   - Inputs: Deterministic generic variation envelopes and replay keys.
//   - Outputs: Assertions over bounds, metadata retention, scales, and
//     identity.
//   - Side effects: Process-local test allocation only.
// - Split-When:
//   - Sampling or statistical model validation gains independent fixtures.
// - Merge-When:
//   - Variation validation moves into another pure handwriting domain harness.
// - Summary:
//   - Proves variation envelopes remain bounded and replay inputs remain typed.
// - Description:
//   - Covers bound provenance, central tendency, scales, and generic metadata.
// - Usage:
//   - Compile directly against the handwriting-variation domain.
// - Defaults:
//   - No distribution or correlation algorithm is assumed.
//
use atrament_handwriting_variation::{
    VariationBound, VariationBoundBasis, VariationParameter,
    VariationParameterError, VariationReplayKey, VariationScale,
    validate_variation_parameter,
};

type Parameter =
    VariationParameter<i32, &'static str, &'static str, u8, &'static str>;

fn parameter(minimum: i32, central_tendency: i32, maximum: i32) -> Parameter {
    VariationParameter {
        central_tendency,
        context_rules: vec!["caller-owned-context-rule"],
        correlation_groups: vec![7],
        distribution: "caller-owned-distribution-family",
        maximum: VariationBound {
            basis: VariationBoundBasis::Authorized,
            value: maximum,
        },
        minimum: VariationBound {
            basis: VariationBoundBasis::Observed,
            value: minimum,
        },
        scale: VariationScale::Line,
        unit: "caller-owned-unit",
    }
}

#[test]
fn parameter_retains_independent_bound_basis_and_generic_model_metadata() {
    let value = parameter(-10, 0, 15);
    assert_eq!(validate_variation_parameter(&value), Ok(()));
    assert_eq!(value.minimum.basis, VariationBoundBasis::Observed);
    assert_eq!(value.maximum.basis, VariationBoundBasis::Authorized);
    assert_eq!(value.distribution, "caller-owned-distribution-family");
    assert_eq!(value.correlation_groups, [7]);
    assert_eq!(value.context_rules, ["caller-owned-context-rule"]);
    assert_eq!(value.unit, "caller-owned-unit");
}

#[test]
fn central_tendency_must_stay_inside_the_admitted_bounds() {
    assert_eq!(
        validate_variation_parameter(&parameter(-10, -11, 15)),
        Err(VariationParameterError::CentralTendencyBelowMinimum),
    );
    assert_eq!(
        validate_variation_parameter(&parameter(-10, 16, 15)),
        Err(VariationParameterError::CentralTendencyAboveMaximum),
    );
}

#[test]
fn minimum_must_not_exceed_maximum() {
    assert_eq!(
        validate_variation_parameter(&parameter(16, 16, 15)),
        Err(VariationParameterError::MinimumAboveMaximum),
    );
}

#[test]
fn accepted_scales_cover_profile_through_stroke_scope() {
    let scales = [
        VariationScale::Character,
        VariationScale::Document,
        VariationScale::Line,
        VariationScale::Page,
        VariationScale::Profile,
        VariationScale::Stroke,
        VariationScale::Word,
    ];
    assert_eq!(scales.len(), 7);
}

#[test]
fn replay_key_retains_document_seed_and_stable_semantic_identity() {
    let first = VariationReplayKey {
        document_seed: 41_u64,
        semantic_identity: "block-7",
    };
    let same = first.clone();
    let different_seed = VariationReplayKey {
        document_seed: 42_u64,
        semantic_identity: "block-7",
    };
    let different_identity = VariationReplayKey {
        document_seed: 41_u64,
        semantic_identity: "block-8",
    };
    assert_eq!(first, same);
    assert_ne!(first, different_seed);
    assert_ne!(first, different_identity);
}
