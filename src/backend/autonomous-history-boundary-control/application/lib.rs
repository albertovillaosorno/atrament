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
//   - Direction-aware local stop evidence for a semantic history boundary.
// - Must-Not:
//   - Traverse history, infer boundaries, stop the opposite direction, decide
//     goal completion, create terminal outcomes, retry, or mutate state.
// - Allows:
//   - Inputs: One completed in-memory history traversal outcome.
//   - Outputs: The exact direction that reached a history boundary, if any.
//   - Side effects: None.
// - Split-When:
//   - Stateful history scheduling or traversal gains authority here.
// - Merge-When:
//   - One coordinator directly owns direction-aware history retry control.
// - Summary:
//   - Stops blind retry only in the direction that actually hit its boundary.
// - Description:
//   - Keeps the opposite history direction independent and potentially valid.
// - Usage:
//   - Apply after one typed history traversal outcome is available.
// - Defaults:
//   - Non-boundary outcomes establish no direction-specific stop here.
//

//! Direction-aware local stop control for semantic history boundaries.

use atrament_semantic_notebook_port::{
    HistoryDirection, HistoryTraversalOutcome,
};

/// Return the exact history direction that must stop blind retry at a boundary.
#[must_use]
pub const fn autonomous_history_boundary_stop_direction(
    outcome: &HistoryTraversalOutcome,
) -> Option<HistoryDirection> {
    match outcome {
        HistoryTraversalOutcome::Boundary { direction, .. } => Some(*direction),
        HistoryTraversalOutcome::IdentityExhausted { .. }
        | HistoryTraversalOutcome::NoAcceptedRevision
        | HistoryTraversalOutcome::StaleBase { .. }
        | HistoryTraversalOutcome::Traversed { .. } => None,
    }
}
