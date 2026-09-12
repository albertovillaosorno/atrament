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
    ContextualStrokeCandidate, ContextualStrokeCandidateIdentityError,
    ContextualStrokePlanningInput, StrokePlanningContext,
    validate_contextual_stroke_candidate_identities,
    validate_contextual_stroke_planning_input,
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
#[test]
fn complete_planning_input_explicitly_validates_candidate_addressing() {
    let valid = ContextualStrokePlanningInput {
        candidates: vec![candidate(2, "variant-b"), candidate(1, "variant-a")],
        context: context(),
    };
    assert_eq!(validate_contextual_stroke_planning_input(&valid), Ok(()));
    assert_eq!(valid.context, context());
    assert_eq!(valid.candidates[0].candidate_identity, 2);

    let duplicate = ContextualStrokePlanningInput {
        candidates: vec![
            candidate(7, "variant-a"),
            candidate(3, "variant-b"),
            candidate(7, "variant-c"),
        ],
        context: context(),
    };
    assert_eq!(
        validate_contextual_stroke_planning_input(&duplicate),
        Err(ContextualStrokeCandidateIdentityError {
            duplicate_index: 2,
            first_index: 0,
        }),
    );
    assert_eq!(duplicate.context, context());
}

#[test]
fn distinct_candidate_identities_are_addressable_without_ranking() {
    let candidates = [candidate(2, "variant-b"), candidate(1, "variant-a")];
    assert_eq!(
        validate_contextual_stroke_candidate_identities(&candidates),
        Ok(()),
    );
}

#[test]
fn duplicate_candidate_identity_reports_earliest_prior_owner() {
    let candidates = [
        candidate(7, "variant-a"),
        candidate(3, "variant-b"),
        candidate(7, "variant-c"),
    ];
    assert_eq!(
        validate_contextual_stroke_candidate_identities(&candidates),
        Err(ContextualStrokeCandidateIdentityError {
            duplicate_index: 2,
            first_index: 0,
        }),
    );
}

#[test]
fn all_31_two_identity_sequences_match_first_duplicate_oracle() {
    let mut cases = 0_u8;
    let mut saw_unique = false;
    let mut saw_duplicate = false;
    for length in 0_u32..=4 {
        for encoded in 0_u32..2_u32.pow(length) {
            let mut state = encoded;
            let mut identities = Vec::new();
            let mut candidates = Vec::new();
            for index in 0..length {
                let identity = (state % 2) as u16;
                state /= 2;
                identities.push(identity);
                candidates.push(candidate(
                    identity,
                    if index % 2 == 0 { "variant-a" } else { "variant-b" },
                ));
            }
            let mut expected = Ok(());
            'outer: for duplicate_index in 0..identities.len() {
                for first_index in 0..duplicate_index {
                    if identities[first_index] == identities[duplicate_index] {
                        expected = Err(ContextualStrokeCandidateIdentityError {
                            duplicate_index,
                            first_index,
                        });
                        break 'outer;
                    }
                }
            }
            if expected.is_ok() {
                saw_unique = true;
            } else {
                saw_duplicate = true;
            }
            assert_eq!(
                validate_contextual_stroke_candidate_identities(&candidates),
                expected,
                "direct length {length}, encoded {encoded}",
            );
            let input = ContextualStrokePlanningInput {
                candidates,
                context: context(),
            };
            assert_eq!(
                validate_contextual_stroke_planning_input(&input),
                expected,
                "input length {length}, encoded {encoded}",
            );
            cases = cases.saturating_add(1);
        }
    }
    assert_eq!(cases, 31);
    assert!(saw_unique);
    assert!(saw_duplicate);
}
