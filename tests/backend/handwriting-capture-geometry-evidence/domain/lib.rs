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
//   - Regression evidence for photographed calibration-geometry completeness.
// - Must-Not:
//   - Detect marks, fit transforms, correct images, choose units/tolerances,
//     score captures, or extract handwriting.
// - Allows:
//   - Inputs: Deterministic caller-owned per-axis evidence fixtures.
//   - Outputs: Assertions over retention, duplicates, and missing axes.
//   - Side effects: Process-local test allocation only.
// - Split-When:
//   - Geometry algorithms or capture-quality policy gain independent fixtures.
// - Merge-When:
//   - Evidence completeness moves into an input-adapter acceptance harness.
// - Summary:
//   - Proves every TODO-named photographed geometry family remains explicit.
// - Description:
//   - Exhausts compact presence states without implementing geometry math.
// - Usage:
//   - Compile directly against the capture-geometry-evidence domain.
// - Defaults:
//   - Evidence values are supplied by fixtures rather than inferred.
//
use atrament_handwriting_capture_geometry_evidence::{
    PhotographedCalibrationGeometryAxis,
    PhotographedCalibrationGeometryEvidence,
    PhotographedCalibrationGeometryEvidenceError,
    PhotographedCalibrationGeometryObservation,
    validate_photographed_calibration_geometry_evidence,
    validate_photographed_calibration_geometry_evidence_view,
};

const AXES: [PhotographedCalibrationGeometryAxis; 7] = [
    PhotographedCalibrationGeometryAxis::ReferenceMarks,
    PhotographedCalibrationGeometryAxis::Perspective,
    PhotographedCalibrationGeometryAxis::LensDistortion,
    PhotographedCalibrationGeometryAxis::PhysicalScale,
    PhotographedCalibrationGeometryAxis::GridRegistration,
    PhotographedCalibrationGeometryAxis::BaselineReference,
    PhotographedCalibrationGeometryAxis::CaptureQuality,
];

fn complete_report(
) -> PhotographedCalibrationGeometryEvidence<&'static str, &'static str> {
    PhotographedCalibrationGeometryEvidence {
        capture_identity: "capture-17",
        observations: AXES
            .into_iter()
            .enumerate()
            .map(|(index, axis)| PhotographedCalibrationGeometryObservation {
                axis,
                evidence: match index {
                    0 => "reference-mark-evidence",
                    1 => "perspective-evidence",
                    2 => "lens-distortion-evidence",
                    3 => "physical-scale-evidence",
                    4 => "grid-registration-evidence",
                    5 => "baseline-reference-evidence",
                    _ => "capture-quality-evidence",
                },
            })
            .collect(),
    }
}

#[test]
fn complete_report_retains_capture_identity_and_all_seven_evidence_families() {
    let report = complete_report();
    assert_eq!(report.capture_identity, "capture-17");
    assert_eq!(report.observations.len(), 7);
    assert_eq!(
        validate_photographed_calibration_geometry_evidence(&report),
        Ok(()),
    );
    let validated =
        validate_photographed_calibration_geometry_evidence_view(&report)
            .expect("complete report seals evidence");
    assert!(std::ptr::eq(validated.report(), &report));
    assert_eq!(
        report.observations[0].evidence,
        "reference-mark-evidence",
    );
    assert_eq!(report.observations[6].evidence, "capture-quality-evidence");
}

#[test]
fn every_duplicate_axis_rejects_before_missing_axis_evaluation() {
    for duplicate in AXES {
        let mut report = complete_report();
        let duplicate_observation = report
            .observations
            .iter()
            .find(|item| item.axis == duplicate)
            .cloned()
            .expect("complete fixture contains every axis");
        let missing_index = report
            .observations
            .iter()
            .position(|item| item.axis != duplicate)
            .expect("seven-axis fixture always has another axis");
        report.observations.remove(missing_index);
        report.observations.insert(2, duplicate_observation);
        assert_eq!(
            validate_photographed_calibration_geometry_evidence(&report),
            Err(
                PhotographedCalibrationGeometryEvidenceError::DuplicateAxis {
                    axis: duplicate,
                },
            ),
            "duplicate axis {duplicate:?}",
        );
    }
}

