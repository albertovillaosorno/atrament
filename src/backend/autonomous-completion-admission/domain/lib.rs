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
//   - Frozen autonomous completion admission from already-qualified goal and
//     requested-output satisfaction evidence.
// - Must-Not:
//   - Infer goal satisfaction, execute output, construct receipts, inspect
//     revisions, retry, schedule work, or create non-completion stop reasons.
// - Allows:
//   - Inputs: Semantic-goal satisfaction and requested-output completion state.
//   - Outputs: Completed terminal outcome only when both required axes
//     permit it.
//   - Side effects: None.
// - Split-When:
//   - Completion receipts or stateful satisfaction detection gain authority.
// - Merge-When:
//   - One autonomous coordinator directly owns exact completion construction.
// - Summary:
//   - Prevents incomplete semantic or explicitly requested output work from
//     being reported as successful goal completion.
// - Description:
//   - Encodes only the frozen conjunction required for autonomous completion.
// - Usage:
//   - Apply after owning boundaries qualify semantic and output satisfaction.
// - Defaults:
//   - Unsatisfied or incomplete evidence yields no terminal completion.
//

//! Autonomous completion admission from already-qualified satisfaction facts.

use atrament_autonomous_goal_outcome::AutonomousGoalTerminalClass;

/// Completion state of one explicitly requested output operation.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum AutonomousRequestedOutputItemCompletion {
    /// This requested output completed successfully.
    Complete,
    /// This requested output remains incomplete.
    Incomplete,
}

/// Completion state of outputs explicitly required by the admitted goal.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum AutonomousRequestedOutputCompletion {
    /// Every explicitly requested output completed successfully.
    Complete,
    /// At least one explicitly requested output remains incomplete.
    Incomplete,
    /// The admitted goal requested no output operation.
    NotRequested,
}

/// Aggregate already-qualified requested output items for goal completion.
///
/// An empty slice means the goal requested no output. Any incomplete requested
/// item keeps the aggregate incomplete; otherwise every requested item is
/// complete.
#[must_use]
pub fn autonomous_requested_outputs_completion(
    outputs: &[AutonomousRequestedOutputItemCompletion],
) -> AutonomousRequestedOutputCompletion {
    if outputs.is_empty() {
        AutonomousRequestedOutputCompletion::NotRequested
    } else if outputs.contains(
        &AutonomousRequestedOutputItemCompletion::Incomplete,
    ) {
        AutonomousRequestedOutputCompletion::Incomplete
    } else {
        AutonomousRequestedOutputCompletion::Complete
    }
}

/// Satisfaction state of the semantic portion of the admitted goal.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum AutonomousSemanticGoalCompletion {
    /// Current accepted semantic state satisfies the admitted goal.
    Satisfied,
    /// Current accepted semantic state does not satisfy the admitted goal.
    Unsatisfied,
}

/// Return successful completion only when all frozen completion axes permit it.
///
/// Inputs are already-qualified facts. This function never compares revisions,
/// evaluates semantic intent, runs outputs, or constructs completion receipts.
#[must_use]
pub const fn autonomous_completion_terminal_outcome(
    semantic: AutonomousSemanticGoalCompletion,
    output: AutonomousRequestedOutputCompletion,
) -> Option<AutonomousGoalTerminalClass> {
    match (semantic, output) {
        (
            AutonomousSemanticGoalCompletion::Satisfied,
            AutonomousRequestedOutputCompletion::Complete
            | AutonomousRequestedOutputCompletion::NotRequested,
        ) => Some(AutonomousGoalTerminalClass::Completed),
        (
            AutonomousSemanticGoalCompletion::Satisfied,
            AutonomousRequestedOutputCompletion::Incomplete,
        )
        | (AutonomousSemanticGoalCompletion::Unsatisfied, _) => None,
    }
}
