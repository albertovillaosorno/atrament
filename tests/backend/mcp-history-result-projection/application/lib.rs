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
//   - Exhaustive evidence for MCP semantic-history result projection.
// - Must-Not:
//   - Traverse history, expose MCP tools, simulate retry/cancel, or mutate.
// - Allows:
//   - Inputs: All MCP capabilities, six results, and ten availability cases.
//   - Outputs: Exact optional commit/availability projection assertions.
//   - Side effects: Test-only revision identity allocation.
// - Split-When:
//   - Executable MCP history adapter parity needs independent fixtures.
// - Merge-When:
//   - Live MCP history tests fully subsume this structural projection.
// - Summary:
//   - Pins 48 result pairs and 80 directional availability combinations.
// - Description:
//   - Proves only HistoryTraversal can expose shared history semantics.
// - Usage:
//   - Compare complete finite cross-products with independent expected values.
// - Defaults:
//   - Non-history capabilities never acquire history authority by vocabulary.
//
use atrament_application_operation_lifecycle::{
    ApplicationCancellationObservation,
};
use atrament_mcp_capability_effect::McpApplicationCapabilityClass;
use atrament_mcp_history_result_projection::{
    mcp_history_cancellation_result, mcp_history_commit_disposition,
    mcp_history_direction_is_available,
};
use atrament_semantic_history_result::{
    SemanticHistoryCommitDisposition, SemanticHistoryResultClass,
};
use atrament_semantic_notebook::IdentityAllocator;
use atrament_semantic_notebook_port::{
    HistoryAvailability, HistoryAvailabilityOutcome, HistoryDirection,
};

const CAPABILITIES: [McpApplicationCapabilityClass; 8] = [
    McpApplicationCapabilityClass::Apply,
    McpApplicationCapabilityClass::CommandContext,
    McpApplicationCapabilityClass::Export,
    McpApplicationCapabilityClass::HistoryTraversal,
    McpApplicationCapabilityClass::Inspect,
    McpApplicationCapabilityClass::Plan,
    McpApplicationCapabilityClass::Render,
    McpApplicationCapabilityClass::Validate,
];

const RESULTS: [SemanticHistoryResultClass; 6] = [
    SemanticHistoryResultClass::CancelledBeforeCommit,
    SemanticHistoryResultClass::HistoryBoundary,
    SemanticHistoryResultClass::IdempotentReplay,
    SemanticHistoryResultClass::KnownNoCommitFailure,
    SemanticHistoryResultClass::StaleCurrentRevision,
    SemanticHistoryResultClass::Traversed,
];

fn expected_disposition(
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

#[test]
fn all_48_capability_result_pairs_project_only_through_history() {
    let mut cases = 0_usize;
    for capability in CAPABILITIES {
        for result in RESULTS {
            let expected = (capability
                == McpApplicationCapabilityClass::HistoryTraversal)
                .then(|| expected_disposition(result));
            assert_eq!(
                mcp_history_commit_disposition(capability, result),
                expected,
                "capability={capability:?} result={result:?}",
            );
            cases += 1;
        }
    }
    assert_eq!(cases, 48);
}

#[test]
fn all_80_capability_and_direction_availability_cases_are_exact() {
    let identities = IdentityAllocator::new();
    let revision = identities.allocate_revision().expect("revision identity");
    let mut availabilities = Vec::new();
    for can_redo in [false, true] {
        for can_undo in [false, true] {
            availabilities.push(HistoryAvailabilityOutcome::Available(
                HistoryAvailability {
                    can_redo,
                    can_undo,
                    revision,
                },
            ));
        }
    }
    availabilities.push(HistoryAvailabilityOutcome::NoAcceptedRevision);
    let mut cases = 0_usize;
    for capability in CAPABILITIES {
        for availability in &availabilities {
            for direction in [HistoryDirection::Redo, HistoryDirection::Undo] {
                let fact = match availability {
                    HistoryAvailabilityOutcome::Available(value) => {
                        match direction {
                            HistoryDirection::Redo => value.can_redo,
                            HistoryDirection::Undo => value.can_undo,
                        }
                    },
                    HistoryAvailabilityOutcome::NoAcceptedRevision => false,
                };
                let expected = (capability
                    == McpApplicationCapabilityClass::HistoryTraversal)
                    .then_some(fact);
                assert_eq!(
                    mcp_history_direction_is_available(
                        capability,
                        availability,
                        direction,
                    ),
                    expected,
                );
                cases += 1;
            }
        }
    }
    assert_eq!(cases, 80);
}

#[test]
fn all_24_capability_cancellation_pairs_project_only_through_history() {
    const OBSERVATIONS: [ApplicationCancellationObservation; 3] = [
        ApplicationCancellationObservation::EffectBoundaryCrossed,
        ApplicationCancellationObservation::RequestOnly,
        ApplicationCancellationObservation::TookEffectBeforeBoundary,
    ];
    let mut cases = 0_usize;
    for capability in CAPABILITIES {
        for observation in OBSERVATIONS {
            let expected = if capability
                != McpApplicationCapabilityClass::HistoryTraversal
            {
                None
            } else {
                match observation {
                    ApplicationCancellationObservation::RequestOnly => None,
                    ApplicationCancellationObservation::
                        TookEffectBeforeBoundary => Some(
                        SemanticHistoryResultClass::CancelledBeforeCommit,
                    ),
                    ApplicationCancellationObservation::
                        EffectBoundaryCrossed => {
                        Some(SemanticHistoryResultClass::Traversed)
                    },
                }
            };
            assert_eq!(
                mcp_history_cancellation_result(capability, observation),
                expected,
                "capability={capability:?} observation={observation:?}",
            );
            cases += 1;
        }
    }
    assert_eq!(cases, 24);
}
