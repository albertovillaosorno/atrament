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
//   - Regression evidence for exact Unicode grapheme cursor-position and
//     anchor/focus selection resolution.
// - Must-Not:
//   - Normalize Unicode, define cursor movement/selection-extension policy,
//     mutate notebooks, or choose browser keybindings.
// - Allows:
//   - Inputs: Deterministic composed/decomposed and multi-code-point text plus
//     injected provider failures.
//   - Outputs: Exact boundary positions and typed rejection assertions.
//   - Side effects: Process-local test allocation only.
// - Split-When:
//   - Cursor movement or visual-affinity policy receives independent fixtures.
// - Merge-When:
//   - Cursor positioning is covered by a broader language acceptance harness.
// - Summary:
//   - Proves cursor positions resolve only at admitted grapheme boundaries.
// - Description:
//   - Covers every boundary, empty text, combining marks, emoji, and provider
//     inconsistencies without choosing movement behavior.
// - Usage:
//   - Compile against the cursor application and segmentation adapter.
// - Defaults:
//   - Start and final boundaries are valid positions.
//
use std::cell::Cell;

use atrament_english_spanish_grapheme_inventory::REQUIRED_TEXT_GRAPHEMES;
use atrament_unicode_grapheme_boundary_port::GraphemeBoundaryProvider;
use atrament_unicode_grapheme_cursor::GraphemeCursorError;
use atrament_unicode_grapheme_cursor::GraphemeCursorPosition;
use atrament_unicode_grapheme_cursor::GraphemeCursorSelection;
use atrament_unicode_grapheme_cursor::resolve_grapheme_cursor_position;
use atrament_unicode_grapheme_cursor::resolve_grapheme_cursor_selection;
use atrament_unicode_grapheme_cursor::resolve_grapheme_cursor_step;
use atrament_unicode_grapheme_segmentation::UnicodeGraphemeSegmentation;

#[test]
fn composed_decomposed_and_emoji_text_resolves_every_cursor_boundary() {
    let provider = UnicodeGraphemeSegmentation;
    let source = "Áe\u{301}👩‍🔬Z";
    let expected = [
        0,
        "Á".len(),
        "Áe\u{301}".len(),
        "Áe\u{301}👩‍🔬".len(),
        source.len(),
    ];
    for (grapheme_index, byte_offset) in expected.into_iter().enumerate() {
        let resolved =
            resolve_grapheme_cursor_position(&provider, source, grapheme_index);
        assert_eq!(
            resolved,
            Ok(GraphemeCursorPosition {
                byte_offset,
                grapheme_index,
            }),
        );
    }
}

#[test]
fn empty_text_has_one_valid_cursor_position() {
    let provider = UnicodeGraphemeSegmentation;
    assert_eq!(
        resolve_grapheme_cursor_position(&provider, "", 0),
        Ok(GraphemeCursorPosition {
            byte_offset: 0,
            grapheme_index: 0,
        }),
    );
    assert_eq!(
        resolve_grapheme_cursor_position(&provider, "", 1),
        Err(GraphemeCursorError::CursorOutOfBounds {
            grapheme_count: 0,
            grapheme_index: 1,
        }),
    );
}

struct ChangingInternalBoundaryProvider {
    internal_calls: Cell<usize>,
}

impl GraphemeBoundaryProvider for ChangingInternalBoundaryProvider {
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
            }
            2 => Some(source.len()),
            _ => None,
        }
    }

    fn grapheme_count(&self, _source: &str) -> usize {
        2
    }
}

#[test]
fn internal_cursor_boundary_is_resolved_once() {
    let provider = ChangingInternalBoundaryProvider {
        internal_calls: Cell::new(0),
    };
    assert_eq!(
        resolve_grapheme_cursor_position(&provider, "éx", 1),
        Ok(GraphemeCursorPosition {
            byte_offset: "é".len(),
            grapheme_index: 1,
        }),
    );
    assert_eq!(provider.internal_calls.get(), 1);
}

struct BrokenBoundaryProvider {
    mode: BrokenBoundaryMode,
}

#[derive(Clone, Copy)]
enum BrokenBoundaryMode {
    InvalidUtf8,
    MissingInternal,
    ShiftedFirst,
    TruncatedFinal,
    UnderreportedCount,
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
            BrokenBoundaryMode::MissingInternal => match grapheme_index {
                0 => Some(0),
                1 => None,
                _ => Some(source.len()),
            },
            BrokenBoundaryMode::ShiftedFirst => match grapheme_index {
                0 => Some("é".len()),
                _ => Some(source.len()),
            },
            BrokenBoundaryMode::TruncatedFinal => match grapheme_index {
                0 => Some(0),
                _ => Some("é".len()),
            },
            BrokenBoundaryMode::UnderreportedCount => Some(0),
        }
    }

    fn grapheme_count(&self, _source: &str) -> usize {
        match self.mode {
            BrokenBoundaryMode::UnderreportedCount => 0,
            _ => 2,
        }
    }
}

