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
//   - Exhaustive evidence for repeated-No-op detection and local mutation-stop
//     control.
// - Must-Not:
//   - Infer intent identity or evidence relevance, decide completion, create
//     terminal outcomes, retry, mutate state, or run a loop.
// - Allows:
//   - Inputs: All semantic-result pairs crossed with intent equality and
//     new-evidence state, plus all nine progress-evidence classes.
//   - Outputs: Exact repeated-No-op evidence and stop-axis assertions.
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
use atrament_autonomous_repeated_no_op_control::{
    AutonomousNoOpObservation, autonomous_repeated_no_op_progress_evidence,
    autonomous_repeated_no_op_requires_equivalent_mutation_stop,
};
use atrament_semantic_notebook_port::SemanticCommandResultClass;

const RESULTS: [SemanticCommandResultClass; 15] = [
    SemanticCommandResultClass::Applied,
    SemanticCommandResultClass::CancelledBeforeCommit,
    SemanticCommandResultClass::CommandContextMismatch,
    SemanticCommandResultClass::DependencyGraphRejection,
    SemanticCommandResultClass::IdempotentReplay,
    SemanticCommandResultClass::InternalFailureKnownNoCommit,
    SemanticCommandResultClass::NoOp,
    SemanticCommandResultClass::ResourceLimitRejection,
    SemanticCommandResultClass::RetryConflict,
    SemanticCommandResultClass::SemanticValidationRejection,
    SemanticCommandResultClass::StaleBase,
    SemanticCommandResultClass::SuccessfulValidation,
    SemanticCommandResultClass::UnrepresentableOrUnresolved,
    SemanticCommandResultClass::UnsupportedProtocolOrCapability,
    SemanticCommandResultClass::WritableScopeViolation,
];

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
        let expected =
            evidence == AutonomousProgressEvidenceClass::RepeatedNoOpSameIntent;
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

#[test]
fn all_900_result_intent_evidence_states_match_repeated_no_op_rule() {
    let mut combinations = 0_usize;
    let mut repeated = 0_usize;
    for previous_result in RESULTS {
        for current_result in RESULTS {
            for same_intent in [false, true] {
                for new_relevant_evidence in [false, true] {
                    let previous = AutonomousNoOpObservation {
                        intent: 1_u8,
                        result: previous_result,
                    };
                    let current = AutonomousNoOpObservation {
                        intent: if same_intent { 1 } else { 2 },
                        result: current_result,
                    };
                    let expected = if previous_result
                        == SemanticCommandResultClass::NoOp
                        && current_result == SemanticCommandResultClass::NoOp
                        && same_intent
                        && !new_relevant_evidence
                    {
                        Some(
                            AutonomousProgressEvidenceClass::
                                RepeatedNoOpSameIntent,
                        )
                    } else {
                        None
                    };
                    assert_eq!(
                        autonomous_repeated_no_op_progress_evidence(
                            &previous,
                            &current,
                            new_relevant_evidence,
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
    assert_eq!(combinations, 900);
    assert_eq!(repeated, 1);
}
