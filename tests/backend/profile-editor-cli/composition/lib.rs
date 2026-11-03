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
fn notebook_geometry_round_trips_distinct_grid_and_ruled_layouts() {
    let grid = atrament_notebook_paper_cli::default_paper_profile_json();
    let ruled = grid
        .replace("\"style\": \"grid\"", "\"style\": \"ruled\"")
        .replace("\"spacing_um\": 5000", "\"spacing_um\": 7000")
        .replace("\"margin_top_um\": 22000", "\"margin_top_um\": 25000")
        .replace("\"margin_bottom_um\": 16000", "\"margin_bottom_um\": 15000")
        .replace("\"margin_left_um\": 20000", "\"margin_left_um\": 18000")
        .replace("\"margin_right_um\": 14000", "\"margin_right_um\": 12000");
    let paper_paths = [test_path(), test_path()];
    let notebook_paths = [test_path(), test_path()];
    let layouts = [(grid, "line", "top-left"), (ruled, "box", "top-right")];
    let mut rendered = Vec::new();
    for (index, (paper, style, placement)) in layouts.iter().enumerate() {
        fs::write(paper_paths.get(index).expect("paper fixture"), paper)
            .expect("owned paper fixture");
        let paper_name = paper_paths
            .get(index)
            .expect("paper fixture")
            .to_str()
            .expect("paper path");
        let notebook_name = notebook_paths
            .get(index)
            .expect("notebook fixture")
            .to_str()
            .expect("notebook path");
        assert!(
            execute_notebook(&args(&[
                "--paper-profile",
                paper_name,
                "--date-style",
                style,
                "--date-placement",
                placement,
                "--border-radius-um",
                "3000",
                "--page-numbers",
                "true",
                "--printed-sheet",
                "false",
                "--output",
                notebook_name,
            ]))
            .expect("save edited notebook geometry")
            .is_none()
        );
        let notebook_path = notebook_paths
            .get(index)
            .expect("notebook fixture");
        let stored = fs::read(notebook_path).expect("stored notebook");
        assert_eq!(
            execute_notebook(&args(&["--profile", notebook_name]))
                .expect("reopen notebook")
                .expect("canonical JSON"),
            stored,
        );
        let loaded = NotebookProfile::from_json(&stored)
            .expect("Rust geometry validation");
        assert_eq!(&loaded.paper_json, paper);
        assert_eq!(loaded.date_style.key(), *style);
        assert_eq!(loaded.date_placement.key(), *placement);
        assert_eq!(loaded.border_radius_um, 3_000);
        assert!(loaded.page_numbers);
        assert!(!loaded.printed_sheet);
        let pdf = execute_notebook(&args(&[
            "--profile",
            notebook_name,
            "--export-paper",
            "pdf",
            "--pages",
            "2",
        ]))
        .expect("physical export")
        .expect("PDF bytes");
        assert_eq!(
            pdf,
            loaded.paper_layer_pdf(2).expect("shared Rust renderer")
        );
        assert!(String::from_utf8_lossy(&pdf).contains("/Count 2"));
        rendered.push(pdf);
    }
    assert_ne!(
        rendered.first().expect("grid PDF"),
        rendered.get(1).expect("ruled PDF"),
        "distinct grid and ruled marks"
    );

    for path in paper_paths.into_iter().chain(notebook_paths) {
        fs::remove_file(path).expect("remove owned profile");
    }
}

