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
//   - Regression evidence for calibration provenance and held-out sample roles.
// - Must-Not:
//   - Define capture geometry, confidence scales, units, extraction algorithms,
//     minimum sample sets, or handwriting synthesis behavior.
// - Allows:
//   - Inputs: Deterministic caller-owned evidence and sample-role fixtures.
//   - Outputs: Assertions over provenance retention and role separation.
//   - Side effects: Process-local test allocation only.
// - Split-When:
//   - Capture, extraction, or calibration workflow gains independent fixtures.
// - Merge-When:
//   - Calibration evidence validation moves into another pure domain harness.
// - Summary:
//   - Proves observed/inferred provenance and held-out separation stay
//     explicit.
// - Description:
//   - Keeps evidence vocabulary generic while enforcing accepted ADR
//     boundaries.
// - Usage:
//   - Compile directly against the handwriting-calibration-evidence domain.
// - Defaults:
//   - Underdetermined required behavior requests another sample.
//
use atrament_handwriting_calibration_evidence::{
    CalibrationDetermination, CalibrationEvidenceBasis,
    CalibrationParameterEvidence, CalibrationSample, CalibrationSampleRole,
    CalibrationSampleRoleError, HeldOutQualityDimension,
    HeldOutQualityMeasurement, HeldOutQualityReport, HeldOutQualityReportError,
    validate_calibration_sample_roles, validate_held_out_quality_report,
    validate_held_out_quality_report_view,
};

