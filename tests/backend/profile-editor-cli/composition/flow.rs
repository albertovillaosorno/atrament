// Copyright:
//   - Copyright © 2026 Alberto Villa Osorno.
// SPDX-License-Identifier:
//   - LicenseRef-Proprietary
// Confidential:
//   - true
// License-File:
//   - LICENSE
//
// Boundary-Contract:
// - Owns:
//   - Native measured flow Unicode, whitespace and row-pitch unit tests.
// - Must-Not:
//   - Implement layout or introduce second Unicode segmentation semantics.
// - Allows:
//   - Inputs: Existing Rust flow engine and exact synthetic text samples.
//   - Outputs: Assertions over source admission and measured physical rows.
//   - Side effects: None.
// - Split-When:
//   - Persisted notebook flow gains an independent public test contract.
// - Merge-When:
//   - The native flow compiler replaces CLI-only text layout.
// - Summary:
//   - Tests Unicode admission and syntax-sensitive row-height measurement.
// - Description:
//   - Exact NFC/NFD graphemes, tabs, structural spaces and Setext boundaries.
// - Usage:
//   - cargo test --offline -p atrament_profile_editor_cli
// - Defaults:
//   - Deterministic unit assertions, no files or subprocesses.
//

//! Native source-flow regressions included by the production CLI module.

use super::{FlowFormat, FlowSource};

#[test]
fn typed_flow_regions_reject_zero_area_before_ink_validation()
-> Result<(), String> {
    use atrament_notebook_profile::NotebookProfile;

    let paper = NotebookProfile::default();
    let printable = paper.printable_rect_um()?;
    let region = super::FlowRegion {
        left: printable.0,
        top: printable.1,
        width: 14_000,
        height: 14_000,
    };
    let _: (u32, u32, u32, u32) = region.bounds_within(printable)?;
    region.validate_ink(printable, &[])?;
    let source = FlowSource::new(FlowFormat::Markdown, "a")?;
    for invalid in [
        super::FlowRegion { width: 0, ..region },
        super::FlowRegion { height: 0, ..region },
    ] {
        let boundary = invalid
            .bounds_within(printable)
            .expect_err("zero area cannot be a printable rectangle");
        assert_eq!(boundary, "flow region width and height must be positive");
        let ink = invalid
            .validate_ink(printable, &[])
            .expect_err("empty ink cannot validate an invalid rectangle");
        assert_eq!(ink, boundary);
        let layout = super::layout_flow_with_region(
            &source,
            &paper,
            None,
            super::HeadingDecoration::default(),
            Some(invalid),
        )
        .expect_err("zero area refuses before measuring content");
        assert_eq!(layout, boundary);
    }
    Ok(())
}

#[test]
fn typed_flow_source_must_retain_bounded_canonical_admission()
-> Result<(), String> {
    use atrament_notebook_profile::NotebookProfile;

    let paper = NotebookProfile::default();
    let source = FlowSource::new(
        FlowFormat::Markdown, "a\u{2028}ó",
    )?;
    assert_eq!(source.text, "a  \nó");
    let literal_bom = FlowSource::new(
        FlowFormat::Plain, "\u{feff}\u{feff}a",
    )?;
    assert_eq!(literal_bom.text, "\u{feff}a");
    let _literal = super::layout_flow_with_region(
        &literal_bom, &paper, None,
        super::HeadingDecoration::default(), None,
    )?;
    let _accepted = super::layout_flow_with_region(
        &source, &paper, None, super::HeadingDecoration::default(), None,
    )?;

    for (text, expected) in [
        ("a".repeat(4_097), "flow text needs 1..4096 UTF-8 bytes"),
        ("a\u{0007}".into(), "flow text needs 1..4096 UTF-8 bytes"),
        ("a\r\nó".into(), "flow source needs canonical normalized text"),
        ("a\tó".into(), "flow source needs canonical normalized text"),
        ("a\u{2028}ó".into(), "flow source needs canonical normalized text"),
    ] {
        let forged = FlowSource { format: FlowFormat::Markdown, text };
        let error = super::layout_flow_with_region(
            &forged, &paper, None,
            super::HeadingDecoration::default(), None,
        ).expect_err("unadmitted typed flow must not reach physical layout");
        assert!(error.contains(expected), "unexpected refusal: {error}");
    }
    Ok(())
}

#[test]
fn ordered_hanging_indent_refuses_glyph_wider_than_continuation()
-> Result<(), String> {
    use atrament_handwriting_statistics::{
        Dimension, DimensionStatistics, HandwritingStatisticsProfile,
        ScalarStatistics,
    };
    use atrament_notebook_profile::NotebookProfile;
    use atrament_unicode_grapheme_segmentation::UnicodeGraphemeSegmentation;

    let region = super::FlowRegion {
        left: 30_000,
        top: 40_000,
        width: 28_000,
        height: 40_000,
    };
    let source = FlowSource::new(FlowFormat::Markdown, "1. aó")
        .expect("bounded numbered item with exact Unicode ink");
    let nominal = super::layout_flow_with_region(
        &source,
        &NotebookProfile::default(),
        None,
        super::HeadingDecoration::default(),
        Some(region),
    )
    .expect("nominal 4-mm glyphs fit the 6-mm continuation");
    assert_eq!(
        nominal
            .0
            .iter()
            .map(|run| run.text.as_str())
            .collect::<Vec<_>>(),
        ["1. a", "ó"]
    );
    assert_eq!(nominal.0[1].origin_um.0, nominal.0[0].origin_um.0 + 12_000);

    let wide = ScalarStatistics {
        minimum: 9_000,
        maximum: 9_000,
        mean: 9_000,
        median: 9_000,
        standard_deviation: 0,
    };
    let stats = HandwritingStatisticsProfile::baseline(3)
        .with_glyph(
            "ó",
            DimensionStatistics {
                dimension: Dimension::Width,
                statistics: wide,
            },
            &UnicodeGraphemeSegmentation,
        )
        .expect("valid wide authored accent glyph");
    for text in ["1. aó", "1. a ó"] {
        let narrow = FlowSource::new(FlowFormat::Markdown, text)?;
        let refused = super::layout_flow_with_region(
            &narrow,
            &NotebookProfile::default(),
            Some(&stats),
            super::HeadingDecoration::default(),
            Some(region),
        )
        .expect_err("the wide glyph cannot fit a narrowed continuation");
        assert!(
            refused.contains("horizontal printable region"),
            "explicit refusal rather than overflow for {text:?}: {refused}"
        );

        let fitting = super::FlowRegion {
            width: region.width + 1_000,
            ..region
        };
        let accepted = super::layout_flow_with_region(
            &narrow,
            &NotebookProfile::default(),
            Some(&stats),
            super::HeadingDecoration::default(),
            Some(fitting),
        )
        .expect("one extra millimetre admits the full glyph contour");
        assert_eq!(accepted.0.len(), 2);
        assert_eq!(accepted.0[0].text, "1. a");
        assert_eq!(accepted.0[1].text, "ó");
        let (_, contour) =
            super::GlyphOverlay::flow_advance_um("ó", Some(&stats))?;
        let right_edge = accepted.0[1].origin_um.0 + contour + 800;
        assert!(
            right_edge <= fitting.left + fitting.width - 1_500,
            "wide exact Unicode contour must fit the new width"
        );
    }
    Ok(())
}

#[test]
fn ordered_soft_continuation_keeps_statistical_hang_across_pages()
-> Result<(), String> {
    use atrament_handwriting_statistics::{
        Dimension, DimensionStatistics, HandwritingStatisticsProfile,
        ScalarStatistics,
    };
    use atrament_notebook_profile::NotebookProfile;
    use atrament_unicode_grapheme_segmentation::UnicodeGraphemeSegmentation;

    let mut stats = HandwritingStatisticsProfile::baseline(53);
    for (glyph, width) in [
        ("1", 5_000), ("2", 5_000), (")", 4_000), ("ó", 9_000),
    ] {
        let value = ScalarStatistics {
            minimum: width, maximum: width, mean: width,
            median: width, standard_deviation: 0,
        };
        stats = stats.with_glyph(glyph, DimensionStatistics {
            dimension: Dimension::Width, statistics: value,
        }, &UnicodeGraphemeSegmentation)
            .map_err(|error| format!("{error:?}"))?;
    }
    let region = super::FlowRegion {
        left: 30_000, top: 40_000, width: 42_000, height: 14_000,
    };
    let render = |text: &str, region| -> Result<_, String> {
        let source = FlowSource::new(FlowFormat::Markdown, text)?;
        super::layout_flow_with_region(
            &source, &NotebookProfile::default(), Some(&stats),
            super::HeadingDecoration::default(), Some(region),
        )
    };
    let joined = render("12) a\n  ó", region)?;
    let inline = render("12) a ó", region)?;
    assert_eq!(joined.2, 2);
    assert_eq!(joined.0.len(), 2);
    assert_eq!(joined.0[0].text, "12) a");
    assert_eq!(joined.0[1].text, "ó");
    assert_eq!(joined.0[0].origin_um, (38_500, 42_500));
    assert_eq!(joined.0[1].origin_um, (57_550, 42_500));
    assert_eq!(joined.0.iter().map(|row| row.page)
        .collect::<Vec<_>>(), [1, 2]);
    assert!(joined.1.is_empty());
    assert_eq!(joined.0.iter().map(|row| row.text.as_str())
        .collect::<String>(), "12) aó");
    assert_eq!(joined.0.iter().map(|row| (&row.text, row.origin_um, row.page))
        .collect::<Vec<_>>(), inline.0.iter()
        .map(|row| (&row.text, row.origin_um, row.page))
        .collect::<Vec<_>>());
    assert_eq!(joined.1.len(), inline.1.len());
    assert_eq!(joined.2, inline.2);

    let too_narrow = super::FlowRegion { width: 36_000, ..region };
    let error = render("12) a\n  ó", too_narrow)
        .expect_err("the continuation glyph must fit after the hang");
    assert!(error.contains("horizontal printable region"), "{error}");
    Ok(())
}

#[test]
fn measured_words_keep_nfc_nfd_and_joined_unicode_graphemes() {
    let word = "aóo\u{301}👩\u{200d}🏫";
    let (glyphs, width, occupied) = super::measured_word(word, None)
        .expect("valid authored Unicode word");
    let segments = glyphs.iter().map(|(text, _, _)| *text)
        .collect::<Vec<_>>();
    assert_eq!(segments, ["a", "ó", "o\u{301}", "👩\u{200d}🏫"]);
    assert_eq!(width, 16_000);
    assert_eq!(occupied, 16_300);
    assert!(glyphs.iter().all(|(_, advance, _)| *advance == 4_000));

    // Bounded input can still contain thousands of short graphemes;
    // measure each exact span once rather than rescanning its prefix.
    let long = "o\u{301}".repeat(1_024);
    let (many, width, occupied_long) = super::measured_word(&long, None)
        .expect("bounded decomposed source");
    assert_eq!(many.len(), 1_024);
    assert_eq!(width, 4_096_000);
    assert_eq!(occupied_long, 4_096_300);
    assert!(many.iter().all(|(text, _, _)| *text == "o\u{301}"));
}

#[test]
fn authored_unicode_spaces_are_not_implicit_blank_source() {
    for format in [FlowFormat::Plain, FlowFormat::Markdown] {
        for authored in ["\u{00a0}", "\u{2009}", "\u{202f}"] {
            let source = FlowSource::new(format, authored)
                .expect("Unicode space can have exact authored ink");
            assert_eq!(source.text, authored);
        }
        for structural in [" ", "\n", "\t", "\u{2028}",
            "\u{2029}", " \t\n"] {
            assert!(FlowSource::new(format, structural).is_err(),
                "ASCII spacing and source separators have no ink");
        }
    }
}

#[test]
fn max_tabbed_source_expands_to_exact_byte_limit() {
    let source = "a\t".repeat(1_024);
    let admitted = FlowSource::new(FlowFormat::Plain, &source)
        .expect("expanded text at the 4096-byte limit");
    assert_eq!(admitted.text, "a   ".repeat(1_024));
    let refused = FlowSource::new(FlowFormat::Plain,
        &"a\t".repeat(1_025));
    assert!(refused.is_err(), "over-limit expansion must refuse");
}

#[test]
fn grapheme_tab_stops_survive_mixed_unicode_source_breaks() {
    use atrament_handwriting_statistics::{
        Dimension, DimensionStatistics, HandwritingStatisticsProfile,
        ScalarStatistics,
    };
    use atrament_notebook_profile::NotebookProfile;
    use atrament_unicode_grapheme_segmentation::UnicodeGraphemeSegmentation;

    let original = concat!(
        "\u{feff}\u{e9}\tn\u{303}\u{2028}",
        "\u{1f469}\u{200d}\u{1f3eb}\t=\u{2029}a",
    );
    let canonical = "\u{e9}   n\u{303}  \n\u{1f469}\u{200d}\u{1f3eb}   =\n\na";
    let tall = ScalarStatistics {
        maximum: 25_000, mean: 25_000, median: 25_000,
        minimum: 25_000, standard_deviation: 0,
    };
    let statistics = HandwritingStatisticsProfile::baseline(1)
        .with_glyph("=", DimensionStatistics {
            dimension: Dimension::Height, statistics: tall,
        }, &UnicodeGraphemeSegmentation)
        .expect("valid height for literal equals glyph");
    for format in [FlowFormat::Markdown, FlowFormat::Plain] {
        let pasted = FlowSource::new(format, original)
            .expect("source with tabs, NFC, NFD and Unicode separators");
        let equivalent = FlowSource::new(format, canonical)
            .expect("canonical source with the same exact graphemes");
        assert_eq!(pasted.text, canonical);
        assert_eq!(super::measured_pitch(&pasted, Some(&statistics))
            .expect("literal equals determines row height"), 29_000);
        let render = |source: &FlowSource| super::layout_flow(
            source, &NotebookProfile::default(), Some(&statistics),
            super::HeadingDecoration::default(),
        ).expect("admitted source fits on the selected physical page");
        let (rows, ornaments, pages) = render(&pasted);
        let (expected, expected_ornaments, expected_pages) =
            render(&equivalent);
        assert_eq!(rows.iter().map(|row| (
            row.page, row.origin_um, row.text.as_str(),
        )).collect::<Vec<_>>(), expected.iter().map(|row| (
            row.page, row.origin_um, row.text.as_str(),
        )).collect::<Vec<_>>());
        assert_eq!(ornaments, expected_ornaments);
        assert_eq!(pages, expected_pages);
        assert_eq!(pages, 1);
        assert!(ornaments.is_empty(),
            "a literal equals following an emoji is not heading syntax");
        let expected_ink = [
            "\u{e9}   n\u{303}",
            "\u{1f469}\u{200d}\u{1f3eb}   =",
            "a",
        ];
        assert_eq!(rows.iter().map(|row| row.text.as_str())
            .collect::<Vec<_>>(), expected_ink);
        assert_eq!(rows[1].origin_um.1 - rows[0].origin_um.1,
            29_000, "line separator must end the first physical row");
        let after_paragraph = if format == FlowFormat::Markdown {
            33_000
        } else {
            58_000
        };
        assert_eq!(rows[2].origin_um.1 - rows[1].origin_um.1,
            after_paragraph, "paragraph separator keeps source semantics");
    }
}

#[test]
fn mixed_transport_breaks_cannot_promote_literal_indented_setext() {
    use atrament_handwriting_statistics::{
        Dimension, DimensionStatistics, HandwritingStatisticsProfile,
        ScalarStatistics,
    };
    use atrament_notebook_profile::NotebookProfile;
    use atrament_unicode_grapheme_segmentation::UnicodeGraphemeSegmentation;

    let transported = "\u{feff}a\r\n\tb\u{2028}===\r\nc";
    let canonical = "a\n    b  \n===\nc";
    for format in [FlowFormat::Markdown, FlowFormat::Plain] {
        let source = FlowSource::new(format, transported)
            .expect("valid BOM, CRLF, tab and Unicode source breaks");
        assert_eq!(source.text, canonical);
        for refused in ["\u{feff}", "a\r", "a\rb", "a\0b",
            "a\u{001f}b", "a\u{007f}b", "a\u{0085}b",
            "a\u{009f}b"] {
            assert!(FlowSource::new(format, refused).is_err(),
                "unsupported source control: {refused:?}");
        }
    }

    let tall = ScalarStatistics {
        maximum: 25_000, mean: 25_000, median: 25_000,
        minimum: 25_000, standard_deviation: 0,
    };
    let statistics = HandwritingStatisticsProfile::baseline(1)
        .with_glyph("=", DimensionStatistics {
            dimension: Dimension::Height, statistics: tall,
        }, &UnicodeGraphemeSegmentation)
        .expect("literal Setext-like marks have authored tall ink");
    let render = |text: &str| {
        let source = FlowSource::new(FlowFormat::Markdown, text)
            .expect("admitted Markdown source");
        super::layout_flow(
            &source, &NotebookProfile::default(), Some(&statistics),
            super::HeadingDecoration::default(),
        ).expect("all visible marks render as authored text")
    };
    let actual = render(transported);
    let expected = render(canonical);
    assert_eq!(actual.0.iter().map(|row| (
        row.page, row.origin_um, row.text.as_str(),
    )).collect::<Vec<_>>(), expected.0.iter().map(|row| (
        row.page, row.origin_um, row.text.as_str(),
    )).collect::<Vec<_>>());
    assert_eq!(actual.0.iter().map(|row| row.text.as_str())
        .collect::<Vec<_>>(), ["a", "    b", "=== c"]);
    assert!(actual.1.is_empty(),
        "a hard break after a literal four-space row cannot create a heading");
    assert_eq!(actual.0[2].origin_um.1 - actual.0[1].origin_um.1,
        29_000, "the equal signs remain authored height-bearing ink");
}

#[test]
fn literal_indent_before_setext_mark_keeps_its_glyph_pitch() {
    use atrament_handwriting_statistics::{
        Dimension, DimensionStatistics, HandwritingStatisticsProfile,
        ScalarStatistics,
    };
    use atrament_notebook_profile::NotebookProfile;
    use atrament_unicode_grapheme_segmentation::UnicodeGraphemeSegmentation;

    let tall = ScalarStatistics {
        maximum: 25_000, mean: 25_000, median: 25_000,
        minimum: 25_000, standard_deviation: 0,
    };
    let stats = HandwritingStatisticsProfile::baseline(1)
        .with_glyph("=", DimensionStatistics {
            dimension: Dimension::Height, statistics: tall,
        }, &UnicodeGraphemeSegmentation)
        .expect("valid tall literal glyph");
    let literal = FlowSource::new(
        FlowFormat::Markdown, "    a\n=\na",
    ).expect("four-space literal row before an equal mark");
    let (runs, _, _) = super::layout_flow(
        &literal, &NotebookProfile::default(), Some(&stats),
        super::HeadingDecoration::default(),
    ).expect("literal equal glyph is visible ink");
    assert_eq!(runs.iter().map(|run| run.text.as_str())
        .collect::<Vec<_>>(), ["    a", "= a"]);
    assert_eq!(runs[1].origin_um.1 - runs[0].origin_um.1,
        29_000, "visible equal glyph must determine row pitch");

    let heading = FlowSource::new(
        FlowFormat::Markdown, "a\n=\na",
    ).expect("actual Setext heading");
    let (rows, _, _) = super::layout_flow(
        &heading, &NotebookProfile::default(), Some(&stats),
        super::HeadingDecoration::default(),
    ).expect("heading underline is syntax, not glyph ink");
    assert_eq!(rows[1].origin_um.1 - rows[0].origin_um.1,
        9_000, "actual Setext syntax must not inflate row pitch");

    for continuation in ["  ", "   "] {
        let continued = FlowSource::new(
            FlowFormat::Markdown,
            &format!("- a\n{continuation}a\n=\na"),
        ).expect("indented continuation belongs to the list item");
        let (list, _, _) = super::layout_flow(
            &continued, &NotebookProfile::default(), Some(&stats),
            super::HeadingDecoration::default(),
        ).expect("equal mark remains literal after the list");
        assert_eq!(list.iter().map(|run| run.text.as_str())
            .collect::<Vec<_>>(), ["a a", "= a"]);
        assert_eq!(list[1].origin_um.1 - list[0].origin_um.1,
            29_000, "list continuation must not arm Setext syntax");
    }
}

#[test]
fn quote_continuation_cannot_hide_literal_setext_marker_height() {
    use atrament_handwriting_statistics::{
        Dimension, DimensionStatistics, HandwritingStatisticsProfile,
        ScalarStatistics,
    };
    use atrament_notebook_profile::NotebookProfile;
    use atrament_unicode_grapheme_segmentation::UnicodeGraphemeSegmentation;

    let tall = ScalarStatistics {
        maximum: 25_000, mean: 25_000, median: 25_000,
        minimum: 25_000, standard_deviation: 0,
    };
    let statistics = HandwritingStatisticsProfile::baseline(1)
        .with_glyph("=", DimensionStatistics {
            dimension: Dimension::Height, statistics: tall,
        }, &UnicodeGraphemeSegmentation)
        .expect("valid literal equal glyph height");
    let render = |text: &str| {
        let source = FlowSource::new(FlowFormat::Markdown, text)
            .expect("valid quoted Markdown");
        super::layout_flow(
            &source, &NotebookProfile::default(), Some(&statistics),
            super::HeadingDecoration::default(),
        ).expect("quoted and literal rows render")
    };
    let canonical = render("> a a\n= a");
    assert_eq!(canonical.0.iter().map(|row| row.text.as_str())
        .collect::<Vec<_>>(), ["a a", "= a"]);
    for continued in ["> a\n  a\n=\na", "> a\n   a\n=\na"] {
        let actual = render(continued);
        assert_eq!(actual.0.iter().map(|row| (
            row.page, row.origin_um, row.text.as_str(),
        )).collect::<Vec<_>>(), canonical.0.iter().map(|row| (
            row.page, row.origin_um, row.text.as_str(),
        )).collect::<Vec<_>>(), "source: {continued:?}");
        assert_eq!(actual.1, canonical.1,
            "only the original quote row receives a bar");
        assert_eq!(actual.0[1].origin_um.1 - actual.0[0].origin_um.1,
            29_000, "literal equal height determines the common row pitch");
    }
}

#[test]
fn indented_blockquote_lines_keep_the_quote_rule_and_exact_ink() {
    use atrament_notebook_profile::NotebookProfile;

    let paper = NotebookProfile::default();
    let (left, top, _, _) = paper.printable_rect_um()
        .expect("default writing rectangle");
    let region = super::FlowRegion {
        left, top, width: 24_000, height: 36_000,
    };
    let render = |text: &str| {
        let source = FlowSource::new(FlowFormat::Markdown, text)
            .expect("bounded Markdown quote source");
        super::layout_flow_with_region(
            &source, &paper, None, super::HeadingDecoration::default(),
            Some(region),
        ).expect("quoted paragraph fits the selected rectangle")
    };
    let canonical = render("> a ó a a");
    for continued in [
        "> a\n  ó a a", "> a\n   ó a a",
        "> a\n  ó a\n   a",
    ] {
        let actual = render(continued);
        assert_eq!(actual.0.iter().map(|row| (
            row.page, row.origin_um, row.text.as_str(),
        )).collect::<Vec<_>>(), canonical.0.iter().map(|row| (
            row.page, row.origin_um, row.text.as_str(),
        )).collect::<Vec<_>>());
        assert_eq!(actual.1, canonical.1,
            "every wrapped row must retain the quote bar");
        assert_eq!(actual.2, canonical.2);
    }
    assert!(canonical.0.len() >= 2,
        "narrow region must wrap authored quote ink");
    assert_eq!(canonical.1.len(), canonical.0.len(),
        "quote bar accompanies every physical row");
}

#[test]
fn indented_blockquote_ends_at_explicit_breaks_and_new_blocks() {
    use atrament_notebook_profile::NotebookProfile;

    let paper = NotebookProfile::default();
    let render = |text: &str| {
        let source = FlowSource::new(FlowFormat::Markdown, text)
            .expect("bounded Markdown quote source");
        super::layout_flow(
            &source, &paper, None, super::HeadingDecoration::default(),
        ).expect("quoted and ordinary source are both physical ink")
    };
    for source in [
        "> a\nó", "> a  \nó", "> a\\\nó", "> a\n\nó",
        "> a  \n  ó", "> a\\\n  ó",
        "> a\n# ó", "> a\n- ó", "> a\n~~~txt\nó\n~~~",
        "> a\n    ó", "> a\n===",
    ] {
        let (rows, marks, _) = render(source);
        assert_eq!(rows.len(), 2, "source: {source:?}");
        assert_eq!(marks.iter().filter(|mark| {
            mark.start_um.0 == mark.end_um.0
        }).count(), 1, "only quoted rows have vertical bars");
        assert_eq!(rows[0].text, "a");
        assert_ne!(rows[1].text, "a ó",
            "structural or explicit break must end quote continuation");
    }
}

#[test]
fn trailing_blank_rows_do_not_create_invisible_pages() {
    use atrament_notebook_profile::NotebookProfile;

    let paper = NotebookProfile::default();
    let heading = super::HeadingDecoration::default();
    for (format, trailing) in [
        (FlowFormat::Plain, format!("a\n{}", "\n".repeat(130))),
        (FlowFormat::Markdown,
            format!("a\n~~~txt\n{}~~~", "\n".repeat(130))),
    ] {
        let clean = FlowSource::new(format, "a")
            .expect("short visible source");
        let (clean_runs, clean_marks, clean_pages) = super::layout_flow(
            &clean, &paper, None, heading,
        ).expect("single visible line");
        let source = FlowSource::new(format, &trailing)
            .expect("bounded source with trailing blank lines");
        let (runs, marks, pages) = super::layout_flow(
            &source, &paper, None, heading,
        ).expect("trailing blank rows cannot demand empty pages");
        assert_eq!(clean_pages, 1);
        assert_eq!(pages, clean_pages, "no ink-free final pages");
        assert_eq!(runs.len(), clean_runs.len());
        assert_eq!(runs[0].page, clean_runs[0].page);
        assert_eq!(runs[0].origin_um, clean_runs[0].origin_um);
        assert_eq!(runs[0].text, clean_runs[0].text);
        assert_eq!(marks, clean_marks);
    }
}

#[test]
fn interior_blank_rows_remain_measured_before_printable_ink() {
    use atrament_notebook_profile::NotebookProfile;

    let paper = NotebookProfile::default();
    for (format, source) in [
        (FlowFormat::Plain, "a\n\nb"),
        (FlowFormat::Markdown, "a\n~~~txt\n\nb\n~~~"),
    ] {
        let source = FlowSource::new(format, source)
            .expect("visible rows with an interior blank row");
        let (runs, marks, pages) = super::layout_flow(
            &source, &paper, None, super::HeadingDecoration::default(),
        ).expect("interior blank spacing must remain physical");
        assert_eq!(pages, 1);
        assert_eq!(runs.iter().map(|run| run.text.as_str())
            .collect::<Vec<_>>(), ["a", "b"]);
        assert_eq!(runs[1].origin_um.1 - runs[0].origin_um.1,
            18_000, "blank row reserves an actual writing row");
        assert!(marks.is_empty());
    }

    let rule = FlowSource::new(
        FlowFormat::Markdown, "a\n~~~txt\n\n~~~\n---",
    ).expect("terminal rule after blank code row");
    let (runs, marks, pages) = super::layout_flow(
        &rule, &paper, None, super::HeadingDecoration::default(),
    ).expect("terminal rule counts as physical ink");
    assert_eq!(pages, 1);
    assert_eq!(runs.len(), 1);
    assert_eq!(marks.len(), 1);
    assert!(marks[0].start_um.1 > runs[0].origin_um.1 + 9_000);

    let oversized = FlowSource::new(
        FlowFormat::Plain, &format!("{}a", "\n".repeat(130)),
    ).expect("bounded source that needs 130 interior blank rows");
    assert!(super::layout_flow(
        &oversized, &paper, None, super::HeadingDecoration::default(),
    ).is_err(), "interior spacing still obeys the measured row budget");
}

#[test]
fn heading_does_not_move_for_empty_trailing_fenced_rows() {
    use atrament_notebook_profile::NotebookProfile;

    let paper = NotebookProfile::default();
    let (left, top, _, _) = paper.printable_rect_um()
        .expect("default physical writing rectangle");
    let region = super::FlowRegion {
        left, top, width: 100_000, height: 23_000,
    };
    let expected = FlowSource::new(FlowFormat::Markdown, "a\n# a")
        .expect("visible heading at last physical row");
    let with_blanks = FlowSource::new(FlowFormat::Markdown,
        &format!("a\n# a\n~~~txt\n{}~~~", "\n".repeat(55)))
        .expect("trailing fenced whitespace source");
    let layout = |source: &FlowSource| super::layout_flow_with_region(
        source, &paper, None, super::HeadingDecoration::default(),
        Some(region),
    ).expect("the same two visible rows must fit on one page");
    let (clean_runs, clean_marks, clean_pages) = layout(&expected);
    let (runs, marks, pages) = layout(&with_blanks);
    assert_eq!(clean_pages, 1);
    assert_eq!(pages, clean_pages);
    assert_eq!(runs.iter().map(|run| (run.page, run.origin_um))
        .collect::<Vec<_>>(), clean_runs.iter()
        .map(|run| (run.page, run.origin_um)).collect::<Vec<_>>());
    assert_eq!(marks, clean_marks);
}

#[test]
fn heading_keeps_next_ink_after_interior_fenced_blank_rows() {
    use atrament_notebook_profile::NotebookProfile;

    let paper = NotebookProfile::default();
    let (left, top, _, _) = paper.printable_rect_um()
        .expect("default physical writing rectangle");
    // Three 9-mm physical rows fit per sheet. Starting a heading on
    // the second row cannot fit both a fenced blank row and next ink.
    let region = super::FlowRegion {
        left, top, width: 100_000, height: 36_000,
    };
    for (source, distance) in [
        ("a\n# a\n~~~txt\n\nó\n~~~", 18_000),
        ("a\n# a\n~~~txt\n\n~~~\nó", 18_000),
        ("a\n# a\n\n~~~txt\n\nó\n~~~", 22_000),
        ("a  \na\n===\n~~~txt\n\nó\n~~~", 18_000),
    ] {
        let source = FlowSource::new(FlowFormat::Markdown, source)
            .expect("valid heading and fenced blank source");
        let (runs, _, pages) = super::layout_flow_with_region(
            &source, &paper, None, super::HeadingDecoration::default(),
            Some(region),
        ).expect("three physical rows should fit on the second page");
        assert_eq!(pages, 2);
        assert_eq!(runs.iter().map(|run| run.page).collect::<Vec<_>>(),
            [1, 2, 2], "heading must stay with later physical ink");
        assert_eq!(runs[2].origin_um.1 - runs[1].origin_um.1,
            distance, "fenced rows and any paragraph gap remain physical");
    }
}

