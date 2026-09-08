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
//   - Outbound Unicode extended-grapheme boundary query contract.
// - Must-Not:
//   - Normalize text, mutate semantic state, choose language, or expose a
//     segmentation library type.
// - Allows:
//   - Inputs: Exact authored UTF-8 text and grapheme indexes.
//   - Outputs: Grapheme counts and exact UTF-8 byte-boundary offsets.
//   - Side effects: None required by the contract.
// - Split-When:
//   - Grapheme measurement requires independently versioned behavior.
// - Merge-When:
//   - Unicode segmentation no longer requires an external implementation.
// - Summary:
//   - Abstracts Unicode grapheme boundaries from text-edit application logic.
// - Description:
//   - Keeps external segmentation packages behind an outbound port.
// - Usage:
//   - Inject one implementation into grapheme-aware text editing.
// - Defaults:
//   - No normalization or locale-sensitive segmentation is implied.
//

//! Outbound contract for Unicode extended-grapheme boundary discovery.

/// Read-only Unicode extended-grapheme boundary provider.
pub trait GraphemeBoundaryProvider {
    /// Return the UTF-8 byte offset for one grapheme boundary index.
    fn byte_offset(&self, source: &str, grapheme_index: usize) -> Option<usize>;

    /// Return the number of Unicode extended grapheme clusters in `source`.
    fn grapheme_count(&self, source: &str) -> usize;
}
