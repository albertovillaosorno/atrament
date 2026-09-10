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
//   - Exact range replacement over externally supplied Unicode grapheme
//     boundaries.
// - Must-Not:
//   - Normalize Unicode, generate punctuation, choose language, wrap text,
//     mutate notebooks, or define cursor or transport semantics.
// - Allows:
//   - Inputs: Exact UTF-8 text, grapheme ranges, replacement text, and one
//     outbound grapheme-boundary provider.
//   - Outputs: Exact replacement text or typed grapheme-range rejection.
//   - Side effects: Process-local string allocation only.
// - Split-When:
//   - Language-aware normalization or punctuation gains independent authority.
// - Merge-When:
//   - Grapheme editing becomes inseparable from one semantic text owner.
// - Summary:
//   - Edits authored Unicode at grapheme boundaries without normalization.
// - Description:
//   - Uses injected extended-grapheme boundaries while preserving exact bytes.
// - Usage:
//   - Transform one current text value before existing semantic replacement.
// - Defaults:
//   - Zero-length ranges are insertion points, including at the text end.
//

//! Exact Unicode grapheme-range editing without normalization.

use atrament_unicode_grapheme_boundary_port::GraphemeBoundaryProvider;

/// One half-open range measured in Unicode extended grapheme clusters.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct GraphemeRange {
    /// Number of grapheme clusters replaced from `start`.
    pub count: usize,
    /// Zero-based grapheme-cluster start index.
    pub start: usize,
}

/// Typed rejection of one grapheme range.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GraphemeRangeError {
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
    /// `start + count` overflowed the addressable index type.
    EndIndexOverflow {
        /// Requested grapheme count.
        count: usize,
        /// Requested grapheme start index.
        start: usize,
    },
    /// Range end lies after the final grapheme boundary.
    EndOutOfBounds {
        /// Requested exclusive grapheme end index.
        end: usize,
        /// Number of grapheme clusters in the source text.
        grapheme_count: usize,
    },
    /// Provider returned a byte offset that is not a valid source boundary.
    InvalidBoundary {
        /// Invalid UTF-8 byte offset returned by the provider.
        byte_offset: usize,
        /// Grapheme boundary index that produced the invalid byte offset.
        grapheme_index: usize,
    },
    /// Provider returned equal boundaries for a non-empty grapheme range.
    NonAdvancingBoundaries {
        /// Exclusive range-end byte offset returned by the provider.
        end_byte: usize,
        /// Range-start byte offset returned by the provider.
        start_byte: usize,
    },
    /// Provider returned an end boundary before the start boundary.
    ReversedBoundaries {
        /// Exclusive range-end byte offset returned by the provider.
        end_byte: usize,
        /// Range-start byte offset returned by the provider.
        start_byte: usize,
    },
    /// Range start lies after the final grapheme boundary.
    StartOutOfBounds {
        /// Number of grapheme clusters in the source text.
        grapheme_count: usize,
        /// Requested grapheme start index.
        start: usize,
    },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct AnchoredBoundaries {
    final_byte: usize,
    first_byte: usize,
    total: usize,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct ResolvedRange {
    end: usize,
    end_byte: usize,
    start_byte: usize,
}

/// Replace one grapheme range while preserving all unaffected authored bytes.
///
/// No Unicode normalization is performed. The replacement is copied exactly as
/// supplied, so NFC and NFD spellings remain distinct values.
///
/// # Errors
///
/// Returns a typed error when the range start/end is outside the source or when
/// the exclusive end index cannot be represented.
pub fn replace_grapheme_range(
    boundaries: &dyn GraphemeBoundaryProvider,
    source: &str,
    range: GraphemeRange,
    replacement: &str,
) -> Result<String, GraphemeRangeError> {
    let anchors = provider_anchors(boundaries, source)?;
    let resolved = resolve_range(boundaries, source, range, anchors)?;
    let prefix = source.get(..resolved.start_byte).ok_or(
        GraphemeRangeError::InvalidBoundary {
            byte_offset: resolved.start_byte,
            grapheme_index: range.start,
        },
    )?;
    let suffix = source.get(resolved.end_byte..).ok_or(
        GraphemeRangeError::InvalidBoundary {
            byte_offset: resolved.end_byte,
            grapheme_index: resolved.end,
        },
    )?;
    let mut output = String::new();
    output.push_str(prefix);
    output.push_str(replacement);
    output.push_str(suffix);
    Ok(output)
}

fn provider_anchors(
    boundaries: &dyn GraphemeBoundaryProvider,
    source: &str,
) -> Result<AnchoredBoundaries, GraphemeRangeError> {
    let total = boundaries.grapheme_count(source);
    let first_byte = provider_boundary(boundaries, source, 0)?;
    if first_byte != 0 {
        return Err(GraphemeRangeError::BoundaryAnchorMismatch {
            expected: 0,
            grapheme_index: 0,
            observed: first_byte,
        });
    }
    let final_byte = provider_boundary(boundaries, source, total)?;
    if final_byte != source.len() {
        return Err(GraphemeRangeError::BoundaryAnchorMismatch {
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

fn resolve_range(
    boundaries: &dyn GraphemeBoundaryProvider,
    source: &str,
    range: GraphemeRange,
    anchors: AnchoredBoundaries,
) -> Result<ResolvedRange, GraphemeRangeError> {
    if range.start > anchors.total {
        return Err(GraphemeRangeError::StartOutOfBounds {
            grapheme_count: anchors.total,
            start: range.start,
        });
    }
    let end = range
        .start
        .checked_add(range.count)
        .ok_or(GraphemeRangeError::EndIndexOverflow {
            count: range.count,
            start: range.start,
        })?;
    if end > anchors.total {
        return Err(GraphemeRangeError::EndOutOfBounds {
            end,
            grapheme_count: anchors.total,
        });
    }
    let start_byte = if range.start == 0 {
        anchors.first_byte
    } else if range.start == anchors.total {
        anchors.final_byte
    } else {
        provider_boundary(boundaries, source, range.start)?
    };
    let end_byte = if end == range.start {
        start_byte
    } else if end == anchors.total {
        anchors.final_byte
    } else {
        provider_boundary(boundaries, source, end)?
    };
    if end_byte < start_byte {
        return Err(GraphemeRangeError::ReversedBoundaries {
            end_byte,
            start_byte,
        });
    }
    if range.count > 0 && end_byte == start_byte {
        return Err(GraphemeRangeError::NonAdvancingBoundaries {
            end_byte,
            start_byte,
        });
    }
    Ok(ResolvedRange {
        end,
        end_byte,
        start_byte,
    })
}

fn provider_boundary(
    boundaries: &dyn GraphemeBoundaryProvider,
    source: &str,
    grapheme_index: usize,
) -> Result<usize, GraphemeRangeError> {
    let byte_offset = boundaries.byte_offset(source, grapheme_index).ok_or(
        GraphemeRangeError::BoundaryUnavailable { grapheme_index },
    )?;
    validate_boundary(source, grapheme_index, byte_offset)?;
    Ok(byte_offset)
}

const fn validate_boundary(
    source: &str,
    grapheme_index: usize,
    byte_offset: usize,
) -> Result<(), GraphemeRangeError> {
    if byte_offset > source.len() || !source.is_char_boundary(byte_offset) {
        return Err(GraphemeRangeError::InvalidBoundary {
            byte_offset,
            grapheme_index,
        });
    }
    Ok(())
}
