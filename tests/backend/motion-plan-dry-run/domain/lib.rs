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
//   - Regression evidence for exact-plan dry-run binding and limit refusal.
// - Must-Not:
//   - Compute limits/transforms, render pixels, contact hardware, optimize
//     paths,
//     arm devices, or authorize physical execution.
// - Allows:
//   - Inputs: Deterministic plan, calibration, and limit-evidence fixtures.
//   - Outputs: Assertions over operation classification and fail-closed checks.
//   - Side effects: Process-local test allocation only.
// - Split-When:
//   - Limit geometry or simulator visualization gains independent fixtures.
// - Merge-When:
//   - Dry-run evidence moves into a physical-adapter acceptance harness.
// - Summary:
//   - Proves real motion-plan operations cannot bypass dry-run limit evidence.
// - Description:
//   - Covers plan binding, calibration, operation kinds, and blocked limits.
// - Usage:
//   - Compile against dry-run, motion-plan, and calibration domains.
// - Defaults:
//   - Unknown, violated, or absent motion-limit evidence always rejects.
//
use atrament_device_coordinate_calibration::{
    CoordinateTransformCalibration, DeviceCoordinateCalibration,
    PenMotionCalibration, SheetRegistrationCalibration,
};
use atrament_device_neutral_motion_plan::{
    DeviceNeutralMotionPlan, MotionBoundsEvidence, MotionCapabilityAssumptions,
    MotionContactState, MotionPlanOperation, MotionSegment,
};
use atrament_motion_plan_dry_run::{
    DryRunLimitEvaluation, DryRunLimitState, DryRunOperation,
    DryRunOperationKind, DryRunValidationError, MotionPlanDryRun,
    validate_motion_plan_dry_run,
};

type Segment = MotionSegment<u16, &'static str, u16, &'static str, u16>;
type Operation = MotionPlanOperation<&'static str, &'static str, Segment>;
type Plan = DeviceNeutralMotionPlan<
    MotionBoundsEvidence<&'static str, &'static str>,
    MotionCapabilityAssumptions<&'static str, &'static str>,
    u32,
    Operation,
    &'static str,
    &'static str,
>;
type Calibration = DeviceCoordinateCalibration<
    &'static str,
    CoordinateTransformCalibration<
        &'static str,
        &'static str,
        &'static str,
        &'static str,
    >,
    &'static str,
    &'static [&'static str],
    PenMotionCalibration<
        &'static str,
        &'static str,
        &'static str,
        &'static str,
    >,
    SheetRegistrationCalibration<&'static str, &'static str, &'static str>,
>;

fn calibration() -> Calibration {
    DeviceCoordinateCalibration {
        calibration_identity: "calibration-4",
        coordinate_transform: CoordinateTransformCalibration {
            axis_orientation: "axes",
            origin: "origin",
            scale: "scale",
            skew: "skew",
        },
        device_identity: "device-7",
        evidence: &["registration", "limit-probe"],
        pen_motion: PenMotionCalibration {
            acceleration: "acceleration",
            contact_height: "contact-height",
            pen_up_height: "pen-up-height",
            speed: "speed",
        },
        sheet_registration: SheetRegistrationCalibration {
            boundary_clearance: "clearance",
            page_clamping: "clamping",
            usable_area: "usable-area",
        },
    }
}

fn segment(contact_state: MotionContactState, name: &'static str) -> Operation {
    MotionPlanOperation::Segment(MotionSegment {
        acceleration: 30,
        contact_state,
        geometry: name,
        pressure: None,
        semantic_origin: "span-9",
        speed: 70,
    })
}

fn plan() -> Plan {
    DeviceNeutralMotionPlan {
        bounds: MotionBoundsEvidence {
            physical_bounds: "page-bounds",
            writable_region_check: "writable-clean",
        },
        capability_assumptions: MotionCapabilityAssumptions {
            capability_profile: "single-pen-live",
            pen_identity: "pen-black",
        },
        estimated_duration: Some(4_200),
        operations: vec![
            segment(MotionContactState::PenUp, "travel"),
            MotionPlanOperation::Pause("drying-pause"),
            MotionPlanOperation::Checkpoint("checkpoint-1"),
            segment(MotionContactState::PenDown, "stroke"),
        ],
        plan_identity: "plan-22",
        revision_identity: "revision-14",
    }
}

