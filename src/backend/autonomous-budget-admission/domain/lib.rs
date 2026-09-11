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
//   - Transport-neutral autonomous-budget exhaustion vocabulary and stop gate.
// - Must-Not:
//   - Choose numeric limits, count attempts, measure time, schedule work,
//     mutate state, widen authority, auto-accept partial results, or run
//     agents.
// - Allows:
//   - Inputs: Caller/backend-supplied exhausted budget classes for one goal.
//   - Outputs: Per-axis exhaustion facts plus the frozen budget-only stop fact.
//   - Side effects: None.
// - Split-When:
//   - Budget accounting or scheduling gains executable application authority.
// - Merge-When:
//   - One autonomous-loop owner directly owns all budget evidence and
//     admission.
// - Summary:
//   - Stops autonomous mutation after any declared budget is exhausted.
// - Description:
//   - Preserves five independent exhaustion axes without freezing limit values.
// - Usage:
//   - Evaluate after host/backend budget evidence exists and before mutation.
// - Defaults:
//   - No exhausted budget means this axis alone imposes no mutation stop.
//

//! Budget-only stop semantics for one bounded autonomous goal.

/// Budget class explicitly frozen by the autonomous-loop contract.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum AutonomousBudgetClass {
    /// Bound on admitted autonomous attempts.
    Attempt,
    /// Bound on elapsed autonomous-goal time.
    ElapsedTime,
    /// Bound on model invocations.
    ModelCall,
    /// Bound on output work admitted for the goal.
    Output,
    /// Bound on backend/application resources admitted for the goal.
    Resource,
}

/// Exhaustion fact for one budget class.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AutonomousBudgetExhaustion {
    /// The owning host/backend reports this budget exhausted.
    Exhausted,
    /// The owning host/backend does not report this budget exhausted.
    NotExhausted,
}

/// Mutation disposition implied only by the autonomous budget axis.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AutonomousBudgetMutationDisposition {
    /// Budget evidence alone does not require stopping mutation.
    NoBudgetStop,
    /// At least one budget is exhausted, so further mutation must stop.
    StopForExhaustedBudget,
}

/// Exhaustion facts retained independently for all five frozen budget classes.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AutonomousBudgetAdmission {
    /// Attempt-budget exhaustion.
    pub attempt: AutonomousBudgetExhaustion,
    /// Elapsed-time-budget exhaustion.
    pub elapsed_time: AutonomousBudgetExhaustion,
    /// Model-call-budget exhaustion.
    pub model_call: AutonomousBudgetExhaustion,
    /// Budget-only mutation stop disposition.
    pub mutation_disposition: AutonomousBudgetMutationDisposition,
    /// Output-budget exhaustion.
    pub output: AutonomousBudgetExhaustion,
    /// Resource-budget exhaustion.
    pub resource: AutonomousBudgetExhaustion,
}

const fn exhaustion(
    exhausted_budgets: &[AutonomousBudgetClass],
    budget: AutonomousBudgetClass,
) -> AutonomousBudgetExhaustion {
    let mut index = 0_usize;
    while index < exhausted_budgets.len() {
        if exhausted_budgets[index] as u8 == budget as u8 {
            return AutonomousBudgetExhaustion::Exhausted;
        }
        index += 1;
    }
    AutonomousBudgetExhaustion::NotExhausted
}

/// Project all five budget exhaustion facts plus the budget-only mutation gate.
///
/// This does not prove mutation is otherwise admitted. Revision, capability,
/// context, scope, validation, retry, output, and physical authority remain
/// with
/// their owning boundaries.
#[must_use]
pub const fn autonomous_budget_admission(
    exhausted_budgets: &[AutonomousBudgetClass],
) -> AutonomousBudgetAdmission {
    let attempt = exhaustion(exhausted_budgets, AutonomousBudgetClass::Attempt);
    let elapsed_time =
        exhaustion(exhausted_budgets, AutonomousBudgetClass::ElapsedTime);
    let model_call =
        exhaustion(exhausted_budgets, AutonomousBudgetClass::ModelCall);
    let output = exhaustion(exhausted_budgets, AutonomousBudgetClass::Output);
    let resource =
        exhaustion(exhausted_budgets, AutonomousBudgetClass::Resource);
    let mutation_disposition = if matches!(
        attempt,
        AutonomousBudgetExhaustion::Exhausted
    ) || matches!(elapsed_time, AutonomousBudgetExhaustion::Exhausted)
        || matches!(model_call, AutonomousBudgetExhaustion::Exhausted)
        || matches!(output, AutonomousBudgetExhaustion::Exhausted)
        || matches!(resource, AutonomousBudgetExhaustion::Exhausted)
    {
        AutonomousBudgetMutationDisposition::StopForExhaustedBudget
    } else {
        AutonomousBudgetMutationDisposition::NoBudgetStop
    };
    AutonomousBudgetAdmission {
        attempt,
        elapsed_time,
        model_call,
        mutation_disposition,
        output,
        resource,
    }
}
