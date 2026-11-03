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
//   - Native Rust handwriting profile CLI accepted/refused regressions.
// - Must-Not:
//   - Claim handwriting glyph rendering or full notebook editing.
// - Allows:
//   - Inputs: Bounded UTF-8 profiles and explicit test-owned file paths.
//   - Outputs: Exact profile roundtrips and refusal assertions.
//   - Side effects: Test-owned temporary profile creation and removal.
// - Split-When:
//   - Localhost profile transport gains its own test surface.
// - Merge-When:
//   - A complete native editor integration suite subsumes these fixtures.
// - Summary:
//   - Proves same Rust schema is used for edits, file saves, and refusals.
// - Description:
//   - Tests global/Unicode glyph mutations, invalid statistics, write safety.
// - Usage:
//   - cargo test -p atrament_profile_editor_cli
// - Defaults:
//   - Only exact test-owned files may be removed.
//

//! Tests for the native shared Rust profile entrypoint.

#![expect(
    clippy::expect_used,
    reason = "fail-fast test assertions over CLI profile contracts"
)]

use super::*;
use std::fs;
use std::path::PathBuf;
use std::process;
use std::sync::atomic::{AtomicU64, Ordering};
static NEXT_FILE: AtomicU64 = AtomicU64::new(0);

fn args(values: &[&str]) -> Vec<String> {
    values.iter().map(|value| (*value).to_owned()).collect()
}

fn test_path() -> PathBuf {
    env::temp_dir().join(format!(
        "atrament-handwriting-cli-{}-{}.json",
        process::id(),
        NEXT_FILE.fetch_add(1, Ordering::Relaxed)
    ))
}

#[test]
fn baseline_round_trip_and_inspection_use_single_codec() {
    let baseline = execute_handwriting(&args(&["--seed", "500"]))
        .expect("default baseline accepted")
        .expect("stdout JSON");
    let decoded =
        decode_statistics_json(&baseline, &UnicodeGraphemeSegmentation)
            .expect("canonical JSON reopens");
    assert_eq!(decoded.seed, 500);
    assert_eq!(decoded.globals.len(), Dimension::ALL.len());
    assert!(decoded.glyphs.is_empty());
    let inspected =
        parse_handwriting_options(&args(&["--seed", "500", "--inspect"]))
            .expect("inspection admitted");
    let display = String::from_utf8(inspect_handwriting(&inspected.profile))
        .expect("inspection is readable");
    assert!(display.contains("Seed: 500"));
    assert!(display.contains("Glyph overrides: 0"));
    assert!(display.contains("height (micrometres):"));
}

#[test]
fn global_and_decomposed_accent_glyph_round_trip() {
    let edited = execute_handwriting(&args(&[
        "--set-global",
        "height",
        "2500",
        "3500",
        "3000",
        "3000",
        "200",
        "--set-glyph",
        "n\u{303}",
        "slant",
        "-1000",
        "1000",
        "0",
        "0",
        "400",
        "--set-glyph",
        "\u{f1}",
        "width",
        "1800",
        "2600",
        "2200",
        "2200",
        "200",
    ]))
    .expect("valid glyph edits")
    .expect("stdout");
    let decoded = decode_statistics_json(&edited, &UnicodeGraphemeSegmentation)
        .expect("same Rust codec decodes");
    assert_eq!(decoded.glyphs.len(), 2);
    assert_eq!(
        decoded.glyphs.first().expect("first glyph").grapheme,
        "n\u{303}"
    );
    assert_eq!(
        decoded.glyphs.get(1).expect("second glyph").grapheme,
        "\u{f1}"
    );
    assert_eq!(
        decoded
            .statistics_for("\u{f1}", Dimension::Width)
            .expect("glyph width")
            .mean,
        2_200i32
    );
    assert_eq!(
        decoded
            .statistics_for("n\u{303}", Dimension::Slant)
            .expect("glyph slant")
            .standard_deviation,
        400
    );
}

