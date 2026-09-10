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
//   - Regression evidence for guided calibration categories and resumable
//     state.
// - Must-Not:
//   - Choose sample counts, prompt wording, reference geometry, capture UI,
//     confidence policy, extraction behavior, or physical units.
// - Allows:
//   - Inputs: Deterministic caller-owned calibration plans and progress
//     fixtures.
//   - Outputs: Assertions over plan validation, resume order, and completion.
//   - Side effects: Process-local test allocation only.
// - Split-When:
//   - Capture orchestration or reference geometry gains independent fixtures.
// - Merge-When:
//   - Guided calibration progress moves into another pure domain harness.
// - Summary:
//   - Proves caller-owned prompt plans resume without losing accepted progress.
// - Description:
//   - Covers categories, duplicate identities, completion, and pending order.
// - Usage:
//   - Compile directly against the handwriting-calibration-session domain.
// - Defaults:
//   - The workflow owns prompt count, wording, speed, size, and sufficiency.
//
use atrament_handwriting_calibration_session::{
    CalibrationPrompt, CalibrationPromptKind, CalibrationPromptProgress,
    CalibrationSession, CalibrationSessionError,
};

type Session =
    CalibrationSession<&'static str, u8, u16, &'static str, &'static str>;

fn prompt(
    identity: &'static str,
    kind: CalibrationPromptKind,
    progress: CalibrationPromptProgress<&'static str>,
) -> CalibrationPrompt<&'static str, u8, u16, &'static str> {
    CalibrationPrompt {
        identity,
        kind,
        progress,
        size: 12,
        speed: 3,
    }
}

#[test]
fn accepted_prompt_categories_cover_the_guided_calibration_adr() {
    let categories = [
        CalibrationPromptKind::FreeWriting,
        CalibrationPromptKind::Heading,
        CalibrationPromptKind::IsolatedCharacter,
        CalibrationPromptKind::Join,
        CalibrationPromptKind::MathematicalSymbol,
        CalibrationPromptKind::Numeral,
        CalibrationPromptKind::Punctuation,
        CalibrationPromptKind::Sentence,
        CalibrationPromptKind::Word,
    ];
    assert_eq!(categories.len(), 9);
}

#[test]
fn completed_prompts_resume_at_first_pending_prompt_without_reordering() {
    let session = Session {
        prompts: vec![
            prompt(
                "character-a",
                CalibrationPromptKind::IsolatedCharacter,
                CalibrationPromptProgress::Completed {
                    sample: "sample-character-a",
                },
            ),
            prompt(
                "join-ab",
                CalibrationPromptKind::Join,
                CalibrationPromptProgress::Pending,
            ),
            prompt(
                "sentence-a",
                CalibrationPromptKind::Sentence,
                CalibrationPromptProgress::Pending,
            ),
        ],
        reference_geometry: "caller-owned-reference-geometry",
    };

    let next = session.next_pending_prompt_index()
        .expect("unique prompt identities")
        .expect("one prompt remains pending");
    assert_eq!(next, 1);
    assert_eq!(session.prompts[next].identity, "join-ab");
    assert_eq!(session.prompts[next].speed, 3);
    assert_eq!(session.prompts[next].size, 12);
    assert_eq!(session.prompts[0].identity, "character-a");
    assert_eq!(session.reference_geometry, "caller-owned-reference-geometry");
}

#[test]
fn completed_prompt_projection_preserves_plan_order_and_sample_links() {
    let session = Session {
        prompts: vec![
            prompt(
                "character-a",
                CalibrationPromptKind::IsolatedCharacter,
                CalibrationPromptProgress::Completed {
                    sample: "sample-character-a",
                },
            ),
            prompt(
                "join-ab",
                CalibrationPromptKind::Join,
                CalibrationPromptProgress::Pending,
            ),
            prompt(
                "word-casa",
                CalibrationPromptKind::Word,
                CalibrationPromptProgress::Completed {
                    sample: "sample-word-casa",
                },
            ),
        ],
        reference_geometry: "reference",
    };

    let completed = session
        .completed_prompts()
        .expect("unique prompt identities project completed work");
    assert_eq!(completed.len(), 2);
    assert_eq!(completed[0].identity, "character-a");
    assert_eq!(
        completed[0].progress,
        CalibrationPromptProgress::Completed {
            sample: "sample-character-a",
        },
    );
    assert_eq!(completed[1].identity, "word-casa");
    assert_eq!(
        completed[1].progress,
        CalibrationPromptProgress::Completed {
            sample: "sample-word-casa",
        },
    );
    assert_eq!(completed[0].speed, 3);
    assert_eq!(completed[0].size, 12);
}

