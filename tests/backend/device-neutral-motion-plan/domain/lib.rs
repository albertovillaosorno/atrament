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
//   - Regression evidence for device-neutral motion-plan content retention.
// - Must-Not:
//   - Optimize paths, calibrate coordinates, call hardware, emit vendor
//     commands,
//     choose dynamics, or interpret plan identity as arming.
// - Allows:
//   - Inputs: Deterministic caller-owned motion fixtures.
//   - Outputs: Assertions over operation order, dynamics, provenance, and
//     bounds.
//   - Side effects: Process-local test allocation only.
// - Split-When:
//   - Optimizer, simulator, or device-adapter behavior gains fixtures.
// - Merge-When:
//   - Motion evidence moves into an application-level Plan harness.
// - Summary:
//   - Proves inspectable live motion remains device-neutral and attributable.
// - Description:
//   - Covers pen-up/down geometry, dynamics, pressure, pauses, checkpoints,
//     capability assumptions, bounds, plan/revision identity, and duration.
// - Usage:
//   - Compile directly against the device-neutral-motion-plan domain.
// - Defaults:
//   - No plan value authorizes physical execution.
//
use atrament_device_neutral_motion_plan::{
    DeviceNeutralMotionPlan, MotionBoundsEvidence, MotionCapabilityAssumptions,
    MotionContactState, MotionPlanOperation, MotionPlanOperationKind,
    MotionSegment,
};

type Segment = MotionSegment<u16, &'static str, u16, &'static str, u16>;
type Operation = MotionPlanOperation<&'static str, &'static str, Segment>;

#[test]
fn ordered_motion_retains_contact_geometry_dynamics_and_semantic_origin() {
    let operations: [Operation; 2] = [
        MotionPlanOperation::Segment(MotionSegment {
            acceleration: 40_u16,
            contact_state: MotionContactState::PenUp,
            geometry: "move-to-title",
            pressure: None,
            semantic_origin: "title-17",
            speed: 120_u16,
        }),
        MotionPlanOperation::Segment(MotionSegment {
            acceleration: 25_u16,
            contact_state: MotionContactState::PenDown,
            geometry: "title-stroke-1",
            pressure: Some(640_u16),
            semantic_origin: "title-17",
            speed: 70_u16,
        }),
    ];
    let MotionPlanOperation::Segment(first) = &operations[0] else {
        panic!("first operation must stay a segment");
    };
    let MotionPlanOperation::Segment(second) = &operations[1] else {
        panic!("second operation must stay a segment");
    };
    assert_eq!(first.contact_state, MotionContactState::PenUp);
    assert_eq!(second.contact_state, MotionContactState::PenDown);
    assert_eq!(second.geometry, "title-stroke-1");
    assert_eq!(second.pressure, Some(640));
    assert_eq!(second.semantic_origin, "title-17");
}

#[test]
fn operation_kind_is_owned_by_the_device_neutral_plan() {
    let operations: [Operation; 4] = [
        MotionPlanOperation::Checkpoint("checkpoint"),
        MotionPlanOperation::Pause("pause"),
        MotionPlanOperation::Segment(MotionSegment {
            acceleration: 20,
            contact_state: MotionContactState::PenUp,
            geometry: "travel",
            pressure: None,
            semantic_origin: "paragraph-2",
            speed: 80,
        }),
        MotionPlanOperation::Segment(MotionSegment {
            acceleration: 20,
            contact_state: MotionContactState::PenDown,
            geometry: "stroke",
            pressure: Some(500),
            semantic_origin: "paragraph-2",
            speed: 60,
        }),
    ];
    assert_eq!(operations[0].kind(), MotionPlanOperationKind::Checkpoint);
    assert_eq!(operations[1].kind(), MotionPlanOperationKind::Pause);
    assert_eq!(operations[2].kind(), MotionPlanOperationKind::PenUp);
    assert_eq!(operations[3].kind(), MotionPlanOperationKind::PenDown);
}

#[test]
fn pauses_and_checkpoints_remain_explicit_in_operation_order() {
    let operations: [Operation; 3] = [
        MotionPlanOperation::Checkpoint("checkpoint-before-page-2"),
        MotionPlanOperation::Pause("drying-pause"),
        MotionPlanOperation::Segment(MotionSegment {
            acceleration: 20,
            contact_state: MotionContactState::PenDown,
            geometry: "line-2",
            pressure: None,
            semantic_origin: "paragraph-2",
            speed: 60,
        }),
    ];
    assert!(matches!(operations[0], MotionPlanOperation::Checkpoint(_)));
    assert!(matches!(operations[1], MotionPlanOperation::Pause(_)));
    assert!(matches!(operations[2], MotionPlanOperation::Segment(_)));
}

#[test]
fn plan_retains_revision_capability_pen_bounds_identity_and_duration() {
    let plan = DeviceNeutralMotionPlan {
        bounds: MotionBoundsEvidence {
            physical_bounds: "a4-physical-bounds",
            writable_region_check: "all-motion-inside-writable-region",
        },
        capability_assumptions: MotionCapabilityAssumptions {
            capability_profile: "single-pen-live-v1",
            pen_identity: "calibrated-pen-blue",
        },
        estimated_duration: Some(93_u32),
        operations: Vec::<Operation>::new(),
        plan_identity: "plan-identity-8",
        revision_identity: "revision-22",
    };
    assert_eq!(plan.revision_identity, "revision-22");
    assert_eq!(plan.plan_identity, "plan-identity-8");
    assert_eq!(plan.estimated_duration, Some(93));
    assert_eq!(
        plan.capability_assumptions.capability_profile,
        "single-pen-live-v1",
    );
    assert_eq!(plan.capability_assumptions.pen_identity, "calibrated-pen-blue");
    assert_eq!(plan.bounds.physical_bounds, "a4-physical-bounds");
}
