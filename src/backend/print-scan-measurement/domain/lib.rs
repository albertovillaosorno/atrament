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
//   - Transport-neutral print/scan measurement evidence required by output ADR.
// - Must-Not:
//   - Choose devices, units, tolerances, calibration algorithms, correction
//     transforms, acceptance thresholds, scanner models, or renderer behavior.
// - Allows:
//   - Inputs: Caller-owned output identity, measurement source, unit, and
//     value.
//   - Outputs: Typed print/scan measurement evidence in caller-owned order.
//   - Side effects: None.
// - Split-When:
//   - Calibration fitting or correction transforms gain executable authority.
// - Merge-When:
//   - Measurement evidence becomes inseparable from one calibration workflow.
// - Summary:
//   - Makes required physical-output measurements inspectable before fitting.
// - Description:
//   - Covers the complete P5 physical round-trip measurement vocabulary.
// - Usage:
//   - Attach representative-device observations to one rendered output
//     identity.
// - Defaults:
//   - No tolerance, correction, or pass/fail conclusion is inferred.
//

//! Typed print/scan measurement evidence without calibration algorithms.

use std::collections::BTreeSet;

const REQUIRED_MEASUREMENT_KINDS: [PrintScanMeasurementKind; 8] = [
    PrintScanMeasurementKind::PhysicalScale,
    PrintScanMeasurementKind::Clipping,
    PrintScanMeasurementKind::Margins,
    PrintScanMeasurementKind::GridRegistration,
    PrintScanMeasurementKind::ColorShift,
    PrintScanMeasurementKind::PhotoPlacement,
    PrintScanMeasurementKind::LineWeight,
    PrintScanMeasurementKind::ScannerDistortion,
];

/// Physical-output measurement families required by the first-release task.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum PrintScanMeasurementKind {
    /// Observed clipping against the intended physical page projection.
    Clipping,
    /// Color shift observed across the physical print/scan round trip.
    ColorShift,
    /// Registration of printed or scanned grid/rule geometry.
    GridRegistration,
    /// Observed physical line-weight behavior.
    LineWeight,
    /// Physical page-margin observation.
    Margins,
    /// Placement of accepted photographic content.
    PhotoPlacement,
    /// Physical output scale relative to the intended page geometry.
    PhysicalScale,
    /// Scanner-introduced geometric distortion observation.
    ScannerDistortion,
}

/// One caller-owned physical-output observation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PrintScanMeasurement<Source, Unit, Value> {
    /// Measurement family.
    pub kind: PrintScanMeasurementKind,
    /// Caller-owned device/capture/provenance source.
    pub source: Source,
    /// Caller-owned measurement unit.
    pub unit: Unit,
    /// Caller-owned measured value or structured observation.
    pub value: Value,
}

/// Measurement evidence associated with one rendered or exported output.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PrintScanEvidence<OutputIdentity, Measurement> {
    /// Measurements in caller-owned deterministic observation order.
    pub measurements: Vec<Measurement>,
    /// Render/output identity whose physical round trip was measured.
    pub output_identity: OutputIdentity,
}

/// Why one physical round-trip evidence set is structurally incomplete.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PrintScanEvidenceError {
    /// One required first-release measurement family has no observation.
    MissingMeasurement {
        /// First missing family in canonical requirement order.
        kind: PrintScanMeasurementKind,
    },
}

/// Constructor-sealed evidence that all eight physical measurement families
/// are represented for one exact output identity.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ValidatedPrintScanEvidence<
    'evidence,
    OutputIdentity,
    Source,
    Unit,
    Value,
> {
    evidence: &'evidence PrintScanEvidence<
        OutputIdentity,
        PrintScanMeasurement<Source, Unit, Value>,
    >,
}

impl<'evidence, OutputIdentity, Source, Unit, Value>
    ValidatedPrintScanEvidence<'evidence, OutputIdentity, Source, Unit, Value>
{
    /// Return the exact caller-owned evidence set that was admitted.
    #[must_use]
    pub const fn evidence(
        &self,
    ) -> &'evidence PrintScanEvidence<
        OutputIdentity,
        PrintScanMeasurement<Source, Unit, Value>,
    > {
        self.evidence
    }
}

/// Require at least one observation for every first-release round-trip family.
///
/// Repeated measurements remain valid caller-owned evidence. This structural
/// gate does not choose devices, aggregate observations, apply tolerances, or
/// infer a calibration result.
///
/// # Errors
///
/// Returns the first absent family in canonical requirement order.
pub fn validate_print_scan_evidence<OutputIdentity, Source, Unit, Value>(
    evidence: &PrintScanEvidence<
        OutputIdentity,
        PrintScanMeasurement<Source, Unit, Value>,
    >,
) -> Result<(), PrintScanEvidenceError> {
    let seen = evidence
        .measurements
        .iter()
        .map(|measurement| measurement.kind)
        .collect::<BTreeSet<_>>();
    for kind in REQUIRED_MEASUREMENT_KINDS {
        if !seen.contains(&kind) {
            return Err(PrintScanEvidenceError::MissingMeasurement { kind });
        }
    }
    Ok(())
}

/// Validate and seal one exact complete physical round-trip evidence set.
///
/// # Errors
///
/// Returns the same first missing family as [`validate_print_scan_evidence`].
pub fn validate_print_scan_evidence_view<OutputIdentity, Source, Unit, Value>(
    evidence: &PrintScanEvidence<
        OutputIdentity,
        PrintScanMeasurement<Source, Unit, Value>,
    >,
) -> Result<
    ValidatedPrintScanEvidence<'_, OutputIdentity, Source, Unit, Value>,
    PrintScanEvidenceError,
> {
    validate_print_scan_evidence(evidence)?;
    Ok(ValidatedPrintScanEvidence { evidence })
}
