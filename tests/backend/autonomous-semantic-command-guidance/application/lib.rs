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
//   - Exhaustive evidence for frozen partial semantic-command automation
//     advice.
// - Must-Not:
//   - Execute retries, schedule loops, choose budgets, or widen authority.
// - Allows:
//   - Inputs: All 15 frozen semantic command result classes.
//   - Outputs: Exact guidance-or-unclassified assertions for every result.
//   - Side effects: None.
// - Split-When:
//   - Stateful autonomous-loop fixtures require independent evidence.
// - Merge-When:
//   - Executable loop parity tests fully subsume this structural
//     classification.
// - Summary:
//   - Pins the 12 explicit branches and three deliberately unclassified cases.
// - Description:
//   - Uses an independent exhaustive oracle over the frozen result vocabulary.
// - Usage:
//   - Compare every result with the contract-defined autonomous next step.
// - Defaults:
//   - Cancellation, validation success, and internal no-commit stay
//     unclassified.
//
use atrament_autonomous_semantic_command_guidance::{
    AutonomousSemanticCommandGuidance, autonomous_semantic_command_guidance,
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

fn expected(
    result: SemanticCommandResultClass,
) -> Option<AutonomousSemanticCommandGuidance> {
    match result {
        SemanticCommandResultClass::Applied
        | SemanticCommandResultClass::IdempotentReplay
        | SemanticCommandResultClass::NoOp => {
            Some(
                AutonomousSemanticCommandGuidance::ContinueFromReportedRevision,
            )
        },
        SemanticCommandResultClass::CommandContextMismatch
        | SemanticCommandResultClass::StaleBase => {
            Some(AutonomousSemanticCommandGuidance::FreshInspectionOrContext)
        },
        SemanticCommandResultClass::DependencyGraphRejection
        | SemanticCommandResultClass::SemanticValidationRejection
        | SemanticCommandResultClass::WritableScopeViolation => {
            Some(AutonomousSemanticCommandGuidance::CorrectRequestOrContext)
        },
        SemanticCommandResultClass::ResourceLimitRejection => {
            Some(AutonomousSemanticCommandGuidance::SmallerOrAlternateWorkflow)
        },
        SemanticCommandResultClass::RetryConflict => Some(
            AutonomousSemanticCommandGuidance::CorrectRetryBookkeepingOrStop,
        ),
        SemanticCommandResultClass::UnrepresentableOrUnresolved => Some(
            AutonomousSemanticCommandGuidance::StopOrRequestBroaderWorkflow,
        ),
        SemanticCommandResultClass::UnsupportedProtocolOrCapability => {
            Some(AutonomousSemanticCommandGuidance::CompatibilityNegotiation)
        },
        SemanticCommandResultClass::CancelledBeforeCommit
        | SemanticCommandResultClass::InternalFailureKnownNoCommit
        | SemanticCommandResultClass::SuccessfulValidation => None,
    }
}

#[test]
fn all_15_result_classes_match_frozen_partial_guidance() {
    let mut classified = 0_usize;
    for result in RESULTS {
        let expected = expected(result);
        assert_eq!(autonomous_semantic_command_guidance(result), expected);
        if expected.is_some() {
            classified += 1;
        }
    }
    assert_eq!(classified, 12);
}