#[test]
fn adjacent_heading_levels_stay_with_their_first_body_row() {
    use atrament_notebook_profile::NotebookProfile;

    let paper = NotebookProfile::default();
    let (left, top, _, _) = paper.printable_rect_um()
        .expect("default physical writing rectangle");
    // Four 9-mm rows fit per page. Two prelude rows leave enough room
    // for either heading alone, but not both titles and their body.
    let region = super::FlowRegion {
        left, top, width: 100_000, height: 45_000,
    };
    for (source, expected) in [
        ("a  \na\n# a\n## ó\na", vec![1, 1, 2, 2, 2]),
        ("a  \na\n# a\n## ó\n\na", vec![1, 1, 2, 2, 2]),
        ("a  \na\n# a\n## ó\n### a\na", vec![1, 1, 2, 2, 2, 2]),
        ("a  \na\n# a\n## ó\n~~~txt\n\na\n~~~", vec![1, 1, 2, 2, 2]),
    ] {
        let input = FlowSource::new(FlowFormat::Markdown, source)
            .expect("adjacent ATX headings and body text");
        let (runs, _, pages) = super::layout_flow_with_region(
            &input, &paper, None, super::HeadingDecoration::default(),
            Some(region),
        ).expect("heading chain fits with its body on a new page");
        assert_eq!(pages, 2);
        assert_eq!(runs.iter().map(|run| run.page).collect::<Vec<_>>(),
            expected, "adjacent headings must not orphan earlier title");
        if source.contains("## ó\n\na") {
            assert_eq!(runs[4].origin_um.1 - runs[3].origin_um.1,
                13_000, "blank Markdown paragraph retains its 4-mm gap");
        }
    }
}

#[test]
fn heading_chain_never_reserves_nonprinting_trailing_code_rows() {
    use atrament_notebook_profile::NotebookProfile;

    let paper = NotebookProfile::default();
    let (left, top, _, _) = paper.printable_rect_um()
        .expect("default physical writing rectangle");
    let region = super::FlowRegion {
        left, top, width: 100_000, height: 45_000,
    };
    let render = |text: &str| {
        let source = FlowSource::new(FlowFormat::Markdown, text)
            .expect("valid ending headings and fenced blank lines");
        super::layout_flow_with_region(
            &source, &paper, None, super::HeadingDecoration::default(),
            Some(region),
        ).expect("terminal blank code rows cannot require another page")
    };
    let baseline = render("a  \na\n# a\n## ó");
    let trailing = render("a  \na\n# a\n## ó\n~~~txt\n\n\n~~~");
    assert_eq!(baseline.2, trailing.2);
    assert_eq!(baseline.1, trailing.1);
    assert_eq!(baseline.0.iter().map(|run| (
        run.page, run.origin_um, run.text.as_str(),
    )).collect::<Vec<_>>(), trailing.0.iter().map(|run| (
        run.page, run.origin_um, run.text.as_str(),
    )).collect::<Vec<_>>());
    assert_eq!(trailing.2, 1);
    assert_eq!(trailing.0.iter().map(|run| run.page)
        .collect::<Vec<_>>(), [1, 1, 1, 1]);

    // When a region cannot fit a three-row group, normal flow remains
    // accepted and the later heading can move without losing ink.
    let too_short = super::FlowRegion { height: 23_000, ..region };
    let source = FlowSource::new(
        FlowFormat::Markdown, "# a\n## ó\na",
    ).expect("valid title and subtitle");
    let (runs, _, pages) = super::layout_flow_with_region(
        &source, &paper, None, super::HeadingDecoration::default(),
        Some(too_short),
    ).expect("unfittable group falls back to normal pagination");
    assert_eq!(pages, 2);
    assert_eq!(runs.iter().map(|run| run.page).collect::<Vec<_>>(),
        [1, 2, 2]);
}

#[test]
fn heading_chain_keeps_setext_subtitles_with_their_first_body_row() {
    use atrament_notebook_profile::NotebookProfile;

    let paper = NotebookProfile::default();
    let (left, top, _, _) = paper.printable_rect_um()
        .expect("default physical writing rectangle");
    // Four rows per sheet: the two initial rows leave room for an
    // isolated heading, but not an adjacent Setext subtitle and body.
    let region = super::FlowRegion {
        left, top, width: 100_000, height: 45_000,
    };
    for (text, pages) in [
        ("a  \na\n# a\nó\n===\na", vec![1, 1, 2, 2, 2]),
        ("a  \na  \na\n===\nó\n---\na", vec![1, 1, 2, 2, 2]),
        ("a  \na\n# a\nó\n===\n## a\na",
            vec![1, 1, 2, 2, 2, 2]),
    ] {
        let source = FlowSource::new(FlowFormat::Markdown, text)
            .expect("valid adjacent Markdown heading chain");
        let (runs, _, count) = super::layout_flow_with_region(
            &source, &paper, None, super::HeadingDecoration::default(),
            Some(region),
        ).expect("heading chain fits together on the second sheet");
        assert_eq!(count, 2, "unexpected count for {text:?}: {runs:?}");
        assert_eq!(runs.iter().map(|run| run.page).collect::<Vec<_>>(),
            pages, "Setext subtitles should travel with adjacent titles");
    }
}

#[test]
fn setext_heading_guard_does_not_promote_hard_or_literal_rows() {
    use atrament_notebook_profile::NotebookProfile;

    let paper = NotebookProfile::default();
    let (left, top, _, _) = paper.printable_rect_um()
        .expect("default physical writing rectangle");
    let region = super::FlowRegion {
        left, top, width: 100_000, height: 45_000,
    };
    for literal in ["ó  ", "ó\\", "    ó"] {
        let text = format!("a  \na\n# a\n{literal}\n===\na");
        let source = FlowSource::new(FlowFormat::Markdown, &text)
            .expect("source has one title and subsequent literal ink");
        let (runs, ornaments, pages) = super::layout_flow_with_region(
            &source, &paper, None, super::HeadingDecoration::default(),
            Some(region),
        ).expect("hard breaks and indentation remain literal");
        assert_eq!(pages, 2);
        assert_eq!(runs.iter().map(|run| run.page).collect::<Vec<_>>(),
            [1, 1, 1, 1, 2], "non-Setext line stays next to its title");
        assert_eq!(ornaments.len(), 1, "only ATX has heading ink");
        assert_eq!(runs.last().expect("final row").text, "=== a");
    }

    let source = FlowSource::new(
        FlowFormat::Markdown, "a  \na\n# a\nó\n===",
    ).expect("headings without a body");
    let (runs, _, pages) = super::layout_flow_with_region(
        &source, &paper, None, super::HeadingDecoration::default(),
        Some(region),
    ).expect("terminal titles still fit four physical rows");
    assert_eq!(pages, 1);
    assert_eq!(runs.iter().map(|run| run.page).collect::<Vec<_>>(),
        [1, 1, 1, 1]);
}

#[test]
fn multiline_setext_titles_stay_in_the_adjacent_heading_chain() {
    use atrament_notebook_profile::NotebookProfile;

    let paper = NotebookProfile::default();
    let (left, top, _, _) = paper.printable_rect_um()
        .expect("default physical writing rectangle");
    let region = super::FlowRegion {
        left, top, width: 100_000, height: 45_000,
    };
    for (source, expected_text) in [
        ("a  \na\n# a\nó\na\n===\na",
            ["a", "a", "a", "ó a", "a"]),
        ("a  \na  \nó\na\n===\na\nó\n---\na",
            ["a", "a", "ó a", "a ó", "a"]),
    ] {
        let source = FlowSource::new(FlowFormat::Markdown, source)
            .expect("soft-joined multiline Setext subtitle");
        let (runs, _, pages) = super::layout_flow_with_region(
            &source, &paper, None, super::HeadingDecoration::default(),
            Some(region),
        ).expect("all three adjacent rows fit page two");
        assert_eq!(pages, 2);
        assert_eq!(runs.iter().map(|run| run.text.as_str())
            .collect::<Vec<_>>(), expected_text);
        assert_eq!(runs.iter().map(|run| run.page).collect::<Vec<_>>(),
            [1, 1, 2, 2, 2], "soft-joined Setext title must not orphan");
    }
}

#[test]
fn multiline_setext_guard_respects_soft_paragraph_boundaries() {
    use atrament_notebook_profile::NotebookProfile;

    let paper = NotebookProfile::default();
    let (left, top, _, _) = paper.printable_rect_um()
        .expect("default physical writing rectangle");
    let region = super::FlowRegion {
        left, top, width: 100_000, height: 45_000,
    };
    for (middle, expected_pages) in [
        ("  a  ", vec![1, 1, 1, 1, 2]),
        ("    a", vec![1, 1, 1, 1, 2, 2]),
    ] {
        let text = format!("a  \na\n# a\nó\n{middle}\n===\na");
        let source = FlowSource::new(FlowFormat::Markdown, &text)
            .expect("body continuation followed by non-Setext syntax");
        let (runs, marks, pages) = super::layout_flow_with_region(
            &source, &paper, None, super::HeadingDecoration::default(),
            Some(region),
        ).expect("ordinary rows keep their original pagination");
        assert_eq!(pages, 2);
        assert_eq!(runs.iter().map(|run| run.page).collect::<Vec<_>>(),
            expected_pages, "hard and literal rows separate title groups");
        assert_eq!(marks.len(), 1, "only the ATX title is decorated");
        assert_eq!(runs.last().expect("final text").text, "=== a");
    }

    let terminal = FlowSource::new(FlowFormat::Markdown,
        "a  \na\n# a\nó\na\n===")
        .expect("multiline heading at the end of the source");
    let (runs, _, pages) = super::layout_flow_with_region(
        &terminal, &paper, None, super::HeadingDecoration::default(),
        Some(region),
    ).expect("trailing heading doesn't create an invisible extra sheet");
    assert_eq!(pages, 1);
    assert_eq!(runs.last().expect("final heading").text, "ó a");
}

#[test]
fn multiline_setext_lookahead_stops_at_fenced_code() {
    use atrament_notebook_profile::NotebookProfile;

    let paper = NotebookProfile::default();
    let (left, top, _, _) = paper.printable_rect_um()
        .expect("default physical writing rectangle");
    let region = super::FlowRegion {
        left, top, width: 100_000, height: 45_000,
    };
    for opening in ["~~~txt", "```txt"] {
        let closing = &opening[..3];
        let source = FlowSource::new(FlowFormat::Markdown,
            &format!("a  \na\n# a\nó\n{opening}\n===\n{closing}\na"))
            .expect("code fence with a literal Setext-like row");
        let (runs, ornaments, pages) = super::layout_flow_with_region(
            &source, &paper, None, super::HeadingDecoration::default(),
            Some(region),
        ).expect("fenced code remains ordinary physical ink");
        assert_eq!(pages, 2);
        assert_eq!(runs.iter().map(|run| run.page).collect::<Vec<_>>(),
            [1, 1, 1, 1, 2, 2],
            "fence cannot turn preceding body into a Setext heading");
        assert_eq!(runs[4].text, "===");
        assert_eq!(ornaments.len(), 1, "only ATX is decorated");
    }
}

#[test]
fn blank_separated_heading_chain_keeps_first_body_on_one_page() {
    use atrament_notebook_profile::NotebookProfile;

    let paper = NotebookProfile::default();
    let (left, top, _, _) = paper.printable_rect_um()
        .expect("default physical writing rectangle");
    // One prelude row leaves room for the next heading alone, but not a
    // blank-separated subtitle and its body. A new page fits all three.
    let region = super::FlowRegion {
        left, top, width: 100_000, height: 41_000,
    };
    for text in [
        "a  \n# a\n\n## ó\na",
        "a  \n# a\n\nó\n===\na",
        "a  \n# a\n\n\n## ó\na",
    ] {
        let source = FlowSource::new(FlowFormat::Markdown, text)
            .expect("heading and blank-separated subtitle");
        let (runs, _, pages) = super::layout_flow_with_region(
            &source, &paper, None, super::HeadingDecoration::default(),
            Some(region),
        ).expect("heading group fits on the next physical sheet");
        assert_eq!(pages, 2);
        assert_eq!(runs.iter().map(|run| run.page).collect::<Vec<_>>(),
            [1, 2, 2, 2], "first title must not orphan from subtitle");
    }
}

#[test]
fn separate_blank_gaps_between_three_titles_are_measured_once_each() {
    use atrament_notebook_profile::NotebookProfile;

    let paper = NotebookProfile::default();
    let (left, top, _, _) = paper.printable_rect_um()
        .expect("default physical writing rectangle");
    // Four visible group rows and two 4-mm gaps fit a 50-mm region,
    // but cannot fit after one prelude row on the first page.
    let region = super::FlowRegion {
        left, top, width: 100_000, height: 50_000,
    };
    let source = FlowSource::new(FlowFormat::Markdown,
        "a  \n# a\n\n## ó\n\n### a\na")
        .expect("three blank-separated headings");
    let (runs, _, pages) = super::layout_flow_with_region(
        &source, &paper, None, super::HeadingDecoration::default(),
        Some(region),
    ).expect("whole heading group fits on second page");
    assert_eq!(pages, 2);
    assert_eq!(runs.iter().map(|run| run.page).collect::<Vec<_>>(),
        [1, 2, 2, 2, 2]);
    assert_eq!(runs[2].origin_um.1 - runs[1].origin_um.1, 13_000);
    assert_eq!(runs[3].origin_um.1 - runs[2].origin_um.1, 13_000);

    let too_short = super::FlowRegion { height: 36_000, ..region };
    let (fallback, _, count) = super::layout_flow_with_region(
        &source, &paper, None, super::HeadingDecoration::default(),
        Some(too_short),
    ).expect("unfittable group still paginates without refusing");
    assert!(count > 1);
    assert_eq!(fallback.len(), 5);

    let terminal = FlowSource::new(FlowFormat::Markdown,
        "a  \n# a\n\n## ó\n\n\n")
        .expect("trailing blank heading source");
    let (_, _, count) = super::layout_flow_with_region(
        &terminal, &paper, None, super::HeadingDecoration::default(),
        Some(region),
    ).expect("trailing gaps cannot create an ink-free sheet");
    assert_eq!(count, 1);
}

#[test]
fn blank_fenced_rows_keep_following_title_chain_with_its_body() {
    use atrament_notebook_profile::NotebookProfile;

    let paper = NotebookProfile::default();
    let (left, top, _, _) = paper.printable_rect_um()
        .expect("default physical writing rectangle");
    // Four rows fit on the page, but only three remain after the prelude.
    // Title, fenced blank, subtitle and body fit together on page two.
    let region = super::FlowRegion {
        left, top, width: 100_000, height: 41_000,
    };
    for text in [
        "a  \n# a\n~~~txt\n\n~~~\n## ó\nb",
        "a  \n# a\n~~~txt\n\n~~~\nó\n===\nb",
    ] {
        let source = FlowSource::new(FlowFormat::Markdown, text)
            .expect("valid heading chain with fenced blank row");
        let (runs, marks, pages) = super::layout_flow_with_region(
            &source, &paper, None, super::HeadingDecoration::default(),
            Some(region),
        ).expect("heading group fits in the physical region");
        assert_eq!(pages, 2);
        assert_eq!(runs.iter().map(|run| run.page).collect::<Vec<_>>(),
            [1, 2, 2, 2], "title must not orphan from subtitle");
        assert_eq!(runs[2].origin_um.1 - runs[1].origin_um.1, 18_000);
        assert_eq!(runs[3].origin_um.1 - runs[2].origin_um.1, 9_000);
        assert_eq!(marks.len(), 2, "both headings remain decorated");
        assert!(marks.iter().all(|mark| mark.page == 2));
    }
}

#[test]
fn terminal_heading_chain_counts_interior_fenced_blank_rows() {
    use atrament_notebook_profile::NotebookProfile;

    let paper = NotebookProfile::default();
    let (left, top, _, _) = paper.printable_rect_um()
        .expect("default writing rectangle");
    // Three physical rows fit per sheet. After one prelude row, the
    // heading, fenced blank row and terminal subtitle must move together.
    let region = super::FlowRegion {
        left, top, width: 100_000, height: 36_000,
    };
    for source in [
        "a\n# a\n~~~txt\n\n~~~\n## ó",
        "a\n# a\n~~~txt\n\n~~~\nó\n===",
        "a\n# a\n~~~txt\n\n~~~\n## ó\n~~~txt\n\n~~~",
    ] {
        let source = FlowSource::new(FlowFormat::Markdown, source)
            .expect("valid terminal heading chain");
        let (runs, ornaments, pages) = super::layout_flow_with_region(
            &source, &paper, None, super::HeadingDecoration::default(),
            Some(region),
        ).expect("terminal headings and interior blanks fit page two");
        assert_eq!(pages, 2);
        assert_eq!(runs.iter().map(|run| run.page).collect::<Vec<_>>(),
            [1, 2, 2], "interior fence gap must not orphan first title");
        assert_eq!(runs[2].origin_um.1 - runs[1].origin_um.1,
            18_000, "fenced blank reserves physical ink spacing");
        assert_eq!(ornaments.len(), 2);
        assert!(ornaments.iter().all(|mark| mark.page == 2));
    }
}

#[test]
fn measured_pitch_only_counts_rendered_markdown_ink_at_block_boundaries() {
    use atrament_handwriting_statistics::{
        Dimension, DimensionStatistics, HandwritingStatisticsProfile,
        ScalarStatistics,
    };
    use atrament_notebook_profile::NotebookProfile;
    use atrament_unicode_grapheme_segmentation::UnicodeGraphemeSegmentation;

    let tall = ScalarStatistics {
        maximum: 25_000, mean: 25_000, median: 25_000,
        minimum: 25_000, standard_deviation: 0,
    };
    let fragments = [
        "a", "a  ", "a\\", "> a", "> a\n  a", "> a\n   a",
        "- a", "- a\n  a", "1. a", "1. a\n  a", "# a",
        "- [ ] a", "* [X] a", "- [x] a\n  a",
        "~~~txt\na\n~~~", "~~~txt\n\n~~~", "    a", "\\# a",
        "a\n", "a\n\n", "a\n~~~txt\n\n~~~", "a\n# a",
    ];
    let suffixes = [
        "=", "=\na", "===\na", "---\na", "# a", "> a",
        "  =", "  =\na", "   =\na", "    =\na",
        "~~~txt\n=\n~~~\na", "\n=\na", "\n===\na",
        "\\=\na", "- a\n  =\na", "> a\n  =\na",
        "- [ ] a\n  =\na", "[ ] a",
    ];
    // Cross-check source parsing against actual physical text rather
    // than duplicating the parser's idea of structural syntax.
    let mut verified = 0;
    for marker in ["=", "#", "-", ">", "\\", "[", "~"] {
        let statistics = HandwritingStatisticsProfile::baseline(1)
            .with_glyph(marker, DimensionStatistics {
                dimension: Dimension::Height, statistics: tall,
            }, &UnicodeGraphemeSegmentation)
            .expect("valid tall authored marker");
        for prefix in fragments {
            for suffix in suffixes {
                let text = format!("{prefix}\n{suffix}");
                let source = FlowSource::new(FlowFormat::Markdown, &text)
                    .expect("bounded Markdown candidate");
                let Ok((runs, _, _)) = super::layout_flow(
                    &source, &NotebookProfile::default(),
                    Some(&statistics), super::HeadingDecoration::default(),
                ) else {
                    continue;
                };
                let measured = super::measured_pitch(&source,
                    Some(&statistics)).expect("bounded pitch");
                let expected = if runs.iter()
                    .any(|run| run.text.contains(marker)) {
                    29_000
                } else {
                    9_000
                };
                assert_eq!(measured, expected,
                    "marker={marker:?}, source={text:?}, runs={runs:?}");
                verified += 1;
            }
        }
    }
    assert!(verified > 1_000, "admitted boundary sample is meaningful");
}

#[test]
fn checkbox_syntax_does_not_inherit_literal_bracket_height() {
    use atrament_handwriting_statistics::{
        Dimension, DimensionStatistics, HandwritingStatisticsProfile,
        ScalarStatistics,
    };
    use atrament_notebook_profile::NotebookProfile;
    use atrament_unicode_grapheme_segmentation::UnicodeGraphemeSegmentation;

    let tall = ScalarStatistics {
        maximum: 25_000, mean: 25_000, median: 25_000,
        minimum: 25_000, standard_deviation: 0,
    };
    let statistics = HandwritingStatisticsProfile::baseline(1)
        .with_glyph("[", DimensionStatistics {
            dimension: Dimension::Height, statistics: tall,
        }, &UnicodeGraphemeSegmentation)
        .expect("valid tall literal bracket");
    for (text, expected_ink, height, marks) in [
        ("- [ ] a", "a", 9_000, 4),
        ("* [X] a", "a", 9_000, 6),
        ("[ ] a", "[ ] a", 29_000, 0),
    ] {
        let source = FlowSource::new(FlowFormat::Markdown, text)
            .expect("bounded task or literal text");
        let (runs, ink, _) = super::layout_flow(
            &source, &NotebookProfile::default(), Some(&statistics),
            super::HeadingDecoration::default(),
        ).expect("valid visible Markdown text");
        assert_eq!(runs.iter().map(|run| run.text.as_str())
            .collect::<Vec<_>>(), [expected_ink]);
        assert_eq!(ink.len(), marks, "source: {text:?}");
        assert_eq!(super::measured_pitch(&source, Some(&statistics))
            .expect("bounded glyph height"), height);
    }
}

#[test]
fn measured_pitch_matches_rendered_ink_across_mixed_markdown_blocks() {
    use atrament_handwriting_statistics::{
        Dimension, DimensionStatistics, HandwritingStatisticsProfile,
        ScalarStatistics,
    };
    use atrament_notebook_profile::NotebookProfile;
    use atrament_unicode_grapheme_segmentation::UnicodeGraphemeSegmentation;

    let tall = ScalarStatistics {
        maximum: 25_000, mean: 25_000, median: 25_000,
        minimum: 25_000, standard_deviation: 0,
    };
    let lines = [
        "a", "a  ", "a\\", "- a", "* a", "+ a", "- [ ] a",
        "- [x] a", "1. a", "2) a", "# a", "### a", "> a",
        "  a", "   a", "    a", "= a", "=", "==", "===",
        "---", "* * *", "_ _ _", "~~~txt", "~~~", "```txt",
        "```", "\\# a", "\\- a", "\\> a", "[ ] a",
        "  =", "  = a", "    =", "  - a", "- a  ",
        "  + a", "a # #", "\\1. a", "a\\a", "a\n",
        "ó", "o\u{301}", "- ó", "> o\u{301}", "## ñ",
        "¿a?", "¡a!", "«a»", "a∞", "#\u{301} a",
    ];
    let paper = NotebookProfile::default();
    let mut cases = 0usize;
    let mut seed = 0xD00D_F00Du32;
    for marker in ["=", "#", "-", "+", "*", ">", "\\",
        "[", "]", "`", "~", "_", "1", ".", "ó",
        "o\u{301}", "ñ", "¿", "∞"] {
        let statistics = HandwritingStatisticsProfile::baseline(1)
            .with_glyph(marker, DimensionStatistics {
                dimension: Dimension::Height, statistics: tall,
            }, &UnicodeGraphemeSegmentation)
            .expect("valid tall visible glyph");
        for _ in 0..512 {
            let mut parts = Vec::new();
            for _ in 0..5 {
                seed = seed.wrapping_mul(1_664_525)
                    .wrapping_add(1_013_904_223);
                parts.push(lines[(seed as usize) % lines.len()]);
            }
            let text = parts.join("\n");
            let source = FlowSource::new(FlowFormat::Markdown, &text)
                .expect("bounded Markdown sample");
            let Ok((runs, _, _)) = super::layout_flow(
                &source, &paper, Some(&statistics),
                super::HeadingDecoration::default(),
            ) else {
                continue;
            };
            // A combining acute can attach to a punctuation mark: the
            // visible grapheme "#́" is not the authored exact glyph "#".
            let has_exact_marker = runs.iter().any(|run| {
                let mut remaining = run.text.as_str();
                while !remaining.is_empty() {
                    let (grapheme, rest) = super::first_grapheme(remaining)
                        .expect("laid-out text is valid Unicode");
                    if grapheme == marker {
                        return true;
                    }
                    remaining = rest;
                }
                false
            });
            let expected = if has_exact_marker { 29_000 } else { 9_000 };
            let measured = super::measured_pitch(&source,
                Some(&statistics)).expect("valid pitch");
            assert_eq!(measured, expected,
                "marker={marker:?}, source={text:?}, runs={runs:?}");
            cases += 1;
        }
    }
    assert!(cases > 2_000, "mixed grammar must admit enough cases");
}

#[test]
fn seven_line_markdown_boundary_stress_preserves_exact_ink_pitch() {
    // Longer sequences stress list, quote and code-fence state crossing.
    // Compare physical ink rather than copying the Markdown parser.
    use atrament_handwriting_statistics::{
        Dimension, DimensionStatistics, HandwritingStatisticsProfile,
        ScalarStatistics,
    };
    use atrament_notebook_profile::NotebookProfile;
    use atrament_unicode_grapheme_segmentation::UnicodeGraphemeSegmentation;

    let tall = ScalarStatistics {
        maximum: 25_000, mean: 25_000, median: 25_000,
        minimum: 25_000, standard_deviation: 0,
    };
    let fragments = [
        "a", "a  ", "a\\", "a\\\\", "a\\\\\\", "  a", "   a", "    a",
        "=", "==", "===", "---", "  =", "   ===", "    ===",
        "> a", "> a  ", "> a\\", ">", ">  =", "  > a",
        "- a", "- a  ", "- a\\", "- [x] a", "- [ ] a", "+ a",
        "-", "*", "1. a", "2) a", "10. a", "  - a", "   1. a",
        "# a", "## a", "###### a", "# a #", "# a##", "#", "\\# a",
        "```", "```txt", "~~~", "~~~txt", "~~~py", "  ~~~",
        "", " ", "  ", "\\- a", "\\1. a", "[ ] a", "a # a", "a\\a",
    ];
    let paper = NotebookProfile::default();
    let mut seed = 0x9988_7766u32;
    let mut checked = 0usize;
    for glyph in ["=", "#", "-", ">", "\\", "[", "]", "`", "~", "1", "."] {
        let stats = HandwritingStatisticsProfile::baseline(1)
            .with_glyph(glyph, DimensionStatistics {
                dimension: Dimension::Height, statistics: tall,
            }, &UnicodeGraphemeSegmentation)
            .unwrap();
        for _ in 0..500 {
            let mut lines = Vec::new();
            for _ in 0..7 {
                seed = seed.wrapping_mul(1_664_525)
                    .wrapping_add(1_013_904_223);
                lines.push(fragments[(seed as usize) % fragments.len()]);
            }
            let source_text = lines.join("\n");
            let source = FlowSource::new(FlowFormat::Markdown, &source_text)
                .unwrap();
            let Ok((rows, _, _)) = super::layout_flow(
                &source, &paper, Some(&stats),
                super::HeadingDecoration::default(),
            ) else { continue };
            let rendered = rows.iter().any(|row| {
                let mut rest = row.text.as_str();
                while !rest.is_empty() {
                    let (grapheme, after) = super::first_grapheme(rest)
                        .unwrap();
                    if grapheme == glyph { return true }
                    rest = after;
                }
                false
            });
            let actual = super::measured_pitch(&source, Some(&stats))
                .unwrap();
            let expected = if rendered { 29_000 } else { 9_000 };
            assert_eq!(actual, expected,
                "glyph={glyph:?}, source={source_text:?}, rows={rows:?}");
            checked += 1;
        }
    }
    assert!(checked > 500, "need meaningful accepted coverage");
}

#[test]
fn escaped_setext_equals_stays_literal_ink_after_body() {
    use atrament_handwriting_statistics::{
        Dimension, DimensionStatistics, HandwritingStatisticsProfile,
        ScalarStatistics,
    };
    use atrament_notebook_profile::NotebookProfile;
    use atrament_unicode_grapheme_segmentation::UnicodeGraphemeSegmentation;

    let tall = ScalarStatistics {
        maximum: 25_000, mean: 25_000, median: 25_000,
        minimum: 25_000, standard_deviation: 0,
    };
    let statistics = HandwritingStatisticsProfile::baseline(1)
        .with_glyph("=", DimensionStatistics {
            dimension: Dimension::Height, statistics: tall,
        }, &UnicodeGraphemeSegmentation)
        .expect("valid tall authored equals glyph");
    let paper = NotebookProfile::default();
    for escaped in ["\\=", "\\==", "\\===", "  \\==="] {
        let source = FlowSource::new(
            FlowFormat::Markdown, &format!("a\n{escaped}\na"),
        ).expect("bounded Markdown with literal Setext characters");
        let (runs, ornaments, pages) = super::layout_flow(
            &source, &paper, Some(&statistics),
            super::HeadingDecoration::default(),
        ).expect("escaped marker remains authored ink");
        assert_eq!(pages, 1);
        assert!(ornaments.is_empty(), "no heading underline");
        assert_eq!(runs.len(), 1);
        let expected = format!(
            "a {} a", escaped.trim_start().trim_start_matches('\\'),
        );
        assert_eq!(runs[0].text, expected);
        assert_eq!(super::measured_pitch(&source, Some(&statistics))
            .expect("literal equals determines row pitch"), 29_000);
    }
    // Unescaped equals is still title syntax and cannot become glyph ink.
    let title = FlowSource::new(FlowFormat::Markdown, "a\n===\na")
        .expect("valid Setext source");
    assert_eq!(super::measured_pitch(&title, Some(&statistics))
        .expect("heading syntax has no equals glyph"), 9_000);
}

#[test]
fn unicode_source_breaks_keep_literal_setext_markers_and_exact_pitch() {
    use atrament_handwriting_statistics::{
        Dimension, DimensionStatistics, HandwritingStatisticsProfile,
        ScalarStatistics,
    };
    use atrament_notebook_profile::NotebookProfile;
    use atrament_unicode_grapheme_segmentation::UnicodeGraphemeSegmentation;

    let tall = ScalarStatistics {
        maximum: 25_000, mean: 25_000, median: 25_000,
        minimum: 25_000, standard_deviation: 0,
    };
    let mut statistics = HandwritingStatisticsProfile::baseline(1);
    for glyph in ["=", "-"] {
        statistics = statistics.with_glyph(glyph, DimensionStatistics {
            dimension: Dimension::Height, statistics: tall,
        }, &UnicodeGraphemeSegmentation)
            .expect("valid tall authored punctuation");
    }
    for (source_text, expected_rows, expected_pitch, gap) in [
        ("a\u{2028}===\na", ["a", "=== a"], 29_000, 0),
        ("a\u{2029}===\na", ["a", "=== a"], 29_000, 4_000),
        ("a\u{2028}\\===\na", ["a", "=== a"], 29_000, 0),
        ("a\u{2029}\\===\na", ["a", "=== a"], 29_000, 4_000),
    ] {
        let source = FlowSource::new(FlowFormat::Markdown, source_text)
            .expect("Unicode source breaks are structural");
        let (rows, ornaments, pages) = super::layout_flow(
            &source, &NotebookProfile::default(), Some(&statistics),
            super::HeadingDecoration::default(),
        ).expect("literal punctuation remains written ink");
        assert_eq!(pages, 1);
        assert!(ornaments.is_empty(), "no invented Setext underline");
        assert_eq!(rows.iter().map(|row| row.text.as_str())
            .collect::<Vec<_>>(), expected_rows, "{source_text:?}");
        assert_eq!(super::measured_pitch(&source, Some(&statistics))
            .expect("literal glyph height is measurable"), expected_pitch);
        assert_eq!(rows[1].origin_um.1 - rows[0].origin_um.1,
            expected_pitch + gap, "{source_text:?}");
    }
    for (source_text, expected_rows, rule) in [
        ("a\u{2028}---\na", ["a", "a"], true),
        ("a\u{2029}---\na", ["a", "a"], true),
        ("a\u{2028}\\---\na", ["a", "--- a"], false),
        ("a\u{2029}\\---\na", ["a", "--- a"], false),
    ] {
        let source = FlowSource::new(FlowFormat::Markdown, source_text)
            .expect("Unicode breaks preserve thematic syntax and escapes");
        let (rows, ornaments, _) = super::layout_flow(
            &source, &NotebookProfile::default(), Some(&statistics),
            super::HeadingDecoration::default(),
        ).expect("thematic rules or literal hyphens render");
        assert_eq!(rows.iter().map(|row| row.text.as_str())
            .collect::<Vec<_>>(), expected_rows, "{source_text:?}");
        assert_eq!(ornaments.len(), usize::from(rule),
            "only an unescaped thematic separator has an ornament");
        let expected_pitch = if rule { 9_000 } else { 29_000 };
        assert_eq!(super::measured_pitch(&source, Some(&statistics))
            .expect("rendered ink determines physical pitch"),
            expected_pitch, "{source_text:?}");
    }
    let heading = FlowSource::new(FlowFormat::Markdown, "a\n===\na")
        .expect("Setext title has no authored equals mark");
    let (rows, ornaments, _) = super::layout_flow(
        &heading, &NotebookProfile::default(), Some(&statistics),
        super::HeadingDecoration::default(),
    ).expect("actual Setext syntax draws a heading rule");
    assert_eq!(rows.iter().map(|row| row.text.as_str())
        .collect::<Vec<_>>(), ["a", "a"]);
    assert_eq!(ornaments.len(), 1);
    assert_eq!(super::measured_pitch(&heading, Some(&statistics))
        .expect("syntax must not inflate the pitch"), 9_000);
}

