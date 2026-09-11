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
//   - Exact compositional-rule applicability for decomposed Spanish diacritics.
// - Must-Not:
//   - Normalize Unicode, segment arbitrary text, choose profile declarations,
//     place accents, compute geometry/collision, render, or select fallback.
// - Allows:
//   - Inputs: One exact already-segmented grapheme spelling.
//   - Outputs: Acute, tilde, diaeresis, or no bilingual diacritic rule.
//   - Side effects: None.
// - Split-When:
//   - Another language or non-diacritic composition gains rule semantics.
// - Merge-When:
//   - Language-rule evaluation becomes inseparable from profile coverage.
// - Summary:
//   - Matches only the frozen decomposed Spanish diacritic graphemes.
// - Description:
//   - Provides exact external rule evidence without Unicode normalization.
// - Usage:
//   - Match one bilingual grapheme before profile coverage classification.
// - Defaults:
//   - Precomposed and unrecognized spellings have no compositional rule.
//

//! Exact rule matching for first-release Spanish decomposed diacritics.

/// Language-owned compositional rule for one decomposed Spanish grapheme.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum EnglishSpanishDiacriticRule {
    /// Acute accent over an admitted Latin vowel.
    Acute,
    /// Diaeresis over an admitted Latin `U` or `u`.
    Diaeresis,
    /// Tilde over an admitted Latin `N` or `n`.
    Tilde,
}

/// Match one exact decomposed Spanish grapheme to its language rule.
///
/// The matcher performs no normalization. Precomposed spellings deliberately
/// return `None` because exact profile coverage remains a separate path.
#[must_use]
pub fn applicable_english_spanish_diacritic_rule(
    grapheme: &str,
) -> Option<EnglishSpanishDiacriticRule> {
    match grapheme {
        "A\u{301}" | "E\u{301}" | "I\u{301}" | "O\u{301}"
        | "U\u{301}" | "a\u{301}" | "e\u{301}" | "i\u{301}"
        | "o\u{301}" | "u\u{301}" => {
            Some(EnglishSpanishDiacriticRule::Acute)
        }
        "U\u{308}" | "u\u{308}" => Some(EnglishSpanishDiacriticRule::Diaeresis),
        "N\u{303}" | "n\u{303}" => Some(EnglishSpanishDiacriticRule::Tilde),
        _ => None,
    }
}
