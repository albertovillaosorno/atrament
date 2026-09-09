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
//   - Regression evidence for honest empirical pen-contact model inputs.
// - Must-Not:
//   - Choose force units, proxy scales, transfer functions, material output,
//     rendering, or physical-device behavior.
// - Allows:
//   - Inputs: Deterministic contact, output, and fitted-evidence fixtures.
//   - Outputs: Assertions over pressure, output vocabulary, preset evidence,
//     and calibrated range status.
//   - Side effects: Process-local test allocation only.
// - Split-When:
//   - Transfer evaluation or material projection gains independent fixtures.
// - Merge-When:
//   - Contact evidence validation moves into another empirical-model harness.
// - Summary:
//   - Proves proxy pressure cannot masquerade as calibrated force evidence.
// - Description:
//   - Covers contact inputs, fitted evidence, and explicit extrapolation
//     status.
// - Usage:
//   - Compile directly against the pen-contact-model domain.
// - Defaults:
//   - Out-of-range inputs are never silently admitted as calibrated.
//
use atrament_pen_contact_model::{
    CalibratedInputRange, CalibratedInputRangeError, ContactInputAdmission,
    ContactModelInput, ContactModelOutput, ContactOutputAdmission,
    ContactPresetEvidence, FittedParameterEvidence, ObservableOutputRange,
    ObservableOutputRangeError, PressureInput, classify_contact_input,
    classify_contact_output,
};

#[test]
fn pressure_force_and_dimensionless_proxy_remain_distinct() {
    let force: PressureInput<u16, &'static str, u16> =
        PressureInput::CalibratedForce(3);
    let proxy = PressureInput::DimensionlessProxy {
        name: "tablet-pressure",
        value: 3_u16,
    };
    assert_ne!(force, proxy);
}

#[test]
fn contact_input_retains_all_measurable_model_inputs() {
    let input = ContactModelInput {
        contact_state: "down",
        curvature: 9_i32,
        direction: "north-east",
        dwell_time: 4_u16,
        position: (120_i32, 240_i32),
        pressure: PressureInput::<u16, &str, u16>::DimensionlessProxy {
            name: "capture-proxy",
            value: 700,
        },
        speed: 18_u16,
    };
    assert_eq!(input.position, (120, 240));
    assert_eq!(input.direction, "north-east");
    assert_eq!(input.curvature, 9);
    assert_eq!(input.speed, 18);
    assert_eq!(input.dwell_time, 4);
    assert_eq!(input.contact_state, "down");
}

#[test]
fn contact_output_retains_all_accepted_observable_response_families() {
    let output = ContactModelOutput {
        absorption_or_drying_response: "drying-12",
        coverage: 84_u16,
        edge_displacement: -3_i16,
        pooling: 7_u16,
        starvation: 2_u16,
        trace_width: 410_u16,
    };
    assert_eq!(output.trace_width, 410);
    assert_eq!(output.coverage, 84);
    assert_eq!(output.starvation, 2);
    assert_eq!(output.pooling, 7);
    assert_eq!(output.edge_displacement, -3);
    assert_eq!(output.absorption_or_drying_response, "drying-12");
}

#[test]
fn contact_preset_retains_validation_scope_and_known_failure_evidence(
) {
    let evidence = ContactPresetEvidence {
        conditions: ["room-temperature", "flat-sheet"],
        error_measures: ["width-rmse", "coverage-rmse"],
        input_ranges: ["speed:2-30", "pressure:100-900"],
        ink_identity: "ink-black-4",
        known_failure_modes: ["glossy-paper"],
        output_ranges: ContactModelOutput {
            absorption_or_drying_response: "drying-envelope",
            coverage: "coverage-envelope",
            edge_displacement: "edge-envelope",
            pooling: "pooling-envelope",
            starvation: "starvation-envelope",
            trace_width: "width-envelope",
        },
        paper_identity: "paper-ruled-2",
        pen_identity: "pen-ballpoint-7",
    };
    assert_eq!(evidence.pen_identity, "pen-ballpoint-7");
    assert_eq!(evidence.ink_identity, "ink-black-4");
    assert_eq!(evidence.paper_identity, "paper-ruled-2");
    assert_eq!(evidence.conditions.len(), 2);
    assert_eq!(evidence.input_ranges.len(), 2);
    assert_eq!(evidence.error_measures.len(), 2);
    assert_eq!(evidence.known_failure_modes, ["glossy-paper"]);
    assert_eq!(evidence.output_ranges.trace_width, "width-envelope");
    assert_eq!(evidence.output_ranges.coverage, "coverage-envelope");
}

#[test]
fn fitted_parameter_evidence_retains_units_provenance_confidence_and_error() {
    let evidence = FittedParameterEvidence {
        confidence: "high",
        error_measure: "rmse-0.7",
        evidence: "controlled-line-17",
        input_range: CalibratedInputRange {
            maximum: 900_u16,
            minimum: 100_u16,
        },
        unit: "dimensionless-proxy",
    };
    assert_eq!(evidence.unit, "dimensionless-proxy");
    assert_eq!(evidence.evidence, "controlled-line-17");
    assert_eq!(evidence.confidence, "high");
    assert_eq!(evidence.error_measure, "rmse-0.7");
}

#[test]
fn calibrated_range_classifies_boundaries_and_extrapolation_explicitly() {
    let range = CalibratedInputRange {
        maximum: 20_i32,
        minimum: 10_i32,
    };
    assert_eq!(
        classify_contact_input(&10, &range),
        Ok(ContactInputAdmission::Calibrated),
    );
    assert_eq!(
        classify_contact_input(&20, &range),
        Ok(ContactInputAdmission::Calibrated),
    );
    assert_eq!(
        classify_contact_input(&9, &range),
        Ok(ContactInputAdmission::ExtrapolationRequired),
    );
    assert_eq!(
        classify_contact_input(&21, &range),
        Ok(ContactInputAdmission::ExtrapolationRequired),
    );
}

#[test]
fn inverted_calibrated_range_rejects_before_classification() {
    let range = CalibratedInputRange {
        maximum: 10_i32,
        minimum: 20_i32,
    };
    assert_eq!(
        classify_contact_input(&15, &range),
        Err(CalibratedInputRangeError::MinimumAboveMaximum),
    );
}


#[test]
fn observable_output_envelope_includes_both_boundaries() {
    let range = ObservableOutputRange {
        maximum: 480_u16,
        minimum: 320_u16,
    };
    assert_eq!(
        classify_contact_output(&320, &range),
        Ok(ContactOutputAdmission::Bounded),
    );
    assert_eq!(
        classify_contact_output(&480, &range),
        Ok(ContactOutputAdmission::Bounded),
    );
    assert_eq!(
        classify_contact_output(&319, &range),
        Ok(ContactOutputAdmission::OutsideEnvelope),
    );
    assert_eq!(
        classify_contact_output(&481, &range),
        Ok(ContactOutputAdmission::OutsideEnvelope),
    );
}

#[test]
fn inverted_observable_output_envelope_rejects_before_classification() {
    let range = ObservableOutputRange {
        maximum: 10_i32,
        minimum: 20_i32,
    };
    assert_eq!(
        classify_contact_output(&15, &range),
        Err(ObservableOutputRangeError::MinimumAboveMaximum),
    );
}