#[test]
fn markdown_boundary_combinations_measure_only_rendered_equals() {
    use atrament_handwriting_statistics::{
        Dimension, DimensionStatistics, HandwritingStatisticsProfile,
        ScalarStatistics,
    };
    use atrament_notebook_profile::NotebookProfile;
    use atrament_unicode_grapheme_segmentation::UnicodeGraphemeSegmentation;

    let tall = ScalarStatistics {
        maximum: 25_000,
        mean: 25_000,
        median: 25_000,
        minimum: 25_000,
        standard_deviation: 0,
    };
    let statistics = HandwritingStatisticsProfile::baseline(1)
        .with_glyph(
            "=",
            DimensionStatistics {
                dimension: Dimension::Height,
                statistics: tall,
            },
            &UnicodeGraphemeSegmentation,
        )
        .unwrap();
    let fragments = [
        "a",
        "a  ",
        "## a",
        "\\===",
        "===",
        "---",
        "> a",
        "> =",
        "  =",
        "    =",
        "  \\===",
        "- a",
        "- [x] a",
        "1. a",
        "```",
        "````",
        "~~~",
        "  ~~~",
        "  > =",
        "  a",
        "a\\",
        "\\#",
        "\t=",
        "=",
        "a\u{2028}",
        "a\u{2029}",
        "a\r\n",
        "  a  ",
        "  \\---",
        "a\t=",
    ];
    let mut accepted = 0usize;
    for left in fragments {
        for right in fragments {
            for suffix in
                ["a", "===", "\\===", "> a", "```", "  a", "a  ", "- a"]
            {
                let text = format!("{left}\n{right}\n{suffix}\na");
                let source =
                    FlowSource::new(FlowFormat::Markdown, &text).unwrap();
                let Ok((rows, _, _)) = super::layout_flow(
                    &source,
                    &NotebookProfile::default(),
                    Some(&statistics),
                    super::HeadingDecoration::default(),
                ) else {
                    continue;
                };
                let rendered = rows.iter().any(|row| row.text.contains('='));
                let pitch =
                    super::measured_pitch(&source, Some(&statistics)).unwrap();
                assert_eq!(
                    pitch,
                    if rendered {
                        29_000
                    } else {
                        9_000
                    },
                    "source={text:?}, rows={rows:?}"
                );
                accepted += 1;
            }
        }
    }
    assert!(accepted > 3_000, "insufficient valid boundary combinations");
}

#[test]
fn markdown_mixed_transport_pitch_matches_exact_rendered_ink()
    -> Result<(), String> {
    // Independent rendered runs, not the syntax parser, determine whether
    // a structural marker contributes physical pen height.
    use atrament_handwriting_statistics::{Dimension, DimensionStatistics,
        HandwritingStatisticsProfile, ScalarStatistics};
    use atrament_notebook_profile::NotebookProfile;
    use atrament_unicode_grapheme_segmentation::UnicodeGraphemeSegmentation;
    let tall = ScalarStatistics { maximum: 25_000, mean: 25_000,
        median: 25_000, minimum: 25_000, standard_deviation: 0 };
    let fragments = ["a", "===", "--", "---", "~~~", "~~~~", "```", "````",
        "~~~lang", "  ~~~", "    ~~~", "a  ", "a\\", "a\\\\", "\\===",
        "  \\===", "  a", "   a", "    a", "> a", "> =", ">", "  > a",
        "- a", "- [x] a", "  - a", "   - a", "1. a", "  1. a",
        "# a", "###### a", "#", "# a #", "a\u{2028}=", "a\u{2029}=",
        "a\t=", "a\r\n=", "a\u{301}=", "", " ", "   ", "=", "~", "`",
        "\\=", "\\- a", "a\n=", "_ _ _", "> a  ", "  =", "    ="];
    let paper = NotebookProfile::default();
    let mut seed = 0x1357_2468u32;
    for glyph in ["=", "-", "~", "`", ">", "\\"] {
        let stats = HandwritingStatisticsProfile::baseline(1)
            .with_glyph(glyph, DimensionStatistics {
                dimension: Dimension::Height, statistics: tall,
            }, &UnicodeGraphemeSegmentation)
            .map_err(|error| format!("{error:?}"))?;
        let mut accepted = 0usize;
        for _ in 0..1_500usize {
            let mut parts = Vec::new();
            for _ in 0..7usize {
                seed = seed.wrapping_mul(1_664_525)
                    .wrapping_add(1_013_904_223);
                let index = usize::try_from(seed)
                    .map_err(|error| error.to_string())?
                    .rem_euclid(fragments.len());
                parts.push(*fragments.get(index)
                    .ok_or("generated fragment outside inventory")?);
            }
            let text = parts.join("\n");
            let source = FlowSource::new(FlowFormat::Markdown, &text)?;
            let Ok((runs, _, _)) = super::layout_flow(&source, &paper,
                Some(&stats), super::HeadingDecoration::default())
                else { continue };
            let actual = super::measured_pitch(&source, Some(&stats))?;
            let mut rendered = false;
            for run in &runs {
                let mut remaining = run.text.as_str();
                while !remaining.is_empty() {
                    let (grapheme, rest) = super::first_grapheme(remaining)?;
                    rendered |= grapheme == glyph;
                    remaining = rest;
                }
            }
            let expected = if rendered { 29_000 } else { 9_000 };
            if actual != expected {
                return Err(format!(
                    "glyph={glyph:?} pitch {actual} != {expected}: \
                     {text:?}; {runs:?}"
                ));
            }
            accepted += 1usize;
        }
        if accepted <= 400usize {
            return Err(format!("insufficient {glyph} source coverage"));
        }
    }
    Ok(())
}

#[test]
fn plain_wrapping_preserves_exact_unicode_graphemes_across_pages()
    -> Result<(), String> {
    use atrament_notebook_profile::NotebookProfile;

    let alphabet = ["a", "\u{f3}", "o\u{301}", "\u{f1}", "n\u{303}",
        "\u{1f469}\u{200d}\u{1f3eb}", "\u{221e}", "\u{bf}",
        "\u{a1}", "\u{e9}", "i\u{301}", "="];
    let paper = NotebookProfile::default();
    let region = super::FlowRegion {
        left: 30_000, top: 40_000, width: 32_000, height: 25_000,
    };
    let mut seed = 0x3e6d_5a21u32;
    let mut positioned_rows = 0usize;
    for sample in 0usize..512usize {
        let mut words = Vec::new();
        for _ in 0usize..9usize {
            seed = seed.wrapping_mul(1_664_525)
                .wrapping_add(1_013_904_223);
            let size = 4usize + usize::try_from(seed.checked_rem(10)
                .ok_or("invalid seed divisor")?)
                .map_err(|error| error.to_string())?;
            let mut word = String::new();
            for _ in 0usize..size {
                seed = seed.wrapping_mul(1_664_525)
                    .wrapping_add(1_013_904_223);
                let index = usize::try_from(seed)
                    .map_err(|error| error.to_string())?
                    .checked_rem(alphabet.len())
                    .ok_or("invalid alphabet divisor")?;
                word.push_str(alphabet.get(index)
                    .ok_or("generated glyph outside alphabet")?);
            }
            words.push(word);
        }
        let source_text = words.join(" ");
        let source = FlowSource::new(FlowFormat::Plain, &source_text)?;
        let (rows, ornaments, pages) = super::layout_flow_with_region(
            &source, &paper, None, super::HeadingDecoration::default(),
            Some(region),
        )?;
        if !ornaments.is_empty() || pages <= 1 {
            return Err(format!("sample {sample} did not paginate plain ink"));
        }
        let nonspace = |text: &str| -> Result<Vec<String>, String> {
            let mut remaining = text;
            let mut exact = Vec::new();
            while !remaining.is_empty() {
                let (glyph, rest) = super::first_grapheme(remaining)?;
                if glyph != " " { exact.push(glyph.to_owned()); }
                remaining = rest;
            }
            Ok(exact)
        };
        let expected = nonspace(&source_text)?;
        let actual = rows.iter().map(|row| nonspace(&row.text))
            .collect::<Result<Vec<_>, _>>()?
            .into_iter().flatten().collect::<Vec<_>>();
        if actual != expected {
            return Err(format!(
                "sample {sample} lost or split Unicode graphemes: {actual:?}"
            ));
        }
        for row in rows {
            if !(1..=pages).contains(&row.page)
                || row.origin_um.0 != region.left + 2_500
                || row.origin_um.1 < region.top + 2_500
                || row.origin_um.1 + 9_000 >
                    region.top + region.height - 2_500
                || row.text.is_empty() {
                return Err(format!(
                    "sample {sample} placed a row outside the region: {row:?}"
                ));
            }
            positioned_rows += 1;
        }
    }
    if positioned_rows <= 8_000 {
        return Err("insufficient physical wrap boundaries".into());
    }
    Ok(())
}

#[test]
fn markdown_blocks_preserve_exact_unicode_graphemes_across_pages()
    -> Result<(), String> {
    use atrament_notebook_profile::NotebookProfile;

    let alphabet = ["a", "ó", "o\u{301}", "ñ", "n\u{303}",
        "👩\u{200d}🏫", "∞", "¿", "¡", "é", "e\u{301}"];
    let paper = NotebookProfile::default();
    let region = super::FlowRegion {
        left: 30_000, top: 40_000, width: 34_000, height: 25_000,
    };
    let mut seed = 0x6721_5badu32;
    let mut checked_rows = 0usize;
    for sample in 0..256usize {
        let mut words = Vec::new();
        for _ in 0..12 {
            seed = seed.wrapping_mul(1_664_525)
                .wrapping_add(1_013_904_223);
            let length = 5 + usize::try_from(seed % 6)
                .map_err(|error| error.to_string())?;
            let mut word = String::new();
            for _ in 0..length {
                seed = seed.wrapping_mul(1_664_525)
                    .wrapping_add(1_013_904_223);
                let index = usize::try_from(seed)
                    .map_err(|error| error.to_string())? % alphabet.len();
                word.push_str(alphabet.get(index)
                    .ok_or("Unicode test glyph outside inventory")?);
            }
            words.push(word);
        }
        // List continuations, adjacent quotes and ordinary soft/hard
        // paragraph breaks must not become authored pen glyphs.
        let body_break = if sample % 2 == 0 { "\n" } else { "\u{2028}" };
        let source_text = format!(
            "# a\n- {} {}\n  {} {}\n> {} {}\n> {} {}\n{} {}{}{} {}",
            words[0], words[1], words[2], words[3],
            words[4], words[5], words[6], words[7],
            words[8], words[9], body_break, words[10], words[11],
        );
        let source = FlowSource::new(FlowFormat::Markdown, &source_text)?;
        let (rows, ornaments, pages) = super::layout_flow_with_region(
            &source, &paper, None, super::HeadingDecoration::default(),
            Some(region),
        )?;
        if pages < 3 || pages > 32 {
            return Err(format!(
                "sample {sample} has unexpected physical page count {pages}"
            ));
        }
        let nonspace = |value: &str| -> Result<Vec<String>, String> {
            let mut remaining = value;
            let mut glyphs = Vec::new();
            while !remaining.is_empty() {
                let (glyph, rest) = super::first_grapheme(remaining)?;
                if glyph != " " {
                    glyphs.push(glyph.to_owned());
                }
                remaining = rest;
            }
            Ok(glyphs)
        };
        let mut expected = nonspace("a")?;
        for word in &words {
            expected.extend(nonspace(word)?);
        }
        let actual = rows.iter().map(|row| nonspace(&row.text))
            .collect::<Result<Vec<_>, _>>()?
            .into_iter().flatten().collect::<Vec<_>>();
        if actual != expected {
            return Err(format!(
                "Markdown sample {sample} lost or split Unicode ink: \
                 {source_text:?}; rows={rows:?}"
            ));
        }
        if rows.iter().filter(|row| {
            row.origin_um.0 == region.left + 2_500
        }).count() < 2
            || ornaments.iter().filter(|line| line.width_um == 550).count() != 1
            || ornaments.iter().filter(|line| line.width_um == 450).count() != 1
            || ornaments.iter().filter(|line| line.width_um == 400).count() < 2
        {
            return Err(format!(
                "sample {sample} lost body, heading, bullet or quote ink"
            ));
        }
        for row in rows {
            if !(1..=pages).contains(&row.page)
                || ![region.left + 2_500, region.left + 8_500]
                    .contains(&row.origin_um.0)
                || row.origin_um.1 < region.top + 2_500
                || row.origin_um.1 + 9_000 >
                    region.top + region.height - 2_500 {
                return Err(format!(
                    "Markdown sample {sample} escaped physical region: \
                     {row:?}"
                ));
            }
            checked_rows += 1;
        }
    }
    if checked_rows <= 3_000 {
        return Err("insufficient Markdown physical wrap coverage".into());
    }
    Ok(())
}

#[test]
fn mixed_handwriting_metrics_keep_unicode_inside_paged_markdown()
    -> Result<(), String> {
    use atrament_handwriting_statistics::{
        Dimension, DimensionStatistics, HandwritingStatisticsProfile,
        ScalarStatistics,
    };
    use atrament_notebook_profile::NotebookProfile;
    use atrament_unicode_grapheme_segmentation::UnicodeGraphemeSegmentation;

    let mut stats = HandwritingStatisticsProfile::baseline(33);
    for (glyph, dimension, mean) in [
        ("a", Dimension::Width, 5_200),
        ("a", Dimension::LetterSpacing, -1_200),
        ("ó", Dimension::Width, 6_400),
        ("ó", Dimension::LetterSpacing, -1_000),
        ("o\u{301}", Dimension::Width, 3_300),
        ("o\u{301}", Dimension::LetterSpacing, 1_700),
        ("👩\u{200d}🏫", Dimension::Width, 8_500),
        ("👩\u{200d}🏫", Dimension::LetterSpacing, -2_000),
        ("∞", Dimension::Height, 6_000),
        ("∞", Dimension::BaselineOffset, 2_000),
    ] {
        let value = ScalarStatistics { minimum: mean, maximum: mean,
            mean, median: mean, standard_deviation: 0 };
        stats = stats.with_glyph(glyph, DimensionStatistics {
            dimension, statistics: value,
        }, &UnicodeGraphemeSegmentation)
            .map_err(|error| format!("{error:?}"))?;
    }
    let glyphs = ["a", "ó", "o\u{301}", "👩\u{200d}🏫", "∞", "ñ", "¿"];
    let paper = NotebookProfile::default();
    let region = super::FlowRegion {
        left: 30_000, top: 40_000, width: 37_000, height: 31_000,
    };
    let mut seed = 0x475a_39cdu32;
    let mut checked = 0usize;
    let mut changed_wrap = 0usize;
    for sample in 0..256usize {
        let mut words = Vec::new();
        for _ in 0..12 {
            let mut word = String::new();
            for _ in 0..7 {
                seed = seed.wrapping_mul(1_664_525)
                    .wrapping_add(1_013_904_223);
                word.push_str(glyphs[(seed as usize) % glyphs.len()]);
            }
            words.push(word);
        }
        let text = format!(
            "# a\n- {} {}\n  {} {}\n> {} {}\n> {} {}\n{} {}\n{} {}",
            words[0], words[1], words[2], words[3],
            words[4], words[5], words[6], words[7],
            words[8], words[9], words[10], words[11],
        );
        let source = FlowSource::new(FlowFormat::Markdown, &text)?;
        let (rows, marks, pages) = super::layout_flow_with_region(
            &source, &paper, Some(&stats),
            super::HeadingDecoration::default(), Some(region),
        )?;
        let (nominal_rows, _, _) = super::layout_flow_with_region(
            &source, &paper, None,
            super::HeadingDecoration::default(), Some(region),
        )?;
        changed_wrap += usize::from(rows.len() != nominal_rows.len());
        if pages < 3 || marks.len() < 3 {
            return Err(format!("sample {sample} missed page ornaments"));
        }
        let mut expected = Vec::new();
        for word in std::iter::once(&"a".to_owned()).chain(words.iter()) {
            let mut remaining = word.as_str();
            while !remaining.is_empty() {
                let (glyph, rest) = super::first_grapheme(remaining)?;
                expected.push(glyph.to_owned());
                remaining = rest;
            }
        }
        let mut actual = Vec::new();
        for row in &rows {
            if row.page > pages || row.page == 0
                || row.origin_um.1 < region.top + 2_500
                || row.origin_um.1 + 12_000 >
                    region.top + region.height - 2_500 {
                return Err(format!("sample {sample} bad page row {row:?}"));
            }
            let right = region.left + region.width - 1_500;
            let mut x = row.origin_um.0;
            let mut remaining = row.text.as_str();
            while !remaining.is_empty() {
                let (glyph, rest) = super::first_grapheme(remaining)?;
                let (advance, contour) = super::GlyphOverlay::flow_advance_um(
                    glyph, Some(&stats),
                )?;
                let endpoint = x.checked_add(advance.max(contour + 800))
                    .ok_or("row glyph extent overflow")?;
                if endpoint > right {
                    return Err(format!(
                        "sample {sample} exceeded horizontal text region: \
                         {row:?}; glyph {glyph:?}, endpoint {endpoint}"
                    ));
                }
                if glyph != " " { actual.push(glyph.to_owned()); }
                x = x.checked_add(advance)
                    .ok_or("row advance overflow")?;
                remaining = rest;
            }
            checked += 1;
        }
        if actual != expected {
            return Err(format!(
                "sample {sample} lost mixed-width Unicode: {rows:?}"
            ));
        }
    }
    if checked < 2_000 || changed_wrap < 100 {
        return Err(format!(
            "insufficient mixed-metric coverage: {checked} rows, \
             {changed_wrap} changed wraps"
        ));
    }
    Ok(())
}

#[test]
fn mixed_markdown_pitch_matches_rendered_ink_at_fence_and_list_edges()
    -> Result<(), String> {
    use atrament_handwriting_statistics::{
        Dimension, DimensionStatistics, HandwritingStatisticsProfile,
        ScalarStatistics,
    };
    use atrament_notebook_profile::NotebookProfile;
    use atrament_unicode_grapheme_segmentation::UnicodeGraphemeSegmentation;

    let parts = [
        "a", "a  ", "a\\", "a  b", "# a", "## a", "a\n===",
        "===", "---", "\\===", "> a", "> a  ", "  a", "    a",
        "- a", "- [x] a", "1. a", "  ===", "~~~", "~~~rs",
        "```", "```rs", "", "  ", "a\u{2028}b", "a\u{2029}b",
        "ó", "o\u{301}", "ñ",
    ];
    let paper = NotebookProfile::default();
    let mut tested = 0usize;
    for tall_glyph in ["=", "-", "#", ">", "\\", "`", "~", "a",
        "ó", "o\u{301}", "ñ"] {
        let tall = ScalarStatistics {
            minimum: 25_000, maximum: 25_000, mean: 25_000,
            median: 25_000, standard_deviation: 0,
        };
        let statistics = HandwritingStatisticsProfile::baseline(0)
            .with_glyph(tall_glyph, DimensionStatistics {
                dimension: Dimension::Height, statistics: tall,
            }, &UnicodeGraphemeSegmentation)
            .map_err(|error| format!("{error:?}"))?;
        let mut seed = 0x83e1_7ca5u32;
        for sample in 0..512usize {
            let mut source_text = String::new();
            for line in 0..5 {
                seed = seed.wrapping_mul(1_664_525)
                    .wrapping_add(1_013_904_223);
                if line != 0 { source_text.push('\n'); }
                source_text.push_str(parts[((seed >> 8) as usize)
                    % parts.len()]);
            }
            let Ok(source) = FlowSource::new(FlowFormat::Markdown,
                &source_text) else { continue };
            let Ok((rows, _, _)) = super::layout_flow(
                &source, &paper, Some(&statistics),
                super::HeadingDecoration::default(),
            ) else { continue };
            let mut visible = false;
            for row in &rows {
                let mut remaining = row.text.as_str();
                while !remaining.is_empty() {
                    let (grapheme, rest) = super::first_grapheme(remaining)?;
                    visible |= grapheme == tall_glyph;
                    remaining = rest;
                }
            }
            let expected = if visible { 29_000 } else { 9_000 };
            let pitch = super::measured_pitch(&source, Some(&statistics))?;
            assert_eq!(pitch, expected,
                "sample {sample}, glyph {tall_glyph:?}, \
                 source {source_text:?}, actual rows {rows:?}");
            tested += 1;
        }
    }
    assert!(tested > 900, "insufficient admitted Markdown cases");
    Ok(())
}

#[test]
fn signed_baseline_pitch_tracks_only_exact_rendered_unicode_ink()
    -> Result<(), String> {
    use atrament_handwriting_statistics::{
        Dimension, DimensionStatistics, HandwritingStatisticsProfile,
        ScalarStatistics,
    };
    use atrament_notebook_profile::NotebookProfile;
    use atrament_unicode_grapheme_segmentation::UnicodeGraphemeSegmentation;

    let paper = NotebookProfile::default();
    let cases = [
        ("=", -20_000), ("#", 20_000), ("-", -20_000),
        (">", 20_000), ("`", -20_000), ("o\u{301}", 20_000),
        ("👩\u{200d}🏫", -20_000), ("\u{00a0}", 20_000),
    ];
    let sources = [
        "a\n===\na", "a  \n===\na", "a\n\\===\na",
        "# a\n## a\n### a", "\\# a\na", "- a\n  a",
        "\\- a\na", "> a\n  a", "\\> a\na",
        "a\n```txt\n===\n```\na", "a\n````\n`\n````\na",
        "a\n---\na", "a\n\\---\na", "a\no\u{301} 👩\u{200d}🏫",
        "a\n\u{00a0}\na", "a\n# a\no\u{301} 👩\u{200d}🏫",
    ];
    let mut checked = 0usize;
    let mut tall = 0usize;
    let mut nominal = 0usize;
    for (glyph, baseline) in cases {
        let statistics = ScalarStatistics {
            minimum: baseline, maximum: baseline, mean: baseline,
            median: baseline, standard_deviation: 0,
        };
        let profile = HandwritingStatisticsProfile::baseline(0)
            .with_glyph(glyph, DimensionStatistics {
                dimension: Dimension::BaselineOffset, statistics,
            }, &UnicodeGraphemeSegmentation)
            .map_err(|error| format!("{error:?}"))?;
        let mut glyph_tall = 0usize;
        let mut glyph_nominal = 0usize;
        for text in sources {
            let source = FlowSource::new(FlowFormat::Markdown, text)?;
            let (rows, _, _) = super::layout_flow(
                &source, &paper, Some(&profile),
                super::HeadingDecoration::default(),
            )?;
            let mut rendered = false;
            for row in &rows {
                let mut remaining = row.text.as_str();
                while !remaining.is_empty() {
                    let (exact, rest) = super::first_grapheme(remaining)?;
                    rendered |= exact == glyph;
                    remaining = rest;
                }
            }
            let expected = if rendered { 27_000 } else { 9_000 };
            let actual = super::measured_pitch(&source, Some(&profile))?;
            assert_eq!(actual, expected,
                "signed baseline {baseline} for {glyph:?} in {text:?}, \
                 rendered rows: {rows:?}");
            tall += usize::from(rendered);
            nominal += usize::from(!rendered);
            glyph_tall += usize::from(rendered);
            glyph_nominal += usize::from(!rendered);
            checked += 1;
        }
        assert!(glyph_tall > 0 && glyph_nominal > 0,
            "{glyph:?} must appear as ink and as absent/syntax");
    }
    assert_eq!(checked, 128);
    assert!(tall > 10 && nominal > 25,
        "exercise both real Unicode ink and syntax-only markers");
    Ok(())
}

#[test]
fn markdown_pitch_agrees_with_actual_ink_for_escaped_and_nested_edges()
    -> Result<(), String> {
    use atrament_handwriting_statistics::{
        Dimension, DimensionStatistics, HandwritingStatisticsProfile,
        ScalarStatistics,
    };
    use atrament_notebook_profile::NotebookProfile;
    use atrament_unicode_grapheme_segmentation::UnicodeGraphemeSegmentation;

    let parts = [
        "a", "a  ", "a\\", "a\\\\", "# a", "## a #", "### a ###",
        "\\# a", "\\- a", "\\===", "\\---", "\\~~~", "\\`a",
        "a\n===", "a\n---", "=", "===", "---", "* * *", "_ _ _",
        "~~~", "~~~rs", "~~~~", "```", "```rs", "````", "``",
        "a  b", "- a", "+ a", "* a", "- [x] a", "- [ ] a",
        "1. a", "12) a", "1\\. a", "> a", ">a", "> a  ",
        "  a", "   a", "    a", "   ===", "a\u{2028}a",
        "a\u{2029}a", "ó", "o\u{301}", "👩\u{200d}🏫", "",
    ];
    let paper = NotebookProfile::default();
    let mut checked = 0;
    for glyph in ["#", "=", "-", "*", "~", "`", ">", "\\", "a",
        "ó", "o\u{301}", "👩\u{200d}🏫"] {
        let height = ScalarStatistics {
            minimum: 25_000, maximum: 25_000, mean: 25_000,
            median: 25_000, standard_deviation: 0,
        };
        let stats = HandwritingStatisticsProfile::baseline(0)
            .with_glyph(glyph, DimensionStatistics {
                dimension: Dimension::Height, statistics: height,
            }, &UnicodeGraphemeSegmentation)
            .map_err(|error| format!("{error:?}"))?;
        let mut seed = 0x7292_313du32;
        for sample in 0..480 {
            let mut text = String::new();
            for i in 0..7 {
                seed = seed.wrapping_mul(1_664_525)
                    .wrapping_add(1_013_904_223);
                if i != 0 { text.push('\n'); }
                text.push_str(parts[(seed as usize >> 7) % parts.len()]);
            }
            let Ok(source) = FlowSource::new(FlowFormat::Markdown, &text)
                else { continue };
            let Ok((rows, _, _)) = super::layout_flow(
                &source, &paper, Some(&stats),
                super::HeadingDecoration::default(),
            ) else { continue };
            let mut visible = false;
            for row in &rows {
                let mut remaining = row.text.as_str();
                while !remaining.is_empty() {
                    let (exact, next) = super::first_grapheme(remaining)?;
                    visible |= exact == glyph;
                    remaining = next;
                }
            }
            let expected = if visible { 29_000 } else { 9_000 };
            assert_eq!(super::measured_pitch(&source, Some(&stats))?,
                expected, "sample {sample}, glyph {glyph:?}, \
                 source {text:?}, rows {rows:?}");
            checked += 1;
        }
    }
    assert!(checked > 2_000, "admit enough mixed Markdown sources");
    Ok(())
}

#[test]
fn varied_ordered_labels_keep_unicode_within_physical_page_regions()
    -> Result<(), String> {
    use atrament_handwriting_statistics::{
        Dimension, DimensionStatistics, HandwritingStatisticsProfile,
        ScalarStatistics,
    };
    use atrament_notebook_profile::NotebookProfile;
    use atrament_unicode_grapheme_segmentation::UnicodeGraphemeSegmentation;

    let glyphs = ["0", "1", "2", "3", "4", "5", "6", "7", "8", "9",
        ".", ")", "a", "ó", "o\u{301}", "ñ"];
    let mut stats = HandwritingStatisticsProfile::baseline(47);
    for (index, glyph) in glyphs.iter().enumerate() {
        let width = 2_300 + (index as i32 % 7) * 620;
        let spacing = (index as i32 % 5 - 2) * 550;
        for (dimension, mean) in [
            (Dimension::Width, width),
            (Dimension::LetterSpacing, spacing),
        ] {
            let value = ScalarStatistics {
                minimum: mean, maximum: mean, mean, median: mean,
                standard_deviation: 0,
            };
            stats = stats.with_glyph(glyph, DimensionStatistics {
                dimension, statistics: value,
            }, &UnicodeGraphemeSegmentation)
                .map_err(|error| format!("{error:?}"))?;
        }
    }
    let region = super::FlowRegion {
        left: 30_000, top: 40_000, width: 120_000, height: 14_000,
    };
    let paper = NotebookProfile::default();
    let first_x = region.left + 8_500;
    let right = region.left + region.width - 1_500;
    let mut seed = 0x8b29_7a45u32;
    let mut admitted = 0usize;
    let mut page_rows = 0usize;
    let mut label_lengths = [0usize; 9];
    // All admitted Markdown marker lengths, including the nine-digit edge.
    // Keep only one physical row per page so wraps test page-local origin.
    for sample in 0..540usize {
        seed = seed.wrapping_mul(1_664_525)
            .wrapping_add(1_013_904_223);
        let magnitude = sample % 9;
        let first = 10u32.pow(magnitude as u32);
        let digits = (first + seed % (first * 9)).to_string();
        let marker = format!("{digits}{}", if sample % 2 == 0 {
            '.'
        } else { ')' });
        let spaces = 1 + sample % 4;
        let mut body = String::new();
        for _ in 0..40 {
            seed = seed.wrapping_mul(1_664_525)
                .wrapping_add(1_013_904_223);
            body.push_str(glyphs[12 + (seed as usize % 4)]);
        }
        let source_text = format!("{marker}{}{body}", " ".repeat(spaces));
        let source = FlowSource::new(FlowFormat::Markdown, &source_text)?;
        let (rows, marks, pages) = match super::layout_flow_with_region(
            &source, &paper, Some(&stats),
            super::HeadingDecoration::default(), Some(region),
        ) {
            Ok(result) => result,
            Err(message) if message.contains(
                "ordered list label and first glyph exceed") => continue,
            Err(message) => return Err(format!(
                "sample {sample}, source {source_text:?}: {message}"
            )),
        };
        if !marks.is_empty() || pages != rows.len() || rows.len() < 2 {
            return Err(format!(
                "sample {sample} has unexpected physical pages or marks: \
                 {rows:?}, {pages} pages"
            ));
        }
        let joined = rows.iter().map(|row| row.text.as_str())
            .collect::<String>();
        if joined != source_text {
            return Err(format!(
                "sample {sample} lost exact ordered Unicode ink: \
                 {source_text:?} != {joined:?}"
            ));
        }
        let mut indent = 4_000u32 * spaces as u32;
        let mut marker_glyph = marker.as_str();
        while !marker_glyph.is_empty() {
            let (glyph, remaining) = super::first_grapheme(marker_glyph)?;
            indent += super::GlyphOverlay::flow_advance_um(
                glyph, Some(&stats),
            )?.0;
            marker_glyph = remaining;
        }
        if !rows[0].text.starts_with(&format!(
            "{marker}{}", " ".repeat(spaces)
        )) || rows[0].text == format!("{marker}{}", " ".repeat(spaces)) {
            return Err(format!(
                "sample {sample} split its numeral from first body glyph"
            ));
        }
        for (index, row) in rows.iter().enumerate() {
            let x = if index == 0 { first_x } else { first_x + indent };
            if row.origin_um != (x, region.top + 2_500)
                || row.page != index + 1 {
                return Err(format!(
                    "sample {sample} lost its measured hanging indent: \
                     {row:?}, expected x {x} on page {}", index + 1
                ));
            }
            let mut cursor = x;
            let mut remaining = row.text.as_str();
            while !remaining.is_empty() {
                let (glyph, rest) = super::first_grapheme(remaining)?;
                let (advance, contour) =
                    super::GlyphOverlay::flow_advance_um(glyph, Some(&stats))?;
                if cursor.checked_add(advance.max(contour + 800))
                    .is_none_or(|end| end > right) {
                    return Err(format!(
                        "sample {sample} glyph {glyph:?} escapes page {}: \
                         {row:?}", row.page
                    ));
                }
                cursor += advance;
                remaining = rest;
            }
            page_rows += 1;
        }
        admitted += 1;
        label_lengths[magnitude] += 1;
    }
    if label_lengths.iter().any(|&count| count < 40)
        || admitted < 450 || page_rows < 900 {
        return Err(format!(
            "insufficient ordered-label coverage: {label_lengths:?}, \
             {admitted} admitted sources and {page_rows} physical rows"
        ));
    }
    Ok(())
}

