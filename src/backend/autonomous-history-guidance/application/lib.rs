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
//   - Frozen autonomous guidance for typed semantic-history result classes.
// - Must-Not:
//   - Traverse history, retry automatically, choose budgets, execute
//     cancellation, widen authority, schedule loops, or infer transport loss.
// - Allows:
//   - Inputs: One completed semantic history result class.
//   - Outputs: One frozen guidance class when the contracts define it exactly.
//   - Side effects: None.
// - Split-When:
//   - Executable autonomous history orchestration gains application authority.
// - Merge-When:
//   - A final automation coordinator directly owns all history-result guidance.
// - Summary:
//   - Prevents stale/boundary/replay outcomes from becoming blind retries.
// - Description:
//   - Keeps successful traversal, replay recovery, boundary, and stale
//     distinct.
// - Usage:
//   - Interpret one typed history result before considering another traversal.
// - Defaults:
//   - Cancellation and known-no-commit failure require owning workflow policy.
//

//! Frozen partial autonomous guidance for semantic-history outcomes.

use atrament_semantic_history_result::SemanticHistoryResultClass;

/// Frozen autonomous next-step class for accepted-history outcomes.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AutonomousHistoryGuidance {
    /// Continue from the fresh revision produced by the committed traversal.
    ContinueFromReportedRevision,
    /// Inspect the current revision/history position before choosing again.
    FreshInspection,
    /// Prior completion was recovered; do not traverse again as fake progress.
    RecoveredPriorCompletion,
    /// Current direction is at its history boundary; do not blind-retry it.
    StopAtHistoryBoundary,
}

/// Return history guidance only where the frozen contracts define one branch.
///
/// Cancellation before commit and known-no-commit failure return `None` because
/// the owning workflow still decides whether to stop, inspect, or issue a new
/// request from current authority.
#[must_use]
pub const fn autonomous_history_guidance(
    result: SemanticHistoryResultClass,
) -> Option<AutonomousHistoryGuidance> {
    match result {
        SemanticHistoryResultClass::HistoryBoundary => {
            Some(AutonomousHistoryGuidance::StopAtHistoryBoundary)
        },
        SemanticHistoryResultClass::IdempotentReplay => {
            Some(AutonomousHistoryGuidance::RecoveredPriorCompletion)
        },
        SemanticHistoryResultClass::StaleCurrentRevision => {
            Some(AutonomousHistoryGuidance::FreshInspection)
        },
        SemanticHistoryResultClass::Traversed => {
            Some(AutonomousHistoryGuidance::ContinueFromReportedRevision)
        },
        SemanticHistoryResultClass::CancelledBeforeCommit
        | SemanticHistoryResultClass::KnownNoCommitFailure => None,
    }
}