#[derive(Clone, Debug, Eq, PartialEq)]
struct Correction(&'static str);

#[test]
fn parameter_evidence_retains_source_units_confidence_and_corrections() {
    let evidence = CalibrationParameterEvidence {
        accepted_corrections: vec![Correction("accepted-baseline-adjustment")],
        basis: CalibrationEvidenceBasis::Observed,
        confidence: "reviewed-high-confidence",
        parameter: "body-x-height",
        source_region: "sheet-2/region-14",
        unit: "caller-owned-physical-unit",
        value: 37_u32,
    };

    assert_eq!(evidence.parameter, "body-x-height");
    assert_eq!(evidence.source_region, "sheet-2/region-14");
    assert_eq!(evidence.unit, "caller-owned-physical-unit");
    assert_eq!(evidence.confidence, "reviewed-high-confidence");
    assert_eq!(
        evidence.accepted_corrections,
        [Correction("accepted-baseline-adjustment")],
    );
}

#[test]
fn observed_evidence_and_inferred_extremes_never_share_a_basis() {
    let observed = CalibrationEvidenceBasis::Observed;
    let inferred = CalibrationEvidenceBasis::InferredExtreme;

    assert_ne!(observed, inferred);
    assert_eq!(observed, CalibrationEvidenceBasis::Observed);
    assert_eq!(inferred, CalibrationEvidenceBasis::InferredExtreme);
}

#[test]
fn underdetermined_required_behavior_requests_another_sample() {
    assert!(
        CalibrationDetermination::Underdetermined.requires_additional_sample()
    );
    assert!(!CalibrationDetermination::Determined.requires_additional_sample());
}

#[test]
fn held_out_and_training_sample_identities_must_be_disjoint() {
    let samples = vec![
        CalibrationSample {
            identity: "training-a",
            role: CalibrationSampleRole::Training,
        },
        CalibrationSample {
            identity: "held-out-a",
            role: CalibrationSampleRole::HeldOut,
        },
        CalibrationSample {
            identity: "training-a",
            role: CalibrationSampleRole::Training,
        },
    ];
    assert_eq!(validate_calibration_sample_roles(&samples), Ok(()));

    let conflicting = vec![
        CalibrationSample {
            identity: "sample-a",
            role: CalibrationSampleRole::Training,
        },
        CalibrationSample {
            identity: "sample-a",
            role: CalibrationSampleRole::HeldOut,
        },
    ];
    assert_eq!(
        validate_calibration_sample_roles(&conflicting),
        Err(CalibrationSampleRoleError::ConflictingRole {
            sample: "sample-a",
        }),
    );
}

#[test]
fn first_role_conflict_follows_sample_order_not_identity_order() {
    let samples = vec![
        CalibrationSample {
            identity: "z-later-key",
            role: CalibrationSampleRole::Training,
        },
        CalibrationSample {
            identity: "a-earlier-key",
            role: CalibrationSampleRole::Training,
        },
        CalibrationSample {
            identity: "z-later-key",
            role: CalibrationSampleRole::HeldOut,
        },
        CalibrationSample {
            identity: "a-earlier-key",
            role: CalibrationSampleRole::HeldOut,
        },
    ];
    assert_eq!(
        validate_calibration_sample_roles(&samples),
        Err(CalibrationSampleRoleError::ConflictingRole {
            sample: "z-later-key",
        }),
    );
}

fn reference_sample_role_validation(
    samples: &[CalibrationSample<u8>],
) -> Result<(), CalibrationSampleRoleError<u8>> {
    let mut roles = [None; 2];
    for sample in samples {
        let slot = &mut roles[usize::from(sample.identity)];
        if let Some(existing) = slot {
            if *existing != sample.role {
                return Err(CalibrationSampleRoleError::ConflictingRole {
                    sample: sample.identity,
                });
            }
        } else {
            *slot = Some(sample.role);
        }
    }
    Ok(())
}

#[test]
fn every_compact_sample_role_sequence_matches_separation_oracle() {
    let mut cases = 0_usize;
    let mut saw_valid = false;
    let mut saw_conflict = [false; 2];
    for length in 0_u32..=6 {
        for encoded in 0_usize..4_usize.pow(length) {
            let mut value = encoded;
            let mut samples = Vec::with_capacity(length as usize);
            for _ in 0..length {
                let symbol = value % 4;
                value /= 4;
                let identity = u8::try_from(symbol / 2)
                    .expect("compact identity fits u8");
                let role = if symbol % 2 == 0 {
                    CalibrationSampleRole::Training
                } else {
                    CalibrationSampleRole::HeldOut
                };
                samples.push(CalibrationSample { identity, role });
            }
            let expected = reference_sample_role_validation(&samples);
            match expected {
                Ok(()) => saw_valid = true,
                Err(CalibrationSampleRoleError::ConflictingRole { sample }) => {
                    saw_conflict[usize::from(sample)] = true;
                },
            }
            assert_eq!(
                validate_calibration_sample_roles(&samples),
                expected,
                "role mismatch at length {length} encoding {encoded:#x}",
            );
            cases += 1;
        }
    }
    assert_eq!(cases, 5_461);
    assert!(saw_valid);
    assert!(saw_conflict.into_iter().all(|seen| seen));
}

fn held_out_samples() -> Vec<CalibrationSample<&'static str>> {
    vec![
        CalibrationSample {
            identity: "held-a",
            role: CalibrationSampleRole::HeldOut,
        },
        CalibrationSample {
            identity: "held-b",
            role: CalibrationSampleRole::HeldOut,
        },
        CalibrationSample {
            identity: "training-a",
            role: CalibrationSampleRole::Training,
        },
    ]
}

fn complete_quality_report(
) -> HeldOutQualityReport<&'static str, i32, &'static str, &'static str> {
    use HeldOutQualityDimension::{
        Geometry, Joins, PerceptualFidelity, Punctuation, Rhythm, Spacing,
    };
    HeldOutQualityReport {
        known_failures: vec!["weak-terminal-join-on-small-writing"],
        measurements: vec![
            HeldOutQualityMeasurement {
                dimension: Geometry,
                evidence: "geometry-evidence",
                sample: "held-a",
                value: 91,
            },
            HeldOutQualityMeasurement {
                dimension: Rhythm,
                evidence: "rhythm-evidence",
                sample: "held-b",
                value: 87,
            },
            HeldOutQualityMeasurement {
                dimension: Joins,
                evidence: "join-evidence",
                sample: "held-a",
                value: 84,
            },
            HeldOutQualityMeasurement {
                dimension: Spacing,
                evidence: "spacing-evidence",
                sample: "held-b",
                value: 93,
            },
            HeldOutQualityMeasurement {
                dimension: Punctuation,
                evidence: "punctuation-evidence",
                sample: "held-a",
                value: 89,
            },
            HeldOutQualityMeasurement {
                dimension: PerceptualFidelity,
                evidence: "perceptual-evidence",
                sample: "held-b",
                value: 86,
            },
        ],
    }
}