#[test]
fn ordered_marker_contours_are_preflighted_before_hanging_indent()
    -> Result<(), String> {
    use atrament_handwriting_statistics::{
        Dimension, DimensionStatistics, HandwritingStatisticsProfile,
        ScalarStatistics,
    };
    use atrament_notebook_profile::NotebookProfile;
    use atrament_unicode_grapheme_segmentation::UnicodeGraphemeSegmentation;

    let source = FlowSource::new(FlowFormat::Markdown, "1. a")?;
    let region = super::FlowRegion {
        left: 30_000, top: 40_000, width: 22_000, height: 20_000,
    };
    // Positive 12-mm glyph contours with a 2-mm advance are valid
    // statistics. A nominal-width-only label test would admit these even
    // though the pen extends beyond a narrow 12-mm first-row cap.
    for glyph in ["1", "."] {
        let mut stats = HandwritingStatisticsProfile::baseline(61);
        for (dimension, mean) in [
            (Dimension::Width, 12_000),
            (Dimension::LetterSpacing, -10_000),
        ] {
            stats = stats.with_glyph(glyph, DimensionStatistics {
                dimension,
                statistics: ScalarStatistics {
                    minimum: mean, maximum: mean, mean, median: mean,
                    standard_deviation: 0,
                },
            }, &UnicodeGraphemeSegmentation)
                .map_err(|error| format!("{error:?}"))?;
        }
        let error = super::layout_flow_with_region(
            &source, &NotebookProfile::default(), Some(&stats),
            super::HeadingDecoration::default(), Some(region),
        ).expect_err("the marker contour exceeds the first row's cap");
        assert!(error.contains("ordered list label and first glyph exceed"),
            "refuse the whole label before partial {glyph:?} rows: {error}");

        let wider = super::FlowRegion { width: 27_000, ..region };
        let (rows, ornaments, pages) = super::layout_flow_with_region(
            &source, &NotebookProfile::default(), Some(&stats),
            super::HeadingDecoration::default(), Some(wider),
        )?;
        assert_eq!(pages, 1);
        assert!(ornaments.is_empty());
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].text, "1. a");
    }
    Ok(())
}

#[test]
fn compressed_heading_decoration_covers_actual_authored_contour()
    -> Result<(), String> {
    use atrament_handwriting_statistics::{
        Dimension, DimensionStatistics, HandwritingStatisticsProfile,
        ScalarStatistics,
    };
    use atrament_notebook_profile::NotebookProfile;
    use atrament_unicode_grapheme_segmentation::UnicodeGraphemeSegmentation;

    let mut stats = HandwritingStatisticsProfile::baseline(71);
    for (dimension, mean) in [
        (Dimension::Width, 12_000),
        (Dimension::LetterSpacing, -10_000),
    ] {
        stats = stats.with_glyph("a", DimensionStatistics {
            dimension,
            statistics: ScalarStatistics {
                minimum: mean, maximum: mean, mean, median: mean,
                standard_deviation: 0,
            },
        }, &UnicodeGraphemeSegmentation)
            .map_err(|error| format!("{error:?}"))?;
    }
    let paper = NotebookProfile::default();
    for style in [super::HeadingStyle::Underline,
        super::HeadingStyle::DoubleUnderline,
        super::HeadingStyle::Marker] {
        let heading = super::HeadingDecoration {
            style: Some(style), ..super::HeadingDecoration::default()
        };
        for (markdown, expected_extent) in [
            ("# aa", 14_000), ("aa\n===", 14_000),
            ("# a\u{f3}", 12_000),
        ] {
            let source = FlowSource::new(FlowFormat::Markdown, markdown)?;
            let (rows, marks, pages) = super::layout_flow(
                &source, &paper, Some(&stats), heading,
            )?;
            assert_eq!(pages, 1);
            assert_eq!(rows.len(), 1);
            let origin = rows[0].origin_um.0;
            assert_eq!(marks.len(), match style {
                super::HeadingStyle::Underline => 1,
                super::HeadingStyle::DoubleUnderline => 2,
                super::HeadingStyle::Marker => 4,
                super::HeadingStyle::None => 0,
            });
            assert!(marks.iter().all(|mark| {
                mark.start_um.0 == origin
                    && mark.end_um.0 == origin + expected_extent
            }), "{markdown:?} decoration must follow physical ink");
        }
    }
    Ok(())
}

#[test]
fn tall_compressed_headings_keep_page_local_contours_and_paragraph_gap()
-> Result<(), String> {
    use atrament_handwriting_statistics::{
        Dimension, DimensionStatistics, HandwritingStatisticsProfile,
        ScalarStatistics,
    };
    use atrament_notebook_profile::NotebookProfile;
    use atrament_unicode_grapheme_segmentation::UnicodeGraphemeSegmentation;

    let mut stats = HandwritingStatisticsProfile::baseline(97);
    for (glyph, dimension, mean) in [
        ("a", Dimension::Width, 12_000),
        ("a", Dimension::LetterSpacing, -10_000),
        ("\u{f3}", Dimension::Height, 20_000),
    ] {
        stats = stats
            .with_glyph(
                glyph,
                DimensionStatistics {
                    dimension,
                    statistics: ScalarStatistics {
                        minimum: mean,
                        maximum: mean,
                        mean,
                        median: mean,
                        standard_deviation: 0,
                    },
                },
                &UnicodeGraphemeSegmentation,
            )
            .map_err(|error| format!("{error:?}"))?;
    }
    // Three 24-mm rows fit on each sheet. The two opening rows leave
    // room for a title but not its first body row and 4-mm paragraph gap.
    let region = super::FlowRegion {
        left: 30_000,
        top: 40_000,
        width: 110_000,
        height: 80_000,
    };
    for style in [
        super::HeadingStyle::Underline,
        super::HeadingStyle::DoubleUnderline,
        super::HeadingStyle::Marker,
        super::HeadingStyle::None,
    ] {
        let heading = super::HeadingDecoration {
            style: Some(style),
            ..super::HeadingDecoration::default()
        };
        for source_text in
            ["a  \na  \n# aa\n\n\u{f3}", "a  \na  \naa\n===\n\n\u{f3}"]
        {
            let source = FlowSource::new(FlowFormat::Markdown, source_text)?;
            assert_eq!(super::measured_pitch(&source, Some(&stats))?, 24_000);
            let (rows, ornaments, pages) = super::layout_flow_with_region(
                &source,
                &NotebookProfile::default(),
                Some(&stats),
                heading,
                Some(region),
            )?;
            assert_eq!(pages, 2, "{source_text:?}");
            assert_eq!(
                rows.iter()
                    .map(|row| (row.page, row.origin_um, row.text.as_str(),))
                    .collect::<Vec<_>>(),
                [
                    (1, (32_500, 42_500), "a"),
                    (1, (32_500, 66_500), "a"),
                    (2, (32_500, 42_500), "aa"),
                    (2, (32_500, 70_500), "\u{f3}"),
                ]
            );
            let expected_widths: &[u32] = match style {
                super::HeadingStyle::Underline => &[550],
                super::HeadingStyle::DoubleUnderline => &[550, 550],
                super::HeadingStyle::Marker => &[1_400; 4],
                super::HeadingStyle::None => &[],
            };
            assert_eq!(
                ornaments
                    .iter()
                    .map(|mark| mark.width_um)
                    .collect::<Vec<_>>(),
                expected_widths
            );
            for mark in &ornaments {
                assert_eq!(mark.page, 2);
                assert_eq!(mark.start_um.0, 32_500);
                assert_eq!(
                    mark.end_um.0, 46_500,
                    "compressed advance must not truncate the contour"
                );
                assert!((40_000..120_000).contains(&mark.start_um.1));
            }
        }
    }
    Ok(())
}

#[test]
fn compressed_following_word_wraps_without_phantom_trailing_space()
-> Result<(), String> {
    use atrament_handwriting_statistics::{
        Dimension, DimensionStatistics, HandwritingStatisticsProfile,
        ScalarStatistics,
    };
    use atrament_notebook_profile::NotebookProfile;
    use atrament_unicode_grapheme_segmentation::UnicodeGraphemeSegmentation;

    let mut stats = HandwritingStatisticsProfile::baseline(107);
    for glyph in ["ó", "o\u{301}"] {
        for (dimension, mean) in [
            (Dimension::Width, 12_000),
            (Dimension::LetterSpacing, -10_000),
        ] {
            stats = stats.with_glyph(glyph, DimensionStatistics {
                dimension,
                statistics: ScalarStatistics {
                    minimum: mean, maximum: mean, mean, median: mean,
                    standard_deviation: 0,
                },
            }, &UnicodeGraphemeSegmentation)
                .map_err(|error| format!("{error:?}"))?;
        }
    }
    // Its 2-mm advance fits after "a ", but the 12-mm ink contour
    // requires a fresh row. No printable line should retain the separator.
    let region = super::FlowRegion {
        left: 30_000, top: 40_000, width: 22_000, height: 14_000,
    };
    for format in [FlowFormat::Plain, FlowFormat::Markdown] {
        let source = FlowSource::new(format, "a ó")?;
        let (runs, marks, pages) = super::layout_flow_with_region(
            &source, &NotebookProfile::default(), Some(&stats),
            super::HeadingDecoration::default(), Some(region),
        )?;
        assert_eq!(pages, 2);
        assert!(marks.is_empty());
        assert_eq!(runs.iter().map(|run| (run.page, run.text.as_str()))
            .collect::<Vec<_>>(), [(1, "a"), (2, "ó")]);
        assert_eq!(runs[0].origin_um, (32_500, 42_500));
        assert_eq!(runs[1].origin_um, (32_500, 42_500));

        // The two compressed accents also fit together on a fresh row.
        // Predicting only their 4-mm total advance would incorrectly place
        // the first accent after "a " and break the word in the middle.
        let word = FlowSource::new(format, "a óó")?;
        let (runs, _, pages) = super::layout_flow_with_region(
            &word, &NotebookProfile::default(), Some(&stats),
            super::HeadingDecoration::default(), Some(
                super::FlowRegion { width: 24_800, ..region }
            ),
        )?;
        assert_eq!(pages, 2);
        assert_eq!(runs.iter().map(|run| (run.page, run.text.as_str()))
            .collect::<Vec<_>>(), [(1, "a"), (2, "óó")]);

        // The first accent fits after "a " at 20.8 mm, exactly its
        // measured clearance boundary, without requiring a page turn.
        let (one, _, pages) = super::layout_flow_with_region(
            &source, &NotebookProfile::default(), Some(&stats),
            super::HeadingDecoration::default(), Some(
                super::FlowRegion { width: 24_800, ..region }
            ),
        )?;
        assert_eq!(pages, 1);
        assert_eq!(one.len(), 1);
        assert_eq!(one[0].text, "a ó");

        let decomposed = FlowSource::new(format,
            "a o\u{301}o\u{301}")?;
        let (nfd, _, nfd_pages) = super::layout_flow_with_region(
            &decomposed, &NotebookProfile::default(), Some(&stats),
            super::HeadingDecoration::default(), Some(
                super::FlowRegion { width: 24_800, ..region }
            ),
        )?;
        assert_eq!(nfd_pages, 2);
        assert_eq!(nfd.iter().map(|run| run.text.as_str())
            .collect::<Vec<_>>(), ["a", "o\u{301}o\u{301}"]);
    }

    // An ordered item reserves its measured numeral/space hang. The same
    // compressed second word must move intact to that continuation,
    // rather than leaving a trailing space after the first item glyph.
    let ordered = FlowSource::new(FlowFormat::Markdown, "1. a óó")?;
    let (runs, marks, pages) = super::layout_flow_with_region(
        &ordered, &NotebookProfile::default(), Some(&stats),
        super::HeadingDecoration::default(), Some(super::FlowRegion {
            width: 36_800, ..region
        }),
    )?;
    assert_eq!(pages, 2);
    assert!(marks.is_empty());
    assert_eq!(runs.iter().map(|run| (run.page, run.origin_um,
        run.text.as_str())).collect::<Vec<_>>(), [
        (1, (38_500, 42_500), "1. a"),
        (2, (47_600, 42_500), "óó"),
    ]);
    Ok(())
}

#[test]
fn measured_pitch_matches_emitted_ink_for_mixed_sources()
-> Result<(), String> {
    use atrament_handwriting_statistics::{
        Dimension, DimensionStatistics, HandwritingStatisticsProfile,
        ScalarStatistics,
    };
    use atrament_notebook_profile::NotebookProfile;
    use atrament_unicode_grapheme_segmentation::UnicodeGraphemeSegmentation;

    let mut stats = HandwritingStatisticsProfile::baseline(3);
    for (glyph, height) in [
        ("#", 18_000), (">", 19_000), ("=", 17_000),
        ("-", 16_000), ("*", 15_000), ("~", 14_000),
        ("`", 13_000), ("1", 12_000), (".", 11_000),
        ("a", 10_000), ("ó", 20_000), ("\\", 9_000),
        ("o\u{301}", 21_000),
    ] {
        stats = stats.with_glyph(
            glyph,
            DimensionStatistics {
                dimension: Dimension::Height,
                statistics: ScalarStatistics {
                    minimum: height,
                    maximum: height,
                    mean: height,
                    median: height,
                    standard_deviation: 0,
                },
            },
            &UnicodeGraphemeSegmentation,
        ).map_err(|error| format!("{error:?}"))?;
    }

    // Deliberately interleave syntax-only and visible tall graphemes.
    // Compare the pitch preflight to the independently parsed ink runs:
    // formatting syntax must not influence the physical row height.
    let examples = [
        "a", "ó", "o\u{301}", "# a", "## ó", "- a", "1. a", "> a",
        "  a", "    a", "  ó", "- ó", "  - a", "   ó",
        "===", "---", "***", "~~~", "```", "```rust",
        "# ó", "ó\\", "a  ", "a\\", "", "  ",
        "  ===", "  ---", "\\===", "\\# a", "1. ó",
        "- [x] a", "- [ ] ó", "> ó", "  > a",
        "\tó", "a\u{2028}ó", "a\u{2029}ó", "### a",
        "a\\  ", "# a ###", "```\nó\n```",
    ];
    let example_count = u64::try_from(examples.len())
        .map_err(|_| "source example count refused")?;
    let profile = NotebookProfile::default();
    let mut seed = 0x9e37_79b9_7f4a_7c15_u64;
    let mut admitted = 0;
    for case in 0..6_000 {
        let line_count = 2 + (case % 8);
        let mut source_lines = Vec::new();
        for _ in 0..line_count {
            seed ^= seed << 13;
            seed ^= seed >> 7;
            seed ^= seed << 17;
            let index = usize::try_from(seed % example_count)
                .map_err(|_| "source index refused")?;
            source_lines.push(examples[index]);
        }
        let raw = source_lines.join("\n");
        let format = if case % 4 == 0 {
            FlowFormat::Plain
        } else {
            FlowFormat::Markdown
        };
        let Ok(source) = FlowSource::new(format, &raw) else {
            continue;
        };
        let Ok((runs, _, _)) = super::layout_flow(
            &source, &profile, Some(&stats),
            super::HeadingDecoration::default(),
        ) else {
            continue;
        };
        let pitch = super::measured_pitch(&source, Some(&stats))?;
        let mut emitted_pitch = 9_000_u32;
        for run in &runs {
            let mut remaining = run.text.as_str();
            while !remaining.is_empty() {
                let (grapheme, rest) = super::first_grapheme(remaining)?;
                remaining = rest;
                if grapheme == " " {
                    continue;
                }
                let height = stats.statistics_for(
                    grapheme, Dimension::Height,
                ).ok_or("missing glyph height statistics")?.mean;
                let baseline = stats.statistics_for(
                    grapheme, Dimension::BaselineOffset,
                ).ok_or("missing glyph baseline statistics")?.mean;
                let needed = height.checked_add(baseline.abs())
                    .and_then(|sum| sum.checked_add(4_000))
                    .ok_or("emitted pitch overflow")?;
                emitted_pitch = emitted_pitch.max(u32::try_from(needed)
                    .map_err(|_| "emitted pitch exceeds u32")?);
            }
        }
        assert_eq!(pitch, emitted_pitch,
            "source {case} ({format:?}): {raw:?}; ink: {runs:?}");
        admitted += 1;
    }
    assert!(admitted > 1_000, "too few admitted source sequences");
    Ok(())
}

// Independently traverse accepted row ink, without using the word wrapper's
// preflight calculation. A compressed advance is not the contour edge.
fn nominal_row_right_clearance(
    run: &super::TextRun,
    stats: &atrament_handwriting_statistics::HandwritingStatisticsProfile,
    right: u32,
) -> Result<(), String> {
    let mut cursor = run.origin_um.0;
    let mut remaining = run.text.as_str();
    while !remaining.is_empty() {
        let (grapheme, rest) = super::first_grapheme(remaining)?;
        remaining = rest;
        let (advance, contour) =
            super::GlyphOverlay::flow_advance_um(grapheme, Some(stats))?;
        let furthest = cursor.checked_add(advance.max(
            contour.checked_add(800)
                .ok_or("nominal contour clearance overflow")?,
        )).ok_or("nominal contour position overflow")?;
        if furthest > right {
            return Err(format!(
                "{run:?}, {grapheme:?}: edge {furthest} beyond {right}"
            ));
        }
        cursor = cursor.checked_add(advance)
            .ok_or("nominal glyph advance overflow")?;
    }
    Ok(())
}

#[test]
fn compressed_mixed_flow_keeps_every_nominal_contour_inside_its_region()
-> Result<(), String> {
    use atrament_handwriting_statistics::{
        Dimension, DimensionStatistics, HandwritingStatisticsProfile,
        ScalarStatistics,
    };
    use atrament_notebook_profile::NotebookProfile;
    use atrament_unicode_grapheme_segmentation::UnicodeGraphemeSegmentation;

    let mut stats = HandwritingStatisticsProfile::baseline(91);
    for glyph in ["a", "\u{f3}", "o\u{301}", "1", ".", "2", ")"] {
        for (dimension, value) in [
            (Dimension::Width, 12_000i32),
            (Dimension::LetterSpacing, -10_000i32),
        ] {
            stats = stats.with_glyph(glyph, DimensionStatistics {
                dimension,
                statistics: ScalarStatistics {
                    minimum: value, maximum: value, mean: value,
                    median: value, standard_deviation: 0,
                },
            }, &UnicodeGraphemeSegmentation)
                .map_err(|error| format!("{error:?}"))?;
        }
    }
    let snippets = [
        "a \u{f3}", "\u{f3}\u{f3}", "o\u{301}o\u{301}", "a a",
        "1. a \u{f3}", "12) a \u{f3}", "- a \u{f3}",
        "> a \u{f3}", "# a \u{f3}", "a  \u{f3}",
        "a\\", "  \u{f3}",
        "  a", "    a", "## a", "- [x] \u{f3}", "a\u{2028}\u{f3}",
        "---", "===", "```,", "```", "\u{f3} a \u{f3}", "", "\t\u{f3}",
    ];
    let region = super::FlowRegion {
        left: 30_000u32, top: 40_000u32,
        width: 55_000u32, height: 35_000u32,
    };
    let right = region.left + region.width - super::EXTRA_RIGHT_UM;
    let paper = NotebookProfile::default();
    let mut rng_seed = 0x1234_5678u64;
    let snippet_count = u64::try_from(snippets.len())
        .map_err(|error| format!(
            "snippet count exceeds u64: {error}"
        ))?;
    let mut admitted = 0usize;
    let mut second_page_rows = 0usize;
    let mut indented_rows = 0usize;
    for index in 0usize..8_000usize {
        let mut source_lines = Vec::new();
        for _ in 0..(2usize + index.rem_euclid(5)) {
            rng_seed ^= rng_seed << 13u32;
            rng_seed ^= rng_seed >> 7u32;
            rng_seed ^= rng_seed << 17u32;
            let position = usize::try_from(
                rng_seed.rem_euclid(snippet_count),
            ).map_err(|error| format!(
                "snippet index exceeds usize: {error}"
            ))?;
            source_lines.push(*snippets.get(position)
                .ok_or("snippet index out of bounds")?);
        }
        let raw = source_lines.join("\n");
        let format = if index.rem_euclid(4) == 0 {
            FlowFormat::Plain
        } else {
            FlowFormat::Markdown
        };
        let Ok(source) = FlowSource::new(format, &raw) else {
            continue;
        };
        let Ok((runs, _, _)) = super::layout_flow_with_region(
            &source, &paper, Some(&stats),
            super::HeadingDecoration::default(), Some(region),
        ) else {
            continue;
        };
        for run in runs {
            if run.page > 1 {
                second_page_rows += 1;
            }
            if run.origin_um.0 > region.left + super::INSET_UM {
                indented_rows += 1;
            }
            nominal_row_right_clearance(&run, &stats, right).map_err(
                |error| format!("case {index}, {raw:?}, {format:?}: {error}"),
            )?;
        }
        admitted += 1;
    }
    if admitted <= 1_000 || second_page_rows == 0 || indented_rows == 0 {
        return Err(format!(
            "insufficient accepted compressed flow coverage: \
             {admitted} sources, {second_page_rows} page-two rows, \
             {indented_rows} indented rows"
        ));
    }
    Ok(())
}

// A successful plain-text layout may skip whitespace at physical word-wrap
// boundaries, but must neither lose nor duplicate any authored visible
// grapheme. Check the admitted, tab-expanded input independently of the
// page-run writer, including multi-codepoint Unicode and compressed advances.
#[test]
fn paged_plain_flow_preserves_every_visible_grapheme_in_source_order()
-> Result<(), String> {
    use atrament_handwriting_statistics::{
        Dimension, DimensionStatistics, HandwritingStatisticsProfile,
        ScalarStatistics,
    };
    use atrament_notebook_profile::NotebookProfile;
    use atrament_unicode_grapheme_segmentation::UnicodeGraphemeSegmentation;

    let mut stats = HandwritingStatisticsProfile::baseline(117);
    for glyph in ["a", "ó", "o\u{301}", "👩‍🔬"] {
        for (dimension, value) in [
            (Dimension::Width, 12_000),
            (Dimension::LetterSpacing, -10_000),
        ] {
            stats = stats.with_glyph(glyph, DimensionStatistics {
                dimension,
                statistics: ScalarStatistics {
                    minimum: value, maximum: value, mean: value,
                    median: value, standard_deviation: 0,
                },
            }, &UnicodeGraphemeSegmentation)
                .map_err(|error| format!("{error:?}"))?;
        }
    }
    let snippets = [
        "a ó", "o\u{301}a", "👩‍🔬 a", "a  a", " ó ", "\tó",
        "o\u{301}👩‍🔬", "   a", "    a", "# a", "> ó", "a\u{2028}ó",
        "ó\u{2029}a", "a\t👩‍🔬", "", "   ", "óóóó", "á", "ñ",
    ];
    let region = super::FlowRegion {
        left: 30_000, top: 40_000, width: 28_000, height: 32_000,
    };
    let mut seed = 0x83ac_67e2_dafe_9017_u64;
    let snippet_count = u64::try_from(snippets.len())
        .map_err(|_| "source snippet count overflow")?;
    let mut admitted = 0usize;
    let mut paged = 0usize;
    let mut compound = 0usize;
    for case in 0..6_000usize {
        let mut selected = Vec::new();
        for _ in 0..(2 + case % 7) {
            seed ^= seed << 13;
            seed ^= seed >> 7;
            seed ^= seed << 17;
            let index = usize::try_from(seed % snippet_count)
                .map_err(|_| "source snippet index overflow")?;
            selected.push(snippets[index]);
        }
        let raw = selected.join("\n");
        let Ok(source) = FlowSource::new(FlowFormat::Plain, &raw) else {
            continue;
        };
        let Ok((runs, ornaments, pages)) = super::layout_flow_with_region(
            &source, &NotebookProfile::default(), Some(&stats),
            super::HeadingDecoration::default(), Some(region),
        ) else {
            continue;
        };
        assert!(ornaments.is_empty(), "plain text cannot add ornaments");
        let authored = visible_graphemes(&source.text)?;
        let emitted = runs.iter().map(|run| run.text.as_str())
            .collect::<Vec<_>>().join("\n");
        assert_eq!(visible_graphemes(&emitted)?, authored,
            "case {case}, source {raw:?}, pages {pages}, runs {runs:?}");
        if pages > 1 {
            paged += 1;
        }
        if authored.iter().any(|g| *g == "o\u{301}" || *g == "👩‍🔬") {
            compound += 1;
        }
        admitted += 1;
    }
    assert!(admitted > 1_000 && paged > 500 && compound > 500,
        "coverage: {admitted} admitted, {paged} paged, {compound} compound");
    Ok(())
}

#[test]
fn empty_quote_separator_keeps_visible_paragraphs_on_physical_pages()
-> Result<(), String> {
    use atrament_notebook_profile::NotebookProfile;

    let region = super::FlowRegion {
        left: 30_000, top: 40_000, width: 55_000, height: 15_000,
    };
    let render = |text| -> Result<_, String> {
        let source = FlowSource::new(FlowFormat::Markdown, text)?;
        super::layout_flow_with_region(
            &source, &NotebookProfile::default(), None,
            super::HeadingDecoration::default(), Some(region),
        )
    };
    let separated = render("> a\n>\n> ó")?;
    let repeated = render("> a\n>\n> \n  >\n> \u{f3}")?;
    let many_source = format!("> a\n{}> \u{f3}", ">\n".repeat(100));
    let many = render(&many_source)?;
    let blank = render("> a\n\n> ó")?;
    assert_eq!(separated.0.len(), 2);
    assert_eq!(separated.2, 2);
    assert_eq!(separated.0.iter().map(|row| (
        row.text.as_str(), row.origin_um, row.page,
    )).collect::<Vec<_>>(), [
        ("a", (38_500, 42_500), 1),
        ("ó", (38_500, 42_500), 2),
    ]);
    assert_eq!(separated.0.iter().map(|row| (
        row.text.as_str(), row.origin_um, row.page,
    )).collect::<Vec<_>>(), blank.0.iter().map(|row| (
        row.text.as_str(), row.origin_um, row.page,
    )).collect::<Vec<_>>());
    assert_eq!(separated.1.len(), 2);
    assert_eq!(separated.1, blank.1);
    let row_geometry = |runs: &[super::TextRun]| runs.iter()
        .map(|run| (run.page, run.origin_um, run.text.clone()))
        .collect::<Vec<_>>();
    assert_eq!(row_geometry(&repeated.0), row_geometry(&separated.0));
    assert_eq!(repeated.1, separated.1);
    assert_eq!(repeated.2, separated.2);
    assert_eq!(row_geometry(&many.0), row_geometry(&separated.0));
    assert_eq!(many.1, separated.1);
    assert_eq!(many.2, separated.2);
    for source in [">", ">\n> a", "> a\n>",
        "> a\n>\na", "> a\n>\n>\na",
        "> a\n>\n>\n> ", "> a\n>\n>\n- a"] {
        let error = render(source)
            .expect_err("empty quote run without quoted continuation refused");
        assert!(error.contains("empty Markdown block"), "{source:?}: {error}");
    }
    Ok(())
}

#[test]
fn consecutive_empty_quote_markers_do_not_stack_visible_paragraph_gaps()
-> Result<(), String> {
    use atrament_notebook_profile::NotebookProfile;

    fn geometry(runs: &[super::TextRun])
    -> Vec<(usize, (u32, u32), &str)> {
        runs.iter().map(|row| (
            row.page, row.origin_um, row.text.as_str(),
        )).collect::<Vec<_>>()
    }
    let region = super::FlowRegion {
        left: 30_000, top: 40_000, width: 55_000, height: 40_000,
    };
    let render = |text| -> Result<_, String> {
        let source = FlowSource::new(FlowFormat::Markdown, text)?;
        super::layout_flow_with_region(
            &source, &NotebookProfile::default(), None,
            super::HeadingDecoration::default(), Some(region),
        )
    };
    let reference = render("> a\n\n> \u{f3}")?;
    if reference.2 != 1 || geometry(&reference.0) != [
        (1, (38_500, 42_500), "a"),
        (1, (38_500, 55_500), "\u{f3}"),
    ] {
        return Err("single quote gap does not fit at expected rows".into());
    }
    let many = format!("> a\n{}> \u{f3}", ">\n".repeat(100));
    let alternatives = [
        "> a\n>\n> \u{f3}", "> a\n>\n>\n>\n> \u{f3}",
        "> a\n> \n  >\n> \u{f3}", "> a\n\n>\n> \u{f3}",
        "> a\r\n>\r\n>\r\n> \u{f3}",
    ];
    for source in alternatives.into_iter().chain([many.as_str()]) {
        let actual = render(source)?;
        if geometry(&actual.0) != geometry(&reference.0)
            || actual.1 != reference.1 || actual.2 != reference.2 {
            return Err(format!(
                "quote delimiters must collapse to one physical gap: {source:?}"
            ));
        }
    }
    Ok(())
}

#[test]
fn wrapped_list_marker_stays_with_first_continuation_when_page_has_room()
-> Result<(), String> {
    use atrament_notebook_profile::NotebookProfile;

    let region = super::FlowRegion {
        left: 30_000, top: 40_000, width: 35_000, height: 23_000,
    };
    for (list, continuation) in [
        ("- aaaaaa aaaaaa", "aaaaaa"),
        ("12) aa aa", "aa"),
        ("- [x] aaaaaa aaaaaa", "aaaaaa"),
        ("- aaaaaaa", "a"),
        ("12) aaa", "a"),
        ("- [x] aaaaaaa", "a"),
    ] {
        let source = FlowSource::new(
            FlowFormat::Markdown, &format!("p\n{list}"),
        )?;
        let render = |height| -> Result<_, String> {
            super::layout_flow_with_region(
                &source, &NotebookProfile::default(), None,
                super::HeadingDecoration::default(),
                Some(super::FlowRegion { height, ..region }),
            )
        };
        let (rows, marks, pages) = render(region.height)?;
        assert_eq!(pages, 2, "{list}");
        assert_eq!(rows.len(), 3, "{list}");
        assert_eq!(rows.iter().map(|row| (
            row.page, row.origin_um.1,
        )).collect::<Vec<_>>(), [
            (1, 42_500), (2, 42_500), (2, 51_500),
        ], "{list}");
        assert_eq!(rows[2].text, continuation, "{list}");
        assert!(marks.iter().all(|mark| mark.page == 2), "{list}");

        // A page with three rows fits the source and its full marker;
        // the two-row guard must not introduce a gratuitous page turn.
        let (fitting, _, fitting_pages) = render(32_000)?;
        assert_eq!(fitting_pages, 1, "{list}");
        assert_eq!(fitting.iter().map(|row| row.page)
            .collect::<Vec<_>>(), [1, 1, 1], "{list}");

        // A one-row page cannot hold two rows. Ordinary pagination must
        // still admit this item instead of refusing it as impossible.
        let (single, _, single_pages) = render(14_000)?;
        assert_eq!(single_pages, 3, "{list}");
        assert_eq!(single.iter().map(|row| row.page)
            .collect::<Vec<_>>(), [1, 2, 3], "{list}");
    }
    Ok(())
}