#[test]
fn duplicate_prompt_identity_rejects_before_resume_or_completion() {
    let session = Session {
        prompts: vec![
            prompt(
                "duplicate",
                CalibrationPromptKind::Word,
                CalibrationPromptProgress::Pending,
            ),
            prompt(
                "duplicate",
                CalibrationPromptKind::Sentence,
                CalibrationPromptProgress::Pending,
            ),
        ],
        reference_geometry: "reference",
    };
    assert_eq!(
        session.validate(),
        Err(CalibrationSessionError::DuplicatePromptIdentity {
            prompt: "duplicate",
        }),
    );
    assert_eq!(
        session.is_complete(),
        Err(CalibrationSessionError::DuplicatePromptIdentity {
            prompt: "duplicate",
        }),
    );
    assert_eq!(
        session.completed_prompts(),
        Err(CalibrationSessionError::DuplicatePromptIdentity {
            prompt: "duplicate",
        }),
    );
    assert_eq!(
        session.next_pending_prompt_index(),
        Err(CalibrationSessionError::DuplicatePromptIdentity {
            prompt: "duplicate",
        }),
    );
}

#[test]
fn completion_is_derived_only_from_caller_supplied_prompt_progress() {
    let pending = Session {
        prompts: vec![prompt(
            "free-writing",
            CalibrationPromptKind::FreeWriting,
            CalibrationPromptProgress::Pending,
        )],
        reference_geometry: "reference",
    };
    assert_eq!(pending.is_complete(), Ok(false));

    let complete = Session {
        prompts: vec![prompt(
            "free-writing",
            CalibrationPromptKind::FreeWriting,
            CalibrationPromptProgress::Completed {
                sample: "held-by-caller",
            },
        )],
        reference_geometry: "reference",
    };
    assert_eq!(complete.is_complete(), Ok(true));
    assert_eq!(complete.next_pending_prompt_index(), Ok(None));
}

#[test]
fn every_compact_progress_mask_matches_resume_and_inspection_oracle() {
    type CompactSession = CalibrationSession<u8, u8, u8, u8, ()>;
    let mut cases = 0_u16;
    for prompt_count in 0_u8..=8 {
        let state_count = 1_u16 << prompt_count;
        for mask in 0_u16..state_count {
            let prompts = (0..prompt_count)
                .map(|identity| CalibrationPrompt {
                    identity,
                    kind: CalibrationPromptKind::Word,
                    progress: if mask & (1_u16 << identity) == 0 {
                        CalibrationPromptProgress::Pending
                    } else {
                        CalibrationPromptProgress::Completed {
                            sample: identity.saturating_add(40),
                        }
                    },
                    size: 12,
                    speed: 3,
                })
                .collect::<Vec<_>>();
            let session = CompactSession {
                prompts,
                reference_geometry: (),
            };
            let expected_pending = (0..prompt_count).find(|identity| {
                mask & (1_u16 << identity) == 0
            });
            let expected_completed = (0..prompt_count)
                .filter(|identity| mask & (1_u16 << identity) != 0)
                .collect::<Vec<_>>();
            let completed = session
                .completed_prompts()
                .expect("compact plan identities are unique");
            assert_eq!(
                session.next_pending_prompt_index(),
                Ok(expected_pending.map(usize::from)),
                "resume mismatch for count {prompt_count} mask {mask:#x}",
            );
            assert_eq!(
                session.is_complete(),
                Ok(expected_pending.is_none()),
                "completion mismatch for count {prompt_count} mask {mask:#x}",
            );
            assert_eq!(
                completed
                    .iter()
                    .map(|prompt| prompt.identity)
                    .collect::<Vec<_>>(),
                expected_completed,
                "inspection mismatch for count {prompt_count} mask {mask:#x}",
            );
            cases = cases.saturating_add(1);
        }
    }
    assert_eq!(cases, 511);
}

#[test]
fn first_duplicate_prompt_follows_plan_order_not_identity_order() {
    let session = Session {
        prompts: vec![
            prompt(
                "z-later-key",
                CalibrationPromptKind::Word,
                CalibrationPromptProgress::Pending,
            ),
            prompt(
                "a-earlier-key",
                CalibrationPromptKind::Sentence,
                CalibrationPromptProgress::Pending,
            ),
            prompt(
                "z-later-key",
                CalibrationPromptKind::Heading,
                CalibrationPromptProgress::Pending,
            ),
            prompt(
                "a-earlier-key",
                CalibrationPromptKind::Join,
                CalibrationPromptProgress::Pending,
            ),
        ],
        reference_geometry: "reference",
    };
    assert_eq!(
        session.validate(),
        Err(CalibrationSessionError::DuplicatePromptIdentity {
            prompt: "z-later-key",
        }),
    );
}