#[test]
fn held_out_report_retains_all_required_dimensions_and_known_failures() {
    let report = complete_quality_report();
    let samples = held_out_samples();
    assert_eq!(
        validate_held_out_quality_report(&samples, &report),
        Ok(()),
    );
    let validated = validate_held_out_quality_report_view(&samples, &report)
        .expect("complete held-out report seals exact evidence");
    assert!(std::ptr::eq(validated.report(), &report));
    assert!(std::ptr::eq(validated.samples(), samples.as_slice()));
    assert_eq!(report.measurements.len(), 6);
    assert_eq!(
        report.known_failures,
        ["weak-terminal-join-on-small-writing"],
    );
    assert_eq!(report.measurements[0].sample, "held-a");
    assert_eq!(report.measurements[0].value, 91);
    assert_eq!(report.measurements[0].evidence, "geometry-evidence");
}

#[test]
fn held_out_report_rejects_training_and_unknown_measurement_samples_in_order() {
    let mut report = complete_quality_report();
    report.measurements[0].sample = "training-a";
    report.measurements[1].sample = "unknown-a";
    assert_eq!(
        validate_held_out_quality_report(&held_out_samples(), &report),
        Err(HeldOutQualityReportError::TrainingSample {
            sample: "training-a",
        }),
    );

    report.measurements[0].sample = "held-a";
    assert_eq!(
        validate_held_out_quality_report(&held_out_samples(), &report),
        Err(HeldOutQualityReportError::UnknownSample {
            sample: "unknown-a",
        }),
    );
}

#[test]
fn sample_role_conflict_rejects_before_held_out_report_measurements() {
    let samples = vec![
        CalibrationSample {
            identity: "conflict",
            role: CalibrationSampleRole::Training,
        },
        CalibrationSample {
            identity: "conflict",
            role: CalibrationSampleRole::HeldOut,
        },
    ];
    let mut report = complete_quality_report();
    report.measurements[0].sample = "unknown-first-measurement";
    let expected = HeldOutQualityReportError::ConflictingSampleRole {
        sample: "conflict",
    };
    assert_eq!(
        validate_held_out_quality_report(&samples, &report),
        Err(expected.clone()),
    );
    assert_eq!(
        validate_held_out_quality_report_view(&samples, &report),
        Err(expected),
    );
}

#[test]
fn held_out_report_requires_every_dimension_without_metric_thresholds() {
    let required = [
        HeldOutQualityDimension::Geometry,
        HeldOutQualityDimension::Rhythm,
        HeldOutQualityDimension::Joins,
        HeldOutQualityDimension::Spacing,
        HeldOutQualityDimension::Punctuation,
        HeldOutQualityDimension::PerceptualFidelity,
    ];
    for (index, missing) in required.into_iter().enumerate() {
        let mut report = complete_quality_report();
        report.measurements.remove(index);
        assert_eq!(
            validate_held_out_quality_report(&held_out_samples(), &report),
            Err(HeldOutQualityReportError::MissingDimension {
                dimension: missing,
            }),
            "missing dimension at report requirement index {index}",
        );
    }
}

#[test]
fn repeated_held_out_measurements_and_empty_known_failures_remain_valid() {
    let mut report = complete_quality_report();
    report.known_failures.clear();
    report.measurements.push(report.measurements[0].clone());
    assert_eq!(
        validate_held_out_quality_report(&held_out_samples(), &report),
        Ok(()),
    );
    assert_eq!(report.measurements.len(), 7);
}