fn clean_evaluations() -> Vec<DryRunLimitEvaluation<&'static str>> {
    vec![
        DryRunLimitEvaluation {
            evidence: "travel-within-limits",
            state: DryRunLimitState::WithinLimits,
        },
        DryRunLimitEvaluation {
            evidence: "pause-no-motion",
            state: DryRunLimitState::NotApplicable,
        },
        DryRunLimitEvaluation {
            evidence: "checkpoint-no-motion",
            state: DryRunLimitState::NotApplicable,
        },
        DryRunLimitEvaluation {
            evidence: "stroke-within-limits",
            state: DryRunLimitState::WithinLimits,
        },
    ]
}

#[test]
fn actual_motion_plan_operations_expose_dry_run_kinds() {
    let plan = plan();
    assert_eq!(plan.operations[0].dry_run_kind(), DryRunOperationKind::PenUp);
    assert_eq!(plan.operations[1].dry_run_kind(), DryRunOperationKind::Pause);
    assert_eq!(
        plan.operations[2].dry_run_kind(),
        DryRunOperationKind::Checkpoint,
    );
    assert_eq!(
        plan.operations[3].dry_run_kind(),
        DryRunOperationKind::PenDown,
    );
}

#[test]
fn exact_plan_and_calibration_with_complete_limits_passes_dry_run() {
    let dry_run = MotionPlanDryRun {
        calibration: calibration(),
        limit_evaluations: clean_evaluations(),
        plan: plan(),
    };
    assert_eq!(validate_motion_plan_dry_run(&dry_run), Ok(()));
    assert_eq!(dry_run.plan.plan_identity, "plan-22");
    assert_eq!(dry_run.calibration.calibration_identity, "calibration-4");
    assert_eq!(dry_run.calibration.device_identity, "device-7");
}

#[test]
fn operation_count_mismatch_rejects_before_limit_interpretation() {
    let mut evaluations = clean_evaluations();
    evaluations.pop();
    let dry_run = MotionPlanDryRun {
        calibration: calibration(),
        limit_evaluations: evaluations,
        plan: plan(),
    };
    assert_eq!(
        validate_motion_plan_dry_run(&dry_run),
        Err(DryRunValidationError::EvaluationCountMismatch {
            observed: 3,
            required: 4,
        }),
    );
}

#[test]
fn motion_segment_cannot_claim_limit_checks_are_not_applicable() {
    let mut evaluations = clean_evaluations();
    evaluations[0].state = DryRunLimitState::NotApplicable;
    let dry_run = MotionPlanDryRun {
        calibration: calibration(),
        limit_evaluations: evaluations,
        plan: plan(),
    };
    assert_eq!(
        validate_motion_plan_dry_run(&dry_run),
        Err(DryRunValidationError::MotionLimitMissing { operation_index: 0 }),
    );
}

#[test]
fn unknown_or_violated_limit_state_blocks_exact_operation() {
    let mut unknown = clean_evaluations();
    unknown[3].state = DryRunLimitState::Unknown;
    let unknown_run = MotionPlanDryRun {
        calibration: calibration(),
        limit_evaluations: unknown,
        plan: plan(),
    };
    assert_eq!(
        validate_motion_plan_dry_run(&unknown_run),
        Err(DryRunValidationError::UnknownLimitState { operation_index: 3 }),
    );

    let mut violated = clean_evaluations();
    violated[0].state = DryRunLimitState::Violated;
    let violated_run = MotionPlanDryRun {
        calibration: calibration(),
        limit_evaluations: violated,
        plan: plan(),
    };
    assert_eq!(
        validate_motion_plan_dry_run(&violated_run),
        Err(DryRunValidationError::ViolatedBoundary { operation_index: 0 }),
    );
}
