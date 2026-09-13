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
//   - Structural completeness of handwriting variation-quality evidence.
// - Must-Not:
//   - Sample variation, calculate statistics, choose thresholds, infer artifact
//     findings, define perceptual metrics, render handwriting, or fit models.
// - Allows:
//   - Inputs: Caller-computed independent quality findings for frozen artifact
//     axes.
//   - Outputs: Completeness validation and canonical detected-artifact axes.
//   - Side effects: Process-local result allocation only.
// - Split-When:
//   - One artifact axis gains independently owned statistical or perceptual
//     comparison authority.
// - Merge-When:
//   - Artifact evidence becomes inseparable from one variation test harness.
// - Summary:
//   - Requires every accepted handwriting artifact risk to be checked.
// - Description:
//   - Keeps omission or ordering from hiding a caller-detected variation
//     defect.
// - Usage:
//   - Supply one independently computed finding per required artifact axis.
// - Defaults:
//   - Findings are caller-owned evidence; no detector or release policy exists.
//

//! Structural evidence for handwriting variation artifact validation.

use std::collections::BTreeSet;

const REQUIRED_AXES: [HandwritingVariationQualityAxis; 7] = [
    HandwritingVariationQualityAxis::CalibratedEnvelope,
    HandwritingVariationQualityAxis::CorrelationRetention,
    HandwritingVariationQualityAxis::ExtremeLegibility,
    HandwritingVariationQualityAxis::FrozenContours,
    HandwritingVariationQualityAxis::LocalWhiteNoise,
    HandwritingVariationQualityAxis::MechanicalBaselines,
    HandwritingVariationQualityAxis::WordRhythmRepetition,
];

/// Independent first-release handwriting variation-quality axes.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum HandwritingVariationQualityAxis {
    /// Sampled configuration remains inside the calibrated writer envelope.
    CalibratedEnvelope,
    /// Expected cross-scale or cross-parameter correlation remains present.
    CorrelationRetention,
    /// Authorized extreme configurations remain legible.
    ExtremeLegibility,
    /// Repeated glyphs do not collapse to frozen contour geometry.
    FrozenContours,
    /// Local variation does not become independent white-noise vibration.
    LocalWhiteNoise,
    /// Line baselines do not collapse into mechanically repeated rhythm.
    MechanicalBaselines,
    /// Repeated words do not reuse an identical variation rhythm.
    WordRhythmRepetition,
}

/// Caller-computed disposition for one independent quality axis.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum HandwritingVariationQualityFinding {
    /// Caller-owned evidence detected an artifact for this axis.
    ArtifactDetected,
    /// Caller-owned evidence did not detect an artifact for this axis.
    ArtifactNotDetected,
}

/// One independently computed handwriting variation-quality result.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct HandwritingVariationQualityEvidence {
    /// Independent artifact-risk axis represented by this evidence.
    pub axis: HandwritingVariationQualityAxis,
    /// Caller-computed finding for this axis only.
    pub finding: HandwritingVariationQualityFinding,
}

/// Constructor-sealed evidence that all required variation-quality axes exist.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ValidatedHandwritingVariationQualityEvidence<'evidence> {
    evidence: &'evidence [HandwritingVariationQualityEvidence],
}

impl<'evidence> ValidatedHandwritingVariationQualityEvidence<'evidence> {
    /// Return every caller-detected artifact in canonical required-axis order.
    #[must_use]
    pub fn detected_artifacts(&self) -> Vec<HandwritingVariationQualityAxis> {
        REQUIRED_AXES
            .into_iter()
            .filter(|axis| {
                self.evidence.iter().any(|item| {
                    item.axis == *axis
                        && item.finding
                            == HandwritingVariationQualityFinding::
                                ArtifactDetected
                })
            })
            .collect()
    }

    /// Return the exact caller-owned evidence slice that was admitted.
    #[must_use]
    pub const fn evidence(
        &self,
    ) -> &'evidence [HandwritingVariationQualityEvidence] {
        self.evidence
    }
}

/// Structural failure in one variation-quality evidence report.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum HandwritingVariationQualityEvidenceError {
    /// The same independent artifact-risk axis was reported more than once.
    DuplicateAxis {
        /// Duplicate quality axis.
        axis: HandwritingVariationQualityAxis,
    },
    /// One required independent artifact-risk axis is absent.
    MissingAxis {
        /// Missing quality axis.
        axis: HandwritingVariationQualityAxis,
    },
}

/// Return every caller-detected artifact in canonical axis order.
///
/// # Errors
///
/// Returns the same structural error as
/// [`validate_handwriting_variation_quality_evidence`] before exposing
/// findings.
pub fn detected_handwriting_variation_artifacts(
    evidence: &[HandwritingVariationQualityEvidence],
) -> Result<
    Vec<HandwritingVariationQualityAxis>,
    HandwritingVariationQualityEvidenceError,
> {
    Ok(
        validate_handwriting_variation_quality_evidence_view(evidence)?
            .detected_artifacts(),
    )
}

/// Validate exact evidence and seal borrowed structural completeness.
///
/// # Errors
///
/// Returns exactly the same duplicate-before-missing structural failure as
/// [`validate_handwriting_variation_quality_evidence`].
pub fn validate_handwriting_variation_quality_evidence_view(
    evidence: &[HandwritingVariationQualityEvidence],
) -> Result<
    ValidatedHandwritingVariationQualityEvidence<'_>,
    HandwritingVariationQualityEvidenceError,
> {
    validate_handwriting_variation_quality_evidence(evidence)?;
    Ok(ValidatedHandwritingVariationQualityEvidence { evidence })
}

/// Validate independent coverage of every first-release variation-quality axis.
///
/// A caller-detected artifact is valid evidence and does not become a
/// structural
/// report error. Detection metrics and pass/fail release policy stay outside
/// this domain.
///
/// # Errors
///
/// Returns the first duplicate axis in report order or the first missing axis
/// in
/// canonical required-axis order.
pub fn validate_handwriting_variation_quality_evidence(
    evidence: &[HandwritingVariationQualityEvidence],
) -> Result<(), HandwritingVariationQualityEvidenceError> {
    let mut seen = BTreeSet::new();
    for item in evidence {
        if !seen.insert(item.axis) {
            return Err(HandwritingVariationQualityEvidenceError::DuplicateAxis {
                axis: item.axis,
            });
        }
    }
    for axis in REQUIRED_AXES {
        if !seen.contains(&axis) {
            return Err(HandwritingVariationQualityEvidenceError::MissingAxis {
                axis,
            });
        }
    }
    Ok(())
}
