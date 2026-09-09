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
//   - Transport-neutral interruption/recovery state and fail-closed resume
//     admission.
// - Must-Not:
//   - Contact devices, infer position, repair partial strokes, perform homing,
//     issue resume/safe-stop commands, arm hardware, or choose operator steps.
// - Allows:
//   - Inputs: Explicit interruption, feedback, position, boundary, and stroke
//     state.
//   - Outputs: Resume-known-state or operator-recovery-required disposition.
//   - Side effects: None.
// - Split-When:
//   - Adapter-specific recovery execution gains executable authority.
// - Merge-When:
//   - Recovery admission becomes inseparable from one physical adapter.
// - Summary:
//   - Refuses unsafe resume whenever physical state is not fully known.
// - Description:
//   - Treats missing feedback, unknown position, violated bounds, and partial
//     stroke state as operator-recovery conditions.
// - Usage:
//   - Evaluate an explicit recovery snapshot before any adapter resume action.
// - Defaults:
//   - Uncertain state never becomes resumable by assumption.
//

//! Fail-closed physical interruption recovery without device-control authority.

/// Why a physical-writing session entered interruption/recovery handling.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum PhysicalInterruptionKind {
    /// Device or transport disconnected unexpectedly.
    Disconnect,
    /// Operator or external safety system requested an emergency stop.
    EmergencyStop,
    /// Power was lost or device state reset unexpectedly.
    PowerLoss,
    /// Atrament or its managed adapter process terminated unexpectedly.
    ProcessCrash,
    /// Atrament or its managed adapter process restarted before recovery
    /// review.
    ProcessRestart,
    /// User requested an ordinary pause.
    UserPause,
}

/// Whether required device feedback is currently available.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum PhysicalFeedbackState {
    /// Required device feedback is available and caller-validated.
    Available,
    /// Required device feedback is missing or unavailable.
    Missing,
}

/// Whether the current physical position is known to the adapter boundary.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PhysicalPositionState<Position> {
    /// Exact caller-owned current position evidence.
    Known(Position),
    /// Current carriage/pen position is unknown.
    Unknown,
}

/// Whether the current physical state remains inside admitted boundaries.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum PhysicalBoundaryState {
    /// A physical boundary has been violated.
    Violated,
    /// Caller-owned boundary validation remains satisfied.
    WithinBounds,
}

/// Whether interruption occurred while a stroke was partially executed.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum PhysicalStrokeState {
    /// Interruption did not leave a partially executed stroke.
    BetweenStrokes,
    /// A stroke was only partially executed and ink state may be uncertain.
    PartialStroke,
}

/// Explicit physical recovery snapshot after interruption.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PhysicalRecoverySnapshot<Position> {
    /// Current boundary-validation state.
    pub boundary: PhysicalBoundaryState,
    /// Current required-feedback state.
    pub feedback: PhysicalFeedbackState,
    /// Interruption provenance retained for operator inspection.
    pub interruption: PhysicalInterruptionKind,
    /// Current position knowledge.
    pub position: PhysicalPositionState<Position>,
    /// Whether a partial stroke remains unresolved.
    pub stroke: PhysicalStrokeState,
}

/// Resume admission derived only from explicit physical recovery evidence.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PhysicalResumeDisposition {
    /// Physical state is not sufficiently known; operator recovery is required.
    OperatorRecoveryRequired,
    /// State is fully known enough for an adapter to consider explicit resume.
    ResumeKnownState,
}

/// Return fail-closed resume admission for one physical recovery snapshot.
#[must_use]
pub const fn physical_resume_disposition<Position>(
    snapshot: &PhysicalRecoverySnapshot<Position>,
) -> PhysicalResumeDisposition {
    if matches!(snapshot.feedback, PhysicalFeedbackState::Missing)
        || matches!(snapshot.position, PhysicalPositionState::Unknown)
        || matches!(snapshot.boundary, PhysicalBoundaryState::Violated)
        || matches!(snapshot.stroke, PhysicalStrokeState::PartialStroke)
    {
        return PhysicalResumeDisposition::OperatorRecoveryRequired;
    }
    PhysicalResumeDisposition::ResumeKnownState
}
