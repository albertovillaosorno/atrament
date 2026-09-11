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
//   - Structural completeness of photographed handwriting-calibration geometry
//     evidence.
// - Must-Not:
//   - Detect marks, decode images, fit transforms, correct perspective or lens
//     distortion, choose units/tolerances, score capture quality, or extract
//     handwriting.
// - Allows:
//   - Inputs: One caller-owned capture identity and independently produced
//     geometry-evidence observations.
//   - Outputs: Exact evidence retention and completeness validation.
//   - Side effects: Process-local validation allocation only.
// - Split-When:
//   - Mark detection, transform fitting, distortion correction, or capture
//     quality gains independent executable authority.
// - Merge-When:
//   - Capture-geometry evidence becomes inseparable from one calibration input
//     adapter.
// - Summary:
//   - Prevents photographed calibration from silently omitting geometry checks.
// - Description:
//   - Requires every TODO-named capture-geometry family without interpreting
//     the caller-produced evidence.
// - Usage:
//   - Validate a complete geometry-evidence report before downstream
//     extraction.
// - Defaults:
//   - No geometry result, correction, threshold, or quality disposition is
//     inferred.
//

//! Structural evidence for photographed handwriting-calibration geometry.

use std::collections::BTreeSet;

const REQUIRED_AXES: [PhotographedCalibrationGeometryAxis; 7] = [
    PhotographedCalibrationGeometryAxis::ReferenceMarks,
    PhotographedCalibrationGeometryAxis::Perspective,
    PhotographedCalibrationGeometryAxis::LensDistortion,
    PhotographedCalibrationGeometryAxis::PhysicalScale,
    PhotographedCalibrationGeometryAxis::GridRegistration,
    PhotographedCalibrationGeometryAxis::BaselineReference,
    PhotographedCalibrationGeometryAxis::CaptureQuality,
];

/// Independent geometry-evidence families required before stroke extraction.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum PhotographedCalibrationGeometryAxis {
    /// Caller-produced baseline-reference evidence.
    BaselineReference,
    /// Caller-produced capture-quality evidence.
    CaptureQuality,
    /// Caller-produced grid-registration evidence.
    GridRegistration,
    /// Caller-produced lens-distortion evidence.
    LensDistortion,
    /// Caller-produced perspective evidence.
    Perspective,
    /// Caller-produced physical-scale evidence.
    PhysicalScale,
    /// Caller-produced reference-mark evidence.
    ReferenceMarks,
}

/// One independently produced photographed-calibration geometry observation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PhotographedCalibrationGeometryObservation<Evidence> {
    /// Geometry family represented by this observation.
    pub axis: PhotographedCalibrationGeometryAxis,
    /// Caller-owned measured, detected, or reviewed evidence.
    pub evidence: Evidence,
}

/// Complete geometry-evidence report for one photographed calibration capture.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PhotographedCalibrationGeometryEvidence<CaptureIdentity, Evidence> {
    /// Stable caller-owned identity of the photographed capture.
    pub capture_identity: CaptureIdentity,
    /// Caller-supplied independent observations in report order.
    pub observations: Vec<PhotographedCalibrationGeometryObservation<Evidence>>,
}

/// Structural failure in one photographed-calibration geometry report.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PhotographedCalibrationGeometryEvidenceError {
    /// One geometry family appears more than once.
    DuplicateAxis {
        /// First duplicated family in report order.
        axis: PhotographedCalibrationGeometryAxis,
    },
    /// One required geometry family is absent.
    MissingAxis {
        /// First missing family in canonical requirement order.
        axis: PhotographedCalibrationGeometryAxis,
    },
}

/// Require exactly one observation for every photographed-calibration axis.
///
/// Evidence values remain caller-owned and uninterpreted. Structural
/// completeness does not establish that any measurement passes an external
/// tolerance or that the capture is safe for extraction.
///
/// # Errors
///
/// Returns the first duplicate axis in report order, then the first missing
/// axis in canonical requirement order.
pub fn validate_photographed_calibration_geometry_evidence<
    CaptureIdentity,
    Evidence,
>(
    report: &PhotographedCalibrationGeometryEvidence<CaptureIdentity, Evidence>,
) -> Result<(), PhotographedCalibrationGeometryEvidenceError> {
    let mut seen = BTreeSet::new();
    for observation in &report.observations {
        if !seen.insert(observation.axis) {
            return Err(
                PhotographedCalibrationGeometryEvidenceError::DuplicateAxis {
                    axis: observation.axis,
                },
            );
        }
    }
    for axis in REQUIRED_AXES {
        if !seen.contains(&axis) {
            return Err(
                PhotographedCalibrationGeometryEvidenceError::MissingAxis {
                    axis,
                },
            );
        }
    }
    Ok(())
}
