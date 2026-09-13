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
//   - Transport-neutral calibration evidence provenance, sample roles, and
//     held-out quality-report structure.
// - Must-Not:
//   - Define capture geometry, confidence scales, unit vocabularies, extraction
//     algorithms, minimum sample sets, or handwriting synthesis behavior.
// - Allows:
//   - Inputs: Typed caller-owned parameter evidence, calibration sample IDs,
//     held-out measurements, and known failure modes.
//   - Outputs: Preserved evidence, role/report validation, and sample requests.
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
//     writing while retaining source evidence and quality-report provenance.
// - Usage:
//   - Attach caller-owned evidence vocabulary to extracted profile parameters.
// - Defaults:
//   - Underdetermined required behavior requests more evidence rather than
//     silently treating inference as observation.
//

//! Transport-neutral provenance for handwriting calibration evidence.

use std::collections::{BTreeMap, BTreeSet};

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

/// Required evidence dimension in the final held-out quality report.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum HeldOutQualityDimension {
    /// Geometric fidelity to held-out writing.
    Geometry,
    /// Join behavior fidelity to held-out writing.
    Joins,
    /// Perceptual fidelity evidence retained without choosing a scoring model.
    PerceptualFidelity,
    /// Punctuation fidelity to held-out writing.
    Punctuation,
    /// Word and line rhythm fidelity to held-out writing.
    Rhythm,
    /// Spacing fidelity to held-out writing.
    Spacing,
}

/// One quality measurement attributed to one held-out sample.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HeldOutQualityMeasurement<Identity, Value, Evidence> {
    /// Required quality dimension represented by this measurement.
    pub dimension: HeldOutQualityDimension,
    /// Caller-owned measurement provenance or supporting evidence.
    pub evidence: Evidence,
    /// Stable calibration sample identity measured by this observation.
    pub sample: Identity,
    /// Caller-owned measured value, including units when applicable.
    pub value: Value,
}

/// Final held-out calibration-quality evidence before score policy.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HeldOutQualityReport<Identity, Value, Evidence, Failure> {
    /// Caller-owned known failure modes published with this report.
    pub known_failures: Vec<Failure>,
    /// Held-out measurements retained in caller-supplied report order.
    pub measurements: Vec<HeldOutQualityMeasurement<Identity, Value, Evidence>>,
}

/// Constructor-sealed evidence for one structurally admitted held-out report.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ValidatedHeldOutQualityReport<
    'samples,
    'report,
    Identity,
    Value,
    Evidence,
    Failure,
> {
    report: &'report HeldOutQualityReport<Identity, Value, Evidence, Failure>,
    samples: &'samples [CalibrationSample<Identity>],
}

impl<'samples, 'report, Identity, Value, Evidence, Failure>
    ValidatedHeldOutQualityReport<
        'samples,
        'report,
        Identity,
        Value,
        Evidence,
        Failure,
    >
{
    /// Return the exact admitted quality report.
    #[must_use]
    pub const fn report(
        &self,
    ) -> &'report HeldOutQualityReport<Identity, Value, Evidence, Failure> {
        self.report
    }

    /// Return the exact sample-role declarations used for admission.
    #[must_use]
    pub const fn samples(&self) -> &'samples [CalibrationSample<Identity>] {
        self.samples
    }
}

/// Result of admitting one exact held-out quality report.
pub type HeldOutQualityReportValidationResult<
    'samples,
    'report,
    Identity,
    Value,
    Evidence,
    Failure,
> = Result<
    ValidatedHeldOutQualityReport<
        'samples,
        'report,
        Identity,
        Value,
        Evidence,
        Failure,
    >,
    HeldOutQualityReportError<Identity>,
>;

/// Why held-out quality evidence cannot be admitted structurally.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum HeldOutQualityReportError<Identity> {
    /// The underlying calibration sample-role declarations conflict.
    ConflictingSampleRole {
        /// Sample identity that appears in both evidence roles.
        sample: Identity,
    },
    /// One of the six required report dimensions has no measurement.
    MissingDimension {
        /// First missing dimension in the accepted requirement order.
        dimension: HeldOutQualityDimension,
    },
    /// A report measurement references a sample reserved for training.
    TrainingSample {
        /// Training sample incorrectly used for held-out quality evidence.
        sample: Identity,
    },
    /// A report measurement references no declared calibration sample.
    UnknownSample {
        /// Unknown sample identity retained exactly for diagnostics.
        sample: Identity,
    },
}

