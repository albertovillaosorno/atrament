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
//   - Transport-neutral interpretation of frozen semantic command results.
// - Must-Not:
//   - Choose wire names, normalize receipts, persist retry state, emit
//     diagnostics, mutate notebooks, or infer unknown transport outcomes.
// - Allows:
//   - Inputs: One completed core semantic command result class.
//   - Outputs: Frozen commit disposition implied by that result class.
//   - Side effects: None.
// - Split-When:
//   - Receipt normalization or retry recovery gains executable authority.
// - Merge-When:
//   - Final semantic Apply owns every result interpretation directly.
// - Summary:
//   - Separates known commit effects from transport and receipt concerns.
// - Description:
//   - Distinguishes current-call commit, no-new-commit, and replay recovery.
// - Usage:
//   - Classify a completed core result before automation branches on commit.
// - Defaults:
//   - No result class implies transport failure or an unobserved commit.
//

//! Application semantics for frozen semantic command result classes.

use atrament_semantic_notebook_port::{
    SemanticCommandCommitDisposition, SemanticCommandResultClass,
};

/// Classify the accepted-commit effect guaranteed by one core result class.
///
/// Idempotent replay recovers a prior completion but never claims that the
/// current call committed again. This does not infer whether the recovered
/// prior completion originally produced a new revision or a no-op.
#[must_use]
pub const fn semantic_command_commit_disposition(
    result: SemanticCommandResultClass,
) -> SemanticCommandCommitDisposition {
    match result {
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
    }
}
