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
//   - Validation and exact UTF-8 resolution of caller-supplied Unicode
//     grapheme positions, one anchor/focus pair, and one signed grapheme step.
// - Must-Not:
//   - Normalize Unicode, choose movement direction or step size, clamp cursors,
//     choose language or keybindings, mutate notebooks, generate punctuation,
//     wrap text, or define transport.
// - Allows:
//   - Inputs: Exact UTF-8 text, grapheme-boundary indexes, one signed step, and
//     one outbound grapheme-boundary provider.
//   - Outputs: Exact positions or one caller-ordered anchor/focus pair.
//   - Side effects: None.
// - Split-When:
//   - Direction choice, clamping, selection extension, or visual affinity gains
//     independent policy.
// - Merge-When:
//   - Cursor-position validation becomes inseparable from semantic text edits.
// - Summary:
//   - Resolves positions, selections, and caller-supplied steps at boundaries.
// - Description:
//   - Checks one provider snapshot before accepting caller boundary positions.
// - Usage:
//   - Validate a transport-neutral text cursor before movement or selection.
// - Defaults:
//   - Both text endpoints are valid cursor positions, including empty text.
//

//! Exact Unicode grapheme cursor/selection resolution without UI movement
//! policy.

use atrament_unicode_grapheme_boundary_port::GraphemeBoundaryProvider;

/// One validated cursor position at a Unicode extended-grapheme boundary.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct GraphemeCursorPosition {
    /// Exact UTF-8 byte offset corresponding to `grapheme_index`.
    pub byte_offset: usize,
    /// Zero-based grapheme boundary index in the unchanged source.
    pub grapheme_index: usize,
}

/// One caller-ordered anchor/focus pair on the same unchanged source text.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct GraphemeCursorSelection {
    /// Anchor endpoint retained exactly as supplied by the caller.
    pub anchor: GraphemeCursorPosition,
    /// Focus endpoint retained exactly as supplied by the caller.
    pub focus: GraphemeCursorPosition,
}

/// Typed rejection of one grapheme cursor position.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GraphemeCursorError {
    /// Provider's first or final boundary does not match the source anchor.
    BoundaryAnchorMismatch {
        /// Required UTF-8 byte offset for this boundary index.
        expected: usize,
        /// Grapheme boundary index whose anchor is inconsistent.
        grapheme_index: usize,
        /// UTF-8 byte offset reported by the provider.
        observed: usize,
    },
    /// Provider omitted a boundary that its grapheme count advertised.
    BoundaryUnavailable {
        /// Grapheme boundary index whose byte offset was unavailable.
        grapheme_index: usize,
    },
    /// Caller requested a position after the final grapheme boundary.
    CursorOutOfBounds {
        /// Number of grapheme clusters in the source text.
        grapheme_count: usize,
        /// Requested grapheme boundary index.
        grapheme_index: usize,
    },
    /// Provider byte order contradicts caller grapheme selection index order.
    SelectionBoundaryOrderMismatch {
        /// UTF-8 byte offset resolved for the caller anchor.
        anchor_byte: usize,
        /// Grapheme boundary index supplied as the caller anchor.
        anchor_index: usize,
        /// UTF-8 byte offset resolved for the caller focus.
        focus_byte: usize,
        /// Grapheme boundary index supplied as the caller focus.
        focus_index: usize,
    },
    /// Provider boundary order contradicts the caller-supplied step direction.
    StepBoundaryOrderMismatch {
        /// UTF-8 byte offset for the admitted origin boundary.
        origin_byte: usize,
        /// Grapheme boundary index supplied as the origin.
        origin_index: usize,
        /// UTF-8 byte offset for the resolved target boundary.
        target_byte: usize,
        /// Grapheme boundary index reached by the signed step.
        target_index: usize,
    },
    /// Caller-supplied signed step would leave the source boundary range.
    CursorStepOutOfBounds {
        /// Number of grapheme clusters in the source text.
        grapheme_count: usize,
        /// Current grapheme boundary index supplied by the caller.
        origin_index: usize,
        /// Caller-supplied signed grapheme step.
        step: isize,
    },
    /// Provider returned a byte offset that is not a valid source boundary.
    InvalidBoundary {
        /// Invalid UTF-8 byte offset returned by the provider.
        byte_offset: usize,
        /// Grapheme boundary index that produced the invalid byte offset.
        grapheme_index: usize,
    },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct AnchoredBoundaries {
    final_byte: usize,
    first_byte: usize,
    total: usize,
}

