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
//   - Regression evidence for contextual stroke-candidate planner inputs.
// - Must-Not:
//   - Score/select candidates, synthesize joins, deform/space strokes, render,
//     or emit machine paths.
// - Allows:
//   - Inputs: Deterministic candidate, provenance, and context fixtures.
//   - Outputs: Assertions over exact candidate/context retention.
//   - Side effects: Process-local test allocation only.
// - Split-When:
//   - Candidate scoring or planning policy gains independent fixtures.
// - Merge-When:
//   - Candidate-input evidence moves into a planner harness.
// - Summary:
//   - Proves planner inputs remain explicit and inspectable before selection.
// - Description:
//   - Covers candidate provenance, conditions, order, and planning context.
// - Usage:
//   - Compile directly against the contextual-stroke-candidates domain.
// - Defaults:
//   - No candidate is preferred implicitly.
//
use atrament_handwriting_contextual_stroke_candidates::{
    ContextualStrokeCandidate, ContextualStrokePlanningInput,
    StrokePlanningContext,
};

type Candidate = ContextualStrokeCandidate<
    u16,
    &'static str,
    &'static str,
    &'static str,
    &'static str,
    &'static str,
    &'static str,
>;

type Context = StrokePlanningContext<
    &'static str,
    (&'static str, &'static str),
    &'static str,
    &'static str,
    &'static str,
>;

fn candidate(candidate_identity: u16, payload: &'static str) -> Candidate {
    ContextualStrokeCandidate {
        candidate_identity,
        character_intent: "n",
        entry_condition: "entry-left-low",
        exit_condition: "exit-right-mid",
        profile_choice: "writer-profile-7/body",
        semantic_origin: "span-42",
        stroke_payload: payload,
    }
}

fn context() -> Context {
    StrokePlanningContext {
        line_geometry: "baseline-3",
        neighboring_graphemes: ("a", "d"),
        semantic_role: "body",
        word_position: "medial",
        writing_style: "calibrated-style-9",
    }
}

#[test]
fn candidate_retains_profile_semantic_and_entry_exit_evidence() {
    let value = candidate(7, "stroke-vocabulary-11");
    assert_eq!(value.candidate_identity, 7);
    assert_eq!(value.character_intent, "n");
    assert_eq!(value.entry_condition, "entry-left-low");
    assert_eq!(value.exit_condition, "exit-right-mid");
    assert_eq!(value.profile_choice, "writer-profile-7/body");
    assert_eq!(value.semantic_origin, "span-42");
    assert_eq!(value.stroke_payload, "stroke-vocabulary-11");
}

#[test]
fn planning_context_retains_every_frozen_context_family() {
    let context = context();
    assert_eq!(context.neighboring_graphemes, ("a", "d"));
    assert_eq!(context.word_position, "medial");
    assert_eq!(context.line_geometry, "baseline-3");
    assert_eq!(context.semantic_role, "body");
    assert_eq!(context.writing_style, "calibrated-style-9");
}

#[test]
fn planning_input_preserves_caller_candidate_order_without_selection() {
    let input = ContextualStrokePlanningInput {
        candidates: vec![candidate(2, "variant-b"), candidate(1, "variant-a")],
        context: context(),
    };
    assert_eq!(input.candidates[0].candidate_identity, 2);
    assert_eq!(input.candidates[1].candidate_identity, 1);
    assert_eq!(input.candidates[0].stroke_payload, "variant-b");
}

#[test]
fn empty_candidate_collection_has_no_implicit_fallback_candidate() {
    let input = ContextualStrokePlanningInput::<Candidate, Context> {
        candidates: vec![],
        context: context(),
    };
    assert!(input.candidates.is_empty());
}
