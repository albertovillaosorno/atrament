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
//   - Regression evidence for exact Spanish diacritic-rule applicability and
//     its profile-coverage integration.
// - Must-Not:
//   - Normalize Unicode, infer geometry, render accents, or select fallback.
// - Allows:
//   - Inputs: Frozen bilingual graphemes and deterministic near misses.
//   - Outputs: Assertions over exact acute, tilde, and diaeresis matching.
//   - Side effects: Process-local test allocation only.
// - Split-When:
//   - Another language or non-diacritic rule gains fixtures.
// - Merge-When:
//   - Rule evidence moves into a complete language acceptance harness.
// - Summary:
//   - Proves only admitted decomposed Spanish forms receive rule evidence.
// - Description:
//   - Covers all 14 decomposed requirements, rejects neighboring spellings,
//     and feeds exact language matches into generic profile coverage.
// - Usage:
//   - Compile against the bilingual rule and grapheme-inventory domains.
// - Defaults:
//   - Precomposed spellings remain exact-coverage candidates, not rule matches.
//
use atrament_english_spanish_diacritic_rule::{
    EnglishSpanishDiacriticRule, applicable_english_spanish_diacritic_rule,
};
use atrament_english_spanish_grapheme_inventory::{
    EnglishSpanishGraphemeCategory, REQUIRED_TEXT_GRAPHEMES,
};
use atrament_handwriting_glyph_coverage::{
    HandwritingCoverageProfile, HandwritingCoverageQuery,
    missing_handwriting_coverage,
};

#[test]
fn all_fourteen_decomposed_inventory_entries_have_exact_rule_evidence() {
    let mut acute = 0_usize;
    let mut diaeresis = 0_usize;
    let mut tilde = 0_usize;
    let mut total = 0_usize;
    for required in REQUIRED_TEXT_GRAPHEMES.iter().filter(|required| {
        required.category
            == EnglishSpanishGraphemeCategory::SpanishDecomposedEquivalent
    }) {
        match applicable_english_spanish_diacritic_rule(required.grapheme) {
            Some(EnglishSpanishDiacriticRule::Acute) => acute += 1,
            Some(EnglishSpanishDiacriticRule::Diaeresis) => diaeresis += 1,
            Some(EnglishSpanishDiacriticRule::Tilde) => tilde += 1,
            None => panic!("missing rule for {:?}", required.grapheme),
        }
        total += 1;
    }
    assert_eq!(acute, 10);
    assert_eq!(diaeresis, 2);
    assert_eq!(tilde, 2);
    assert_eq!(total, 14);
}

#[test]
fn every_non_decomposed_text_requirement_has_no_diacritic_rule() {
    for required in REQUIRED_TEXT_GRAPHEMES.iter().filter(|required| {
        required.category
            != EnglishSpanishGraphemeCategory::SpanishDecomposedEquivalent
    }) {
        assert_eq!(
            applicable_english_spanish_diacritic_rule(required.grapheme),
            None,
            "unexpected rule for {:?}",
            required.grapheme,
        );
    }
}

#[test]
fn near_miss_spellings_do_not_gain_language_rule_evidence() {
    for grapheme in [
        "Y\u{301}", "y\u{301}", "N\u{301}", "n\u{301}",
        "A\u{308}", "a\u{308}", "\u{301}", "\u{303}",
        "\u{308}", "á", "ñ", "ü",
    ] {
        assert_eq!(applicable_english_spanish_diacritic_rule(grapheme), None);
    }
}

#[test]
fn language_rule_evidence_completes_required_profile_coverage() {
    let required = REQUIRED_TEXT_GRAPHEMES
        .iter()
        .map(|entry| entry.grapheme)
        .collect::<Vec<_>>();
    let exact = REQUIRED_TEXT_GRAPHEMES
        .iter()
        .filter(|entry| {
            entry.category
                != EnglishSpanishGraphemeCategory::SpanishDecomposedEquivalent
        })
        .map(|entry| entry.grapheme)
        .collect::<Vec<_>>();
    let matched_rules = required
        .iter()
        .map(|grapheme| applicable_english_spanish_diacritic_rule(grapheme))
        .collect::<Vec<_>>();
    let queries = required
        .iter()
        .zip(matched_rules.iter())
        .map(|(grapheme, matched_rule)| HandwritingCoverageQuery {
            grapheme,
            matched_compositional_rule: matched_rule.as_ref(),
        })
        .collect::<Vec<_>>();
    let profile = HandwritingCoverageProfile {
        compositional_rules: vec![
            EnglishSpanishDiacriticRule::Acute,
            EnglishSpanishDiacriticRule::Diaeresis,
            EnglishSpanishDiacriticRule::Tilde,
        ],
        exact_graphemes: exact,
        profile_identity: "bilingual-compositional-profile",
    };
    assert_eq!(required.len(), 114);
    assert_eq!(profile.exact_graphemes.len(), 100);
    assert_eq!(matched_rules.iter().flatten().count(), 14);
    assert!(missing_handwriting_coverage(&profile, &queries).is_empty());
}