/// Resolve one cursor boundary without choosing any cursor movement policy.
///
/// Provider anchor consistency is checked before caller bounds, so an
/// underreported grapheme count cannot be misreported as a caller error.
/// Internal positions are resolved exactly once. No Unicode normalization is
/// performed.
///
/// # Errors
///
/// Returns a typed provider-consistency error or the cursor-out-of-bounds
/// error when `grapheme_index` lies after the final source boundary.
pub fn resolve_grapheme_cursor_position(
    boundaries: &dyn GraphemeBoundaryProvider,
    source: &str,
    grapheme_index: usize,
) -> Result<GraphemeCursorPosition, GraphemeCursorError> {
    let anchors = provider_anchors(boundaries, source)?;
    resolve_position(boundaries, source, grapheme_index, anchors)
}

/// Resolve one caller-ordered selection without choosing extension policy.
///
/// Start/end provider anchors are validated once for the shared source. The
/// anchor endpoint is resolved before focus, and a collapsed selection reuses
/// the same resolved position instead of querying an internal boundary twice.
/// Caller anchor/focus order is retained even when focus precedes anchor. The
/// provider's resolved byte offsets must still preserve the same relative order
/// as those two grapheme boundary indexes.
///
/// # Errors
///
/// Returns the same typed cursor-position failures as
/// [`resolve_grapheme_cursor_position`], or
/// [`GraphemeCursorError::SelectionBoundaryOrderMismatch`] when two
/// individually
/// valid provider boundaries contradict their grapheme index order. A
/// caller-bound failure identifies its exact requested grapheme index.
pub fn resolve_grapheme_cursor_selection(
    boundaries: &dyn GraphemeBoundaryProvider,
    source: &str,
    anchor_index: usize,
    focus_index: usize,
) -> Result<GraphemeCursorSelection, GraphemeCursorError> {
    let anchors = provider_anchors(boundaries, source)?;
    let anchor = resolve_position(boundaries, source, anchor_index, anchors)?;
    let focus = if focus_index == anchor_index {
        anchor
    } else {
        resolve_position(boundaries, source, focus_index, anchors)?
    };
    let ordered = if focus_index > anchor_index {
        focus.byte_offset > anchor.byte_offset
    } else if focus_index < anchor_index {
        focus.byte_offset < anchor.byte_offset
    } else {
        true
    };
    if !ordered {
        return Err(GraphemeCursorError::SelectionBoundaryOrderMismatch {
            anchor_byte: anchor.byte_offset,
            anchor_index,
            focus_byte: focus.byte_offset,
            focus_index,
        });
    }
    Ok(GraphemeCursorSelection { anchor, focus })
}