#[test]
fn all_128_presence_masks_match_independent_first_missing_axis_oracle() {
    let mut cases = 0_u16;
    let mut saw_complete = false;
    let mut saw_each_missing = [false; 7];
    for mask in 0_u8..128 {
        let report = PhotographedCalibrationGeometryEvidence {
            capture_identity: 17_u8,
            observations: AXES
                .into_iter()
                .enumerate()
                .filter_map(|(index, axis)| {
                    (mask & (1_u8 << index) != 0).then_some(
                        PhotographedCalibrationGeometryObservation {
                            axis,
                            evidence: index as u8,
                        },
                    )
                })
                .collect(),
        };
        let expected = AXES
            .into_iter()
            .enumerate()
            .find_map(|(index, axis)| {
                (mask & (1_u8 << index) == 0).then_some((index, axis))
            });
        let expected_result = match expected {
            Some((index, axis)) => {
                saw_each_missing[index] = true;
                Err(PhotographedCalibrationGeometryEvidenceError::MissingAxis {
                    axis,
                })
            },
            None => {
                saw_complete = true;
                Ok(())
            },
        };
        assert_eq!(
            validate_photographed_calibration_geometry_evidence(&report),
            expected_result,
            "presence mask {mask:#09b}",
        );
        match expected_result {
            Ok(()) => {
                let validated =
                    validate_photographed_calibration_geometry_evidence_view(
                        &report,
                    )
                    .expect("complete mask seals exact report");
                assert!(std::ptr::eq(validated.report(), &report));
            },
            Err(reason) => assert_eq!(
                validate_photographed_calibration_geometry_evidence_view(
                    &report,
                ),
                Err(reason),
                "sealed presence mask {mask:#09b}",
            ),
        }
        cases = cases.saturating_add(1);
    }
    assert_eq!(cases, 128);
    assert!(saw_complete);
    assert!(saw_each_missing.into_iter().all(|seen| seen));
}
#[test]
fn all_2801_short_axis_sequences_match_duplicate_then_missing_oracle() {
    let mut cases = 0_u32;
    let mut saw_duplicate = false;
    let mut saw_missing = false;
    for length in 0_u32..=4 {
        for encoded in 0_u32..7_u32.pow(length) {
            let mut state = encoded;
            let mut sequence = Vec::new();
            for _ in 0..length {
                let axis = AXES[(state % 7) as usize];
                state /= 7;
                sequence.push(axis);
            }
            let report = PhotographedCalibrationGeometryEvidence {
                capture_identity: 17_u8,
                observations: sequence
                    .iter()
                    .copied()
                    .enumerate()
                    .map(|(index, axis)| {
                        PhotographedCalibrationGeometryObservation {
                            axis,
                            evidence: index as u8,
                        }
                    })
                    .collect(),
            };
            let mut expected = None;
            'duplicate: for duplicate_index in 0..sequence.len() {
                for prior in sequence.iter().take(duplicate_index) {
                    if *prior == sequence[duplicate_index] {
                        expected = Some(
                            PhotographedCalibrationGeometryEvidenceError::
                                DuplicateAxis {
                                    axis: sequence[duplicate_index],
                                },
                        );
                        saw_duplicate = true;
                        break 'duplicate;
                    }
                }
            }
            if expected.is_none() {
                let missing = AXES
                    .into_iter()
                    .find(|axis| !sequence.contains(axis))
                    .expect("a sequence of at most four cannot cover seven");
                expected = Some(
                    PhotographedCalibrationGeometryEvidenceError::MissingAxis {
                        axis: missing,
                    },
                );
                saw_missing = true;
            }
            let expected =
                expected.expect("short sequence always has an error");
            assert_eq!(
                validate_photographed_calibration_geometry_evidence(&report),
                Err(expected),
                "length {length}, encoded {encoded}",
            );
            assert_eq!(
                validate_photographed_calibration_geometry_evidence_view(
                    &report,
                ),
                Err(expected),
                "sealed length {length}, encoded {encoded}",
            );
            cases = cases.saturating_add(1);
        }
    }
    assert_eq!(cases, 2_801);
    assert!(saw_duplicate);
    assert!(saw_missing);
}