#[test]
fn provider_anchor_failures_precede_caller_bounds() {
    let source = "éx";
    assert_eq!(
        resolve_grapheme_cursor_position(
            &BrokenBoundaryProvider {
                mode: BrokenBoundaryMode::UnderreportedCount,
            },
            source,
            1,
        ),
        Err(GraphemeCursorError::BoundaryAnchorMismatch {
            expected: source.len(),
            grapheme_index: 0,
            observed: 0,
        }),
    );
    assert_eq!(
        resolve_grapheme_cursor_position(
            &BrokenBoundaryProvider {
                mode: BrokenBoundaryMode::ShiftedFirst,
            },
            source,
            3,
        ),
        Err(GraphemeCursorError::BoundaryAnchorMismatch {
            expected: 0,
            grapheme_index: 0,
            observed: "é".len(),
        }),
    );
    assert_eq!(
        resolve_grapheme_cursor_position(
            &BrokenBoundaryProvider {
                mode: BrokenBoundaryMode::TruncatedFinal,
            },
            source,
            3,
        ),
        Err(GraphemeCursorError::BoundaryAnchorMismatch {
            expected: source.len(),
            grapheme_index: 2,
            observed: "é".len(),
        }),
    );
}

#[test]
fn invalid_and_missing_internal_boundaries_are_typed_failures() {
    let source = "éx";
    assert_eq!(
        resolve_grapheme_cursor_position(
            &BrokenBoundaryProvider {
                mode: BrokenBoundaryMode::InvalidUtf8,
            },
            source,
            1,
        ),
        Err(GraphemeCursorError::InvalidBoundary {
            byte_offset: 1,
            grapheme_index: 1,
        }),
    );
    assert_eq!(
        resolve_grapheme_cursor_position(
            &BrokenBoundaryProvider {
                mode: BrokenBoundaryMode::MissingInternal,
            },
            source,
            1,
        ),
        Err(GraphemeCursorError::BoundaryUnavailable { grapheme_index: 1 }),
    );
}

#[test]
fn valid_provider_reports_caller_positions_after_the_end_as_out_of_bounds() {
    let provider = UnicodeGraphemeSegmentation;
    assert_eq!(
        resolve_grapheme_cursor_position(&provider, "aé", 3),
        Err(GraphemeCursorError::CursorOutOfBounds {
            grapheme_count: 2,
            grapheme_index: 3,
        }),
    );
}

#[test]
fn bilingual_inventory_exposes_only_start_and_end_cursor_positions() {
    let provider = UnicodeGraphemeSegmentation;
    for required in REQUIRED_TEXT_GRAPHEMES {
        assert_eq!(
            resolve_grapheme_cursor_position(&provider, required.grapheme, 0),
            Ok(GraphemeCursorPosition {
                byte_offset: 0,
                grapheme_index: 0,
            }),
            "start boundary for {:?}",
            required.grapheme,
        );
        assert_eq!(
            resolve_grapheme_cursor_position(&provider, required.grapheme, 1),
            Ok(GraphemeCursorPosition {
                byte_offset: required.grapheme.len(),
                grapheme_index: 1,
            }),
            "end boundary for {:?}",
            required.grapheme,
        );
        assert_eq!(
            resolve_grapheme_cursor_position(&provider, required.grapheme, 2),
            Err(GraphemeCursorError::CursorOutOfBounds {
                grapheme_count: 1,
                grapheme_index: 2,
            }),
            "post-end rejection for {:?}",
            required.grapheme,
        );
    }
}

#[test]
fn selection_preserves_forward_reverse_and_collapsed_endpoint_order() {
    let provider = UnicodeGraphemeSegmentation;
    let source = "Áe\u{301}👩‍🔬Z";
    let position =
        |grapheme_index: usize, byte_offset: usize| GraphemeCursorPosition {
            byte_offset,
            grapheme_index,
        };
    assert_eq!(
        resolve_grapheme_cursor_selection(&provider, source, 1, 3),
        Ok(GraphemeCursorSelection {
            anchor: position(1, "Á".len()),
            focus: position(3, "Áe\u{301}👩‍🔬".len()),
        }),
    );
    assert_eq!(
        resolve_grapheme_cursor_selection(&provider, source, 3, 1),
        Ok(GraphemeCursorSelection {
            anchor: position(3, "Áe\u{301}👩‍🔬".len()),
            focus: position(1, "Á".len()),
        }),
    );
    assert_eq!(
        resolve_grapheme_cursor_selection(&provider, source, 2, 2),
        Ok(GraphemeCursorSelection {
            anchor: position(2, "Áe\u{301}".len()),
            focus: position(2, "Áe\u{301}".len()),
        }),
    );
}

#[test]
fn collapsed_internal_selection_resolves_boundary_once() {
    let provider = ChangingInternalBoundaryProvider {
        internal_calls: Cell::new(0),
    };
    let position = GraphemeCursorPosition {
        byte_offset: "é".len(),
        grapheme_index: 1,
    };
    assert_eq!(
        resolve_grapheme_cursor_selection(&provider, "éx", 1, 1),
        Ok(GraphemeCursorSelection {
            anchor: position,
            focus: position,
        }),
    );
    assert_eq!(provider.internal_calls.get(), 1);
}

