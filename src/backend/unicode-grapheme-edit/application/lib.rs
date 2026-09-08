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
    /// Range start lies after the final grapheme boundary.
    StartOutOfBounds {
        /// Number of grapheme clusters in the source text.
        grapheme_count: usize,
        /// Requested grapheme start index.
        start: usize,
    },
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
    let total = boundaries.grapheme_count(source);
    if range.start > total {
        return Err(GraphemeRangeError::StartOutOfBounds {
            grapheme_count: total,
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
    if end > total {
        return Err(GraphemeRangeError::EndOutOfBounds {
            end,
            grapheme_count: total,
        });
    }
    let start_byte = boundaries.byte_offset(source, range.start).ok_or(
        GraphemeRangeError::StartOutOfBounds {
            grapheme_count: total,
            start: range.start,
        },
    )?;
    let end_byte = boundaries.byte_offset(source, end).ok_or(
        GraphemeRangeError::EndOutOfBounds {
            end,
            grapheme_count: total,
        },
    )?;
    let prefix = source.get(..start_byte).ok_or(
        GraphemeRangeError::StartOutOfBounds {
            grapheme_count: total,
            start: range.start,
        },
    )?;
    let suffix = source.get(end_byte..).ok_or(
        GraphemeRangeError::EndOutOfBounds {
            end,
            grapheme_count: total,
        },
    )?;
    let mut output = String::new();
    output.push_str(prefix);
    output.push_str(replacement);
    output.push_str(suffix);
    Ok(output)
}