/// Constructor-sealed evidence that one exact sample declaration sequence
/// preserves training/held-out separation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ValidatedCalibrationSampleRoles<'samples, Identity> {
    samples: &'samples [CalibrationSample<Identity>],
}

impl<'samples, Identity> ValidatedCalibrationSampleRoles<'samples, Identity> {
    /// Return the exact caller-owned sample declarations that were admitted.
    #[must_use]
    pub const fn samples(&self) -> &'samples [CalibrationSample<Identity>] {
        self.samples
    }
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

/// Validate one exact sample declaration sequence and seal its role evidence.
///
/// # Errors
///
/// Returns the same first caller-order conflict as
/// [`validate_calibration_sample_roles`].
pub fn validate_calibration_sample_roles_view<Identity>(
    samples: &[CalibrationSample<Identity>],
) -> Result<ValidatedCalibrationSampleRoles<'_, Identity>,
    CalibrationSampleRoleError<Identity>>
where
    Identity: Clone + Ord,
{
    validate_calibration_sample_roles(samples)?;
    Ok(ValidatedCalibrationSampleRoles { samples })
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
/// Validate that a final quality report is driven only by held-out writing.
///
/// This structural boundary requires measurements for geometry, rhythm, joins,
/// spacing, punctuation, and perceptual fidelity. It does not choose metric
/// units, thresholds, aggregation, perceptual models, or a passing score.
/// Repeated measurements remain caller-owned evidence rather than being
/// deduplicated or averaged here.
///
/// # Errors
///
/// Returns a conflicting sample-role declaration first. Otherwise returns the
/// first invalid measurement sample in report order, then the first missing
/// required quality dimension.
pub fn validate_held_out_quality_report<Identity, Value, Evidence, Failure>(
    samples: &[CalibrationSample<Identity>],
    report: &HeldOutQualityReport<Identity, Value, Evidence, Failure>,
) -> Result<(), HeldOutQualityReportError<Identity>>
where
    Identity: Clone + Ord,
{
    validate_calibration_sample_roles(samples).map_err(|error| match error {
        CalibrationSampleRoleError::ConflictingRole { sample } => {
            HeldOutQualityReportError::ConflictingSampleRole { sample }
        },
    })?;

    let roles = samples
        .iter()
        .map(|sample| (sample.identity.clone(), sample.role))
        .collect::<BTreeMap<_, _>>();
    let mut dimensions = BTreeSet::new();
    for measurement in &report.measurements {
        match roles.get(&measurement.sample) {
            Some(CalibrationSampleRole::HeldOut) => {},
            Some(CalibrationSampleRole::Training) => {
                return Err(HeldOutQualityReportError::TrainingSample {
                    sample: measurement.sample.clone(),
                });
            },
            None => {
                return Err(HeldOutQualityReportError::UnknownSample {
                    sample: measurement.sample.clone(),
                });
            },
        }
        let _inserted = dimensions.insert(measurement.dimension);
    }

    let required_dimensions = [
        HeldOutQualityDimension::Geometry,
        HeldOutQualityDimension::Rhythm,
        HeldOutQualityDimension::Joins,
        HeldOutQualityDimension::Spacing,
        HeldOutQualityDimension::Punctuation,
        HeldOutQualityDimension::PerceptualFidelity,
    ];
    for dimension in required_dimensions {
        if !dimensions.contains(&dimension) {
            return Err(HeldOutQualityReportError::MissingDimension {
                dimension,
            });
        }
    }
    Ok(())
}

/// Validate one exact held-out report and seal borrowed admission evidence.
///
/// The returned value proves only structural sample-role separation, held-out
/// measurement attribution, and six-dimension completeness. It does not choose
/// units, aggregation, thresholds, quality scores, or pass/fail policy.
///
/// # Errors
///
/// Returns exactly the same role-conflict, measurement-reference, then
/// missing-dimension precedence as [`validate_held_out_quality_report`].
pub fn validate_held_out_quality_report_view<
    'samples,
    'report,
    Identity,
    Value,
    Evidence,
    Failure,
>(
    samples: &'samples [CalibrationSample<Identity>],
    report: &'report HeldOutQualityReport<Identity, Value, Evidence, Failure>,
) -> HeldOutQualityReportValidationResult<
    'samples,
    'report,
    Identity,
    Value,
    Evidence,
    Failure,
>
where
    Identity: Clone + Ord,
{
    validate_held_out_quality_report(samples, report)?;
    Ok(ValidatedHeldOutQualityReport { report, samples })
}
