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
//   - Read-only offline simulation traces over validated motion-plan dry runs.
// - Must-Not:
//   - Compute geometry/transforms/limits, invent timing, render pixels, contact
//     hardware, optimize paths, or authorize physical motion.
// - Allows:
//   - Inputs: One already structured exact plan/calibration dry-run package.
//   - Outputs: Ordered borrowed operation and limit-evidence inspection trace.
//   - Side effects: Process-local trace allocation only.
// - Split-When:
//   - Simulator visualization or timing algorithms gain executable authority.
// - Merge-When:
//   - Offline simulation stops being independently inspectable from dry-run.
// - Summary:
//   - Exposes the exact validated physical plan without creating device state.
// - Description:
//   - Preserves plan/calibration identity and operation order by borrowing the
//     validated dry-run package rather than copying or rewriting plan data.
// - Usage:
//   - Build a trace only after complete dry-run validation succeeds.
// - Defaults:
//   - Invalid dry runs produce no simulation trace.
//

//! Read-only offline simulation trace over an exact validated motion plan.

use atrament_motion_plan_dry_run::{
    DryRunCalibration, DryRunLimitEvaluation, DryRunOperation as _,
    DryRunOperationKind,
    DryRunPlan, DryRunValidationError, MotionPlanDryRun,
    validate_motion_plan_dry_run,
};

/// One exact plan operation plus its inspected limit evidence.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MotionPlanSimulationStep<'trace, LimitEvidence, Operation> {
    /// Actual operation family derived from the exact plan operation.
    pub kind: DryRunOperationKind,
    /// Exact dry-run limit evidence aligned to this operation.
    pub limit_evaluation: &'trace DryRunLimitEvaluation<LimitEvidence>,
    /// Exact borrowed plan operation; geometry/payload is never rewritten.
    pub operation: &'trace Operation,
    /// Zero-based operation position in the exact plan.
    pub operation_index: usize,
}

/// Complete inspectable offline trace retaining the validated dry-run package.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MotionPlanSimulationTrace<'trace, Calibration, LimitEvidence, Plan>
where
    Plan: DryRunPlan,
{
    /// Exact validated dry-run package, including plan and calibration
    /// identity.
    pub dry_run: &'trace MotionPlanDryRun<Calibration, LimitEvidence, Plan>,
    /// Ordered operation inspection trace.
    pub steps:
        Vec<MotionPlanSimulationStep<'trace, LimitEvidence, Plan::Operation>>,
}

/// Build an offline simulation trace only from a completely valid dry run.
///
/// # Errors
///
/// Returns the same fail-closed dry-run validation error and creates no trace
/// when limit evidence is incomplete, unknown, or violated.
pub fn build_motion_plan_simulation_trace<Calibration, LimitEvidence, Plan>(
    dry_run: &MotionPlanDryRun<Calibration, LimitEvidence, Plan>,
) -> Result<
    MotionPlanSimulationTrace<'_, Calibration, LimitEvidence, Plan>,
    DryRunValidationError,
>
where
    Calibration: DryRunCalibration,
    Plan: DryRunPlan,
{
    validate_motion_plan_dry_run(dry_run)?;
    let steps = dry_run
        .plan
        .dry_run_operations()
        .iter()
        .zip(&dry_run.limit_evaluations)
        .enumerate()
        .map(|(operation_index, (operation, limit_evaluation))| {
            MotionPlanSimulationStep {
                kind: operation.dry_run_kind(),
                limit_evaluation,
                operation_index,
                operation,
            }
        })
        .collect();
    Ok(MotionPlanSimulationTrace { dry_run, steps })
}
