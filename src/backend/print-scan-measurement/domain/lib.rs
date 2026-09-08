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
//   - Covers ADR-required scale, clipping, grid registration, and photo
//     placement.
// - Usage:
//   - Attach representative-device observations to one rendered output
//     identity.
// - Defaults:
//   - No tolerance, correction, or pass/fail conclusion is inferred.
//

//! Typed print/scan measurement evidence without calibration algorithms.

/// Physical-output measurement families required by the accepted PDF ADR.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum PrintScanMeasurementKind {
    /// Observed clipping against the intended physical page projection.
    Clipping,
    /// Registration of printed or scanned grid/rule geometry.
    GridRegistration,
    /// Placement of accepted photographic content.
    PhotoPlacement,
    /// Physical output scale relative to the intended page geometry.
    PhysicalScale,
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
