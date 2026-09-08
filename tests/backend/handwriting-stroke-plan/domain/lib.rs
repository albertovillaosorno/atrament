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
//   - Regression evidence for continuous handwriting stroke-plan structure.
// - Must-Not:
//   - Choose geometry, units, planning, joins, interpolation, rendering, PDF,
//     or machine-motion behavior.
// - Allows:
//   - Inputs: Deterministic caller-owned stroke/sample fixtures.
//   - Outputs: Assertions over ordering, contact, provenance, and validation.
//   - Side effects: Process-local test allocation only.
// - Split-When:
//   - Contextual planning or projection gains independent executable fixtures.
// - Merge-When:
//   - Stroke-plan structural validation moves into another pure domain harness.
// - Summary:
//   - Proves inspectable stroke authority remains ordered and provenance-rich.
// - Description:
//   - Covers dynamics fields, contact states, profile choice, and empty
//     strokes.
// - Usage:
//   - Compile directly against the handwriting-stroke-plan domain.
// - Defaults:
//   - No continuity or physical interpretation is inferred.
//
use atrament_handwriting_stroke_plan::{
    PlannedStroke, StrokeContactState, StrokePlan, StrokePlanError,
    StrokeSample,
    validate_stroke_plan,
};

type Sample = StrokeSample<&'static str, &'static str, i32, u16, i32>;
type Stroke = PlannedStroke<
    &'static str,
    &'static str,
    &'static str,
    &'static str,
    Sample,
>;

fn sample(position: &'static str, contact_state: StrokeContactState) -> Sample {
    StrokeSample {
        contact_state,
        curvature: 3,
        position,
        tangent: "east",
        velocity: 12,
        width_or_pressure: 7,
    }
}

#[test]
fn ordered_samples_retain_all_continuous_stroke_inputs() {
    let samples = [
        sample("p0", StrokeContactState::Down),
        sample("p1", StrokeContactState::Down),
        sample("p2", StrokeContactState::Up),
    ];
    assert_eq!(samples[0].position, "p0");
    assert_eq!(samples[1].position, "p1");
    assert_eq!(samples[2].position, "p2");
    assert_eq!(samples[0].tangent, "east");
    assert_eq!(samples[0].curvature, 3);
    assert_eq!(samples[0].width_or_pressure, 7);
    assert_eq!(samples[0].velocity, 12);
    assert_eq!(samples[2].contact_state, StrokeContactState::Up);
}

#[test]
fn every_declared_stroke_retains_semantic_origin_profile_choice_and_conditions()
{
    let stroke = Stroke {
        entry_condition: "entry-from-left",
        exit_condition: "exit-to-right",
        profile_choice: "contextual-form-a",
        samples: vec![sample("p0", StrokeContactState::Down)],
        semantic_origin: "semantic-span-42",
    };
    assert_eq!(stroke.semantic_origin, "semantic-span-42");
    assert_eq!(stroke.profile_choice, "contextual-form-a");
    assert_eq!(stroke.entry_condition, "entry-from-left");
    assert_eq!(stroke.exit_condition, "exit-to-right");
}

#[test]
fn stroke_order_is_preserved_without_projection_or_reclassification() {
    let plan = StrokePlan {
        strokes: vec![
            Stroke {
                entry_condition: "entry-a",
                exit_condition: "exit-a",
                profile_choice: "choice-a",
                samples: vec![sample("a", StrokeContactState::Down)],
                semantic_origin: "span-a",
            },
            Stroke {
                entry_condition: "entry-b",
                exit_condition: "exit-b",
                profile_choice: "choice-b",
                samples: vec![sample("b", StrokeContactState::Up)],
                semantic_origin: "span-b",
            },
        ],
    };
    assert_eq!(validate_stroke_plan(&plan), Ok(()));
    assert_eq!(plan.strokes[0].semantic_origin, "span-a");
    assert_eq!(plan.strokes[1].semantic_origin, "span-b");
}

#[test]
fn empty_declared_stroke_rejects_at_exact_planner_index() {
    let plan = StrokePlan {
        strokes: vec![
            Stroke {
                entry_condition: "entry-a",
                exit_condition: "exit-a",
                profile_choice: "choice-a",
                samples: vec![sample("a", StrokeContactState::Down)],
                semantic_origin: "span-a",
            },
            Stroke {
                entry_condition: "entry-b",
                exit_condition: "exit-b",
                profile_choice: "choice-b",
                samples: Vec::new(),
                semantic_origin: "span-b",
            },
        ],
    };
    assert_eq!(
        validate_stroke_plan(&plan),
        Err(StrokePlanError::EmptyStroke { stroke_index: 1 }),
    );
}

#[test]
fn empty_plan_is_valid_for_content_with_no_handwriting_projection() {
    let plan: StrokePlan<Stroke> = StrokePlan { strokes: Vec::new() };
    assert_eq!(validate_stroke_plan(&plan), Ok(()));
}
