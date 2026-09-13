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
//   - Transport-neutral interpretation of frozen semantic command results and
//     qualified Apply cancellation-boundary observations.
// - Must-Not:
//   - Choose wire names, normalize receipts, persist retry state, emit
//     diagnostics, mutate notebooks, or infer unknown transport outcomes.
// - Allows:
//   - Inputs: One completed core semantic command result class.
//   - Outputs: Frozen commit disposition implied by that result class.
//   - Side effects: None.
// - Split-When:
//   - Receipt normalization, retry recovery, or cancellation execution gains
//     executable authority.
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

use atrament_application_operation_lifecycle::{
    ApplicationCancellationDisposition, ApplicationCancellationObservation,
    ApplicationOperationClass, application_cancellation_resolution,
};
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
        }
        SemanticCommandResultClass::IdempotentReplay => {
            SemanticCommandCommitDisposition::RecoveredPriorCompletion
        }
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
        }
    }
}

/// Project one already-qualified Apply cancellation observation into the frozen
/// semantic command result taxonomy.
///
/// A request alone has no result. Cancellation proven before the accepted
/// semantic commit maps to `CancelledBeforeCommit`; a crossed commit remains
/// `Applied`. This does not signal cancellation, execute Apply, or implement
/// retry identity/recovery.
#[must_use]
pub const fn classify_semantic_apply_cancellation_result(
    observation: ApplicationCancellationObservation,
) -> Option<SemanticCommandResultClass> {
    match application_cancellation_resolution(
        ApplicationOperationClass::Apply,
        observation,
    ) {
        None => None,
        Some(resolution) => match resolution.disposition {
            ApplicationCancellationDisposition::CancelledBeforeEffect => {
                Some(SemanticCommandResultClass::CancelledBeforeCommit)
            }
            ApplicationCancellationDisposition::EffectRemainsAuthoritative => {
                Some(SemanticCommandResultClass::Applied)
            }
        },
    }
}
