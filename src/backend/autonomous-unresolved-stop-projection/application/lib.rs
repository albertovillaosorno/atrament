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
//   - Explicit owner resolution of one unrepresentable or unresolved result.
// - Must-Not:
//   - Broaden workflow authority, fabricate evidence, infer an owner decision,
//     mutate state, retry, or stop unrelated result classes.
// - Allows:
//   - Inputs: One semantic result and one explicit owning-caller resolution.
//   - Outputs: Unresolved terminal stop only for an explicit stop decision.
//   - Side effects: None.
// - Split-When:
//   - Broader-workflow negotiation gains executable application authority.
// - Merge-When:
//   - One coordinator directly owns unresolved escalation and terminal stops.
// - Summary:
//   - Keeps explicit broader-workflow requests distinct from unresolved stop.
// - Description:
//   - An unresolved result never chooses its own escalation or terminal state.
// - Usage:
//   - Apply after the owning caller chooses how to resolve unresolved intent.
// - Defaults:
//   - Requesting broader workflow or any unrelated result yields no stop here.
//

//! Explicit owner resolution for unrepresentable or unresolved semantic intent.

use atrament_autonomous_goal_outcome::AutonomousGoalTerminalClass;
use atrament_semantic_notebook_port::SemanticCommandResultClass;

/// Explicit owning-caller choice after unrepresentable or unresolved intent.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AutonomousUnresolvedResolution {
    /// Ask the owning caller for an explicitly broader admitted workflow.
    RequestBroaderWorkflow,
    /// Stop this bounded autonomous goal as unresolved.
    StopUnresolved,
}

/// Return unresolved terminal stop only for the exact result and explicit stop.
#[must_use]
pub const fn autonomous_unresolved_terminal_outcome(
    result: SemanticCommandResultClass,
    resolution: AutonomousUnresolvedResolution,
) -> Option<AutonomousGoalTerminalClass> {
    match (result, resolution) {
        (
            SemanticCommandResultClass::UnrepresentableOrUnresolved,
            AutonomousUnresolvedResolution::StopUnresolved,
        ) => Some(AutonomousGoalTerminalClass::StoppedUnresolvedEvidence),
        (
            _,
            AutonomousUnresolvedResolution::RequestBroaderWorkflow
            | AutonomousUnresolvedResolution::StopUnresolved,
        ) => None,
    }
}
