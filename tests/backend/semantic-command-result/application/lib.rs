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
//   - Regression evidence for frozen semantic result commit disposition.
// - Must-Not:
//   - Invent wire mappings, receipt semantics, retry storage, or transport
//     outcomes.
// - Allows:
//   - Inputs: Every frozen semantic command result class.
//   - Outputs: Exhaustive assertions over guaranteed commit disposition.
//   - Side effects: None.
// - Split-When:
//   - Receipt or retry fixtures require independent stateful application tests.
// - Merge-When:
//   - Final Apply result fixtures fully subsume this exhaustive mapping.
// - Summary:
//   - Pins commit effects for all core semantic command result classes.
// - Description:
//   - Keeps replay recovery distinct from both new commit and known no-commit.
// - Usage:
//   - Compile exhaustively against semantic-command-result application logic.
// - Defaults:
//   - Unknown transport outcome is absent because it is not a core result.
//
use atrament_semantic_command_result::semantic_command_commit_disposition;
use atrament_semantic_notebook_port::{
    SemanticCommandCommitDisposition, SemanticCommandResultClass,
};

const ALL_CORE_RESULT_CLASSES: [SemanticCommandResultClass; 15] = [
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

#[test]
fn all_core_result_classes_have_exact_commit_disposition() {
    for result in ALL_CORE_RESULT_CLASSES {
        let expected = match result {
            SemanticCommandResultClass::Applied => {
                SemanticCommandCommitDisposition::CommittedThisCall
            },
            SemanticCommandResultClass::IdempotentReplay => {
                SemanticCommandCommitDisposition::RecoveredPriorCompletion
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
            | SemanticCommandResultClass::WritableScopeViolation => {
                SemanticCommandCommitDisposition::KnownNoNewCommit
            },
        };
        assert_eq!(semantic_command_commit_disposition(result), expected);
    }
}