#[test]
fn every_quality_dimension_subset_matches_completeness_oracle() {
    use HeldOutQualityDimension::{
        Geometry, Joins, PerceptualFidelity, Punctuation, Rhythm, Spacing,
    };
    let dimensions = [
        Geometry,
        Rhythm,
        Joins,
        Spacing,
        Punctuation,
        PerceptualFidelity,
    ];
    let samples = [CalibrationSample {
        identity: 0_u8,
        role: CalibrationSampleRole::HeldOut,
    }];
    let mut cases = 0_u8;
    for mask in 0_u8..64 {
        let report = HeldOutQualityReport::<u8, u8, (), ()> {
            known_failures: Vec::new(),
            measurements: dimensions
                .iter()
                .enumerate()
                .filter_map(|(index, dimension)| {
                    (mask & (1_u8 << index) != 0).then_some(
                        HeldOutQualityMeasurement {
                            dimension: *dimension,
                            evidence: (),
                            sample: 0,
                            value: index as u8,
                        },
                    )
                })
                .collect(),
        };
        let expected = dimensions
            .iter()
            .enumerate()
            .find(|(index, _)| mask & (1_u8 << index) == 0)
            .map_or(Ok(()), |(_, dimension)| {
                Err(HeldOutQualityReportError::MissingDimension {
                    dimension: *dimension,
                })
            });
        assert_eq!(
            validate_held_out_quality_report(&samples, &report),
            expected.clone(),
            "dimension subset mask {mask:#08b}",
        );
        match expected {
            Ok(()) => {
                let validated =
                    validate_held_out_quality_report_view(&samples, &report)
                        .expect("complete dimension set seals exact report");
                assert!(std::ptr::eq(validated.report(), &report));
                assert!(std::ptr::eq(validated.samples(), samples.as_slice()));
            },
            Err(reason) => assert_eq!(
                validate_held_out_quality_report_view(&samples, &report),
                Err(reason),
                "sealed dimension subset mask {mask:#08b}",
            ),
        }
        cases = cases.saturating_add(1);
    }
    assert_eq!(cases, 64);
}

#[test]
fn every_compact_measurement_role_sequence_matches_first_failure_oracle() {
    use HeldOutQualityDimension::{
        Geometry, Joins, PerceptualFidelity, Punctuation, Rhythm, Spacing,
    };
    let dimensions = [
        Geometry,
        Rhythm,
        Joins,
        Spacing,
        Punctuation,
        PerceptualFidelity,
    ];
    let samples = [
        CalibrationSample {
            identity: 0_u8,
            role: CalibrationSampleRole::HeldOut,
        },
        CalibrationSample {
            identity: 1_u8,
            role: CalibrationSampleRole::Training,
        },
    ];
    let mut cases = 0_u16;
    let mut saw = [false; 3];
    for encoded in 0_u16..729 {
        let mut state = encoded;
        let mut references = [0_u8; 6];
        for reference in &mut references {
            *reference = (state % 3) as u8;
            state /= 3;
        }
        let report = HeldOutQualityReport::<u8, u8, (), ()> {
            known_failures: Vec::new(),
            measurements: dimensions
                .iter()
                .zip(references)
                .enumerate()
                .map(|(index, (dimension, sample))| {
                    HeldOutQualityMeasurement {
                        dimension: *dimension,
                        evidence: (),
                        sample,
                        value: index as u8,
                    }
                })
                .collect(),
        };
        let expected = match references.iter().find(|sample| **sample != 0) {
            None => {
                saw[0] = true;
                Ok(())
            },
            Some(1) => {
                saw[1] = true;
                Err(HeldOutQualityReportError::TrainingSample { sample: 1 })
            },
            Some(2) => {
                saw[2] = true;
                Err(HeldOutQualityReportError::UnknownSample { sample: 2 })
            },
            Some(_) => unreachable!("base-3 selector is bounded"),
        };
        assert_eq!(
            validate_held_out_quality_report(&samples, &report),
            expected.clone(),
            "measurement role sequence {encoded}",
        );
        match expected {
            Ok(()) => {
                let validated =
                    validate_held_out_quality_report_view(&samples, &report)
                        .expect("all measurements reference held-out sample");
                assert!(std::ptr::eq(validated.report(), &report));
            },
            Err(reason) => assert_eq!(
                validate_held_out_quality_report_view(&samples, &report),
                Err(reason),
                "sealed measurement role sequence {encoded}",
            ),
        }
        cases = cases.saturating_add(1);
    }
    assert_eq!(cases, 729);
    assert!(saw.into_iter().all(|seen| seen));
}
