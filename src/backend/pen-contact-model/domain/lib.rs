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
//   - Transport-neutral empirical pen-contact input and fitted-evidence bounds.
// - Must-Not:
//   - Choose force units, proxy scales, fitted functions, material outputs,
//     hidden physical state, rendering, or machine-motion behavior.
// - Allows:
//   - Inputs: Caller-owned measurable contact inputs and calibrated evidence.
//   - Outputs: Typed pressure provenance, fitted evidence, and range admission.
//   - Side effects: None.
// - Split-When:
//   - Contact-transfer evaluation or material projection gains executable
//     logic.
// - Merge-When:
//   - Contact-model evidence becomes inseparable from one material preset.
// - Summary:
//   - Keeps empirical contact evidence honest before any ink-transfer model.
// - Description:
//   - Separates force from pressure proxies and exposes extrapolation
//     explicitly.
// - Usage:
//   - Validate fitted preset inputs before evaluating material response.
// - Defaults:
//   - Out-of-range inputs never silently become calibrated evidence.
//

//! Honest input and calibration evidence for empirical pen-contact models.

/// Pressure evidence admitted by an empirical contact model.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PressureInput<Force, ProxyName, ProxyValue> {
    /// Pressure measured in a caller-owned calibrated force unit.
    CalibratedForce(Force),
    /// Named dimensionless proxy retained without force-unit claims.
    DimensionlessProxy {
        /// Caller-owned proxy identity.
        name: ProxyName,
        /// Caller-owned normalized or device-local proxy value.
        value: ProxyValue,
    },
}

/// Complete measurable contact inputs before material transfer evaluation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ContactModelInput<
    Position,
    Direction,
    Curvature,
    Speed,
    DwellTime,
    ContactState,
    Pressure,
> {
    /// Caller-owned contact state.
    pub contact_state: ContactState,
    /// Caller-owned centerline curvature.
    pub curvature: Curvature,
    /// Caller-owned motion direction.
    pub direction: Direction,
    /// Caller-owned dwell time.
    pub dwell_time: DwellTime,
    /// Caller-owned physical position.
    pub position: Position,
    /// Force-calibrated or explicitly proxied pressure input.
    pub pressure: Pressure,
    /// Caller-owned motion speed.
    pub speed: Speed,
}

/// One calibrated inclusive input range.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CalibratedInputRange<Value> {
    /// Maximum calibrated input value.
    pub maximum: Value,
    /// Minimum calibrated input value.
    pub minimum: Value,
}

/// Fitted parameter evidence required by the accepted contact-model ADR.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FittedParameterEvidence<
    Value,
    Unit,
    Evidence,
    Confidence,
    ErrorMeasure,
> {
    /// Caller-owned confidence evidence.
    pub confidence: Confidence,
    /// Caller-owned error measure for the fitted parameter.
    pub error_measure: ErrorMeasure,
    /// Caller-owned calibration or provenance evidence.
    pub evidence: Evidence,
    /// Valid calibrated input range for this parameter.
    pub input_range: CalibratedInputRange<Value>,
    /// Caller-owned measurement unit.
    pub unit: Unit,
}

/// Admission status of one input against calibrated evidence.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ContactInputAdmission {
    /// Input is inside the inclusive calibrated range.
    Calibrated,
    /// Input lies outside the calibrated range and needs explicit
    /// extrapolation.
    ExtrapolationRequired,
}

/// Why one calibrated input range is invalid.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CalibratedInputRangeError {
    /// Minimum exceeds maximum.
    MinimumAboveMaximum,
}

/// Classify one model input against its inclusive calibrated range.
///
/// # Errors
///
/// Returns a typed error when the declared calibrated range is inverted.
pub fn classify_contact_input<Value>(
    input: &Value,
    range: &CalibratedInputRange<Value>,
) -> Result<ContactInputAdmission, CalibratedInputRangeError>
where
    Value: Ord,
{
    if range.minimum > range.maximum {
        return Err(CalibratedInputRangeError::MinimumAboveMaximum);
    }
    if input < &range.minimum || input > &range.maximum {
        return Ok(ContactInputAdmission::ExtrapolationRequired);
    }
    Ok(ContactInputAdmission::Calibrated)
}
