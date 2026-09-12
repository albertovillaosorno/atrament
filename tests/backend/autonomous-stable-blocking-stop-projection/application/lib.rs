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
//   - Exhaustive stable-blocking detection and terminal-projection evidence.
// - Must-Not:
//   - Construct diagnostic fingerprints, stop on unrelated evidence, retry, or
//     execute a loop.
// - Allows:
//   - Inputs: Every progress-evidence class plus all equality/admission states
//     for consecutive qualified blocking observations.
//   - Outputs: Exact repeated-blocking evidence and terminal-stop assertions.
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
    AutonomousStableBlockingObservation,
    autonomous_stable_blocking_progress_evidence,
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

#[test]
fn all_16_blocking_observation_states_match_stability_rule() {
    let previous = AutonomousStableBlockingObservation {
        diagnostic: 1_u8,
        intent: 1_u16,
        revision: 1_u32,
    };
    let mut combinations = 0_usize;
    let mut repeated = 0_usize;
    for same_diagnostic in [false, true] {
        for same_intent in [false, true] {
            for same_revision in [false, true] {
                for new_admission in [false, true] {
                    let current = AutonomousStableBlockingObservation {
                        diagnostic: if same_diagnostic { 1 } else { 2 },
                        intent: if same_intent { 1 } else { 2 },
                        revision: if same_revision { 1 } else { 2 },
                    };
                    let expected = if same_diagnostic
                        && same_intent
                        && same_revision
                        && !new_admission
                    {
                        Some(
                            AutonomousProgressEvidenceClass::
                                StableBlockingDiagnosticRepeated,
                        )
                    } else {
                        None
                    };
                    assert_eq!(
                        autonomous_stable_blocking_progress_evidence(
                            &previous,
                            &current,
                            new_admission,
                        ),
                        expected,
                    );
                    combinations += 1;
                    if expected.is_some() {
                        repeated += 1;
                    }
                }
            }
        }
    }
    assert_eq!(combinations, 16);
    assert_eq!(repeated, 1);
}