#[test]
fn selection_reports_anchor_then_focus_bounds_without_reordering() {
    let provider = UnicodeGraphemeSegmentation;
    assert_eq!(
        resolve_grapheme_cursor_selection(&provider, "aé", 3, 1),
        Err(GraphemeCursorError::CursorOutOfBounds {
            grapheme_count: 2,
            grapheme_index: 3,
        }),
    );
    assert_eq!(
        resolve_grapheme_cursor_selection(&provider, "aé", 1, 3),
        Err(GraphemeCursorError::CursorOutOfBounds {
            grapheme_count: 2,
            grapheme_index: 3,
        }),
    );
}

#[test]
fn empty_text_accepts_only_collapsed_zero_selection() {
    let provider = UnicodeGraphemeSegmentation;
    let zero = GraphemeCursorPosition {
        byte_offset: 0,
        grapheme_index: 0,
    };
    assert_eq!(
        resolve_grapheme_cursor_selection(&provider, "", 0, 0),
        Ok(GraphemeCursorSelection {
            anchor: zero,
            focus: zero,
        }),
    );
    assert_eq!(
        resolve_grapheme_cursor_selection(&provider, "", 0, 1),
        Err(GraphemeCursorError::CursorOutOfBounds {
            grapheme_count: 0,
            grapheme_index: 1,
        }),
    );
}

#[test]
fn bilingual_inventory_resolves_both_full_grapheme_selection_directions() {
    let provider = UnicodeGraphemeSegmentation;
    for required in REQUIRED_TEXT_GRAPHEMES {
        let start = GraphemeCursorPosition {
            byte_offset: 0,
            grapheme_index: 0,
        };
        let end = GraphemeCursorPosition {
            byte_offset: required.grapheme.len(),
            grapheme_index: 1,
        };
        assert_eq!(
            resolve_grapheme_cursor_selection(
                &provider,
                required.grapheme,
                0,
                1,
            ),
            Ok(GraphemeCursorSelection {
                anchor: start,
                focus: end,
            }),
            "forward selection for {:?}",
            required.grapheme,
        );
        assert_eq!(
            resolve_grapheme_cursor_selection(
                &provider,
                required.grapheme,
                1,
                0,
            ),
            Ok(GraphemeCursorSelection {
                anchor: end,
                focus: start,
            }),
            "reverse selection for {:?}",
            required.grapheme,
        );
    }
}

#[test]
fn signed_steps_resolve_exact_grapheme_boundaries_without_clamping() {
    let provider = UnicodeGraphemeSegmentation;
    let source = "Áe\u{301}👩‍🔬Z";
    assert_eq!(
        resolve_grapheme_cursor_step(&provider, source, 2, 1),
        Ok(GraphemeCursorPosition {
            byte_offset: "Áe\u{301}👩‍🔬".len(),
            grapheme_index: 3,
        }),
    );
    assert_eq!(
        resolve_grapheme_cursor_step(&provider, source, 2, -2),
        Ok(GraphemeCursorPosition {
            byte_offset: 0,
            grapheme_index: 0,
        }),
    );
    assert_eq!(
        resolve_grapheme_cursor_step(&provider, source, 4, 1),
        Err(GraphemeCursorError::CursorStepOutOfBounds {
            grapheme_count: 4,
            origin_index: 4,
            step: 1,
        }),
    );
    assert_eq!(
        resolve_grapheme_cursor_step(&provider, source, 0, -1),
        Err(GraphemeCursorError::CursorStepOutOfBounds {
            grapheme_count: 4,
            origin_index: 0,
            step: -1,
        }),
    );
    assert_eq!(
        resolve_grapheme_cursor_step(&provider, source, 5, 0),
        Err(GraphemeCursorError::CursorOutOfBounds {
            grapheme_count: 4,
            grapheme_index: 5,
        }),
    );
}

#[test]
fn zero_step_reuses_the_resolved_origin_boundary() {
    let provider = ChangingInternalBoundaryProvider {
        internal_calls: Cell::new(0),
    };
    assert_eq!(
        resolve_grapheme_cursor_step(&provider, "éx", 1, 0),
        Ok(GraphemeCursorPosition {
            byte_offset: "é".len(),
            grapheme_index: 1,
        }),
    );
    assert_eq!(provider.internal_calls.get(), 1);
}

#[test]
fn every_required_bilingual_grapheme_steps_between_its_two_boundaries() {
    let provider = UnicodeGraphemeSegmentation;
    for required in REQUIRED_TEXT_GRAPHEMES {
        assert_eq!(
            resolve_grapheme_cursor_step(&provider, required.grapheme, 0, 1),
            Ok(GraphemeCursorPosition {
                byte_offset: required.grapheme.len(),
                grapheme_index: 1,
            }),
            "forward step for {:?}",
            required.grapheme,
        );
        assert_eq!(
            resolve_grapheme_cursor_step(&provider, required.grapheme, 1, -1),
            Ok(GraphemeCursorPosition {
                byte_offset: 0,
                grapheme_index: 0,
            }),
            "backward step for {:?}",
            required.grapheme,
        );
    }
}
