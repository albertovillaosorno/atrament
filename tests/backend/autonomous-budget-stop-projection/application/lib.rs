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
//   - Exhaustive budget-mask evidence for exhausted-budget terminal projection.
// - Must-Not:
//   - Detect other terminal reasons, count budgets, execute, retry, or mutate.
// - Allows:
//   - Inputs: Every five-budget exhaustion mask through budget admission.
//   - Outputs: Exact optional terminal outcome for all 32 masks.
//   - Side effects: None.
// - Split-When:
//   - Multi-axis autonomous coordinator fixtures require independent tests.
// - Merge-When:
//   - Coordinator tests fully subsume this exact budget-stop projection.
// - Summary:
//   - Proves one empty mask is nonterminal and 31 exhausted masks stop.
// - Description:
//   - Composes budget evidence with terminal vocabulary without hidden policy.
// - Usage:
//   - Derive each disposition through the real budget domain before projection.
// - Defaults:
//   - Empty exhaustion evidence cannot fabricate a terminal result.
//
use atrament_autonomous_budget_admission::{
    AutonomousBudgetClass, autonomous_budget_admission,
};
use atrament_autonomous_budget_stop_projection::{
    autonomous_budget_terminal_outcome,
};
use atrament_autonomous_goal_outcome::AutonomousGoalTerminalClass;

const BUDGETS: [AutonomousBudgetClass; 5] = [
    AutonomousBudgetClass::Attempt,
    AutonomousBudgetClass::ElapsedTime,
    AutonomousBudgetClass::ModelCall,
    AutonomousBudgetClass::Output,
    AutonomousBudgetClass::Resource,
];

#[test]
fn all_32_budget_masks_project_only_exhausted_masks_to_terminal_stop() {
    let mut no_terminal = 0_usize;
    let mut stopped = 0_usize;
    for mask in 0_u8..32 {
        let exhausted: Vec<_> = BUDGETS
            .into_iter()
            .enumerate()
            .filter_map(|(index, budget)| {
                (mask & (1_u8 << index) != 0).then_some(budget)
            })
            .collect();
        let admission = autonomous_budget_admission(&exhausted);
        let projected =
            autonomous_budget_terminal_outcome(admission.mutation_disposition);
        if exhausted.is_empty() {
            assert_eq!(projected, None);
            no_terminal += 1;
        } else {
            assert_eq!(
                projected,
                Some(AutonomousGoalTerminalClass::StoppedExhaustedBudget),
            );
            stopped += 1;
        }
    }
    assert_eq!(no_terminal, 1);
    assert_eq!(stopped, 31);
}
