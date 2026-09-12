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
//   - Regression evidence for complete independent visual-regression axes.
// - Must-Not:
//   - Render, compare pixels or geometry, choose tolerances, hash snapshots, or
//     convert mismatches into release policy.
// - Allows:
//   - Inputs: Deterministic caller-owned per-axis comparison outcomes.
//   - Outputs: Assertions over completeness, duplicates, and mismatches.
//   - Side effects: Process-local test allocation only.
// - Split-When:
//   - Axis-specific comparators gain independent fixtures.
// - Merge-When:
//   - Evidence completeness moves into a renderer regression harness.
// - Summary:
//   - Pins independent coverage of every first-release visual-regression axis.
// - Description:
//   - Proves omitted axes cannot hide behind a passing final-pixel comparison.
// - Usage:
//   - Compile directly against the render-regression-evidence domain.
// - Defaults:
//   - Comparison outcomes are supplied, not calculated, by this fixture.
//
use atrament_render_regression_evidence::{
    RenderRegressionAxis, RenderRegressionComparison, RenderRegressionEvidence,
    RenderRegressionEvidenceError, render_regression_mismatches,
    validate_render_regression_evidence,
};

const AXES: [RenderRegressionAxis; 9] = [
    RenderRegressionAxis::DigitalTheme,
    RenderRegressionAxis::FinalPixels,
    RenderRegressionAxis::LayerComposition,
    RenderRegressionAxis::LiveTheme,
    RenderRegressionAxis::OverflowOverlay,
    RenderRegressionAxis::PhysicalBounds,
    RenderRegressionAxis::RenderSeeds,
    RenderRegressionAxis::SemanticLayout,
    RenderRegressionAxis::VectorTopology,
];

fn complete_evidence() -> Vec<RenderRegressionEvidence> {
    AXES
        .into_iter()
    .map(|axis| RenderRegressionEvidence {
        axis,
        comparison: RenderRegressionComparison::Match,
    })
    .collect()
}

#[test]
fn every_first_release_regression_axis_is_required_independently() {
    let evidence = complete_evidence();
    assert_eq!(evidence.len(), 9);
    assert_eq!(validate_render_regression_evidence(&evidence), Ok(()));
    assert_eq!(render_regression_mismatches(&evidence), Ok(vec![]));
}

#[test]
fn every_axis_is_independently_required_even_when_other_axes_match() {
    for missing in AXES {
        let mut evidence = complete_evidence();
        evidence.retain(|item| item.axis != missing);
        assert_eq!(
            validate_render_regression_evidence(&evidence),
            Err(RenderRegressionEvidenceError::MissingAxis { axis: missing }),
            "missing axis {missing:?}",
        );
        if missing != RenderRegressionAxis::FinalPixels {
            assert!(evidence.iter().any(|item| {
                item.axis == RenderRegressionAxis::FinalPixels
                    && item.comparison == RenderRegressionComparison::Match
            }));
        }
    }
}

#[test]
fn every_duplicate_axis_rejects_before_completeness_is_claimed() {
    for duplicate in AXES {
        let mut evidence = complete_evidence();
        let item = evidence
            .iter()
            .find(|item| item.axis == duplicate)
            .copied()
            .expect("complete evidence contains every axis");
        evidence.insert(4, item);
        assert_eq!(
            validate_render_regression_evidence(&evidence),
            Err(RenderRegressionEvidenceError::DuplicateAxis {
                axis: duplicate,
            }),
            "duplicate axis {duplicate:?}",
        );
    }
}

#[test]
fn mismatches_remain_independent_and_canonically_ordered() {
    let mut evidence = complete_evidence();
    for item in &mut evidence {
        if matches!(
            item.axis,
            RenderRegressionAxis::LayerComposition
                | RenderRegressionAxis::OverflowOverlay
                | RenderRegressionAxis::VectorTopology
        ) {
            item.comparison = RenderRegressionComparison::Mismatch;
        }
    }
    evidence.reverse();
    assert_eq!(
        render_regression_mismatches(&evidence),
        Ok(vec![
            RenderRegressionAxis::LayerComposition,
            RenderRegressionAxis::OverflowOverlay,
            RenderRegressionAxis::VectorTopology,
        ]),
    );
}

#[test]
fn every_match_mismatch_mask_projects_exact_canonical_axes() {
    const MASKS: usize = 1 << AXES.len();
    let mut saw_match = [false; AXES.len()];
    let mut saw_mismatch = [false; AXES.len()];
    let mut cases = 0_usize;
    for mask in 0..MASKS {
        let mut evidence = AXES
            .into_iter()
            .enumerate()
            .map(|(index, axis)| {
                let comparison = if mask & (1 << index) == 0 {
                    saw_match[index] = true;
                    RenderRegressionComparison::Match
                } else {
                    saw_mismatch[index] = true;
                    RenderRegressionComparison::Mismatch
                };
                RenderRegressionEvidence { axis, comparison }
            })
            .collect::<Vec<_>>();
        evidence.reverse();
        let expected = AXES
            .into_iter()
            .enumerate()
            .filter_map(|(index, axis)| {
                (mask & (1 << index) != 0).then_some(axis)
            })
            .collect::<Vec<_>>();
        assert_eq!(
            validate_render_regression_evidence(&evidence),
            Ok(()),
            "structural completeness for mask {mask:#011b}",
        );
        assert_eq!(
            render_regression_mismatches(&evidence),
            Ok(expected),
            "canonical mismatch projection for mask {mask:#011b}",
        );
        cases += 1;
    }
    assert_eq!(cases, MASKS);
    assert!(saw_match.into_iter().all(|seen| seen));
    assert!(saw_mismatch.into_iter().all(|seen| seen));
}

#[test]
fn every_mismatched_axis_is_returned_in_canonical_order() {
    let mut evidence = complete_evidence();
    for item in &mut evidence {
        item.comparison = RenderRegressionComparison::Mismatch;
    }
    evidence.reverse();
    assert_eq!(render_regression_mismatches(&evidence), Ok(AXES.to_vec()));
}
