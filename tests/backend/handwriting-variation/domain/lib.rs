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
    VariationParameterError, VariationReplayKey, VariationSample,
    VariationSampleError, VariationScale,
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
    assert_eq!(value.validate(), Ok(()));
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
        parameter(-10, -11, 15).validate(),
        Err(VariationParameterError::CentralTendencyBelowMinimum),
    );
    assert_eq!(
        parameter(-10, 16, 15).validate(),
        Err(VariationParameterError::CentralTendencyAboveMaximum),
    );
}

#[test]
fn minimum_must_not_exceed_maximum() {
    assert_eq!(
        parameter(16, 16, 15).validate(),
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

#[test]
fn sampled_values_are_checked_against_inclusive_parameter_bounds() {
    let parameter = parameter(-10, 0, 15);
    for sampled_value in [-10, -3, 0, 15] {
        let sample = VariationSample {
            replay_key: VariationReplayKey {
                document_seed: 41_u64,
                semantic_identity: "block-7",
            },
            value: sampled_value,
        };
        assert_eq!(parameter.validate_sample(&sample), Ok(()));
    }
}

#[test]
fn sampled_value_outside_parameter_bounds_rejects_explicitly() {
    let parameter = parameter(-10, 0, 15);
    let below = VariationSample {
        replay_key: VariationReplayKey {
            document_seed: 41_u64,
            semantic_identity: "block-7",
        },
        value: -11,
    };
    let above = VariationSample {
        replay_key: VariationReplayKey {
            document_seed: 41_u64,
            semantic_identity: "block-7",
        },
        value: 16,
    };
    assert_eq!(
        parameter.validate_sample(&below),
        Err(VariationSampleError::BelowMinimum),
    );
    assert_eq!(
        parameter.validate_sample(&above),
        Err(VariationSampleError::AboveMaximum),
    );
}

#[test]
fn sampled_value_retains_exact_replay_inputs_without_interpretation() {
    let parameter = parameter(-10, 0, 15);
    let sample = VariationSample {
        replay_key: VariationReplayKey {
            document_seed: 9001_u64,
            semantic_identity: "glyph-é-17",
        },
        value: 7,
    };
    assert_eq!(parameter.validate_sample(&sample), Ok(()));
    assert_eq!(sample.replay_key.document_seed, 9001);
    assert_eq!(sample.replay_key.semantic_identity, "glyph-é-17");
    assert_eq!(sample.value, 7);
}

#[test]
fn every_compact_parameter_and_sample_value_matches_bounds_oracle() {
    let mut parameter_cases = 0_u16;
    let mut sample_cases = 0_u16;
    let mut parameter_outcomes = [false; 4];
    let mut sample_outcomes = [false; 3];

    for minimum in -3_i32..=3 {
        for central_tendency in -3_i32..=3 {
            for maximum in -3_i32..=3 {
                let parameter = parameter(minimum, central_tendency, maximum);
                let expected_parameter = if minimum > maximum {
                    parameter_outcomes[1] = true;
                    Err(VariationParameterError::MinimumAboveMaximum)
                } else if central_tendency < minimum {
                    parameter_outcomes[2] = true;
                    Err(VariationParameterError::CentralTendencyBelowMinimum)
                } else if central_tendency > maximum {
                    parameter_outcomes[3] = true;
                    Err(VariationParameterError::CentralTendencyAboveMaximum)
                } else {
                    parameter_outcomes[0] = true;
                    Ok(())
                };
                assert_eq!(
                    parameter.validate(),
                    expected_parameter,
                    "parameter ({minimum}, {central_tendency}, {maximum})",
                );
                parameter_cases = parameter_cases.saturating_add(1);

                if expected_parameter.is_err() {
                    continue;
                }
                for sampled_value in -4_i32..=4 {
                    let sample = VariationSample {
                        replay_key: VariationReplayKey {
                            document_seed: 17_u8,
                            semantic_identity: 23_u8,
                        },
                        value: sampled_value,
                    };
                    let expected_sample = if sampled_value < minimum {
                        sample_outcomes[1] = true;
                        Err(VariationSampleError::BelowMinimum)
                    } else if sampled_value > maximum {
                        sample_outcomes[2] = true;
                        Err(VariationSampleError::AboveMaximum)
                    } else {
                        sample_outcomes[0] = true;
                        Ok(())
                    };
                    assert_eq!(
                        parameter.validate_sample(&sample),
                        expected_sample,
                        concat!(
                            "sample {} for parameter ({}, {}, {})",
                        ),
                        sampled_value,
                        minimum,
                        central_tendency,
                        maximum,
                    );
                    assert_eq!(sample.replay_key.document_seed, 17);
                    assert_eq!(sample.replay_key.semantic_identity, 23);
                    sample_cases = sample_cases.saturating_add(1);
                }
            }
        }
    }

    assert_eq!(parameter_cases, 343);
    assert_eq!(sample_cases, 756);
    assert!(parameter_outcomes.into_iter().all(|seen| seen));
    assert!(sample_outcomes.into_iter().all(|seen| seen));
}