#[test]
fn invalid_stats_and_unknown_flags_refuse_before_file_effect() {
    for values in [
        args(&[
            "--set-global",
            "height",
            "2500",
            "3500",
            "4500",
            "3000",
            "200",
        ]),
        args(&[
            "--set-glyph",
            "ab",
            "width",
            "2000",
            "2200",
            "2100",
            "2100",
            "20",
        ]),
        args(&["--set-glyph", "\u{f1}", "bogus", "1", "1", "1", "1", "0"]),
        args(&["--set-global", "width", "1", "2"]),
        args(&["--seed", "-1"]),
        args(&["--replace"]),
        args(&["--inspect", "--output", "invalid.json"]),
        args(&["--nonexistent"]),
    ] {
        assert!(execute_handwriting(&values).is_err(), "{values:?}");
    }
}

#[test]
fn preserve_existing_on_refused_update_and_allow_explicit_replacement() {
    let path = test_path();
    let target = path.to_str().expect("test path UTF-8");
    let initial = args(&["--seed", "8", "--output", target]);
    assert!(
        execute_handwriting(&initial)
            .expect("first write")
            .is_none()
    );
    let original = fs::read(&path).expect("file committed");
    let replacing =
        args(&["--profile", target, "--seed", "9", "--output", target]);
    assert!(execute_handwriting(&replacing).is_err());
    assert_eq!(fs::read(&path).expect("still present"), original);
    let mut approved = replacing;
    approved.push("--replace".into());
    assert!(execute_handwriting(&approved).expect("replace").is_none());
    let revised = fs::read(&path).expect("new file");
    assert_ne!(revised, original);
    assert_eq!(
        decode_statistics_json(&revised, &UnicodeGraphemeSegmentation)
            .expect("saved profile valid")
            .seed,
        9
    );
    fs::remove_file(path).expect("owned temp removed");
}

#[test]
fn corrupt_and_oversized_sources_do_not_modify_output() {
    let source = test_path();
    let output = test_path();
    let source_name = source.to_str().expect("source path UTF-8");
    let output_name = output.to_str().expect("output path UTF-8");
    fs::write(&output, b"preserved").expect("sentinel output");
    for invalid in [
        b"not json".to_vec(),
        vec![b' '; MAXIMUM_STATISTICS_JSON_BYTES + 1],
        br#"{"version":"atrament.handwriting.statistics.v9"}"#.to_vec(),
    ] {
        fs::write(&source, invalid).expect("invalid fixture");
        let attempt = args(&[
            "--profile",
            source_name,
            "--output",
            output_name,
            "--replace",
        ]);
        assert!(execute_handwriting(&attempt).is_err());
        assert_eq!(
            fs::read(&output).expect("sentinel persisted"),
            b"preserved"
        );
    }
    fs::remove_file(source).expect("owned source removed");
    fs::remove_file(output).expect("owned output removed");
}

#[cfg(unix)]
#[test]
fn cannot_replace_symlink_even_with_explicit_replace() {
    use std::os::unix::fs::symlink;
    let actual = test_path();
    let link = test_path();
    fs::write(&actual, b"original").expect("actual source");
    symlink(&actual, &link).expect("link fixture");
    let attempt = args(&[
        "--seed",
        "10",
        "--output",
        link.to_str().expect("link path"),
        "--replace",
    ]);
    assert!(execute_handwriting(&attempt).is_err());
    assert_eq!(fs::read(&actual).expect("actual unchanged"), b"original");
    fs::remove_file(link).expect("owned link removed");
    fs::remove_file(actual).expect("owned file removed");
}

fn glyph_removal_fixture() -> PathBuf {
    let source = test_path();
    let target = source.to_str().expect("input filename");
    let initial = args(&[
        "--set-glyph",
        "\u{f1}",
        "height",
        "2500",
        "3500",
        "3000",
        "3000",
        "100",
        "--set-glyph",
        "\u{f1}",
        "slant",
        "-1000",
        "1000",
        "0",
        "0",
        "100",
        "--set-glyph",
        "n\u{303}",
        "height",
        "2500",
        "3500",
        "3000",
        "3000",
        "100",
        "--output",
        target,
    ]);
    assert!(
        execute_handwriting(&initial)
            .expect("initial save")
            .is_none()
    );
    source
}

