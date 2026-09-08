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
//   - Transport-neutral physical device coordinate-calibration evidence.
// - Must-Not:
//   - Measure hardware, choose units/tolerances, infer axis conventions,
//     perform
//     transforms, validate machine limits, arm hardware, or authorize motion.
// - Allows:
//   - Inputs: Caller-owned device/calibration identities and measured evidence.
//   - Outputs: One inspectable grouped calibration record.
//   - Side effects: None.
// - Split-When:
//   - Coordinate transform math or hardware measurement gains executable
//     authority.
// - Merge-When:
//   - Calibration evidence becomes inseparable from one hardware adapter.
// - Summary:
//   - Preserves machine calibration inputs without claiming calibration math.
// - Description:
//   - Separates coordinate, pen-motion, and blank-sheet registration evidence.
// - Usage:
//   - Bind an identified device to measured calibration before dry-run checks.
// - Defaults:
//   - No units, safe thresholds, coordinate conventions, or validity are
//     inferred.
//

//! Physical device coordinate-calibration evidence without hardware authority.

/// Measured transform evidence between page and device coordinate spaces.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CoordinateTransformCalibration<
    AxisOrientation,
    Origin,
    Scale,
    Skew,
> {
    /// Caller-owned axis orientation measurement or convention evidence.
    pub axis_orientation: AxisOrientation,
    /// Caller-owned device/page origin evidence.
    pub origin: Origin,
    /// Caller-owned physical scale evidence.
    pub scale: Scale,
    /// Caller-owned skew evidence.
    pub skew: Skew,
}

/// Measured pen and motion envelope for one identified device setup.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PenMotionCalibration<
    Acceleration,
    ContactHeight,
    PenUpHeight,
    Speed,
> {
    /// Caller-owned admitted acceleration evidence.
    pub acceleration: Acceleration,
    /// Caller-owned physical contact-height evidence.
    pub contact_height: ContactHeight,
    /// Caller-owned physical pen-up-height evidence.
    pub pen_up_height: PenUpHeight,
    /// Caller-owned admitted speed evidence.
    pub speed: Speed,
}

/// Measured blank-sheet registration and motion-clearance evidence.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SheetRegistrationCalibration<
    BoundaryClearance,
    PageClamping,
    UsableArea,
> {
    /// Caller-owned boundary-clearance evidence.
    pub boundary_clearance: BoundaryClearance,
    /// Caller-owned page-clamping evidence.
    pub page_clamping: PageClamping,
    /// Caller-owned usable physical writing area.
    pub usable_area: UsableArea,
}

/// Complete calibration evidence bound to one identified device setup.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DeviceCoordinateCalibration<
    CalibrationIdentity,
    CoordinateTransform,
    DeviceIdentity,
    Evidence,
    PenMotion,
    SheetRegistration,
> {
    /// Stable caller/backend-owned identity for this calibration evidence set.
    pub calibration_identity: CalibrationIdentity,
    /// Coordinate transform measurements.
    pub coordinate_transform: CoordinateTransform,
    /// Exact identified physical device or adapter setup.
    pub device_identity: DeviceIdentity,
    /// Caller-owned measurement/provenance evidence.
    pub evidence: Evidence,
    /// Pen heights and admitted motion envelope.
    pub pen_motion: PenMotion,
    /// Blank-sheet registration and clearance measurements.
    pub sheet_registration: SheetRegistration,
}
