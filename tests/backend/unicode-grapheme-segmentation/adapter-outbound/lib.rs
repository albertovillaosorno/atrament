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
//   - Regression evidence for pinned Unicode extended-grapheme segmentation
//     across constructed cases and the frozen bilingual text inventory.
// - Must-Not:
//   - Normalize text, mutate notebooks, or infer language-specific boundaries.
// - Allows:
//   - Inputs: Deterministic combining-mark, emoji-ZWJ, and required bilingual
//     text graphemes.
//   - Outputs: Exact grapheme counts and UTF-8 boundary offsets.
//   - Side effects: None.
// - Split-When:
//   - Another segmentation implementation requires parity fixtures.
// - Merge-When:
//   - Standard-library segmentation replaces this adapter.
// - Summary:
//   - Proves the external adapter reports extended grapheme boundaries.
// - Description:
//   - Covers decomposed accents, emoji ZWJ clusters, end boundaries, and every
//     required English/Spanish visible-text grapheme.
// - Usage:
//   - Compile directly against the segmentation adapter and outbound port.
// - Defaults:
//   - Source normalization remains unchanged.
//
use atrament_english_spanish_grapheme_inventory::REQUIRED_TEXT_GRAPHEMES;
use atrament_unicode_grapheme_boundary_port::GraphemeBoundaryProvider;
use atrament_unicode_grapheme_segmentation::UnicodeGraphemeSegmentation;

#[test]
fn adapter_reports_extended_grapheme_boundaries_without_normalization() {
    let provider = UnicodeGraphemeSegmentation;
    let source = "e\u{301}👩‍🔬Z";
    assert_eq!(provider.grapheme_count(source), 3);
    assert_eq!(provider.byte_offset(source, 0), Some(0));
    assert_eq!(provider.byte_offset(source, 1), Some("e\u{301}".len()));
    assert_eq!(provider.byte_offset(source, 2), Some("e\u{301}👩‍🔬".len()));
    assert_eq!(provider.byte_offset(source, 3), Some(source.len()));
    assert_eq!(provider.byte_offset(source, 4), None);
}

#[test]
fn adapter_boundaries_match_constructed_extended_grapheme_corpus() {
    let provider = UnicodeGraphemeSegmentation;
    let cases: &[&[&str]] = &[
        &["e\u{301}", "ñ", "Z"],
        &["👩‍🔬", "👍🏽", "!"],
        &["🇲🇽", "🇺🇸"],
        &["\r\n", "A"],
    ];
    for clusters in cases {
        let source = clusters.concat();
        assert_eq!(provider.grapheme_count(&source), clusters.len());
        let mut expected_offset = 0usize;
        assert_eq!(provider.byte_offset(&source, 0), Some(expected_offset));
        for (index, cluster) in clusters.iter().enumerate() {
            expected_offset = expected_offset
                .checked_add(cluster.len())
                .expect("small fixture boundary must be representable");
            assert_eq!(
                provider.byte_offset(&source, index + 1),
                Some(expected_offset),
                "constructed cluster boundary {index}",
            );
        }
        assert_eq!(expected_offset, source.len());
        assert_eq!(provider.byte_offset(&source, clusters.len() + 1), None);
    }
}

#[test]
fn adapter_treats_every_required_bilingual_text_entry_as_one_grapheme() {
    let provider = UnicodeGraphemeSegmentation;
    let mut cases = 0_usize;
    for (index, required) in REQUIRED_TEXT_GRAPHEMES.iter().enumerate() {
        let source = required.grapheme;
        assert_eq!(
            provider.grapheme_count(source),
            1,
            "required inventory index {index}",
        );
        assert_eq!(provider.byte_offset(source, 0), Some(0));
        assert_eq!(provider.byte_offset(source, 1), Some(source.len()));
        assert_eq!(provider.byte_offset(source, 2), None);
        cases = cases.saturating_add(1);
    }
    assert_eq!(cases, 114);
}
