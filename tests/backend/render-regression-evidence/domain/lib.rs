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

fn complete_evidence() -> Vec<RenderRegressionEvidence> {
    [
        RenderRegressionAxis::DigitalTheme,
        RenderRegressionAxis::FinalPixels,
        RenderRegressionAxis::LayerComposition,
        RenderRegressionAxis::LiveTheme,
        RenderRegressionAxis::OverflowOverlay,
        RenderRegressionAxis::PhysicalBounds,
        RenderRegressionAxis::RenderSeeds,
        RenderRegressionAxis::SemanticLayout,
        RenderRegressionAxis::VectorTopology,
    ]
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
fn missing_axis_rejects_even_when_final_pixels_match() {
    let mut evidence = complete_evidence();
    evidence.retain(|item| item.axis != RenderRegressionAxis::VectorTopology);
    assert_eq!(
        validate_render_regression_evidence(&evidence),
        Err(RenderRegressionEvidenceError::MissingAxis {
            axis: RenderRegressionAxis::VectorTopology,
        }),
    );
    assert!(evidence.iter().any(|item| {
        item.axis == RenderRegressionAxis::FinalPixels
            && item.comparison == RenderRegressionComparison::Match
    }));
}

#[test]
fn duplicate_axis_rejects_before_completeness_is_claimed() {
    let mut evidence = complete_evidence();
    evidence.insert(4, evidence[0]);
    assert_eq!(
        validate_render_regression_evidence(&evidence),
        Err(RenderRegressionEvidenceError::DuplicateAxis {
            axis: RenderRegressionAxis::DigitalTheme,
        }),
    );
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
