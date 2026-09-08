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
//   - Inspectable dry-run binding and fail-closed per-operation limit evidence.
// - Must-Not:
//   - Compute coordinate transforms/limits, contact hardware, render pixels,
//     optimize paths, arm devices, or authorize physical execution.
// - Allows:
//   - Inputs: One exact device-neutral plan, coordinate calibration, and
//     ordered
//     caller-owned limit evidence.
//   - Outputs: Structural dry-run validation and operation-kind inspection.
//   - Side effects: None.
// - Split-When:
//   - Limit geometry or simulator visualization gains executable authority.
// - Merge-When:
//   - Dry-run validation becomes inseparable from one physical adapter.
// - Summary:
//   - Binds an exact plan to calibration and refuses incomplete limit evidence.
// - Description:
//   - Classifies actual plan operations while treating unknown/violated limits
//     as blocking.
// - Usage:
//   - Validate a complete dry-run package before any physical arming boundary.
// - Defaults:
//   - Missing or uncertain motion-limit evidence never passes implicitly.
//

//! Inspectable plan dry-run validation without hardware or geometry authority.

use atrament_device_coordinate_calibration::DeviceCoordinateCalibration;
use atrament_device_neutral_motion_plan::{
    DeviceNeutralMotionPlan, MotionContactState, MotionPlanOperation,
    MotionSegment,
};

/// Caller-owned limit evidence for one operation in plan order.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DryRunLimitEvaluation<Evidence> {
    /// Caller-owned evidence supporting this limit classification.
    pub evidence: Evidence,
    /// Explicit limit classification.
    pub state: DryRunLimitState,
}

/// Per-operation limit state consumed by fail-closed dry-run validation.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum DryRunLimitState {
    /// No motion-limit evaluation applies to this non-motion operation.
    NotApplicable,
    /// Limit status is not known well enough to proceed.
    Unknown,
    /// At least one admitted physical boundary is violated.
    Violated,
    /// Caller-owned limit evaluation reports motion inside admitted bounds.
    WithinLimits,
}

/// Actual operation family exposed by dry-run inspection.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum DryRunOperationKind {
    /// Inspectable plan checkpoint.
    Checkpoint,
    /// Explicit plan pause.
    Pause,
    /// Physical pen-down motion segment.
    PenDown,
    /// Physical pen-up motion segment.
    PenUp,
}

/// Why one dry-run package is incomplete or physically blocked.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DryRunValidationError {
    /// Limit-evidence count does not match exact plan operation count.
    EvaluationCountMismatch {
        /// Number of evaluations provided by the dry-run caller.
        observed: usize,
        /// Number of operations in the exact plan.
        required: usize,
    },
    /// A pen-up/down motion operation has no limit evaluation.
    MotionLimitMissing {
        /// Zero-based operation index in plan order.
        operation_index: usize,
    },
    /// Limit state is unknown for this operation.
    UnknownLimitState {
        /// Zero-based operation index in plan order.
        operation_index: usize,
    },
    /// This operation carries explicit boundary-violation evidence.
    ViolatedBoundary {
        /// Zero-based operation index in plan order.
        operation_index: usize,
    },
}

/// Complete dry-run package binding an exact plan to one calibration record.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MotionPlanDryRun<Calibration, LimitEvidence, Plan> {
    /// Exact coordinate-calibration evidence used by this dry run.
    pub calibration: Calibration,
    /// Per-operation limit evidence in exact plan order.
    pub limit_evaluations: Vec<DryRunLimitEvaluation<LimitEvidence>>,
    /// Exact device-neutral plan being inspected.
    pub plan: Plan,
}

/// Calibration identity surface admitted by the dry-run validator.
pub trait DryRunCalibration {
    /// Exact calibration identity type.
    type CalibrationIdentity;
    /// Exact device identity type.
    type DeviceIdentity;

    /// Return the calibration identity retained by this record.
    fn dry_run_calibration_identity(&self) -> &Self::CalibrationIdentity;

    /// Return the device identity retained by this record.
    fn dry_run_device_identity(&self) -> &Self::DeviceIdentity;
}

impl<
    CalibrationIdentity,
    CoordinateTransform,
    DeviceIdentity,
    Evidence,
    PenMotion,
    SheetRegistration,
