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
    VariationParameterError, VariationReplayConsistencyError,
    VariationReplayKey, VariationSample, VariationSampleError,
    VariationSampleSetError, VariationScale,
    validate_variation_replay_consistency, validate_variation_sample_set,
};

type Parameter = VariationParameter<
    &'static str,
    i32,
    &'static str,
    &'static str,
    u8,
    &'static str,
>;

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
        parameter_identity: "slant",
        scale: VariationScale::Line,
        unit: "caller-owned-unit",
    }
}

#[test]
fn parameter_retains_independent_bound_basis_and_generic_model_metadata() {
    let value = parameter(-10, 0, 15);
    assert_eq!(value.validate(), Ok(()));
    assert_eq!(value.minimum.basis, VariationBoundBasis::Observed);
    assert_eq!(value.parameter_identity, "slant");
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
fn replay_key_retains_parameter_seed_and_semantic_identity() {
    let first = VariationReplayKey {
        document_seed: 41_u64,
        parameter_identity: "slant",
        semantic_identity: "block-7",
    };
    let same = first.clone();
    let different_seed = VariationReplayKey {
        document_seed: 42_u64,
        parameter_identity: "slant",
        semantic_identity: "block-7",
    };
    let different_identity = VariationReplayKey {
        document_seed: 41_u64,
        parameter_identity: "slant",
        semantic_identity: "block-8",
    };
    let different_parameter = VariationReplayKey {
        document_seed: 41_u64,
        parameter_identity: "spacing",
        semantic_identity: "block-7",
    };
    assert_eq!(first, same);
    assert_ne!(first, different_seed);
    assert_ne!(first, different_identity);
    assert_ne!(first, different_parameter);
}

#[test]
fn sampled_values_are_checked_against_inclusive_parameter_bounds() {
    let parameter = parameter(-10, 0, 15);
    for sampled_value in [-10, -3, 0, 15] {
        let sample = VariationSample {
            replay_key: VariationReplayKey {
                document_seed: 41_u64,
                parameter_identity: "slant",
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
            parameter_identity: "slant",
            semantic_identity: "block-7",
        },
        value: -11,
    };
    let above = VariationSample {
        replay_key: VariationReplayKey {
            document_seed: 41_u64,
            parameter_identity: "slant",
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
            parameter_identity: "slant",
            semantic_identity: "glyph-é-17",
        },
        value: 7,
    };
    assert_eq!(parameter.validate_sample(&sample), Ok(()));
    assert_eq!(sample.replay_key.document_seed, 9001);
    assert_eq!(sample.replay_key.parameter_identity, "slant");
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
                            parameter_identity: "slant",
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

#[test]
fn repeated_replay_key_requires_the_exact_same_sampled_value() {
    let samples = vec![
        VariationSample {
            replay_key: VariationReplayKey {
                document_seed: 41_u64,
                parameter_identity: "slant",
                semantic_identity: "glyph-a",
            },
            value: 7_i32,
        },
        VariationSample {
            replay_key: VariationReplayKey {
                document_seed: 41_u64,
                parameter_identity: "slant",
                semantic_identity: "glyph-a",
            },
            value: 7_i32,
        },
    ];
    assert_eq!(validate_variation_replay_consistency(&samples), Ok(()));
}

#[test]
fn replay_key_distinguishes_parameter_seed_and_semantic_identity() {
    let samples = vec![
        VariationSample {
            replay_key: VariationReplayKey {
                document_seed: 41_u64,
                parameter_identity: "slant",
                semantic_identity: "glyph-a",
            },
            value: 1_i32,
        },
        VariationSample {
            replay_key: VariationReplayKey {
                document_seed: 42_u64,
                parameter_identity: "slant",
                semantic_identity: "glyph-a",
            },
            value: 2_i32,
        },
        VariationSample {
            replay_key: VariationReplayKey {
                document_seed: 41_u64,
                parameter_identity: "slant",
                semantic_identity: "glyph-b",
            },
            value: 3_i32,
        },
    ];
    assert_eq!(validate_variation_replay_consistency(&samples), Ok(()));
}

#[test]
fn replay_conflict_reports_first_later_observation_and_earliest_prior_match() {
    let key_a = VariationReplayKey {
        document_seed: 41_u8,
        parameter_identity: "slant",
        semantic_identity: "glyph-a",
    };
    let key_b = VariationReplayKey {
        document_seed: 41_u8,
        parameter_identity: "slant",
        semantic_identity: "glyph-b",
    };
    let samples = vec![
        VariationSample {
            replay_key: key_a.clone(),
            value: 3_u8,
        },
        VariationSample {
            replay_key: key_b,
            value: 9_u8,
        },
        VariationSample {
            replay_key: key_a.clone(),
            value: 3_u8,
        },
        VariationSample {
            replay_key: key_a,
            value: 4_u8,
        },
    ];
    assert_eq!(
        validate_variation_replay_consistency(&samples),
        Err(VariationReplayConsistencyError {
            conflicting_index: 3,
            first_index: 0,
        }),
    );
}

#[test]
fn compact_replay_sequences_match_independent_first_conflict_oracle() {
    let mut cases = 0_u16;
    let mut saw_consistent = false;
    let mut saw_conflict = false;
    for length in 0_u32..=4 {
        let case_count = 4_u32.pow(length);
        for encoded in 0_u32..case_count {
            let mut state = encoded;
            let mut samples = Vec::new();
            for _ in 0..length {
                let symbol = state % 4;
                state /= 4;
                samples.push(VariationSample {
                    replay_key: VariationReplayKey {
                        document_seed: 17_u8,
                        parameter_identity: "slant",
                        semantic_identity: (symbol / 2) as u8,
                    },
                    value: (symbol % 2) as u8,
                });
            }

            let mut expected = Ok(());
            'outer: for conflicting_index in 0..samples.len() {
                for first_index in 0..conflicting_index {
                    if samples[first_index].replay_key
                        == samples[conflicting_index].replay_key
                        && samples[first_index].value
                            != samples[conflicting_index].value
                    {
                        expected = Err(VariationReplayConsistencyError {
                            conflicting_index,
                            first_index,
                        });
                        break 'outer;
                    }
                }
            }
            if expected.is_ok() {
                saw_consistent = true;
            } else {
                saw_conflict = true;
            }
            assert_eq!(
                validate_variation_replay_consistency(&samples),
                expected,
                "length {length}, encoded {encoded}",
            );
            cases = cases.saturating_add(1);
        }
    }
    assert_eq!(cases, 341);
    assert!(saw_consistent);
    assert!(saw_conflict);
}
#[test]
fn different_parameter_identities_do_not_collide_in_replay_consistency() {
    let samples = vec![
        VariationSample {
            replay_key: VariationReplayKey {
                document_seed: 41_u8,
                parameter_identity: "slant",
                semantic_identity: "glyph-a",
            },
            value: 1_i32,
        },
        VariationSample {
            replay_key: VariationReplayKey {
                document_seed: 41_u8,
                parameter_identity: "spacing",
                semantic_identity: "glyph-a",
            },
            value: 2_i32,
        },
    ];
    assert_eq!(validate_variation_replay_consistency(&samples), Ok(()));
}

#[test]
fn complete_sample_set_rejects_a_different_parameter_identity_first() {
    let parameter = parameter(-10, 0, 15);
    let samples = vec![VariationSample {
        replay_key: VariationReplayKey {
            document_seed: 41_u8,
            parameter_identity: "spacing",
            semantic_identity: "glyph-a",
        },
        value: 100_i32,
    }];
    assert_eq!(
        validate_variation_sample_set(&parameter, &samples),
        Err(VariationSampleSetError::ParameterIdentityMismatch {
            sample_index: 0,
        }),
    );
}

#[test]
fn complete_sample_set_checks_parameter_bounds_before_replay_consistency() {
    let invalid_parameter = parameter(5, 4, 3);
    let invalid_samples = vec![
        VariationSample {
            replay_key: VariationReplayKey {
                document_seed: 1_u8,
                parameter_identity: "slant",
                semantic_identity: 1_u8,
            },
            value: 2_i32,
        },
        VariationSample {
            replay_key: VariationReplayKey {
                document_seed: 1_u8,
                parameter_identity: "slant",
                semantic_identity: 1_u8,
            },
            value: 3_i32,
        },
    ];
    assert_eq!(
        validate_variation_sample_set(&invalid_parameter, &invalid_samples),
        Err(VariationSampleSetError::Parameter(
            VariationParameterError::MinimumAboveMaximum,
        )),
    );

    let valid_parameter = parameter(-2, 0, 2);
    assert_eq!(
        validate_variation_sample_set(&valid_parameter, &invalid_samples),
        Err(VariationSampleSetError::Sample {
            error: VariationSampleError::AboveMaximum,
            sample_index: 1,
        }),
    );
}

#[test]
fn complete_sample_set_reports_replay_only_after_all_values_are_in_bounds() {
    let parameter = parameter(-2, 0, 2);
    let samples = vec![
        VariationSample {
            replay_key: VariationReplayKey {
                document_seed: 17_u8,
                parameter_identity: "slant",
                semantic_identity: 23_u8,
            },
            value: -1_i32,
        },
        VariationSample {
            replay_key: VariationReplayKey {
                document_seed: 17_u8,
                parameter_identity: "slant",
                semantic_identity: 23_u8,
            },
            value: 1_i32,
        },
    ];
    assert_eq!(
        validate_variation_sample_set(&parameter, &samples),
        Err(VariationSampleSetError::ReplayConflict(
            VariationReplayConsistencyError {
                conflicting_index: 1,
                first_index: 0,
            },
        )),
    );
}

#[test]
fn compact_sample_sets_match_identity_bounds_and_replay_precedence() {
    let mut cases = 0_u16;
    let mut outcomes = [false; 5];
    for minimum in -1_i8..=1 {
        for central in -1_i8..=1 {
            for maximum in -1_i8..=1 {
                let parameter = VariationParameter {
                    central_tendency: central,
                    context_rules: Vec::<u8>::new(),
                    correlation_groups: Vec::<u8>::new(),
                    distribution: (),
                    maximum: VariationBound {
                        basis: VariationBoundBasis::Authorized,
                        value: maximum,
                    },
                    minimum: VariationBound {
                        basis: VariationBoundBasis::Observed,
                        value: minimum,
                    },
                    parameter_identity: 5_u8,
                    scale: VariationScale::Character,
                    unit: (),
                };
                for first_identity_matches in [false, true] {
                    for second_identity_matches in [false, true] {
                        let first_parameter_identity =
                            if first_identity_matches { 5_u8 } else { 6_u8 };
                        let second_parameter_identity =
                            if second_identity_matches { 5_u8 } else { 6_u8 };
                        for first_value in -2_i8..=2 {
                            for second_value in -2_i8..=2 {
                                let samples = [
                                    VariationSample {
                                        replay_key: VariationReplayKey {
                                            document_seed: 7_u8,
                                            parameter_identity:
                                                first_parameter_identity,
                                            semantic_identity: 9_u8,
                                        },
                                        value: first_value,
                                    },
                                    VariationSample {
                                        replay_key: VariationReplayKey {
                                            document_seed: 7_u8,
                                            parameter_identity:
                                                second_parameter_identity,
                                            semantic_identity: 9_u8,
                                        },
                                        value: second_value,
                                    },
                                ];
                                let expected = match parameter.validate() {
                                    Err(error) => {
                                        outcomes[0] = true;
                                        Err(VariationSampleSetError::Parameter(
                                            error,
                                        ))
                                    }
                                    Ok(()) if !first_identity_matches => {
                                        outcomes[1] = true;
                                        Err(
                                            VariationSampleSetError::
                                                ParameterIdentityMismatch {
                                                    sample_index: 0,
                                                },
                                        )
                                    }
                                    Ok(()) if !second_identity_matches => {
                                        outcomes[1] = true;
                                        Err(
                                            VariationSampleSetError::
                                                ParameterIdentityMismatch {
                                                    sample_index: 1,
                                                },
                                        )
                                    }
                                    Ok(()) if first_value < minimum => {
                                        outcomes[2] = true;
                                        Err(VariationSampleSetError::Sample {
                                            error: VariationSampleError::
                                                BelowMinimum,
                                            sample_index: 0,
                                        })
                                    }
                                    Ok(()) if first_value > maximum => {
                                        outcomes[2] = true;
                                        Err(VariationSampleSetError::Sample {
                                            error: VariationSampleError::
                                                AboveMaximum,
                                            sample_index: 0,
                                        })
                                    }
                                    Ok(()) if second_value < minimum => {
                                        outcomes[2] = true;
                                        Err(VariationSampleSetError::Sample {
                                            error: VariationSampleError::
                                                BelowMinimum,
                                            sample_index: 1,
                                        })
                                    }
                                    Ok(()) if second_value > maximum => {
                                        outcomes[2] = true;
                                        Err(VariationSampleSetError::Sample {
                                            error: VariationSampleError::
                                                AboveMaximum,
                                            sample_index: 1,
                                        })
                                    }
                                    Ok(()) if first_value != second_value => {
                                        outcomes[3] = true;
                                        let conflict =
                                            VariationReplayConsistencyError {
                                                conflicting_index: 1,
                                                first_index: 0,
                                            };
                                        Err(
                                            VariationSampleSetError::
                                                ReplayConflict(conflict),
                                        )
                                    }
                                    Ok(()) => {
                                        outcomes[4] = true;
                                        Ok(())
                                    }
                            };
                            assert_eq!(
                                validate_variation_sample_set(
                                    &parameter,
                                    &samples,
                                ),
                                expected,
                                concat!(
                                    "({}, {}, {}); ids {}/{}; [{}, {}]",
                                ),
                                minimum,
                                central,
                                maximum,
                                first_identity_matches,
                                second_identity_matches,
                                first_value,
                                second_value,
                            );
                            cases = cases.saturating_add(1);
                        }
                    }
                }
            }
        }
    }
    }
    assert_eq!(cases, 2_700);
    assert!(outcomes.into_iter().all(|seen| seen));
}
