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
//   - Projection from budget-only mutation stop into autonomous terminal
//     outcome.
// - Must-Not:
//   - Count budgets, detect goal completion, execute work, retry, or invent any
//     terminal reason when the budget axis does not require a stop.
// - Allows:
//   - Inputs: One budget-only mutation disposition.
//   - Outputs: Optional exhausted-budget terminal outcome.
//   - Side effects: None.
// - Split-When:
//   - A full autonomous coordinator combines multiple terminal-condition axes.
// - Merge-When:
//   - The owning coordinator directly projects this exact budget stop reason.
// - Summary:
//   - Turns exhausted budget admission into the frozen exhausted-budget stop.
// - Description:
//   - Leaves non-budget terminal detection and successful completion untouched.
// - Usage:
//   - Project the already-computed budget disposition before coordinator merge.
// - Defaults:
//   - No budget-only stop yields no terminal outcome from this boundary.
//

//! Budget-stop projection into the frozen autonomous terminal vocabulary.

use atrament_autonomous_budget_admission::AutonomousBudgetMutationDisposition;
use atrament_autonomous_goal_outcome::AutonomousGoalTerminalClass;

/// Project budget-only stop evidence into the matching terminal outcome.
#[must_use]
pub const fn autonomous_budget_terminal_outcome(
    disposition: AutonomousBudgetMutationDisposition,
) -> Option<AutonomousGoalTerminalClass> {
    match disposition {
        AutonomousBudgetMutationDisposition::NoBudgetStop => None,
        AutonomousBudgetMutationDisposition::StopForExhaustedBudget => {
            Some(AutonomousGoalTerminalClass::StoppedExhaustedBudget)
        },
    }
}
