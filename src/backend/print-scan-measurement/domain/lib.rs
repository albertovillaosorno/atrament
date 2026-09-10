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
