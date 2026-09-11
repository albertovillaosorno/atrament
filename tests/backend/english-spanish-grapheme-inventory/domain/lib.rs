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
//   - Regression evidence for the exact non-mathematical English/Spanish
//     handwriting grapheme inventory.
// - Must-Not:
//   - Normalize text, infer mathematical symbols, segment arbitrary input,
//     evaluate profile coverage, or render glyphs.
// - Allows:
//   - Inputs: The frozen language-owned textual grapheme declarations.
//   - Outputs: Assertions over membership, uniqueness, category, and exact
//     precomposed/decomposed spellings.
//   - Side effects: Process-local test allocation only.
// - Split-When:
//   - Mathematical inventory or full corpus integration gains fixtures.
// - Merge-When:
//   - One complete language acceptance harness supersedes inventory checks.
// - Summary:
//   - Pins the exact textual graphemes required by the bilingual baseline.
// - Description:
//   - Prevents silent loss of letters, Spanish diacritics, numerals, or named
//     punctuation while keeping mathematical authority separate.
// - Usage:
//   - Compile directly against the English/Spanish grapheme-inventory domain.
// - Defaults:
//   - NFC and NFD forms remain distinct entries.
//
use std::collections::BTreeSet;

use EnglishSpanishGraphemeCategory::{
    Numeral, Punctuation, SpanishDecomposedEquivalent, SpanishPrecomposedLetter,
};
use atrament_english_spanish_grapheme_inventory::{
    EnglishSpanishGraphemeCategory, REQUIRED_TEXT_GRAPHEMES,
};

fn values_for(category: EnglishSpanishGraphemeCategory) -> Vec<&'static str> {
    REQUIRED_TEXT_GRAPHEMES
        .iter()
        .filter(|required| required.category == category)
        .map(|required| required.grapheme)
        .collect()
}

#[test]
fn inventory_has_exact_category_sizes_and_no_duplicate_spellings() {
    let mut unique = BTreeSet::new();
    let mut basic_letters = 0_usize;
    let mut numerals = 0_usize;
    let mut punctuation = 0_usize;
    let mut decomposed = 0_usize;
    let mut precomposed = 0_usize;

    for required in REQUIRED_TEXT_GRAPHEMES {
        assert!(
            unique.insert(required.grapheme),
            "duplicate required grapheme {:?}",
            required.grapheme,
        );
        match required.category {
            EnglishSpanishGraphemeCategory::BasicLatinLetter => {
                basic_letters += 1;
            }
            EnglishSpanishGraphemeCategory::Numeral => numerals += 1,
            EnglishSpanishGraphemeCategory::Punctuation => punctuation += 1,
            EnglishSpanishGraphemeCategory::SpanishDecomposedEquivalent => {
                decomposed += 1;
            }
            EnglishSpanishGraphemeCategory::SpanishPrecomposedLetter => {
                precomposed += 1;
            }
        }
    }

    assert_eq!(basic_letters, 52);
    assert_eq!(precomposed, 14);
    assert_eq!(decomposed, 14);
    assert_eq!(numerals, 10);
    assert_eq!(punctuation, 24);
    assert_eq!(unique.len(), 114);
    assert_eq!(REQUIRED_TEXT_GRAPHEMES.len(), 114);
}

#[test]
fn basic_latin_letters_are_complete_in_uppercase_then_lowercase() {
    let letters = values_for(EnglishSpanishGraphemeCategory::BasicLatinLetter);
    let expected = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz"
        .chars()
        .map(|character| character.to_string())
        .collect::<Vec<_>>();
    assert_eq!(letters.len(), expected.len());
    for (actual, expected) in letters.iter().zip(expected.iter()) {
        assert_eq!(*actual, expected);
    }
}

#[test]
fn spanish_precomposed_and_decomposed_forms_are_both_exact_requirements() {
    let precomposed = values_for(SpanishPrecomposedLetter);
    let decomposed = values_for(SpanishDecomposedEquivalent);
    assert_eq!(
        precomposed,
        "ÁÉÍÑÓÚÜáéíñóúü"
            .chars()
            .map(|character| character.to_string())
            .collect::<Vec<_>>(),
    );
    assert_eq!(
        decomposed,
        [
            concat!("A", "\u{301}"),
            concat!("E", "\u{301}"),
            concat!("I", "\u{301}"),
            concat!("N", "\u{303}"),
            concat!("O", "\u{301}"),
            concat!("U", "\u{301}"),
            concat!("U", "\u{308}"),
            concat!("a", "\u{301}"),
            concat!("e", "\u{301}"),
            concat!("i", "\u{301}"),
            concat!("n", "\u{303}"),
            concat!("o", "\u{301}"),
            concat!("u", "\u{301}"),
            concat!("u", "\u{308}"),
        ],
    );
    for (normalized, decomposed) in precomposed.iter().zip(decomposed.iter()) {
        assert_ne!(normalized, decomposed);
        assert!(decomposed.chars().count() > 1);
    }
}

#[test]
fn numerals_and_named_language_punctuation_remain_explicit() {
    assert_eq!(
        values_for(Numeral),
        "0123456789"
            .chars()
            .map(|character| character.to_string())
            .collect::<Vec<_>>(),
    );
    assert_eq!(
        values_for(Punctuation),
        ".,;:?!¿¡\"'“”‘’«»…–—-()[]"
            .chars()
            .map(|character| character.to_string())
            .collect::<Vec<_>>(),
    );
}

#[test]
fn mathematical_symbols_are_not_redefined_by_language_inventory() {
    let required = REQUIRED_TEXT_GRAPHEMES
        .iter()
        .map(|entry| entry.grapheme)
        .collect::<BTreeSet<_>>();
    for mathematical in ["∫", "√", "≤", "α", "∞"] {
        assert!(
            !required.contains(mathematical),
            "mathematical authority leaked into textual inventory: \
             {mathematical}",
        );
    }
}

#[test]
fn required_graphemes_are_nonempty_and_never_whitespace_only() {
    for required in REQUIRED_TEXT_GRAPHEMES {
        assert!(!required.grapheme.is_empty());
        assert!(
            required
                .grapheme
                .chars()
                .any(|character| !character.is_whitespace()),
        );
    }
}
