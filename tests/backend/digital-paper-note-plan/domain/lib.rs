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
use atrament_digital_paper_note_plan::{
    DigitalPaperNoteContrast, DigitalPaperNotePlan, DigitalPaperNotePlanError,
    validate_digital_paper_note_plan,
};

#[test]
fn digital_note_retains_fill_fold_stack_and_soft_shadow_intent() {
    let plan = DigitalPaperNotePlan {
        fill: "warm-paper-fill",
        fold: ("top-right", 14_u8),
        readable_contrast: DigitalPaperNoteContrast::Readable,
        shadow: ("soft-shadow", 9_u8),
        stack_order: 3_i16,
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
        fill: 101_u32,
        fold: 202_u32,
        readable_contrast: DigitalPaperNoteContrast::NotEstablished,
        shadow: 303_u32,
        stack_order: 404_u32,
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
    for value in 0_u8..=31 {
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
        assert_eq!(
            validate_digital_paper_note_plan(&not_established),
            Err(DigitalPaperNotePlanError::ReadableContrastNotEstablished),
        );
    }
}
