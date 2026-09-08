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
//   - Regression evidence for grouped device-coordinate calibration inputs.
// - Must-Not:
//   - Measure hardware, choose units/tolerances, transform coordinates,
//     validate
//     limits, arm devices, or authorize motion.
// - Allows:
//   - Inputs: Deterministic caller-owned calibration fixtures.
//   - Outputs: Assertions that every frozen measurement remains explicit.
//   - Side effects: Process-local test allocation only.
// - Split-When:
//   - Calibration math or hardware measurement gains independent fixtures.
// - Merge-When:
//   - Calibration evidence moves into a hardware-adapter acceptance harness.
// - Summary:
//   - Proves no device calibration measurement is hidden or inferred.
// - Description:
//   - Covers transforms, motion envelope, sheet registration, and provenance.
// - Usage:
//   - Compile directly against the device-coordinate-calibration domain.
// - Defaults:
//   - Caller-owned values remain uninterpreted by this evidence layer.
//
use atrament_device_coordinate_calibration::{
    CoordinateTransformCalibration, DeviceCoordinateCalibration,
    PenMotionCalibration, SheetRegistrationCalibration,
};

#[test]
fn coordinate_transform_retains_origin_axes_scale_and_skew() {
    let transform = CoordinateTransformCalibration {
        axis_orientation: "x-right/y-down",
        origin: (1_200_i32, -350_i32),
        scale: (1_000_u32, 1_002_u32),
        skew: (3_i16, -2_i16),
    };
    assert_eq!(transform.axis_orientation, "x-right/y-down");
    assert_eq!(transform.origin, (1_200, -350));
    assert_eq!(transform.scale, (1_000, 1_002));
    assert_eq!(transform.skew, (3, -2));
}

#[test]
fn pen_motion_retains_heights_speed_and_acceleration() {
    let motion = PenMotionCalibration {
        acceleration: 32_u16,
        contact_height: 410_u16,
        pen_up_height: 920_u16,
        speed: 75_u16,
    };
    assert_eq!(motion.pen_up_height, 920);
    assert_eq!(motion.contact_height, 410);
    assert_eq!(motion.speed, 75);
    assert_eq!(motion.acceleration, 32);
}

#[test]
fn sheet_registration_retains_usable_area_clamping_and_clearance() {
    let sheet = SheetRegistrationCalibration {
        boundary_clearance: 5_000_u32,
        page_clamping: "four-corner-clamps",
        usable_area: (10_000_u32, 12_000_u32, 190_000_u32, 270_000_u32),
    };
    assert_eq!(sheet.boundary_clearance, 5_000);
    assert_eq!(sheet.page_clamping, "four-corner-clamps");
    assert_eq!(sheet.usable_area, (10_000, 12_000, 190_000, 270_000));
}

#[test]
fn complete_calibration_binds_measurements_to_device_identity_and_evidence() {
    let calibration = DeviceCoordinateCalibration {
        calibration_identity: "calibration-11",
        coordinate_transform: CoordinateTransformCalibration {
            axis_orientation: "measured-axis-orientation",
            origin: "measured-origin",
            scale: "measured-scale",
            skew: "measured-skew",
        },
        device_identity: "device:model/firmware/transport",
        evidence: ["registration-sheet-4", "limit-probe-7"],
        pen_motion: PenMotionCalibration {
            acceleration: "measured-acceleration",
            contact_height: "measured-contact-height",
            pen_up_height: "measured-pen-up-height",
            speed: "measured-speed",
        },
        sheet_registration: SheetRegistrationCalibration {
            boundary_clearance: "measured-boundary-clearance",
            page_clamping: "measured-page-clamping",
            usable_area: "measured-usable-area",
        },
    };
    assert_eq!(calibration.calibration_identity, "calibration-11");
    assert_eq!(
        calibration.device_identity,
        "device:model/firmware/transport",
    );
    assert_eq!(
        calibration.evidence,
        ["registration-sheet-4", "limit-probe-7"],
    );
}