#[test]
fn heading_first_row_guard_takes_priority_over_list_continuation_guard()
-> Result<(), String> {
    use atrament_notebook_profile::NotebookProfile;

    for (prefix, height, pages) in [
        ("", 23_000, &[1, 1, 2][..]),
        ("a  \n", 32_000, &[1, 2, 2, 2][..]),
    ] {
        let source = FlowSource::new(
            FlowFormat::Markdown,
            &format!("{prefix}# a\n- aaaaaa aaaaaa"),
        )?;
        let (rows, ornaments, total) = super::layout_flow_with_region(
            &source, &NotebookProfile::default(), None,
            super::HeadingDecoration::default(),
            Some(super::FlowRegion {
                left: 30_000, top: 40_000,
                width: 35_000, height,
            }),
        )?;
        assert_eq!(total, 2);
        assert_eq!(rows.iter().map(|run| run.page)
            .collect::<Vec<_>>(), pages, "{prefix:?}");
        let heading_page = if prefix.is_empty() { 1 } else { 2 };
        assert!(ornaments.iter().all(|mark| mark.page == heading_page),
            "heading and bullet marks follow their authored block");
    }
    Ok(())
}

fn visible_graphemes(text: &str) -> Result<Vec<&str>, String> {
    let mut result = Vec::new();
    let mut remaining = text;
    while !remaining.is_empty() {
        let (grapheme, next) = super::first_grapheme(remaining)?;
        if grapheme != " " && grapheme != "\n" {
            result.push(grapheme);
        }
        remaining = next;
    }
    Ok(result)
}

#[test]
fn wrapped_quote_rules_stay_with_first_continuation_when_page_has_room()
-> Result<(), String> {
    use atrament_notebook_profile::NotebookProfile;

    let region = super::FlowRegion {
        left: 30_000, top: 40_000, width: 35_000, height: 23_000,
    };
    for source in ["p\n> aaaaaa aaaaaa", "p\n> aaaaaaa"] {
        let input = FlowSource::new(FlowFormat::Markdown, source)?;
        let render = |height| -> Result<_, String> {
            super::layout_flow_with_region(
                &input, &NotebookProfile::default(), None,
                super::HeadingDecoration::default(),
                Some(super::FlowRegion { height, ..region }),
            )
        };
        let (rows, rules, pages) = render(region.height)?;
        assert_eq!(pages, 2, "{source:?}");
        assert_eq!(rows.iter().map(|run| (run.page, run.origin_um))
            .collect::<Vec<_>>(), [
                (1, (32_500, 42_500)),
                (2, (38_500, 42_500)),
                (2, (38_500, 51_500)),
            ], "{source:?}");
        assert_eq!(rules.iter().map(|rule| (
            rule.page, rule.start_um, rule.end_um,
        )).collect::<Vec<_>>(), [
            (2, (35_500, 43_000), (35_500, 50_900)),
            (2, (35_500, 52_000), (35_500, 59_900)),
        ], "quote rule stays with each physical text row");

        let (fitting, _, fit_pages) = render(32_000)?;
        assert_eq!(fit_pages, 1);
        assert_eq!(fitting.iter().map(|row| row.page)
            .collect::<Vec<_>>(), [1, 1, 1]);

        let (one_row, _, single_pages) = render(14_000)?;
        assert_eq!(single_pages, 3);
        assert_eq!(one_row.iter().map(|row| row.page)
            .collect::<Vec<_>>(), [1, 2, 3]);
    }
    Ok(())
}

#[test]
fn heading_first_row_guard_takes_priority_over_quote_continuation_guard()
-> Result<(), String> {
    use atrament_notebook_profile::NotebookProfile;

    for (prefix, height, pages) in [
        ("", 23_000, &[1, 1, 2][..]),
        ("a  \n", 32_000, &[1, 2, 2, 2][..]),
    ] {
        let source = FlowSource::new(
            FlowFormat::Markdown,
            &format!("{prefix}# a\n> aaaaaa aaaaaa"),
        )?;
        let (rows, ornaments, total) = super::layout_flow_with_region(
            &source, &NotebookProfile::default(), None,
            super::HeadingDecoration::default(),
            Some(super::FlowRegion {
                left: 30_000, top: 40_000,
                width: 35_000, height,
            }),
        )?;
        assert_eq!(total, 2);
        assert_eq!(rows.iter().map(|run| run.page)
            .collect::<Vec<_>>(), pages, "{prefix:?}");
        let page_one_marks = if prefix.is_empty() { 2 } else { 0 };
        assert_eq!(ornaments.iter().filter(|mark| mark.page == 1).count(),
            page_one_marks, "heading and quote ink share a page");
        assert_eq!(ornaments.iter().filter(|mark| mark.page == 2).count(),
            3 - page_one_marks, "quote rules follow their body rows");
    }
    Ok(())
}

#[test]
fn wrapped_markdown_paragraph_starter_keeps_its_second_physical_row()
-> Result<(), String> {
    use atrament_notebook_profile::NotebookProfile;

    let region = super::FlowRegion {
        left: 30_000, top: 40_000, width: 35_000, height: 23_000,
    };
    for source in ["a  \naaaaaa aaaaaa", "a  \naaaaaaaa"] {
        let input = FlowSource::new(FlowFormat::Markdown, source)?;
        let render = |height| -> Result<_, String> {
            super::layout_flow_with_region(
                &input, &NotebookProfile::default(), None,
                super::HeadingDecoration::default(),
                Some(super::FlowRegion { height, ..region }),
            )
        };
        let (rows, marks, pages) = render(region.height)?;
        assert_eq!(pages, 2, "{source:?}");
        assert!(marks.is_empty());
        assert_eq!(rows.iter().map(|row| (row.page, row.origin_um.1))
            .collect::<Vec<_>>(), [
                (1, 42_500), (2, 42_500), (2, 51_500),
            ], "{source:?}");
        let (fitting, _, fit_pages) = render(32_000)?;
        assert_eq!(fit_pages, 1);
        assert_eq!(fitting.iter().map(|row| row.page)
            .collect::<Vec<_>>(), [1, 1, 1]);
        let (single, _, single_pages) = render(14_000)?;
        assert_eq!(single_pages, 3);
        assert_eq!(single.iter().map(|row| row.page)
            .collect::<Vec<_>>(), [1, 2, 3]);
    }

    // A heading wins the first-row reservation. A code fence and plain
    // source lines are literal rows rather than Markdown paragraphs.
    for (source, format, expected) in [
        ("# a\naaaaaa aaaaaa", FlowFormat::Markdown, [1, 1, 2]),
        ("a\n```\naaaaaa aaaaaa\n```", FlowFormat::Markdown,
            [1, 1, 2]),
        ("a\naaaaaa aaaaaa", FlowFormat::Plain, [1, 1, 2]),
    ] {
        let input = FlowSource::new(format, source)?;
        let (rows, _, pages) = super::layout_flow_with_region(
            &input, &NotebookProfile::default(), None,
            super::HeadingDecoration::default(), Some(region),
        )?;
        assert_eq!(pages, 2);
        assert_eq!(rows.iter().map(|row| row.page)
            .collect::<Vec<_>>(), expected, "{source:?}");
    }
    Ok(())
}

#[test]
fn thematic_rule_stays_with_following_markdown_ink_when_sheet_has_room()
-> Result<(), String> {
    use atrament_notebook_profile::NotebookProfile;

    let region = super::FlowRegion {
        left: 30_000, top: 40_000, width: 35_000, height: 32_000,
    };
    let render = |source: &str, height| -> Result<_, String> {
        super::layout_flow_with_region(
            &FlowSource::new(FlowFormat::Markdown, source)?,
            &NotebookProfile::default(), None,
            super::HeadingDecoration::default(),
            Some(super::FlowRegion { height, ..region }),
        )
    };
    let (rows, rules, pages) = render("a\n\n---\na", region.height)?;
    assert_eq!(pages, 2);
    assert_eq!(rows.iter().map(|row| (row.page, row.origin_um))
        .collect::<Vec<_>>(), [
            (1, (32_500, 42_500)), (2, (32_500, 51_500)),
        ]);
    assert_eq!(rules.iter().map(|rule| (
        rule.page, rule.start_um, rule.end_um,
    )).collect::<Vec<_>>(), [
        (2, (32_500, 47_000), (63_500, 47_000)),
    ], "a thematic rule must not orphan its next visible row");

    // An ending rule has no following ink to keep, and must stay on
    // the already fitting page instead of forcing a gratuitous turn.
    let (ending, ending_rules, ending_pages) =
        render("a\n\n---", region.height)?;
    assert_eq!(ending_pages, 1);
    assert_eq!(ending.iter().map(|row| row.page)
        .collect::<Vec<_>>(), [1]);
    assert_eq!(ending_rules[0].page, 1);

    // Collapsed Markdown gaps and literal fenced-code blank rows must
    // count before reserving the next printable row after a rule.
    for (source, expected_y) in [
        ("a\n\n---\n\na", 55_500),
        ("a\n\n---\n```\n\na\n```", 60_500),
    ] {
        let (code_rows, code_rules, code_pages) =
            render(source, region.height)?;
        assert_eq!(code_pages, 2, "{source:?}");
        assert_eq!(code_rows.iter().map(|row| (row.page, row.origin_um.1))
            .collect::<Vec<_>>(), [(1, 42_500), (2, expected_y)],
            "{source:?}");
        assert_eq!(code_rules[0].page, 2, "{source:?}");
    }

    // A preceding heading keeps its first following rule; the rule may
    // not detach that pair to favor its own next-row association.
    let (heading, marks, heading_pages) = render("# a\n---\na", 23_000)?;
    assert_eq!(heading_pages, 2);
    assert_eq!(heading.iter().map(|row| row.page)
        .collect::<Vec<_>>(), [1, 2]);
    assert_eq!(marks.iter().map(|mark| mark.page)
        .collect::<Vec<_>>(), [1, 1]);

    // A single-row sheet cannot hold a rule and another ink row.
    let (single, single_marks, single_pages) =
        render("a\n\n---\na", 14_000)?;
    assert_eq!(single_pages, 3);
    assert_eq!(single.iter().map(|row| row.page)
        .collect::<Vec<_>>(), [1, 3]);
    assert_eq!(single_marks[0].page, 2);
    Ok(())
}

#[test]
fn thematic_rule_pair_takes_priority_over_following_block_orphan_guards()
-> Result<(), String> {
    use atrament_notebook_profile::NotebookProfile;

    for (source, pages) in [
        ("a\n\n---\naaaaaa aaaaaa", [1, 2, 3]),
        ("a\n\n---\n- aaaaaa aaaaaa", [1, 2, 3]),
        ("a\n\n---\n> aaaaaa aaaaaa", [1, 2, 3]),
        ("a\n\n---\n# a\nb", [1, 3, 3]),
        ("a\n\n---\na\n===\nb", [1, 3, 3]),
    ] {
        let input = FlowSource::new(FlowFormat::Markdown, source)?;
        let (rows, rules, count) = super::layout_flow_with_region(
            &input, &NotebookProfile::default(), None,
            super::HeadingDecoration::default(),
            Some(super::FlowRegion {
                left: 30_000, top: 40_000,
                width: 35_000, height: 23_000,
            }),
        )?;
        assert_eq!(count, 3, "{source:?}");
        assert_eq!(rows.iter().map(|row| row.page)
            .collect::<Vec<_>>(), pages, "{source:?}");
        assert_eq!(rules[0].page, 2, "{source:?}");
        if source == "a\n\n---\naaaaaa aaaaaa" {
            assert_eq!(rows.iter().map(|row| (row.page, row.origin_um))
                .collect::<Vec<_>>(), [
                    (1, (32_500, 42_500)),
                    (2, (32_500, 51_500)),
                    (3, (32_500, 42_500)),
                ]);
            assert_eq!(rules[0].start_um, (32_500, 47_000));
            assert_eq!(rules[0].end_um, (63_500, 47_000));
        }
    }

    // With enough room for the whole rule/title/body span, the same
    // headings stay on the reserved second sheet rather than moving.
    for source in ["a\n\n---\n# a\nb", "a\n\n---\na\n===\nb"] {
        let input = FlowSource::new(FlowFormat::Markdown, source)?;
        let (rows, rules, count) = super::layout_flow_with_region(
            &input, &NotebookProfile::default(), None,
            super::HeadingDecoration::default(),
            Some(super::FlowRegion {
                left: 30_000, top: 40_000,
                width: 35_000, height: 32_000,
            }),
        )?;
        assert_eq!(count, 2, "{source:?}");
        assert_eq!(rows.iter().map(|row| row.page)
            .collect::<Vec<_>>(), [1, 2, 2], "{source:?}");
        assert_eq!(rules[0].page, 2, "{source:?}");
    }

    // The first rule owns its following row even when that row is a
    // second rule; the second rule may not pull away from the first.
    let input = FlowSource::new(FlowFormat::Markdown,
        "a\n\n---\n---\na")?;
    let (rows, rules, count) = super::layout_flow_with_region(
        &input, &NotebookProfile::default(), None,
        super::HeadingDecoration::default(),
        Some(super::FlowRegion {
            left: 30_000, top: 40_000,
            width: 35_000, height: 23_000,
        }),
    )?;
    assert_eq!(count, 3);
    assert_eq!(rules.iter().map(|rule| rule.page)
        .collect::<Vec<_>>(), [2, 2]);
    assert_eq!(rows.iter().map(|row| row.page)
        .collect::<Vec<_>>(), [1, 3]);
    Ok(())
}

#[test]
fn long_markdown_body_balances_a_lone_final_row_when_previous_page_has_three()
-> Result<(), String> {
    use atrament_notebook_profile::NotebookProfile;

    let render = |source: &str, format, height| -> Result<_, String> {
        super::layout_flow_with_region(
            &FlowSource::new(format, source)?,
            &NotebookProfile::default(), None,
            super::HeadingDecoration::default(),
            Some(super::FlowRegion {
                left: 30_000, top: 40_000, width: 35_000, height,
            }),
        )
    };
    for (count, expected) in [
        (4, vec![1, 1, 2, 2]),
        (5, vec![1, 1, 1, 2, 2]),
        (7, vec![1, 1, 1, 2, 2, 3, 3]),
    ] {
        let source = vec!["aaaaaa"; count].join(" ");
        let (rows, marks, pages) = render(&source, FlowFormat::Markdown,
            32_000)?;
        assert_eq!(rows.len(), count);
        assert!(marks.is_empty());
        assert_eq!(pages, *expected.last().unwrap_or(&0));
        assert_eq!(rows.iter().map(|run| run.page).collect::<Vec<_>>(),
            expected, "{count} wrapped rows");
        for page in 1..=pages {
            let origins = rows.iter().filter(|row| row.page == page)
                .map(|row| row.origin_um.1).collect::<Vec<_>>();
            assert_eq!(origins,
                (0..origins.len()).map(|index| 42_500 + index as u32
                    * 9_000).collect::<Vec<_>>());
        }
    }

    // A two-row sheet cannot leave two rows on the prior sheet and
    // move two to the new sheet; retain its original physical splits.
    let five = vec!["aaaaaa"; 5].join(" ");
    let (short, _, short_pages) = render(&five, FlowFormat::Markdown,
        23_000)?;
    assert_eq!(short_pages, 3);
    assert_eq!(short.iter().map(|row| row.page).collect::<Vec<_>>(),
        [1, 1, 2, 2, 3]);

    // Plain physical lines and literal fenced Markdown rows never gain
    // this paragraph-tail optimization.
    for (source, format) in [
        (vec!["aaaaaa"; 4].join("\n"), FlowFormat::Plain),
        (format!("```\n{}\n```", vec!["aaaaaa"; 4].join("\n")),
            FlowFormat::Markdown),
    ] {
        let (rows, _, pages) = render(&source, format, 32_000)?;
        assert_eq!(pages, 2);
        assert_eq!(rows.iter().map(|row| row.page).collect::<Vec<_>>(),
            [1, 1, 1, 2], "{source:?}");
    }

    // A single long word follows the same rule after grapheme wrapping;
    // no punctuation or grapheme may disappear in the moved last pair.
    let long_word = "a".repeat(22);
    let (word_rows, _, word_pages) = render(&long_word,
        FlowFormat::Markdown, 32_000)?;
    assert_eq!(word_pages, 2);
    assert_eq!(word_rows.iter().map(|row| (row.page, row.text.len()))
        .collect::<Vec<_>>(), [(1, 7), (1, 7), (2, 7), (2, 1)]);
    assert_eq!(word_rows.iter().map(|row| row.text.as_str())
        .collect::<String>(), long_word);

    // A later paragraph starts at the real repositioned cursor rather
    // than overwriting the second row on the rebalanced last sheet.
    let with_following = format!("{}\n\na", vec!["aaaaaa"; 4].join(" "));
    let (continued, _, continued_pages) = render(&with_following,
        FlowFormat::Markdown, 32_000)?;
    assert_eq!(continued_pages, 3);
    assert_eq!(continued.iter().map(|row| row.page).collect::<Vec<_>>(),
        [1, 1, 2, 2, 3]);

    // Decorated quote/list blocks share the same balanced physical
    // tail as body paragraphs; ornament placement is checked below.
    for source in [
        format!("- {}", vec!["aaaaaa"; 4].join(" ")),
        format!("> {}", vec!["aaaaaa"; 4].join(" ")),
    ] {
        let (rows, marks, pages) = render(&source,
            FlowFormat::Markdown, 32_000)?;
        assert_eq!(pages, 2);
        assert_eq!(rows.iter().map(|row| row.page).collect::<Vec<_>>(),
            [1, 1, 2, 2], "{source:?}");
        assert!(!marks.is_empty());
    }

    // A heading retains its first following body row. Later rows may
    // rebalance without changing the heading or its underline.
    let source = format!("# a\n{}", vec!["aaaaaa"; 6].join(" "));
    let (rows, marks, pages) = render(&source, FlowFormat::Markdown,
        32_000)?;
    assert_eq!(pages, 3);
    assert_eq!(rows.iter().map(|row| row.page).collect::<Vec<_>>(),
        [1, 1, 1, 2, 2, 3, 3]);
    assert_eq!(marks.len(), 1);
    assert_eq!(marks[0].page, 1);
    Ok(())
}

#[test]
fn long_markdown_decorated_blocks_balance_lone_tail_with_ink_marks()
-> Result<(), String> {
    use atrament_notebook_profile::NotebookProfile;

    let render = |source: &str, height| -> Result<_, String> {
        super::layout_flow_with_region(
            &FlowSource::new(FlowFormat::Markdown, source)?,
            &NotebookProfile::default(), None,
            super::HeadingDecoration::default(),
            Some(super::FlowRegion {
                left: 30_000, top: 40_000,
                width: 35_000, height,
            }),
        )
    };
    let words = vec!["aaaaaa"; 4].join(" ");
    for (source, quote, marker_count) in [
        (format!("- {words}"), false, 1),
        (format!("- [ ] {words}"), false, 4),
        (format!("- [x] {words}"), false, 6),
        (format!("> {words}"), true, 4),
    ] {
        let (rows, marks, pages) = render(&source, 32_000)?;
        assert_eq!(pages, 2, "{source:?}");
        assert_eq!(rows.iter().map(|row| row.page)
            .collect::<Vec<_>>(), [1, 1, 2, 2], "{source:?}");
        assert_eq!(rows.iter().map(|row| row.origin_um.1)
            .collect::<Vec<_>>(), [42_500, 51_500, 42_500, 51_500]);
        assert_eq!(rows.iter().map(|row| row.text.as_str())
            .collect::<Vec<_>>(), ["aaaaaa"; 4]);
        assert_eq!(marks.len(), marker_count, "{source:?}");
        if quote {
            assert_eq!(marks.iter().map(|mark| (
                mark.page, mark.start_um.1, mark.end_um.1,
            )).collect::<Vec<_>>(), [
                (1, 43_000, 50_900),
                (1, 52_000, 59_900),
                (2, 43_000, 50_900),
                (2, 52_000, 59_900),
            ]);
        } else {
            assert!(marks.iter().all(|mark| mark.page == 1),
                "list marker stays with first row");
        }
    }

    // The measured numbered hanging indent remains unchanged across
    // rebalanced pages, including a second tail on page three.
    let ordered = format!("1. {}", vec!["aaa"; 7].join(" "));
    let (numbered, no_marks, numbered_pages) = render(&ordered, 32_000)?;
    assert_eq!(numbered_pages, 3);
    assert!(no_marks.is_empty());
    assert_eq!(numbered.iter().map(|row| row.page)
        .collect::<Vec<_>>(), [1, 1, 1, 2, 2, 3, 3]);
    assert_eq!(numbered[0].text, "1. aaa");
    assert!(numbered[1..].iter().all(|row| row.text == "aaa"));
    assert_eq!(numbered[0].origin_um.0, 38_500);
    assert!(numbered[1..].iter().all(|row| row.origin_um.0 == 50_500));

    // A following block starts after both moved rows, not on top of
    // the second relocated quote bar and its corresponding text.
    let (following, rules, pages) = render(&format!("> {words}\na"),
        32_000)?;
    assert_eq!(pages, 2);
    assert_eq!(following.iter().map(|row| (row.page, row.origin_um.1))
        .collect::<Vec<_>>(), [
            (1, 42_500), (1, 51_500), (2, 42_500),
            (2, 51_500), (2, 60_500),
        ]);
    assert_eq!(rules.len(), 4);

    // A two-row sheet cannot move a full tail without also disturbing
    // the first-row widow guard, so it retains its original page breaks.
    let (short, short_marks, short_pages) = render(
        &format!("> {}", vec!["aaaaaa"; 5].join(" ")), 23_000)?;
    assert_eq!(short_pages, 3);
    assert_eq!(short.iter().map(|row| row.page)
        .collect::<Vec<_>>(), [1, 1, 2, 2, 3]);
    assert_eq!(short_marks.iter().map(|rule| rule.page)
        .collect::<Vec<_>>(), [1, 1, 2, 2, 3]);

    // The first two physical rows of a block still belong to their
    // preceding title/rule, before any tail balancing can apply.
    for source in [
        format!("# a\n> {words}"),
        format!("a\n\n---\n> {words}"),
    ] {
        let (rows, marks, pages) = render(&source, 32_000)?;
        assert!(pages >= 2, "{source:?}");
        let quote_rows = rows.iter().filter(|row| row.text == "aaaaaa")
            .collect::<Vec<_>>();
        assert_eq!(quote_rows.len(), 4);
        assert_eq!(quote_rows[0].page, quote_rows[1].page,
            "reserved first quote row stays with its successor");
        assert!(!marks.is_empty());
    }
    Ok(())
}

#[test]
fn three_row_markdown_blocks_move_together_without_detaching_prior_ink()
-> Result<(), String> {
    use atrament_notebook_profile::NotebookProfile;

    let render = |source: &str, height| -> Result<_, String> {
        super::layout_flow_with_region(
            &FlowSource::new(FlowFormat::Markdown, source)?,
            &NotebookProfile::default(), None,
            super::HeadingDecoration::default(),
            Some(super::FlowRegion {
                left: 30_000, top: 40_000, width: 35_000, height,
            }),
        )
    };
    // A literal code row precedes each block; only two more physical
    // rows fit on the first sheet. A fresh sheet holds the complete block.
    for (block, expected, mark_count) in [
        ("aaaaaa aaaaaa aaaaaa", ["aaaaaa", "aaaaaa", "aaaaaa"], 0),
        ("- aaaaaa aaaaaa aaaaaa", ["aaaaaa", "aaaaaa", "aaaaaa"], 1),
        ("- [ ] aaaaaa aaaaaa aaaaaa",
            ["aaaaaa", "aaaaaa", "aaaaaa"], 4),
        ("- [x] aaaaaa aaaaaa aaaaaa",
            ["aaaaaa", "aaaaaa", "aaaaaa"], 6),
        ("> aaaaaa aaaaaa aaaaaa", ["aaaaaa", "aaaaaa", "aaaaaa"], 3),
        ("1. a a a a a", ["1. a a", "a a", "a"], 0),
    ] {
        let source = format!("```\na\n```\n{block}");
        let (rows, marks, pages) = render(&source, 32_000)?;
        assert_eq!(pages, 2, "{block}");
        assert_eq!(rows.iter().map(|row| row.page)
            .collect::<Vec<_>>(), [1, 2, 2, 2], "{block}");
        assert_eq!(rows.iter().map(|row| row.origin_um.1)
            .collect::<Vec<_>>(), [42_500, 42_500, 51_500, 60_500],
            "{block}");
        assert_eq!(rows[1..].iter().map(|row| row.text.as_str())
            .collect::<Vec<_>>(), expected, "{block}");
        assert_eq!(marks.len(), mark_count, "{block}");
        assert!(marks.iter().all(|mark| mark.page == 2), "{block}");
        if block.starts_with("> ") {
            assert_eq!(marks.iter().map(|bar| (
                bar.start_um.1, bar.end_um.1,
            )).collect::<Vec<_>>(), [
                (43_000, 50_900),
                (52_000, 59_900),
                (61_000, 68_900),
            ]);
        } else if block.starts_with("- [") {
            assert!(marks.iter().all(|mark| {
                (43_700..=46_200).contains(&mark.start_um.1)
                    && (43_700..=46_200).contains(&mark.end_um.1)
            }), "checkbox belongs to its relocated first row: {block}");
        } else if block.starts_with("- ") {
            assert_eq!((marks[0].start_um, marks[0].end_um),
                ((34_100, 44_700), (35_800, 44_700)));
        } else if block.starts_with("1. ") {
            assert_eq!(rows[1].origin_um.0, 38_500);
            assert_eq!(rows[2].origin_um.0, 50_500);
            assert_eq!(rows[3].origin_um.0, 50_500);
        }
    }

    // After relocation, the next block starts on page three instead
    // of being written over the relocated third row.
    let source = "```\na\n```\naaaaaa aaaaaa aaaaaa\n# a";
    let (rows, heading_ink, pages) = render(source, 32_000)?;
    assert_eq!(pages, 3);
    assert_eq!(rows.iter().map(|row| row.page)
        .collect::<Vec<_>>(), [1, 2, 2, 2, 3]);
    assert_eq!(rows[4].origin_um, (32_500, 42_500));
    assert_eq!(heading_ink.len(), 1);
    assert_eq!(heading_ink[0].page, 3);

    // A preceding title or thematic rule reserves its first quote row.
    // Moving the whole quote would violate that higher-priority pairing.
    for source in [
        "# a\n> aaaaaa aaaaaa aaaaaa",
        "---\n> aaaaaa aaaaaa aaaaaa",
    ] {
        let (rows, bars, pages) = render(source, 32_000)?;
        assert_eq!(pages, 2, "{source:?}");
        let quoted = rows.iter().filter(|row| row.text == "aaaaaa")
            .collect::<Vec<_>>();
        assert_eq!(quoted.iter().map(|row| row.page)
            .collect::<Vec<_>>(), [1, 1, 2], "{source:?}");
        let quotes = bars.iter().filter(|bar| bar.start_um.0 == 35_500)
            .collect::<Vec<_>>();
        assert_eq!(quotes.iter().map(|bar| bar.page)
            .collect::<Vec<_>>(), [1, 1, 2], "{source:?}");
    }

    // Two-row sheets cannot admit a complete three-row block.
    let (short, _, short_pages) = render(
        "```\na\n```\n- aaaaaa aaaaaa aaaaaa", 23_000)?;
    assert_eq!(short_pages, 3);
    assert_eq!(short.iter().map(|row| row.page)
        .collect::<Vec<_>>(), [1, 2, 2, 3]);
    Ok(())
}

#[test]
fn four_row_markdown_blocks_relocate_all_ink_when_new_sheet_fits()
-> Result<(), String> {
    use atrament_notebook_profile::NotebookProfile;

    let render = |source: &str, height| -> Result<_, String> {
        super::layout_flow_with_region(
            &FlowSource::new(FlowFormat::Markdown, source)?,
            &NotebookProfile::default(), None,
            super::HeadingDecoration::default(),
            Some(super::FlowRegion {
                left: 30_000, top: 40_000, width: 35_000, height,
            }),
        )
    };
    // With one or two earlier code rows, the four-row block initially
    // splits 3+1 or 2+2. The next sheet fits all four physical rows.
    for prior_rows in 1..=2 {
        let prefix = format!(
            "```\n{}\n```\n", vec!["a"; prior_rows].join("\n"),
        );
        for (block, mark_count) in [
            ("aaaaaa aaaaaa aaaaaa aaaaaa", 0),
            ("- aaaaaa aaaaaa aaaaaa aaaaaa", 1),
            ("- [ ] aaaaaa aaaaaa aaaaaa aaaaaa", 4),
            ("- [x] aaaaaa aaaaaa aaaaaa aaaaaa", 6),
            ("> aaaaaa aaaaaa aaaaaa aaaaaa", 4),
            ("1. a a a a a a a a", 0),
        ] {
            let source = format!("{prefix}{block}");
            let (rows, marks, pages) = render(&source, 41_000)?;
            assert_eq!(pages, 2, "{prior_rows}: {block}");
            assert_eq!(rows.len(), prior_rows + 4, "{block}");
            assert_eq!(rows[..prior_rows].iter().map(|row| row.page)
                .collect::<Vec<_>>(), vec![1; prior_rows], "{block}");
            assert_eq!(rows[prior_rows..].iter().map(|row| (
                row.page, row.origin_um.1,
            )).collect::<Vec<_>>(), [
                (2, 42_500), (2, 51_500), (2, 60_500), (2, 69_500),
            ], "{prior_rows}: {block}");
            assert_eq!(marks.len(), mark_count, "{block}");
            assert!(marks.iter().all(|mark| mark.page == 2), "{block}");
            if block.starts_with("> ") {
                assert_eq!(marks.iter().map(|bar| (
                    bar.start_um.1, bar.end_um.1,
                )).collect::<Vec<_>>(), [
                    (43_000, 50_900), (52_000, 59_900),
                    (61_000, 68_900), (70_000, 77_900),
                ]);
            } else if block.starts_with("- [") {
                assert!(marks.iter().all(|mark| {
                    (43_700..=46_200).contains(&mark.start_um.1)
                        && (43_700..=46_200).contains(&mark.end_um.1)
                }), "checkbox ink belongs to relocated first row");
            } else if block.starts_with("- ") {
                assert_eq!((marks[0].start_um, marks[0].end_um),
                    ((34_100, 44_700), (35_800, 44_700)));
            } else if block.starts_with("1. ") {
                assert_eq!(rows[prior_rows].origin_um.0, 38_500);
                assert!(rows[prior_rows + 1..].iter()
                    .all(|row| row.origin_um.0 == 50_500));
            }
        }
    }

    // The relocated last row advances the physical cursor. The next
    // visible heading belongs on page three rather than overwriting it.
    let source = "```\na\n```\naaaaaa aaaaaa aaaaaa aaaaaa\n# a";
    let (rows, marks, pages) = render(source, 41_000)?;
    assert_eq!(pages, 3);
    assert_eq!(rows.iter().map(|row| row.page)
        .collect::<Vec<_>>(), [1, 2, 2, 2, 2, 3]);
    assert_eq!(rows[5].origin_um, (32_500, 42_500));
    assert_eq!(marks.len(), 1);
    assert_eq!(marks[0].page, 3);

    // A preceding heading or rule must remain paired with its first
    // quote row. The short-block relocation must not undo its reservation.
    for source in [
        "# a\n> aaaaaa aaaaaa aaaaaa aaaaaa",
        "---\n> aaaaaa aaaaaa aaaaaa aaaaaa",
    ] {
        let (rows, marks, pages) = render(source, 41_000)?;
        assert_eq!(pages, 2, "{source}");
        assert_eq!(rows[0].page, 1);
        assert_eq!(rows[1].page, 1);
        let quote_marks = marks.iter()
            .filter(|mark| mark.start_um.0 == 35_500)
            .collect::<Vec<_>>();
        assert_eq!(quote_marks.len(), 4);
        assert_eq!(quote_marks[0].page, 1);
    }

    // A three-row or shorter sheet cannot contain four rows. Preserve
    // the established two-row tail balance rather than forcing relocation.
    let (rows, marks, pages) = render(
        "> aaaaaa aaaaaa aaaaaa aaaaaa", 32_000)?;
    assert_eq!(pages, 2);
    assert_eq!(rows.iter().map(|row| row.page)
        .collect::<Vec<_>>(), [1, 1, 2, 2]);
    assert_eq!(marks.iter().map(|mark| mark.page)
        .collect::<Vec<_>>(), [1, 1, 2, 2]);
    Ok(())
}

