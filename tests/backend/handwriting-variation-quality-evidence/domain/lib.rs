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
//   - Regression evidence for independent handwriting variation-quality axes.
// - Must-Not:
//   - Sample variation, calculate statistics, choose thresholds, infer
//     findings,
//     define perceptual metrics, render handwriting, or choose release policy.
// - Allows:
//   - Inputs: Deterministic caller-owned per-axis artifact findings.
//   - Outputs: Assertions over completeness, duplicates, and detected axes.
//   - Side effects: Process-local test allocation only.
// - Split-When:
//   - Axis-specific statistical or perceptual comparators gain fixtures.
// - Merge-When:
//   - Evidence completeness moves into a variation validation harness.
// - Summary:
//   - Proves every accepted variation artifact risk remains independently seen.
// - Description:
//   - Covers TODO and ADR artifact axes without implementing their detectors.
// - Usage:
//   - Compile directly against the variation-quality-evidence domain.
// - Defaults:
//   - Findings are supplied by fixtures rather than calculated here.
//
use atrament_handwriting_variation_quality_evidence::{
    HandwritingVariationQualityAxis, HandwritingVariationQualityEvidence,
    HandwritingVariationQualityEvidenceError,
    HandwritingVariationQualityFinding,
    detected_handwriting_variation_artifacts,
    validate_handwriting_variation_quality_evidence,
    validate_handwriting_variation_quality_evidence_view,
};

const AXES: [HandwritingVariationQualityAxis; 7] = [
    HandwritingVariationQualityAxis::CalibratedEnvelope,
    HandwritingVariationQualityAxis::CorrelationRetention,
    HandwritingVariationQualityAxis::ExtremeLegibility,
    HandwritingVariationQualityAxis::FrozenContours,
    HandwritingVariationQualityAxis::LocalWhiteNoise,
    HandwritingVariationQualityAxis::MechanicalBaselines,
    HandwritingVariationQualityAxis::WordRhythmRepetition,
];

fn complete_evidence() -> Vec<HandwritingVariationQualityEvidence> {
    AXES
        .into_iter()
        .map(|axis| HandwritingVariationQualityEvidence {
            axis,
            finding: HandwritingVariationQualityFinding::ArtifactNotDetected,
        })
        .collect()
}

#[test]
fn every_todo_and_adr_artifact_axis_is_required_independently() {
    let evidence = complete_evidence();
    assert_eq!(evidence.len(), 7);
    assert_eq!(
        validate_handwriting_variation_quality_evidence(&evidence),
        Ok(()),
    );
    let validated =
        validate_handwriting_variation_quality_evidence_view(&evidence)
            .expect("complete evidence seals exact report");
    assert!(std::ptr::eq(validated.evidence(), evidence.as_slice()));
    assert!(validated.detected_artifacts().is_empty());
    assert_eq!(
        detected_handwriting_variation_artifacts(&evidence),
        Ok(vec![]),
    );
}

#[test]
fn missing_axis_rejects_when_other_axes_have_no_detected_artifact() {
    for missing in AXES {
        let mut evidence = complete_evidence();
        evidence.retain(|item| item.axis != missing);
        let expected =
            HandwritingVariationQualityEvidenceError::MissingAxis {
                axis: missing,
            };
        assert_eq!(
            validate_handwriting_variation_quality_evidence(&evidence),
            Err(expected),
            "missing axis {missing:?}",
        );
        assert_eq!(
            validate_handwriting_variation_quality_evidence_view(&evidence),
            Err(expected),
            "sealed missing axis {missing:?}",
        );
    }
}

#[test]
fn each_duplicate_axis_rejects_before_completeness_is_claimed() {
    for duplicate in AXES {
        let mut evidence = complete_evidence();
        let duplicate_item = evidence
            .iter()
            .find(|item| item.axis == duplicate)
            .copied()
            .expect("complete fixture contains every required axis");
        evidence.insert(3, duplicate_item);
        let expected =
            HandwritingVariationQualityEvidenceError::DuplicateAxis {
                axis: duplicate,
            };
        assert_eq!(
            validate_handwriting_variation_quality_evidence(&evidence),
            Err(expected),
            "duplicate axis {duplicate:?}",
        );
        assert_eq!(
            validate_handwriting_variation_quality_evidence_view(&evidence),
            Err(expected),
            "sealed duplicate axis {duplicate:?}",
        );
    }
}

#[test]
fn all_128_finding_masks_return_exact_detected_axes_in_canonical_order() {
    let mut cases = 0_u16;
    let mut saw_axis_detected = [false; 7];
    for mask in 0_u8..128 {
        let mut evidence = AXES
            .into_iter()
            .enumerate()
            .map(|(index, axis)| HandwritingVariationQualityEvidence {
                axis,
                finding: if mask & (1_u8 << index) == 0 {
                    HandwritingVariationQualityFinding::ArtifactNotDetected
                } else {
                    saw_axis_detected[index] = true;
                    HandwritingVariationQualityFinding::ArtifactDetected
                },
            })
            .collect::<Vec<_>>();
        evidence.reverse();
        let expected = AXES
            .into_iter()
            .enumerate()
            .filter_map(|(index, axis)| {
                (mask & (1_u8 << index) != 0).then_some(axis)
            })
            .collect::<Vec<_>>();
        assert_eq!(
            detected_handwriting_variation_artifacts(&evidence),
            Ok(expected.clone()),
            "finding mask {mask:#09b}",
        );
        let validated =
            validate_handwriting_variation_quality_evidence_view(&evidence)
                .expect("every finding mask remains structurally complete");
        assert!(std::ptr::eq(validated.evidence(), evidence.as_slice()));
        assert_eq!(
            validated.detected_artifacts(),
            expected,
            "sealed finding mask {mask:#09b}",
        );
        cases = cases.saturating_add(1);
    }
    assert_eq!(cases, 128);
    assert!(saw_axis_detected.into_iter().all(|seen| seen));
}
