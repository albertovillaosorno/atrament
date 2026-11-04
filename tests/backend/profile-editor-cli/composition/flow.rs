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
fn measured_words_keep_nfc_nfd_and_joined_unicode_graphemes() {
    let word = "aóo\u{301}👩\u{200d}🏫";
    let (glyphs, width) = super::measured_word(word, None)
        .expect("valid authored Unicode word");
    let segments = glyphs.iter().map(|(text, _, _)| *text)
        .collect::<Vec<_>>();
    assert_eq!(segments, ["a", "ó", "o\u{301}", "👩\u{200d}🏫"]);
    assert_eq!(width, 16_000);
    assert!(glyphs.iter().all(|(_, advance, _)| *advance == 4_000));

    // Bounded input can still contain thousands of short graphemes;
    // measure each exact span once rather than rescanning its prefix.
    let long = "o\u{301}".repeat(1_024);
    let (many, width) = super::measured_word(&long, None)
        .expect("bounded decomposed source");
    assert_eq!(many.len(), 1_024);
    assert_eq!(width, 4_096_000);
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
