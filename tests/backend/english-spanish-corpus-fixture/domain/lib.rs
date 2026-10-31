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
//   - Regression evidence for checked-in English/Spanish corpus contents.
// - Must-Not:
//   - Normalize text, infer glyph support, measure, wrap, render, or exercise
//     CLI/MCP transport.
// - Allows:
//   - Inputs: Frozen corpus fixtures and visible-text grapheme inventory.
//   - Outputs: Assertions over scenario content and exact inventory coverage.
//   - Side effects: Process-local test allocation only.
// - Split-When:
//   - Cross-surface corpus behavior gains dedicated acceptance fixtures.
// - Merge-When:
//   - One complete language acceptance harness supersedes corpus checks.
// - Summary:
//   - Proves concrete bilingual fixture text stays complete and exact.
// - Description:
//   - Pins all eleven scenario texts and all 114 visible text requirements.
// - Usage:
//   - Compile against the corpus-fixture and grapheme-inventory domains.
// - Defaults:
//   - NFC and NFD spellings remain distinct authored corpus content.
//

//! English spanish corpus fixture domain regression tests.

use atrament_english_spanish_corpus_evidence::EnglishSpanishCorpusScenario;
use atrament_english_spanish_corpus_fixture::{
    REQUIRED_SCENARIO_FIXTURES, REQUIRED_TEXT_INVENTORY_SWEEP,
    validate_checked_in_english_spanish_corpus,
};
use atrament_english_spanish_grapheme_inventory::REQUIRED_TEXT_GRAPHEMES;

#[test]
fn checked_in_corpus_matches_both_frozen_language_authorities() {
    assert_eq!(validate_checked_in_english_spanish_corpus(), Ok(()));
}

#[test]
fn scenario_fixture_order_and_identities_remain_explicit() {
    let expected = [
        (EnglishSpanishCorpusScenario::EnglishProse, "english-prose"),
        (EnglishSpanishCorpusScenario::SpanishProse, "spanish-prose"),
        (EnglishSpanishCorpusScenario::Names, "names"),
        (EnglishSpanishCorpusScenario::Quotations, "quotations"),
        (EnglishSpanishCorpusScenario::Questions, "questions"),
        (EnglishSpanishCorpusScenario::Exclamations, "exclamations"),
        (EnglishSpanishCorpusScenario::EnDash, "en-dash"),
        (EnglishSpanishCorpusScenario::EmDash, "em-dash"),
        (
            EnglishSpanishCorpusScenario::CombiningMarks,
            "combining-marks",
        ),
        (
            EnglishSpanishCorpusScenario::NormalizedEquivalents,
            "normalized-equivalents",
        ),
        (
            EnglishSpanishCorpusScenario::MixedMathematics,
            "mixed-mathematics",
        ),
    ];
    let fixtures = REQUIRED_SCENARIO_FIXTURES.iter().zip(expected);
    for (fixture, (scenario, artifact_identity)) in fixtures {
        assert_eq!(fixture.scenario, scenario);
        assert_eq!(fixture.artifact_identity, artifact_identity);
        assert!(!fixture.text.is_empty());
    }
}

#[test]
fn exact_inventory_sweep_matches_every_required_grapheme_in_order() {
    let actual = REQUIRED_TEXT_INVENTORY_SWEEP
        .split_ascii_whitespace()
        .collect::<Vec<_>>();
    let expected = REQUIRED_TEXT_GRAPHEMES
        .iter()
        .map(|required| required.grapheme)
        .collect::<Vec<_>>();
    assert_eq!(actual, expected);
    assert_eq!(actual.len(), 114);
}

#[test]
fn normalized_equivalent_fixture_preserves_both_exact_spellings() {
    let scenario = EnglishSpanishCorpusScenario::NormalizedEquivalents;
    let matching = REQUIRED_SCENARIO_FIXTURES
        .iter()
        .find(|fixture| fixture.scenario == scenario);
    assert!(
        matching.is_some(),
        "normalized-equivalent fixture must remain present",
    );
    let Some(fixture) = matching else {
        return;
    };
    for (precomposed, decomposed) in [
        ("\u{c1}", "A\u{301}"),
        ("\u{c9}", "E\u{301}"),
        ("\u{cd}", "I\u{301}"),
        ("\u{d1}", "N\u{303}"),
        ("\u{d3}", "O\u{301}"),
        ("\u{da}", "U\u{301}"),
        ("\u{dc}", "U\u{308}"),
        ("\u{e1}", "a\u{301}"),
        ("\u{e9}", "e\u{301}"),
        ("\u{ed}", "i\u{301}"),
        ("\u{f1}", "n\u{303}"),
        ("\u{f3}", "o\u{301}"),
        ("\u{fa}", "u\u{301}"),
        ("\u{fc}", "u\u{308}"),
    ] {
        assert!(fixture.text.contains(precomposed));
        assert!(fixture.text.contains(decomposed));
    }
}

#[test]
fn mixed_mathematics_fixture_keeps_math_source_visible_but_uninterpreted() {
    let scenario = EnglishSpanishCorpusScenario::MixedMathematics;
    let matching = REQUIRED_SCENARIO_FIXTURES
        .iter()
        .find(|fixture| fixture.scenario == scenario);
    assert!(
        matching.is_some(),
        "mixed-mathematics fixture must remain present",
    );
    let Some(fixture) = matching else {
        return;
    };
    assert!(fixture.text.contains("x^2 + y^2 = z^2"));
    assert!(fixture.text.contains("\\frac{1}{2}"));
    assert!(fixture.text.contains("\\sqrt{x}"));
}
