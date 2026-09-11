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
//   - Transport-neutral terminal outcome vocabulary for one autonomous goal.
// - Must-Not:
//   - Decide goal satisfaction, inspect revisions, execute work, create
//     receipts,
//     choose budgets, retry, persist reasoning, or broaden any authority.
// - Allows:
//   - Inputs: One terminal outcome class already established by an owner.
//   - Outputs: Successful-completion versus stopped-without-completion meaning.
//   - Side effects: None.
// - Split-When:
//   - Completion proof or stop-condition detection gains application authority.
// - Merge-When:
//   - One autonomous coordinator directly owns terminal result construction.
// - Summary:
//   - Keeps successful completion distinct from four frozen stop reasons.
// - Description:
//   - Prevents stopped loops from being reported as successfully completed.
// - Usage:
//   - Project an already-established terminal reason into completion semantics.
// - Defaults:
//   - Nonterminal loop state is outside this terminal-only vocabulary.
//

//! Terminal outcome semantics for one bounded autonomous goal.

/// Terminal outcome class frozen by the autonomous-loop contract.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum AutonomousGoalTerminalClass {
    /// The caller's admitted goal is satisfied by accepted state and outputs.
    Completed,
    /// Automation stopped because an admitted budget was exhausted.
    StoppedExhaustedBudget,
    /// Automation stopped on a stable blocking failure without new progress.
    StoppedStableBlockingFailure,
    /// Automation stopped on an unavailable required capability.
    StoppedUnavailableCapability,
    /// Automation stopped on unresolved evidence or intent.
    StoppedUnresolvedEvidence,
}

/// Completion meaning of one already-established terminal class.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AutonomousGoalTerminalDisposition {
    /// The loop stopped without establishing successful completion.
    StoppedWithoutCompletion,
    /// The terminal state is successful completion of the admitted goal.
    SuccessfulCompletion,
}

/// Project a terminal class into successful-completion versus stopped meaning.
#[must_use]
pub const fn autonomous_goal_terminal_disposition(
    terminal: AutonomousGoalTerminalClass,
) -> AutonomousGoalTerminalDisposition {
    match terminal {
        AutonomousGoalTerminalClass::Completed => {
            AutonomousGoalTerminalDisposition::SuccessfulCompletion
        },
        AutonomousGoalTerminalClass::StoppedExhaustedBudget
        | AutonomousGoalTerminalClass::StoppedStableBlockingFailure
        | AutonomousGoalTerminalClass::StoppedUnavailableCapability
        | AutonomousGoalTerminalClass::StoppedUnresolvedEvidence => {
            AutonomousGoalTerminalDisposition::StoppedWithoutCompletion
        },
    }
}
