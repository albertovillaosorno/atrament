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
    CalibrationSampleRoleError, validate_calibration_sample_roles,
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
