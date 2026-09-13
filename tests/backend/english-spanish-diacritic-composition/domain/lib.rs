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
//   - Regression evidence joining the bilingual diacritic matcher to generic
//     profile-evidenced composition admission.
// - Must-Not:
//   - Normalize Unicode, choose geometry, evaluate collisions, render, or
//     select fallback.
// - Allows:
//   - Inputs: Frozen bilingual graphemes and explicit profile rule
//     declarations.
//   - Outputs: Exact admitted rule/profile/presentation or typed rejection.
//   - Side effects: Process-local test allocation only.
// - Split-When:
//   - Another language gains equivalent integration fixtures.
// - Merge-When:
//   - One complete handwriting-language acceptance suite supersedes this seam.
// - Summary:
//   - Proves all 14 decomposed bilingual forms use language-owned rule
//     evidence.
// - Description:
//   - Also proves precomposed and near-miss spellings never gain
//     decomposition.
// - Usage:
//   - Compile against the bilingual matcher, composition adapter, and
//     inventory.
// - Defaults:
//   - Exact profile coverage still precedes compositional reuse.
//
use atrament_english_spanish_diacritic_composition::{
    EnglishSpanishDiacriticCompositionError,
    admit_english_spanish_diacritic_composition,
};
use atrament_english_spanish_diacritic_rule::{
    EnglishSpanishDiacriticRule, applicable_english_spanish_diacritic_rule,
};
use atrament_english_spanish_grapheme_inventory::{
    EnglishSpanishGraphemeCategory, REQUIRED_TEXT_GRAPHEMES,
};
use atrament_handwriting_diacritic_composition::{
    DiacriticCompositionError, DiacriticPresentation,
};
use atrament_handwriting_glyph_coverage::HandwritingCoverageProfile;

type Presentation = DiacriticPresentation<&'static str, &'static str, u16, u16>;
type Profile = HandwritingCoverageProfile<
    &'static str,
    String,
    EnglishSpanishDiacriticRule,
>;

fn presentation() -> Presentation {
    Presentation {
        collision_evidence: "clear",
        language_form: "spanish",
        placement: 7,
        scale: 9,
    }
}

fn profile(
    exact_graphemes: Vec<String>,
    compositional_rules: Vec<EnglishSpanishDiacriticRule>,
) -> Profile {
    HandwritingCoverageProfile {
        compositional_rules,
        exact_graphemes,
        profile_identity: "writer-a",
    }
}

#[test]
fn every_decomposed_bilingual_requirement_uses_its_language_rule() {
    let profile = profile(
        Vec::new(),
        vec![
            EnglishSpanishDiacriticRule::Acute,
            EnglishSpanishDiacriticRule::Diaeresis,
            EnglishSpanishDiacriticRule::Tilde,
        ],
    );
    let mut cases = 0_usize;
    for required in REQUIRED_TEXT_GRAPHEMES.iter().filter(|required| {
        required.category
            == EnglishSpanishGraphemeCategory::SpanishDecomposedEquivalent
    }) {
        let admitted = admit_english_spanish_diacritic_composition(
            &profile,
            String::from(required.grapheme),
            presentation(),
        )
        .expect("frozen decomposed bilingual grapheme must admit composition");
        assert_eq!(admitted.intent().target_grapheme, required.grapheme);
        assert_eq!(admitted.intent().presentation, presentation());
        assert_eq!(admitted.profile_identity(), &"writer-a");
        assert_eq!(admitted.profile_rule(), &admitted.intent().matched_rule);
        cases = cases.saturating_add(1);
    }
    assert_eq!(cases, 14);
}

