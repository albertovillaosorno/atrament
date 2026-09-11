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
//   - Projection from frozen semantic-history results to directly implied
//     autonomous progress evidence.
// - Must-Not:
//   - Traverse history, infer repeated boundaries, compare revisions, retry,
//     schedule work, or manufacture evidence for unrelated results.
// - Allows:
//   - Inputs: One completed semantic-history result class.
//   - Outputs: Progress evidence only when the result implies it exactly.
//   - Side effects: None.
// - Split-When:
//   - Stateful history comparison or retry proof gains authority here.
// - Merge-When:
//   - One autonomous coordinator directly owns history-result evidence.
// - Summary:
//   - Distinguishes committed traversal progress from replay recovery.
// - Description:
//   - Leaves boundary, stale, cancellation, and no-commit results unclassified.
// - Usage:
//   - Project a completed history result before progress/stop composition.
// - Defaults:
//   - Only Traversed and Idempotent replay directly imply progress evidence.
//

//! Semantic-history result projection into autonomous progress evidence.

use atrament_autonomous_progress_evidence::AutonomousProgressEvidenceClass;
use atrament_semantic_history_result::SemanticHistoryResultClass;

/// Project directly implied history-result progress evidence.
#[must_use]
pub const fn autonomous_history_result_progress_evidence(
    result: SemanticHistoryResultClass,
) -> Option<AutonomousProgressEvidenceClass> {
    match result {
        SemanticHistoryResultClass::IdempotentReplay => {
            Some(AutonomousProgressEvidenceClass::IdempotentReplayRecovery)
        },
        SemanticHistoryResultClass::Traversed => {
            Some(AutonomousProgressEvidenceClass::AcceptedHistoryRevisionChange)
        },
        SemanticHistoryResultClass::CancelledBeforeCommit
        | SemanticHistoryResultClass::HistoryBoundary
        | SemanticHistoryResultClass::KnownNoCommitFailure
        | SemanticHistoryResultClass::StaleCurrentRevision => None,
    }
}
