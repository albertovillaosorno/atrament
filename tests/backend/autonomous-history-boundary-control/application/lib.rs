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
//   - Exhaustive current-outcome evidence for direction-aware boundary control.
// - Must-Not:
//   - Traverse history, infer boundaries, decide completion, retry, or mutate.
// - Allows:
//   - Inputs: Both boundary directions plus every other current outcome class.
//   - Outputs: Exact stopped direction or no boundary stop.
//   - Side effects: None.
// - Split-When:
//   - Stateful history scheduling needs independent execution fixtures.
// - Merge-When:
//   - Coordinator tests fully subsume this finite outcome oracle.
// - Summary:
//   - Proves only the boundary direction is selected for blind-retry stop.
// - Description:
//   - The opposite direction is never inferred as stopped by this boundary.
// - Usage:
//   - Compare representative typed outcomes with an independent expected value.
// - Defaults:
//   - Non-boundary outcomes establish no direction-specific stop.
//
use atrament_autonomous_history_boundary_control::
    autonomous_history_boundary_stop_direction;
use atrament_semantic_notebook::{IdentityAllocator, IdentityExhausted};
use atrament_semantic_notebook_port::{
    HistoryDirection, HistoryTraversalOutcome,
};

#[test]
fn current_history_outcomes_preserve_exact_boundary_direction_only() {
    let identities = IdentityAllocator::new();
    let base = identities.allocate_revision().expect("base revision");
    let current = identities.allocate_revision().expect("current revision");
    let traversed = identities.allocate_revision().expect("result revision");
    let cases = [
        (
            HistoryTraversalOutcome::Boundary {
                direction: HistoryDirection::Redo,
                revision: base,
            },
            Some(HistoryDirection::Redo),
        ),
        (
            HistoryTraversalOutcome::Boundary {
                direction: HistoryDirection::Undo,
                revision: base,
            },
            Some(HistoryDirection::Undo),
        ),
        (
            HistoryTraversalOutcome::IdentityExhausted {
                sequence: IdentityExhausted::Revision,
            },
            None,
        ),
        (HistoryTraversalOutcome::NoAcceptedRevision, None),
        (
            HistoryTraversalOutcome::StaleBase {
                current,
                requested: base,
            },
            None,
        ),
        (
            HistoryTraversalOutcome::Traversed {
                base,
                direction: HistoryDirection::Undo,
                revision: traversed,
            },
            None,
        ),
    ];
    for (outcome, expected) in cases {
        assert_eq!(
            autonomous_history_boundary_stop_direction(&outcome),
            expected
        );
    }
}
