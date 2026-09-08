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
//   - Transport-neutral calibration evidence provenance and sample roles.
// - Must-Not:
//   - Define capture geometry, confidence scales, unit vocabularies, extraction
//     algorithms, minimum sample sets, or handwriting synthesis behavior.
// - Allows:
//   - Inputs: Typed caller-owned parameter evidence and calibration sample IDs.
//   - Outputs: Preserved evidence records, role conflicts, and sample requests.
//   - Side effects: Process-local validation allocation only.
// - Split-When:
//   - Capture geometry, extraction, or calibration workflow gains independent
//     application authority.
// - Merge-When:
//   - Calibration provenance becomes inseparable from a broader profile domain.
// - Summary:
//   - Preserves calibration evidence status without inventing measurement
//     policy.
// - Description:
//   - Separates observations from inferred extremes and training from held-out
//     writing while retaining source, unit, confidence, and correction
//     evidence.
// - Usage:
//   - Attach caller-owned evidence vocabulary to extracted profile parameters.
// - Defaults:
//   - Underdetermined required behavior requests more evidence rather than
//     silently treating inference as observation.
//

//! Transport-neutral provenance for handwriting calibration evidence.

use std::collections::BTreeMap;

/// Evidentiary basis of one extracted calibration parameter.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum CalibrationEvidenceBasis {
    /// Parameter value is an inferred extreme, not an observation.
    InferredExtreme,
    /// Parameter value comes from observed calibration evidence.
    Observed,
}

/// Whether one required calibration behavior is sufficiently determined.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum CalibrationDetermination {
    /// Existing evidence determines the required behavior.
    Determined,
    /// Existing evidence does not determine the required behavior.
    Underdetermined,
}

impl CalibrationDetermination {
    /// Whether the accepted calibration decision requires another sample.
    #[must_use]
    pub const fn requires_additional_sample(self) -> bool {
        matches!(self, Self::Underdetermined)
    }
}

/// One extracted parameter with all evidence required by calibration policy.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CalibrationParameterEvidence<
    Parameter,
    Value,
    SourceRegion,
    Unit,
    Confidence,
    Correction,
> {
    /// Accepted correction history retained in order.
    pub accepted_corrections: Vec<Correction>,
    /// Whether this value is observed evidence or an inferred extreme.
    pub basis: CalibrationEvidenceBasis,
    /// Caller-owned confidence value retained without reinterpretation.
    pub confidence: Confidence,
    /// Caller-owned parameter identity.
    pub parameter: Parameter,
    /// Caller-owned source region from which the value was extracted.
    pub source_region: SourceRegion,
    /// Caller-owned measurement unit.
    pub unit: Unit,
    /// Extracted parameter value in its declared unit.
    pub value: Value,
}

/// Role of one complete writing sample in calibration and validation.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum CalibrationSampleRole {
    /// Sample is excluded from extraction and reserved for quality validation.
    HeldOut,
    /// Sample contributes to calibration or parameter extraction.
    Training,
}

/// One calibration sample identity with its accepted evidence role.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CalibrationSample<Identity> {
    /// Caller-owned stable sample identity.
    pub identity: Identity,
    /// Whether the sample participates in extraction or held-out validation.
    pub role: CalibrationSampleRole,
}

/// Why calibration sample roles violate held-out separation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CalibrationSampleRoleError<Identity> {
    /// One sample identity appears as both training and held-out writing.
    ConflictingRole {
        /// Conflicting caller-owned sample identity.
        sample: Identity,
    },
}

/// Validate that held-out writing never participates in training evidence.
///
/// Repeated declarations with the same role are left to the owning workflow;
/// only cross-role reuse violates the accepted calibration evidence boundary.
///
/// # Errors
///
/// Returns the first sample identity observed with conflicting roles.
pub fn validate_calibration_sample_roles<Identity>(
    samples: &[CalibrationSample<Identity>],
) -> Result<(), CalibrationSampleRoleError<Identity>>
where
    Identity: Clone + Ord,
{
    let mut roles = BTreeMap::new();
    for sample in samples {
        if let Some(existing) = roles.get(&sample.identity) {
            if *existing != sample.role {
                return Err(CalibrationSampleRoleError::ConflictingRole {
                    sample: sample.identity.clone(),
                });
            }
        } else {
            let _previous = roles.insert(sample.identity.clone(), sample.role);
        }
    }
    Ok(())
}
