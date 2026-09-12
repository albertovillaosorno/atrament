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
//   - Exact non-mathematical graphemes required by the first English/Spanish
//     handwriting baseline.
// - Must-Not:
//   - Normalize authored text, segment arbitrary Unicode, define mathematical
//     symbol coverage, choose compositional rules, inspect profile coverage,
//     render glyphs, or define portable wire fields.
// - Allows:
//   - Inputs: None for the frozen textual baseline; separately admitted
//     mathematical graphemes remain owned by their mathematical authority.
//   - Outputs: Exact required grapheme strings and their language categories.
//   - Side effects: None.
// - Split-When:
//   - Mathematical glyph inventory or language-specific shaping gains its own
//     executable authority.
// - Merge-When:
//   - Textual and mathematical glyph authorities share one frozen inventory.
// - Summary:
//   - Freezes exact English/Spanish handwriting graphemes without
//     normalization.
// - Description:
//   - Enumerates Latin letters, Spanish precomposed and decomposed forms,
//     numerals, and the punctuation explicitly required by the accepted
//     English/Spanish language baseline.
// - Usage:
//   - Combine this textual inventory with the separately admitted mathematical
//     symbol inventory before checking handwriting-profile completeness.
// - Defaults:
//   - NFC and NFD spellings remain distinct exact grapheme requirements.
//

//! Exact textual grapheme inventory for the first English/Spanish baseline.

