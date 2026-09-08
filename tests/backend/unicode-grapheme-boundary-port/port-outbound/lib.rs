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
//   - Compile-time evidence for the grapheme boundary outbound contract.
// - Must-Not:
//   - Select a segmentation engine or assert Unicode behavior.
// - Allows:
//   - Inputs: One deterministic local trait implementation.
//   - Outputs: Assertions that both contract methods remain callable.
//   - Side effects: None.
// - Split-When:
//   - The outbound contract gains independently testable protocol behavior.
// - Merge-When:
//   - Adapter fixtures completely subsume port compilation evidence.
// - Summary:
//   - Pins the minimal grapheme-boundary provider surface.
// - Description:
//   - Prevents application code from depending on a concrete Unicode crate.
// - Usage:
//   - Compile directly against the outbound port.
// - Defaults:
//   - No segmentation semantics are implied by the port itself.
//
use atrament_unicode_grapheme_boundary_port::GraphemeBoundaryProvider;

struct FixtureProvider;

impl GraphemeBoundaryProvider for FixtureProvider {
    fn byte_offset(
        &self,
        source: &str,
        grapheme_index: usize,
    ) -> Option<usize> {
        (grapheme_index <= source.len()).then_some(grapheme_index)
    }

    fn grapheme_count(&self, source: &str) -> usize {
        source.len()
    }
}

#[test]
fn outbound_port_exposes_count_and_boundary_queries() {
    let provider = FixtureProvider;
    assert_eq!(provider.grapheme_count("abc"), 3);
    assert_eq!(provider.byte_offset("abc", 2), Some(2));
}
