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

//! Handwriting calibration session domain regression tests.

#![expect(
    clippy::expect_used,
    reason = "fail-fast assertions over bounded test fixtures"
)]

use std::ptr;

use atrament_handwriting_calibration_session::{
    CalibrationPrompt, CalibrationPromptKind, CalibrationPromptProgress,
    CalibrationSampleReplacement, CalibrationSampleReplacementError,
    CalibrationSession, CalibrationSessionError,
};

type Session =
    CalibrationSession<&'static str, u8, u16, &'static str, &'static str>;

const fn prompt(
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

    let next = session
        .next_pending_prompt_index()
        .expect("unique prompt identities")
        .expect("one prompt remains pending");
    assert_eq!(next, 1);
    assert_eq!(
        session
            .prompts
            .get(next)
            .expect("bounded prompt index")
            .identity,
        "join-ab"
    );
    assert_eq!(
        session
            .prompts
            .get(next)
            .expect("bounded prompt index")
            .speed,
        3
    );
    assert_eq!(
        session
            .prompts
            .get(next)
            .expect("bounded prompt index")
            .size,
        12
    );
    assert_eq!(
        session
            .prompts
            .first()
            .expect("bounded prompt index")
            .identity,
        "character-a"
    );
    assert_eq!(
        session.reference_geometry,
        "caller-owned-reference-geometry"
    );
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
    assert_eq!(
        completed
            .first()
            .expect("bounded completed prompt index")
            .identity,
        "character-a"
    );
    assert_eq!(
        completed
            .first()
            .expect("bounded completed prompt index")
            .progress,
        CalibrationPromptProgress::Completed {
            sample: "sample-character-a",
        },
    );
    assert_eq!(
        completed
            .get(1)
            .expect("bounded completed prompt index")
            .identity,
        "word-casa"
    );
    assert_eq!(
        completed
            .get(1)
            .expect("bounded completed prompt index")
            .progress,
        CalibrationPromptProgress::Completed {
            sample: "sample-word-casa",
        },
    );
    assert_eq!(
        completed
            .first()
            .expect("bounded completed prompt index")
            .speed,
        3
    );
    assert_eq!(
        completed
            .first()
            .expect("bounded completed prompt index")
            .size,
        12
    );
}

#[test]
fn exact_completed_sample_replacement_changes_only_the_target_link() {
    let mut session = Session {
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
    let before = session.clone();

    assert_eq!(
        session.replace_completed_sample(
            &"character-a",
            &"sample-character-a",
            "sample-character-a-2",
        ),
        Ok(CalibrationSampleReplacement::Applied),
    );
    assert_eq!(
        session
            .prompts
            .first()
            .expect("bounded prompt index")
            .identity,
        before
            .prompts
            .first()
            .expect("bounded prompt index")
            .identity
    );
    assert_eq!(
        session.prompts.first().expect("bounded prompt index").kind,
        before.prompts.first().expect("bounded prompt index").kind
    );
    assert_eq!(
        session.prompts.first().expect("bounded prompt index").speed,
        before.prompts.first().expect("bounded prompt index").speed
    );
    assert_eq!(
        session.prompts.first().expect("bounded prompt index").size,
        before.prompts.first().expect("bounded prompt index").size
    );
    assert_eq!(session.prompts.get(1..), before.prompts.get(1..));
    assert_eq!(session.reference_geometry, before.reference_geometry);
    assert_eq!(
        session
            .prompts
            .first()
            .expect("bounded prompt index")
            .progress,
        CalibrationPromptProgress::Completed {
            sample: "sample-character-a-2",
        },
    );
}

#[test]
fn completed_sample_replacement_is_exact_preconditioned_and_fail_closed() {
    let base = Session {
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
        ],
        reference_geometry: "reference",
    };

    let mut no_op = base.clone();
    assert_eq!(
        no_op.replace_completed_sample(
            &"character-a",
            &"sample-character-a",
            "sample-character-a",
        ),
        Ok(CalibrationSampleReplacement::NoOp),
    );
    assert_eq!(no_op, base);

    let cases = [
        (
            "stale sample",
            "character-a",
            "sample-older",
            Err(CalibrationSampleReplacementError::SampleMismatch {
                actual: "sample-character-a",
                expected: "sample-older",
                prompt: "character-a",
            }),
        ),
        (
            "pending prompt",
            "join-ab",
            "sample-join-ab",
            Err(CalibrationSampleReplacementError::PromptPending {
                prompt: "join-ab",
            }),
        ),
        (
            "unknown prompt",
            "missing",
            "sample-missing",
            Err(CalibrationSampleReplacementError::UnknownPrompt {
                prompt: "missing",
            }),
        ),
    ];
    for (case, prompt, expected, outcome) in cases {
        let mut session = base.clone();
        assert_eq!(
            session
                .replace_completed_sample(&prompt, &expected, "replacement",),
            outcome,
            "{case}",
        );
        assert_eq!(session, base, "{case} must not mutate session");
    }
}

