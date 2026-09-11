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
//   - Regression evidence for frozen semantic history result interpretation.
// - Must-Not:
//   - Implement traversal, retry recovery, cancellation, transport, or wire
//     result mapping.
// - Allows:
//   - Inputs: Every frozen result class and every current traversal outcome.
//   - Outputs: Exhaustive commit and current-outcome projection assertions.
//   - Side effects: Test-only revision identity allocation.
// - Split-When:
//   - Stateful retry or cancellation fixtures require independent tests.
// - Merge-When:
//   - Final history result execution fixtures fully subsume these projections.
// - Summary:
//   - Pins history commit meaning without claiming blocked retry behavior.
// - Description:
//   - Proves current outcomes map only where frozen semantics are unambiguous.
// - Usage:
//   - Compile exhaustively against semantic-history-result application logic.
// - Defaults:
//   - NoAcceptedRevision remains intentionally unclassified.
//
use atrament_semantic_history_result::{
    SemanticHistoryCommitDisposition, SemanticHistoryResultClass,
    classify_history_traversal_result, semantic_history_commit_disposition,
    semantic_history_direction_is_available,
};
use atrament_semantic_notebook::{IdentityAllocator, IdentityExhausted};
use atrament_semantic_notebook_port::{
    HistoryAvailability, HistoryAvailabilityOutcome, HistoryDirection,
    HistoryTraversalOutcome,
};

const ALL_RESULT_CLASSES: [SemanticHistoryResultClass; 6] = [
    SemanticHistoryResultClass::CancelledBeforeCommit,
    SemanticHistoryResultClass::HistoryBoundary,
    SemanticHistoryResultClass::IdempotentReplay,
    SemanticHistoryResultClass::KnownNoCommitFailure,
    SemanticHistoryResultClass::StaleCurrentRevision,
    SemanticHistoryResultClass::Traversed,
];

#[test]
fn all_six_result_classes_have_exact_commit_disposition() {
    for result in ALL_RESULT_CLASSES {
        let expected = match result {
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
        };
        assert_eq!(semantic_history_commit_disposition(result), expected);
    }
}

#[test]
fn current_traversal_outcomes_project_only_unambiguous_classes() {
    let identities = IdentityAllocator::new();
    let base = identities.allocate_revision().expect("base revision");
    let current = identities.allocate_revision().expect("current revision");
    let traversed = identities.allocate_revision().expect("result revision");
    let cases = [
        (
            HistoryTraversalOutcome::Boundary {
                direction: HistoryDirection::Undo,
                revision: base,
            },
            Some(SemanticHistoryResultClass::HistoryBoundary),
        ),
        (
            HistoryTraversalOutcome::IdentityExhausted {
                sequence: IdentityExhausted::Revision,
            },
            Some(SemanticHistoryResultClass::KnownNoCommitFailure),
        ),
        (HistoryTraversalOutcome::NoAcceptedRevision, None),
        (
            HistoryTraversalOutcome::StaleBase {
                current,
                requested: base,
            },
            Some(SemanticHistoryResultClass::StaleCurrentRevision),
        ),
        (
            HistoryTraversalOutcome::Traversed {
                base,
                direction: HistoryDirection::Redo,
                revision: traversed,
            },
            Some(SemanticHistoryResultClass::Traversed),
        ),
    ];
    for (outcome, expected) in cases {
        assert_eq!(classify_history_traversal_result(&outcome), expected);
    }
}

#[test]
fn frozen_taxonomy_contains_exactly_six_core_result_classes() {
    assert_eq!(ALL_RESULT_CLASSES.len(), 6);
}

#[test]
fn all_direction_availability_states_match_exact_backend_facts() {
    let identities = IdentityAllocator::new();
    let revision = identities.allocate_revision().expect("revision identity");
    let mut cases = 0_usize;
    for can_redo in [false, true] {
        for can_undo in [false, true] {
            let availability = HistoryAvailabilityOutcome::Available(
                HistoryAvailability {
                    can_redo,
                    can_undo,
                    revision,
                },
            );
            for direction in [HistoryDirection::Redo, HistoryDirection::Undo] {
                let expected = match direction {
                    HistoryDirection::Redo => can_redo,
                    HistoryDirection::Undo => can_undo,
                };
                assert_eq!(
                    semantic_history_direction_is_available(
                        &availability,
                        direction,
                    ),
                    expected,
                );
                cases += 1;
            }
        }
    }
    let empty = HistoryAvailabilityOutcome::NoAcceptedRevision;
    for direction in [HistoryDirection::Redo, HistoryDirection::Undo] {
        assert!(!semantic_history_direction_is_available(&empty, direction));
        cases += 1;
    }
    assert_eq!(cases, 10);
}
