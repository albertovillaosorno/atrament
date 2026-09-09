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
//   - Transport-neutral empirical contact inputs, outputs, and evidence.
// - Must-Not:
//   - Choose force units, proxy scales, fitted functions, concrete output
//     units or ranges, hidden physical state, rendering, or machine motion.
// - Allows:
//   - Inputs: Caller-owned contact inputs, outputs, and calibrated evidence.
//   - Outputs: Typed pressure, transfer-output, preset-evidence, and admission
//     values.
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

//! Honest inputs, observable outputs, and evidence for pen-contact models.

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

/// Observable outputs produced by one empirical contact-transfer evaluation.
///
/// The six value types remain caller-owned. This structure does not choose
/// transfer equations, physical units, calibrated output ranges, or rendering.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ContactModelOutput<
    AbsorptionOrDryingResponse,
    Coverage,
    EdgeDisplacement,
    Pooling,
    Starvation,
    TraceWidth,
> {
    /// Caller-owned absorption or drying response.
    pub absorption_or_drying_response: AbsorptionOrDryingResponse,
    /// Caller-owned deposited coverage response.
    pub coverage: Coverage,
    /// Caller-owned edge displacement response around geometric authority.
    pub edge_displacement: EdgeDisplacement,
    /// Caller-owned pooling response.
    pub pooling: Pooling,
    /// Caller-owned starvation response.
    pub starvation: Starvation,
    /// Caller-owned trace-width response around the unchanged centerline.
    pub trace_width: TraceWidth,
}

/// Evidence carried by one empirically validated contact preset.
///
/// Collection and identity types remain caller-owned so this value does not
/// choose media identifiers, condition vocabulary, or numeric range policy.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ContactPresetEvidence<
    Conditions,
    ErrorMeasures,
    FailureModes,
    InkIdentity,
    InputRanges,
    PaperIdentity,
    PenIdentity,
> {
    /// Caller-owned validation conditions.
    pub conditions: Conditions,
    /// Caller-owned fitted or observed error measures.
    pub error_measures: ErrorMeasures,
    /// Validated ink identity.
    pub ink_identity: InkIdentity,
    /// Caller-owned calibrated input-range evidence.
    pub input_ranges: InputRanges,
    /// Caller-owned known failure modes.
    pub known_failure_modes: FailureModes,
    /// Validated paper identity.
    pub paper_identity: PaperIdentity,
    /// Validated pen identity.
    pub pen_identity: PenIdentity,
}

/// One caller-owned inclusive envelope for an observable transfer output.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ObservableOutputRange<Value> {
    /// Maximum admitted observable output value.
    pub maximum: Value,
    /// Minimum admitted observable output value.
    pub minimum: Value,
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

/// Admission status of one observable output against its declared envelope.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ContactOutputAdmission {
    /// Output lies inside the inclusive declared envelope.
    Bounded,
    /// Output lies outside the declared envelope.
    OutsideEnvelope,
}

/// Why one observable output range is invalid.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ObservableOutputRangeError {
    /// Minimum exceeds maximum.
    MinimumAboveMaximum,
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

/// Classify one observable transfer output against its inclusive envelope.
///
/// # Errors
///
/// Returns a typed error when the declared output envelope is inverted.
pub fn classify_contact_output<Value>(
    output: &Value,
    range: &ObservableOutputRange<Value>,
) -> Result<ContactOutputAdmission, ObservableOutputRangeError>
where
    Value: Ord,
{
    if range.minimum > range.maximum {
        return Err(ObservableOutputRangeError::MinimumAboveMaximum);
    }
    if output < &range.minimum || output > &range.maximum {
        return Ok(ContactOutputAdmission::OutsideEnvelope);
    }
    Ok(ContactOutputAdmission::Bounded)
}