#[test]
fn impossible_notebook_margins_refuse_without_replacing_output() {
    let grid = atrament_notebook_paper_cli::default_paper_profile_json();
    let impossible = test_path();
    fs::write(
        &impossible,
        grid.replace("\"margin_left_um\": 20000", "\"margin_left_um\": 204000"),
    )
    .expect("invalid paper fixture");
    let output = test_path();
    fs::write(&output, b"preserved").expect("owned output sentinel");
    assert!(
        execute_notebook(&args(&[
            "--paper-profile",
            impossible.to_str().expect("paper path"),
            "--date-style",
            "box",
            "--date-placement",
            "top-right",
            "--output",
            output.to_str().expect("output path"),
            "--replace",
        ]))
        .is_err(),
        "impossible margins refuse before replacement"
    );
    assert_eq!(fs::read(&output).expect("sentinel survives"), b"preserved");
    fs::remove_file(impossible).expect("remove invalid fixture");
    fs::remove_file(output).expect("remove owned sentinel");
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
        "--date-text", "2026-11-03", "--printed-sheet", "true",
        "--output", saved_name,
    ])).expect("save notebook with date").is_none());
    let stored = fs::read(&saved_path).expect("saved notebook JSON");
    let notebook = NotebookProfile::from_json(&stored).expect("reopen date");
    assert_eq!(notebook.date_style, DateStyle::Box);
    assert_eq!(notebook.date_placement, DatePlacement::TopRight);
    assert_eq!(notebook.date_text, "2026-11-03");

    let pdf = execute_notebook(&args(&[
        "--profile", saved_name, "--export-paper", "pdf", "--pages", "2",
    ]))
    .expect("two-page PDF")
    .expect("PDF stdout");
    assert_eq!(pdf, notebook.paper_layer_pdf(2).expect("same PDF"));
    let mut without_date = notebook.clone();
    without_date.date_text.clear();
    let empty_pdf = without_date.paper_layer_pdf(2).expect("empty boxes");
    assert_eq!(
        String::from_utf8_lossy(&empty_pdf).matches(" m\n").count(), 8,
        "four box strokes per page, no duplicated ruling",
    );
    assert_ne!(empty_pdf, pdf, "CLI exports printed date ink");
    assert!(
        String::from_utf8_lossy(&pdf).matches("
f
").count() > 150,
        "selected printed date appears on both saved-page exports",
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
    assert!(String::from_utf8_lossy(&left_line).matches(" m\n").count()
        < String::from_utf8_lossy(&pdf).matches(" m\n").count(),
        "line has fewer ruled placeholder paths than date box");
    assert_eq!(fs::read(&saved_path).expect("source preserved"), stored);
    fs::remove_file(saved_path).expect("remove owned notebook");
    fs::remove_file(png_path).expect("remove owned PNG");
}

#[test]
fn saved_notebook_rounded_border_exports_in_cli_pdf_and_png() {
    let saved = test_path();
    let source = saved.to_str().expect("profile filename");
    assert!(execute_notebook(&args(&[
        "--border-radius-um", "5000", "--printed-sheet", "true",
        "--output", source,
    ])).expect("save rounded notebook").is_none());
    let stored = fs::read(&saved).expect("saved notebook source");
    let reopened = NotebookProfile::from_json(&stored)
        .expect("roundtrip rounded notebook");
    assert_eq!(reopened.border_radius_um, 5_000);

    let pdf = execute_notebook(&args(&[
        "--profile", source, "--export-paper", "pdf", "--pages", "2",
    ])).expect("two-page PDF").expect("PDF stdout");
    assert_eq!(pdf, reopened.paper_layer_pdf(2).expect("shared Rust PDF"));
    assert_eq!(
        String::from_utf8_lossy(&pdf).matches(" c\n").count(), 8,
        "four rounded corners on each preprinted page",
    );
    assert_eq!(
        String::from_utf8_lossy(&pdf).matches(" m\n").count(), 2,
        "one border per page without duplicating printed ruling",
    );
    let png = execute_notebook(&args(&[
        "--profile", source, "--export-paper", "png", "--dpi", "72",
        "--png-page", "2",
    ])).expect("selected rounded PNG").expect("PNG stdout");
    assert_eq!(
        png,
        reopened.paper_layer_png_page(72, 2).expect("shared Rust PNG"),
    );
    let mut borderless = reopened;
    borderless.border_radius_um = 0;
    assert_ne!(pdf, borderless.paper_layer_pdf(2).expect("plain PDF"));
    assert_ne!(
        png, borderless.paper_layer_png_page(72, 2)
            .expect("transparent page without border"),
    );
    assert_eq!(fs::read(&saved).expect("original saved intact"), stored);
    fs::remove_file(saved).expect("remove test-owned notebook");
}

#[test]
fn saved_notebook_page_numbers_export_sequential_pdf_and_png() {
    let saved = test_path();
    let name = saved.to_str().expect("profile path");
    assert!(execute_notebook(&args(&[
        "--page-numbers", "true", "--printed-sheet", "true",
        "--output", name,
    ])).expect("save page numbers").is_none());
    let source = fs::read(&saved).expect("saved profile");
    let profile = NotebookProfile::from_json(&source).expect("load profile");
    assert!(profile.page_numbers);
    let pdf = execute_notebook(&args(&[
        "--profile", name, "--export-paper", "pdf", "--pages", "12",
    ]))
    .expect("sequential PDF").expect("PDF output");
    assert_eq!(pdf, profile.paper_layer_pdf(12).expect("canonical output"));
    let pdf_text = String::from_utf8_lossy(&pdf);
    let streams = pdf_text.split("\nstream\n").skip(1)
        .map(|entry| entry.split("\nendstream").next().unwrap_or(""))
        .collect::<Vec<_>>();
    assert_eq!(streams.len(), 12);
    assert_ne!(
        streams.first().expect("PDF page stream"),
        streams.get(1).expect("PDF page stream"),
    );
    assert_ne!(
        streams.get(8).expect("PDF page stream"),
        streams.get(9).expect("PDF page stream"),
    );
    let png = execute_notebook(&args(&[
        "--profile", name, "--export-paper", "png", "--dpi", "72",
    ]))
    .expect("numbered PNG").expect("PNG bytes");
    assert_eq!(png, profile.paper_layer_png(72).expect("canonical PNG"));
    let tenth_png = execute_notebook(&args(&[
        "--profile", name, "--export-paper", "png", "--dpi", "72",
        "--png-page", "10",
    ]))
    .expect("numbered page 10 PNG").expect("PNG bytes");
    assert_ne!(png, tenth_png, "selected page changes printed number");
    assert_eq!(
        tenth_png,
        profile.paper_layer_png_page(72, 10).expect("page 10 PNG"),
    );
    assert_eq!(fs::read(&saved).expect("source remains"), source);
    fs::remove_file(saved).expect("remove owned profile");
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
        args(&["--png-page", "10"]),
        args(&["--export-paper", "pdf", "--png-page", "2"]),
        args(&["--export-paper", "png", "--png-page", "0"]),
        args(&["--export-paper", "png", "--png-page", "33"]),
        args(&["--export-paper", "png", "--png-page", "-1"]),
        args(&["--export-paper", "png", "--png-page", "no"]),
        args(&["--export-paper", "png", "--png-page", "2", "--png-page", "3"]),
        args(&["--export-paper", "png", "--png-page"]),
        args(&["--export-paper", "pdf", "--export-paper", "png"]),
        args(&["--inspect", "--export-paper", "pdf"]),
        args(&["--date-style", "box", "--export-paper", "pdf"]),
        args(&["--date-text", "2026-11-03"]),
        args(&["--date-style", "line", "--date-placement", "top-left",
            "--date-text", "2026-02-29"]),
        args(&["--date-style", "line", "--date-placement", "top-left",
            "--date-text", "03/11.2026"]),
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

#[test]
fn inventory_cli_reports_full_bilingual_text_baseline_without_render_claim() {
    let sample = REQUIRED_TEXT_GRAPHEMES
        .iter()
        .map(|item| item.grapheme)
        .collect::<Vec<_>>()
        .join(" ");
    let report = String::from_utf8(
        execute_handwriting(&args(&["--inventory-text", &sample]))
            .expect("baseline corpus audit accepted")
            .expect("audit prints to stdout"),
    )
    .expect("human-readable exact grapheme report");
    assert!(report.contains("Distinct visible graphemes: 114"));
    assert!(report.contains("Outside English/Spanish text baseline: 0"));
    assert!(report.contains("[U+006E U+0303]"));
    assert!(report.contains("\"\u{f1}\": text-baseline / global-statistics"));
    assert!(report.contains("\"n\u{303}\": text-baseline / global-statistics"));
    assert!(report.contains("\"\u{ab}\": text-baseline / global-statistics"));
    assert!(report.contains("\"\u{bf}\": text-baseline / global-statistics"));
    assert!(report.contains("membership does not imply renderability"));
}

#[test]
fn inventory_cli_preserves_nfc_nfd_and_reports_external_symbols() {
    let report = String::from_utf8(
        execute_handwriting(&args(&[
            "--set-glyph", "n\u{303}", "width",
            "1800", "2600", "2200", "2200", "200",
            "--inventory-text", concat!(
                "\u{f1} n\u{303} \u{f1} \u{bf}\u{a1} ",
                "\u{ab}\u{bb} \u{201c}Q\u{201d} + = \u{221e} \u{1f408}",
            ),
        ]))
        .expect("text audit accepted")
        .expect("audit text produced"),
    )
    .expect("audit is UTF-8");
    assert!(report.contains("\"\u{f1}\": text-baseline / global-statistics"));
    let nfd_pattern = "\"n\u{303}\": text-baseline / statistical-override";
    assert!(report.contains(nfd_pattern));
    let math_pattern = "\"\u{221e}\": outside-text-baseline";
    assert!(report.contains(math_pattern));
    let emoji_pattern = "\"\u{1f408}\": outside-text-baseline";
    assert!(report.contains(emoji_pattern));
    assert!(report.contains("Outside English/Spanish text baseline: 4"));
    assert!(report.contains("Distinct visible graphemes: 13"));
}

#[test]
fn inventory_cli_refuses_unsafe_or_conflicting_requests_without_file_effects() {
    let target = test_path();
    fs::write(&target, b"unchanged").expect("write owned sentinel");
    let filename = target.to_str().expect("UTF-8 path");
    for invalid in [
        args(&["--inventory-text"]),
        args(&["--inventory-text", ""]),
        args(&["--inventory-text", "\u{1b}[31m"]),
        args(&["--inventory-text", "a", "--inventory-text", "b"]),
        args(&["--inventory-text", "\u{f1}", "--inspect"]),
        args(&[
            "--inventory-text", "\u{f1}", "--output",
            filename, "--replace",
        ]),
        args(&["--inventory-text", &"a".repeat(MAX_INVENTORY_TEXT_BYTES + 1)]),
    ] {
        assert!(execute_handwriting(&invalid).is_err(), "{invalid:?}");
        assert_eq!(fs::read(&target).expect("sentinel survives"), b"unchanged");
    }
    fs::remove_file(target).expect("remove owned sentinel");
}

#[test]
fn numeric_handwriting_preview_is_repeatable_and_uses_exact_glyph_overrides() {
    let base = args(&[
        "--seed", "77",
        "--set-global", "height", "2000", "4000", "3000", "3000", "350",
        "--set-glyph", "\u{f1}", "height",
        "4500", "5500", "5000", "5000", "220",
        "--preview-glyph", "\u{f1}", "7",
    ]);
    let first = execute_handwriting(&base)
        .expect("preview admitted").expect("stdout only");
    assert_eq!(first, execute_handwriting(&base)
        .expect("repeat accepted").expect("same bytes"));
    let rendered = String::from_utf8(first).expect("readable numeric preview");
    assert!(rendered.contains("Seeded numeric preview (not ink"));
    assert!(rendered.contains("Grapheme: \"\u{f1}\""));
    assert!(rendered.contains("Seed: 77\nOccurrence: 7"));
    assert!(rendered.contains(
        "Median and exact population moments are not reproduced",
    ));
    let height = rendered.lines()
        .find(|line| line.starts_with("height (micrometres): "))
        .expect("height value reported");
    let value = height.split_once(": ").expect("one value").1
        .parse::<i32>().expect("integer preview");
    assert!((4_500i32..=5_500i32).contains(&value));
    assert_eq!(value, 5_479i32, "stable cross-run preview value");
    assert!(rendered.contains("width (micrometres): 2200"));

    let nfd = args(&[
        "--seed", "77",
        "--set-global", "height", "2000", "4000", "3000", "3000", "350",
        "--set-glyph", "\u{f1}", "height",
        "4500", "5500", "5000", "5000", "220",
        "--preview-glyph", "n\u{303}", "7",
    ]);
    let nfd_output = String::from_utf8(execute_handwriting(&nfd)
        .expect("NFD admitted").expect("NFD preview bytes"))
        .expect("UTF-8 report");
    assert!(nfd_output.contains("Grapheme: \"n\u{303}\""));
    let nfd_height = nfd_output.lines()
        .find(|line| line.starts_with("height (micrometres): "))
        .expect("NFD height");
    let nfd_value = nfd_height.split_once(": ").expect("one value").1
        .parse::<i32>().expect("integer preview");
    assert!((2_000i32..=4_000i32).contains(&nfd_value));
}

#[test]
fn numeric_preview_refusals_are_stdout_only_and_preserve_file_bytes() {
    let target = test_path();
    fs::write(&target, b"unchanged").expect("owned sentinel");
    let filename = target.to_str().expect("UTF-8 output name");
    for invalid in [
        args(&["--preview-glyph", "\u{f1}"]),
        args(&["--preview-glyph", "\u{f1}", "-1"]),
        args(&["--preview-glyph", "\u{f1}", "1.0"]),
        args(&["--preview-glyph", "\u{f1}", "18446744073709551616"]),
        args(&["--preview-glyph", "ab", "0"]),
        args(&["--preview-glyph", "\u{f1}\n", "0"]),
        args(&["--preview-glyph", "\u{f1}", "0", "--inspect"]),
        args(&["--preview-glyph", "\u{f1}", "0", "--inventory-text", "\u{f1}"]),
        args(&["--preview-glyph", "\u{f1}", "0", "--preview-glyph", "a", "0"]),
        args(&["--preview-glyph", "\u{f1}", "0", "--output", filename,
            "--replace"]),
    ] {
        assert!(execute_handwriting(&invalid).is_err(), "{invalid:?}");
        assert_eq!(fs::read(&target).expect("unchanged output"), b"unchanged");
    }
    fs::remove_file(target).expect("remove owned sentinel");
}

#[test]
fn notebook_cli_exports_manual_pen_ink_to_both_native_formats() {
    let draw = [
        "--ink-line",
        "2",
        "25400",
        "25400",
        "50800",
        "50800",
        "700",
        "#DE1144",
    ];
    let mut pdf_args = args(&[
        "--printed-sheet",
        "true",
        "--export-paper",
        "pdf",
        "--pages",
        "2",
    ]);
    pdf_args.extend(args(&draw));
    let pdf = execute_notebook(&pdf_args)
        .expect("bounded manual ink PDF")
        .expect("PDF stdout");
    assert!(pdf.starts_with(b"%PDF-1.4"));
    let text = String::from_utf8_lossy(&pdf);
    assert!(text.contains("/Count 2"));
    assert_eq!(text.matches(" m\n").count(), 1);

    let mut png_args = args(&[
        "--printed-sheet",
        "true",
        "--export-paper",
        "png",
        "--dpi",
        "72",
        "--png-page",
        "2",
    ]);
    png_args.extend(args(&draw));
    let png = execute_notebook(&png_args)
        .expect("bounded manual ink PNG")
        .expect("PNG stdout");
    assert!(png.starts_with(b"\x89PNG\r\n\x1a\n"));
    assert_ne!(
        png,
        execute_notebook(&args(&[
            "--printed-sheet",
            "true",
            "--export-paper",
            "png",
            "--dpi",
            "72",
            "--png-page",
            "2",
        ]))
        .expect("blank PNG")
        .expect("blank stdout")
    );
    assert_eq!(
        png,
        execute_notebook(&png_args)
            .expect("repeated export")
            .expect("deterministic PNG")
    );
}

#[test]
fn notebook_cli_ink_refusals_leave_preexisting_output_unchanged() {
    let target = test_path();
    fs::write(&target, b"sentinel").expect("test-owned output exists");
    let filename = target.to_str().expect("test path is UTF-8");
    let good = [
        "--ink-line",
        "1",
        "25400",
        "25400",
        "50800",
        "50800",
        "700",
        "#DE1144",
    ];
    for invalid in [
        args(&good),
        args(&["--export-paper", "pdf", "--ink-line"]),
        args(&[
            "--export-paper",
            "pdf",
            "--ink-line",
            "0",
            "25400",
            "25400",
            "50800",
            "50800",
            "700",
            "#DE1144",
        ]),
        args(&[
            "--export-paper",
            "pdf",
            "--ink-line",
            "1",
            "25400",
            "25400",
            "25400",
            "25400",
            "700",
            "#DE1144",
        ]),
        args(&[
            "--export-paper",
            "pdf",
            "--ink-line",
            "1",
            "25400",
            "25400",
            "50800",
            "50800",
            "79",
            "#DE1144",
        ]),
        args(&[
            "--export-paper",
            "pdf",
            "--ink-line",
            "1",
            "25400",
            "25400",
            "50800",
            "50800",
            "700",
            "#QQ1144",
        ]),
        args(&[
            "--export-paper",
            "pdf",
            "--ink-line",
            "1",
            "25400",
            "25400",
            "50800",
            "50800",
            "700",
            "#\u{f1}E1144",
        ]),
        args(&[
            "--export-paper",
            "pdf",
            "--ink-line",
            "1",
            "0",
            "25400",
            "50800",
            "50800",
            "700",
            "#DE1144",
        ]),
        args(&[
            "--export-paper",
            "pdf",
            "--ink-line",
            "2",
            "25400",
            "25400",
            "50800",
            "50800",
            "700",
            "#DE1144",
        ]),
        args(&[
            "--export-paper",
            "pdf",
            "--inspect",
            "--ink-line",
            "1",
            "25400",
            "25400",
            "50800",
            "50800",
            "700",
            "#DE1144",
        ]),
    ] {
        let mut selected = invalid.clone();
        selected.extend(args(&["--output", filename, "--replace"]));
        assert!(execute_notebook(&selected).is_err(), "{invalid:?}");
        assert_eq!(
            fs::read(&target).expect("sentinel is preserved"),
            b"sentinel"
        );
    }
    let mut over_budget = args(&["--export-paper", "pdf"]);
    for _ in 0..=MAX_NOTEBOOK_INK_LINES {
        over_budget.extend(args(&good));
    }
    assert!(
        execute_notebook(&over_budget).is_err(),
        "no hidden truncation"
    );
    fs::remove_file(target).expect("remove test-owned output");
}

#[test]
fn exact_authored_unicode_glyphs_match_page_specific_manual_strokes() {
    // Each shape is user authored, not a font or statistical substitution.
    // The NFD grapheme occupies one 4-mm cell even though it has two scalars.
    let shape_args = args(&[
        "--glyph-stroke", "ñ", "0", "0", "2500", "3500", "350",
        "#DE1144",
        "--glyph-stroke", "n\u{303}", "250", "3500", "2700", "250",
        "350", "#22AA33",
        "--glyph-stroke", "∞", "250", "3500", "2700", "250",
        "350", "#3344AA",
        "--place-text", "2", "25000", "30000", "ñ n\u{303} ∞ ñ",
    ]);
    let mut glyph_pdf = args(&[
        "--printed-sheet", "true", "--export-paper", "pdf",
        "--pages", "2",
    ]);
    glyph_pdf.extend(shape_args.clone());
    let text_pdf = execute_notebook(&glyph_pdf)
        .expect("exact custom glyph shapes accepted").expect("PDF bytes");
    assert!(text_pdf.starts_with(b"%PDF-1.4"));
    let pdf_text = String::from_utf8_lossy(&text_pdf);
    let streams = pdf_text
        .split("\nstream\n").skip(1)
        .map(|entry| entry.split("\nendstream").next().unwrap_or(""))
        .collect::<Vec<_>>();
    assert_eq!(streams.len(), 2);
    assert!(!streams[0].contains(" m\n"), "no page 1 ink");
    assert_eq!(streams[1].matches(" m\n").count(), 4);
    assert!(streams[1].contains("0.871 0.067 0.267 RG"));
    assert!(streams[1].contains("0.133 0.667 0.200 RG"));

    // Identical bytes from the existing handwritten-line Rust compositor.
    let manual = args(&[
        "--printed-sheet", "true", "--export-paper", "pdf",
        "--pages", "2",
        "--ink-line", "2", "25000", "30000", "27500", "33500",
        "350", "#DE1144",
        "--ink-line", "2", "33250", "33500", "35700", "30250",
        "350", "#22AA33",
        "--ink-line", "2", "41250", "33500", "43700", "30250",
        "350", "#3344AA",
        "--ink-line", "2", "49000", "30000", "51500", "33500",
        "350", "#DE1144",
    ]);
    assert_eq!(
        text_pdf,
        execute_notebook(&manual).expect("manual counterpart")
            .expect("manual PDF"),
    );

    let mut glyph_png = args(&[
        "--printed-sheet", "true", "--export-paper", "png",
        "--dpi", "72", "--png-page", "2",
    ]);
    glyph_png.extend(shape_args);
    let page_two = execute_notebook(&glyph_png)
        .expect("glyph PNG page 2").expect("PNG bytes");
    assert!(page_two.starts_with(b"\x89PNG\r\n\x1a\n"));
    let mut page_one_args = glyph_png.clone();
    let page_position = page_one_args.iter()
        .position(|item| item == "--png-page").expect("page flag");
    page_one_args[page_position + 1] = "1".into();
    let page_one = execute_notebook(&page_one_args)
        .expect("blank PNG page 1").expect("PNG bytes");
    assert_ne!(page_two, page_one);
    // Rebuild the manual PNG counterpart without shifting flag positions.
    let manual_png = args(&[
        "--printed-sheet", "true", "--export-paper", "png",
        "--dpi", "72", "--png-page", "2",
        "--ink-line", "2", "25000", "30000", "27500", "33500",
        "350", "#DE1144",
        "--ink-line", "2", "33250", "33500", "35700", "30250",
        "350", "#22AA33",
        "--ink-line", "2", "41250", "33500", "43700", "30250",
        "350", "#3344AA",
        "--ink-line", "2", "49000", "30000", "51500", "33500",
        "350", "#DE1144",
    ]);
    assert_eq!(page_two, execute_notebook(&manual_png)
        .expect("manual glyph PNG").expect("PNG bytes"));
}

#[test]
fn exact_unicode_missing_shapes_report_all_and_preserve_output() {
    let target = test_path();
    let filename = target.to_str().expect("target name");
    fs::write(&target, b"preserved").expect("test-owned sentinel");
    let selected = args(&[
        "--printed-sheet", "true", "--export-paper", "pdf",
        "--glyph-stroke", "ñ", "0", "0", "3000", "4000", "200",
        "#000000",
        "--place-text", "1", "25000", "30000",
        "ñ n\u{303} ∞ « ¿ n\u{303}",
        "--output", filename, "--replace",
    ]);
    let error = execute_notebook(&selected).expect_err("missing shapes refuse");
    assert!(error.starts_with("missing authored glyph shapes:"), "{error}");
    for codepoints in [
        "U+006E U+0303", "U+221E", "U+00AB", "U+00BF",
    ] {
        assert!(error.contains(codepoints), "{error}");
    }
    assert!(!error.contains("U+00F1"), "NFC is exactly covered");
    assert_eq!(error.matches("U+006E U+0303").count(), 1);
    assert_eq!(fs::read(&target).expect("unchanged target"), b"preserved");

    for invalid in [
        args(&["--glyph-stroke", "ab", "0", "0", "1", "1",
            "200", "#000000", "--place-text", "1", "25000",
            "30000", "ab"]),
        args(&["--glyph-stroke", "ñ", "0", "0", "0", "0",
            "200", "#000000", "--place-text", "1", "25000",
            "30000", "ñ"]),
        args(&["--glyph-stroke", "ñ", "0", "0", "10", "10",
            "200", "#QQ0000", "--place-text", "1", "25000",
            "30000", "ñ"]),
        args(&["--glyph-stroke", "ñ", "0", "0", "10", "10",
            "200", "#000000"]),
        args(&["--place-text", "1", "25000", "30000", "ñ"]),
        args(&["--place-text", "1", "25000", "30000", "ñ\n"]),
        args(&["--place-text", "1", "25000", "30000", "   "]),
    ] {
        let mut attempt = args(&["--export-paper", "pdf"]);
        attempt.extend(invalid);
        attempt.extend(args(&["--output", filename, "--replace"]));
        assert!(execute_notebook(&attempt).is_err(), "{attempt:?}");
        assert_eq!(fs::read(&target).expect("still unchanged"), b"preserved");
    }
    fs::remove_file(target).expect("remove test-owned output");
}

#[test]
fn glyph_overlay_budgets_and_nonselected_page_clearance_refuse() {
    let sentinel = test_path();
    let name = sentinel.to_str().expect("owned file name");
    fs::write(&sentinel, b"original").expect("owned sentinel");

    let mut excessive = args(&[
        "--export-paper", "pdf", "--printed-sheet", "true",
    ]);
    for _ in 0..6 {
        excessive.extend(args(&[
            "--glyph-stroke", "ñ", "0", "0", "3000", "3000",
            "250", "#123456",
        ]));
    }
    excessive.extend(args(&[
        "--place-text", "1", "25000", "30000", &"ñ".repeat(96),
        "--output", name, "--replace",
    ]));
    let error = execute_notebook(&excessive).expect_err("more than 512");
    assert!(error.contains("512"), "{error}");
    assert_eq!(fs::read(&sentinel).expect("not overwritten"), b"original");

    // Physical clearance is checked for every supplied page, even when the
    // selected PNG sheet omits that page's ink.
    let bad_position = args(&[
        "--printed-sheet", "true", "--export-paper", "png",
        "--dpi", "72", "--png-page", "1",
        "--glyph-stroke", "ñ", "0", "0", "3000", "3000",
        "350", "#123456",
        "--place-text", "2", "199000", "30000", "ñ",
        "--output", name, "--replace",
    ]);
    let error = execute_notebook(&bad_position)
        .expect_err("nonselected page still validates");
    assert!(error.contains("printable endpoints"), "{error}");
    assert_eq!(fs::read(&sentinel).expect("still original"), b"original");

    for invalid in [
        args(&["--export-paper", "pdf", "--place-text", "1", "25000",
            "30000", "ñ", "--glyph-stroke", "ñ", "0", "0", "3000",
            "3000", "350", "#123456", "--inspect"]),
        args(&["--export-paper", "pdf", "--glyph-stroke", "ñ", "0",
            "0", "3000", "3000", "350", "#123456",
            "--place-text", "1", "25000", "30000", "ñ\n∞"]),
    ] {
        let mut attempt = invalid;
        attempt.extend(args(&["--output", name, "--replace"]));
        assert!(execute_notebook(&attempt).is_err(), "{attempt:?}");
        assert_eq!(fs::read(&sentinel).expect("unchanged"), b"original");
    }
    fs::remove_file(sentinel).expect("remove owned sentinel");
}

#[test]
fn saved_handwriting_means_transform_authored_unicode_glyphs_in_both_formats() {
    let saved_path = test_path();
    let name = saved_path.to_str().expect("statistics test path");
    let stats = execute_handwriting(&args(&[
        "--set-glyph", "ñ", "width", "3000", "3000", "3000",
        "3000", "0",
        "--set-glyph", "ñ", "height", "4000", "4000", "4000",
        "4000", "0",
        "--set-glyph", "ñ", "ink_thickness", "500", "500", "500",
        "500", "0",
        "--set-glyph", "ñ", "letter_spacing", "600", "600", "600",
        "600", "0",
        "--set-glyph", "ñ", "baseline_offset", "1000", "1000", "1000",
        "1000", "0",
    ])).expect("saved statistics admitted").expect("statistics JSON");
    fs::write(&saved_path, stats).expect("persist test-owned statistics");
    let authored = args(&[
        "--glyph-stroke", "ñ", "0", "0", "3500", "4500",
        "300", "#123456",
        "--place-text", "1", "25000", "30000", "ññ",
    ]);
    let manual_strokes = args(&[
        "--ink-line", "1", "25000", "31000", "28000", "35000",
        "500", "#123456",
        "--ink-line", "1", "28600", "31000", "31600", "35000",
        "500", "#123456",
    ]);
    for format in ["pdf", "png"] {
        let mut selected = args(&[
            "--printed-sheet", "true", "--export-paper", format,
            "--handwriting-profile", name,
        ]);
        selected.extend(authored.clone());
        let result = execute_notebook(&selected)
            .expect("saved means applied").expect("output bytes");
        let mut expected = args(&[
            "--printed-sheet", "true", "--export-paper", format,
        ]);
        expected.extend(manual_strokes.clone());
        assert_eq!(result, execute_notebook(&expected)
            .expect("explicit physical ink accepted").expect("physical bytes"));
        let without_stats = execute_notebook(&args(&[
            "--printed-sheet", "true", "--export-paper", format,
            "--glyph-stroke", "ñ", "0", "0", "3500", "4500",
            "300", "#123456",
            "--place-text", "1", "25000", "30000", "ññ",
        ])).expect("original glyphs").expect("unscaled bytes");
        assert_ne!(result, without_stats, "statistics transform real ink");
    }
    let mixed = args(&[
        "--printed-sheet", "true", "--export-paper", "pdf",
        "--handwriting-profile", name,
        "--glyph-stroke", "ñ", "0", "0", "3500", "4500",
        "300", "#123456",
        "--glyph-stroke", "n\u{303}", "0", "0", "3500", "4500",
        "300", "#123456",
        "--place-text", "1", "25000", "30000", "ñn\u{303}",
    ]);
    let expected_mixed = args(&[
        "--printed-sheet", "true", "--export-paper", "pdf",
        "--ink-line", "1", "25000", "31000", "28000", "35000",
        "500", "#123456",
        "--ink-line", "1", "28600", "30000", "30800", "33000",
        "300", "#123456",
    ]);
    assert_eq!(execute_notebook(&mixed).expect("NFC and NFD edit distinct")
        .expect("mixed PDF"),
        execute_notebook(&expected_mixed).expect("manual NFD ink")
            .expect("manual PDF"));
    fs::remove_file(saved_path).expect("remove owned statistics profile");
}

#[test]
fn statistical_glyph_layout_refuses_bad_advance_and_keeps_output() {
    let source = test_path();
    let source_name = source.to_str().expect("statistics path");
    let target = test_path();
    let target_name = target.to_str().expect("output path");
    let stats = execute_handwriting(&args(&[
        "--set-glyph", "ñ", "letter_spacing", "-5000", "-5000",
        "-5000", "-5000", "0",
    ])).expect("negative letter spacing admitted").expect("JSON");
    fs::write(&source, stats).expect("owned statistics file");
    fs::write(&target, b"original").expect("owned output sentinel");
    let glyph = args(&[
        "--glyph-stroke", "ñ", "0", "0", "3500", "4500",
        "300", "#123456",
        "--place-text", "1", "25000", "30000", "ñ",
    ]);
    let mut invalid = args(&[
        "--printed-sheet", "true", "--export-paper", "pdf",
        "--handwriting-profile", source_name,
    ]);
    invalid.extend(glyph.clone());
    invalid.extend(args(&["--output", target_name, "--replace"]));
    let error = execute_notebook(&invalid).expect_err("negative advance");
    assert!(error.contains("must be positive"), "{error}");
    assert_eq!(fs::read(&target).expect("still original"), b"original");

    let mut invalid_alone = args(&[
        "--export-paper", "pdf", "--handwriting-profile", source_name,
        "--output", target_name, "--replace",
    ]);
    assert!(execute_notebook(&invalid_alone).is_err());
    invalid_alone.extend(glyph);
    invalid_alone.extend(args(&[
        "--handwriting-profile", source_name,
    ]));
    assert!(execute_notebook(&invalid_alone).is_err(), "duplicate profile");
    assert_eq!(fs::read(&target).expect("unchanged"), b"original");
    fs::remove_file(source).expect("remove owned stats");
    fs::remove_file(target).expect("remove owned target");
}

#[test]
fn saved_slant_means_shear_authored_glyphs_in_pdf_and_png() {
    let path = test_path();
    let filename = path.to_str().expect("temporary statistics path");
    for (slant, first_x) in [("45000", "29500"), ("-45000", "20500")] {
        let stats = execute_handwriting(&args(&[
            "--set-glyph", "ñ", "width", "3500", "3500", "3500",
            "3500", "0",
            "--set-glyph", "ñ", "height", "4500", "4500", "4500",
            "4500", "0",
            "--set-glyph", "ñ", "slant", slant, slant, slant,
            slant, "0",
        ]))
        .expect("explicit slant mean admitted")
        .expect("native statistics bytes");
        fs::write(&path, stats).expect("test-owned statistics profile");

        for format in ["pdf", "png"] {
            let mut selected = args(&[
                "--printed-sheet", "true", "--export-paper", format,
                "--handwriting-profile", filename,
                "--glyph-stroke", "ñ", "0", "0", "3500", "4500",
                "300", "#123456",
                "--place-text", "1", "25000", "30000", "ñ",
            ]);
            if format == "png" {
                selected.extend(args(&["--dpi", "72"]));
            }
            let mut expected = args(&[
                "--printed-sheet", "true", "--export-paper", format,
                "--ink-line", "1", first_x, "30000", "28500",
                "34500", "300", "#123456",
            ]);
            if format == "png" {
                expected.extend(args(&["--dpi", "72"]));
            }
            assert_eq!(
                execute_notebook(&selected)
                    .expect("mean slant export")
                    .expect("output bytes"),
                execute_notebook(&expected)
                    .expect("manually sheared physical stroke")
                    .expect("manual output"),
                "{format} with {slant} millidegrees must tilt actual ink",
            );
        }
    }
    fs::remove_file(path).expect("remove owned statistics fixture");
}

#[test]
fn out_of_page_slant_refuses_before_replacing_existing_pdf() {
    let profile = test_path();
    let profile_name = profile.to_str().expect("statistics path");
    let output = test_path();
    let output_name = output.to_str().expect("output path");
    let stats = execute_handwriting(&args(&[
        "--set-glyph", "ñ", "slant", "-85000", "-85000",
        "-85000", "-85000", "0",
    ]))
    .expect("allowed negative slant")
    .expect("profile bytes");
    fs::write(&profile, stats).expect("saved statistics");
    fs::write(&output, b"original PDF bytes").expect("sentinel target");
    let command = args(&[
        "--export-paper", "pdf", "--printed-sheet", "true",
        "--handwriting-profile", profile_name,
        "--glyph-stroke", "ñ", "0", "0", "3500", "4500",
        "300", "#123456",
        "--place-text", "1", "25000", "30000", "ñ",
        "--output", output_name, "--replace",
    ]);
    assert!(
        execute_notebook(&command).is_err(),
        "unprintable slanted stroke must refuse",
    );
    assert_eq!(
        fs::read(&output).expect("output unchanged"),
        b"original PDF bytes",
    );
    fs::remove_file(profile).expect("remove profile fixture");
    fs::remove_file(output).expect("remove output fixture");
}

#[test]
fn saved_glyph_library_reopens_exact_unicode_shapes_for_pdf_and_png() {
    let path = test_path().with_extension("tsv");
    let filename = path.to_str().expect("glyph library filename");
    let original = args(&[
        "--glyph-stroke",
        "ñ",
        "0",
        "0",
        "2500",
        "3500",
        "350",
        "#12ab34",
        "--glyph-stroke",
        "n\u{303}",
        "250",
        "3500",
        "2700",
        "250",
        "250",
        "#0000FF",
        "--glyph-stroke",
        "«",
        "50",
        "2000",
        "1000",
        "4000",
        "250",
        "#0000FF",
        "--glyph-stroke",
        "»",
        "1000",
        "2000",
        "50",
        "4000",
        "250",
        "#0000FF",
        "--glyph-stroke",
        "∞",
        "500",
        "400",
        "2500",
        "2800",
        "300",
        "#445566",
    ]);
    let mut write = original.clone();
    write.extend(args(&["--output", filename]));
    assert!(
        execute_glyph_library(&write)
            .expect("save named glyph library")
            .is_none()
    );
    let bytes = fs::read(&path).expect("saved library");
    let text = String::from_utf8(bytes.clone()).expect("UTF-8 library");
    assert!(text.starts_with("atrament.glyph-strokes.v1\nñ\t"));
    assert!(text.contains("n\u{303}\t250\t3500\t2700\t250\t250\t#0000FF"));
    let reopened = execute_glyph_library(&args(&["--profile", filename]))
        .expect("open saved glyph library")
        .expect("library bytes");
    assert_eq!(reopened, bytes, "canonical source must roundtrip exactly");

    for format in ["pdf", "png"] {
        let mut expected = args(&[
            "--export-paper", format, "--printed-sheet", "true",
        ]);
        expected.extend(original.clone());
        expected.extend(args(&[
            "--place-text",
            "2",
            "25000",
            "30000",
            "ñ n\u{303} «∞»",
        ]));
        let mut from_file = args(&[
            "--export-paper",
            format,
            "--printed-sheet",
            "true",
            "--glyph-file",
            filename,
            "--place-text",
            "2",
            "25000",
            "30000",
            "ñ n\u{303} «∞»",
        ]);
        if format == "pdf" {
            expected.extend(args(&["--pages", "2"]));
            from_file.extend(args(&["--pages", "2"]));
        } else {
            expected.extend(args(&["--png-page", "2", "--dpi", "72"]));
            from_file.extend(args(&["--png-page", "2", "--dpi", "72"]));
        }
        assert_eq!(
            execute_notebook(&from_file)
                .expect("file-backed glyph ink")
                .expect("native output"),
            execute_notebook(&expected)
                .expect("inline glyph ink")
                .expect("native output"),
            "file and inline glyph ink differ for {format}",
        );
    }
    fs::remove_file(path).expect("remove test-owned library");
}

#[test]
fn glyph_libraries_append_explicit_shapes_and_preserve_existing_output() {
    let library = test_path().with_extension("tsv");
    let name = library.to_str().expect("library path");
    let first = args(&[
        "--glyph-stroke",
        "ñ",
        "0",
        "0",
        "2500",
        "3500",
        "250",
        "#123456",
        "--output",
        name,
    ]);
    assert!(
        execute_glyph_library(&first)
            .expect("save library")
            .is_none()
    );
    let saved = fs::read(&library).expect("source bytes");
    assert!(
        execute_glyph_library(&first).is_err(),
        "existing file requires explicit replacement"
    );
    assert_eq!(saved, fs::read(&library).expect("unchanged"));
    let extended = execute_glyph_library(&args(&[
        "--profile",
        name,
        "--glyph-stroke",
        "¿",
        "0",
        "3500",
        "2500",
        "0",
        "250",
        "#abcDEF",
    ]))
    .expect("append authored glyph")
    .expect("canonical bytes");
    assert!(String::from_utf8_lossy(&extended)
        .contains("¿\t0\t3500\t2500\t0\t250\t#ABCDEF\n"));
    assert_eq!(saved, fs::read(&library).expect("read-only source"));
    assert!(
        execute_glyph_library(&args(&[
            "--profile", name, "--profile", name,
        ])).is_err(),
        "duplicate library load must refuse"
    );
    assert!(
        execute_glyph_library(&args(&[
            "--glyph-stroke",
            "ñ",
            "0",
            "0",
            "1",
            "1",
            "250",
            "#123456",
            "--profile",
            name,
        ]))
        .is_err(),
        "late file cannot silently replace authored strokes"
    );
    fs::remove_file(library).expect("remove test-owned library");
}

#[test]
fn malformed_or_unbounded_glyph_library_refuses_without_pdf_replacement() {
    let library = test_path().with_extension("tsv");
    let target = test_path().with_extension("pdf");
    let name = library.to_str().expect("library name");
    let output = target.to_str().expect("PDF name");
    fs::write(&target, b"previous approved PDF").expect("sentinel");
    for invalid in [
        b"unsupported.version\n".to_vec(),
        b"atrament.glyph-strokes.v1\r\n".to_vec(),
        b"atrament.glyph-strokes.v1\n\xff".to_vec(),
        [b"atrament.glyph-strokes.v1\n".as_slice(),
         b"\xc3\xb1\t0\t0\t0\t0\t250\t#123456\n"].concat(),
        [b"atrament.glyph-strokes.v1\n".as_slice(),
         b"\xc3\xb1\t0\t0\t250\t350\t250\t#GGGGGG\n"].concat(),
        b"atrament.glyph-strokes.v1\n\xc3\xb1\t0\t0\t250\t350\t250\n".to_vec(),
        vec![b'x'; MAX_GLYPH_LIBRARY_BYTES + 1],
    ] {
        fs::write(&library, invalid).expect("write invalid test library");
        let command = args(&[
            "--export-paper",
            "pdf",
            "--glyph-file",
            name,
            "--place-text",
            "1",
            "25000",
            "30000",
            "ñ",
            "--output",
            output,
            "--replace",
        ]);
        assert!(
            execute_notebook(&command).is_err(),
            "invalid glyph input must refuse before output"
        );
        assert_eq!(
            fs::read(&target).expect("sentinel survives"),
            b"previous approved PDF"
        );
    }
    fs::remove_file(library).expect("remove invalid source");
    fs::remove_file(target).expect("remove target");
}

#[test]
fn glyph_library_rejects_missing_shapes_and_over_budget_strokes() {
    let source = test_path().with_extension("tsv");
    let name = source.to_str().expect("source");
    let result = execute_glyph_library(&args(&[
        "--glyph-stroke",
        "ñ",
        "0",
        "0",
        "3500",
        "4500",
        "250",
        "#123456",
    ]))
    .expect("valid single glyph")
    .expect("library bytes");
    fs::write(&source, result).expect("test library");
    let error = execute_notebook(&args(&[
        "--export-paper",
        "pdf",
        "--glyph-file",
        name,
        "--place-text",
        "1",
        "25000",
        "30000",
        "ñ¡",
    ]))
    .expect_err("missing exclamation glyph must fail");
    assert!(error.contains("missing authored glyph shapes"), "{error}");

    let mut extended = String::from("atrament.glyph-strokes.v1\n");
    for _ in 0..600 {
        extended.push_str("ñ\t0\t0\t3500\t4500\t250\t#123456\n");
    }
    fs::write(&source, extended.as_bytes()).expect("test expanded library");
    let loaded = execute_glyph_library(&args(&["--profile", name]))
        .expect("expanded library accepted")
        .expect("expanded bytes");
    assert_eq!(loaded, extended.as_bytes());

    let mut too_many = String::from("atrament.glyph-strokes.v1\n");
    for _ in 0..4097 {
        too_many.push_str("ñ\t0\t0\t3500\t4500\t250\t#123456\n");
    }
    fs::write(&source, too_many).expect("test over-budget library");
    let error =
        execute_glyph_library(&args(&["--profile", name]))
            .expect_err("too many authored segments");
    assert!(error.contains("4096 strokes"), "{error}");
    fs::remove_file(source).expect("remove source");
}

#[test]
fn glyph_library_audit_separates_nfc_nfd_math_and_missing_punctuation() {
    let text = "ñ n\u{303} «¡∞!» ñ";
    let report = execute_glyph_library(&args(&[
        "--glyph-stroke", "ñ", "0", "0", "3500", "4500",
        "250", "#000000",
        "--glyph-stroke", "∞", "0", "0", "3500", "4500",
        "250", "#000000",
        "--audit-text", text,
    ])).expect("shape audit accepted").expect("stdout-only audit");
    let report = String::from_utf8(report).expect("readable audit");
    assert!(report.contains("Distinct visible graphemes: 7"), "{report}");
    assert!(report.contains("Missing authored glyphs: 5"), "{report}");
    assert!(report.contains("[U+00F1]: 1 authored strokes"), "{report}");
    assert!(report.contains(
        "[U+006E U+0303]: missing authored strokes"
    ), "{report}");
    assert!(report.contains("[U+221E]: 1 authored strokes"), "{report}");
    assert!(report.contains("[U+00AB]: missing authored strokes"), "{report}");
    assert!(report.contains("[U+00A1]: missing authored strokes"), "{report}");
    assert!(report.contains("[U+0021]: missing authored strokes"), "{report}");
    assert!(!report.contains("[U+0020]"), "whitespace is not ink");

    let empty = execute_glyph_library(&args(&[
        "--audit-text", "¿Qué tal?",
    ])).expect("no glyphs still auditable").expect("report");
    assert!(String::from_utf8_lossy(&empty)
        .contains("Missing authored glyphs: 8"));
}

#[test]
fn glyph_library_removal_and_reauthoring_updates_pdf_and_png() {
    let library = test_path().with_extension("tsv");
    let path = library.to_str().expect("test library");
    let first = args(&[
        "--glyph-stroke", "ñ", "0", "0", "3500", "4500",
        "300", "#123456",
        "--glyph-stroke", "ñ", "0", "4500", "3500", "0",
        "300", "#123456",
        "--glyph-stroke", "n\u{303}", "0", "0", "3500", "4500",
        "300", "#123456",
        "--output", path,
    ]);
    assert!(execute_glyph_library(&first)
        .expect("write original library").is_none());
    let before = fs::read(&library).expect("read saved strokes");
    let replaced = execute_glyph_library(&args(&[
        "--profile", path,
        "--remove-glyph", "ñ",
        "--glyph-stroke", "ñ", "300", "100", "2800", "3700",
        "500", "#123456",
    ])).expect("replace glyph contours").expect("new library");
    assert_eq!(replaced.iter().filter(|&&byte| byte == b'\n').count(), 3);
    assert!(String::from_utf8_lossy(&replaced)
        .contains("ñ\t300\t100\t2800\t3700\t500\t#123456"));
    assert!(!String::from_utf8_lossy(&replaced)
        .contains("ñ\t0\t4500"));
    assert_eq!(fs::read(&library).expect("not implicitly saved"), before);
    let updated = args(&[
        "--profile", path,
        "--remove-glyph", "ñ",
        "--glyph-stroke", "ñ", "300", "100", "2800", "3700",
        "500", "#123456", "--output", path, "--replace",
    ]);
    assert!(execute_glyph_library(&updated)
        .expect("explicitly replace library").is_none());
    assert_eq!(fs::read(&library).expect("updated library"), replaced);

    for format in ["pdf", "png"] {
        let mut selected = args(&[
            "--export-paper", format, "--printed-sheet", "true",
            "--glyph-file", path,
            "--place-text", "1", "25000", "30000", "ñn\u{303}",
        ]);
        let mut explicit = args(&[
            "--export-paper", format, "--printed-sheet", "true",
            "--glyph-stroke", "ñ", "300", "100", "2800", "3700",
            "500", "#123456",
            "--glyph-stroke", "n\u{303}", "0", "0", "3500", "4500",
            "300", "#123456",
            "--place-text", "1", "25000", "30000", "ñn\u{303}",
        ]);
        if format == "png" {
            selected.extend(args(&["--dpi", "72"]));
            explicit.extend(args(&["--dpi", "72"]));
        }
        assert_eq!(
            execute_notebook(&selected).expect("updated library exports")
                .expect("native bytes"),
            execute_notebook(&explicit).expect("inline new contours")
                .expect("native bytes"),
            "updated {format} must use only replacement glyph geometry",
        );
    }
    fs::remove_file(library).expect("remove owned library");
}

#[test]
fn glyph_audit_and_removal_refuse_bad_flags_without_writing_files() {
    let library = test_path().with_extension("tsv");
    let path = library.to_str().expect("source path");
    let initial = execute_glyph_library(&args(&[
        "--glyph-stroke", "ñ", "0", "0", "3500", "4500",
        "300", "#123456",
    ])).expect("valid template").expect("saved bytes");
    fs::write(&library, &initial).expect("test source");
    let target = test_path().with_extension("tsv");
    let output = target.to_str().expect("output path");
    fs::write(&target, b"existing library bytes").expect("sentinel");
    for input in [
        args(&["--profile", path, "--remove-glyph", "n\u{303}"]),
        args(&["--profile", path, "--remove-glyph", "z"]),
        args(&["--profile", path, "--remove-glyph"]),
        args(&["--profile", path, "--remove-glyph", "ñ"]),
        args(&["--profile", path, "--audit-text", "\n"]),
        args(&["--profile", path, "--audit-text", "A\u{0007}"]),
        args(&["--profile", path, "--audit-text", "a",
            "--audit-text", "b"]),
        args(&["--profile", path, "--audit-text", "a",
            "--output", output, "--replace"]),
        args(&["--profile", path, "--audit-text", &"ñ".repeat(4097)]),
    ] {
        let mut candidate = input;
        if !candidate.contains(&"--output".to_owned()) {
            candidate.extend(args(&["--output", output, "--replace"]));
        }
        assert!(execute_glyph_library(&candidate).is_err(),
            "mutating or unsupported audit must refuse");
        assert_eq!(fs::read(&target).expect("unchanged sentinel"),
            b"existing library bytes");
        assert_eq!(fs::read(&library).expect("unchanged source"), initial);
    }
    let audit = execute_glyph_library(&args(&[
        "--profile", path, "--audit-text", "ñ",
    ])).expect("audit stdout").expect("read-only report");
    assert!(String::from_utf8_lossy(&audit)
        .contains("Missing authored glyphs: 0"));
    fs::remove_file(library).expect("remove library");
    fs::remove_file(target).expect("remove target");
}

#[test]
fn english_spanish_glyph_baseline_audits_all_114_real_shapes() {
    let report = execute_glyph_library(&args(&["--audit-baseline"]))
        .expect("baseline audit without shapes")
        .expect("read-only report");
    let report = String::from_utf8(report).expect("UTF-8 codepoint report");
    assert!(report.contains("Frozen English/Spanish text baseline"));
    assert!(report.contains("math symbols are separate"));
    assert!(report.contains("Distinct visible graphemes: 114"));
    assert!(report.contains("Missing authored glyphs: 114"));
    assert!(report.contains("[U+00F1]: missing authored strokes"));
    assert!(report.contains("[U+00AB]: missing authored strokes"));
    assert!(report.contains("[U+00BB]: missing authored strokes"));
    assert!(report.contains("[U+006E U+0303]: missing authored strokes"));
    assert!(!report.contains("[U+221E]"),
        "mathematical glyphs require the separate inventory");

    let with_one = execute_glyph_library(&args(&[
        "--glyph-stroke", "ñ", "0", "0", "3500", "4500",
        "250", "#112233", "--audit-baseline",
    ])).expect("authored shape audited").expect("report");
    let with_one = String::from_utf8(with_one).expect("valid report");
    assert!(with_one.contains("Missing authored glyphs: 113"));
    assert!(with_one.contains("[U+00F1]: 1 authored strokes"));
    assert!(with_one.contains("[U+006E U+0303]: missing authored strokes"));
}

#[test]
fn glyph_baseline_audit_refuses_duplicate_or_mutating_selection() {
    for command in [
        args(&["--audit-baseline", "--audit-baseline"]),
        args(&["--audit-baseline", "--audit-text", "ñ"]),
        args(&["--audit-text", "ñ", "--audit-baseline"]),
        args(&["--audit-baseline", "--replace"]),
        args(&["--audit-baseline", "--output", "unused.tsv"]),
    ] {
        assert!(execute_glyph_library(&command).is_err(),
            "auditing must stay read-only and unambiguous");
    }
}

#[test]
fn explicit_unicode_fallback_exports_exact_selected_shapes_pdf_and_png() {
    let saved = test_path().with_extension("tsv");
    let name = saved.to_str().expect("library path");
    let saved_bytes = execute_glyph_library(&args(&[
        "--glyph-stroke", "n\u{303}", "0", "0", "3500", "4500",
        "280", "#1A2B3C",
        "--glyph-stroke", "n\u{303}", "0", "4500", "3500", "0",
        "280", "#1A2B3C",
    ])).expect("authored shape").expect("library bytes");
    fs::write(&saved, &saved_bytes).expect("test-owned library");

    for format in ["pdf", "png"] {
        let mut with_fallbacks = args(&[
            "--export-paper", format, "--printed-sheet", "true",
            "--glyph-file", name,
            "--fallback-glyph", "ñ", "n\u{303}",
            "--fallback-glyph", "∞", "n\u{303}",
            "--place-text", "2", "25000", "30000", "ñ n\u{303} ∞",
        ]);
        let mut manual_substitution = args(&[
            "--export-paper", format, "--printed-sheet", "true",
            "--glyph-file", name,
            "--place-text", "2", "25000", "30000",
            "n\u{303} n\u{303} n\u{303}",
        ]);
        if format == "pdf" {
            with_fallbacks.extend(args(&["--pages", "2"]));
            manual_substitution.extend(args(&["--pages", "2"]));
        } else {
            with_fallbacks.extend(args(&[
                "--png-page", "2", "--dpi", "72",
            ]));
            manual_substitution.extend(args(&[
                "--png-page", "2", "--dpi", "72",
            ]));
        }
        let fallback_output = execute_notebook(&with_fallbacks)
            .expect("explicit Unicode fallback accepted")
            .expect("native rendered bytes");
        let substituted_output = execute_notebook(&manual_substitution)
            .expect("explicit substitution accepted")
            .expect("native rendered bytes");
        assert_eq!(fallback_output, substituted_output,
            "{format} must render exactly the selected authored geometry");
        if format == "pdf" {
            assert!(fallback_output.starts_with(b"%PDF-"));
        } else {
            assert!(fallback_output.starts_with(b"\x89PNG\r\n\x1a\n"));
        }
    }
    assert_eq!(fs::read(&saved).expect("library unchanged"), saved_bytes);
    fs::remove_file(saved).expect("remove test-owned library");
}

#[test]
fn fallback_requires_valid_explicit_unambiguous_authored_target() {
    let template = args(&[
        "--export-paper", "pdf", "--printed-sheet", "true",
        "--glyph-stroke", "n\u{303}", "0", "0", "3500", "4500",
        "280", "#123456",
        "--place-text", "1", "25000", "30000", "ñ",
    ]);
    let missing = execute_notebook(&template)
        .expect_err("NFC may not borrow NFD shape implicitly");
    assert!(missing.contains("missing authored glyph shapes"), "{missing}");
    assert!(missing.contains("U+00F1"), "{missing}");

    for (extra, message) in [
        (args(&["--fallback-glyph", "ñ", "∞"]),
            "fallback target has no authored strokes"),
        (args(&["--fallback-glyph", "ñ", "ñ"]),
            "two distinct exact visible graphemes"),
        (args(&["--fallback-glyph", "ab", "n\u{303}"]),
            "two distinct exact visible graphemes"),
        (args(&["--fallback-glyph", "\n", "n\u{303}"]),
            "two distinct exact visible graphemes"),
        (args(&["--fallback-glyph", "ñ", "n\u{303}",
            "--fallback-glyph", "ñ", "n\u{303}"]),
            "duplicate exact glyph fallback"),
        (args(&["--fallback-glyph", "ñ", "∞",
            "--fallback-glyph", "∞", "n\u{303}"]),
            "chained glyph fallbacks refused"),
        (args(&["--fallback-glyph", "ñ"]),
            "requires MISSING AUTHORED"),
    ] {
        let mut command = template.clone();
        command.extend(extra);
        let error = execute_notebook(&command)
            .expect_err("ambiguous or unbacked fallback refused");
        assert!(error.contains(message), "{error}");
    }

    let mut already_authored = template.clone();
    already_authored.extend(args(&[
        "--glyph-stroke", "ñ", "0", "0", "3500", "4500",
        "280", "#123456", "--fallback-glyph", "ñ", "n\u{303}",
    ]));
    let error = execute_notebook(&already_authored)
        .expect_err("cannot override existing exact authored geometry");
    assert!(error.contains("source already has authored strokes"));

    let mut extra_only = args(&[
        "--export-paper", "pdf",
        "--glyph-stroke", "n\u{303}", "0", "0", "3500", "4500",
        "280", "#123456", "--fallback-glyph", "ñ", "n\u{303}",
    ]);
    let error = execute_notebook(&extra_only)
        .expect_err("unused fallback without placed text refused");
    assert!(error.contains("--place-text require one another"));
    extra_only.extend(args(&["--place-text", "1", "25000", "30000", "ñ"]));
    assert!(execute_notebook(&extra_only).is_ok());
}

#[test]
fn fallback_rejection_preserves_existing_selected_output() {
    let destination = test_path().with_extension("pdf");
    fs::write(&destination, b"previous valid output")
        .expect("test-owned sentinel");
    let path = destination.to_str().expect("path");
    let error = execute_notebook(&args(&[
        "--export-paper", "pdf", "--glyph-stroke", "a",
        "0", "0", "3500", "4500", "250", "#123456",
        "--place-text", "1", "25000", "30000", "«ñ»",
        "--fallback-glyph", "«", "a",
        "--fallback-glyph", "»", "a",
        "--output", path, "--replace",
    ])).expect_err("unmapped missing ñ refused");
    assert!(error.contains("U+00F1"), "{error}");
    assert!(!error.contains("U+00AB"), "mapped guillemet not missing");
    assert!(!error.contains("U+00BB"), "mapped guillemet not missing");
    assert_eq!(fs::read(&destination).expect("existing file"),
        b"previous valid output");
    fs::remove_file(destination).expect("remove test-owned output");
}

#[test]
fn fallback_uses_requested_graphemes_statistical_override() {
    let profile = test_path();
    let file = profile.to_str().expect("profile path");
    assert!(execute_handwriting(&args(&[
        "--set-glyph", "ñ", "width", "2800", "2800", "2800", "2800", "0",
        "--set-glyph", "ñ", "slant", "12000", "12000", "12000",
        "12000", "0",
        "--output", file,
    ])).expect("test-owned statistics file").is_none());
    let borrowed = execute_notebook(&args(&[
        "--export-paper", "png", "--dpi", "72",
        "--printed-sheet", "true", "--handwriting-profile", file,
        "--glyph-stroke", "n\u{303}", "0", "0", "3500", "4500",
        "300", "#1A2B3C", "--fallback-glyph", "ñ", "n\u{303}",
        "--place-text", "1", "25000", "30000", "ñ",
    ])).expect("authored substitute under requested statistics")
        .expect("PNG bytes");
    let original = execute_notebook(&args(&[
        "--export-paper", "png", "--dpi", "72",
        "--printed-sheet", "true", "--handwriting-profile", file,
        "--glyph-stroke", "ñ", "0", "0", "3500", "4500",
        "300", "#1A2B3C", "--place-text", "1", "25000", "30000", "ñ",
    ])).expect("same original-specific statistics")
        .expect("PNG bytes");
    assert_eq!(borrowed, original,
        "shape substitute must retain original glyph mean width and slant");
    fs::remove_file(profile).expect("remove test-owned profile");
}

#[test]
fn flow_markdown_wraps_to_two_physical_pages_with_headings_and_bullets() {
    let markdown = format!("# a\n\na a a\n- a\n{}", "a\n".repeat(34));
    let mut glyphs = GlyphOverlay::default();
    glyphs.parse("--glyph-stroke", &mut args(&[
        "a", "0", "0", "3500", "4500", "250", "#204060",
    ]).iter()).expect("one exact authored shape");
    let source = FlowSource::new(FlowFormat::Markdown, &markdown)
        .expect("bounded source");
    let profile = NotebookProfile::default();
    let (runs, decorations, pages) = layout_flow(
        &source, &profile, None, HeadingDecoration::default(),
    ).expect("measured flow");
    assert_eq!(pages, 2);
    assert_eq!(runs.first().expect("heading").text, "a",
        "heading marker removed");
    assert_eq!(runs.get(1).expect("paragraph").text, "a a a");
    assert_eq!(runs.get(2).expect("list").text, "a",
        "list marker replaced by ink");
    assert_eq!(decorations.len(), 2, "underline and list mark");
    assert_eq!(decorations.first().expect("heading rule").page, 1);
    assert_eq!(decorations.get(1).expect("bullet marker").page, 1);
    assert!(runs.iter().any(|run| run.page == 2));
    for run in &runs {
        assert!(run.origin_um.0 >= 22_500);
        assert!(run.origin_um.1 >= 24_500);
    }
    // The exported PDF must auto-select exactly two physical-size pages,
    // and PNG page 2 must contain different, actually placed ink.
    let command = args(&[
        "--printed-sheet", "true", "--export-paper", "pdf",
        "--glyph-stroke", "a", "0", "0", "3500", "4500",
        "250", "#204060", "--flow-markdown", &markdown,
    ]);
    let pdf = execute_notebook(&command).expect("two-page PDF")
        .expect("PDF bytes");
    assert!(pdf.starts_with(b"%PDF-1.4"));
    let streams = String::from_utf8_lossy(&pdf)
        .split("\nstream\n").skip(1)
        .map(|entry| entry.split("\nendstream").next().unwrap_or("")
            .to_owned()).collect::<Vec<_>>();
    assert_eq!(streams.len(), 2);
    assert!(streams.first().expect("page 1").contains(" m\n"));
    assert!(streams.get(1).expect("page 2").contains(" m\n"));
    let mut png_command = args(&[
        "--printed-sheet", "true", "--export-paper", "png",
        "--dpi", "72", "--png-page", "2",
        "--glyph-stroke", "a", "0", "0", "3500", "4500",
        "250", "#204060", "--flow-markdown", &markdown,
    ]);
    let png_second = execute_notebook(&png_command)
        .expect("page 2 PNG").expect("PNG bytes");
    assert!(png_second.starts_with(b"\x89PNG\r\n\x1a\n"));
    let page_field = png_command.iter_mut().find(|value| value.as_str() == "2")
        .expect("selected PNG page 2");
    *page_field = "1".into();
    let png_first = execute_notebook(&png_command)
        .expect("page 1 PNG").expect("PNG bytes");
    assert_ne!(png_second, png_first);
}

#[test]
fn flow_wraps_long_unicode_words_by_exact_grapheme_and_preserves_ink() {
    let source = FlowSource::new(
        FlowFormat::Plain, &"n\u{303}".repeat(130),
    ).expect("Unicode source");
    let mut glyphs = GlyphOverlay::default();
    glyphs.parse("--glyph-stroke", &mut args(&[
        "n\u{303}", "0", "0", "3500", "4500", "250", "#204060",
    ]).iter()).expect("decomposed authored shape");
    let (runs, _, pages) = layout_flow(
        &source, &NotebookProfile::default(), None,
        HeadingDecoration::default(),
    ).expect("exact grapheme boundaries");
    assert_eq!(pages, 1);
    assert!(runs.len() > 1);
    assert_eq!(runs.iter().map(|run| UnicodeGraphemeSegmentation
        .grapheme_count(&run.text)).sum::<usize>(), 130);
    assert!(runs.iter().all(|run| {
        run.text.chars().count() == UnicodeGraphemeSegmentation
            .grapheme_count(&run.text).saturating_mul(2)
    }), "no decomposed combining mark is split across rows");
    glyphs.runs = runs;
    assert_eq!(glyphs.expand(&[], None)
        .expect("authoritative physical glyph placement").len(), 130);
}

#[test]
fn flow_refuses_overflow_missing_shapes_and_mutation_before_commit() {
    let target = test_path().with_extension("pdf");
    let filename = target.to_str().expect("owned target");
    fs::write(&target, b"previous valid PDF").expect("sentinel");
    let base = args(&[
        "--printed-sheet", "true", "--export-paper", "pdf",
        "--glyph-stroke", "a", "0", "0", "3500", "4500",
        "250", "#204060",
    ]);
    for (source, expected) in [
        (format!("# {}", "a".repeat(65)),
            "heading exceeds one-line fixed region"),
        ("a \u{f1}".into(), "U+00F1"),
    ] {
        let mut command = base.clone();
        command.extend(args(&[
            "--flow-markdown", &source, "--output", filename, "--replace",
        ]));
        let error = execute_notebook(&command)
            .expect_err("invalid flow refuses before overwrite");
        assert!(error.contains(expected), "{error}");
        assert_eq!(fs::read(&target).expect("unchanged"),
            b"previous valid PDF");
    }
    let mut insufficient = base;
    insufficient.extend(args(&[
        "--flow-text", &"a\n".repeat(40), "--pages", "1",
        "--output", filename, "--replace",
    ]));
    assert!(execute_notebook(&insufficient).expect_err("page shortage")
        .contains("requires 2 physical pages"));
    assert_eq!(fs::read(&target).expect("unchanged"),
        b"previous valid PDF");
    for invalid in ["\t", "a\0", "a\r", "a\n\u{0008}"] {
        assert!(FlowSource::new(FlowFormat::Plain, invalid).is_err());
    }
    fs::remove_file(target).expect("remove owned target");
}

#[test]
fn flow_file_reads_bounded_local_markdown_and_refuses_unsafe_inputs() {
    let source = test_path().with_extension("md");
    let name = source.to_str().expect("selected source file");
    let document = "# a\n\na a\n- a\n";
    fs::write(&source, document).expect("test-owned Markdown");
    let base = args(&[
        "--printed-sheet", "true", "--export-paper", "pdf",
        "--glyph-stroke", "a", "0", "0", "3500", "4500",
        "250", "#204060",
    ]);
    let mut inline = base.clone();
    inline.extend(args(&["--flow-markdown", document]));
    let mut from_file = base;
    from_file.extend(args(&["--flow-markdown-file", name]));
    assert_eq!(execute_notebook(&inline).expect("inline PDF"),
        execute_notebook(&from_file).expect("read-only file PDF"));
    assert_eq!(fs::read(&source).expect("still original"),
        document.as_bytes());
    fs::write(&source, vec![b'a'; 4_097]).expect("oversize file");
    assert!(execute_notebook(&from_file).expect_err("bound refused")
        .contains("notebook input refused"));
    fs::write(&source, [0xff, 0xfe]).expect("invalid bytes");
    assert!(execute_notebook(&from_file).expect_err("UTF-8 refused")
        .contains("flow file must be UTF-8"));
    fs::remove_file(source).expect("remove test-owned Markdown");
}

#[test]
fn flow_preserves_explicit_ascii_spaces_and_same_ink_advances() {
    let source = FlowSource::new(FlowFormat::Plain, "  a  a")
        .expect("visible text with preserved indentation");
    let mut glyphs = GlyphOverlay::default();
    glyphs.parse("--glyph-stroke", &mut args(&[
        "a", "0", "0", "3500", "4500", "250", "#204060",
    ]).iter()).expect("authored a shape");
    let (runs, ornaments, pages) = layout_flow(
        &source, &NotebookProfile::default(), None,
        HeadingDecoration::default(),
    ).expect("preserved whitespace");
    assert_eq!(pages, 1);
    assert!(ornaments.is_empty());
    assert_eq!(runs.first().expect("one text row").text, "  a  a");
    glyphs.runs = runs;
    let ink = glyphs.expand(&[], None).expect("same glyph placement");
    assert_eq!(ink.len(), 2);
    let first = ink.first().expect("first a");
    let second = ink.get(1).expect("second a");
    assert_eq!(first.start_um.0, 30_500);
    assert_eq!(second.start_um.0, 42_500);
}


#[test]
fn native_seeded_variation_matches_browser_accent_replay_on_three_occurrences() {
    // Expected local segments/tracking were independently generated by the
    // current TypeScript varyGlyphStrokes and browser spacing sampler.
    let mut glyphs = GlyphOverlay::default();
    for fields in [
        ["á", "900", "1200", "1700", "2400", "250", "#A1B2C3"],
        ["á", "1700", "2400", "2400", "3100", "250", "#A1B2C3"],
        ["á", "1300", "700", "1600", "400", "250", "#A1B2C3"],
    ] {
        glyphs.parse("--glyph-stroke", &mut args(&fields).iter())
            .expect("authored connected ink");
    }
    glyphs.parse("--glyph-variation", &mut args(&[
        "42", "8", "8", "10", "7", "200", "40",
    ]).iter()).expect("bounded creative variation");
    glyphs.parse("--place-text", &mut args(&[
        "1", "25000", "30000", "ááá",
    ]).iter()).expect("three occurrences");
    let placed = glyphs.expand(&[], None).expect("all ink stays on paper");
    assert_eq!(placed.len(), 9);
    let expected = [
        (0, -93, [(1170, 1225, 1904, 2362, 245),
                   (1904, 2362, 2486, 3067, 245),
                   (1641, 741, 1927, 497, 245)]),
        (1, 143, [(595, 1214, 1471, 2375, 274),
                  (1471, 2375, 2287, 3074, 274),
                  (906, 720, 1219, 450, 274)]),
        (2, 173, [(868, 1202, 1702, 2429, 250),
                  (1702, 2429, 2494, 3124, 250),
                  (1265, 700, 1616, 362, 250)]),
    ];
    for (occurrence, tracking, segments) in expected {
        for (i, (x1, y1, x2, y2, width)) in
            segments.into_iter().enumerate() {
            let line = &placed[occurrence * 3 + i];
            let x = 25_000 + u32::try_from(occurrence * 4_000)
                .expect("finite index") + u32::try_from(
                    200 + tracking,
                ).expect("bounded offset");
            assert_eq!(line.start_um, (x + x1 - 200, 30_000 + y1));
            assert_eq!(line.end_um, (x + x2 - 200, 30_000 + y2));
            assert_eq!(line.width_um, width);
            assert_eq!(line.rgb, [0xA1, 0xB2, 0xC3]);
        }
    }
    assert_eq!(glyphs.strokes[0].start_um, (900, 1200),
        "replay never mutates saved authored ink");
    assert_eq!(placed, glyphs.expand(&[], None).expect("identical replay"));
}

#[test]
fn native_variation_explicitly_changes_pdf_png_without_changing_base_ink() {
    let mut command = args(&[
        "--printed-sheet", "true", "--export-paper", "pdf",
        "--pages", "2", "--glyph-stroke", "ó", "500", "1000",
        "2000", "3000", "250", "#123456",
        "--place-text", "1", "25000", "30000", "óó",
        "--place-text", "2", "25000", "30000", "óó",
    ]);
    let ideal = execute_notebook(&command).expect("canonical PDF")
        .expect("PDF bytes");
    command.extend(args(&[
        "--glyph-variation", "42", "8", "8", "10", "7", "200", "40",
    ]));
    let varied = execute_notebook(&command).expect("varied PDF")
        .expect("PDF bytes");
    assert!(varied.starts_with(b"%PDF-1.4"));
    assert_ne!(varied, ideal);
    let mut different_seed = command.clone();
    let seed_at = different_seed.iter()
        .position(|entry| entry == "--glyph-variation")
        .expect("explicit variation seed");
    *different_seed.get_mut(seed_at.saturating_add(1))
        .expect("selected seed value") = "45".into();
    assert_ne!(varied, execute_notebook(&different_seed)
        .expect("different seed").expect("PDF bytes"));
    assert_eq!(varied, execute_notebook(&command).expect("replay")
        .expect("PDF bytes"));
    let contents = String::from_utf8_lossy(&varied);
    let streams = contents.split("\nstream\n").skip(1)
        .map(|entry| entry.split("\nendstream").next().unwrap_or("")
            .to_owned()).collect::<Vec<_>>();
    assert_eq!(streams.len(), 2);
    assert_eq!(streams[0].matches(" m\n").count(), 2);
    assert_eq!(streams[1].matches(" m\n").count(), 2);
    assert_ne!(streams[0], streams[1],
        "occurrences should keep varying across physical pages");

    let mut png = command.clone();
    let _replaced = png.splice(2..6, args(&[
        "--export-paper", "png", "--dpi", "72", "--png-page", "2",
    ])).collect::<Vec<_>>();
    let first = execute_notebook(&png).expect("varied PNG")
        .expect("PNG bytes");
    assert!(first.starts_with(b"\x89PNG\r\n\x1a\n"));
    assert_eq!(first, execute_notebook(&png).expect("PNG replay")
        .expect("PNG bytes"));
    let mut ideal_png = png.clone();
    ideal_png.truncate(ideal_png.len() - 8);
    let untouched = execute_notebook(&ideal_png).expect("ideal PNG")
        .expect("PNG bytes");
    assert_ne!(first, untouched);
}

#[test]
fn invalid_native_variation_is_atomic_and_disallows_hidden_sampling() {
    let target = test_path();
    let filename = target.to_str().expect("test target");
    fs::write(&target, b"keep-current-output").expect("owned sentinel");
    let valid = args(&[
        "--export-paper", "pdf", "--glyph-stroke", "ñ", "500",
        "1000", "2000", "3000", "250", "#123456",
        "--place-text", "1", "25000", "30000", "ññ",
        "--output", filename, "--replace",
    ]);
    for invalid in [
        args(&["--glyph-variation", "-1", "0", "0", "0", "0", "0", "0"]),
        args(&["--glyph-variation", "4294967296", "0", "0", "0", "0", "0", "0"]),
        args(&["--glyph-variation", "42", "21", "0", "0", "0", "0", "0"]),
        args(&["--glyph-variation", "42", "0", "0", "0", "13", "0", "0"]),
        args(&["--glyph-variation", "42", "0", "0", "0", "0", "501", "0"]),
        args(&["--glyph-variation", "42", "0", "0", "0", "0", "0", "151"]),
        args(&["--glyph-variation", "42", "20", "20", "20", "12", "500", "150",
            "--glyph-variation", "0", "0", "0", "0", "0", "0", "0"]),
    ] {
        let mut command = valid.clone();
        command.extend(invalid);
        assert!(execute_notebook(&command).is_err(), "{command:?}");
        assert_eq!(fs::read(&target).expect("immutable output"),
            b"keep-current-output");
    }
    // A source segment touching the cell border can be valid authored
    // ink for older exports but cannot be varied with pen clearance.
    let edge = args(&[
        "--export-paper", "pdf", "--glyph-stroke", "ñ", "0", "0",
        "3500", "4500", "250", "#123456",
        "--place-text", "1", "25000", "30000", "ñ",
        "--glyph-variation", "0", "0", "0", "0", "0", "0", "0",
        "--output", filename, "--replace",
    ]);
    let error = execute_notebook(&edge).expect_err("ink clearance refusal");
    assert!(error.contains("fit inside glyph cell"), "{error}");
    assert_eq!(fs::read(&target).expect("refusal unchanged"),
        b"keep-current-output");
    let mut crowded = GlyphOverlay::default();
    for _ in 0..129 {
        crowded.parse("--glyph-stroke", &mut args(&[
            "ñ", "500", "1000", "2000", "3000", "250", "#123456",
        ]).iter()).expect("valid authored library");
    }
    crowded.parse("--place-text", &mut args(&[
        "1", "25000", "30000", "ñ",
    ]).iter()).expect("placed Unicode source");
    crowded.parse("--glyph-variation", &mut args(&[
        "0", "0", "0", "0", "0", "0", "0",
    ]).iter()).expect("zero spread allowed");
    assert!(crowded.expand(&[], None).expect_err("128 cap")
        .contains("limited to 128 strokes"));
    assert!(execute_notebook(&args(&[
        "--export-paper", "pdf", "--glyph-variation", "42",
        "0", "0", "0", "0", "0", "0",
    ])).is_err(), "no ink or placed text must refuse");
    fs::remove_file(target).expect("remove test-owned sentinel");
}


#[test]
fn seeded_notebook_markdown_flow_varies_across_physical_pages() {
    // The existing measured-flow layout, heading and bullet rules are
    // unchanged; native ink variation is applied after pagination.
    let markdown = format!("# á\n\n- á á\n{}", "á\n".repeat(34));
    let original = args(&[
        "--printed-sheet", "true", "--export-paper", "pdf",
        "--glyph-stroke", "á", "900", "1200", "2400", "3100",
        "250", "#123456", "--flow-markdown", &markdown,
    ]);
    let ideal = execute_notebook(&original)
        .expect("original flow accepted").expect("PDF bytes");
    let mut sampled = original.clone();
    sampled.extend(args(&[
        "--glyph-variation", "4294967295", "8", "8", "10",
        "7", "200", "40",
    ]));
    let pdf = execute_notebook(&sampled).expect("sampled flow")
        .expect("PDF bytes");
    let content = String::from_utf8_lossy(&pdf);
    let streams = content.split("\nstream\n").skip(1)
        .map(|entry| entry.split("\nendstream").next().unwrap_or(""))
        .collect::<Vec<_>>();
    assert_eq!(streams.len(), 2, "two measured physical PDF pages");
    assert!(streams.iter().all(|page| page.contains(" m\n")));
    assert_ne!(ideal, pdf);
    assert_eq!(pdf, execute_notebook(&sampled).expect("replay")
        .expect("PDF bytes"));
    let mut png = args(&[
        "--printed-sheet", "true", "--export-paper", "png",
        "--png-page", "2", "--dpi", "72",
        "--glyph-stroke", "á", "900", "1200", "2400", "3100",
        "250", "#123456", "--flow-markdown", &markdown,
    ]);
    png.extend(args(&[
        "--glyph-variation", "4294967295", "8", "8", "10",
        "7", "200", "40",
    ]));
    let ink_page_two = execute_notebook(&png)
        .expect("sampled PNG page 2").expect("PNG bytes");
    assert!(ink_page_two.starts_with(b"\x89PNG\r\n\x1a\n"));
    assert_eq!(ink_page_two, execute_notebook(&png)
        .expect("replayed PNG page 2").expect("PNG bytes"));
}


#[test]
fn seeded_variation_keeps_exact_nfd_fallback_and_applies_selected_means() {
    // The target NFD identity controls statistics, even when an exact
    // separately authored source provides the fallback pen gesture.
    let statistics = execute_handwriting(&args(&[
        "--set-glyph", "n\u{303}", "width", "3000", "3000",
        "3000", "3000", "0",
        "--set-glyph", "n\u{303}", "height", "4000", "4000",
        "4000", "4000", "0",
        "--set-glyph", "n\u{303}", "ink_thickness", "500", "500",
        "500", "500", "0",
        "--set-glyph", "n\u{303}", "letter_spacing", "600", "600",
        "600", "600", "0",
        "--set-glyph", "n\u{303}", "baseline_offset", "1000", "1000",
        "1000", "1000", "0",
    ])).expect("target grapheme statistical profile")
        .expect("canonical JSON");
    let model = decode_statistics_json(&statistics, &UnicodeGraphemeSegmentation)
        .expect("reopen exact grapheme means");
    let mut glyphs = GlyphOverlay::default();
    glyphs.parse("--glyph-stroke", &mut args(&[
        "á", "900", "1200", "1700", "2400", "250", "#A1B2C3",
    ]).iter()).expect("authored source ink");
    glyphs.parse("--fallback-glyph", &mut args(&[
        "n\u{303}", "á",
    ]).iter()).expect("explicit NFD fallback");
    glyphs.parse("--place-text", &mut args(&[
        "1", "25000", "30000", "n\u{303}n\u{303}",
    ]).iter()).expect("two NFD graphemes");
    glyphs.parse("--glyph-variation", &mut args(&[
        "42", "8", "8", "10", "7", "200", "40",
    ]).iter()).expect("admitted variation");
    let placed = glyphs.expand(&[], Some(&model))
        .expect("fallback, mean and per-occurrence variation admitted");
    assert_eq!(placed.len(), 2);
    assert_eq!(placed.first().expect("first NFD").start_um,
        (25_910, 32_089));
    assert_eq!(placed.first().expect("first NFD").width_um, 408);
    assert_eq!(placed.get(1).expect("second NFD").start_um,
        (29_253, 32_079));
    assert_eq!(placed.get(1).expect("second NFD").width_um, 457);
    assert_eq!(placed, glyphs.expand(&[], Some(&model))
        .expect("replayed NFD variation"));
}

#[test]
fn marker_headings_keep_authored_text_and_bullets_with_colored_ink() {
    let source = FlowSource::new(FlowFormat::Markdown, "# a\n- a")
        .expect("bounded Markdown");
    let mut marker = HeadingDecoration::default();
    marker.parse("--heading-style", &mut args(&["marker"]).iter())
        .expect("marker selection");
    marker.parse("--heading-color", &mut args(&["#E2D16F"]).iter())
        .expect("exact RGB selection");
    let (runs, ornaments, pages) = layout_flow(
        &source, &NotebookProfile::default(), None, marker,
    ).expect("marker heading measured");
    assert_eq!(pages, 1);
    assert_eq!(runs.len(), 2);
    assert_eq!(runs.first().expect("heading").text, "a");
    assert_eq!(runs.get(1).expect("bullet").text, "a");
    assert_eq!(ornaments.len(), 5, "four marker bands and one bullet");
    for (index, stroke) in ornaments.iter().take(4).enumerate() {
        assert_eq!(stroke.width_um, 1_400);
        assert_eq!(stroke.rgb, [0xE2, 0xD1, 0x6F]);
        assert_eq!(stroke.page, 1);
        assert_eq!(stroke.start_um.0, runs[0].origin_um.0);
        assert_eq!(stroke.end_um.0, runs[0].origin_um.0 + 4_000);
        assert_eq!(stroke.start_um.1,
            runs[0].origin_um.1 + [900, 1900, 2900, 3900][index]);
    }
    assert_eq!(ornaments.last().expect("bullet").width_um, 450);

    let (_, default_ornaments, _) = layout_flow(
        &source, &NotebookProfile::default(), None,
        HeadingDecoration::default(),
    ).expect("original underline unchanged");
    assert_eq!(default_ornaments.len(), 2);
    assert_eq!(default_ornaments[0].width_um, 550);
    assert_eq!(default_ornaments[0].rgb, [30, 48, 74]);

    let mut none = HeadingDecoration::default();
    none.parse("--heading-style", &mut args(&["none"]).iter())
        .expect("no heading ornament");
    let (_, none_ornaments, _) = layout_flow(
        &source, &NotebookProfile::default(), None, none,
    ).expect("no ornament measured");
    assert_eq!(none_ornaments.len(), 1, "bullet remains visible");
}

#[test]
fn custom_marker_heading_exports_native_pdf_png_and_replays() {
    let mut command = args(&[
        "--printed-sheet", "true", "--export-paper", "pdf",
        "--glyph-stroke", "a", "900", "1200", "2400", "3200",
        "250", "#123456", "--flow-markdown", "# a\n\na",
    ]);
    let underline = execute_notebook(&command).expect("default heading")
        .expect("PDF bytes");
    command.extend(args(&[
        "--heading-style", "marker", "--heading-color", "#E2D16F",
    ]));
    let pdf = execute_notebook(&command).expect("marker PDF")
        .expect("PDF bytes");
    assert!(pdf.starts_with(b"%PDF-1.4"));
    assert_ne!(pdf, underline);
    let content = String::from_utf8_lossy(&pdf);
    let first_marker = content.find("0.886 0.820 0.435 RG")
        .expect("exact heading color in native PDF");
    let glyph_ink = content.find("0.071 0.204 0.337 RG")
        .expect("original glyph stroke color in native PDF");
    assert!(first_marker < glyph_ink,
        "the wide marker band must be laid behind authored glyph ink");
    assert_eq!(content.matches("0.886 0.820 0.435 RG").count(), 4);
    assert_eq!(pdf, execute_notebook(&command).expect("replay")
        .expect("PDF bytes"));

    let mut png = command.clone();
    let format_at = png.iter().position(|item| item == "pdf")
        .expect("PDF switch");
    *png.get_mut(format_at).expect("format value") = "png".into();
    png.extend(args(&["--dpi", "72"]));
    let png_bytes = execute_notebook(&png).expect("marker PNG")
        .expect("PNG bytes");
    assert!(png_bytes.starts_with(b"\x89PNG\r\n\x1a\n"));
    assert_eq!(png_bytes, execute_notebook(&png).expect("PNG replay")
        .expect("PNG bytes"));
    let mut plain = png.clone();
    plain.truncate(plain.len() - 6);
    assert_ne!(png_bytes, execute_notebook(&plain).expect("plain PNG")
        .expect("PNG bytes"));
}

#[test]
fn invalid_heading_decorations_do_not_replace_existing_pdf() {
    let target = test_path();
    let filename = target.to_str().expect("owned target");
    fs::write(&target, b"keep-existing-pdf").expect("sentinel");
    let base = args(&[
        "--printed-sheet", "true", "--export-paper", "pdf",
        "--glyph-stroke", "a", "900", "1200", "2400", "3200",
        "250", "#123456", "--flow-markdown", "# a",
        "--output", filename, "--replace",
    ]);
    for invalid in [
        args(&["--heading-style", "invalid"]),
        args(&["--heading-style", "marker", "--heading-style", "none"]),
        args(&["--heading-color", "#XYZXYZ"]),
        args(&["--heading-color", "#12345"]),
        args(&["--heading-color", "#112233", "--heading-color", "#445566"]),
        args(&["--heading-style", "none", "--heading-color", "#112233"]),
        args(&["--heading-color"]),
    ] {
        let mut command = base.clone();
        command.extend(invalid);
        assert!(execute_notebook(&command).is_err(), "{command:?}");
        assert_eq!(fs::read(&target).expect("unchanged"), b"keep-existing-pdf");
    }
    assert!(execute_notebook(&args(&[
        "--export-paper", "pdf", "--heading-style", "marker",
    ])).is_err(), "heading customization requires Markdown");
    fs::remove_file(target).expect("remove test-owned sentinel");
}

#[test]
fn marker_and_seeded_handwriting_share_two_measured_pdf_png_pages() {
    // A real heading-only Markdown page fills, so new marker bands and
    // varied glyphs must be kept in their assigned physical page order.
    let markdown = "# á\n".repeat(36);
    let mut command = args(&[
        "--printed-sheet", "true", "--export-paper", "pdf",
        "--glyph-stroke", "á", "1100", "1300", "2200", "2800",
        "250", "#123456", "--flow-markdown", &markdown,
        "--heading-style", "marker", "--heading-color", "#F8DE91",
        "--glyph-variation", "42", "8", "8", "10", "7", "200", "40",
    ]);
    let pdf = execute_notebook(&command).expect("2-page marker and ink PDF")
        .expect("PDF bytes");
    let content = String::from_utf8_lossy(&pdf);
    let streams = content.split("\nstream\n").skip(1)
        .map(|entry| entry.split("\nendstream").next().unwrap_or(""))
        .collect::<Vec<_>>();
    assert_eq!(streams.len(), 2);
    assert_eq!(content.matches("0.973 0.871 0.569 RG").count(), 144,
        "four colored background strokes for each Markdown heading");
    for page in &streams {
        let marker = page.find("0.973 0.871 0.569 RG")
            .expect("marker on every page");
        let glyph = page.find("0.071 0.204 0.337 RG")
            .expect("authored glyph on every page");
        assert!(marker < glyph, "marker is below ink on each page");
    }
    assert_eq!(pdf, execute_notebook(&command).expect("PDF replay")
        .expect("replayed bytes"));
    let format_at = command.iter().position(|entry| entry == "pdf")
        .expect("format value");
    *command.get_mut(format_at).expect("format switch") = "png".into();
    command.extend(args(&["--dpi", "72", "--png-page", "2"]));
    let png = execute_notebook(&command).expect("page 2 ink and marker")
        .expect("PNG bytes");
    assert!(png.starts_with(b"\x89PNG\r\n\x1a\n"));
    assert_eq!(png, execute_notebook(&command).expect("PNG replay")
        .expect("PNG bytes"));
}