> DryRunCalibration
    for DeviceCoordinateCalibration<
        CalibrationIdentity,
        CoordinateTransform,
        DeviceIdentity,
        Evidence,
        PenMotion,
        SheetRegistration,
    >
{
    type CalibrationIdentity = CalibrationIdentity;
    type DeviceIdentity = DeviceIdentity;

    fn dry_run_calibration_identity(&self) -> &Self::CalibrationIdentity {
        &self.calibration_identity
    }

    fn dry_run_device_identity(&self) -> &Self::DeviceIdentity {
        &self.device_identity
    }
}

/// Minimal operation inspection required by the dry-run validator.
pub trait DryRunOperation {
    /// Return the actual plan-operation family without changing plan semantics.
    fn dry_run_kind(&self) -> DryRunOperationKind;
}

/// Minimal plan inspection required by the dry-run validator.
pub trait DryRunPlan {
    /// Exact operation type owned by this plan.
    type Operation: DryRunOperation;

    /// Return operations in exact plan order.
    fn dry_run_operations(&self) -> &[Self::Operation];
}

impl<Bounds, Capability, Duration, Operation, PlanIdentity, RevisionIdentity>
    DryRunPlan
    for DeviceNeutralMotionPlan<
        Bounds,
        Capability,
        Duration,
        Operation,
        PlanIdentity,
        RevisionIdentity,
    >
where
    Operation: DryRunOperation,
{
    type Operation = Operation;

    fn dry_run_operations(&self) -> &[Self::Operation] {
        &self.operations
    }
}

impl<
    Acceleration,
    Checkpoint,
    Geometry,
    Pause,
    Pressure,
    SemanticOrigin,
    Speed,
> DryRunOperation
    for MotionPlanOperation<
        Checkpoint,
        Pause,
        MotionSegment<Acceleration, Geometry, Pressure, SemanticOrigin, Speed>,
    >
{
    fn dry_run_kind(&self) -> DryRunOperationKind {
        match self {
            Self::Checkpoint(_) => DryRunOperationKind::Checkpoint,
            Self::Pause(_) => DryRunOperationKind::Pause,
            Self::Segment(segment) => match segment.contact_state {
                MotionContactState::PenDown => DryRunOperationKind::PenDown,
                MotionContactState::PenUp => DryRunOperationKind::PenUp,
            },
        }
    }
}

/// Validate one exact plan/calibration dry-run package fail closed.
///
/// The calibration type is intentionally the owned coordinate-calibration
/// authority, while this validator does not interpret any calibration values.
///
/// # Errors
///
/// Returns the first structural or limit-evidence failure in plan order.
pub fn validate_motion_plan_dry_run<Calibration, LimitEvidence, Plan>(
    dry_run: &MotionPlanDryRun<Calibration, LimitEvidence, Plan>,
) -> Result<(), DryRunValidationError>
where
    Calibration: DryRunCalibration,
    Plan: DryRunPlan,
{
    let operations = dry_run.plan.dry_run_operations();
    if dry_run.limit_evaluations.len() != operations.len() {
        return Err(DryRunValidationError::EvaluationCountMismatch {
            observed: dry_run.limit_evaluations.len(),
            required: operations.len(),
        });
    }
    for (operation_index, (operation, evaluation)) in operations
        .iter()
        .zip(&dry_run.limit_evaluations)
        .enumerate()
    {
        match evaluation.state {
            DryRunLimitState::NotApplicable
                if matches!(
                    operation.dry_run_kind(),
                    DryRunOperationKind::PenDown | DryRunOperationKind::PenUp
                ) =>
            {
                return Err(DryRunValidationError::MotionLimitMissing {
                    operation_index,
                });
            }
            DryRunLimitState::Unknown => {
                return Err(DryRunValidationError::UnknownLimitState {
                    operation_index,
                });
            }
            DryRunLimitState::Violated => {
                return Err(DryRunValidationError::ViolatedBoundary {
                    operation_index,
                });
            }
            DryRunLimitState::NotApplicable
            | DryRunLimitState::WithinLimits => {}
        }
    }
    Ok(())
}
