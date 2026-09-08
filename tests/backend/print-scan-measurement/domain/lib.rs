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
//   - Regression evidence for required print/scan measurement families.
// - Must-Not:
//   - Choose devices, units, tolerances, correction algorithms, or pass/fail
//     policy.
// - Allows:
//   - Inputs: Deterministic caller-owned physical measurement fixtures.
//   - Outputs: Assertions over typed measurement and output provenance.
//   - Side effects: Process-local test allocation only.
// - Split-When:
//   - Calibration fitting or correction gains independent executable fixtures.
// - Merge-When:
//   - Measurement evidence moves into another physical-validation harness.
// - Summary:
//   - Proves required physical-output observations remain typed and
//     attributable.
// - Description:
//   - Covers scale, clipping, grid registration, photo placement, and
//     provenance.
// - Usage:
//   - Compile directly against the print-scan-measurement domain.
// - Defaults:
//   - No observation implies a calibration conclusion by itself.
//
use atrament_print_scan_measurement::{
    PrintScanEvidence, PrintScanMeasurement, PrintScanMeasurementKind,
};

#[test]
fn accepted_pdf_round_trip_measurement_families_are_explicit() {
    let kinds = [
        PrintScanMeasurementKind::Clipping,
        PrintScanMeasurementKind::GridRegistration,
        PrintScanMeasurementKind::PhotoPlacement,
        PrintScanMeasurementKind::PhysicalScale,
    ];
    assert_eq!(kinds.len(), 4);
}

#[test]
fn measurement_retains_source_unit_and_value_without_interpretation() {
    let measurement = PrintScanMeasurement {
        kind: PrintScanMeasurementKind::PhysicalScale,
        source: "printer-a/scanner-b/run-7",
        unit: "caller-owned-ratio-unit",
        value: (999_u32, 1000_u32),
    };
    assert_eq!(measurement.source, "printer-a/scanner-b/run-7");
    assert_eq!(measurement.unit, "caller-owned-ratio-unit");
    assert_eq!(measurement.value, (999, 1000));
}

#[test]
fn evidence_keeps_output_identity_and_measurement_order() {
    let evidence = PrintScanEvidence {
        measurements: vec![
            PrintScanMeasurement {
                kind: PrintScanMeasurementKind::GridRegistration,
                source: "scan-1",
                unit: "micrometre-like-caller-unit",
                value: 12_i32,
            },
            PrintScanMeasurement {
                kind: PrintScanMeasurementKind::PhotoPlacement,
                source: "scan-1",
                unit: "micrometre-like-caller-unit",
                value: -4_i32,
            },
        ],
        output_identity: "render-output-22",
    };
    assert_eq!(evidence.output_identity, "render-output-22");
    assert_eq!(
        evidence.measurements[0].kind,
        PrintScanMeasurementKind::GridRegistration,
    );
    assert_eq!(
        evidence.measurements[1].kind,
        PrintScanMeasurementKind::PhotoPlacement,
    );
}
