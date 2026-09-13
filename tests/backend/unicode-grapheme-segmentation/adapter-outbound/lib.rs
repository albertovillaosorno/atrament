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
//     across constructed, generated, and frozen bilingual text cases.
// - Must-Not:
//   - Normalize text, mutate notebooks, or infer language-specific boundaries.
// - Allows:
//   - Inputs: Deterministic mixed-Unicode sources and required bilingual text
//     graphemes.
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

fn next_corpus_value(seed: &mut u64) -> u64 {
    *seed = seed
        .wrapping_mul(6_364_136_223_846_793_005)
        .wrapping_add(1_442_695_040_888_963_407);
    *seed
}

#[test]
fn generated_mixed_unicode_corpus_preserves_boundary_port_invariants() {
    const CASES: usize = 4_096;
    const FRAGMENTS: [&str; 16] = [
        "A",
        "ñ",
        "n\u{301}",
        "\u{301}",
        "👩‍🔬",
        "👍🏽",
        "🇲",
        "🇽",
        "\r",
        "\n",
        "\u{200d}",
        "❤️",
        "¿",
        "…",
        "—",
        "क",
    ];
    let provider = UnicodeGraphemeSegmentation;
    let mut seed = 0x6a09_e667_f3bc_c909_u64;
    let mut seen_fragment_counts = [false; 9];
    let mut seen_fragments = [false; FRAGMENTS.len()];
    let mut saw_empty = false;
    let mut saw_single = false;
    let mut saw_multiple = false;

    for case in 0..CASES {
        let fragment_count = usize::try_from(
            (next_corpus_value(&mut seed) >> 56) % 9,
        )
        .expect("generated fragment count fits usize");
        seen_fragment_counts[fragment_count] = true;
        let mut source = String::new();
        for _ in 0..fragment_count {
            let fragment_index = usize::try_from(
                (next_corpus_value(&mut seed) >> 48)
                    % u64::try_from(FRAGMENTS.len())
                        .expect("fragment table length fits u64"),
            )
            .expect("generated fragment index fits usize");
            seen_fragments[fragment_index] = true;
            source.push_str(FRAGMENTS[fragment_index]);
        }

        let count = provider.grapheme_count(&source);
        assert!(
            count <= source.len(),
            "case {case}: grapheme count exceeds UTF-8 bytes",
        );
        if source.is_empty() {
            saw_empty = true;
            assert_eq!(count, 0, "case {case}: empty source count");
        } else {
            assert!(count > 0, "case {case}: nonempty source count");
            saw_single |= count == 1;
            saw_multiple |= count > 1;
        }

        let mut previous = None;
        for index in 0..=count {
            let first = provider.byte_offset(&source, index);
            let second = provider.byte_offset(&source, index);
            assert_eq!(
                first, second,
                "case {case}: boundary {index} is nondeterministic",
            );
            let offset = first.expect("in-range boundary must exist");
            assert!(
                offset <= source.len(),
                "case {case}: boundary {index} exceeds source",
            );
            assert!(
                source.is_char_boundary(offset),
                "case {case}: boundary {index} is not UTF-8 aligned",
            );
            if let Some(prior) = previous {
                assert!(
                    prior < offset,
                    "case {case}: boundaries must advance",
                );
            } else {
                assert_eq!(offset, 0, "case {case}: first boundary anchor");
            }
            previous = Some(offset);
        }
        assert_eq!(
            previous,
            Some(source.len()),
            "case {case}: final boundary anchor",
        );
        let past_end = count.checked_add(1).expect("small generated count");
        assert_eq!(
            provider.byte_offset(&source, past_end),
            None,
            "case {case}: boundary past end must reject",
        );
        assert_eq!(
            provider.byte_offset(&source, usize::MAX),
            None,
            "case {case}: maximum index must reject",
        );
    }

    assert!(seen_fragment_counts.into_iter().all(|seen| seen));
    assert!(seen_fragments.into_iter().all(|seen| seen));
    assert!(saw_empty);
    assert!(saw_single);
    assert!(saw_multiple);
}
