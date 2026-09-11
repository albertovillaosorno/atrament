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
//   - Projection from frozen semantic command results to directly implied
//     autonomous progress evidence.
// - Must-Not:
//   - Execute Apply, infer repeated No-op, compare revisions or diagnostics,
//     retry, schedule work, or manufacture evidence for unrelated results.
// - Allows:
//   - Inputs: One completed semantic command result class.
//   - Outputs: Progress evidence only when the result implies it exactly.
//   - Side effects: None.
// - Split-When:
//   - Stateful revision or retry proof gains authority here.
// - Merge-When:
//   - One autonomous coordinator directly owns semantic result evidence.
// - Summary:
//   - Distinguishes new accepted Apply progress from replay recovery.
// - Description:
//   - Leaves results requiring extra state comparison unclassified.
// - Usage:
//   - Project a completed semantic result before progress/stop composition.
// - Defaults:
//   - Results other than Applied and Idempotent replay yield no evidence.
//

//! Semantic-command result projection into autonomous progress evidence.

use atrament_autonomous_progress_evidence::AutonomousProgressEvidenceClass;
use atrament_semantic_notebook_port::SemanticCommandResultClass;

/// Project directly implied semantic-result progress evidence.
#[must_use]
pub const fn autonomous_semantic_result_progress_evidence(
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