#[test]
fn profile_reader_rejects_directory_before_decoding() {
    let directory = env::temp_dir();
    let name = directory.to_str().expect("temporary directory UTF-8");
    let error = read_handwriting_profile(name)
        .expect_err("a directory is not a regular profile file");
    assert!(error.contains("NotRegularFile"));
}

#[test]
fn remove_glyph_cli_round_trip_preserves_exact_nfd_key() {
    let source = glyph_removal_fixture();
    let output = test_path();
    let input_name = source.to_str().expect("input filename");
    let output_name = output.to_str().expect("output filename");
    let removal = args(&[
        "--profile",
        input_name,
        "--remove-glyph-dimension",
        "\u{f1}",
        "height",
        "--output",
        output_name,
    ]);
    assert!(
        execute_handwriting(&removal)
            .expect("updated save")
            .is_none()
    );
    let decoded = decode_statistics_json(
        &fs::read(&output).expect("saved JSON"),
        &UnicodeGraphemeSegmentation,
    )
    .expect("same native codec");
    assert_eq!(decoded.glyphs.len(), 2);
    assert_eq!(
        decoded.statistics_for("\u{f1}", Dimension::Height),
        decoded.statistics_for("z", Dimension::Height),
    );
    assert_eq!(
        decoded
            .statistics_for("\u{f1}", Dimension::Slant)
            .expect("slant override")
            .standard_deviation,
        100,
    );
    assert_ne!(
        decoded.statistics_for("n\u{303}", Dimension::Height),
        decoded.statistics_for("\u{f1}", Dimension::Height),
    );
    let full = execute_handwriting(&args(&[
        "--profile",
        input_name,
        "--remove-glyph",
        "\u{f1}",
    ]))
    .expect("whole glyph removal")
    .expect("stdout JSON");
    let no_glyph = decode_statistics_json(&full, &UnicodeGraphemeSegmentation)
        .expect("whole glyph accepted");
    assert_eq!(no_glyph.glyphs.len(), 1);
    assert_eq!(
        no_glyph.glyphs.first().expect("remaining glyph").grapheme,
        "n\u{303}",
    );
    fs::remove_file(source).expect("remove owned input");
    fs::remove_file(output).expect("remove owned output");
}

#[test]
fn invalid_glyph_removal_preserves_existing_target_bytes() {
    let source = glyph_removal_fixture();
    let output = test_path();
    let input_name = source.to_str().expect("input filename");
    let output_name = output.to_str().expect("output filename");
    fs::write(&output, b"preserved").expect("sentinel output");
    for bad in [
        args(&["--profile", input_name, "--remove-glyph", "a"]),
        args(&[
            "--profile",
            input_name,
            "--remove-glyph-dimension",
            "\u{f1}",
            "width",
        ]),
        args(&[
            "--profile",
            input_name,
            "--remove-glyph-dimension",
            "\u{f1}",
            "unknown",
        ]),
        args(&["--remove-glyph", "\u{f1}", "--profile", input_name]),
        args(&["--profile", input_name, "--remove-glyph"]),
    ] {
        let mut command = bad;
        command.extend(args(&["--output", output_name, "--replace"]));
        assert!(execute_handwriting(&command).is_err());
        assert_eq!(fs::read(&output).expect("unchanged target"), b"preserved");
    }
    fs::remove_file(source).expect("remove owned input");
    fs::remove_file(output).expect("remove owned output");
}

#[cfg(unix)]
#[test]
fn handwriting_profile_named_pipe_refuses_without_blocking() {
    use nix::sys::stat::Mode;
    use nix::unistd::mkfifo;

    let path = test_path().with_extension("fifo");
    mkfifo(&path, Mode::S_IRUSR | Mode::S_IWUSR)
        .expect("create a test-owned named pipe");
    let name = path.to_str().expect("UTF-8 pipe path");
    let error = read_handwriting_profile(name)
        .expect_err("nonregular profile path must refuse before open");
    assert!(error.contains("NotRegularFile"));
    fs::remove_file(&path).expect("remove test-owned pipe");
}

