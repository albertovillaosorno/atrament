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
//   - Covers every first-release round-trip family plus provenance.
// - Usage:
//   - Compile directly against the print-scan-measurement domain.
// - Defaults:
//   - No observation implies a calibration conclusion by itself.
//

//! Print scan measurement domain regression tests.

use std::ptr;

use atrament_print_scan_measurement::{
    validate_print_scan_evidence, validate_print_scan_evidence_view,
    PrintScanEvidence, PrintScanEvidenceError, PrintScanMeasurement,
    PrintScanMeasurementKind,
};

const REQUIRED_KINDS: [PrintScanMeasurementKind; 8] = [
    PrintScanMeasurementKind::PhysicalScale,
    PrintScanMeasurementKind::Clipping,
    PrintScanMeasurementKind::Margins,
    PrintScanMeasurementKind::GridRegistration,
    PrintScanMeasurementKind::ColorShift,
    PrintScanMeasurementKind::PhotoPlacement,
    PrintScanMeasurementKind::LineWeight,
    PrintScanMeasurementKind::ScannerDistortion,
];

#[test]
fn first_release_pdf_round_trip_measurement_families_are_explicit() {
    let kinds = [
        PrintScanMeasurementKind::Clipping,
        PrintScanMeasurementKind::ColorShift,
        PrintScanMeasurementKind::GridRegistration,
        PrintScanMeasurementKind::LineWeight,
        PrintScanMeasurementKind::Margins,
        PrintScanMeasurementKind::PhotoPlacement,
        PrintScanMeasurementKind::PhysicalScale,
        PrintScanMeasurementKind::ScannerDistortion,
    ];
    assert_eq!(kinds.len(), 8);
}

#[test]
fn measurement_retains_source_unit_and_value_without_interpretation() {
    let measurement = PrintScanMeasurement {
        kind: PrintScanMeasurementKind::PhysicalScale,
        source: "printer-a/scanner-b/run-7",
        unit: "caller-owned-ratio-unit",
        value: (999u32, 1000u32),
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
                value: 12i32,
            },
            PrintScanMeasurement {
                kind: PrintScanMeasurementKind::PhotoPlacement,
                source: "scan-1",
                unit: "micrometre-like-caller-unit",
                value: -4i32,
            },
        ],
        output_identity: "render-output-22",
    };
    assert_eq!(evidence.output_identity, "render-output-22");
    assert_eq!(
        evidence.measurements.first().map(|sample| sample.kind),
        Some(PrintScanMeasurementKind::GridRegistration),
    );
    assert_eq!(
        evidence.measurements.get(1).map(|sample| sample.kind),
        Some(PrintScanMeasurementKind::PhotoPlacement),
    );
}

#[test]
fn all_256_measurement_presence_masks_match_completeness_oracle() {
    let mut cases = 0u16;
    let mut saw_complete = false;
    let mut saw_missing = [false; 8];
    for mask in 0u16..256 {
        let evidence = PrintScanEvidence {
            measurements: REQUIRED_KINDS
                .into_iter()
                .enumerate()
                .zip(0u8..)
                .filter_map(|((index, kind), ordinal)| {
                    (mask & (1u16 << index) != 0).then_some(
                        PrintScanMeasurement {
                            kind,
                            source: ordinal,
                            unit: (),
                            value: i16::from(ordinal),
                        },
                    )
                })
                .collect(),
            output_identity: 22u8,
        };
        let expected =
            REQUIRED_KINDS
                .into_iter()
                .enumerate()
                .find_map(|(index, kind)| {
                    (mask & (1u16 << index) == 0).then_some((index, kind))
                });
        let expected_result = if let Some((index, kind)) = expected {
            if let Some(seen) = saw_missing.get_mut(index) {
                *seen = true;
            }
            Err(PrintScanEvidenceError::MissingMeasurement { kind })
        } else {
            saw_complete = true;
            Ok(())
        };
        assert_eq!(
            validate_print_scan_evidence(&evidence),
            expected_result,
            "measurement presence mask {mask:#010b}",
        );
        match expected_result {
            Ok(()) => {
                let validation = validate_print_scan_evidence_view(&evidence);
                assert!(validation.is_ok(), "complete evidence must seal");
                if let Ok(sealed) = validation {
                    assert!(ptr::eq(
                        sealed.evidence(),
                        ptr::from_ref(&evidence)
                    ));
                }
            }
            Err(reason) => assert_eq!(
                validate_print_scan_evidence_view(&evidence),
                Err(reason),
                "sealed measurement presence mask {mask:#010b}",
            ),
        }
        cases = cases.saturating_add(1);
    }
    assert_eq!(cases, 256);
    assert!(saw_complete);
    assert!(saw_missing.into_iter().all(|seen| seen));
}

#[test]
fn repeated_measurements_remain_valid_after_complete_coverage() {
    let mut evidence = PrintScanEvidence {
        measurements: REQUIRED_KINDS
            .into_iter()
            .zip(0u8..)
            .map(|(kind, ordinal)| PrintScanMeasurement {
                kind,
                source: ordinal,
                unit: (),
                value: i16::from(ordinal),
            })
            .collect::<Vec<_>>(),
        output_identity: 22u8,
    };
    evidence.measurements.push(PrintScanMeasurement {
        kind: PrintScanMeasurementKind::PhysicalScale,
        source: 99,
        unit: (),
        value: 99,
    });
    assert_eq!(validate_print_scan_evidence(&evidence), Ok(()));
    assert_eq!(evidence.measurements.len(), 9);
}
