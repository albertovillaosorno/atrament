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
//   - Exhaustive evidence for semantic-result progress projection.
// - Must-Not:
//   - Execute Apply, infer repeated No-op, retry, compare state, or run loops.
// - Allows:
//   - Inputs: All 15 frozen semantic command result classes.
//   - Outputs: Exact directly-implied progress-evidence-or-none assertion.
//   - Side effects: None.
// - Split-When:
//   - Stateful progress fixtures require independent evidence.
// - Merge-When:
//   - Coordinator tests fully subsume this finite projection oracle.
// - Summary:
//   - Pins Applied as revision progress and replay as recovery non-progress.
// - Description:
//   - Leaves all result classes requiring more context unclassified.
// - Usage:
//   - Compare every semantic result with an independent expected mapping.
// - Defaults:
//   - Only Applied and Idempotent replay directly imply progress evidence.
//
use atrament_autonomous_progress_evidence::AutonomousProgressEvidenceClass;
use atrament_autonomous_semantic_progress_projection::
    autonomous_semantic_result_progress_evidence;
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

fn expected(
    result: SemanticCommandResultClass,
) -> Option<AutonomousProgressEvidenceClass> {
    match result {
        SemanticCommandResultClass::Applied => {
            Some(AutonomousProgressEvidenceClass::AcceptedApplyRevisionChange)
        },
        SemanticCommandResultClass::IdempotentReplay => {
            Some(AutonomousProgressEvidenceClass::IdempotentReplayRecovery)
        },
        SemanticCommandResultClass::CancelledBeforeCommit
        | SemanticCommandResultClass::CommandContextMismatch
        | SemanticCommandResultClass::DependencyGraphRejection
        | SemanticCommandResultClass::InternalFailureKnownNoCommit
        | SemanticCommandResultClass::NoOp
        | SemanticCommandResultClass::ResourceLimitRejection
        | SemanticCommandResultClass::RetryConflict
        | SemanticCommandResultClass::SemanticValidationRejection
        | SemanticCommandResultClass::StaleBase
        | SemanticCommandResultClass::SuccessfulValidation
        | SemanticCommandResultClass::UnrepresentableOrUnresolved
        | SemanticCommandResultClass::UnsupportedProtocolOrCapability
        | SemanticCommandResultClass::WritableScopeViolation => None,
    }
}

#[test]
fn all_15_semantic_results_match_direct_progress_projection() {
    let mut classified = 0_usize;
    for result in RESULTS {
        let expected = expected(result);
        assert_eq!(
            autonomous_semantic_result_progress_evidence(result),
            expected
        );
        if expected.is_some() {
            classified += 1;
        }
    }
    assert_eq!(classified, 2);
}