#[test]
fn notebook_cli_round_trips_selected_file_and_modified_copy() {
    let source = test_path();
    let copy = test_path();
    let source_name = source.to_str().expect("source filename");
    let copy_name = copy.to_str().expect("copy filename");
    let initial = args(&[
        "--date-style",
        "box",
        "--date-placement",
        "top-right",
        "--border-radius-um",
        "4500",
        "--page-numbers",
        "true",
        "--printed-sheet",
        "true",
        "--output",
        source_name,
    ]);
    assert!(
        execute_notebook(&initial)
            .expect("saved notebook")
            .is_none()
    );
    let saved = fs::read(&source).expect("canonical JSON saved");
    let reopen = execute_notebook(&args(&["--profile", source_name]))
        .expect("native load")
        .expect("stdout profile");
    assert_eq!(saved, reopen);
    let parsed = NotebookProfile::from_json(&reopen).expect("valid notebook");
    assert_eq!(parsed.date_style, DateStyle::Box);
    assert_eq!(parsed.date_placement, DatePlacement::TopRight);
    assert_eq!(parsed.border_radius_um, 4_500);
    assert!(parsed.page_numbers);
    assert!(parsed.printed_sheet);
    let edit = args(&[
        "--profile",
        source_name,
        "--page-numbers",
        "false",
        "--output",
        copy_name,
    ]);
    assert!(execute_notebook(&edit).expect("copy saved").is_none());
    let edited = NotebookProfile::from_json(&fs::read(&copy).expect("copy"))
        .expect("valid modified copy");
    assert!(!edited.page_numbers);
    assert!(edited.printed_sheet);
    assert_eq!(fs::read(&source).expect("source kept"), saved);
    fs::remove_file(source).expect("owned source removed");
    fs::remove_file(copy).expect("owned copy removed");
}

#[test]
fn notebook_cli_imports_canonical_rust_paper_without_divergent_schema() {
    let source = test_path();
    let name = source.to_str().expect("paper filename");
    let canonical_paper =
        atrament_notebook_paper_cli::default_paper_profile_json()
            .replace("\"style\": \"grid\"", "\"style\": \"ruled\"");
    fs::write(&source, &canonical_paper).expect("paper fixture");
    let nested = execute_notebook(&args(&[
        "--paper-profile",
        name,
        "--date-style",
        "line",
        "--date-placement",
        "top-left",
        "--inspect",
    ]))
    .expect("inspect accepted")
    .expect("stdout inspection");
    assert!(String::from_utf8_lossy(&nested).contains("Date: line / top-left"));
    let profile = execute_notebook(&args(&["--paper-profile", name]))
        .expect("paper accepted")
        .expect("stdout JSON");
    assert_eq!(
        NotebookProfile::from_json(&profile)
            .expect("notebook codec")
            .paper_json,
        canonical_paper,
    );
    fs::remove_file(source).expect("owned fixture removed");
}

#[test]
fn notebook_cli_refusals_preserve_output_and_source() {
    let source = test_path();
    let output = test_path();
    let source_name = source.to_str().expect("source filename");
    let output_name = output.to_str().expect("output filename");
    let baseline = execute_notebook(&args(&[]))
        .expect("default")
        .expect("JSON");
    fs::write(&source, baseline).expect("valid source");
    fs::write(&output, b"preserved").expect("sentinel output");
    for invalid in [
        args(&["--profile", source_name, "--date-style", "box"]),
        args(&["--profile", source_name, "--border-radius-um", "999999"]),
        args(&["--profile", source_name, "--page-numbers", "sometimes"]),
        args(&["--profile", source_name, "--date-placement", "bottom"]),
        args(&["--date-style", "box", "--profile", source_name]),
        args(&["--border-radius-um", "-1"]),
        args(&["--paper-profile", source_name]),
        args(&["--unknown"]),
    ] {
        let mut attempt = invalid;
        attempt.extend(args(&["--output", output_name, "--replace"]));
        assert!(execute_notebook(&attempt).is_err(), "{attempt:?}");
        assert_eq!(fs::read(&output).expect("unmodified output"), b"preserved");
    }
    assert_eq!(
        NotebookProfile::from_json(&fs::read(&source).expect("source intact"))
            .expect("source still decodes"),
        NotebookProfile::default(),
    );
    assert!(
        execute_notebook(&args(&[
            "--profile",
            source_name,
            "--output",
            output_name,
        ]))
        .is_err()
    );
    assert!(
        execute_notebook(&args(&[
            "--profile",
            source_name,
            "--output",
            output_name,
            "--replace",
        ]))
        .expect("explicitly replaced")
        .is_none()
    );
    assert_eq!(
        fs::read(&output).expect("replacement"),
        fs::read(&source).expect("source")
    );
    fs::remove_file(source).expect("owned source removed");
    fs::remove_file(output).expect("owned output removed");
}