#[test]
fn five_row_markdown_blocks_relocate_all_ink_when_new_sheet_fits()
-> Result<(), String> {
    use atrament_notebook_profile::NotebookProfile;

    let render = |source: &str, height| -> Result<_, String> {
        super::layout_flow_with_region(
            &FlowSource::new(FlowFormat::Markdown, source)?,
            &NotebookProfile::default(),
            None,
            super::HeadingDecoration::default(),
            Some(super::FlowRegion {
                left: 30_000,
                top: 40_000,
                width: 35_000,
                height,
            }),
        )
    };
    // A five-row block otherwise splits 4+1, 3+2, or 2+3 after one,
    // two, or three earlier literal rows on a five-row physical sheet.
    for prior_rows in 1..=3 {
        let prefix =
            format!("```\n{}\n```\n", vec!["a"; prior_rows].join("\n"));
        for (block, mark_count) in [
            ("aaaaaa aaaaaa aaaaaa aaaaaa aaaaaa", 0),
            ("- aaaaaa aaaaaa aaaaaa aaaaaa aaaaaa", 1),
            ("- [ ] aaaaaa aaaaaa aaaaaa aaaaaa aaaaaa", 4),
            ("- [x] aaaaaa aaaaaa aaaaaa aaaaaa aaaaaa", 6),
            ("> aaaaaa aaaaaa aaaaaa aaaaaa aaaaaa", 5),
            ("1. a a a a a a a a a a", 0),
        ] {
            let (rows, marks, pages) =
                render(&format!("{prefix}{block}"), 50_000)?;
            assert_eq!(pages, 2, "{prior_rows}: {block}");
            assert_eq!(rows.len(), prior_rows + 5, "{block}");
            assert_eq!(
                rows[..prior_rows]
                    .iter()
                    .map(|row| row.page)
                    .collect::<Vec<_>>(),
                vec![1; prior_rows],
                "{block}"
            );
            assert_eq!(
                rows[prior_rows..]
                    .iter()
                    .map(|row| (row.page, row.origin_um.1,))
                    .collect::<Vec<_>>(),
                [
                    (2, 42_500),
                    (2, 51_500),
                    (2, 60_500),
                    (2, 69_500),
                    (2, 78_500),
                ],
                "{prior_rows}: {block}"
            );
            assert_eq!(marks.len(), mark_count, "{block}");
            assert!(marks.iter().all(|mark| mark.page == 2), "{block}");
            if block.starts_with("> ") {
                assert_eq!(
                    marks
                        .iter()
                        .map(|bar| (bar.start_um.1, bar.end_um.1,))
                        .collect::<Vec<_>>(),
                    [
                        (43_000, 50_900),
                        (52_000, 59_900),
                        (61_000, 68_900),
                        (70_000, 77_900),
                        (79_000, 86_900),
                    ]
                );
            } else if block.starts_with("- [") {
                assert!(
                    marks.iter().all(|mark| {
                        (43_700..=46_200).contains(&mark.start_um.1)
                            && (43_700..=46_200).contains(&mark.end_um.1)
                    }),
                    "checkbox ink belongs to relocated first row"
                );
            } else if block.starts_with("1. ") {
                assert_eq!(rows[prior_rows].origin_um.0, 38_500);
                assert!(
                    rows[prior_rows + 1..]
                        .iter()
                        .all(|row| row.origin_um.0 == 50_500)
                );
            }
        }
    }

    // With no independent ink to preserve, a complete five-row block
    // simply occupies the first sheet without being relocated.
    let (alone, bars, pages) =
        render("> aaaaaa aaaaaa aaaaaa aaaaaa aaaaaa", 50_000)?;
    assert_eq!(pages, 1);
    assert!(alone.iter().all(|row| row.page == 1));
    assert_eq!(bars.len(), 5);
    assert!(bars.iter().all(|mark| mark.page == 1));

    // A following heading must not overwrite the relocated fifth row.
    let (continued, heading_marks, pages) = render(
        "```\na\n```\naaaaaa aaaaaa aaaaaa aaaaaa aaaaaa\n# a",
        50_000,
    )?;
    assert_eq!(pages, 3);
    assert_eq!(continued.iter().map(|row| row.page).collect::<Vec<_>>(), [
        1, 2, 2, 2, 2, 2, 3
    ]);
    assert_eq!(continued[6].origin_um, (32_500, 42_500));
    assert_eq!(heading_marks.len(), 1);
    assert_eq!(heading_marks[0].page, 3);

    // Heading/rule pairings take precedence, and short sheets cannot
    // move a block that does not fit entirely on a new sheet.
    for source in [
        "# a\n> aaaaaa aaaaaa aaaaaa aaaaaa aaaaaa",
        "---\n> aaaaaa aaaaaa aaaaaa aaaaaa aaaaaa",
    ] {
        let (rows, marks, pages) = render(source, 50_000)?;
        assert_eq!(pages, 2, "{source}");
        assert_eq!([rows[0].page, rows[1].page], [1, 1]);
        assert_eq!(
            marks
                .iter()
                .filter(|mark| mark.start_um.0 == 35_500)
                .map(|mark| mark.page)
                .collect::<Vec<_>>(),
            [1, 1, 1, 2, 2]
        );
    }
    let (short, short_marks, pages) =
        render("> aaaaaa aaaaaa aaaaaa aaaaaa aaaaaa", 41_000)?;
    assert_eq!(pages, 2);
    assert_eq!(short.iter().map(|row| row.page).collect::<Vec<_>>(), [
        1, 1, 1, 2, 2
    ]);
    assert_eq!(
        short_marks.iter().map(|mark| mark.page).collect::<Vec<_>>(),
        [1, 1, 1, 2, 2]
    );
    Ok(())
}

#[test]
fn whole_fitting_markdown_blocks_keep_six_to_twelve_rows_together()
-> Result<(), String> {
    use atrament_notebook_profile::NotebookProfile;

    let render = |source: &str, height| -> Result<_, String> {
        super::layout_flow_with_region(
            &FlowSource::new(FlowFormat::Markdown, source)?,
            &NotebookProfile::default(),
            None,
            super::HeadingDecoration::default(),
            Some(super::FlowRegion {
                left: 30_000,
                top: 40_000,
                width: 35_000,
                height,
            }),
        )
    };
    for (count, priors) in
        [(6usize, [1usize, 3, 4]), (8, [1, 4, 6]), (12, [1, 6, 10])]
    {
        let height = 5_000
            + u32::try_from(count).map_err(|error| error.to_string())? * 9_000;
        for prior in priors {
            let code_rows = vec!["a"; prior].join("\n");
            let prefix = format!("```\n{code_rows}\n```\n");
            for (marker, marks_expected) in [
                ("", 0),
                ("- ", 1),
                ("- [ ] ", 4),
                ("- [x] ", 6),
                ("> ", count),
            ] {
                let words = vec!["aaaaaa"; count].join(" ");
                let block = format!("{marker}{words}");
                let (rows, marks, pages) =
                    render(&format!("{prefix}{block}"), height)?;
                assert_eq!(pages, 2, "{count} {prior} {marker}");
                assert_eq!(rows.len(), prior + count);
                assert!(rows.iter().take(prior).all(|row| row.page == 1));
                for (index, row) in rows.iter().skip(prior).enumerate() {
                    let offset = u32::try_from(index)
                        .map_err(|error| error.to_string())?
                        * 9_000;
                    assert_eq!(row.page, 2, "{count} {prior} {marker}");
                    assert_eq!(row.text, "aaaaaa");
                    assert_eq!(row.origin_um.1, 42_500 + offset);
                }
                assert_eq!(marks.len(), marks_expected);
                assert!(marks.iter().all(|mark| mark.page == 2));
                if marker == "> " {
                    for (index, mark) in marks.iter().enumerate() {
                        let offset = u32::try_from(index)
                            .map_err(|error| error.to_string())?
                            * 9_000;
                        assert_eq!(mark.start_um.1, 43_000 + offset);
                        assert_eq!(mark.end_um.1, 50_900 + offset);
                    }
                } else if marker.starts_with("- [") {
                    assert!(marks.iter().all(|mark| {
                        (43_700..=46_200).contains(&mark.start_um.1)
                            && (43_700..=46_200).contains(&mark.end_um.1)
                    }));
                } else if marker == "- " {
                    assert_eq!(
                        marks.first().map(|mark| mark.start_um),
                        Some((34_100, 44_700))
                    );
                    assert_eq!(
                        marks.first().map(|mark| mark.end_um),
                        Some((35_800, 44_700))
                    );
                }
            }
        }
    }
    Ok(())
}

#[test]
fn page_fitting_markdown_blocks_preserve_precedence_cursor_and_fallback()
-> Result<(), String> {
    use atrament_notebook_profile::NotebookProfile;

    let render = |source: &str, height| -> Result<_, String> {
        super::layout_flow_with_region(
            &FlowSource::new(FlowFormat::Markdown, source)?,
            &NotebookProfile::default(),
            None,
            super::HeadingDecoration::default(),
            Some(super::FlowRegion {
                left: 30_000,
                top: 40_000,
                width: 35_000,
                height,
            }),
        )
    };
    // A following block begins after the full relocated final row.
    let next = format!("```\na\n```\n{}\n# a", ["aaaaaa"; 8].join(" "));
    let (rows, marks, pages) = render(&next, 77_000)?;
    assert_eq!(pages, 3);
    assert_eq!(rows.get(9).map(|row| row.origin_um), Some((32_500, 42_500)));
    assert_eq!(rows.get(9).map(|row| row.page), Some(3));
    assert_eq!(marks.len(), 1);
    assert_eq!(marks.first().map(|mark| mark.page), Some(3));

    // Ordered item continuation uses the measured label's hanging indent.
    let ordered = format!("```\na\n```\n1. {}", ["a"; 16].join(" "));
    let (numbered, marks, pages) = render(&ordered, 77_000)?;
    assert_eq!(pages, 2);
    assert!(marks.is_empty());
    assert_eq!(numbered.len(), 9);
    assert!(numbered.iter().skip(1).all(|row| row.page == 2));
    assert_eq!(numbered.get(1).map(|row| row.origin_um.0), Some(38_500));
    assert!(numbered.iter().skip(2).all(|row| row.origin_um.0 == 50_500));

    // A heading or rule still owns its first subsequent row.
    for title in ["# a", "---"] {
        let source = format!("{title}\n> {}", ["aaaaaa"; 8].join(" "));
        let (rows, marks, pages) = render(&source, 77_000)?;
        assert_eq!(pages, 2);
        assert_eq!(rows.first().map(|row| row.page), Some(1));
        assert_eq!(rows.get(1).map(|row| row.page), Some(1));
        assert!(
            marks
                .iter()
                .any(|bar| bar.page == 1 && bar.start_um.0 == 35_500)
        );
    }

    // If a fresh sheet cannot hold the block, preserve normal splitting.
    let source = format!("```\na\n```\n> {}", ["aaaaaa"; 8].join(" "));
    let (rows, bars, pages) = render(&source, 68_000)?;
    assert_eq!(pages, 2);
    assert_eq!(rows.first().map(|row| row.page), Some(1));
    assert!(rows.iter().skip(1).any(|row| row.page == 1));
    assert!(rows.iter().skip(1).any(|row| row.page == 2));
    assert_eq!(bars.len(), 8);
    assert!(bars.iter().any(|bar| bar.page == 1));
    assert!(bars.iter().any(|bar| bar.page == 2));
    Ok(())
}

#[test]
fn heading_and_complete_wrapped_block_move_together_after_independent_ink()
-> Result<(), String> {
    use atrament_notebook_profile::NotebookProfile;

    let render = |source: &str, height, decoration| -> Result<_, String> {
        super::layout_flow_with_region(
            &FlowSource::new(FlowFormat::Markdown, source)?,
            &NotebookProfile::default(), None, decoration,
            Some(super::FlowRegion {
                left: 30_000, top: 40_000,
                width: 35_000, height,
            }),
        )
    };
    let marker = super::HeadingDecoration {
        style: Some(super::HeadingStyle::Marker),
        ..super::HeadingDecoration::default()
    };
    let variants = [
        ("aaaaaa aaaaaa aaaaaa", 0, 32_500),
        ("- aaaaaa aaaaaa aaaaaa", 1, 38_500),
        ("- [ ] aaaaaa aaaaaa aaaaaa", 4, 38_500),
        ("- [x] aaaaaa aaaaaa aaaaaa", 6, 38_500),
        ("> aaaaaa aaaaaa aaaaaa", 3, 38_500),
        ("1. a a a a a", 0, 38_500),
    ];
    for heading in ["# a", "## a", "a\n==="] {
        for (block, mark_count, first_x) in variants {
            let source = format!("```\na\n```\n{heading}\n{block}");
            let (rows, marks, pages) = render(&source, 41_000, marker)?;
            assert_eq!(pages, 2, "{heading} / {block}");
            assert_eq!(rows.len(), 5, "{heading} / {block}");
            assert_eq!(rows.iter().map(|row| row.page)
                .collect::<Vec<_>>(), [1, 2, 2, 2, 2],
                "{heading} / {block}");
            assert_eq!(rows.iter().map(|row| row.origin_um.1)
                .collect::<Vec<_>>(),
                [42_500, 42_500, 51_500, 60_500, 69_500]);
            assert_eq!(rows[1].origin_um.0, 32_500);
            assert_eq!(rows[2].origin_um.0, first_x);
            assert_eq!(marks.len(), mark_count
                + if heading == "## a" { 3 } else { 4 });
            assert!(marks.iter().all(|mark| mark.page == 2));
            let heading_ink = if heading == "## a" { 3 } else { 4 };
            assert!(marks[..heading_ink].iter().all(|mark| {
                (43_000..47_000).contains(&mark.start_um.1)
                    && mark.start_um.1 == mark.end_um.1
            }));
            if block.starts_with('>') {
                assert_eq!(marks[heading_ink..].iter().map(|bar| (
                    bar.start_um.1, bar.end_um.1,
                )).collect::<Vec<_>>(), [
                    (52_000, 59_900), (61_000, 68_900),
                    (70_000, 77_900),
                ]);
            } else if block.starts_with("- [") {
                assert!(marks[heading_ink..].iter().all(|mark| {
                    (52_700..=55_200).contains(&mark.start_um.1)
                        && (52_700..=55_200).contains(&mark.end_um.1)
                }));
            } else if block.starts_with("- ") {
                assert_eq!(marks[heading_ink].start_um, (34_100, 53_700));
            } else if block.starts_with("1. ") {
                assert_eq!(rows[2].text, "1. a a");
                assert!(rows[3..].iter().all(|row| {
                    row.origin_um.0 == 50_500
                }));
            }
        }
    }

    // A following heading cannot overwrite the relocated last body row.
    let source = "```\na\n```\n# a\naaaaaa aaaaaa aaaaaa\n# a";
    let (rows, marks, pages) = render(source, 41_000,
        super::HeadingDecoration::default())?;
    assert_eq!(pages, 3);
    assert_eq!(rows.iter().map(|row| row.page)
        .collect::<Vec<_>>(), [1, 2, 2, 2, 2, 3]);
    assert_eq!(rows[5].origin_um, (32_500, 42_500));
    assert_eq!(marks.iter().map(|mark| mark.page)
        .collect::<Vec<_>>(), [2, 3]);
    Ok(())
}

#[test]
fn blank_separated_heading_and_fitting_block_preserve_gap_and_ink()
-> Result<(), String> {
    use atrament_notebook_profile::NotebookProfile;

    let render = |source: &str, height| -> Result<_, String> {
        super::layout_flow_with_region(
            &FlowSource::new(FlowFormat::Markdown, source)?,
            &NotebookProfile::default(), None,
            super::HeadingDecoration {
                style: Some(super::HeadingStyle::Marker),
                ..super::HeadingDecoration::default()
            },
            Some(super::FlowRegion {
                left: 30_000, top: 40_000,
                width: 35_000, height,
            }),
        )
    };
    for count in [2usize, 3, 4, 6] {
        let height = 9_000 * u32::try_from(count + 1)
            .map_err(|error| error.to_string())? + 9_000;
        for heading in ["# a", "## a", "a\n==="] {
            for marker in ["", "- ", "- [x] ", "> "] {
                let block = format!(
                    "{marker}{}", vec!["aaaaaa"; count].join(" "),
                );
                for blanks in ["\n\n", "\n\n\n"] {
                    let source = format!(
                        "```\na\n```\n{heading}{blanks}{block}",
                    );
                    let (rows, marks, pages) = render(&source, height)?;
                    assert_eq!(pages, 2, "{count} {heading} {marker}");
                    assert_eq!(rows.len(), count + 2);
                    assert_eq!(rows[0].page, 1);
                    for (index, row) in rows.iter().skip(1).enumerate() {
                        assert_eq!(row.page, 2,
                            "{count} {heading} {marker}");
                        let gap = if index > 0 { 4_000 } else { 0 };
                        assert_eq!(row.origin_um.1,
                            42_500 + 9_000 * u32::try_from(index)
                                .map_err(|error| error.to_string())? + gap);
                    }
                    assert!(marks.iter().all(|mark| mark.page == 2));
                    if marker == "> " {
                        assert_eq!(marks.len(), count +
                            if heading == "## a" { 3 } else { 4 });
                        for (index, mark) in marks.iter()
                            .skip(if heading == "## a" { 3 } else { 4 })
                            .enumerate() {
                            let y = 56_000 + 9_000 *
                                u32::try_from(index)
                                    .map_err(|error| error.to_string())?;
                            assert_eq!(mark.start_um.1, y);
                            assert_eq!(mark.end_um.1, y + 7_900);
                        }
                    }
                }
            }
        }
    }

    // A later title starts after the relocated final row and cannot
    // overwrite the collapsed-gap group on its full second sheet.
    let (rows, marks, pages) = render(
        "```\na\n```\n# a\n\n> aaaaaa aaaaaa\n# a", 36_000,
    )?;
    assert_eq!(pages, 3);
    assert_eq!(rows.iter().map(|row| row.page).collect::<Vec<_>>(),
        [1, 2, 2, 2, 3]);
    assert_eq!(rows[4].origin_um.1, 42_500);
    assert!(marks.iter().any(|mark| mark.page == 3));

    // Authored decimal labels stay on the first moved body row. The
    // continuation remains aligned under text, not under the numeral.
    let (rows, marks, pages) = render(
        "```\na\n```\n# a\n\n1. a a a a a", 45_000,
    )?;
    assert_eq!(pages, 2);
    assert_eq!(rows.iter().map(|row| row.page).collect::<Vec<_>>(),
        [1, 2, 2, 2, 2]);
    assert_eq!(rows.iter().map(|row| row.text.as_str())
        .collect::<Vec<_>>(), ["a", "a", "1. a a", "a a", "a"]);
    assert_eq!(rows.iter().map(|row| row.origin_um)
        .collect::<Vec<_>>(), [
            (32_500, 42_500), (32_500, 42_500),
            (38_500, 55_500), (50_500, 64_500), (50_500, 73_500),
        ]);
    assert_eq!(marks.len(), 4);
    assert!(marks.iter().all(|mark| mark.page == 2));

    // No independent ink means no relocation or blank-page invention.
    let (rows, marks, pages) = render(
        "# a\n\n> aaaaaa aaaaaa", 36_000,
    )?;
    assert_eq!(pages, 1);
    assert_eq!(rows.iter().map(|row| row.page).collect::<Vec<_>>(),
        [1, 1, 1]);
    assert!(marks.iter().all(|mark| mark.page == 1));

    // A short sheet cannot hold the complete group; normal heading-first
    // spacing and page breaks continue without moving the whole block.
    let (rows, bars, pages) = render(
        "```\na\n```\n# a\n\n> aaaaaa aaaaaa", 32_000,
    )?;
    assert_eq!(pages, 3);
    assert_eq!(rows.iter().map(|row| row.page).collect::<Vec<_>>(),
        [1, 2, 2, 3]);
    assert_eq!(bars.iter().filter(|bar| bar.start_um.0 == 35_500)
        .count(), 2);

    // A heading already attached to a rule must keep that precedence.
    let (rows, marks, pages) = render(
        "```\na\n```\n---\n# a\n\n> aaaaaa aaaaaa", 36_000,
    )?;
    assert_eq!(pages, 3);
    assert_eq!(rows[0].page, 1);
    assert_eq!(rows[1].page, 2);
    assert!(marks.iter().filter(|mark| mark.start_um.0 != 35_500)
        .all(|mark| mark.page == 2));
    Ok(())
}

#[test]
fn thematic_rule_and_fitting_wrapped_block_move_as_one_group()
-> Result<(), String> {
    use atrament_notebook_profile::NotebookProfile;

    let render = |source: &str, height| {
        super::layout_flow_with_region(
            &FlowSource::new(FlowFormat::Markdown, source)?,
            &NotebookProfile::default(),
            None,
            super::HeadingDecoration::default(),
            Some(super::FlowRegion {
                left: 30_000,
                top: 40_000,
                width: 35_000,
                height,
            }),
        )
    };
    for (body, first_x, continuation_x) in [
        ("> aaaaaa aaaaaa aaaaaa", 38_500, 38_500),
        ("- aaaaaa aaaaaa aaaaaa", 38_500, 38_500),
        ("1. a a a a a", 38_500, 50_500),
        ("aaaaaa aaaaaa aaaaaa", 32_500, 32_500),
    ] {
        let source = format!("```\na\n```\n---\n{body}");
        let (rows, marks, pages) = render(&source, 41_000)?;
        assert_eq!(pages, 2, "{body}");
        assert_eq!(
            rows.iter().map(|run| run.page).collect::<Vec<_>>(),
            [1, 2, 2, 2],
            "{body}"
        );
        assert_eq!(
            rows.iter().map(|run| run.origin_um.1).collect::<Vec<_>>(),
            [42_500, 51_500, 60_500, 69_500]
        );
        assert_eq!(rows[1].origin_um.0, first_x);
        assert_eq!(rows[2].origin_um.0, continuation_x);
        assert_eq!(marks[0].page, 2, "rule must travel with body");
        assert_eq!(marks[0].start_um, (32_500, 47_000));
        assert_eq!(marks[0].end_um, (63_500, 47_000));
        assert!(marks.iter().all(|mark| mark.page == 2));
    }
    // One collapsed 4-mm source gap belongs after the rule's physical
    // row, even when the complete two-row quote moves to the next sheet.
    for source in [
        "```\na\n```\n---\n\n> aaaaaa aaaaaa",
        "```\na\n```\n---\n\n\n> aaaaaa aaaaaa",
    ] {
        let (rows, marks, pages) = render(source, 36_000)?;
        assert_eq!(pages, 2);
        assert_eq!(rows.iter().map(|row| row.page).collect::<Vec<_>>(), [
            1, 2, 2
        ]);
        assert_eq!(
            rows.iter().map(|row| row.origin_um.1).collect::<Vec<_>>(),
            [42_500, 55_500, 64_500]
        );
        assert_eq!(marks[0].page, 2);
        assert_eq!(marks[0].start_um, (32_500, 47_000));
        assert_eq!(
            marks
                .iter()
                .skip(1)
                .map(|bar| bar.start_um.1)
                .collect::<Vec<_>>(),
            [56_000, 65_000]
        );
    }
    // An unoccupied sheet needs no artificial relocation.
    let (rows, marks, pages) = render("---\n> aaaaaa aaaaaa aaaaaa", 41_000)?;
    assert_eq!(pages, 1);
    assert!(rows.iter().all(|row| row.page == 1));
    assert_eq!(marks[0].page, 1);

    // Rule and block must really fit together, or normal pagination wins.
    let (rows, marks, _) =
        render("```\na\n```\n---\n> aaaaaa aaaaaa aaaaaa", 32_000)?;
    assert_eq!(rows[0].page, 1);
    assert_eq!(marks[0].page, 1);

    // An earlier heading reserved the rule's first row: it is not free
    // to travel with just the subsequent quote.
    let (rows, marks, _) =
        render("```\na\n```\n# a\n---\n> aaaaaa aaaaaa aaaaaa", 41_000)?;
    assert_eq!(rows[1].page, 1);
    assert_eq!(marks[1].page, 1);
    Ok(())
}

#[test]
fn consecutive_thematic_rules_keep_their_fitting_wrapped_quote()
-> Result<(), String> {
    use atrament_notebook_profile::NotebookProfile;

    let render = |source: &str, height| {
        super::layout_flow_with_region(
            &FlowSource::new(FlowFormat::Markdown, source)?,
            &NotebookProfile::default(), None,
            super::HeadingDecoration::default(),
            Some(super::FlowRegion {
                left: 30_000, top: 40_000,
                width: 35_000, height,
            }),
        )
    };
    let source = "```\na\n```\n---\n***\n> aaaaaa aaaaaa aaaaaa";
    let (rows, marks, pages) = render(source, 50_000)?;
    assert_eq!(pages, 2);
    assert_eq!(rows.iter().map(|row| row.page).collect::<Vec<_>>(),
        [1, 2, 2, 2]);
    assert_eq!(rows.iter().skip(1).map(|row| row.origin_um.1)
        .collect::<Vec<_>>(), [60_500, 69_500, 78_500]);
    assert_eq!(marks.iter().map(|mark| mark.page)
        .collect::<Vec<_>>(), [2, 2, 2, 2, 2]);
    assert_eq!(marks.iter().take(2).map(|mark| mark.start_um.1)
        .collect::<Vec<_>>(), [47_000, 56_000]);
    assert_eq!(marks.iter().skip(2).map(|mark| mark.start_um.1)
        .collect::<Vec<_>>(), [61_000, 70_000, 79_000]);
    // Independent gaps between separators are preserved. Repeated
    // source blanks collapse to the same native physical ink.
    let spaced = "```\na\n```\n---\n\n***\n> aaaaaa aaaaaa aaaaaa";
    let (rows, marks, pages) = render(spaced, 54_000)?;
    assert_eq!(pages, 2);
    assert_eq!(rows.iter().map(|row| row.page)
        .collect::<Vec<_>>(), [1, 2, 2, 2]);
    assert_eq!(rows.iter().skip(1).map(|row| row.origin_um.1)
        .collect::<Vec<_>>(), [64_500, 73_500, 82_500]);
    assert_eq!(marks.iter().map(|mark| mark.start_um.1)
        .collect::<Vec<_>>(), [47_000, 60_000, 65_000, 74_000, 83_000]);
    let repeated = spaced.replace("\n\n", "\n\n\n");
    let (again, again_marks, again_pages) = render(&repeated, 54_000)?;
    assert_eq!(again_pages, pages);
    assert_eq!(again_marks, marks);
    assert_eq!(again.iter().map(|run| (run.page, run.origin_um,
        run.text.as_str())).collect::<Vec<_>>(),
        rows.iter().map(|run| (run.page, run.origin_um,
            run.text.as_str())).collect::<Vec<_>>());
    // The same measured mover supports more than two separators and
    // independent collapsed gaps without inventing text-run owners.
    let triple = "```\na\n```\n---\n\n***\n\n___\n> aaaaaa aaaaaa";
    let (rows, marks, pages) = render(triple, 58_000)?;
    assert_eq!(pages, 2);
    assert_eq!(rows.iter().map(|row| row.page)
        .collect::<Vec<_>>(), [1, 2, 2]);
    assert_eq!(rows.iter().skip(1).map(|row| row.origin_um.1)
        .collect::<Vec<_>>(), [77_500, 86_500]);
    assert_eq!(marks.iter().map(|mark| (mark.page, mark.start_um.1))
        .collect::<Vec<_>>(), [(2, 47_000), (2, 60_000),
            (2, 73_000), (2, 78_000), (2, 87_000)]);
    // A complete rule chain can also own a heading and its entire
    // wrapped block, without leaving either physical separator behind.
    let with_heading = "```\na\n```\n---\n***\n# a\n> aaaaaa aaaaaa aaaaaa";
    let (rows, marks, pages) = render(with_heading, 59_000)?;
    assert_eq!(pages, 2);
    assert_eq!(rows.iter().map(|row| row.page)
        .collect::<Vec<_>>(), [1, 2, 2, 2, 2]);
    assert_eq!(rows.iter().skip(1).map(|row| row.origin_um.1)
        .collect::<Vec<_>>(), [60_500, 69_500, 78_500, 87_500]);
    assert_eq!(marks.iter().map(|mark| mark.page)
        .collect::<Vec<_>>(), [2, 2, 2, 2, 2, 2]);
    assert_eq!(marks.iter().map(|mark| mark.start_um.1)
        .collect::<Vec<_>>(), [47_000, 56_000, 67_200,
            70_000, 79_000, 88_000]);
    // A chain too tall for a fresh page retains the earlier first-row
    // pairing; no impossible all-or-nothing group is forced.
    let (rows, marks, pages) = render(source, 41_000)?;
    assert_eq!(pages, 2);
    assert_eq!(marks[0].page, marks[1].page);
    assert_eq!(rows[0].page, 1);
    Ok(())
}

