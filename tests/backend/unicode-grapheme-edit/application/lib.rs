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
//   - Regression evidence for exact Unicode grapheme-range editing.
// - Must-Not:
//   - Normalize Unicode, mutate notebooks, or define language punctuation.
// - Allows:
//   - Inputs: Deterministic composed/decomposed and multi-code-point text.
//   - Outputs: Exact replacement and typed range assertions.
//   - Side effects: Process-local test allocation only.
// - Split-When:
//   - Language-specific editing receives independent fixtures.
// - Merge-When:
//   - Grapheme editing is fully covered by a broader language test suite.
// - Summary:
//   - Proves grapheme edits preserve authored Unicode outside the chosen range.
// - Description:
//   - Covers combining marks, emoji ZWJ clusters, insertion, and bounds.
// - Usage:
//   - Compile against the edit application and segmentation adapter.
// - Defaults:
//   - NFC and NFD source spellings remain distinct.
//
use atrament_unicode_grapheme_edit::{
    GraphemeRange, GraphemeRangeError, replace_grapheme_range,
};
use atrament_unicode_grapheme_segmentation::UnicodeGraphemeSegmentation;

#[test]
fn combining_mark_and_emoji_sequences_are_edited_as_single_graphemes() {
    let provider = UnicodeGraphemeSegmentation;
    let source = "A e\u{301} 👩‍🔬 Z";
    assert_eq!(
        replace_grapheme_range(
            &provider,
            source,
            GraphemeRange { count: 3, start: 2 },
            "ñ",
        ),
        Ok(String::from("A ñ Z")),
    );
}

#[test]
fn zero_length_ranges_insert_at_start_middle_and_end() {
    let provider = UnicodeGraphemeSegmentation;
    let source = "aé";
    for (start, replacement, expected) in [
        (0, "¡", "¡aé"),
        (1, "—", "a—é"),
        (2, "!", "aé!"),
    ] {
        assert_eq!(
            replace_grapheme_range(
                &provider,
                source,
                GraphemeRange { count: 0, start },
                replacement,
            ),
            Ok(String::from(expected)),
        );
    }
}

#[test]
fn replacement_preserves_normalization_spelling_exactly() {
    let provider = UnicodeGraphemeSegmentation;
    let nfd = "cafe\u{301}";
    let replacement = "sen\u{303}or";
    let edited = replace_grapheme_range(
        &provider,
        nfd,
        GraphemeRange { count: 1, start: 3 },
        replacement,
    )
    .expect("valid exact grapheme replacement");
    assert_eq!(edited, "cafsen\u{303}or");
    assert_ne!(edited, "cafseñor");
}

#[test]
fn invalid_ranges_return_typed_errors() {
    let provider = UnicodeGraphemeSegmentation;
    let source = "abc";
    assert_eq!(
        replace_grapheme_range(
            &provider,
            source,
            GraphemeRange { count: 0, start: 4 },
            "x",
        ),
        Err(GraphemeRangeError::StartOutOfBounds {
            grapheme_count: 3,
            start: 4,
        }),
    );
    assert_eq!(
        replace_grapheme_range(
            &provider,
            source,
            GraphemeRange { count: 2, start: 2 },
            "x",
        ),
        Err(GraphemeRangeError::EndOutOfBounds {
            end: 4,
            grapheme_count: 3,
        }),
    );
    assert_eq!(
        replace_grapheme_range(
            &provider,
            source,
            GraphemeRange {
                count: usize::MAX,
                start: 1,
            },
            "x",
        ),
        Err(GraphemeRangeError::EndIndexOverflow {
            count: usize::MAX,
            start: 1,
        }),
    );
}
