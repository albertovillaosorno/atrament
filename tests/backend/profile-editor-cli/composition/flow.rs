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