#[test]
fn thematic_rule_heading_chain_and_fitting_block_keep_their_full_group()
-> Result<(), String> {
    use atrament_notebook_profile::NotebookProfile;

    let render = |source: &str, height| -> Result<_, String> {
        super::layout_flow_with_region(
            &FlowSource::new(FlowFormat::Markdown, source)?,
            &NotebookProfile::default(), None,
            super::HeadingDecoration {
                style: Some(super::HeadingStyle::Marker),
                ..super::HeadingDecoration::default()
            },
            Some(super::FlowRegion {
                left: 30_000, top: 40_000,
                width: 35_000, height,
            }),
        )
    };
    // The rule is reserved by the first heading, so moving only the
    // heading and body would strand its synthetic physical row.
    for body in [
        "> aaaaaa aaaaaa aaaaaa",
        "- aaaaaa aaaaaa aaaaaa",
        "1. a a a a a",
        "aaaaaa aaaaaa aaaaaa",
    ] {
        let source = format!("```\na\n```\n---\n# a\n{body}");
        let (runs, marks, pages) = render(&source, 50_000)?;
        assert_eq!(pages, 2, "{body}");
        assert_eq!(runs.iter().map(|row| row.page)
            .collect::<Vec<_>>(), [1, 2, 2, 2, 2], "{body}");
        assert_eq!(runs.iter().skip(1).map(|row| row.origin_um.1)
            .collect::<Vec<_>>(), [51_500, 60_500, 69_500, 78_500]);
        assert_eq!(marks[0].page, 2, "rule stays with its heading");
        assert_eq!(marks[0].start_um, (32_500, 47_000));
        assert!(marks.iter().all(|mark| mark.page == 2));
    }
    // Rule/heading, heading/heading and final heading/body each own a
    // distinct collapsed 4-mm gap. Keep all three and the quote bars.
    for subtitle in ["## a", "a\n==="] {
        let source = format!(
            "{}\n\n{subtitle}\n\n> aaaaaa aaaaaa aaaaaa",
            "```\na\na\n```\n---\n\n# a",
        );
        let (runs, marks, pages) = render(&source, 71_000)?;
        assert_eq!(pages, 2);
        assert_eq!(runs.iter().map(|row| row.page)
            .collect::<Vec<_>>(), [1, 1, 2, 2, 2, 2, 2]);
        assert_eq!(runs.iter().skip(2).map(|row| row.origin_um.1)
            .collect::<Vec<_>>(), [55_500, 68_500, 81_500, 90_500, 99_500]);
        assert_eq!(marks[0].start_um, (32_500, 47_000));
        assert!(marks.iter().all(|mark| mark.page == 2));
        assert_eq!(marks.iter().rev().take(3).map(|bar| bar.start_um.1)
            .collect::<Vec<_>>(), [100_000, 91_000, 82_000]);
        let repeated = source.replace("\n\n", "\n\n\n");
        let (replayed, replay_marks, replay_pages) =
            render(&repeated, 71_000)?;
        assert_eq!(replay_pages, pages);
        assert_eq!(replay_marks, marks);
        assert_eq!(replayed.iter().map(|row| (row.page, row.origin_um,
            row.text.as_str())).collect::<Vec<_>>(),
            runs.iter().map(|row| (row.page, row.origin_um,
                row.text.as_str())).collect::<Vec<_>>());
    }
    // One short sheet cannot hold the entire group: preserve older
    // first-row reservations rather than splitting or hiding ink.
    let (runs, marks, pages) = render(
        "```\na\n```\n---\n# a\n> aaaaaa aaaaaa aaaaaa",
        41_000,
    )?;
    assert_eq!(pages, 2);
    assert_eq!(runs[1].page, 1);
    assert_eq!(marks[0].page, 1);
    // Never detach a rule already reserved by an earlier heading.
    let (runs, marks, _) = render(
        "```\na\n```\n# a\n---\n## a\n> aaaaaa aaaaaa aaaaaa",
        50_000,
    )?;
    assert_eq!(runs[1].page, 1);
    assert!(marks.iter().any(|mark| mark.page == 1));
    let (runs, marks, pages) = render(
        "---\n# a\n> aaaaaa aaaaaa aaaaaa", 50_000,
    )?;
    assert_eq!(pages, 1);
    assert!(runs.iter().all(|row| row.page == 1));
    assert!(marks.iter().all(|mark| mark.page == 1));
    Ok(())
}

#[test]
fn adjacent_heading_chain_and_fitting_block_can_move_as_one_group()
-> Result<(), String> {
    use atrament_notebook_profile::NotebookProfile;

    let render = |source: &str, height| -> Result<_, String> {
        super::layout_flow_with_region(
            &FlowSource::new(FlowFormat::Markdown, source)?,
            &NotebookProfile::default(),
            None,
            super::HeadingDecoration {
                style: Some(super::HeadingStyle::Marker),
                ..super::HeadingDecoration::default()
            },
            Some(super::FlowRegion {
                left: 30_000,
                top: 40_000,
                width: 35_000,
                height,
            }),
        )
    };
    for second in ["## a", "a\n==="] {
        for marker in ["", "- ", "- [x] ", "> ", "1. "] {
            let body = if marker == "1. " {
                "1. a a a a a".to_owned()
            } else {
                format!("{marker}aaaaaa aaaaaa aaaaaa")
            };
            let source = format!("```\na\n```\n# a\n{second}\n{body}");
            let (rows, marks, pages) = render(&source, 50_000)?;
            assert_eq!(pages, 2, "{second} / {marker}");
            assert_eq!(rows.len(), 6, "{second} / {marker}");
            assert_eq!(
                rows.iter().map(|row| row.page).collect::<Vec<_>>(),
                [1, 2, 2, 2, 2, 2],
                "two adjacent titles should travel with fitting body"
            );
            assert_eq!(
                rows.iter()
                    .skip(1)
                    .map(|row| row.origin_um.1)
                    .collect::<Vec<_>>(),
                [42_500, 51_500, 60_500, 69_500, 78_500]
            );
            assert!(marks.iter().all(|mark| mark.page == 2));
        }
    }
    // A collapsed gap belongs after the *last* heading, not after the
    // first. Its 4-mm offset must move with all three body rows and bars.
    for second in ["## a", "a\n---"] {
        for marker in ["", "> ", "- [x] ", "1. "] {
            let body = if marker == "1. " {
                "1. a a a a a".to_owned()
            } else {
                format!("{marker}aaaaaa aaaaaa aaaaaa")
            };
            let source = format!("```\na\n```\n# a\n{second}\n\n{body}");
            let (rows, marks, pages) = render(&source, 54_000)?;
            assert_eq!(pages, 2, "{second} / {marker}");
            assert_eq!(rows.iter().map(|row| row.page).collect::<Vec<_>>(), [
                1, 2, 2, 2, 2, 2
            ]);
            assert_eq!(
                rows.iter()
                    .skip(1)
                    .map(|row| row.origin_um.1)
                    .collect::<Vec<_>>(),
                [42_500, 51_500, 64_500, 73_500, 82_500]
            );
            assert!(marks.iter().all(|mark| mark.page == 2));
            if marker == "> " {
                assert_eq!(marks.len(), 10);
                assert_eq!(
                    marks
                        .iter()
                        .skip(7)
                        .map(|bar| (bar.start_um.1, bar.end_um.1,))
                        .collect::<Vec<_>>(),
                    [(65_000, 72_900), (74_000, 81_900), (83_000, 90_900),]
                );
            }
        }
    }
    let (rows, _, pages) = render(
        "```\na\n```\n# a\n## a\n\n> aaaaaa aaaaaa aaaaaa\n# a",
        54_000,
    )?;
    assert_eq!(pages, 3);
    assert_eq!(
        rows.last().expect("following heading").origin_um,
        (32_500, 42_500),
        "page cursor follows relocated ink"
    );

    // An earlier rule owns the first heading. Do not detach that row.
    let (rows, _, _) =
        render("```\na\n```\n---\n# a\n## a\naaaaaa aaaaaa aaaaaa", 50_000)?;
    assert_eq!(rows[1].page, 1);

    // Three adjacent titles and a two-row quote are five physical rows.
    // When one independent row is already present, move all five rows.
    let (rows, marks, pages) =
        render("```\na\n```\n# a\n## a\n### a\n> aaaaaa aaaaaa", 50_000)?;
    assert_eq!(pages, 2);
    assert_eq!(rows.iter().map(|row| row.page).collect::<Vec<_>>(), [
        1, 2, 2, 2, 2, 2
    ]);
    assert_eq!(marks.len(), 11);
    assert!(marks.iter().all(|mark| mark.page == 2));

    // A chain of two headings and exactly two body rows also stays whole.
    let (rows, _, pages) =
        render("```\na\n```\n# a\n## a\n> aaaaaa aaaaaa", 41_000)?;
    assert_eq!(pages, 2);
    assert_eq!(rows.iter().map(|row| row.page).collect::<Vec<_>>(), [
        1, 2, 2, 2, 2
    ]);

    // Heading-style none still moves original authored ink with no marks.
    let (rows, marks, _) = super::layout_flow_with_region(
        &FlowSource::new(
            FlowFormat::Markdown,
            "```\na\n```\n# a\n## a\naaaaaa aaaaaa aaaaaa",
        )?,
        &NotebookProfile::default(),
        None,
        super::HeadingDecoration {
            style: Some(super::HeadingStyle::None),
            ..super::HeadingDecoration::default()
        },
        Some(super::FlowRegion {
            left: 30_000,
            top: 40_000,
            width: 35_000,
            height: 50_000,
        }),
    )?;
    assert_eq!(rows.iter().map(|row| row.page).collect::<Vec<_>>(), [
        1, 2, 2, 2, 2, 2
    ]);
    assert!(marks.is_empty());

    // A prior title separated by one collapsed 4-mm gap owns the next
    // heading row; relocating a later contiguous suffix would detach it.
    let (rows, _, pages) = render(
        "```\na\n```\n# a\n\n## a\n### a\naaaaaa aaaaaa aaaaaa",
        59_000,
    )?;
    assert_eq!(pages, 2);
    assert_eq!(
        rows[1].page, rows[2].page,
        "blank-separated title cannot be orphaned by suffix movement"
    );

    // The chain cannot relocate when five rows will not fit a new sheet.
    let (rows, marks, pages) =
        render("```\na\n```\n# a\n## a\naaaaaa aaaaaa aaaaaa", 41_000)?;
    assert_eq!(pages, 2);
    assert_eq!(rows.len(), 6);
    assert_eq!(marks.len(), 7);
    Ok(())
}

#[test]
fn blank_separated_heading_chain_moves_with_fitting_wrapped_block()
-> Result<(), String> {
    use atrament_notebook_profile::NotebookProfile;

    let render = |source: &str, height| -> Result<_, String> {
        super::layout_flow_with_region(
            &FlowSource::new(FlowFormat::Markdown, source)?,
            &NotebookProfile::default(), None,
            super::HeadingDecoration {
                style: Some(super::HeadingStyle::Marker),
                ..super::HeadingDecoration::default()
            },
            Some(super::FlowRegion {
                left: 30_000, top: 40_000,
                width: 35_000, height,
            }),
        )
    };
    // Both gaps are distinct 4-mm collapsed Markdown paragraph gaps:
    // one inside the heading chain and one before its wrapped quote.
    for subtitle in ["## a", "a\n==="] {
        let source = format!(
            "```\na\na\n```\n# a\n\n{subtitle}\n\n> aaaaaa aaaaaa aaaaaa",
        );
        let (rows, marks, pages) = render(&source, 59_000)?;
        assert_eq!(pages, 2, "{subtitle}");
        assert_eq!(rows.iter().map(|row| row.page)
            .collect::<Vec<_>>(), [1, 1, 2, 2, 2, 2, 2]);
        assert_eq!(rows.iter().skip(2).map(|row| row.origin_um.1)
            .collect::<Vec<_>>(), [42_500, 55_500, 68_500, 77_500, 86_500]);
        assert!(marks.iter().all(|mark| mark.page == 2));
        let title_marks = if subtitle == "## a" { 7 } else { 8 };
        assert_eq!(marks.iter().skip(title_marks)
            .map(|mark| (mark.start_um.1, mark.end_um.1))
            .collect::<Vec<_>>(), [
                (69_000, 76_900), (78_000, 85_900), (87_000, 94_900),
            ]);
        // Multiple source blanks collapse exactly like one blank.
        let repeated = source.replace("\n\n", "\n\n\n");
        let replay = render(&repeated, 59_000)?;
        assert_eq!(replay.0.iter().map(|row| (
            row.page, row.origin_um, row.text.as_str(),
        )).collect::<Vec<_>>(), rows.iter().map(|row| (
            row.page, row.origin_um, row.text.as_str(),
        )).collect::<Vec<_>>());
        assert_eq!(replay.1, marks);
    }
    // The same internal and terminal gaps preserve authored numbered
    // hanging indents, synthetic bullets and checked-box strokes.
    for marker in ["", "- ", "- [x] ", "1. "] {
        let body = if marker == "1. " {
            "1. a a a a a".to_owned()
        } else {
            format!("{marker}aaaaaa aaaaaa aaaaaa")
        };
        let source = format!(
            "```\na\na\n```\n# a\n\n## a\n\n{body}",
        );
        let (rows, marks, pages) = render(&source, 59_000)?;
        assert_eq!(pages, 2, "{marker}");
        assert_eq!(rows.iter().map(|row| row.page).collect::<Vec<_>>(), [
            1, 1, 2, 2, 2, 2, 2,
        ], "{marker}");
        assert_eq!(rows.iter().skip(2).map(|row| row.origin_um.1)
            .collect::<Vec<_>>(), [
                42_500, 55_500, 68_500, 77_500, 86_500,
            ], "{marker}");
        assert!(marks.iter().all(|mark| mark.page == 2), "{marker}");
        if marker == "1. " {
            assert_eq!(rows[4].text, "1. a a");
            assert_eq!(rows[5].origin_um.0, 50_500);
            assert_eq!(rows[6].origin_um.0, 50_500);
        }
    }
    // Three titles with two *interior* gaps still travel with every
    // wrapped continuation when their combined 66 mm fits a new sheet.
    let (rows, marks, pages) = render(
        "```\na\na\n```\n# a\n\n## a\n\n### a\n\n> aaaaaa aaaaaa aaaaaa",
        71_000,
    )?;
    assert_eq!(pages, 2);
    assert_eq!(rows.iter().map(|row| row.page).collect::<Vec<_>>(), [
        1, 1, 2, 2, 2, 2, 2, 2,
    ]);
    assert_eq!(rows.iter().skip(2).map(|row| row.origin_um.1)
        .collect::<Vec<_>>(), [
            42_500, 55_500, 68_500, 81_500, 90_500, 99_500,
        ]);
    assert!(marks.iter().all(|mark| mark.page == 2));

    // A fenced physical blank row is not a collapsed paragraph gap.
    let (rows, _, _) = render(
        "```\na\na\n```\n# a\n```\n\n```\n## a\n> aaaaaa aaaaaa aaaaaa",
        59_000,
    )?;
    assert_eq!(rows[2].page, 1);
    // A thematic rule still owns its following heading and prevents
    // the later heading/block from pulling it off the original page.
    let (rows, _, _) = render(
        "```\na\n```\n---\n# a\n\n## a\n> aaaaaa aaaaaa aaaaaa",
        59_000,
    )?;
    assert_eq!(rows[1].page, 1);
    // A fresh sheet that cannot fit the gaps and whole block falls
    // back to the prior first-row pagination, never silently clipping.
    let (rows, _, pages) = render(
        "```\na\na\n```\n# a\n\n## a\n\n> aaaaaa aaaaaa aaaaaa",
        50_000,
    )?;
    assert!(pages >= 2);
    assert_eq!(rows.len(), 7);
    Ok(())
}

#[test]
fn heading_block_compaction_preserves_prior_reservations_and_short_sheets()
-> Result<(), String> {
    use atrament_notebook_profile::NotebookProfile;

    let render = |source: &str, height, decoration| -> Result<_, String> {
        super::layout_flow_with_region(
            &FlowSource::new(FlowFormat::Markdown, source)?,
            &NotebookProfile::default(), None, decoration,
            Some(super::FlowRegion {
                left: 30_000, top: 40_000,
                width: 35_000, height,
            }),
        )
    };
    let body = "aaaaaa aaaaaa aaaaaa";
    // A title already reserved by an earlier rule or another title does
    // not detach from it to move with its whole following paragraph.
    for source in [
        format!("```\na\n```\n---\n# a\n{body}"),
        format!("```\na\n```\n# a\n## a\n{body}"),
    ] {
        let (rows, marks, pages) = render(&source, 41_000,
            super::HeadingDecoration::default())?;
        assert_eq!(pages, 2, "{source}");
        assert_eq!(rows[1].page, 1, "{source}");
        assert!(marks.iter().any(|mark| mark.page == 1), "{source}");
        assert!(rows.iter().any(|row| row.page == 2), "{source}");
    }
    // A heading and two physical body rows can join on a fresh sheet.
    // One- or two-row sheets cannot fit the full three-row group.
    let two = "```\na\n```\n# a\naaaaaa aaaaaa";
    let (rows, marks, pages) = render(two, 32_000,
        super::HeadingDecoration::default())?;
    assert_eq!(pages, 2);
    assert_eq!(rows.iter().map(|row| row.page)
        .collect::<Vec<_>>(), [1, 2, 2, 2]);
    assert_eq!(marks.len(), 1);
    assert_eq!(marks[0].page, 2);
    let (rows, marks, pages) = render(two, 23_000,
        super::HeadingDecoration::default())?;
    assert_eq!(pages, 3);
    assert_eq!(rows.iter().map(|row| row.page)
        .collect::<Vec<_>>(), [1, 2, 2, 3]);
    assert_eq!(marks[0].page, 2);
    // A body too large to join its heading uses normal pagination.
    let source = format!("```\na\n```\n# a\n{body}");
    let (rows, marks, pages) = render(&source, 32_000,
        super::HeadingDecoration::default())?;
    assert_eq!(pages, 2);
    assert_eq!(rows.iter().map(|row| row.page)
        .collect::<Vec<_>>(), [1, 1, 1, 2, 2]);
    assert_eq!(marks.len(), 1);
    assert_eq!(marks[0].page, 1);

    // No ornamental stroke is invented under a heading-style none.
    // The heading remains real glyph ink and moves with its body.
    let decoration = super::HeadingDecoration {
        style: Some(super::HeadingStyle::None),
        ..super::HeadingDecoration::default()
    };
    let (rows, marks, pages) = render(&source, 41_000, decoration)?;
    assert_eq!(pages, 2);
    assert_eq!(rows.iter().map(|row| row.page)
        .collect::<Vec<_>>(), [1, 2, 2, 2, 2]);
    assert!(marks.is_empty());
    Ok(())
}

#[test]
fn heading_and_variable_page_fitting_blocks_preserve_full_row_order()
-> Result<(), String> {
    use atrament_notebook_profile::NotebookProfile;

    for count in [2usize, 3, 4, 6, 8, 12] {
        let height = 5_000 + u32::try_from(count + 1)
            .map_err(|error| error.to_string())? * 9_000;
        for prior in [1usize, count / 2, count - 1] {
            let prefix = format!(
                "```\n{}\n```\n# a\n", vec!["a"; prior].join("\n"),
            );
            for marker in ["", "> ", "- "] {
                let source = format!(
                    "{prefix}{marker}{}", vec!["aaaaaa"; count].join(" "),
                );
                let (rows, marks, pages) = super::layout_flow_with_region(
                    &FlowSource::new(FlowFormat::Markdown, &source)?,
                    &NotebookProfile::default(), None,
                    super::HeadingDecoration::default(),
                    Some(super::FlowRegion {
                        left: 30_000, top: 40_000,
                        width: 35_000, height,
                    }),
                )?;
                assert_eq!(pages, 2, "{count}, {prior}, {marker}");
                assert_eq!(rows.len(), prior + count + 1);
                assert!(rows.iter().take(prior)
                    .all(|row| row.page == 1));
                for (index, row) in rows.iter().skip(prior).enumerate() {
                    assert_eq!(row.page, 2);
                    assert_eq!(row.origin_um.1, 42_500 +
                        u32::try_from(index)
                            .map_err(|error| error.to_string())? * 9_000);
                }
                assert_eq!(marks.len(), 1 + if marker == "> " {
                    count
                } else if marker == "- " { 1 } else { 0 });
                assert!(marks.iter().all(|mark| mark.page == 2));
            }
        }
    }
    Ok(())
}

#[test]
fn consecutive_rules_keep_their_first_following_row_across_page_turns()
-> Result<(), String> {
    use atrament_notebook_profile::NotebookProfile;

    let render = |source: &str, height| {
        super::layout_flow_with_region(
            &FlowSource::new(FlowFormat::Markdown, source)?,
            &NotebookProfile::default(), None,
            super::HeadingDecoration::default(),
            Some(super::FlowRegion {
                left: 30_000, top: 40_000,
                width: 35_000, height,
            }),
        )
    };
    // Three rows fit the new page, but only two remain after the
    // independent fenced row. Both separators must follow their ink.
    for following in ["a", "> a", "- a", "# a"] {
        let source = format!("```\na\n```\n---\n***\n{following}");
        let (rows, marks, pages) = render(&source, 32_000)?;
        assert_eq!(pages, 2, "{following}");
        assert_eq!(rows.iter().map(|row| row.page)
            .collect::<Vec<_>>(), [1, 2], "{following}");
        assert_eq!(rows[1].origin_um.1, 60_500, "{following}");
        assert_eq!(marks.iter().take(2)
            .map(|mark| (mark.page, mark.start_um.1))
            .collect::<Vec<_>>(), [(2, 47_000), (2, 56_000)],
            "{following}");
    }
    // Each collapsed gap consumes its own 4-mm measured spacing.
    let spaced = "```\na\n```\n---\n\n***\n\n> a";
    let (rows, marks, pages) = render(spaced, 40_000)?;
    assert_eq!(pages, 2);
    assert_eq!(rows.iter().map(|row| row.page).collect::<Vec<_>>(),
        [1, 2]);
    assert_eq!(rows[1].origin_um.1, 68_500);
    assert_eq!(marks.iter().map(|mark| (mark.page, mark.start_um.1))
        .collect::<Vec<_>>(), [(2, 47_000), (2, 60_000),
            (2, 69_000)]);
    let repeated = spaced.replace("\n\n", "\n\n\n");
    let (again, again_marks, again_pages) = render(&repeated, 40_000)?;
    assert_eq!(again_pages, pages);
    assert_eq!(again_marks, marks);
    assert_eq!(again.iter().map(|row| (row.page, row.origin_um,
        row.text.as_str())).collect::<Vec<_>>(),
        rows.iter().map(|row| (row.page, row.origin_um,
            row.text.as_str())).collect::<Vec<_>>());
    // If even a fresh page cannot hold all three, preserve the
    // existing first-rule/second-rule reservation instead of refusing.
    let (fallback, fallback_marks, _) =
        render("```\na\n```\n---\n***\na", 23_000)?;
    assert_eq!(fallback.iter().map(|row| row.page)
        .collect::<Vec<_>>(), [1, 3]);
    assert_eq!(fallback_marks.iter().map(|mark| mark.page)
        .collect::<Vec<_>>(), [2, 2]);
    // Ending separator chains have no future body to reserve.
    let (trailing, trailing_marks, _) =
        render("```\na\n```\n---\n***", 32_000)?;
    assert_eq!(trailing.len(), 1);
    assert_eq!(trailing_marks.iter().map(|mark| mark.page)
        .collect::<Vec<_>>(), [1, 1]);
    Ok(())
}

#[test]
fn terminal_thematic_rule_chains_move_together_when_one_page_fits()
-> Result<(), String> {
    use atrament_notebook_profile::NotebookProfile;

    let render = |source: &str, height| {
        super::layout_flow_with_region(
            &FlowSource::new(FlowFormat::Markdown, source)?,
            &NotebookProfile::default(), None,
            super::HeadingDecoration::default(),
            Some(super::FlowRegion {
                left: 30_000, top: 40_000,
                width: 35_000, height,
            }),
        )
    };
    // One old row leaves room for only two of three terminal rules.
    // All three fit on the next sheet and should remain contiguous.
    let source = "```\na\n```\n---\n***\n___";
    let (rows, marks, pages) = render(source, 32_000)?;
    assert_eq!(pages, 2);
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].page, 1);
    assert_eq!(marks.iter().map(|mark| (mark.page, mark.start_um.1))
        .collect::<Vec<_>>(), [(2, 47_000), (2, 56_000), (2, 65_000)]);
    // Each collapsed Markdown gap between rules survives the move.
    let (rows, marks, pages) = render(
        "```\na\n```\n---\n\n***\n\n___", 40_000,
    )?;
    assert_eq!(pages, 2);
    assert_eq!(rows[0].page, 1);
    assert_eq!(marks.iter().map(|mark| (mark.page, mark.start_um.1))
        .collect::<Vec<_>>(), [(2, 47_000), (2, 60_000), (2, 73_000)]);
    // Three-rule chains cannot all fit on a two-row sheet, so the
    // ordinary split remains available without refusing valid content.
    let (rows, marks, pages) = render(source, 23_000)?;
    assert_eq!(pages, 3);
    assert_eq!(rows[0].page, 1);
    assert_eq!(marks.iter().map(|mark| mark.page)
        .collect::<Vec<_>>(), [2, 2, 3]);
    Ok(())
}

#[test]
fn terminal_rule_chain_stays_with_unattached_heading_when_group_fits()
-> Result<(), String> {
    use atrament_notebook_profile::NotebookProfile;

    let render = |source: &str, height| {
        super::layout_flow_with_region(
            &FlowSource::new(FlowFormat::Markdown, source)?,
            &NotebookProfile::default(), None,
            super::HeadingDecoration::default(),
            Some(super::FlowRegion {
                left: 30_000, top: 40_000,
                width: 35_000, height,
            }),
        )
    };
    // Four grouped rows fit on a new sheet, not after the fenced ink.
    // The title and all three ending separator marks must follow it.
    for title in ["# a", "a\n==="] {
        let source = format!("```\na\n```\n{title}\n---\n***\n___");
        let (rows, marks, pages) = render(&source, 41_000)?;
        assert_eq!(pages, 2, "{title}");
        assert_eq!(rows.iter().map(|row| (row.page, row.origin_um.1))
            .collect::<Vec<_>>(), [(1, 42_500), (2, 42_500)],
            "{title}");
        assert_eq!(marks.iter().map(|mark| (mark.page, mark.start_um.1))
            .collect::<Vec<_>>(),
            [(2, 49_200), (2, 56_000), (2, 65_000), (2, 74_000)],
            "{title}");
    }
    // A shorter terminal pair should move too when only two of the
    // required three physical rows remain after the preceding ink.
    let (rows, marks, pages) = render(
        "```\na\n```\n# a\n---\n***", 32_000,
    )?;
    assert_eq!(pages, 2);
    assert_eq!(rows.iter().map(|row| (row.page, row.origin_um.1))
        .collect::<Vec<_>>(), [(1, 42_500), (2, 42_500)]);
    assert_eq!(marks.iter().map(|mark| (mark.page, mark.start_um.1))
        .collect::<Vec<_>>(),
        [(2, 49_200), (2, 56_000), (2, 65_000)]);
    // Preserve independent 4-mm gaps, including one after the heading.
    let spaced = "```\na\n```\n# a\n\n---\n\n***\n\n___";
    let (rows, marks, pages) = render(spaced, 53_000)?;
    assert_eq!(pages, 2);
    assert_eq!(rows.iter().map(|row| (row.page, row.origin_um.1))
        .collect::<Vec<_>>(), [(1, 42_500), (2, 42_500)]);
    assert_eq!(marks.iter().map(|mark| (mark.page, mark.start_um.1))
        .collect::<Vec<_>>(),
        [(2, 49_200), (2, 60_000), (2, 73_000), (2, 86_000)]);
    // A three-row sheet cannot contain the entire heading/rule group.
    // Preserve the older first-ink reservation rather than refusing.
    let (rows, marks, pages) = render(
        "```\na\n```\n# a\n---\n***\n___", 32_000,
    )?;
    assert_eq!(pages, 2);
    assert_eq!(rows.iter().map(|row| row.page).collect::<Vec<_>>(),
        [1, 1]);
    assert_eq!(marks.iter().map(|mark| mark.page).collect::<Vec<_>>(),
        [1, 1, 2, 2]);
    Ok(())
}

#[test]
fn terminal_single_rule_stays_with_free_heading_when_pair_fits()
-> Result<(), String> {
    use atrament_notebook_profile::NotebookProfile;

    let render = |source: &str, height| {
        super::layout_flow_with_region(
            &FlowSource::new(FlowFormat::Markdown, source)?,
            &NotebookProfile::default(), None,
            super::HeadingDecoration::default(),
            Some(super::FlowRegion {
                left: 30_000, top: 40_000,
                width: 35_000, height,
            }),
        )
    };
    // The previous fenced row leaves one space on a two-row page.
    // The free terminal title and its one separator fit the next page.
    for title in ["# a", "a\n==="] {
        let source = format!("```\na\n```\n{title}\n---");
        let (rows, marks, pages) = render(&source, 23_000)?;
        assert_eq!(pages, 2, "{title}");
        assert_eq!(rows.iter().map(|row| (row.page, row.origin_um.1))
            .collect::<Vec<_>>(), [(1, 42_500), (2, 42_500)],
            "{title}");
        assert_eq!(marks.iter().map(|mark| (mark.page, mark.start_um.1))
            .collect::<Vec<_>>(), [(2, 49_200), (2, 56_000)],
            "{title}");
    }
    // A collapsed Markdown gap after the title must move with the ink.
    let (rows, marks, pages) = render(
        "```\na\n```\n# a\n\n---", 27_000,
    )?;
    assert_eq!(pages, 2);
    assert_eq!(rows.iter().map(|row| row.page).collect::<Vec<_>>(),
        [1, 2]);
    assert_eq!(marks.iter().map(|mark| (mark.page, mark.start_um.1))
        .collect::<Vec<_>>(), [(2, 49_200), (2, 60_000)]);
    // A one-row sheet cannot hold the group; retain normal pagination.
    let (rows, marks, pages) = render(
        "```\na\n```\n# a\n---", 14_000,
    )?;
    assert_eq!(pages, 3);
    assert_eq!(rows.iter().map(|row| row.page).collect::<Vec<_>>(),
        [1, 2]);
    assert_eq!(marks.iter().map(|mark| mark.page)
        .collect::<Vec<_>>(), [2, 3]);
    Ok(())
}


#[test]
fn adjacent_terminal_heading_and_rule_chains_move_when_group_fits()
-> Result<(), String> {
    use atrament_notebook_profile::NotebookProfile;

    let render = |source: &str, height| {
        super::layout_flow_with_region(
            &FlowSource::new(FlowFormat::Markdown, source)?,
            &NotebookProfile::default(), None,
            super::HeadingDecoration::default(),
            Some(super::FlowRegion {
                left: 30_000, top: 40_000,
                width: 35_000, height,
            }),
        )
    };
    let source = "```\na\n```\n# a\n## a\n---\n***";
    let (rows, marks, pages) = render(source, 41_000)?;
    assert_eq!(pages, 2);
    assert_eq!(rows.iter().map(|row| (row.page, row.origin_um.1))
        .collect::<Vec<_>>(),
        [(1, 42_500), (2, 42_500), (2, 51_500)]);
    assert_eq!(marks.iter().map(|mark| (mark.page, mark.start_um.1))
        .collect::<Vec<_>>(),
        [(2, 49_200), (2, 58_200), (2, 65_000), (2, 74_000)]);
    // Each Markdown paragraph gap remains measured after relocation.
    let spaced = "```\na\n```\n# a\n\n## a\n\n---\n\n***";
    let (rows, marks, pages) = render(spaced, 53_000)?;
    assert_eq!(pages, 2);
    assert_eq!(rows.iter().map(|row| (row.page, row.origin_um.1))
        .collect::<Vec<_>>(),
        [(1, 42_500), (2, 42_500), (2, 55_500)]);
    assert_eq!(marks.iter().map(|mark| (mark.page, mark.start_um.1))
        .collect::<Vec<_>>(),
        [(2, 49_200), (2, 62_200), (2, 73_000), (2, 86_000)]);
    // One terminal separator also belongs with both titles when all
    // three rows fit on a new, not the currently occupied, sheet.
    let (rows, marks, pages) = render(
        "```\na\n```\n# a\n## a\n---", 32_000,
    )?;
    assert_eq!(pages, 2);
    assert_eq!(rows.iter().map(|row| row.page).collect::<Vec<_>>(),
        [1, 2, 2]);
    assert_eq!(marks.iter().map(|mark| (mark.page, mark.start_um.1))
        .collect::<Vec<_>>(),
        [(2, 49_200), (2, 58_200), (2, 65_000)]);
    // Setext is a title row, not an additional separator row.
    let (rows, marks, pages) = render(
        "```\na\n```\n# a\na\n===\n---\n***", 41_000,
    )?;
    assert_eq!(pages, 2);
    assert_eq!(rows.iter().map(|row| row.page).collect::<Vec<_>>(),
        [1, 2, 2]);
    assert_eq!(marks.iter().map(|mark| (mark.page, mark.start_um.1))
        .collect::<Vec<_>>(),
        [(2, 49_200), (2, 58_200), (2, 65_000), (2, 74_000)]);
    // The whole chain is too tall for a three-row sheet. Preserve
    // heading-first-ink fallback and paginate the remaining last rule.
    let (rows, marks, pages) = render(source, 32_000)?;
    assert_eq!(pages, 3);
    assert_eq!(rows.iter().map(|row| row.page).collect::<Vec<_>>(),
        [1, 2, 2]);
    assert_eq!(marks.iter().map(|mark| mark.page)
        .collect::<Vec<_>>(), [2, 2, 2, 3]);
    Ok(())
}

