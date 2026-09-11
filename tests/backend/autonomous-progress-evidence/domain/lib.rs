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
//   - Exhaustive evidence for frozen autonomous progress classifications.
// - Must-Not:
//   - Infer evidence, inspect state, retry, mutate, schedule, or run an agent.
// - Allows:
//   - Inputs: Every currently frozen qualified progress-evidence class.
//   - Outputs: Exact progress/non-progress assertion for every class.
//   - Side effects: None.
// - Split-When:
//   - Stateful revision/diagnostic comparison needs independent fixtures.
// - Merge-When:
//   - Executable loop tests subsume this static evidence classification.
// - Summary:
//   - Pins five progress and four non-progress classes without hidden state.
// - Description:
//   - Separates accepted/new evidence from replay, No-op, and diagnostic churn.
// - Usage:
//   - Compare all frozen qualified evidence classes with an independent oracle.
// - Defaults:
//   - Future evidence requires an explicit new contract decision.
//
use atrament_autonomous_progress_evidence::{
    AutonomousProgressDisposition, AutonomousProgressEvidenceClass,
    autonomous_progress_disposition,
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

fn expected(
    evidence: AutonomousProgressEvidenceClass,
) -> AutonomousProgressDisposition {
    match evidence {
        AutonomousProgressEvidenceClass::AcceptedApplyRevisionChange
        | AutonomousProgressEvidenceClass::AcceptedHistoryRevisionChange
        | AutonomousProgressEvidenceClass::BlockingDiagnosticResolved
        | AutonomousProgressEvidenceClass::ExplicitRequestedOutputCompleted
        | AutonomousProgressEvidenceClass::NewAdmittedAuthorityOrContext => {
            AutonomousProgressDisposition::Progress
        },
        AutonomousProgressEvidenceClass::IdempotentReplayRecovery
        | AutonomousProgressEvidenceClass::RepeatedNoOpSameIntent
        | AutonomousProgressEvidenceClass::RepeatedSameInputsWithoutNewAdmission
        | AutonomousProgressEvidenceClass::StableBlockingDiagnosticRepeated => {
            AutonomousProgressDisposition::NonProgress
        },
    }
}

#[test]
fn all_nine_frozen_evidence_classes_match_progress_oracle() {
    let mut progress = 0_usize;
    let mut non_progress = 0_usize;
    for evidence in EVIDENCE {
        let expected = expected(evidence);
        assert_eq!(autonomous_progress_disposition(evidence), expected);
        match expected {
            AutonomousProgressDisposition::NonProgress => non_progress += 1,
            AutonomousProgressDisposition::Progress => progress += 1,
        }
    }
    assert_eq!(progress, 5);
    assert_eq!(non_progress, 4);
}
