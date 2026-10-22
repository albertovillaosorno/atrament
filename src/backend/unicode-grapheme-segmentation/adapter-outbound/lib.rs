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
//   - Unicode extended-grapheme boundary queries through the pinned
//     `unicode-segmentation` implementation.
// - Must-Not:
//   - Normalize text, mutate semantic state, choose language, or expose
//     third-party iterator types across the port.
// - Allows:
//   - Inputs: Exact authored UTF-8 text and grapheme indexes.
//   - Outputs: Grapheme counts and exact UTF-8 byte-boundary offsets.
//   - Side effects: None.
// - Split-When:
//   - Another segmentation engine requires independent behavior/versioning.
// - Merge-When:
//   - Standard-library Unicode segmentation replaces the external adapter.
// - Summary:
//   - Adapts pinned Unicode segmentation to Atrament's outbound boundary port.
// - Description:
//   - Keeps the external Unicode package outside application/domain sources.
// - Usage:
//   - Inject into grapheme-aware application operations at composition time.
// - Defaults:
//   - Uses Unicode extended grapheme clusters with no locale tailoring.
//

//! `unicode-segmentation` adapter for Atrament grapheme-boundary queries.

use atrament_unicode_grapheme_boundary_port::GraphemeBoundaryProvider;
use unicode_segmentation::UnicodeSegmentation as _;

/// Stateless Unicode extended-grapheme boundary adapter.
#[derive(Clone, Copy, Debug, Default)]
pub struct UnicodeGraphemeSegmentation;

impl GraphemeBoundaryProvider for UnicodeGraphemeSegmentation {
    fn byte_offset(
        &self,
        source: &str,
        grapheme_index: usize,
    ) -> Option<usize> {
        let mut observed = 0usize;
        for (byte_offset, _) in source.grapheme_indices(true) {
            if observed == grapheme_index {
                return Some(byte_offset);
            }
            observed = observed.checked_add(1)?;
        }
        (observed == grapheme_index).then_some(source.len())
    }

    fn grapheme_count(&self, source: &str) -> usize {
        source.graphemes(true).count()
    }
}
