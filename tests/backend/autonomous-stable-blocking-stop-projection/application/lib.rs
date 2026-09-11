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
//   - Exhaustive projection evidence across all nine progress-evidence classes.
// - Must-Not:
//   - Infer diagnostics, stop on unrelated evidence, retry, or execute a loop.
// - Allows:
//   - Inputs: Every frozen progress-evidence class.
//   - Outputs: Exactly one stable-blocking terminal mapping and eight `None`s.
//   - Side effects: None.
// - Split-When:
//   - Stateful diagnostic comparison requires independent integration evidence.
// - Merge-When:
//   - Coordinator tests fully subsume this projection oracle.
// - Summary:
//   - Proves only repeated stable blocking evidence constructs this stop class.
// - Description:
//   - Prevents replay, No-op, and generic repeated inputs from auto-stopping.
// - Usage:
//   - Compare all nine evidence classes with an independent exact oracle.
// - Defaults:
//   - Non-stable-blocking evidence remains nonterminal on this projection.
//
use atrament_autonomous_goal_outcome::AutonomousGoalTerminalClass;
use atrament_autonomous_progress_evidence::AutonomousProgressEvidenceClass;
use atrament_autonomous_stable_blocking_stop_projection::{
    autonomous_stable_blocking_terminal_outcome,
};

const EVIDENCE: [AutonomousProgressEvidenceClass; 9] = [
    AutonomousProgressEvidenceClass::AcceptedApplyRevisionChange,
    AutonomousProgressEvidenceClass::AcceptedHistoryRevisionChange,
    AutonomousProgressEvidenceClass::BlockingDiagnosticResolved,
    AutonomousProgressEvidenceClass::ExplicitRequestedOutputCompleted,
    AutonomousProgressEvidenceClass::IdempotentReplayRecovery,
    AutonomousProgressEvidenceClass::NewAdmittedAuthorityOrContext,
    AutonomousProgressEvidenceClass::RepeatedNoOpSameIntent,
    AutonomousProgressEvidenceClass::RepeatedSameInputsWithoutNewAdmission,
    AutonomousProgressEvidenceClass::StableBlockingDiagnosticRepeated,
];

#[test]
fn only_repeated_stable_blocking_evidence_maps_to_terminal_stop() {
    let mut stops = 0_usize;
    for evidence in EVIDENCE {
        let actual = autonomous_stable_blocking_terminal_outcome(evidence);
        let expected = if evidence
            == AutonomousProgressEvidenceClass::StableBlockingDiagnosticRepeated
        {
            Some(AutonomousGoalTerminalClass::StoppedStableBlockingFailure)
        } else {
            None
        };
        assert_eq!(actual, expected);
        if actual.is_some() {
            stops += 1;
        }
    }
    assert_eq!(stops, 1);
}