#[test]
#[expect(
    clippy::too_many_lines,
    reason = "single exhaustive 3076-case sample replacement oracle"
)]
fn every_compact_replacement_case_matches_exact_precondition_oracle() {
    type CompactSession = CalibrationSession<u8, u8, u8, u8, ()>;
    type ReplacementError = CalibrationSampleReplacementError<u8, u8>;
    let mut cases = 0usize;
    let mut saw = [false; 5];
    for prompt_count in 0u8..=6 {
        for mask in 0u16..(1u16 << prompt_count) {
            for target in 0u8..=prompt_count {
                for stale_expected in [false, true] {
                    for different_replacement in [false, true] {
                        let prompts = (0..prompt_count)
                            .map(|identity| CalibrationPrompt {
                                identity,
                                kind: CalibrationPromptKind::Word,
                                progress: if mask & (1u16 << identity) == 0 {
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
                        let mut session = CompactSession {
                            prompts,
                            reference_geometry: (),
                        };
                        let mut expected_session = session.clone();
                        let current_sample = target.saturating_add(40);
                        let expected_sample = if stale_expected {
                            target.saturating_add(180)
                        } else {
                            current_sample
                        };
                        let replacement_sample = if different_replacement {
                            target.saturating_add(100)
                        } else {
                            current_sample
                        };
                        let expected = if target >= prompt_count {
                            saw[0] = true;
                            Err(ReplacementError::UnknownPrompt {
                                prompt: target,
                            })
                        } else if mask & (1u16 << target) == 0 {
                            saw[1] = true;
                            Err(ReplacementError::PromptPending {
                                prompt: target,
                            })
                        } else if stale_expected {
                            saw[2] = true;
                            Err(ReplacementError::SampleMismatch {
                                actual: current_sample,
                                expected: expected_sample,
                                prompt: target,
                            })
                        } else if different_replacement {
                            saw[3] = true;
                            let target_index = usize::from(target);
                            let target_prompt = expected_session
                                .prompts
                                .get_mut(target_index)
                                .expect(
                                    "indexed prompt exists in accepted case",
                                );
                            target_prompt.progress =
                                CalibrationPromptProgress::Completed {
                                    sample: replacement_sample,
                                };
                            Ok(CalibrationSampleReplacement::Applied)
                        } else {
                            saw[4] = true;
                            Ok(CalibrationSampleReplacement::NoOp)
                        };
                        let actual = session.replace_completed_sample(
                            &target,
                            &expected_sample,
                            replacement_sample,
                        );
                        assert_eq!(
                            actual,
                            expected,
                            concat!(
                                "outcome mismatch count={} mask={:#x} ",
                                "target={} stale={} different={}",
                            ),
                            prompt_count,
                            mask,
                            target,
                            stale_expected,
                            different_replacement,
                        );
                        assert_eq!(
                            session,
                            expected_session,
                            concat!(
                                "state mismatch count={} mask={:#x} ",
                                "target={} stale={} different={}",
                            ),
                            prompt_count,
                            mask,
                            target,
                            stale_expected,
                            different_replacement,
                        );
                        cases += 1;
                    }
                }
            }
        }
    }
    assert_eq!(cases, 3_076);
    assert!(saw.into_iter().all(|seen| seen));
}

#[test]
fn duplicate_prompt_identity_rejects_before_sample_replacement() {
    let mut session = Session {
        prompts: vec![
            prompt(
                "duplicate",
                CalibrationPromptKind::Word,
                CalibrationPromptProgress::Completed { sample: "sample-a" },
            ),
            prompt(
                "duplicate",
                CalibrationPromptKind::Sentence,
                CalibrationPromptProgress::Completed { sample: "sample-b" },
            ),
        ],
        reference_geometry: "reference",
    };
    let before = session.clone();
    assert_eq!(
        session.replace_completed_sample(
            &"duplicate",
            &"sample-a",
            "replacement",
        ),
        Err(CalibrationSampleReplacementError::Session {
            reason: CalibrationSessionError::DuplicatePromptIdentity {
                prompt: "duplicate",
            },
        }),
    );
    assert_eq!(session, before);
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
    let expected = CalibrationSessionError::DuplicatePromptIdentity {
        prompt: "duplicate",
    };
    assert_eq!(session.validate(), Err(expected.clone()));
    assert_eq!(session.validate_view(), Err(expected));
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
    let mut cases = 0u16;
    for prompt_count in 0u8..=8 {
        let state_count = 1u16 << prompt_count;
        for mask in 0u16..state_count {
            let prompts = (0..prompt_count)
                .map(|identity| CalibrationPrompt {
                    identity,
                    kind: CalibrationPromptKind::Word,
                    progress: if mask & (1u16 << identity) == 0 {
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
            let expected_pending = (0..prompt_count)
                .find(|identity| mask & (1u16 << identity) == 0);
            let expected_completed = (0..prompt_count)
                .filter(|identity| mask & (1u16 << identity) != 0)
                .collect::<Vec<_>>();
            let validated = session
                .validate_view()
                .expect("compact plan identities are unique");
            assert!(ptr::eq(validated.session(), &raw const session));
            let completed = session
                .completed_prompts()
                .expect("compact plan identities are unique");
            assert_eq!(
                session.next_pending_prompt_index(),
                Ok(expected_pending.map(usize::from)),
                "resume mismatch for count {prompt_count} mask {mask:#x}",
            );
            assert_eq!(
                validated.next_pending_prompt_index(),
                expected_pending.map(usize::from),
                "sealed resume: count {prompt_count}, mask {mask:#x}",
            );
            assert_eq!(
                session.is_complete(),
                Ok(expected_pending.is_none()),
                "completion mismatch for count {prompt_count} mask {mask:#x}",
            );
            assert_eq!(
                validated.is_complete(),
                expected_pending.is_none(),
                "sealed completion: count {prompt_count}, mask {mask:#x}",
            );
            let projected = completed
                .iter()
                .map(|prompt| prompt.identity)
                .collect::<Vec<_>>();
            assert_eq!(
                projected, expected_completed,
                "inspection mismatch for count {prompt_count} mask {mask:#x}",
            );
            assert_eq!(
                validated
                    .completed_prompts()
                    .iter()
                    .map(|prompt| prompt.identity)
                    .collect::<Vec<_>>(),
                expected_completed,
                "sealed inspection: count {prompt_count}, mask {mask:#x}",
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
