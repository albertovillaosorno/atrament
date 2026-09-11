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
//   - Transport-neutral interpretation of frozen semantic history results.
// - Must-Not:
//   - Traverse history, allocate revisions, choose wire names, persist retry
//     state, execute cancellation, or infer unknown transport outcomes.
// - Allows:
//   - Inputs: Frozen history result classes or current traversal outcomes.
//   - Outputs: Commit disposition and unambiguous current-outcome projection.
//   - Side effects: None.
// - Split-When:
//   - History retry or cancellation gains executable application authority.
// - Merge-When:
//   - Final history execution directly owns every result interpretation.
// - Summary:
//   - Separates history result meaning from traversal execution and transport.
// - Description:
//   - Keeps commit, replay recovery, boundary, stale, and no-commit distinct.
// - Usage:
//   - Classify completed history semantics before adapter projection.
// - Defaults:
//   - Missing accepted state remains unclassified by the frozen result cases.
//

//! Application semantics for frozen semantic history result classes.

use atrament_semantic_notebook_port::{
    HistoryAvailabilityOutcome, HistoryDirection, HistoryTraversalOutcome,
};

/// Frozen application-level result classes for accepted history traversal.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum SemanticHistoryResultClass {
    /// Admitted cancellation took effect before history commit.
    CancelledBeforeCommit,
    /// No traversal exists in the requested direction at this history position.
    HistoryBoundary,
    /// Same completed traversal and retry identity recovered prior completion.
    IdempotentReplay,
    /// Failure is proven not to have crossed the history commit boundary.
    KnownNoCommitFailure,
    /// Another accepted mutation changed the current revision or position.
    StaleCurrentRevision,
    /// Exactly one admitted Undo or Redo committed a fresh revision.
    Traversed,
}

/// Commit disposition guaranteed by one completed history result class.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SemanticHistoryCommitDisposition {
    /// This call committed exactly one accepted history traversal.
    CommittedThisCall,
    /// This call is known not to have committed a new history traversal.
    KnownNoNewCommit,
    /// Same-retry recovery returned one previously committed traversal.
    RecoveredPriorCompletion,
}

/// Classify the accepted-history effect implied by one frozen result class.
#[must_use]
pub const fn semantic_history_commit_disposition(
    result: SemanticHistoryResultClass,
) -> SemanticHistoryCommitDisposition {
    match result {
        SemanticHistoryResultClass::CancelledBeforeCommit
        | SemanticHistoryResultClass::HistoryBoundary
        | SemanticHistoryResultClass::KnownNoCommitFailure
        | SemanticHistoryResultClass::StaleCurrentRevision => {
            SemanticHistoryCommitDisposition::KnownNoNewCommit
        },
        SemanticHistoryResultClass::IdempotentReplay => {
            SemanticHistoryCommitDisposition::RecoveredPriorCompletion
        },
        SemanticHistoryResultClass::Traversed => {
            SemanticHistoryCommitDisposition::CommittedThisCall
        },
    }
}

/// Read one direction's currently admitted history availability.
///
/// Missing accepted state admits neither direction. This is read-only
/// interpretation of backend-owned availability, not a traversal attempt.
#[must_use]
pub const fn semantic_history_direction_is_available(
    availability: &HistoryAvailabilityOutcome,
    direction: HistoryDirection,
) -> bool {
    match availability {
        HistoryAvailabilityOutcome::Available(state) => match direction {
            HistoryDirection::Redo => state.can_redo,
            HistoryDirection::Undo => state.can_undo,
        },
        HistoryAvailabilityOutcome::NoAcceptedRevision => false,
    }
}

/// Project one current in-memory traversal outcome into frozen history
/// semantics.
///
/// Current history execution has no retry or cancellation implementation, so it
/// cannot produce those frozen classes yet. Missing accepted state has no final
/// history result assignment in the frozen contract and remains `None`.
#[must_use]
pub const fn classify_history_traversal_result(
    outcome: &HistoryTraversalOutcome,
) -> Option<SemanticHistoryResultClass> {
    match outcome {
        HistoryTraversalOutcome::Boundary { .. } => {
            Some(SemanticHistoryResultClass::HistoryBoundary)
        },
        HistoryTraversalOutcome::IdentityExhausted { .. } => {
            Some(SemanticHistoryResultClass::KnownNoCommitFailure)
        },
        HistoryTraversalOutcome::NoAcceptedRevision => None,
        HistoryTraversalOutcome::StaleBase { .. } => {
            Some(SemanticHistoryResultClass::StaleCurrentRevision)
        },
        HistoryTraversalOutcome::Traversed { .. } => {
            Some(SemanticHistoryResultClass::Traversed)
        },
    }
}