#[test]
fn terminal_interleaved_titles_and_rules_keep_measured_ink_together()
-> Result<(), String> {
    use atrament_notebook_profile::NotebookProfile;

    let render = |source: &str, height| {
        super::layout_flow_with_region(
            &FlowSource::new(FlowFormat::Markdown, source)?,
            &NotebookProfile::default(), None,
            super::HeadingDecoration::default(),
            Some(super::FlowRegion {
                left: 30_000, top: 40_000,
                width: 35_000, height,
            }),
        )
    };
    // The first title and its following rule fit beside the earlier
    // fenced ink; all three terminal rows fit together on a new page.
    for ending in ["## a", "a\n==="] {
        let source = format!("```\na\n```\n# a\n---\n{ending}");
        let (rows, marks, pages) = render(&source, 32_000)?;
        assert_eq!(pages, 2, "{ending}");
        assert_eq!(rows.iter().map(|row| (row.page, row.origin_um.1))
            .collect::<Vec<_>>(),
            [(1, 42_500), (2, 42_500), (2, 60_500)], "{ending}");
        assert_eq!(marks.iter().map(|mark| (mark.page, mark.start_um.1))
            .collect::<Vec<_>>(),
            [(2, 49_200), (2, 56_000), (2, 67_200)], "{ending}");
    }
    // Consecutive blank lines collapse independently on each side of
    // the separator without introducing extra physical rows.
    let spaced = "```\na\n```\n# a\n\n---\n\n## a";
    let (rows, marks, pages) = render(spaced, 40_000)?;
    assert_eq!(pages, 2);
    assert_eq!(rows.iter().map(|row| (row.page, row.origin_um.1))
        .collect::<Vec<_>>(),
        [(1, 42_500), (2, 42_500), (2, 68_500)]);
    assert_eq!(marks.iter().map(|mark| (mark.page, mark.start_um.1))
        .collect::<Vec<_>>(),
        [(2, 49_200), (2, 60_000), (2, 75_200)]);
    let repeated = spaced.replace("\n\n", "\n\n\n");
    let (again, again_marks, again_pages) = render(&repeated, 40_000)?;
    assert_eq!(again_pages, pages);
    assert_eq!(again_marks, marks);
    assert_eq!(again.iter().map(|row| (row.page, row.origin_um))
        .collect::<Vec<_>>(), rows.iter().map(|row|
            (row.page, row.origin_um)).collect::<Vec<_>>());
    // A two-row sheet cannot fit this group: old heading/next-row
    // pairing survives instead of refusing valid Markdown content.
    let (short_rows, short_marks, _) = render(
        "```\na\n```\n# a\n---\n## a", 23_000,
    )?;
    assert_eq!(short_rows.iter().map(|row| row.page)
        .collect::<Vec<_>>(), [1, 2, 3]);
    assert_eq!(short_marks.iter().map(|mark| mark.page)
        .collect::<Vec<_>>(), [2, 2, 3]);
    // Several alternating titles/rules also remain one terminal group;
    // each title still receives its proper native underline hierarchy.
    let (many, ink, count) = render(
        "```\na\n```\n# a\n---\n## a\n***\n### a", 50_000,
    )?;
    assert_eq!(count, 2);
    assert_eq!(many.iter().map(|row| (row.page, row.origin_um.1))
        .collect::<Vec<_>>(),
        [(1, 42_500), (2, 42_500), (2, 60_500), (2, 78_500)]);
    assert_eq!(ink.iter().map(|mark| (mark.page, mark.start_um.1))
        .collect::<Vec<_>>(),
        [(2, 49_200), (2, 56_000), (2, 67_200),
            (2, 74_000), (2, 85_200)]);
    // A real following body ends the terminal-only grouping, even if
    // four rows would fit one fresh sheet with earlier unrelated ink.
    let (ordinary, ordinary_marks, _) = render(
        "```\na\n```\n# a\n---\n## a\na", 41_000,
    )?;
    assert_eq!(ordinary.iter().map(|row| row.page)
        .collect::<Vec<_>>(), [1, 1, 2, 2]);
    assert_eq!(ordinary_marks.iter().map(|mark| mark.page)
        .collect::<Vec<_>>(), [1, 1, 2]);
    Ok(())
}

#[test]
fn six_level_interleaved_terminal_chain_preserves_page_local_ornaments()
-> Result<(), String> {
    use atrament_notebook_profile::NotebookProfile;

    let source = concat!(
        "```\na\n```\n# a\n---\n## a\n***\n### a\n___\n",
        "#### a\n* * *\n##### a\n- - -\n###### a",
    );
    let render = |text: &str, height| -> Result<_, String> {
        super::layout_flow_with_region(
            &FlowSource::new(FlowFormat::Markdown, text)?,
            &NotebookProfile::default(), None,
            super::HeadingDecoration::default(),
            Some(super::FlowRegion {
                left: 30_000, top: 40_000,
                width: 35_000, height,
            }),
        )
    };
    // Eleven terminal physical rows fit a fresh sheet, but not the ten
    // rows left after a separate fenced row. All six title levels and
    // five separators must relocate together with their own ink.
    let (rows, marks, pages) = render(source, 104_000)?;
    assert_eq!(pages, 2);
    assert_eq!(rows.len(), 7);
    assert_eq!(rows.iter().map(|row| (row.page, row.origin_um.1))
        .collect::<Vec<_>>(), [
            (1, 42_500), (2, 42_500), (2, 60_500),
            (2, 78_500), (2, 96_500), (2, 114_500),
            (2, 132_500),
        ]);
    assert_eq!(marks.iter().map(|mark| (
        mark.page, mark.start_um.1, mark.width_um,
    )).collect::<Vec<_>>(), [
        (2, 49_200, 550), (2, 56_000, 400),
        (2, 67_200, 420), (2, 74_000, 400),
        (2, 85_200, 300), (2, 92_000, 400),
        (2, 103_200, 240), (2, 110_000, 400),
        (2, 121_200, 180), (2, 128_000, 400),
        (2, 139_200, 120),
    ]);
    // Each repeated blank source line collapses to one 4-mm gap.
    // Ten independent gaps plus eleven rows fit exactly on page two.
    let tail = source.strip_prefix("```\na\n```\n")
        .ok_or("terminal chain source prefix missing")?;
    let spaced = format!("```\na\n```\n{}", tail.replace('\n', "\n\n"));
    let (spaced_rows, spaced_marks, spaced_pages) =
        render(&spaced, 144_000)?;
    assert_eq!(spaced_pages, 2);
    assert_eq!(spaced_rows.iter().map(|row| (row.page, row.origin_um.1))
        .collect::<Vec<_>>(), [
            (1, 42_500), (2, 42_500), (2, 68_500),
            (2, 94_500), (2, 120_500), (2, 146_500),
            (2, 172_500),
        ]);
    assert_eq!(spaced_marks.iter().map(|mark| mark.start_um.1)
        .collect::<Vec<_>>(), [
            49_200, 60_000, 75_200, 86_000, 101_200, 112_000,
            127_200, 138_000, 153_200, 164_000, 179_200,
        ]);
    let repeated = spaced.replace("\n\n", "\n\n\n");
    let (again_rows, again_marks, again_pages) =
        render(&repeated, 144_000)?;
    assert_eq!(again_pages, spaced_pages);
    assert_eq!(again_marks, spaced_marks);
    assert_eq!(again_rows.iter().map(|row| (
        row.page, row.origin_um, row.text.as_str(),
    )).collect::<Vec<_>>(), spaced_rows.iter().map(|row| (
        row.page, row.origin_um, row.text.as_str(),
    )).collect::<Vec<_>>());

    // The same physical move must retain all six marker hierarchies,
    // including the finer level-five and level-six strokes.
    let (marker_rows, marker_marks, marker_pages) =
        super::layout_flow_with_region(
            &FlowSource::new(FlowFormat::Markdown, source)?,
            &NotebookProfile::default(), None,
            super::HeadingDecoration {
                style: Some(super::HeadingStyle::Marker),
                ..super::HeadingDecoration::default()
            },
            Some(super::FlowRegion {
                left: 30_000, top: 40_000,
                width: 35_000, height: 104_000,
            }),
        )?;
    assert_eq!(marker_pages, 2);
    assert_eq!(marker_rows.iter().map(|row| (
        row.page, row.origin_um, row.text.as_str(),
    )).collect::<Vec<_>>(), rows.iter().map(|row| (
        row.page, row.origin_um, row.text.as_str(),
    )).collect::<Vec<_>>());
    assert_eq!(marker_marks.iter().map(|mark| (
        mark.page, mark.start_um.1, mark.width_um,
    )).collect::<Vec<_>>(), [
        (2, 43_400, 1400), (2, 44_400, 1400),
        (2, 45_400, 1400), (2, 46_400, 1400),
        (2, 56_000, 400),
        (2, 61_500, 1150), (2, 62_500, 1150),
        (2, 63_500, 1150), (2, 74_000, 400),
        (2, 79_900, 950), (2, 80_900, 950),
        (2, 92_000, 400),
        (2, 98_100, 760), (2, 99_000, 760),
        (2, 110_000, 400),
        (2, 116_500, 620), (2, 128_000, 400),
        (2, 135_300, 500),
    ]);
    // If the whole chain cannot fit one sheet, the existing first-row
    // reservation remains in charge; valid narrow paper is not refused.
    let (short_rows, short_marks, short_pages) = render(source, 32_000)?;
    assert!(short_pages > 2);
    assert_eq!(short_rows.len(), 7);
    assert_eq!(short_marks.len(), 11);
    assert_eq!(short_rows[0].page, 1);
    Ok(())
}

#[test]
fn equivalent_markdown_blocks_preserve_layout_across_short_papers()
-> Result<(), String> {
    use atrament_notebook_profile::NotebookProfile;

    // These spellings preserve the same authored content and block types.
    // A blank boundary between generated blocks avoids turning an ordinary
    // thematic rule into a Setext title after an unrelated paragraph.
    let blocks = [
        ("# a", "a\n==="),
        ("## a", "a\n---"),
        ("---", "* * *"),
        ("a a a a", "a\n a a a"),
        ("> a a", "  > a a"),
        ("- a a", "+ a a"),
        ("```\na\n```", "~~~\na\n~~~"),
        ("- [x] a", "+ [X] a"),
        ("# a\n---\n## a", "a\n===\n* * *\na\n---"),
    ];
    let heights = [
        14_000, 23_000, 32_000, 41_000,
        55_000, 68_000, 90_000, 115_000,
    ];
    let mut state = 17_941u64;
    let choices = u64::try_from(blocks.len())
        .map_err(|error| format!("block count: {error}"))?;
    for case in 0..128 {
        let mut canonical = "```\na\n```\n\n".to_owned();
        let mut alternative = "~~~\na\n~~~\n\n".to_owned();
        for index in 0..(case % 8 + 1) {
            state = state.wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1);
            let block = usize::try_from(state % choices)
                .map_err(|error| format!("block index: {error}"))?;
            if index != 0 {
                canonical.push_str("\n\n");
                alternative.push_str("\n\n");
            }
            canonical.push_str(blocks[block].0);
            alternative.push_str(blocks[block].1);
        }
        let height = heights[case % heights.len()];
        let render = |text: &str, marker| -> Result<_, String> {
            let source = FlowSource::new(FlowFormat::Markdown, text)?;
            super::layout_flow_with_region(
                &source, &NotebookProfile::default(), None,
                super::HeadingDecoration {
                    style: if marker {
                        Some(super::HeadingStyle::Marker)
                    } else { None },
                    ..super::HeadingDecoration::default()
                },
                Some(super::FlowRegion {
                    left: 30_000, top: 40_000,
                    width: 35_000, height,
                }),
            )
        };
        for marker in [false, true] {
            let expected = render(&canonical, marker)?;
            let actual = render(&alternative, marker)?;
            let describe = |runs: &[super::TextRun]| {
                runs.iter().map(|row| (
                    row.page, row.origin_um, row.text.clone(),
                    row.heading_ink,
                )).collect::<Vec<_>>()
            };
            assert_eq!(actual.2, expected.2,
                "page count for case {case}, marker={marker}");
            assert_eq!(describe(&actual.0), describe(&expected.0),
                "authored ink for case {case}, marker={marker}");
            assert_eq!(actual.1, expected.1,
                "physical ornaments for case {case}, marker={marker}");
        }
    }
    Ok(())
}

#[test]
fn quoted_blank_markers_match_collapsed_markdown_gaps_across_pages()
-> Result<(), String> {
    use atrament_notebook_profile::NotebookProfile;

    let prefixes = [
        "", "a\n\n", "# a\n", "# a\n\n", "---\n",
        "```\na\n```\n\n", "# a\n---\n", "> a\n\n",
    ];
    let suffixes = ["", "\n\na", "\n# a", "\n\n---\na"];
    for height in [14_000, 18_000, 23_000, 32_000, 41_000,
        50_000, 59_000, 72_000, 100_000] {
        for prefix in prefixes {
            for suffix in suffixes {
                let canonical = format!("{prefix}> a\n\n> b{suffix}");
                let alternatives = [
                    "> a\n>\n> b", "> a\n>\n\n> b",
                    "> a\n>\n\n>\n> b",
                    "> a\n\n>\n> b",
                    "> a\n>\n\n\n>\n\n> b",
                ];
                let render = |text: &str| -> Result<_, String> {
                    super::layout_flow_with_region(
                        &FlowSource::new(FlowFormat::Markdown, text)?,
                        &NotebookProfile::default(), None,
                        super::HeadingDecoration::default(),
                        Some(super::FlowRegion {
                            left: 30_000, top: 40_000,
                            width: 35_000, height,
                        }),
                    )
                };
                let expected = render(&canonical)?;
                let ink = |rows: &[super::TextRun]| {
                    rows.iter().map(|row| (row.page, row.origin_um,
                        row.text.clone(), row.heading_ink))
                        .collect::<Vec<_>>()
                };
                for variant in alternatives {
                    let candidate = format!("{prefix}{variant}{suffix}");
                    let actual = render(&candidate)?;
                    if actual.2 != expected.2 {
                        return Err(format!(
                            "page count differs for {candidate:?}"
                        ));
                    }
                    if ink(&actual.0) != ink(&expected.0) {
                        return Err(format!(
                            "ink rows differ for {candidate:?}"
                        ));
                    }
                    if actual.1 != expected.1 {
                        return Err(format!(
                            "ornaments differ for {candidate:?}"
                        ));
                    }
                }
            }
        }
    }
    let render = |text: &str| -> Result<_, String> {
        super::layout_flow(
            &FlowSource::new(FlowFormat::Markdown, text)?,
            &NotebookProfile::default(), None,
            super::HeadingDecoration::default(),
        )
    };
    for source in [
        ">", ">\n\n> a", "> a\n>\n\n",
        "> a\n>\n\n---\n> b",
        "> a\n>\n\n# b\n> b",
        "> a\n>\n\n- b\n> b",
        "> a\n>\n\n```\n> b\n```\n> c",
        "a\n\n>\n> b",
    ] {
        let error = render(source).err().ok_or_else(|| {
            format!("empty quote unexpectedly accepted: {source:?}")
        })?;
        if !error.contains("empty Markdown block") {
            return Err(format!("{source:?}: {error}"));
        }
    }
    Ok(())
}

#[test]
fn chained_empty_quotes_share_exact_collapsed_gaps_across_pages()
-> Result<(), String> {
    use atrament_notebook_profile::NotebookProfile;

    let prefixes = ["", "a\n\n", "# a\n\n", "---\n", "> a\n\n"];
    let suffixes = ["", "\n\na", "\n\n# a", "\n---\na"];
    let gaps = ["\n\n", "\n>\n", "\n>\n\n", "\n\n>\n",
        "\n>\n\n>\n", "\n  >\n\n> \n"];
    let render = |source: &str, height| -> Result<_, String> {
        super::layout_flow_with_region(
            &FlowSource::new(FlowFormat::Markdown, source)?,
            &NotebookProfile::default(), None,
            super::HeadingDecoration::default(),
            Some(super::FlowRegion {
                left: 30_000, top: 40_000, width: 35_000, height,
            }),
        )
    };
    let ink = |rows: &[super::TextRun]| rows.iter().map(|row| (
        row.page, row.origin_um, row.text.clone(), row.heading_ink,
    )).collect::<Vec<_>>();
    let repeated = "\u{f3}".repeat(5);
    let wrapped_middle = format!("{repeated} {repeated} {repeated}");
    for height in [14_000, 18_000, 23_000, 32_000, 41_000,
        50_000, 59_000, 100_000] {
        for prefix in prefixes {
            for suffix in suffixes {
                for middle in ["\u{f3}", wrapped_middle.as_str()] {
                    let canonical = format!(
                        "{prefix}> a\n\n> {middle}\n\n> a{suffix}"
                    );
                    let reference = render(&canonical, height)?;
                    for first_gap in gaps {
                        for last_gap in gaps {
                            let candidate = format!(
                                "{prefix}> a{first_gap}> {middle}\
                                 {last_gap}> a{suffix}"
                            );
                            let actual = render(&candidate, height)?;
                            if actual.2 != reference.2
                                || ink(&actual.0) != ink(&reference.0)
                                || actual.1 != reference.1 {
                                return Err(format!(
                                    "chained quote gap changed physical ink: \
                                     height={height}, {candidate:?}"
                                ));
                            }
                        }
                    }
                }
            }
        }
    }
    Ok(())
}

#[test]
fn empty_quote_separator_requires_uninterrupted_quoted_ink()
-> Result<(), String> {
    use atrament_notebook_profile::NotebookProfile;

    let paper = NotebookProfile::default();
    let render = |text: &str| -> Result<_, String> {
        let source = FlowSource::new(FlowFormat::Markdown, text)?;
        super::layout_flow(
            &source, &paper, None, super::HeadingDecoration::default(),
        )
    };
    for (boundary, invalid) in [
        ("thematic rule", "> a\n---\n>\n> b"),
        ("ATX heading", "> a\n# c\n>\n> b"),
        ("literal indented row", "> a\n    c\n>\n> b"),
        ("fenced code", "> a\n```\nc\n```\n>\n> b"),
        ("empty fenced code", "> a\n```\n```\n>\n> b"),
    ] {
        match render(invalid) {
            Err(error) if error.contains("empty Markdown block refused") => {},
            Err(error) => return Err(format!(
                "unexpected {boundary} refusal: {error}"
            )),
            Ok(_) => return Err(format!(
                "empty quote crossed {boundary}: {invalid:?}"
            )),
        }
    }
    for valid in [
        "> a\n>\n> b",
        "> a\n\n>\n> b",
        "> a  \n>\n> b",
    ] {
        let (rows, bars, pages) = render(valid)?;
        if pages != 1 || rows.len() != 2 || bars.len() != 2
            || rows[1].origin_um.1.checked_sub(rows[0].origin_um.1)
                != Some(13_000) {
            return Err(format!(
                "legitimate quote gap lost: {valid:?}: {rows:?}"
            ));
        }
    }
    Ok(())
}

#[test]
fn hard_break_only_quote_line_cannot_masquerade_as_empty_separator()
-> Result<(), String> {
    use atrament_notebook_profile::NotebookProfile;

    let paper = NotebookProfile::default();
    let render = |text: &str| -> Result<_, String> {
        let source = FlowSource::new(FlowFormat::Markdown, text)?;
        super::layout_flow(
            &source, &paper, None, super::HeadingDecoration::default(),
        )
    };
    for valid in [">a\n>\n>b", ">a\n>  \n>b", ">a\n>\n\n>b"] {
        let (rows, bars, pages) = render(valid)?;
        let texts = rows.iter().map(|row| row.text.as_str())
            .collect::<Vec<_>>();
        let first = rows.first().ok_or("missing first quoted row")?;
        let second = rows.get(1).ok_or("missing second quoted row")?;
        if pages != 1 || texts != ["a", "b"] || bars.len() != 2
            || second.origin_um.1.checked_sub(first.origin_um.1)
                != Some(13_000) {
            return Err(format!(
                "unexpected physical quote spacing for {valid:?}: {rows:?}"
            ));
        }
    }
    for invalid in [
        ">a\n>\\\n>b", ">a\n> \\\n>b",
        ">a\n>\\  \n>b", ">a\n  >\\\n>b",
    ] {
        match render(invalid) {
            Err(error) if error.contains("empty Markdown block refused") => {},
            Err(error) => return Err(format!(
                "unexpected refusal for {invalid:?}: {error}"
            )),
            Ok(_) => return Err(format!(
                "backslash-only quote unexpectedly admitted: {invalid:?}"
            )),
        }
    }
    Ok(())
}

#[test]
fn markdown_hard_break_forms_match_across_structural_page_boundaries()
-> Result<(), String> {
    use atrament_notebook_profile::NotebookProfile;

    let examples = [
        ("a", "b", "a\\\nb", "a  \nb"),
        ("> a", "> b", "> a\\\n> b", "> a  \n> b"),
        ("- a", "  b", "- a\\\n  b", "- a  \n  b"),
        ("1. a", "  b", "1. a\\\n  b", "1. a  \n  b"),
        ("- [x] a", "  b", "- [x] a\\\n  b", "- [x] a  \n  b"),
    ];
    let visual = |rows: &[super::TextRun]| rows.iter().map(|row| (
        row.page, row.origin_um, row.text.clone(), row.heading_ink,
    )).collect::<Vec<_>>();
    for (first, last, slash, spaces) in examples {
        for height in [14_000, 18_000, 23_000, 32_000, 41_000] {
            for prefix in ["", "# a\n", "a\\\n", "---\n"] {
                let render = |source: &str| -> Result<_, String> {
                    super::layout_flow_with_region(
                        &FlowSource::new(FlowFormat::Markdown, source)?,
                        &NotebookProfile::default(), None,
                        super::HeadingDecoration::default(),
                        Some(super::FlowRegion {
                            left: 30_000, top: 40_000,
                            width: 35_000, height,
                        }),
                    )
                };
                let reference = render(&format!("{prefix}{spaces}"))?;
                let candidate = render(&format!("{prefix}{slash}"))?;
                if candidate.2 != reference.2
                    || visual(&candidate.0) != visual(&reference.0)
                    || candidate.1 != reference.1 {
                    return Err(format!(
                        "hard break mismatch {first:?} / {last:?} at \
                         height={height}, prefix={prefix:?}"
                    ));
                }
            }
        }
    }
    Ok(())
}

#[test]
fn unicode_hard_breaks_match_source_newlines_in_literal_fences()
-> Result<(), String> {
    use atrament_notebook_profile::NotebookProfile;

    let source = |text: &str| -> Result<_, String> {
        super::layout_flow_with_region(
            &FlowSource::new(FlowFormat::Markdown, text)?,
            &NotebookProfile::default(), None,
            super::HeadingDecoration::default(),
            Some(super::FlowRegion {
                left: 30_000, top: 40_000,
                width: 35_000, height: 14_000,
            }),
        )
    };
    let reference = source("```\na\nb\n```")?;
    let unicode = source("```\na\u{2028}b\n```")?;
    let geometry = |rows: &[super::TextRun]| rows.iter().map(|run| (
        run.text.clone(), run.page, run.origin_um,
    )).collect::<Vec<_>>();
    if geometry(&unicode.0) != geometry(&reference.0)
        || unicode.1 != reference.1 || unicode.2 != reference.2 {
        return Err("Unicode fence line break changed physical ink".into());
    }
    Ok(())
}

#[test]
fn empty_quote_separators_respect_every_intervening_block_kind()
-> Result<(), String> {
    use atrament_notebook_profile::NotebookProfile;

    let region = super::FlowRegion {
        left: 30_000, top: 40_000, width: 35_000, height: 23_000,
    };
    let render = |source: &str| -> Result<_, String> {
        super::layout_flow_with_region(
            &FlowSource::new(FlowFormat::Markdown, source)?,
            &NotebookProfile::default(), None,
            super::HeadingDecoration::default(), Some(region),
        )
    };
    let boundaries = [
        ("body", "b"),
        ("soft body", "b\nb"),
        ("explicit body break", "b\\"),
        ("Setext H1", "b\n==="),
        ("Setext H2", "b\n---"),
        ("ordered item", "1. b"),
        ("ordered continuation", "1. b\n  b"),
        ("checked task", "- [x] b"),
        ("unordered item", "* b"),
        ("escaped heading", "\\# b"),
        ("escaped quote", "\\> b"),
        ("literal four-space ink", "    b"),
        ("fenced tilde ink", "~~~\nb\n~~~"),
        ("empty fenced tilde", "~~~\n~~~"),
        ("fenced quoted-looking ink", "```\n> b\n```"),
    ];
    for (label, intervening) in boundaries {
        for newline in ["\n", "\r\n"] {
            for separator in [">", "> ", "  >", "  >  "] {
                let source = format!(
                    "> a\n{intervening}\n{separator}\n> \u{f3}"
                ).replace('\n', newline);
                let Err(error) = render(&source) else {
                    return Err(format!(
                        "{label} did not end quote adjacency: {source:?}"
                    ));
                };
                if !error.contains("empty Markdown block refused") {
                    return Err(format!(
                        "wrong {label} refusal for {source:?}: {error}"
                    ));
                }
            }
        }
    }
    for quoted in ["> a", "> a\n> b", "> a\n  b",
        "> a\\", "> a  "] {
        for newline in ["\n", "\r\n"] {
            let marker = format!("{quoted}\n>\n> \u{f3}")
                .replace('\n', newline);
            let blank = format!("{quoted}\n\n> \u{f3}").replace('\n', newline);
            let first = render(&marker)?;
            let second = render(&blank)?;
            let rows = |runs: &[super::TextRun]| runs.iter()
                .map(|row| (row.page, row.origin_um, row.text.clone(),
                    row.heading_ink))
                .collect::<Vec<_>>();
            if rows(&first.0) != rows(&second.0)
                || first.1 != second.1 || first.2 != second.2 {
                return Err(format!(
                    "valid quoted gap changed physical ink: {marker:?}"
                ));
            }
        }
    }
    Ok(())
}

#[test]
fn setext_atx_equivalence_survives_quoted_paragraph_gaps()
-> Result<(), String> {
    use atrament_notebook_profile::NotebookProfile;

    for (prefix, title_atx, title_setext) in [
        ("", "# a", "a\n==="),
        ("", "## a", "a\n---"),
        ("a\n\n", "# a", "a\n==="),
        ("- a\n", "# a", "a\n==="),
        ("> a\n", "# a", "a\n==="),
        ("---\n", "# a", "a\n==="),
    ] {
        for height in [14_000, 18_000, 23_000, 32_000, 41_000,
            50_000, 59_000] {
            for gap in ["", "\n"] {
                for quote in ["> a\n>\n> \u{f3}", "> a\n\n> \u{f3}",
                    "> a\n  b\n>\n> \u{f3}"] {
                    let render = |title: &str| -> Result<_, String> {
                        let text = format!("{prefix}{title}\n{gap}{quote}");
                        super::layout_flow_with_region(
                            &FlowSource::new(FlowFormat::Markdown, &text)?,
                            &NotebookProfile::default(), None,
                            super::HeadingDecoration::default(),
                            Some(super::FlowRegion {
                                left: 30_000, top: 40_000,
                                width: 35_000, height,
                            }),
                        )
                    };
                    let left = render(title_atx)?;
                    let right = render(title_setext)?;
                    let ink = |rows: &[super::TextRun]| rows.iter().map(|row| (
                        row.page, row.origin_um, row.text.clone(),
                        row.heading_ink,
                    )).collect::<Vec<_>>();
                    if ink(&left.0) != ink(&right.0)
                        || left.1 != right.1 || left.2 != right.2 {
                        return Err(format!(
                            "ATX/Setext quote-gap mismatch: prefix={prefix:?}, \
                             height={height}, gap={gap:?}, quote={quote:?}"
                        ));
                    }
                }
            }
        }
    }
    Ok(())
}

#[test]
fn wrapped_quoted_hard_breaks_and_gaps_keep_setext_title_geometry()
-> Result<(), String> {
    use atrament_notebook_profile::NotebookProfile;

    let paper = NotebookProfile::default();
    let placement = |source: &str, height: u32,
        decoration: super::HeadingDecoration| -> Result<_, String> {
        super::layout_flow_with_region(
            &FlowSource::new(FlowFormat::Markdown, source)?,
            &paper, None, decoration,
            Some(super::FlowRegion {
                left: 30_000, top: 40_000,
                width: 35_000, height,
            }),
        )
    };
    let geometry = |rows: &[super::TextRun]| rows.iter().map(|run| (
        run.text.clone(), run.page, run.origin_um, run.heading_ink,
    )).collect::<Vec<_>>();
    let mut turns = 0;
    let mut wrapped = 0;
    for (atx, setext) in [
        ("# a", "a\n==="),
        ("## a", "a\n---"),
    ] {
        for preceding in ["", "a\n\n", "---\n", "- a\n"] {
            for body in [
                "> a a a a a a a a a a",
                "> a a a a\\\n> a a a a a a",
                "> a a a a  \n> a a a a a a",
            ] {
                for separator in ["\n", ">\n", ">\n\n>\n"] {
                    let suffix = format!("{body}\n{separator}> ó a a a a a a");
                    for height in [14_000, 18_000, 23_000, 32_000,
                        41_000, 50_000, 59_000, 72_000] {
                        for marker in [false, true] {
                            let decoration = super::HeadingDecoration {
                                style: marker.then_some(
                                    super::HeadingStyle::Marker,
                                ),
                                ..super::HeadingDecoration::default()
                            };
                            let first = placement(&format!(
                                "{preceding}{atx}\n{suffix}"
                            ), height, decoration)?;
                            let second = placement(&format!(
                                "{preceding}{setext}\n{suffix}"
                            ), height, decoration)?;
                            if geometry(&first.0) != geometry(&second.0)
                                || first.1 != second.1 || first.2 != second.2 {
                                return Err(format!(
                                    "ATX/Setext wrapped quote mismatch: \
                                     {preceding:?}, {atx:?}, {suffix:?}, \
                                     height={height}, marker={marker}"
                                ));
                            }
                            turns += usize::from(first.2 > 1);
                            wrapped += usize::from(first.0.iter().any(|row| {
                                row.origin_um.0 == 38_500 && row.text.len() > 2
                            }));
                        }
                    }
                }
            }
        }
    }
    if turns <= 100 || wrapped <= 100 {
        return Err(format!(
            "insufficient page turns ({turns}) or wrapped quotes ({wrapped})"
        ));
    }
    Ok(())
}