/// Frozen non-mathematical handwriting inventory for English and Spanish.
///
/// Mathematical symbols are deliberately absent because the accepted language
/// ADR delegates them to the separately admitted mathematical symbol inventory.
/// A complete handwriting-coverage gate must combine both authorities instead
/// of redefining mathematical support here.
pub const REQUIRED_TEXT_GRAPHEMES: &[RequiredEnglishSpanishGrapheme] = &[
    RequiredEnglishSpanishGrapheme {
        category: EnglishSpanishGraphemeCategory::BasicLatinLetter,
        grapheme: "A",
    },
    RequiredEnglishSpanishGrapheme {
        category: EnglishSpanishGraphemeCategory::BasicLatinLetter,
        grapheme: "B",
    },
    RequiredEnglishSpanishGrapheme {
        category: EnglishSpanishGraphemeCategory::BasicLatinLetter,
        grapheme: "C",
    },
    RequiredEnglishSpanishGrapheme {
        category: EnglishSpanishGraphemeCategory::BasicLatinLetter,
        grapheme: "D",
    },
    RequiredEnglishSpanishGrapheme {
        category: EnglishSpanishGraphemeCategory::BasicLatinLetter,
        grapheme: "E",
    },
    RequiredEnglishSpanishGrapheme {
        category: EnglishSpanishGraphemeCategory::BasicLatinLetter,
        grapheme: "F",
    },
    RequiredEnglishSpanishGrapheme {
        category: EnglishSpanishGraphemeCategory::BasicLatinLetter,
        grapheme: "G",
    },
    RequiredEnglishSpanishGrapheme {
        category: EnglishSpanishGraphemeCategory::BasicLatinLetter,
        grapheme: "H",
    },
    RequiredEnglishSpanishGrapheme {
        category: EnglishSpanishGraphemeCategory::BasicLatinLetter,
        grapheme: "I",
    },
    RequiredEnglishSpanishGrapheme {
        category: EnglishSpanishGraphemeCategory::BasicLatinLetter,
        grapheme: "J",
    },
    RequiredEnglishSpanishGrapheme {
        category: EnglishSpanishGraphemeCategory::BasicLatinLetter,
        grapheme: "K",
    },
    RequiredEnglishSpanishGrapheme {
        category: EnglishSpanishGraphemeCategory::BasicLatinLetter,
        grapheme: "L",
    },
    RequiredEnglishSpanishGrapheme {
        category: EnglishSpanishGraphemeCategory::BasicLatinLetter,
        grapheme: "M",
    },
    RequiredEnglishSpanishGrapheme {
        category: EnglishSpanishGraphemeCategory::BasicLatinLetter,
        grapheme: "N",
    },
    RequiredEnglishSpanishGrapheme {
        category: EnglishSpanishGraphemeCategory::BasicLatinLetter,
        grapheme: "O",
    },
    RequiredEnglishSpanishGrapheme {
        category: EnglishSpanishGraphemeCategory::BasicLatinLetter,
        grapheme: "P",
    },
    RequiredEnglishSpanishGrapheme {
        category: EnglishSpanishGraphemeCategory::BasicLatinLetter,
        grapheme: "Q",
    },
    RequiredEnglishSpanishGrapheme {
        category: EnglishSpanishGraphemeCategory::BasicLatinLetter,
        grapheme: "R",
    },
    RequiredEnglishSpanishGrapheme {
        category: EnglishSpanishGraphemeCategory::BasicLatinLetter,
        grapheme: "S",
    },
    RequiredEnglishSpanishGrapheme {
        category: EnglishSpanishGraphemeCategory::BasicLatinLetter,
        grapheme: "T",
    },
    RequiredEnglishSpanishGrapheme {
        category: EnglishSpanishGraphemeCategory::BasicLatinLetter,
        grapheme: "U",
    },
    RequiredEnglishSpanishGrapheme {
        category: EnglishSpanishGraphemeCategory::BasicLatinLetter,
        grapheme: "V",
    },
    RequiredEnglishSpanishGrapheme {
        category: EnglishSpanishGraphemeCategory::BasicLatinLetter,
        grapheme: "W",
    },
    RequiredEnglishSpanishGrapheme {
        category: EnglishSpanishGraphemeCategory::BasicLatinLetter,
        grapheme: "X",
    },
    RequiredEnglishSpanishGrapheme {
        category: EnglishSpanishGraphemeCategory::BasicLatinLetter,
        grapheme: "Y",
    },
    RequiredEnglishSpanishGrapheme {
        category: EnglishSpanishGraphemeCategory::BasicLatinLetter,
        grapheme: "Z",
    },
    RequiredEnglishSpanishGrapheme {
        category: EnglishSpanishGraphemeCategory::BasicLatinLetter,
        grapheme: "a",
    },
    RequiredEnglishSpanishGrapheme {
        category: EnglishSpanishGraphemeCategory::BasicLatinLetter,
        grapheme: "b",
    },
    RequiredEnglishSpanishGrapheme {
        category: EnglishSpanishGraphemeCategory::BasicLatinLetter,
        grapheme: "c",
    },
    RequiredEnglishSpanishGrapheme {
        category: EnglishSpanishGraphemeCategory::BasicLatinLetter,
        grapheme: "d",
    },
    RequiredEnglishSpanishGrapheme {
        category: EnglishSpanishGraphemeCategory::BasicLatinLetter,
        grapheme: "e",
    },
    RequiredEnglishSpanishGrapheme {
        category: EnglishSpanishGraphemeCategory::BasicLatinLetter,
        grapheme: "f",
    },
    RequiredEnglishSpanishGrapheme {
        category: EnglishSpanishGraphemeCategory::BasicLatinLetter,
        grapheme: "g",
    },
    RequiredEnglishSpanishGrapheme {
        category: EnglishSpanishGraphemeCategory::BasicLatinLetter,
        grapheme: "h",
    },
    RequiredEnglishSpanishGrapheme {
        category: EnglishSpanishGraphemeCategory::BasicLatinLetter,
        grapheme: "i",
    },
    RequiredEnglishSpanishGrapheme {
        category: EnglishSpanishGraphemeCategory::BasicLatinLetter,
        grapheme: "j",
    },
    RequiredEnglishSpanishGrapheme {
        category: EnglishSpanishGraphemeCategory::BasicLatinLetter,
        grapheme: "k",
    },
    RequiredEnglishSpanishGrapheme {
        category: EnglishSpanishGraphemeCategory::BasicLatinLetter,
        grapheme: "l",
    },
    RequiredEnglishSpanishGrapheme {
        category: EnglishSpanishGraphemeCategory::BasicLatinLetter,
        grapheme: "m",
    },
    RequiredEnglishSpanishGrapheme {
        category: EnglishSpanishGraphemeCategory::BasicLatinLetter,
        grapheme: "n",
    },
    RequiredEnglishSpanishGrapheme {
        category: EnglishSpanishGraphemeCategory::BasicLatinLetter,
        grapheme: "o",
    },
    RequiredEnglishSpanishGrapheme {
        category: EnglishSpanishGraphemeCategory::BasicLatinLetter,
        grapheme: "p",
    },
    RequiredEnglishSpanishGrapheme {
        category: EnglishSpanishGraphemeCategory::BasicLatinLetter,
        grapheme: "q",
    },
    RequiredEnglishSpanishGrapheme {
        category: EnglishSpanishGraphemeCategory::BasicLatinLetter,
        grapheme: "r",
    },
    RequiredEnglishSpanishGrapheme {
        category: EnglishSpanishGraphemeCategory::BasicLatinLetter,
        grapheme: "s",
    },
    RequiredEnglishSpanishGrapheme {
        category: EnglishSpanishGraphemeCategory::BasicLatinLetter,
        grapheme: "t",
    },
    RequiredEnglishSpanishGrapheme {
        category: EnglishSpanishGraphemeCategory::BasicLatinLetter,
        grapheme: "u",
    },
    RequiredEnglishSpanishGrapheme {
        category: EnglishSpanishGraphemeCategory::BasicLatinLetter,
        grapheme: "v",
    },
    RequiredEnglishSpanishGrapheme {
        category: EnglishSpanishGraphemeCategory::BasicLatinLetter,
        grapheme: "w",
    },
    RequiredEnglishSpanishGrapheme {
        category: EnglishSpanishGraphemeCategory::BasicLatinLetter,
        grapheme: "x",
    },
    RequiredEnglishSpanishGrapheme {
        category: EnglishSpanishGraphemeCategory::BasicLatinLetter,
        grapheme: "y",
    },
    RequiredEnglishSpanishGrapheme {
        category: EnglishSpanishGraphemeCategory::BasicLatinLetter,
        grapheme: "z",
    },
    RequiredEnglishSpanishGrapheme {
        category: EnglishSpanishGraphemeCategory::SpanishPrecomposedLetter,
        grapheme: "\u{c1}",
    },
    RequiredEnglishSpanishGrapheme {
        category: EnglishSpanishGraphemeCategory::SpanishPrecomposedLetter,
        grapheme: "\u{c9}",
    },
    RequiredEnglishSpanishGrapheme {
        category: EnglishSpanishGraphemeCategory::SpanishPrecomposedLetter,
        grapheme: "\u{cd}",
    },
    RequiredEnglishSpanishGrapheme {
        category: EnglishSpanishGraphemeCategory::SpanishPrecomposedLetter,
        grapheme: "\u{d1}",
    },
    RequiredEnglishSpanishGrapheme {
        category: EnglishSpanishGraphemeCategory::SpanishPrecomposedLetter,
        grapheme: "\u{d3}",
    },
    RequiredEnglishSpanishGrapheme {
        category: EnglishSpanishGraphemeCategory::SpanishPrecomposedLetter,
        grapheme: "\u{da}",
    },
    RequiredEnglishSpanishGrapheme {
        category: EnglishSpanishGraphemeCategory::SpanishPrecomposedLetter,
        grapheme: "\u{dc}",
    },
    RequiredEnglishSpanishGrapheme {
        category: EnglishSpanishGraphemeCategory::SpanishPrecomposedLetter,
        grapheme: "\u{e1}",
    },
    RequiredEnglishSpanishGrapheme {
        category: EnglishSpanishGraphemeCategory::SpanishPrecomposedLetter,
        grapheme: "\u{e9}",
    },
    RequiredEnglishSpanishGrapheme {
        category: EnglishSpanishGraphemeCategory::SpanishPrecomposedLetter,
        grapheme: "\u{ed}",
    },
    RequiredEnglishSpanishGrapheme {
        category: EnglishSpanishGraphemeCategory::SpanishPrecomposedLetter,
        grapheme: "\u{f1}",
    },
    RequiredEnglishSpanishGrapheme {
        category: EnglishSpanishGraphemeCategory::SpanishPrecomposedLetter,
        grapheme: "\u{f3}",
    },
    RequiredEnglishSpanishGrapheme {
        category: EnglishSpanishGraphemeCategory::SpanishPrecomposedLetter,
        grapheme: "\u{fa}",
    },
    RequiredEnglishSpanishGrapheme {
        category: EnglishSpanishGraphemeCategory::SpanishPrecomposedLetter,
        grapheme: "\u{fc}",
    },
    RequiredEnglishSpanishGrapheme {
        category: EnglishSpanishGraphemeCategory::SpanishDecomposedEquivalent,
        grapheme: "A\u{301}",
    },
    RequiredEnglishSpanishGrapheme {
        category: EnglishSpanishGraphemeCategory::SpanishDecomposedEquivalent,
        grapheme: "E\u{301}",
    },
    RequiredEnglishSpanishGrapheme {
        category: EnglishSpanishGraphemeCategory::SpanishDecomposedEquivalent,
        grapheme: "I\u{301}",
    },
    RequiredEnglishSpanishGrapheme {
        category: EnglishSpanishGraphemeCategory::SpanishDecomposedEquivalent,
        grapheme: "N\u{303}",
    },
    RequiredEnglishSpanishGrapheme {
        category: EnglishSpanishGraphemeCategory::SpanishDecomposedEquivalent,
        grapheme: "O\u{301}",
    },
    RequiredEnglishSpanishGrapheme {
        category: EnglishSpanishGraphemeCategory::SpanishDecomposedEquivalent,
        grapheme: "U\u{301}",
    },
    RequiredEnglishSpanishGrapheme {
        category: EnglishSpanishGraphemeCategory::SpanishDecomposedEquivalent,
        grapheme: "U\u{308}",
    },
    RequiredEnglishSpanishGrapheme {
        category: EnglishSpanishGraphemeCategory::SpanishDecomposedEquivalent,
        grapheme: "a\u{301}",
    },
    RequiredEnglishSpanishGrapheme {
        category: EnglishSpanishGraphemeCategory::SpanishDecomposedEquivalent,
        grapheme: "e\u{301}",
    },
    RequiredEnglishSpanishGrapheme {
        category: EnglishSpanishGraphemeCategory::SpanishDecomposedEquivalent,
        grapheme: "i\u{301}",
    },
    RequiredEnglishSpanishGrapheme {
        category: EnglishSpanishGraphemeCategory::SpanishDecomposedEquivalent,
        grapheme: "n\u{303}",
    },
    RequiredEnglishSpanishGrapheme {
        category: EnglishSpanishGraphemeCategory::SpanishDecomposedEquivalent,
        grapheme: "o\u{301}",
    },
    RequiredEnglishSpanishGrapheme {
        category: EnglishSpanishGraphemeCategory::SpanishDecomposedEquivalent,
        grapheme: "u\u{301}",
    },
    RequiredEnglishSpanishGrapheme {
        category: EnglishSpanishGraphemeCategory::SpanishDecomposedEquivalent,
        grapheme: "u\u{308}",
    },
    RequiredEnglishSpanishGrapheme {
        category: EnglishSpanishGraphemeCategory::Numeral,
        grapheme: "0",
    },
    RequiredEnglishSpanishGrapheme {
        category: EnglishSpanishGraphemeCategory::Numeral,
        grapheme: "1",
    },
    RequiredEnglishSpanishGrapheme {
        category: EnglishSpanishGraphemeCategory::Numeral,
        grapheme: "2",
    },
    RequiredEnglishSpanishGrapheme {
        category: EnglishSpanishGraphemeCategory::Numeral,
        grapheme: "3",
    },
    RequiredEnglishSpanishGrapheme {
        category: EnglishSpanishGraphemeCategory::Numeral,
        grapheme: "4",
    },
    RequiredEnglishSpanishGrapheme {
        category: EnglishSpanishGraphemeCategory::Numeral,
        grapheme: "5",
    },
    RequiredEnglishSpanishGrapheme {
        category: EnglishSpanishGraphemeCategory::Numeral,
        grapheme: "6",
    },
    RequiredEnglishSpanishGrapheme {
        category: EnglishSpanishGraphemeCategory::Numeral,
        grapheme: "7",
    },
    RequiredEnglishSpanishGrapheme {
        category: EnglishSpanishGraphemeCategory::Numeral,
        grapheme: "8",
    },
    RequiredEnglishSpanishGrapheme {
        category: EnglishSpanishGraphemeCategory::Numeral,
        grapheme: "9",
    },
    RequiredEnglishSpanishGrapheme {
        category: EnglishSpanishGraphemeCategory::Punctuation,
        grapheme: ".",
    },
    RequiredEnglishSpanishGrapheme {
        category: EnglishSpanishGraphemeCategory::Punctuation,
        grapheme: ",",
    },
    RequiredEnglishSpanishGrapheme {
        category: EnglishSpanishGraphemeCategory::Punctuation,
        grapheme: ";",
    },
    RequiredEnglishSpanishGrapheme {
        category: EnglishSpanishGraphemeCategory::Punctuation,
        grapheme: ":",
    },
    RequiredEnglishSpanishGrapheme {
        category: EnglishSpanishGraphemeCategory::Punctuation,
        grapheme: "?",
    },
    RequiredEnglishSpanishGrapheme {
        category: EnglishSpanishGraphemeCategory::Punctuation,
        grapheme: "!",
    },
    RequiredEnglishSpanishGrapheme {
        category: EnglishSpanishGraphemeCategory::Punctuation,
        grapheme: "\u{bf}",
    },
    RequiredEnglishSpanishGrapheme {
        category: EnglishSpanishGraphemeCategory::Punctuation,
        grapheme: "\u{a1}",
    },
    RequiredEnglishSpanishGrapheme {
        category: EnglishSpanishGraphemeCategory::Punctuation,
        grapheme: "\"",
    },
    RequiredEnglishSpanishGrapheme {
        category: EnglishSpanishGraphemeCategory::Punctuation,
        grapheme: "'",
    },
    RequiredEnglishSpanishGrapheme {
        category: EnglishSpanishGraphemeCategory::Punctuation,
        grapheme: "\u{201c}",
    },
    RequiredEnglishSpanishGrapheme {
        category: EnglishSpanishGraphemeCategory::Punctuation,
        grapheme: "\u{201d}",
    },
    RequiredEnglishSpanishGrapheme {
        category: EnglishSpanishGraphemeCategory::Punctuation,
        grapheme: "\u{2018}",
    },
    RequiredEnglishSpanishGrapheme {
        category: EnglishSpanishGraphemeCategory::Punctuation,
        grapheme: "\u{2019}",
    },
    RequiredEnglishSpanishGrapheme {
        category: EnglishSpanishGraphemeCategory::Punctuation,
        grapheme: "\u{ab}",
    },
    RequiredEnglishSpanishGrapheme {
        category: EnglishSpanishGraphemeCategory::Punctuation,
        grapheme: "\u{bb}",
    },
    RequiredEnglishSpanishGrapheme {
        category: EnglishSpanishGraphemeCategory::Punctuation,
        grapheme: "\u{2026}",
    },
    RequiredEnglishSpanishGrapheme {
        category: EnglishSpanishGraphemeCategory::Punctuation,
        grapheme: "\u{2013}",
    },
    RequiredEnglishSpanishGrapheme {
        category: EnglishSpanishGraphemeCategory::Punctuation,
        grapheme: "\u{2014}",
    },
    RequiredEnglishSpanishGrapheme {
        category: EnglishSpanishGraphemeCategory::Punctuation,
        grapheme: "-",
    },
    RequiredEnglishSpanishGrapheme {
        category: EnglishSpanishGraphemeCategory::Punctuation,
        grapheme: "(",
    },
    RequiredEnglishSpanishGrapheme {
        category: EnglishSpanishGraphemeCategory::Punctuation,
        grapheme: ")",
    },
    RequiredEnglishSpanishGrapheme {
        category: EnglishSpanishGraphemeCategory::Punctuation,
        grapheme: "[",
    },
    RequiredEnglishSpanishGrapheme {
        category: EnglishSpanishGraphemeCategory::Punctuation,
        grapheme: "]",
    },
];

/// Semantic reason one exact textual grapheme belongs to the baseline.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum EnglishSpanishGraphemeCategory {
    /// ASCII Latin letter used by both English and Spanish.
    BasicLatinLetter,
    /// Decimal numeral used by both English and Spanish.
    Numeral,
    /// Sentence or typographic punctuation required by the language baseline.
    Punctuation,
    /// Decomposed Unicode equivalent retained as an exact distinct spelling.
    SpanishDecomposedEquivalent,
    /// Precomposed Spanish letter carrying an admitted diacritic.
    SpanishPrecomposedLetter,
}

/// One exact textual grapheme required before handwriting projection.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct RequiredEnglishSpanishGrapheme {
    /// Why this grapheme belongs to the first-release textual baseline.
    pub category: EnglishSpanishGraphemeCategory,
    /// Exact Unicode grapheme spelling retained without normalization.
    pub grapheme: &'static str,
}
