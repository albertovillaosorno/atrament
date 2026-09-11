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
//   - Exhaustive evidence for repeated-No-op local mutation stop control.
// - Must-Not:
//   - Infer repetition, decide completion, create terminal outcomes, retry,
//     mutate state, or run a loop.
// - Allows:
//   - Inputs: All nine frozen autonomous progress-evidence classes.
//   - Outputs: Exact repeated-No-op stop-axis boolean for all nine classes.
//   - Side effects: None.
// - Split-When:
//   - Stateful repeated-No-op fixtures need independent execution evidence.
// - Merge-When:
//   - Coordinator tests fully subsume this finite control oracle.
// - Summary:
//   - Proves only qualified repeated No-op establishes this local stop axis.
// - Description:
//   - Keeps unrelated progress and non-progress evidence independent.
// - Usage:
//   - Cross-check every evidence class against an independent expected value.
// - Defaults:
//   - False never means permission to continue.
//
use atrament_autonomous_progress_evidence::AutonomousProgressEvidenceClass;
use atrament_autonomous_repeated_no_op_control::
    autonomous_repeated_no_op_requires_equivalent_mutation_stop;

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
fn all_nine_progress_evidence_classes_match_repeated_no_op_stop_axis() {
    let mut stops = 0_usize;
    for evidence in EVIDENCE {
        let expected = evidence
            == AutonomousProgressEvidenceClass::RepeatedNoOpSameIntent;
        assert_eq!(
            autonomous_repeated_no_op_requires_equivalent_mutation_stop(
                evidence
            ),
            expected
        );
        if expected {
            stops += 1;
        }
    }
    assert_eq!(stops, 1);
}
