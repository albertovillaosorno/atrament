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
//   - Completeness and independence of first-release render-regression
//     evidence.
// - Must-Not:
//   - Render pixels, compare geometry, choose pixel tolerances, hash snapshots,
//     infer themes, normalize layout, or suppress mismatches.
// - Allows:
//   - Inputs: Caller-owned match/mismatch evidence for each frozen regression
//     axis.
//   - Outputs: Structural completeness validation and explicit mismatch axes.
//   - Side effects: Process-local result allocation only.
// - Split-When:
//   - One regression axis gains an independently owned comparison metric.
// - Merge-When:
//   - Render-regression evidence becomes inseparable from one renderer harness.
// - Summary:
//   - Requires every first-release visual axis to be evaluated independently.
// - Description:
//   - Prevents one passing pixel or topology comparison from masking an omitted
//     semantic, theme, layer, seed, bounds, or overlay comparison.
// - Usage:
//   - Supply one caller-computed outcome per axis before accepting golden
//     regression evidence as complete.
// - Defaults:
//   - Mismatches remain evidence; this domain does not choose release policy.
//

//! Structural completeness for independently computed render regressions.

use std::collections::BTreeSet;

const REQUIRED_AXES: [RenderRegressionAxis; 9] = [
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

/// Independent first-release render-regression evidence axes.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum RenderRegressionAxis {
    /// Digital-theme projection comparison.
    DigitalTheme,
    /// Final rendered pixel comparison.
    FinalPixels,
    /// Material-layer composition and order comparison.
    LayerComposition,
    /// Live-theme projection comparison.
    LiveTheme,
    /// Overflow-overlay comparison.
    OverflowOverlay,
    /// Physical page bounds comparison.
    PhysicalBounds,
    /// Deterministic render-seed comparison.
    RenderSeeds,
    /// Semantic-layout comparison before rendering.
    SemanticLayout,
    /// Authoritative vector-topology comparison.
    VectorTopology,
}

/// Caller-computed result for one independent regression axis.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RenderRegressionComparison {
    /// Expected and observed evidence match under the axis-owned comparator.
    Match,
    /// Expected and observed evidence differ under the axis-owned comparator.
    Mismatch,
}

/// One independently computed render-regression result.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RenderRegressionEvidence {
    /// Independent regression axis represented by this result.
    pub axis: RenderRegressionAxis,
    /// Caller-computed comparison outcome for this axis only.
    pub comparison: RenderRegressionComparison,
}

/// Constructor-sealed evidence that every first-release regression axis is
/// represented exactly once.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ValidatedRenderRegressionEvidence<'evidence> {
    evidence: &'evidence [RenderRegressionEvidence],
}

impl<'evidence> ValidatedRenderRegressionEvidence<'evidence> {
    /// Return the exact caller-owned evidence sequence that was admitted.
    #[must_use]
    pub const fn evidence(&self) -> &'evidence [RenderRegressionEvidence] {
        self.evidence
    }

    /// Return reported mismatches in canonical axis order.
    #[must_use]
    pub fn mismatches(&self) -> Vec<RenderRegressionAxis> {
        REQUIRED_AXES
            .into_iter()
            .filter(|axis| {
                self.evidence.iter().any(|item| {
                    item.axis == *axis
                        && item.comparison
                            == RenderRegressionComparison::Mismatch
                })
            })
            .collect()
    }
}

/// Structural failure in one first-release render-regression report.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RenderRegressionEvidenceError {
    /// The same independent regression axis was reported more than once.
    DuplicateAxis {
        /// Duplicate regression axis.
        axis: RenderRegressionAxis,
    },
    /// One required independent regression axis is absent.
    MissingAxis {
        /// Missing regression axis.
        axis: RenderRegressionAxis,
    },
}

/// Validate that one report independently covers every first-release axis.
///
/// A mismatch is valid evidence and is deliberately not converted into a
/// structural report error. Release policy remains outside this domain.
///
/// # Errors
///
/// Returns the first duplicate axis in report order or the first missing axis
/// in the canonical required-axis order.
pub fn validate_render_regression_evidence(
    evidence: &[RenderRegressionEvidence],
) -> Result<(), RenderRegressionEvidenceError> {
    let mut seen = BTreeSet::new();
    for item in evidence {
        if !seen.insert(item.axis) {
            return Err(RenderRegressionEvidenceError::DuplicateAxis {
                axis: item.axis,
            });
        }
    }
    for axis in REQUIRED_AXES {
        if !seen.contains(&axis) {
            return Err(RenderRegressionEvidenceError::MissingAxis { axis });
        }
    }
    Ok(())
}

/// Validate and seal one complete first-release render-regression report.
///
/// # Errors
///
/// Returns the same duplicate-first or canonical missing-axis failure as
/// [`validate_render_regression_evidence`].
pub fn validate_render_regression_evidence_view(
    evidence: &[RenderRegressionEvidence],
) -> Result<
    ValidatedRenderRegressionEvidence<'_>,
    RenderRegressionEvidenceError,
> {
    validate_render_regression_evidence(evidence)?;
    Ok(ValidatedRenderRegressionEvidence { evidence })
}

/// Return every independently reported mismatch in canonical axis order.
///
/// # Errors
///
/// Returns the same structural failure as
/// [`validate_render_regression_evidence`]
/// before producing mismatch evidence.
pub fn render_regression_mismatches(
    evidence: &[RenderRegressionEvidence],
) -> Result<Vec<RenderRegressionAxis>, RenderRegressionEvidenceError> {
    Ok(validate_render_regression_evidence_view(evidence)?.mismatches())
}