#[cfg(unix)]
#[test]
fn notebook_profile_named_pipe_refuses_before_open() {
    use nix::sys::stat::Mode;
    use nix::unistd::mkfifo;

    let path = test_path().with_extension("fifo");
    mkfifo(&path, Mode::S_IRUSR | Mode::S_IWUSR).expect("owned FIFO");
    let name = path.to_str().expect("FIFO filename");
    let refusal = execute_notebook(&args(&["--profile", name]));
    assert!(
        refusal
            .expect_err("FIFO refused")
            .contains("NotRegularFile")
    );
    fs::remove_file(path).expect("owned FIFO removed");
}

#[test]
fn saved_notebook_exports_printable_paper_pdf_and_transparent_png() {
    let notebook = test_path();
    let pdf_output = test_path().with_extension("pdf");
    let png_output = test_path().with_extension("png");
    let source = notebook.to_str().expect("notebook path");
    let pdf = pdf_output.to_str().expect("pdf path");
    let png = png_output.to_str().expect("png path");
    assert!(execute_notebook(&args(&[
        "--printed-sheet", "false", "--output", source,
    ])).expect("save accepted notebook").is_none());
    let stored = fs::read(&notebook).expect("notebook source saved");
    let profile = NotebookProfile::from_json(&stored)
        .expect("profile reopened");
    assert!(execute_notebook(&args(&[
        "--profile", source, "--export-paper", "pdf", "--pages", "3",
        "--output", pdf,
    ])).expect("three-page paper PDF saved").is_none());
    let pdf_bytes = fs::read(&pdf_output).expect("PDF persisted");
    assert_eq!(
        pdf_bytes,
        profile.paper_layer_pdf(3).expect("same Rust paper output"),
    );
    assert!(String::from_utf8_lossy(&pdf_bytes).contains("/Count 3"));
    assert!(execute_notebook(&args(&[
        "--profile", source, "--export-paper", "png", "--dpi", "72",
        "--output", png,
    ])).expect("transparent PNG saved").is_none());
    assert_eq!(
        fs::read(&png_output).expect("PNG persisted"),
        profile.paper_layer_png(72).expect("same Rust PNG output"),
    );
    assert_eq!(fs::read(&notebook).expect("source immutable"), stored);
    fs::remove_file(notebook).expect("owned notebook removed");
    fs::remove_file(pdf_output).expect("owned PDF removed");
    fs::remove_file(png_output).expect("owned PNG removed");
}

