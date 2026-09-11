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
//   - Exhaustive evidence for autonomous-budget exhaustion and mutation stop.
// - Must-Not:
//   - Choose limits, count attempts, measure time, mutate state, or run agents.
// - Allows:
//   - Inputs: Every five-budget exhaustion mask plus duplicate evidence.
//   - Outputs: Exact per-axis facts and aggregate budget-only stop assertions.
//   - Side effects: None.
// - Split-When:
//   - Stateful budget accounting requires independent evidence.
// - Merge-When:
//   - Executable autonomous-loop tests subsume this budget-only oracle.
// - Summary:
//   - Pins all 32 masks without inventing numeric budget policy.
// - Description:
//   - Proves any exhaustion stops mutation on this axis and duplicates do not.
// - Usage:
//   - Compare finite exhaustion masks with an independent membership oracle.
// - Defaults:
//   - The empty mask imposes no budget-only mutation stop.
//
use atrament_autonomous_budget_admission::{
    AutonomousBudgetAdmission, AutonomousBudgetClass,
    AutonomousBudgetExhaustion,
    AutonomousBudgetMutationDisposition, autonomous_budget_admission,
};

const BUDGETS: [AutonomousBudgetClass; 5] = [
    AutonomousBudgetClass::Attempt,
    AutonomousBudgetClass::ElapsedTime,
    AutonomousBudgetClass::ModelCall,
    AutonomousBudgetClass::Output,
    AutonomousBudgetClass::Resource,
];

fn expected_exhaustion(
    exhausted: &[AutonomousBudgetClass],
    budget: AutonomousBudgetClass,
) -> AutonomousBudgetExhaustion {
    if exhausted.contains(&budget) {
        AutonomousBudgetExhaustion::Exhausted
    } else {
        AutonomousBudgetExhaustion::NotExhausted
    }
}

fn expected(exhausted: &[AutonomousBudgetClass]) -> AutonomousBudgetAdmission {
    AutonomousBudgetAdmission {
        attempt: expected_exhaustion(exhausted, AutonomousBudgetClass::Attempt),
        elapsed_time: expected_exhaustion(
            exhausted,
            AutonomousBudgetClass::ElapsedTime,
        ),
        model_call: expected_exhaustion(
            exhausted,
            AutonomousBudgetClass::ModelCall,
        ),
        mutation_disposition: if exhausted.is_empty() {
            AutonomousBudgetMutationDisposition::NoBudgetStop
        } else {
            AutonomousBudgetMutationDisposition::StopForExhaustedBudget
        },
        output: expected_exhaustion(exhausted, AutonomousBudgetClass::Output),
        resource: expected_exhaustion(
            exhausted,
            AutonomousBudgetClass::Resource,
        ),
    }
}

#[test]
fn all_32_budget_masks_match_independent_stop_oracle() {
    let mut cases = 0_usize;
    for mask in 0_u8..32 {
        let exhausted: Vec<_> = BUDGETS
            .into_iter()
            .enumerate()
            .filter_map(|(index, budget)| {
                (mask & (1_u8 << index) != 0).then_some(budget)
            })
            .collect();
        assert_eq!(
            autonomous_budget_admission(&exhausted),
            expected(&exhausted),
        );
        cases += 1;
    }
    assert_eq!(cases, 32);
}

#[test]
fn duplicate_exhaustion_evidence_does_not_change_budget_meaning() {
    let exhausted = [
        AutonomousBudgetClass::Attempt,
        AutonomousBudgetClass::Attempt,
    ];
    assert_eq!(autonomous_budget_admission(&exhausted), expected(&exhausted));
}
