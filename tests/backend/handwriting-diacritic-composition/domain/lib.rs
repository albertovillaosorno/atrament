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
//   - Regression evidence for profile-evidenced compositional diacritics.
// - Must-Not:
//   - Normalize Unicode, infer accent rules, compute geometry/collision,
//     render,
//     or select fallback styles.
// - Allows:
//   - Inputs: Deterministic coverage and diacritic-presentation fixtures.
//   - Outputs: Assertions over admission, rejection, and evidence retention.
//   - Side effects: Process-local test allocation only.
// - Split-When:
//   - Rule evaluation or accent geometry gains independent fixtures.
// - Merge-When:
//   - Diacritic evidence moves into contextual stroke-planning tests.
// - Summary:
//   - Proves accent reuse cannot bypass profile compositional coverage.
// - Description:
//   - Covers admitted, exact-covered, and missing composition paths.
// - Usage:
//   - Compile directly against the diacritic-composition domain.
// - Defaults:
//   - No geometry, language form, or fallback is synthesized.
//
use atrament_handwriting_diacritic_composition::{
    DiacriticCompositionError, DiacriticCompositionIntent,
    DiacriticPresentation, admit_diacritic_composition,
};
use atrament_handwriting_glyph_coverage::HandwritingCoverageProfile;

fn profile(
) -> HandwritingCoverageProfile<&'static str, &'static str, &'static str> {
    HandwritingCoverageProfile {
        compositional_rules: vec!["latin-base-plus-acute"],
        exact_graphemes: vec!["a", "á"],
        profile_identity: "writer-profile-7",
    }
}

fn intent(
    target_grapheme: &'static str,
    matched_rule: &'static str,
) -> DiacriticCompositionIntent<
    &'static str,
    DiacriticPresentation<&'static str, &'static str, &'static str, u16>,
    &'static str,
> {
    DiacriticCompositionIntent {
        matched_rule,
        presentation: DiacriticPresentation {
            collision_evidence: "clear-of-neighbors",
            language_form: "spanish-acute",
            placement: "above-base-anchor",
            scale: 92,
        },
        target_grapheme,
    }
}

#[test]
fn declared_compositional_coverage_admits_and_retains_presentation() {
    let profile = profile();
    let admitted = admit_diacritic_composition(
        &profile,
        intent("a\u{301}", "latin-base-plus-acute"),
    )
    .expect("declared composition must admit");
    assert_eq!(*admitted.profile_rule, "latin-base-plus-acute");
    assert_eq!(admitted.intent.target_grapheme, "a\u{301}");
    assert_eq!(admitted.intent.presentation.placement, "above-base-anchor");
    assert_eq!(admitted.intent.presentation.scale, 92);
    assert_eq!(
        admitted.intent.presentation.collision_evidence,
        "clear-of-neighbors",
    );
    assert_eq!(
        admitted.intent.presentation.language_form,
        "spanish-acute",
    );
}

#[test]
fn exact_coverage_does_not_silently_switch_to_compositional_reuse() {
    assert_eq!(
        admit_diacritic_composition(
            &profile(),
            intent("á", "latin-base-plus-acute"),
        ),
        Err(DiacriticCompositionError::ExactCoverage),
    );
}

#[test]
fn missing_or_undeclared_composition_rejects_without_fallback() {
    assert_eq!(
        admit_diacritic_composition(
            &profile(),
            intent("a\u{301}", "generic-accent-reuse"),
        ),
        Err(DiacriticCompositionError::MissingCompositionalCoverage),
    );
}
