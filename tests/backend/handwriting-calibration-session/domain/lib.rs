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
    CalibrationSession, CalibrationSessionError, calibration_session_complete,
    next_pending_calibration_prompt_index, validate_calibration_session,
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

    let next = next_pending_calibration_prompt_index(&session)
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
        validate_calibration_session(&session),
        Err(CalibrationSessionError::DuplicatePromptIdentity {
            prompt: "duplicate",
        }),
    );
    assert_eq!(
        calibration_session_complete(&session),
        Err(CalibrationSessionError::DuplicatePromptIdentity {
            prompt: "duplicate",
        }),
    );
    assert_eq!(
        next_pending_calibration_prompt_index(&session),
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
    assert_eq!(calibration_session_complete(&pending), Ok(false));

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
    assert_eq!(calibration_session_complete(&complete), Ok(true));
    assert_eq!(next_pending_calibration_prompt_index(&complete), Ok(None));
}
