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
    StrokeSample, semantic_origin_stroke_indices, validate_stroke_plan,
    validate_stroke_plan_view,
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
    let validated = validate_stroke_plan_view(&plan)
        .expect("nonempty strokes seal exact plan");
    assert!(std::ptr::eq(validated.plan(), &plan));
    assert_eq!(validated.strokes(), plan.strokes.as_slice());
    assert_eq!(plan.strokes[0].semantic_origin, "span-a");
    assert_eq!(plan.strokes[1].semantic_origin, "span-b");
}

#[test]
fn semantic_origin_projection_returns_only_dependent_strokes_in_plan_order() {
    let plan = StrokePlan {
        strokes: vec![
            Stroke {
                entry_condition: "entry-a0",
                exit_condition: "exit-a0",
                profile_choice: "choice-a",
                samples: vec![sample("a0", StrokeContactState::Down)],
                semantic_origin: "span-a",
            },
            Stroke {
                entry_condition: "entry-b",
                exit_condition: "exit-b",
                profile_choice: "choice-b",
                samples: vec![sample("b", StrokeContactState::Up)],
                semantic_origin: "span-b",
            },
            Stroke {
                entry_condition: "entry-a1",
                exit_condition: "exit-a1",
                profile_choice: "choice-a",
                samples: vec![sample("a1", StrokeContactState::Down)],
                semantic_origin: "span-a",
            },
        ],
    };

    assert_eq!(
        semantic_origin_stroke_indices(&plan, &"span-a"),
        Ok(vec![0, 2]),
    );
    assert_eq!(semantic_origin_stroke_indices(&plan, &"span-b"), Ok(vec![1]));
    assert_eq!(semantic_origin_stroke_indices(&plan, &"span-c"), Ok(vec![]));
    let validated = validate_stroke_plan_view(&plan)
        .expect("valid plan seals provenance projection");
    assert_eq!(validated.semantic_origin_stroke_indices(&"span-a"), [0, 2]);
    assert_eq!(validated.semantic_origin_stroke_indices(&"span-b"), [1]);
    assert!(validated.semantic_origin_stroke_indices(&"span-c").is_empty());
}

#[test]
fn semantic_origin_projection_rejects_invalid_plan_before_partial_results() {
    let plan = StrokePlan {
        strokes: vec![
            Stroke {
                entry_condition: "entry-invalid",
                exit_condition: "exit-invalid",
                profile_choice: "choice-a",
                samples: Vec::new(),
                semantic_origin: "span-other",
            },
            Stroke {
                entry_condition: "entry-target",
                exit_condition: "exit-target",
                profile_choice: "choice-b",
                samples: vec![sample("target", StrokeContactState::Down)],
                semantic_origin: "span-target",
            },
        ],
    };

    let expected = StrokePlanError::EmptyStroke { stroke_index: 0 };
    assert_eq!(
        semantic_origin_stroke_indices(&plan, &"span-target"),
        Err(expected),
    );
    assert_eq!(validate_stroke_plan_view(&plan), Err(expected));
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
fn first_empty_stroke_wins_when_multiple_strokes_are_empty() {
    let plan = StrokePlan {
        strokes: vec![
            Stroke {
                entry_condition: "entry-a",
                exit_condition: "exit-a",
                profile_choice: "choice-a",
                samples: Vec::new(),
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
        Err(StrokePlanError::EmptyStroke { stroke_index: 0 }),
    );
}

#[test]
fn all_6561_eight_stroke_states_match_structure_and_origin_oracle() {
    const STROKES: usize = 8;
    let mut valid_cases = 0usize;
    let mut invalid_cases = 0usize;
    for encoded in 0usize..3usize.pow(STROKES as u32) {
        let mut state = encoded;
        let mut expected_a = Vec::new();
        let mut expected_b = Vec::new();
        let mut expected_error = None;
        let mut strokes = Vec::with_capacity(STROKES);
        for stroke_index in 0..STROKES {
            let class = state % 3;
            state /= 3;
            let (semantic_origin, samples) = match class {
                0 => {
                    if expected_error.is_none() {
                        expected_error = Some(StrokePlanError::EmptyStroke {
                            stroke_index,
                        });
                    }
                    ("span-empty", Vec::new())
                },
                1 => {
                    expected_a.push(stroke_index);
                    ("span-a", vec![sample("a", StrokeContactState::Down)])
                },
                _ => {
                    expected_b.push(stroke_index);
                    ("span-b", vec![sample("b", StrokeContactState::Up)])
                },
            };
            strokes.push(Stroke {
                entry_condition: "entry",
                exit_condition: "exit",
                profile_choice: "choice",
                samples,
                semantic_origin,
            });
        }
        let plan = StrokePlan { strokes };
        match expected_error {
            Some(reason) => {
                invalid_cases = invalid_cases.saturating_add(1);
                assert_eq!(
                    validate_stroke_plan(&plan),
                    Err(reason),
                    "validation state {encoded}",
                );
                assert_eq!(
                    validate_stroke_plan_view(&plan),
                    Err(reason),
                    "sealed state {encoded}",
                );
                assert_eq!(
                    semantic_origin_stroke_indices(&plan, &"span-a"),
                    Err(reason),
                    "projection state {encoded}",
                );
            },
            None => {
                valid_cases = valid_cases.saturating_add(1);
                let validated = validate_stroke_plan_view(&plan)
                    .expect("all declared strokes contain samples");
                assert!(std::ptr::eq(validated.plan(), &plan));
                assert_eq!(
                    validated.semantic_origin_stroke_indices(&"span-a"),
                    expected_a,
                    "sealed A state {encoded}",
                );
                assert_eq!(
                    validated.semantic_origin_stroke_indices(&"span-b"),
                    expected_b,
                    "sealed B state {encoded}",
                );
                assert_eq!(
                    semantic_origin_stroke_indices(&plan, &"span-a"),
                    Ok(expected_a),
                    "public A state {encoded}",
                );
                assert_eq!(
                    semantic_origin_stroke_indices(&plan, &"span-b"),
                    Ok(expected_b),
                    "public B state {encoded}",
                );
            },
        }
    }
    assert_eq!(valid_cases, 256);
    assert_eq!(invalid_cases, 6_305);
}

#[test]
fn empty_plan_is_valid_for_content_with_no_handwriting_projection() {
    let plan: StrokePlan<Stroke> = StrokePlan { strokes: Vec::new() };
    assert_eq!(validate_stroke_plan(&plan), Ok(()));
    let validated = validate_stroke_plan_view(&plan)
        .expect("empty no-handwriting plan is structurally valid");
    assert!(validated.strokes().is_empty());
}
