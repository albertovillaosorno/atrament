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
use std::cell::Cell;

use atrament_unicode_grapheme_boundary_port::GraphemeBoundaryProvider;
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
fn empty_source_accepts_zero_grapheme_insertion() {
    let provider = UnicodeGraphemeSegmentation;
    assert_eq!(
        replace_grapheme_range(
            &provider,
            "",
            GraphemeRange { count: 0, start: 0 },
            "ñ",
        ),
        Ok(String::from("ñ")),
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

struct ChangingZeroCountAnchorProvider {
    boundary_calls: Cell<usize>,
}

impl GraphemeBoundaryProvider for ChangingZeroCountAnchorProvider {
    fn byte_offset(
        &self,
        source: &str,
        grapheme_index: usize,
    ) -> Option<usize> {
        assert_eq!(grapheme_index, 0);
        let calls = self.boundary_calls.get();
        self.boundary_calls.set(calls.saturating_add(1));
        if calls == 0 {
            Some(0)
        } else {
            Some(source.len())
        }
    }

    fn grapheme_count(&self, _source: &str) -> usize {
        0
    }
}

#[test]
fn zero_grapheme_count_reuses_the_single_source_anchor() {
    let provider = ChangingZeroCountAnchorProvider {
        boundary_calls: Cell::new(0),
    };
    let source = "x";
    assert_eq!(
        replace_grapheme_range(
            &provider,
            source,
            GraphemeRange { count: 0, start: 0 },
            "!",
        ),
        Err(GraphemeRangeError::BoundaryAnchorMismatch {
            expected: source.len(),
            grapheme_index: 0,
            observed: 0,
        }),
    );
    assert_eq!(provider.boundary_calls.get(), 1);
}

struct ChangingInsertionBoundaryProvider {
    internal_calls: Cell<usize>,
}

impl GraphemeBoundaryProvider for ChangingInsertionBoundaryProvider {
    fn byte_offset(
        &self,
        source: &str,
        grapheme_index: usize,
    ) -> Option<usize> {
        match grapheme_index {
            0 => Some(0),
            1 => {
                let calls = self.internal_calls.get();
                self.internal_calls.set(calls.saturating_add(1));
                if calls == 0 {
                    Some("é".len())
                } else {
                    Some(source.len())
                }
            },
            2 => Some(source.len()),
            _ => None,
        }
    }

    fn grapheme_count(&self, _source: &str) -> usize {
        2
    }
}

#[test]
fn zero_length_range_resolves_one_internal_boundary_once() {
    let provider = ChangingInsertionBoundaryProvider {
        internal_calls: Cell::new(0),
    };
    assert_eq!(
        replace_grapheme_range(
            &provider,
            "éx",
            GraphemeRange { count: 0, start: 1 },
            "!",
        ),
        Ok(String::from("é!x")),
    );
    assert_eq!(provider.internal_calls.get(), 1);
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

#[derive(Clone, Copy)]
enum BrokenBoundaryMode {
    InvalidUtf8,
    Missing,
    NonAdvancing,
    Reversed,
    ShiftedFirst,
    TruncatedFinal,
    UnderreportedCount,
}

struct BrokenBoundaryProvider {
    mode: BrokenBoundaryMode,
}

impl GraphemeBoundaryProvider for BrokenBoundaryProvider {
    fn byte_offset(
        &self,
        source: &str,
        grapheme_index: usize,
    ) -> Option<usize> {
        match self.mode {
            BrokenBoundaryMode::InvalidUtf8 => match grapheme_index {
                0 => Some(0),
                1 => Some(1),
                _ => Some(source.len()),
            },
            BrokenBoundaryMode::Missing => match grapheme_index {
                0 => Some(0),
                1 => None,
                _ => Some(source.len()),
            },
            BrokenBoundaryMode::NonAdvancing => match grapheme_index {
                0 => Some(0),
                _ => Some(source.len()),
            },
            BrokenBoundaryMode::Reversed => match grapheme_index {
                0 | 2 => Some(0),
                _ => Some(source.len()),
            },
            BrokenBoundaryMode::ShiftedFirst => match grapheme_index {
                0 => Some("é".len()),
                1 => Some("é".len()),
                _ => Some(source.len()),
            },
            BrokenBoundaryMode::TruncatedFinal => match grapheme_index {
                0 => Some(0),
                1 => Some("é".len()),
                _ => Some("é".len()),
            },
            BrokenBoundaryMode::UnderreportedCount => Some(0),
        }
    }

    fn grapheme_count(&self, _source: &str) -> usize {
        match self.mode {
            BrokenBoundaryMode::Reversed => 3,
            BrokenBoundaryMode::UnderreportedCount => 0,
            _ => 2,
        }
    }
}

#[test]
fn provider_boundary_failures_are_not_reported_as_user_range_errors() {
    let source = "éx";
    assert_eq!(
        replace_grapheme_range(
            &BrokenBoundaryProvider {
                mode: BrokenBoundaryMode::UnderreportedCount,
            },
            source,
            GraphemeRange { count: 0, start: 1 },
            "z",
        ),
        Err(GraphemeRangeError::BoundaryAnchorMismatch {
            expected: source.len(),
            grapheme_index: 0,
            observed: 0,
        }),
    );
    assert_eq!(
        replace_grapheme_range(
            &BrokenBoundaryProvider {
                mode: BrokenBoundaryMode::ShiftedFirst,
            },
            source,
            GraphemeRange { count: 1, start: 1 },
            "z",
        ),
        Err(GraphemeRangeError::BoundaryAnchorMismatch {
            expected: 0,
            grapheme_index: 0,
            observed: "é".len(),
        }),
    );
    assert_eq!(
        replace_grapheme_range(
            &BrokenBoundaryProvider {
                mode: BrokenBoundaryMode::TruncatedFinal,
            },
            source,
            GraphemeRange { count: 1, start: 0 },
            "z",
        ),
        Err(GraphemeRangeError::BoundaryAnchorMismatch {
            expected: source.len(),
            grapheme_index: 2,
            observed: "é".len(),
        }),
    );
    assert_eq!(
        replace_grapheme_range(
            &BrokenBoundaryProvider {
                mode: BrokenBoundaryMode::Missing,
            },
            source,
            GraphemeRange { count: 1, start: 1 },
            "z",
        ),
        Err(GraphemeRangeError::BoundaryUnavailable { grapheme_index: 1 }),
    );
    assert_eq!(
        replace_grapheme_range(
            &BrokenBoundaryProvider {
                mode: BrokenBoundaryMode::InvalidUtf8,
            },
            source,
            GraphemeRange { count: 1, start: 1 },
            "z",
        ),
        Err(GraphemeRangeError::InvalidBoundary {
            byte_offset: 1,
            grapheme_index: 1,
        }),
    );
    assert_eq!(
        replace_grapheme_range(
            &BrokenBoundaryProvider {
                mode: BrokenBoundaryMode::NonAdvancing,
            },
            source,
            GraphemeRange { count: 1, start: 1 },
            "z",
        ),
        Err(GraphemeRangeError::NonAdvancingBoundaries {
            end_byte: source.len(),
            start_byte: source.len(),
        }),
    );
    assert_eq!(
        replace_grapheme_range(
            &BrokenBoundaryProvider {
                mode: BrokenBoundaryMode::Reversed,
            },
            source,
            GraphemeRange { count: 1, start: 1 },
            "z",
        ),
        Err(GraphemeRangeError::ReversedBoundaries {
            end_byte: 0,
            start_byte: source.len(),
        }),
    );
}
