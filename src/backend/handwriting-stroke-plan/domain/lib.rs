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
//   - Transport-neutral inspectable handwriting stroke-plan structure.
// - Must-Not:
//   - Choose geometry, units, stroke candidates, joins, deformation, spacing,
//     projection, renderer behavior, or machine-motion policy.
// - Allows:
//   - Inputs: Caller-owned stroke samples, semantic origins, profile choices,
//     and contextual entry/exit conditions.
//   - Outputs: Ordered stroke plans and empty-stroke validation failures.
//   - Side effects: Process-local validation only.
// - Split-When:
//   - Contextual planning, vector projection, or machine motion gains
//     independent
//     executable authority.
// - Merge-When:
//   - Stroke-plan identity becomes inseparable from one planning application.
// - Summary:
//   - Preserves continuous-stroke authority without implementing synthesis.
// - Description:
//   - Keeps point dynamics, contact, provenance, and candidate conditions
//     typed.
// - Usage:
//   - Validate planner output before vector, PDF, preview, or motion
//     projection.
// - Defaults:
//   - No continuity, interpolation, or physical interpretation is inferred.
//

//! Transport-neutral authority for inspectable handwriting stroke plans.

/// Explicit pen contact state carried by every ordered stroke sample.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum StrokeContactState {
    /// Pen is in contact with the writing surface.
    Down,
    /// Pen is lifted from the writing surface.
    Up,
}

/// One time-ordered handwriting sample with caller-owned physical vocabulary.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StrokeSample<
    Position,
    Tangent,
    Curvature,
    WidthOrPressure,
    Velocity,
> {
    /// Explicit contact state at this sample.
    pub contact_state: StrokeContactState,
    /// Caller-owned curvature value.
    pub curvature: Curvature,
    /// Caller-owned physical or normalized position.
    pub position: Position,
    /// Caller-owned tangent or direction value.
    pub tangent: Tangent,
    /// Caller-owned velocity value.
    pub velocity: Velocity,
    /// Caller-owned width or pressure-proxy value.
    pub width_or_pressure: WidthOrPressure,
}

/// One contextual continuous stroke with provenance and profile choice.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PlannedStroke<
    SemanticOrigin,
    ProfileChoice,
    EntryCondition,
    ExitCondition,
    Sample,
> {
    /// Caller-owned contextual entry condition used by the planner.
    pub entry_condition: EntryCondition,
    /// Caller-owned contextual exit condition used by the planner.
    pub exit_condition: ExitCondition,
    /// Profile choice that contributed this stroke.
    pub profile_choice: ProfileChoice,
    /// Time-ordered samples. A declared stroke must contain at least one
    /// sample.
    pub samples: Vec<Sample>,
    /// Semantic span or object from which this stroke originates.
    pub semantic_origin: SemanticOrigin,
}

/// Complete inspectable handwriting stroke authority in planner order.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StrokePlan<Stroke> {
    /// Strokes in accepted planner order.
    pub strokes: Vec<Stroke>,
}

/// Why an inspectable stroke plan is structurally invalid.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StrokePlanError {
    /// One declared stroke contains no time-ordered samples.
    EmptyStroke {
        /// Zero-based stroke index in planner order.
        stroke_index: usize,
    },
}

/// Validate structural stroke-plan invariants before any projection.
///
/// Empty plans are valid for accepted content that produces no handwriting.
/// Declared strokes must contain at least one ordered sample; sample geometry
/// and
/// dynamics remain owned by the planner and later model authorities.
///
/// # Errors
///
/// Returns the first empty declared stroke in planner order.
pub fn validate_stroke_plan<
    SemanticOrigin,
    ProfileChoice,
    EntryCondition,
    ExitCondition,
    Sample,
>(
    plan: &StrokePlan<
        PlannedStroke<
            SemanticOrigin,
            ProfileChoice,
            EntryCondition,
            ExitCondition,
            Sample,
        >,
    >,
) -> Result<(), StrokePlanError> {
    for (stroke_index, stroke) in plan.strokes.iter().enumerate() {
        if stroke.samples.is_empty() {
            return Err(StrokePlanError::EmptyStroke { stroke_index });
        }
    }
    Ok(())
}