#[test]
fn every_non_decomposed_bilingual_requirement_rejects_without_inference() {
    let profile = profile(
        Vec::new(),
        vec![
            EnglishSpanishDiacriticRule::Acute,
            EnglishSpanishDiacriticRule::Diaeresis,
            EnglishSpanishDiacriticRule::Tilde,
        ],
    );
    let mut cases = 0_usize;
    for required in REQUIRED_TEXT_GRAPHEMES.iter().filter(|required| {
        required.category
            != EnglishSpanishGraphemeCategory::SpanishDecomposedEquivalent
    }) {
        assert_eq!(
            admit_english_spanish_diacritic_composition(
                &profile,
                String::from(required.grapheme),
                presentation(),
            ),
            Err(
                EnglishSpanishDiacriticCompositionError::
                    NoApplicableLanguageRule,
            ),
            "non-decomposed requirement {:?}",
            required.grapheme,
        );
        cases = cases.saturating_add(1);
    }
    assert_eq!(cases, 100);
}

#[test]
fn every_decomposed_requirement_matches_profile_admission_matrix() {
    let mut cases = 0_usize;
    let mut acute = false;
    let mut diaeresis = false;
    let mut tilde = false;
    for required in REQUIRED_TEXT_GRAPHEMES.iter().filter(|required| {
        required.category
            == EnglishSpanishGraphemeCategory::SpanishDecomposedEquivalent
    }) {
        let rule = applicable_english_spanish_diacritic_rule(required.grapheme)
            .expect("decomposed bilingual requirement must own a rule");
        match rule {
            EnglishSpanishDiacriticRule::Acute => acute = true,
            EnglishSpanishDiacriticRule::Diaeresis => diaeresis = true,
            EnglishSpanishDiacriticRule::Tilde => tilde = true,
        }
        for exact_covered in [false, true] {
            for rule_declared in [false, true] {
                let target = String::from(required.grapheme);
                let profile = profile(
                    exact_covered.then(|| target.clone()).into_iter().collect(),
                    rule_declared.then_some(rule).into_iter().collect(),
                );
                let outcome = admit_english_spanish_diacritic_composition(
                    &profile,
                    target,
                    presentation(),
                );
                if exact_covered {
                    assert_eq!(
                        outcome,
                        Err(
                            EnglishSpanishDiacriticCompositionError::
                                ProfileAdmission(
                                    DiacriticCompositionError::ExactCoverage,
                                ),
                        ),
                        "exact coverage must win for {:?}",
                        required.grapheme,
                    );
                } else if rule_declared {
                    let admitted = outcome.expect(
                        "declared bilingual composition must be admitted",
                    );
                    assert_eq!(
                        admitted.intent().target_grapheme,
                        required.grapheme,
                    );
                    assert_eq!(*admitted.profile_rule(), rule);
                    assert_eq!(admitted.intent().matched_rule, rule);
                } else {
                    assert_eq!(
                        outcome,
                        Err(
                            EnglishSpanishDiacriticCompositionError::
                                ProfileAdmission(
                                    DiacriticCompositionError::
                                        MissingCompositionalCoverage,
                                ),
                        ),
                        "missing declaration must reject {:?}",
                        required.grapheme,
                    );
                }
                cases = cases.saturating_add(1);
            }
        }
    }
    assert_eq!(cases, 56);
    assert!(acute && diaeresis && tilde);
}

#[test]
fn deterministic_near_misses_never_gain_language_rule_or_normalization() {
    let profile = profile(
        Vec::new(),
        vec![
            EnglishSpanishDiacriticRule::Acute,
            EnglishSpanishDiacriticRule::Diaeresis,
            EnglishSpanishDiacriticRule::Tilde,
        ],
    );
    for target in [
        "á",
        "n\u{301}",
        "e\u{308}",
        "u\u{303}",
        "n",
        "\u{301}",
        "a\u{301}\u{301}",
    ] {
        assert_eq!(
            admit_english_spanish_diacritic_composition(
                &profile,
                String::from(target),
                presentation(),
            ),
            Err(
                EnglishSpanishDiacriticCompositionError::
                    NoApplicableLanguageRule,
            ),
            "near miss {target:?}",
        );
    }
}