#[test]
fn saved_notebook_date_export_matches_rust_layers_in_cli() {
    let saved_path = test_path();
    let saved_name = saved_path.to_str().expect("notebook filename");
    assert!(execute_notebook(&args(&[
        "--date-style", "box", "--date-placement", "top-right",
        "--printed-sheet", "true", "--output", saved_name,
    ])).expect("save notebook with date").is_none());
    let stored = fs::read(&saved_path).expect("saved notebook JSON");
    let notebook = NotebookProfile::from_json(&stored).expect("reopen date");
    assert_eq!(notebook.date_style, DateStyle::Box);
    assert_eq!(notebook.date_placement, DatePlacement::TopRight);

    let pdf = execute_notebook(&args(&[
        "--profile", saved_name, "--export-paper", "pdf", "--pages", "2",
    ]))
    .expect("two-page PDF")
    .expect("PDF stdout");
    assert_eq!(pdf, notebook.paper_layer_pdf(2).expect("same PDF"));
    assert_eq!(
        String::from_utf8_lossy(&pdf).matches(" m\n").count(), 8,
        "four box strokes on each preprinted page, with no duplicate ruling",
    );

    let png_path = test_path().with_extension("png");
    let png_name = png_path.to_str().expect("PNG filename");
    assert!(execute_notebook(&args(&[
        "--profile", saved_name, "--export-paper", "png", "--dpi", "72",
        "--output", png_name,
    ])).expect("PNG export").is_none());
    assert_eq!(
        fs::read(&png_path).expect("saved PNG"),
        notebook.paper_layer_png(72).expect("same physical RGBA layer"),
    );

    let left_line = execute_notebook(&args(&[
        "--profile", saved_name, "--date-style", "line",
        "--date-placement", "top-left", "--export-paper", "pdf",
        "--pages", "2",
    ]))
    .expect("edited date PDF")
    .expect("PDF stdout");
    assert_ne!(pdf, left_line);
    assert_eq!(
        String::from_utf8_lossy(&left_line).matches(" m\n").count(), 2,
    );
    assert_eq!(fs::read(&saved_path).expect("source preserved"), stored);
    fs::remove_file(saved_path).expect("remove owned notebook");
    fs::remove_file(png_path).expect("remove owned PNG");
}

#[test]
fn notebook_printed_sheet_suppresses_marks_without_editing_profile() {
    let original = execute_notebook(&args(&["--printed-sheet", "true"]))
        .expect("default notebook")
        .expect("canonical profile");
    let profile = NotebookProfile::from_json(&original)
        .expect("profile parsed");
    let pdf = execute_notebook(&args(&[
        "--printed-sheet", "true", "--export-paper", "pdf", "--pages", "2",
    ]))
    .expect("unmarked PDF")
    .expect("PDF bytes");
    assert_eq!(
        pdf,
        profile.paper_layer_pdf(2).expect("same unmarked PDF"),
    );
    let png = execute_notebook(&args(&[
        "--printed-sheet", "true", "--export-paper", "png", "--dpi", "72",
    ]))
    .expect("unmarked PNG")
    .expect("PNG bytes");
    assert_eq!(
        png,
        profile.paper_layer_png(72).expect("same unmarked PNG"),
    );
    assert!(png.starts_with(b"\x89PNG\r\n\x1a\n"));
}

#[test]
fn invalid_notebook_export_flags_refuse_without_file_changes() {
    let output = test_path();
    let filename = output.to_str().expect("output path");
    fs::write(&output, b"preserved").expect("output sentinel");
    for invalid in [
        args(&["--export-paper", "gif"]),
        args(&["--export-paper", "png", "--pages", "2"]),
        args(&["--export-paper", "pdf", "--dpi", "72"]),
        args(&["--pages", "2"]),
        args(&["--dpi", "150"]),
        args(&["--export-paper", "pdf", "--pages", "0"]),
        args(&["--export-paper", "pdf", "--pages", "33"]),
        args(&["--export-paper", "pdf", "--pages", "-1"]),
        args(&["--export-paper", "pdf", "--pages", "2", "--pages", "2"]),
        args(&["--export-paper", "png", "--dpi", "71"]),
        args(&["--export-paper", "png", "--dpi", "301"]),
        args(&["--export-paper", "png", "--dpi", "x"]),
        args(&["--export-paper", "png", "--dpi", "72", "--dpi", "72"]),
        args(&["--export-paper", "pdf", "--export-paper", "png"]),
        args(&["--inspect", "--export-paper", "pdf"]),
        args(&["--date-style", "box", "--export-paper", "pdf"]),
    ] {
        let mut attempt = invalid;
        attempt.extend(args(&["--output", filename, "--replace"]));
        assert!(execute_notebook(&attempt).is_err(), "{attempt:?}");
        assert_eq!(
            fs::read(&output).expect("sentinel persisted"),
            b"preserved",
        );
    }
    let refusal = execute_notebook(&args(&[
        "--export-paper", "pdf", "--output", filename,
    ]));
    assert!(refusal.is_err(), "explicit --replace is required");
    assert_eq!(fs::read(&output).expect("still unmodified"), b"preserved");
    fs::remove_file(output).expect("owned sentinel removed");
}