/// Resolve one caller-supplied signed grapheme step without clamping.
///
/// The origin must already be an admitted grapheme boundary. The signed step is
/// applied only after provider anchors and the origin index are validated. A
/// step that would leave the source range rejects instead of being clamped. A
/// nonzero step also requires the resolved UTF-8 boundary to advance in the
/// same direction as the grapheme index.
///
/// # Errors
///
/// Returns the same provider/origin failures as
/// [`resolve_grapheme_cursor_position`],
/// [`GraphemeCursorError::CursorStepOutOfBounds`] when the signed step would
/// leave the inclusive boundary range `0..=grapheme_count`, or
/// [`GraphemeCursorError::StepBoundaryOrderMismatch`] when a provider returns
/// individually valid boundaries in an order inconsistent with the step.
pub fn resolve_grapheme_cursor_step(
    boundaries: &dyn GraphemeBoundaryProvider,
    source: &str,
    origin_index: usize,
    step: isize,
) -> Result<GraphemeCursorPosition, GraphemeCursorError> {
    let anchors = provider_anchors(boundaries, source)?;
    let origin = resolve_position(boundaries, source, origin_index, anchors)?;
    let out_of_bounds = || GraphemeCursorError::CursorStepOutOfBounds {
        grapheme_count: anchors.total,
        origin_index,
        step,
    };
    let target_index = if step >= 0 {
        let distance = step.cast_unsigned();
        let target = origin_index
            .checked_add(distance)
            .ok_or_else(out_of_bounds)?;
        if target > anchors.total {
            return Err(out_of_bounds());
        }
        target
    } else {
        origin_index
            .checked_sub(step.unsigned_abs())
            .ok_or_else(out_of_bounds)?
    };
    if target_index == origin_index {
        return Ok(origin);
    }
    let target =
        resolve_position(boundaries, source, target_index, anchors)?;
    let ordered = if step > 0 {
        target.byte_offset > origin.byte_offset
    } else {
        target.byte_offset < origin.byte_offset
    };
    if !ordered {
        return Err(GraphemeCursorError::StepBoundaryOrderMismatch {
            origin_byte: origin.byte_offset,
            origin_index,
            target_byte: target.byte_offset,
            target_index,
        });
    }
    Ok(target)
}

fn resolve_position(
    boundaries: &dyn GraphemeBoundaryProvider,
    source: &str,
    grapheme_index: usize,
    anchors: AnchoredBoundaries,
) -> Result<GraphemeCursorPosition, GraphemeCursorError> {
    if grapheme_index > anchors.total {
        return Err(GraphemeCursorError::CursorOutOfBounds {
            grapheme_count: anchors.total,
            grapheme_index,
        });
    }
    let byte_offset = if grapheme_index == 0 {
        anchors.first_byte
    } else if grapheme_index == anchors.total {
        anchors.final_byte
    } else {
        provider_boundary(boundaries, source, grapheme_index)?
    };
    Ok(GraphemeCursorPosition {
        byte_offset,
        grapheme_index,
    })
}

fn provider_anchors(
    boundaries: &dyn GraphemeBoundaryProvider,
    source: &str,
) -> Result<AnchoredBoundaries, GraphemeCursorError> {
    let total = boundaries.grapheme_count(source);
    let first_byte = provider_boundary(boundaries, source, 0)?;
    if first_byte != 0 {
        return Err(GraphemeCursorError::BoundaryAnchorMismatch {
            expected: 0,
            grapheme_index: 0,
            observed: first_byte,
        });
    }
    let final_byte = if total == 0 {
        first_byte
    } else {
        provider_boundary(boundaries, source, total)?
    };
    if final_byte != source.len() {
        return Err(GraphemeCursorError::BoundaryAnchorMismatch {
            expected: source.len(),
            grapheme_index: total,
            observed: final_byte,
        });
    }
    Ok(AnchoredBoundaries {
        final_byte,
        first_byte,
        total,
    })
}

fn provider_boundary(
    boundaries: &dyn GraphemeBoundaryProvider,
    source: &str,
    grapheme_index: usize,
) -> Result<usize, GraphemeCursorError> {
    let byte_offset = boundaries
        .byte_offset(source, grapheme_index)
        .ok_or(GraphemeCursorError::BoundaryUnavailable { grapheme_index })?;
    validate_boundary(source, grapheme_index, byte_offset)?;
    Ok(byte_offset)
}

const fn validate_boundary(
    source: &str,
    grapheme_index: usize,
    byte_offset: usize,
) -> Result<(), GraphemeCursorError> {
    if byte_offset > source.len() || !source.is_char_boundary(byte_offset) {
        return Err(GraphemeCursorError::InvalidBoundary {
            byte_offset,
            grapheme_index,
        });
    }
    Ok(())
}
