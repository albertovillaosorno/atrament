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
//   - Regression evidence for digital loose-paper-note composition intent.
// - Must-Not:
//   - Render, choose style values, calculate contrast, choose thresholds, or
//     infer a live conversion.
// - Allows:
//   - Inputs: Deterministic caller-owned note appearance and contrast fixtures.
//   - Outputs: Assertions over retained effects and readability admission.
//   - Side effects: Process-local test allocation only.
// - Split-When:
//   - Contrast measurement, note layout, or rendering gains fixtures.
// - Merge-When:
//   - Paper-note evidence moves into a renderer composition harness.
// - Summary:
//   - Proves digital note effects remain explicit and contrast-gated.
// - Description:
//   - Covers fill, fold, stack order, shadow, and caller-computed readability.
// - Usage:
//   - Compile directly against the digital-paper-note-plan domain.
// - Defaults:
//   - No style value, threshold, or live fallback is synthesized.
//

//! Digital paper note plan domain regression tests.

use std::ptr;

use atrament_digital_paper_note_plan::{
    validate_digital_paper_note_plan, validate_digital_paper_note_plan_view,
    DigitalPaperNoteContrast, DigitalPaperNotePlan, DigitalPaperNotePlanError,
};

#[test]
fn digital_note_retains_fill_fold_stack_and_soft_shadow_intent() {
    let plan = DigitalPaperNotePlan {
        fill: "warm-paper-fill",
        fold: ("top-right", 14u8),
        readable_contrast: DigitalPaperNoteContrast::Readable,
        shadow: ("soft-shadow", 9u8),
        stack_order: 3i16,
    };
    assert_eq!(plan.fill, "warm-paper-fill");
    assert_eq!(plan.fold, ("top-right", 14));
    assert_eq!(plan.shadow, ("soft-shadow", 9));
    assert_eq!(plan.stack_order, 3);
    assert_eq!(validate_digital_paper_note_plan(&plan), Ok(()));
}

#[test]
fn missing_readable_contrast_evidence_rejects_without_style_interpretation() {
    let plan = DigitalPaperNotePlan {
        fill: 101u32,
        fold: 202u32,
        readable_contrast: DigitalPaperNoteContrast::NotEstablished,
        shadow: 303u32,
        stack_order: 404u32,
    };
    assert_eq!(
        validate_digital_paper_note_plan(&plan),
        Err(DigitalPaperNotePlanError::ReadableContrastNotEstablished),
    );
    assert_eq!(
        (plan.fill, plan.fold, plan.shadow, plan.stack_order),
        (101, 202, 303, 404),
    );
}

#[test]
fn readability_result_is_independent_of_caller_owned_effect_values() {
    for value in 0u8..=31 {
        let readable = DigitalPaperNotePlan {
            fill: value,
            fold: value,
            readable_contrast: DigitalPaperNoteContrast::Readable,
            shadow: value,
            stack_order: value,
        };
        let not_established = DigitalPaperNotePlan {
            readable_contrast: DigitalPaperNoteContrast::NotEstablished,
            ..readable.clone()
        };
        assert_eq!(validate_digital_paper_note_plan(&readable), Ok(()));
        let validated = validate_digital_paper_note_plan_view(&readable);
        assert!(validated.is_ok(), "readable note must seal");
        if let Ok(sealed) = validated {
            assert!(ptr::eq(sealed.plan(), ptr::from_ref(&readable)));
        }
        let reason = DigitalPaperNotePlanError::ReadableContrastNotEstablished;
        assert_eq!(
            validate_digital_paper_note_plan(&not_established),
            Err(reason),
        );
        assert_eq!(
            validate_digital_paper_note_plan_view(&not_established),
            Err(reason),
        );
    }
}
