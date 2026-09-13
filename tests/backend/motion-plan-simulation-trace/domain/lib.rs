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
//   - Regression evidence for validated offline motion-plan simulation traces.
// - Must-Not:
//   - Compute geometry/transforms/limits, invent timing, render pixels, contact
//     hardware, optimize paths, or authorize physical execution.
// - Allows:
//   - Inputs: Deterministic exact plan/calibration dry-run fixtures.
//   - Outputs: Assertions over ordered borrowed simulation inspection.
//   - Side effects: Process-local test allocation only.
// - Split-When:
//   - Visualization or timing algorithms gain independent fixtures.
// - Merge-When:
//   - Simulation trace evidence moves into the dry-run regression harness.
// - Summary:
//   - Proves offline simulation cannot bypass validated dry-run authority.
// - Description:
//   - Covers exact operations, limits, identities, duration, and invalid
//     refusal.
// - Usage:
//   - Compile against simulation-trace, dry-run, plan, and calibration domains.
// - Defaults:
//   - Invalid dry runs cannot produce an inspectable trace.
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
    DryRunLimitEvaluation, DryRunLimitState, DryRunOperationKind,
    DryRunValidationError, MotionPlanDryRun,
};
use atrament_motion_plan_simulation_trace::build_motion_plan_simulation_trace;

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

type DryRun = MotionPlanDryRun<Calibration, &'static str, Plan>;

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

fn segment(
    contact_state: MotionContactState,
    geometry: &'static str,
) -> Operation {
    MotionPlanOperation::Segment(MotionSegment {
        acceleration: 30,
        contact_state,
        geometry,
        pressure: None,
        semantic_origin: "span-9",
        speed: 70,
    })
}

fn dry_run() -> DryRun {
    MotionPlanDryRun {
        calibration: calibration(),
        limit_evaluations: vec![
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
        ],
        plan: DeviceNeutralMotionPlan {
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
        },
    }
}

#[test]
fn simulation_trace_preserves_exact_operation_order_and_kind() {
    let dry_run = dry_run();
    let trace = build_motion_plan_simulation_trace(&dry_run)
        .expect("clean dry run");
    assert_eq!(trace.steps().len(), 4);
    assert_eq!(trace.steps()[0].operation_index, 0);
    assert_eq!(trace.steps()[0].kind, DryRunOperationKind::PenUp);
    assert_eq!(trace.steps()[1].kind, DryRunOperationKind::Pause);
    assert_eq!(trace.steps()[2].kind, DryRunOperationKind::Checkpoint);
    assert_eq!(trace.steps()[3].kind, DryRunOperationKind::PenDown);
}

#[test]
fn simulation_trace_borrows_exact_geometry_pause_checkpoint_and_limits() {
    let dry_run = dry_run();
    let trace = build_motion_plan_simulation_trace(&dry_run)
        .expect("clean dry run");
    let MotionPlanOperation::Segment(first) = trace.steps()[0].operation else {
        panic!("first operation must remain pen-up segment");
    };
    assert_eq!(first.geometry, "travel");
    assert_eq!(
        trace.steps()[0].limit_evaluation.evidence,
        "travel-within-limits",
    );
    assert_eq!(
        trace.steps()[1].operation,
        &MotionPlanOperation::Pause("drying-pause"),
    );
    assert_eq!(
        trace.steps()[2].operation,
        &MotionPlanOperation::Checkpoint("checkpoint-1"),
    );
}

#[test]
fn trace_retains_exact_plan_calibration_duration_bounds_and_identities() {
    let dry_run = dry_run();
    let trace = build_motion_plan_simulation_trace(&dry_run)
        .expect("clean dry run");
    assert_eq!(trace.dry_run().plan.plan_identity, "plan-22");
    assert_eq!(trace.dry_run().plan.revision_identity, "revision-14");
    assert_eq!(trace.dry_run().plan.estimated_duration, Some(4_200));
    assert_eq!(trace.dry_run().plan.bounds.physical_bounds, "page-bounds");
    assert_eq!(
        trace.dry_run().calibration.calibration_identity,
        "calibration-4",
    );
    assert_eq!(trace.dry_run().calibration.device_identity, "device-7");
}

#[test]
fn invalid_dry_run_produces_no_simulation_trace() {
    let mut dry_run = dry_run();
    dry_run.limit_evaluations[3].state = DryRunLimitState::Unknown;
    assert_eq!(
        build_motion_plan_simulation_trace(&dry_run),
        Err(DryRunValidationError::UnknownLimitState { operation_index: 3 }),
    );
}

#[test]
fn simulation_trace_matches_all_dry_run_limit_states() {
    let states = [
        DryRunLimitState::NotApplicable,
        DryRunLimitState::Unknown,
        DryRunLimitState::Violated,
        DryRunLimitState::WithinLimits,
    ];
    let mut cases = 0_u8;
    for operation_index in 0..4 {
        let is_motion = matches!(operation_index, 0 | 3);
        for state in states {
            let mut dry_run = dry_run();
            dry_run.limit_evaluations[operation_index].state = state;
            let expected_error = match state {
                DryRunLimitState::NotApplicable if is_motion => {
                    Some(DryRunValidationError::MotionLimitMissing {
                        operation_index,
                    })
                },
                DryRunLimitState::Unknown => {
                    Some(DryRunValidationError::UnknownLimitState {
                        operation_index,
                    })
                },
                DryRunLimitState::Violated => {
                    Some(DryRunValidationError::ViolatedBoundary {
                        operation_index,
                    })
                },
                DryRunLimitState::NotApplicable
                | DryRunLimitState::WithinLimits => None,
            };
            match expected_error {
                Some(error) => assert_eq!(
                    build_motion_plan_simulation_trace(&dry_run),
                    Err(error),
                    "trace rejection mismatch at operation {operation_index}",
                ),
                None => {
                    let trace = build_motion_plan_simulation_trace(&dry_run)
                        .expect("admitted dry run must produce a trace");
                    assert!(std::ptr::eq(trace.dry_run(), &dry_run));
                    assert_eq!(trace.steps().len(), 4);
                    assert_eq!(
                        trace.steps()[operation_index].limit_evaluation.state,
                        state,
                    );
                },
            }
            cases = cases.saturating_add(1);
        }
    }
    assert_eq!(cases, 16);
}
