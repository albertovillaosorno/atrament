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
//   - Regression evidence for fail-closed physical recovery admission.
// - Must-Not:
//   - Contact hardware, infer position, home devices, execute resume/safe-stop,
//     repair strokes, or choose operator recovery procedures.
// - Allows:
//   - Inputs: Deterministic recovery snapshots.
//   - Outputs: Assertions over known-state resume and fail-closed refusal.
//   - Side effects: Process-local test allocation only.
// - Split-When:
//   - Adapter-specific recovery execution gains independent fixtures.
// - Merge-When:
//   - Recovery evidence moves into a physical-adapter acceptance harness.
// - Summary:
//   - Proves uncertainty cannot be reclassified as resumable state.
// - Description:
//   - Covers disconnects, power loss, pause, emergency stop, crash, restart,
//     and stroke uncertainty.
// - Usage:
//   - Compile directly against the physical-recovery-state domain.
// - Defaults:
//   - Operator recovery is required for every unresolved physical uncertainty.
//
use atrament_physical_recovery_state::{
    PhysicalBoundaryState, PhysicalFeedbackState, PhysicalInterruptionKind,
    PhysicalPositionState, PhysicalRecoverySnapshot, PhysicalResumeDisposition,
    PhysicalStrokeState, physical_resume_disposition,
};

fn safe_snapshot(
    interruption: PhysicalInterruptionKind,
) -> PhysicalRecoverySnapshot<(i32, i32)> {
    PhysicalRecoverySnapshot {
        boundary: PhysicalBoundaryState::WithinBounds,
        feedback: PhysicalFeedbackState::Available,
        interruption,
        position: PhysicalPositionState::Known((120, 340)),
        stroke: PhysicalStrokeState::BetweenStrokes,
    }
}

#[test]
fn fully_known_state_is_the_only_resumable_shape() {
    for interruption in [
        PhysicalInterruptionKind::Disconnect,
        PhysicalInterruptionKind::EmergencyStop,
        PhysicalInterruptionKind::PowerLoss,
        PhysicalInterruptionKind::ProcessCrash,
        PhysicalInterruptionKind::ProcessRestart,
        PhysicalInterruptionKind::UserPause,
    ] {
        let snapshot = safe_snapshot(interruption);
        assert_eq!(
            physical_resume_disposition(&snapshot),
            PhysicalResumeDisposition::ResumeKnownState,
        );
        assert_eq!(snapshot.interruption, interruption);
    }
}

#[test]
fn unknown_position_always_requires_operator_recovery() {
    let mut snapshot = safe_snapshot(PhysicalInterruptionKind::PowerLoss);
    snapshot.position = PhysicalPositionState::Unknown;
    assert_eq!(
        physical_resume_disposition(&snapshot),
        PhysicalResumeDisposition::OperatorRecoveryRequired,
    );
}

#[test]
fn missing_feedback_requires_operator_recovery_even_with_known_position() {
    let mut snapshot = safe_snapshot(PhysicalInterruptionKind::Disconnect);
    snapshot.feedback = PhysicalFeedbackState::Missing;
    assert_eq!(
        physical_resume_disposition(&snapshot),
        PhysicalResumeDisposition::OperatorRecoveryRequired,
    );
}

#[test]
fn boundary_violation_requires_operator_recovery() {
    let mut snapshot = safe_snapshot(PhysicalInterruptionKind::EmergencyStop);
    snapshot.boundary = PhysicalBoundaryState::Violated;
    assert_eq!(
        physical_resume_disposition(&snapshot),
        PhysicalResumeDisposition::OperatorRecoveryRequired,
    );
}

#[test]
fn partial_stroke_requires_operator_recovery() {
    let mut snapshot = safe_snapshot(PhysicalInterruptionKind::ProcessCrash);
    snapshot.stroke = PhysicalStrokeState::PartialStroke;
    assert_eq!(
        physical_resume_disposition(&snapshot),
        PhysicalResumeDisposition::OperatorRecoveryRequired,
    );
}

#[test]
fn process_restart_does_not_bypass_unknown_physical_state() {
    let mut snapshot = safe_snapshot(PhysicalInterruptionKind::ProcessRestart);
    snapshot.feedback = PhysicalFeedbackState::Missing;
    snapshot.position = PhysicalPositionState::Unknown;
    assert_eq!(
        physical_resume_disposition(&snapshot),
        PhysicalResumeDisposition::OperatorRecoveryRequired,
    );
    assert_eq!(
        snapshot.interruption,
        PhysicalInterruptionKind::ProcessRestart,
    );
}
