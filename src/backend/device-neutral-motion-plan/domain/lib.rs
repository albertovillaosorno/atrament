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
//   - Transport-neutral physical motion-plan contents and provenance.
// - Must-Not:
//   - Optimize path order, calibrate coordinates, connect to hardware, emit
//     vendor commands, choose speeds/accelerations, infer pressure, or
//     authorize
//     physical execution.
// - Allows:
//   - Inputs: Caller-owned physical geometry, dynamics, bounds, pen/profile
//     assumptions, pauses/checkpoints, provenance, and estimated duration.
//   - Outputs: One ordered inspectable device-neutral motion plan.
//   - Side effects: None.
// - Split-When:
//   - Path optimization, simulation, or hardware adaptation gains executable
//     authority.
// - Merge-When:
//   - Motion-plan structure becomes inseparable from one Plan application.
// - Summary:
//   - Keeps inspectable motion intent separate from device execution.
// - Description:
//   - Freezes physical-unit operations and provenance without vendor transport.
// - Usage:
//   - Project an accepted live-capability compilation before hardware adapters.
// - Defaults:
//   - No hardware safety authorization is implied by this value.
//

//! Device-neutral ordered motion intent independent of physical hardware.

/// Whether one physical geometry segment is traversed with the pen lifted or
/// down.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum MotionContactState {
    /// Pen is in contact with the writing surface while traversing geometry.
    PenDown,
    /// Pen remains lifted while traversing geometry.
    PenUp,
}

/// Structural family of one device-neutral plan operation.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum MotionPlanOperationKind {
    /// Inspectable plan checkpoint.
    Checkpoint,
    /// Explicit plan pause.
    Pause,
    /// Physical pen-down motion segment.
    PenDown,
    /// Physical pen-up motion segment.
    PenUp,
}

/// One physical motion segment with semantic provenance and dynamics intent.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MotionSegment<
    Acceleration,
    Geometry,
    Pressure,
    SemanticOrigin,
    Speed,
> {
    /// Caller-owned acceleration intent.
    pub acceleration: Acceleration,
    /// Pen-up or pen-down traversal state.
    pub contact_state: MotionContactState,
    /// Caller-owned physical-unit path geometry.
    pub geometry: Geometry,
    /// Optional admitted pressure intent.
    pub pressure: Option<Pressure>,
    /// Semantic object or span that originated this motion.
    pub semantic_origin: SemanticOrigin,
    /// Caller-owned speed intent.
    pub speed: Speed,
}

/// Ordered device-neutral operations emitted by Plan compilation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum MotionPlanOperation<Checkpoint, Pause, Segment> {
    /// Inspectable recovery/resume checkpoint.
    Checkpoint(Checkpoint),
    /// Explicit pause required by the plan contract.
    Pause(Pause),
    /// Pen-up or pen-down physical geometry traversal.
    Segment(Segment),
}

impl<
    Acceleration,
    Checkpoint,
    Geometry,
    Pause,
    Pressure,
    SemanticOrigin,
    Speed,
>
    MotionPlanOperation<
        Checkpoint,
        Pause,
        MotionSegment<Acceleration, Geometry, Pressure, SemanticOrigin, Speed>,
    >
{
    /// Return this operation's structural family without changing plan intent.
    #[must_use]
    pub const fn kind(&self) -> MotionPlanOperationKind {
        match self {
            Self::Checkpoint(_) => MotionPlanOperationKind::Checkpoint,
            Self::Pause(_) => MotionPlanOperationKind::Pause,
            Self::Segment(segment) => match segment.contact_state {
                MotionContactState::PenDown => MotionPlanOperationKind::PenDown,
                MotionContactState::PenUp => MotionPlanOperationKind::PenUp,
            },
        }
    }
}

/// Physical page and writable-region evidence retained by the plan.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MotionBoundsEvidence<PhysicalBounds, WritableRegionCheck> {
    /// Caller-owned physical page or plan bounds.
    pub physical_bounds: PhysicalBounds,
    /// Caller-owned evidence that motion respects the writable region.
    pub writable_region_check: WritableRegionCheck,
}

/// Pen identity and live-capability assumptions consumed by the plan.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MotionCapabilityAssumptions<CapabilityProfile, PenIdentity> {
    /// Admitted live capability profile identity or value.
    pub capability_profile: CapabilityProfile,
    /// Calibrated pen identity assumed by this device-neutral plan.
    pub pen_identity: PenIdentity,
}

/// Complete inspectable device-neutral motion plan before hardware adaptation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DeviceNeutralMotionPlan<
    BoundsEvidence,
    CapabilityAssumptions,
    EstimatedDuration,
    Operation,
    PlanIdentity,
    RevisionIdentity,
> {
    /// Physical bounds and writable-region validation evidence.
    pub bounds: BoundsEvidence,
    /// Pen and capability assumptions used during compilation.
    pub capability_assumptions: CapabilityAssumptions,
    /// Optional admitted estimated execution duration or equivalent metric.
    pub estimated_duration: Option<EstimatedDuration>,
    /// Ordered device-neutral operations.
    pub operations: Vec<Operation>,
    /// Backend-owned plan identity; never hardware authorization.
    pub plan_identity: PlanIdentity,
    /// Exact accepted revision consumed by plan compilation.
    pub revision_identity: RevisionIdentity,
}
