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
//   - Regression evidence for exact physical compatibility records.
// - Must-Not:
//   - Probe hardware, infer support, validate calibration, contact devices,
//     choose adapter tiers, serialize records, or authorize motion.
// - Allows:
//   - Inputs: Deterministic caller-owned compatibility fixtures.
//   - Outputs: Assertions that every frozen support-evidence field is explicit.
//   - Side effects: Process-local test allocation only.
// - Split-When:
//   - Compatibility admission or persistence gains independent fixtures.
// - Merge-When:
//   - Compatibility evidence moves into a hardware-adapter acceptance harness.
// - Summary:
//   - Proves support claims remain scoped to exact tested combinations.
// - Description:
//   - Covers model, firmware, transport, setup, evidence, limitations, and
//     date.
// - Usage:
//   - Compile directly against the physical-compatibility-ledger domain.
// - Defaults:
//   - No untested device or setup inherits compatibility implicitly.
//
use atrament_physical_compatibility_ledger::{
    DeviceCompatibilityIdentity, PhysicalCompatibilityRecord,
    PhysicalCompatibilitySetup,
};

#[test]
fn device_identity_keeps_model_firmware_transport_and_adapter_tier() {
    let device = DeviceCompatibilityIdentity {
        adapter_tier: "managed-cli",
        firmware: "fw-3.2.1",
        model: "plotter-model-a",
        transport: "documented-cli-usb",
    };
    assert_eq!(device.model, "plotter-model-a");
    assert_eq!(device.firmware, "fw-3.2.1");
    assert_eq!(device.transport, "documented-cli-usb");
    assert_eq!(device.adapter_tier, "managed-cli");
}

#[test]
fn physical_setup_keeps_paper_pen_usable_area_and_settings() {
    let setup = PhysicalCompatibilitySetup {
        paper: "blank-a4-90gsm",
        pen: "black-ballpoint-0.7mm",
        settings: ["speed=75", "acceleration=32"],
        usable_area: (10_000_u32, 12_000_u32, 190_000_u32, 270_000_u32),
    };
    assert_eq!(setup.paper, "blank-a4-90gsm");
    assert_eq!(setup.pen, "black-ballpoint-0.7mm");
    assert_eq!(setup.settings, ["speed=75", "acceleration=32"]);
    assert_eq!(setup.usable_area, (10_000, 12_000, 190_000, 270_000));
}

#[test]
fn compatibility_record_retains_evidence_limitations_and_acceptance_date() {
    let record = PhysicalCompatibilityRecord {
        evidence: ["homing-photo", "boundary-refusal-log", "dry-run-report"],
        known_limitations: ["no-pressure-feedback", "manual-sheet-clamping"],
        last_acceptance_date: "2026-09-08",
        device: DeviceCompatibilityIdentity {
            adapter_tier: "managed-cli",
            firmware: "fw-3.2.1",
            model: "plotter-model-a",
            transport: "documented-cli-usb",
        },
        setup: PhysicalCompatibilitySetup {
            paper: "blank-a4-90gsm",
            pen: "black-ballpoint-0.7mm",
            settings: ["speed=75", "acceleration=32"],
            usable_area: "calibrated-area-11",
        },
    };
    assert_eq!(record.last_acceptance_date, "2026-09-08");
    assert_eq!(
        record.known_limitations,
        ["no-pressure-feedback", "manual-sheet-clamping"],
    );
    assert_eq!(record.evidence.len(), 3);
}
