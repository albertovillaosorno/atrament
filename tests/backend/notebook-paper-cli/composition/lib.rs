// Copyright:
//   - Copyright © 2026 Alberto Villa Osorno.
// SPDX-License-Identifier:
//   - MIT
// Confidential:
//   - false
// License-File:
//   - LICENSE
//
// Boundary-Contract:
// - Owns:
//   - Native paper CLI geometry and argument regression fixtures.
// - Must-Not:
//   - Claim handwriting, embedded images, or full notebook rendering.
// - Allows:
//   - Inputs: Explicit deterministic page and CLI option values.
//   - Outputs: Assertions over PDF bytes and typed refusals.
//   - Side effects: Process-local test allocation only.
// - Split-When:
//   - Full notebook PDF/PNG integration needs separate regression evidence.
// - Merge-When:
//   - Native CLI is subsumed by the shared notebook profile editor.
// - Summary:
//   - Verifies physical paper generation without hidden file effects.
// - Description:
//   - Covers blank, ruled, grid, dots, physical sizes, and invalid options.
// - Usage:
//   - cargo test -p atrament_notebook_paper_cli
// - Defaults:
//   - No filesystem outputs, dependencies, or shared mutable state.
//

//! Compiled against the native binary's real Rust geometry and PDF mapping.

#![expect(
    clippy::expect_used,
    clippy::panic,
    reason = "test-only fail-fast physical PDF fixture assertions"
)]

use super::*;
use crate::cli::{execute, parse_options, read_profile};
use std::env;

#[test]
fn grid_page_contains_physical_media_box_and_vector_marks() {
    let output = render(Options::default()).expect("valid A4 grid");
    let pdf = String::from_utf8_lossy(&output);
    assert!(output.starts_with(b"%PDF-1.4"));
    assert!(pdf.contains("/MediaBox [0 0 595.276 841.890]"));
    assert!(pdf.contains("/Count 1"));
    assert!(pdf.matches(" m\n").count() >= 50);
}

#[test]
fn blank_page_is_smaller_than_horizontal_ruled_paper() {
    let blank = render(Options {
        style: Style::Blank,
        ..Options::default()
    })
    .expect("blank page");
    let ruled = render(Options {
        style: Style::Ruled,
        ..Options::default()
    })
    .expect("ruled page");
    assert!(blank.len() < ruled.len());
    assert!(!String::from_utf8_lossy(&blank).contains(" m\n"));
    assert!(String::from_utf8_lossy(&ruled).matches(" m\n").count() >= 40);
}

#[test]
fn dots_are_vector_painted_and_extreme_density_is_refused() {
    let dots = render(Options {
        style: Style::Dot,
        ..Options::default()
    })
    .expect("dotted page");
    let pdf = String::from_utf8_lossy(&dots);
    assert!(pdf.contains("\nf\n"));
    assert!(pdf.matches(" m\n").count() > 1_000);
    assert!(
        render(Options {
            style: Style::Dot,
            spacing_um: 1_000,
            ..Options::default()
        })
        .is_err()
    );
}

#[test]
fn letter_size_and_exact_fractional_millimeters_round_trip() {
    let args = vec![
        "--width-mm".into(),
        "215.9".into(),
        "--height-mm".into(),
        "279.4".into(),
        "--spacing-mm".into(),
        "4.5".into(),
        "--line-color".into(),
        "#123ABC".into(),
    ];
    let opts = parse_options(&args)
        .expect("valid options")
        .expect("not help")
        .options;
    assert_eq!(opts.width_um, 215_900);
    assert_eq!(opts.spacing_um, 4_500);
    let data = render(opts).expect("valid Letter page");
    let pdf = String::from_utf8_lossy(&data);
    assert!(pdf.contains("/MediaBox [0 0 612.000 792.000]"));
    assert!(pdf.contains("0.071 0.227 0.737 RG"));
}

#[test]
fn invalid_options_are_refused_before_pdf_bytes_exist() {
    let invalid = [
        vec!["--style", "dot", "--spacing-mm", "1"],
        vec!["--margin-left-mm", "210"],
        vec!["--width-mm", "abc"],
        vec!["--line-color", "red"],
        vec!["--line-color", "#\u{e9}1234"],
        vec!["--spacing-mm", "0"],
        vec!["--width-mm", "1.2345"],
        vec!["--does-not-exist", "1"],
        vec!["--height-mm"],
    ];
    for items in invalid {
        let args = items.into_iter().map(str::to_owned).collect::<Vec<_>>();
        match parse_options(&args) {
            Ok(Some(options)) => {
                assert!(render(options.options).is_err(), "{args:?}");
            }
            Err(_error) => {}
            Ok(None) => panic!("invalid options must not show help: {args:?}"),
        }
    }
}

#[test]
fn paper_color_requires_six_ascii_hex_digits_in_cli_and_profile() {
    let canonical = encode_profile(Options::default());
    for color in [
        "#+A1122", "#11+A22", "#1122+A", "#-A1122", "#11G222", "#11_222",
        "#11 222",
    ] {
        let args = ["--line-color", color].map(str::to_owned);
        assert!(
            parse_options(&args).is_err(),
            "invalid color accepted from CLI: {color}"
        );
        let profile = canonical.replace("#C5D4E7", color);
        assert!(
            decode_profile(profile.as_bytes()).is_err(),
            "invalid color accepted from paper profile: {color}"
        );
    }
    for color in ["#012345", "#AABBCC", "#aaBBcc"] {
        let args = ["--line-color", color].map(str::to_owned);
        assert!(parse_options(&args).is_ok(), "valid color: {color}");
        let profile = canonical.replace("#C5D4E7", color);
        assert!(
            decode_profile(profile.as_bytes()).is_ok(),
            "valid profile color: {color}"
        );
    }
}

#[test]
fn versioned_paper_json_round_trips_without_altering_pdf() {
    for style in [Style::Blank, Style::Ruled, Style::Grid, Style::Dot] {
        let options = Options {
            color: PdfRgb {
                blue: 0xF3,
                green: 0x22,
                red: 0x10,
            },
            height_um: 279_400,
            spacing_um: 4_500,
            style,
            width_um: 215_900,
            ..Options::default()
        };
        let json = encode_profile(options);
        let restored = decode_profile(json.as_bytes())
            .expect("canonical paper-only JSON must reopen");
        assert_eq!(encode_profile(restored), json);
        assert_eq!(render(restored), render(options));
    }
}

#[test]
fn json_unicode_escapes_are_admitted_by_shared_syntax_evidence() {
    let canonical = encode_profile(Options::default());
    let escaped = canonical
        .replace("\"style\"", "\"st\\u0079le\"")
        .replace("\"grid\"", "\"gri\\u0064\"");
    let options = decode_profile(escaped.as_bytes())
        .expect("escaped recognized fields and values decode");
    assert_eq!(style_name(options.style), "grid");
}

#[test]
fn paper_profile_rejects_duplicates_unknown_fields_and_malformed_values() {
    let canonical = encode_profile(Options::default());
    let invalid = [
        canonical.replace("\"schema\"", "\"unknown\""),
        canonical.replace("atrament.paper.v1", "atrament.paper.v2"),
        canonical.replace("\"style\": \"grid\"", "\"style\": 5"),
        canonical.replace("\"width_um\": 210000", "\"width_um\": -4"),
        canonical.replace("\"width_um\": 210000", "\"width_um\": 1.5"),
        canonical.replace("\"width_um\": 210000", "\"width_um\": 5"),
        canonical
            .replace("\"line_color\": \"#C5D4E7\"", "\"line_color\": \"blue\""),
        canonical.replace("\"width_um\"", "\"height_um\""),
        canonical.replace("\"width_um\"", "\"widthx_um\""),
        format!("{canonical}{{}}"),
        "a".repeat(MAX_PROFILE_BYTES + 1),
    ];
    for candidate in invalid {
        assert!(
            decode_profile(candidate.as_bytes()).is_err(),
            "invalid candidate was admitted: {candidate}"
        );
    }
    let duplicate = canonical.replace(
        "\"width_um\": 210000,",
        "\"width_um\": 210000, \"width_um\": 210000,",
    );
    assert!(decode_profile(duplicate.as_bytes()).is_err());
    let escaped_duplicate = canonical.replace(
        "\"width_um\": 210000,",
        "\"width_um\": 210000, \"wid\\u0074h_um\": 210000,",
    );
    assert!(decode_profile(escaped_duplicate.as_bytes()).is_err());
    let excessive_dots = canonical
        .replace("\"style\": \"grid\"", "\"style\": \"dot\"")
        .replace("\"spacing_um\": 5000", "\"spacing_um\": 1000");
    assert!(decode_profile(excessive_dots.as_bytes()).is_err());
}

#[test]
fn paper_profile_must_be_loaded_before_edit_flags() {
    let args = ["--width-mm", "210", "--profile", "not-a-real-file.json"]
        .map(str::to_owned);
    let result = parse_options(&args);
    assert!(result.is_err());
    assert!(
        result
            .err()
            .is_some_and(|error| error.contains("before edits"))
    );
}

#[test]
fn export_only_page_count_may_precede_loaded_paper_profile() {
    use std::fs;
    use std::process;

    let path = env::temp_dir().join(format!(
        "atrament-paper-page-count-order-{}.json", process::id(),
    ));
    let source = Options {
        height_um: 279_400,
        style: Style::Ruled,
        width_um: 215_900,
        ..Options::default()
    };
    fs::write(&path, encode_profile(source))
        .expect("test-owned paper source is saved");
    let name = path.to_str().expect("test-owned path is UTF-8");
    let before = parse_options(
        &["--pages", "2", "--profile", name].map(str::to_owned),
    )
    .expect("export count before profile is valid")
    .expect("not help");
    let after = parse_options(
        &["--profile", name, "--pages", "2"].map(str::to_owned),
    )
    .expect("export count after profile is valid")
    .expect("not help");
    assert_eq!(before.pages, 2);
    assert_eq!(before.options.width_um, 215_900);
    assert_eq!(before.options.height_um, 279_400);
    assert_eq!(before.options.style, Style::Ruled);
    let before_pdf = render_pages(before.options, before.pages)
        .expect("count-first physical PDF export");
    let after_pdf = render_pages(after.options, after.pages)
        .expect("profile-first physical PDF export");
    assert_eq!(before_pdf, after_pdf);
    assert!(String::from_utf8_lossy(&before_pdf).contains("/Count 2"));
    // An actual paper edit still has to follow the loaded profile.
    let invalid = parse_options(
        &["--spacing-mm", "4", "--profile", name].map(str::to_owned),
    )
    .expect_err("edits before profile refuse");
    assert!(invalid.contains("before edits"));
    fs::remove_file(path).expect("test-owned paper source removed");
}

#[test]
fn paper_help_must_be_the_only_argument_before_loading_selected_profiles() {
    assert!(parse_options(&["--help".to_owned()])
        .expect("sole help argument is accepted").is_none());

    // Previously any later --help silently discarded malformed option
    // combinations, even when a selected profile could not be opened.
    for values in [
        vec!["--style", "blank", "--help"],
        vec!["--help", "--style", "grid"],
        vec!["--profile", "missing-paper-profile.json", "--help"],
        vec!["--profile", "missing-paper-profile.json",
            "--pages", "2", "--help"],
    ] {
        let args = values.into_iter().map(str::to_owned).collect::<Vec<_>>();
        let refusal = parse_options(&args)
            .expect_err("help mixed with export arguments must refuse");
        assert!(refusal.contains("--help must be the only argument"),
            "unexpected refusal for {args:?}: {refusal}");
    }
}

#[test]
fn paper_profile_preflights_later_options_before_selected_source_io() {
    // An unavailable or malformed profile must not obscure a later typed
    // argument refusal. All options are parsed in source-free mode first.
    let nonexistent = "paper-preflight-does-not-exist.json";
    for (tail, diagnostic) in [
        (vec!["--line-color", "red"], "line color must be #RRGGBB"),
        (vec!["--spacing-mm", "1.1234"],
            "millimeters require up to three decimal places"),
        (vec!["--pages", "33"], "page count must be an integer"),
        (vec!["--format", "png", "--pages", "2"],
            "--pages is only valid for PDF export"),
        (vec!["--output", "-"], "--output requires one explicit"),
        (vec!["--quality", "preview"],
            "--quality requires --format png"),
        (vec!["--landscape", "--portrait"],
            "choose only one page orientation"),
        (vec!["--paper-size", "invalid"],
            "paper size must be"),
        (vec!["--emit-profile", "--emit-profile"],
            "--emit-profile may appear only once"),
        (vec!["--width-mm"], "missing value for --width-mm"),
    ] {
        let mut args = vec!["--profile".to_owned(), nonexistent.to_owned()];
        args.extend(tail.into_iter().map(str::to_owned));
        let error = parse_options(&args)
            .expect_err("invalid late options refuse before reading source");
        assert!(error.contains(diagnostic), "{args:?}: {error}");
    }
    let error = parse_options(&["--profile", ""].map(str::to_owned))
        .expect_err("empty selected profile path refuses in preflight");
    assert!(error.contains("--profile requires one explicit file path"));
    let error = parse_options(&["--profile", nonexistent, "--help"]
        .map(str::to_owned))
        .expect_err("mixed help refuses before a selected source read");
    assert!(error.contains("--help must be the only argument"));
}

#[test]
fn paper_profile_operand_must_not_swallow_an_option() {
    for operand in ["--help", "--output", "--replace", "--format", "-"] {
        let error = parse_options(
            &["--profile", operand].map(str::to_owned),
        ).expect_err("option-shaped paper path must refuse in preflight");
        assert!(
            error.contains("--profile requires one explicit file path"),
            "{operand}: {error}",
        );
    }
    // Spell a legitimate dash-prefixed basename with an explicit
    // relative directory prefix. It reaches file admission, not the
    // option-as-path refusal.
    let error = parse_options(
        &["--profile", "./-paper-profile-does-not-exist.json"]
            .map(str::to_owned),
    ).expect_err("missing but syntactically valid paper path");
    assert!(error.contains("paper profile file refused"), "{error}");
}

#[cfg(target_os = "linux")]
#[test]
fn paper_profile_preflight_refuses_bad_flags_before_fifo_and_output_commit() {
    use nix::sys::stat::Mode;
    use nix::unistd::mkfifo;
    use std::fs;
    use std::process;

    let dir = env::temp_dir().join(format!(
        "atrament-paper-preflight-{}", process::id(),
    ));
    fs::create_dir_all(&dir).expect("test-owned directory");
    let fifo = dir.join("selected-profile.json");
    mkfifo(&fifo, Mode::S_IRUSR | Mode::S_IWUSR)
        .expect("test-owned selected profile pipe");
    let output = dir.join("approved.pdf");
    let approved = b"previously reviewed paper export";
    fs::write(&output, approved).expect("test-owned approved output");
    let args = ["--profile", fifo.to_str().expect("UTF-8 pipe path"),
        "--output", output.to_str().expect("UTF-8 output path"),
        "--replace", "--dpi", "301", "--format", "png"]
        .map(str::to_owned);
    let error = parse_options(&args)
        .expect_err("late bad DPI must refuse before opening selected FIFO");
    assert!(error.contains("PNG DPI must be between 72 and 300"),
        "unexpected source-first diagnostic: {error}");
    assert_eq!(fs::read(&output).expect("approved output survives"), approved);
    fs::remove_dir_all(dir).expect("remove test-owned preflight fixtures");
}

#[test]
fn paper_profile_file_read_is_bounded_and_refuses_corrupt_disk_data() {
    use std::fs;
    use std::process;

    let path = env::temp_dir().join(format!(
        "atrament-paper-profile-read-{}.json",
        process::id(),
    ));
    let filename = path.to_str().expect("temporary path is UTF-8");
    let reference = encode_profile(Options::default());
    fs::write(&path, &reference).expect("temporary profile fixture is written");
    let loaded =
        read_profile(filename).expect("typed paper profile is accepted");
    assert_eq!(encode_profile(loaded), reference);
    fs::write(&path, vec![b'a'; MAX_PROFILE_BYTES + 1])
        .expect("oversized test payload is written");
    assert!(read_profile(filename).is_err());
    fs::write(&path, b"{\"schema\":\"broken\"}")
        .expect("incomplete schema fixture is written");
    assert!(read_profile(filename).is_err());
    fs::remove_file(&path).expect("test fixture is removed");
    assert!(read_profile(filename).is_err());
    let directory = env::temp_dir();
    let directory_name = directory.to_str().expect("temporary directory UTF-8");
    assert!(read_profile(directory_name).is_err());
}

#[test]
fn standard_paper_presets_keep_exact_dimensions_and_pdf_page_boxes() {
    let presets = [
        ("A3", 297_000, 420_000),
        ("a4", 210_000, 297_000),
        ("a5", 148_000, 210_000),
        ("b5", 176_000, 250_000),
        ("letter", 215_900, 279_400),
        ("legal", 215_900, 355_600),
    ];
    for (name, width, height) in presets {
        let args = ["--paper-size".to_owned(), name.to_owned()];
        let request = parse_options(&args)
            .expect("known paper size")
            .expect("not help");
        assert_eq!(request.options.width_um, width);
        assert_eq!(request.options.height_um, height);
        let pdf = render(request.options).expect("physical paper PDF");
        assert!(pdf.starts_with(b"%PDF-1.4"));
        assert!(
            decode_profile(encode_profile(request.options).as_bytes()).is_ok()
        );
    }
}

#[test]
fn landscape_preset_and_manual_dimensions_are_consistent() {
    let args = ["--paper-size", "a4", "--landscape"].map(str::to_owned);
    let request = parse_options(&args)
        .expect("landscape A4")
        .expect("not help");
    assert_eq!(request.options.width_um, 297_000);
    assert_eq!(request.options.height_um, 210_000);
    let bytes = render(request.options).expect("landscape paper PDF");
    let pdf = String::from_utf8_lossy(&bytes);
    assert!(pdf.contains("/MediaBox [0 0 841.890 595.276]"));
}

#[test]
fn paper_preset_overrides_are_ordered_and_invalid_choices_fail() {
    for args in [
        vec!["--paper-size", "not-a-size"],
        vec!["--width-mm", "200", "--paper-size", "a4"],
        vec!["--paper-size", "a4", "--paper-size", "letter"],
        vec!["--landscape", "--landscape"],
        vec!["--portrait", "--portrait"],
        vec!["--landscape", "--portrait"],
        vec!["--portrait", "--landscape"],
        vec!["--landscape", "--paper-size", "a4"],
    ] {
        let inputs = args.into_iter().map(str::to_owned).collect::<Vec<_>>();
        assert!(parse_options(&inputs).is_err(), "{inputs:?}");
    }
    let args = ["--paper-size", "a5", "--width-mm", "150"].map(str::to_owned);
    let request = parse_options(&args)
        .expect("explicit width can override preset")
        .expect("not help");
    assert_eq!(request.options.width_um, 150_000);
    assert_eq!(request.options.height_um, 210_000);
}

#[test]
fn loaded_landscape_profile_does_not_flip_again_and_can_return_to_portrait() {
    use std::fs;
    use std::process;

    let path = env::temp_dir()
        .join(format!("atrament-paper-orientation-{}.json", process::id()));
    let name = path.to_str().expect("temporary path is UTF-8");
    let start = parse_options(
        &["--paper-size", "a4", "--landscape"].map(str::to_owned),
    )
    .expect("landscape A4")
    .expect("not help");
    fs::write(&path, encode_profile(start.options))
        .expect("landscape fixture written");
    let landscape =
        parse_options(&["--profile", name, "--landscape"].map(str::to_owned))
            .expect("idempotent landscape")
            .expect("not help");
    assert_eq!(landscape.options.width_um, 297_000);
    assert_eq!(landscape.options.height_um, 210_000);
    assert_eq!(render(landscape.options), render(start.options));
    let portrait =
        parse_options(&["--profile", name, "--portrait"].map(str::to_owned))
            .expect("portrait conversion")
            .expect("not help");
    assert_eq!(portrait.options.width_um, 210_000);
    assert_eq!(portrait.options.height_um, 297_000);
    assert!(render(portrait.options).is_ok());
    fs::remove_file(path).expect("temporary orientation fixture removed");
}

#[test]
fn multi_page_paper_pdf_preserves_count_order_and_dimensions() {
    let args = ["--paper-size", "letter", "--pages", "3"].map(str::to_owned);
    let request = parse_options(&args)
        .expect("three-page paper export")
        .expect("not help");
    assert_eq!(request.pages, 3);
    let bytes = render_pages(request.options, request.pages)
        .expect("all three physical pages are rendered");
    let pdf = String::from_utf8_lossy(&bytes);
    assert!(pdf.contains("/Count 3"));
    assert_eq!(pdf.matches("/MediaBox [0 0 612.000 792.000]").count(), 3);
    assert_eq!(
        bytes,
        render_pages(request.options, request.pages)
            .expect("repeated multi-page output is deterministic")
    );
}

#[test]
fn multi_page_rejects_zero_excess_duplicate_and_profile_only_pages() {
    for args in [
        vec!["--pages", "0"],
        vec!["--pages", "33"],
        vec!["--pages", "-1"],
        vec!["--pages", "1.5"],
        vec!["--pages", "abc"],
        vec!["--pages", "2", "--pages", "3"],
        vec!["--pages", "4", "--emit-profile"],
    ] {
        let inputs = args.into_iter().map(str::to_owned).collect::<Vec<_>>();
        assert!(parse_options(&inputs).is_err(), "{inputs:?}");
    }
    assert!(render_pages(Options::default(), 0).is_err());
    assert!(render_pages(Options::default(), MAX_PAGES + 1).is_err());
    let boundary = render_pages(
        Options {
            style: Style::Blank,
            ..Options::default()
        },
        MAX_PAGES,
    )
    .expect("largest allowed sheet collection renders");
    assert!(String::from_utf8_lossy(&boundary).contains("/Count 32"));
}

#[test]
fn selected_output_parsing_requires_explicit_path_and_overwrite_intent() {
    for inputs in [
        vec!["--output"],
        vec!["--output", "--style"],
        vec!["--output", "first", "--output", "second"],
        vec!["--replace"],
        vec!["--replace", "--replace", "--output", "first"],
    ] {
        let args = inputs.into_iter().map(str::to_owned).collect::<Vec<_>>();
        assert!(parse_options(&args).is_err(), "{args:?}");
    }
    let args = ["--output", "paper.json", "--emit-profile"]
        .map(str::to_owned);
    let request = parse_options(&args)
        .expect("save one profile")
        .expect("not help");
    assert_eq!(request.output.as_deref(), Some("paper.json"));
    assert!(!request.replace);
}

#[cfg(target_os = "linux")]
#[test]
fn explicit_profile_save_round_trips_and_requires_replace_to_overwrite() {
    use std::fs;
    use std::process;

    let dir = env::temp_dir().join(format!(
        "atrament-paper-cli-profile-save-{}",
        process::id()
    ));
    fs::create_dir_all(&dir).expect("test owns a new directory");
    let output = dir.join("paper.json");
    let filename = output.to_str().expect("fixture path is UTF-8");
    let request = parse_options(
        &["--emit-profile", "--style", "ruled", "--output", filename]
            .map(str::to_owned),
    )
    .expect("valid saved paper profile")
    .expect("not help");
    assert_eq!(execute(request.clone()).expect("save paper profile"), None);
    let initial = fs::read(&output).expect("read committed paper profile");
    let loaded = read_profile(filename).expect("reload saved profile");
    assert_eq!(style_name(loaded.style), "ruled");
    assert!(execute(request).is_err(), "no implicit overwrite");
    assert_eq!(fs::read(&output).expect("original remains"), initial);

    let edited = parse_options(
        &["--profile", filename, "--style", "grid", "--output",
          filename, "--replace", "--emit-profile"].map(str::to_owned),
    )
    .expect("editable loaded profile")
    .expect("not help");
    assert_eq!(execute(edited).expect("atomic replace"), None);
    let edited_profile = read_profile(filename).expect("reload edited profile");
    assert_eq!(style_name(edited_profile.style), "grid");
    assert_ne!(fs::read(&output).expect("edited file"), initial);
    fs::remove_dir_all(&dir).expect("remove owned directory");
}

#[cfg(target_os = "linux")]
#[test]
fn selected_pdf_refuses_invalid_edition_and_existing_symlink() {
    use std::fs;
    use std::os::unix::fs::symlink;
    use std::process;

    let dir = env::temp_dir().join(format!(
        "atrament-paper-cli-pdf-save-{}",
        process::id()
    ));
    fs::create_dir_all(&dir).expect("test owns a new directory");
    let output = dir.join("printed.pdf");
    let filename = output.to_str().expect("fixture path is UTF-8");
    let request = parse_options(
        &["--pages", "2", "--output", filename].map(str::to_owned),
    )
    .expect("valid selected PDF")
    .expect("not help");
    assert_eq!(execute(request).expect("PDF file committed"), None);
    let pdf = fs::read(&output).expect("PDF bytes on disk");
    assert!(pdf.starts_with(b"%PDF-1.4"));
    assert!(String::from_utf8_lossy(&pdf).contains("/Count 2"));

    let invalid = parse_options(
        &["--style", "dot", "--spacing-mm", "1", "--output",
          filename, "--replace"].map(str::to_owned),
    )
    .expect("parsed invalid geometry")
    .expect("not help");
    assert!(execute(invalid).is_err());
    assert_eq!(fs::read(&output).expect("PDF preserved"), pdf);

    let alias = dir.join("alias.pdf");
    symlink(&output, &alias).expect("create symlink fixture");
    let alias_path = alias.to_str().expect("fixture path is UTF-8");
    let replace_alias = parse_options(
        &["--output", alias_path, "--replace"].map(str::to_owned),
    )
    .expect("explicit symlink target")
    .expect("not help");
    assert!(execute(replace_alias).is_err(), "symlink replacement refused");
    assert_eq!(fs::read(&output).expect("PDF preserved"), pdf);
    fs::remove_dir_all(&dir).expect("remove owned directory");
}

#[cfg(target_os = "linux")]
#[test]
fn paper_cli_refuses_linked_source_and_parent_traversal_before_export() {
    use std::fs;
    use std::os::unix::fs::symlink;
    use std::process;

    let dir = env::temp_dir().join(format!(
        "atrament-paper-source-links-{}", process::id(),
    ));
    fs::create_dir_all(&dir).expect("test-owned directory");
    let source_dir = dir.join("real");
    fs::create_dir_all(&source_dir).expect("test-owned real directory");
    let source = source_dir.join("paper.json");
    let options = Options {
        style: Style::Ruled,
        ..Options::default()
    };
    let canonical = encode_profile(options);
    fs::write(&source, &canonical).expect("canonical paper input");
    let name = source.to_str().expect("source path");
    assert_eq!(encode_profile(read_profile(name).expect("real file")),
        canonical);
    for (flags, expected) in [
        (vec!["--profile", name], OutputFormat::Pdf),
        (vec!["--profile", name, "--format", "png", "--dpi", "72"],
            OutputFormat::Png),
    ] {
        let request = parse_options(&flags.iter().map(ToString::to_string)
            .collect::<Vec<_>>()).expect("real source options")
            .expect("not help");
        let actual = execute(request).expect("real native export")
            .expect("in-memory output");
        let reference = match expected {
            OutputFormat::Pdf => render_pages(options, 1),
            OutputFormat::Png => render_png(options, 72),
        }.expect("independent page renderer");
        assert_eq!(actual, reference, "source preserves PDF/PNG output");
    }

    let leaf_link = dir.join("leaf.json");
    let parent_link = dir.join("parent-link");
    symlink(&source, &leaf_link).expect("test-owned linked leaf");
    symlink(&source_dir, &parent_link).expect("test-owned linked parent");
    let destination = dir.join("accepted.pdf");
    let sentinel = b"previous accepted printable PDF";
    fs::write(&destination, sentinel).expect("accepted target sentinel");
    let refused = [
        leaf_link,
        parent_link.join("paper.json"),
        dir.join("real/../real/paper.json"),
    ];
    for (index, path) in refused.iter().enumerate() {
        let input = path.to_str().expect("selected file path");
        let error = read_profile(input).expect_err("unsafe input refuses");
        if index == 2 {
            assert!(error.contains("ParentTraversal"), "{error}");
        } else {
            assert!(error.contains("SymlinkForbidden"), "{error}");
        }
        let args = ["--profile", input, "--output",
            destination.to_str().expect("destination path"), "--replace"];
        assert!(parse_options(&args.map(str::to_owned)).is_err(),
            "unsafe source cannot reach render or file commit");
        assert_eq!(fs::read(&destination).expect("existing target"), sentinel);
        assert_eq!(fs::read(&source).expect("original source"),
            canonical.as_bytes());
    }
    fs::remove_dir_all(&dir).expect("remove owned source fixtures");
}

#[cfg(unix)]
#[test]
fn paper_profile_named_pipe_refuses_without_blocking() {
    use nix::sys::stat::Mode;
    use nix::unistd::mkfifo;
    use std::fs;
    use std::process;

    let path = env::temp_dir().join(format!(
        "atrament-paper-profile-fifo-{}", process::id(),
    ));
    mkfifo(&path, Mode::S_IRUSR | Mode::S_IWUSR)
        .expect("create a test-owned named pipe");
    let name = path.to_str().expect("UTF-8 pipe path");
    let error = read_profile(name)
        .expect_err("nonregular profile path must refuse before open");
    assert!(error.contains("NotRegularFile"));
    fs::remove_file(&path).expect("remove test-owned pipe");
}

// The encoder deliberately uses zlib stored blocks. Recover actual RGBA rows
// instead of accepting an IHDR alone: a valid header is not a rendered page.
fn stored_png_pixels(bytes: &[u8]) -> (usize, usize, Vec<u8>) {
    let width = u32::from_be_bytes(
        bytes.get(16..20).expect("PNG has IHDR width")
            .try_into().expect("width has four bytes"),
    );
    let height = u32::from_be_bytes(
        bytes.get(20..24).expect("PNG has IHDR height")
            .try_into().expect("height has four bytes"),
    );
    let chunk = bytes.windows(4).position(|window| window == b"IDAT")
        .expect("PNG contains IDAT");
    let idat_length = u32::from_be_bytes(
        bytes.get(chunk.saturating_sub(4)..chunk).expect("IDAT length exists")
            .try_into().expect("IDAT length has four bytes"),
    );
    let begin = chunk.saturating_add(4);
    let compressed = bytes.get(
        begin..begin.saturating_add(
            usize::try_from(idat_length).expect("IDAT length fits memory"),
        ),
    ).expect("IDAT bytes exist");
    assert!(compressed.starts_with(&[0x78, 0x01]));
    let mut raw = Vec::new();
    let mut cursor = 2;
    loop {
        let control = *compressed.get(cursor).expect("DEFLATE header exists");
        assert_eq!(control & 0xfe, 0, "only stored DEFLATE is admitted");
        let len = u16::from_le_bytes(
            compressed.get(cursor.saturating_add(1)..cursor.saturating_add(3))
                .expect("block length exists").try_into()
                .expect("block length has two bytes"),
        );
        let inverse = u16::from_le_bytes(
            compressed.get(cursor.saturating_add(3)..cursor.saturating_add(5))
                .expect("block length complement exists").try_into()
                .expect("block complement has two bytes"),
        );
        assert_eq!(inverse, !len);
        cursor = cursor.saturating_add(5);
        let end = cursor.saturating_add(usize::from(len));
        raw.extend_from_slice(
            compressed.get(cursor..end).expect("stored block is complete"),
        );
        cursor = end;
        if control & 1 != 0 {
            break;
        }
    }
    assert_eq!(
        compressed.len(), cursor.saturating_add(4),
        "zlib Adler32 follows data",
    );
    (
        usize::try_from(width).expect("width fits memory"),
        usize::try_from(height).expect("height fits memory"),
        raw,
    )
}

// Independently validate PNG chunk CRCs and the RFC 1950 Adler-32 checksum.
fn assert_png_checksums(bytes: &[u8]) {
    assert!(bytes.starts_with(b"\x89PNG\r\n\x1a\n"));
    let mut offset = 8usize;
    let mut chunk_types = Vec::new();
    let mut raw = Vec::new();
    while offset < bytes.len() {
        let length = u32::from_be_bytes(
            bytes.get(offset..offset + 4).expect("PNG chunk length")
                .try_into().expect("four bytes"),
        );
        let length = usize::try_from(length).expect("bounded PNG chunk");
        let tag = bytes.get(offset + 4..offset + 8)
            .expect("PNG chunk tag");
        let end = offset.checked_add(8 + length).expect("chunk limit");
        let content = bytes.get(offset + 4..end)
            .expect("complete chunk data");
        let recorded_crc = u32::from_be_bytes(
            bytes.get(end..end + 4).expect("PNG chunk CRC")
                .try_into().expect("four bytes"),
        );
        let mut crc = !0u32;
        for byte in content {
            crc ^= u32::from(*byte);
            for _ in 0..8 {
                crc = (crc >> 1) ^ (0xedb8_8320u32 &
                    0u32.wrapping_sub(crc & 1));
            }
        }
        assert_eq!(!crc, recorded_crc, "PNG {:?} CRC", tag);
        chunk_types.push(tag.to_vec());
        if tag == b"IDAT" {
            let stream = bytes.get(offset + 8..end)
                .expect("complete zlib stream");
            assert_eq!(stream.get(..2), Some(&[0x78, 0x01][..]));
            let mut position = 2usize;
            loop {
                let header = *stream.get(position)
                    .expect("stored DEFLATE block header");
                assert_eq!(header & !1u8, 0, "stored block only");
                let length = u16::from_le_bytes(
                    stream.get(position + 1..position + 3)
                        .expect("block length").try_into()
                        .expect("two bytes"),
                );
                let complement = u16::from_le_bytes(
                    stream.get(position + 3..position + 5)
                        .expect("block complement").try_into()
                        .expect("two bytes"),
                );
                assert_eq!(complement, !length, "stored block length");
                position += 5;
                let block_end = position + usize::from(length);
                raw.extend_from_slice(stream.get(position..block_end)
                    .expect("whole stored block"));
                position = block_end;
                if header & 1 != 0 {
                    break;
                }
            }
            let recorded_adler = u32::from_be_bytes(
                stream.get(position..position + 4)
                    .expect("zlib Adler checksum").try_into()
                    .expect("four bytes"),
            );
            assert_eq!(position + 4, stream.len(), "zlib stream boundary");
            let (mut a, mut b) = (1u32, 0u32);
            for chunk in raw.chunks(2_000) {
                for value in chunk {
                    a += u32::from(*value);
                    b += a;
                }
                a %= 65_521;
                b %= 65_521;
            }
            assert_eq!((b << 16) | a, recorded_adler,
                "independent zlib Adler checksum");
        }
        offset = end + 4;
    }
    assert_eq!(offset, bytes.len(), "no trailing PNG bytes");
    assert_eq!(chunk_types, [b"IHDR", b"pHYs", b"IDAT", b"IEND"]);
    let width = usize::try_from(u32::from_be_bytes(
        bytes[16..20].try_into().expect("width"),
    )).expect("bounded width");
    let height = usize::try_from(u32::from_be_bytes(
        bytes[20..24].try_into().expect("height"),
    )).expect("bounded height");
    assert_eq!(raw.len(), (4 * width + 1) * height,
        "decoded stored bytes match full RGBA scanlines");
}

#[test]
fn png_checksums_and_stored_blocks_survive_physical_resolution_boundaries() {
    // At 72 DPI a 22.5 mm sheet is exactly 64 columns (257 RGBA row
    // bytes). Heights of 254, 255 and 256 rows straddle precisely 65,535
    // stored bytes; the middle case needs exactly one full DEFLATE block.
    for (height_um, expected_bytes) in [
        (89_500, 65_278),
        (89_800, 65_535),
        (90_000, 65_792),
    ] {
        let png = png::render_paper_png(
            22_500, height_um, 72, &[], 0, None,
        ).expect("bounded transparent physical PNG");
        assert_png_checksums(&png);
        assert_eq!(stored_png_pixels(&png).2.len(), expected_bytes);
    }
    for (width, height, dpi) in [
        (148_000, 210_000, 72),
        (148_000, 210_000, 150),
        (148_000, 210_000, 300),
    ] {
        let png = png::render_paper_png(
            width, height, dpi, &[], 0, None,
        ).expect("bounded transparent physical PNG");
        assert_png_checksums(&png);
    }
    let profile = encode_profile(Options {
        style: Style::Grid,
        width_um: 148_000,
        height_um: 210_000,
        ..Options::default()
    });
    let pen = NotebookInkLine {
        page: 1,
        rgb: [0x42, 0x90, 0xff],
        start_um: (22_000, 30_000),
        end_um: (100_000, 120_000),
        width_um: 650,
    };
    let png = paper_profile_png_page_with_ink(
        profile.as_bytes(), 300, true, None, true, 1, 1_500,
        None, &[pen],
    ).expect("full paper, border and ink PNG");
    assert_png_checksums(&png);
}

#[test]
fn rounded_notebook_border_matches_pdf_and_transparent_png() {
    let paper = default_paper_profile_json();
    let pdf = paper_profile_pdf_pages_with_border(
        paper.as_bytes(), 2, false, None, false, 5_000,
    ).expect("two outlined pages");
    let pdf_text = String::from_utf8_lossy(&pdf);
    assert_eq!(pdf_text.matches(" c\n").count(), 8);
    assert_eq!(pdf_text.matches(" m\n").count(), 2);
    assert_ne!(
        pdf,
        paper_profile_pdf_pages(paper.as_bytes(), 2, false)
            .expect("borderless PDF"),
    );
    let outlined = paper_profile_png_page_with_border(
        paper.as_bytes(), 72, false, None, false, 1, 5_000,
    ).expect("transparent rounded outline");
    let (width, _, raw) = stored_png_pixels(&outlined);
    let alpha = |x: usize, y: usize| -> u8 {
        let offset = y.checked_mul(width * 4 + 1)
            .and_then(|index| index.checked_add(x * 4 + 4))
            .expect("bounded RGBA offset");
        *raw.get(offset).expect("pixel is in raster")
    };
    assert_eq!(alpha(57, 62), 0, "corner is truly rounded");
    assert_eq!(alpha(85, 62), 255, "top straight side exists");
    assert_eq!(alpha(56, 85), 255, "left straight side exists");
    assert_eq!(alpha(85, 85), 0, "unpainted inside stays transparent");
    assert_eq!(
        paper_profile_png_page_with_border(
            paper.as_bytes(), 72, false, None, false, 1, 0,
        ).expect("no border"),
        paper_profile_png_layer(paper.as_bytes(), 72, false)
            .expect("legacy borderless output"),
    );
    assert_eq!(
        paper_profile_pdf_pages_with_border(
            paper.as_bytes(), 1, false, None, false, 0,
        ).expect("no border"),
        paper_profile_pdf_pages(paper.as_bytes(), 1, false)
            .expect("legacy borderless output"),
    );
}

// The candidate is independently replayable through `atrament-profiles`
// with A5 blank paper, a 3000-um border and 300-DPI transparent output.
// Pin the physical high-resolution contour and its encoded bytes.
#[test]
fn a5_high_dpi_rounded_border_keeps_candidate_png_bytes() {
    use atrament_profile_entry_sha256::sha256_profile_bytes;

    let profile = encode_profile(Options {
        style: Style::Blank,
        width_um: 148_000,
        height_um: 210_000,
        ..Options::default()
    });
    let png = paper_profile_png_page_with_border(
        profile.as_bytes(), 300, true, None, false, 1, 3_000,
    ).expect("physical A5 border");
    assert_eq!(sha256_profile_bytes(&png).bytes(), [
        0x8a, 0x93, 0x33, 0x07, 0x7f, 0x41, 0xcc, 0x59,
        0x8c, 0xa8, 0x5e, 0x86, 0x90, 0xc7, 0x1a, 0x27,
        0x69, 0x2f, 0x33, 0x55, 0x6b, 0x92, 0xa8, 0x6e,
        0xf9, 0xfe, 0x9a, 0x3d, 0x3b, 0x4f, 0x69, 0x92,
    ], "high-DPI border encoded PNG candidate changed");
    let (width, height, raw) = stored_png_pixels(&png);
    assert_eq!((width, height), (1_749, 2_481));
    let alpha = |x: usize, y: usize| -> u8 {
        let offset = y.checked_mul(width * 4 + 1)
            .and_then(|row| row.checked_add(x * 4 + 4))
            .expect("valid candidate pixel");
        *raw.get(offset).expect("pixel in decoded sheet")
    };
    assert_eq!(alpha(236, 260), 0, "arc clears top-left corner");
    assert_eq!(alpha(590, 260), 255, "top border remains visible");
    assert_eq!(alpha(236, 1180), 255, "left border remains visible");
    assert_eq!(alpha(590, 1180), 0, "page interior stays transparent");
}

// These hashes were captured from the unoptimized native CLI, before
// constraining rounded-border raster iteration. They cover small and large
// radii, different physical sheets and all three practical DPI settings.
#[test]
fn rounded_border_raster_window_preserves_existing_png_bytes() {
    use atrament_profile_entry_sha256::sha256_profile_bytes;
    use std::fmt::Write as _;

    let cases = [
        (148_000, 210_000, 72, 25_000,
            "3761e464832b18cd8f04dfa871d94e94be1ddd5d916460fe2c6dad46317b6cb8"),
        (148_000, 210_000, 150, 100,
            "905412354e4a2df95bbcdcbd976e0561ba972d90ca2e94cee61d42b2c617389d"),
        (148_000, 210_000, 300, 30_000,
            "f43ad824eadbdac4a991812196ed31325895127687228a9052f065b4878cd446"),
        (215_900, 279_400, 150, 12_000,
            "c9330f1b664c28e82ad3ab62eef674b4848b3b9799244aaa3438e5dbdb1c9d2d"),
    ];
    for (width_um, height_um, dpi, radius_um, expected) in cases {
        let profile = encode_profile(Options {
            style: Style::Blank,
            width_um,
            height_um,
            ..Options::default()
        });
        let png = paper_profile_png_page_with_border(
            profile.as_bytes(), dpi, false, None, false, 1, radius_um,
        ).expect("admitted native rounded paper PNG");
        let mut digest = String::with_capacity(64);
        for byte in sha256_profile_bytes(&png).bytes() {
            write!(&mut digest, "{byte:02x}").expect("hex digest");
        }
        assert_eq!(
            digest, expected,
            "{width_um}x{height_um}, {dpi} DPI, {radius_um} um",
        );
    }
}

#[test]
fn rounded_border_refuses_unprintable_radius_and_missing_edge_clearance() {
    let paper = default_paper_profile_json();
    assert!(validate_paper_border_clearance(paper.as_bytes(), 100_000)
        .is_err(), "radius larger than half printable height");
    let edge = paper.replace(
        "\"margin_left_um\": 20000", "\"margin_left_um\": 0",
    );
    // The base vector serializer also refuses zero margin when the mark
    // stroke would extend outside the physical page.
    assert!(paper_profile_pdf_pages(edge.as_bytes(), 1, false).is_err());
    assert!(rounded_border(Options {
        margin_left_um: 0,
        ..Options::default()
    }, 500).is_err(), "border must independently enforce edge clearance");
    assert!(validate_paper_border_clearance(edge.as_bytes(), 500)
        .is_err(), "border stroke must stay inside sheet");
    let just_clear = paper.replace(
        "\"margin_left_um\": 20000", "\"margin_left_um\": 60",
    );
    assert!(validate_paper_border_clearance(just_clear.as_bytes(), 500)
        .is_ok(), "60 micrometres are enough for a centered stroke");
    assert!(paper_profile_pdf_pages_with_border(
        edge.as_bytes(), 1, false, None, false, 500,
    ).is_err());
    assert!(paper_profile_png_page_with_border(
        edge.as_bytes(), 72, false, None, false, 1, 500,
    ).is_err());
}

#[test]
fn transparent_png_grid_has_real_ink_and_physical_resolution() {
    let options = Options {
        color: PdfRgb { red: 0x12, green: 0x34, blue: 0x56 },
        style: Style::Grid,
        width_um: 148_000,
        height_um: 210_000,
        ..Options::default()
    };
    let bytes = render_png(options, 150).expect("A5 transparent grid");
    assert!(bytes.starts_with(b"\x89PNG\r\n\x1a\n"));
    assert_eq!(bytes.get(24..26), Some(&[8, 6][..]), "8-bit RGBA");
    let phys = bytes.windows(4).position(|window| window == b"pHYs")
        .expect("physical PNG resolution chunk");
    assert_eq!(bytes.get(phys + 4..phys + 13), Some(&[
        0, 0, 23, 18, 0, 0, 23, 18, 1,
    ][..])); // 5906 pixels/metre is approximately 150 DPI.
    let (width, height, raw) = stored_png_pixels(&bytes);
    assert_eq!((width, height), (875, 1241));
    assert_eq!(raw.len(), height * (width * 4 + 1));
    let mut painted = 0usize;
    let mut transparent = 0usize;
    for row in raw.chunks_exact(width * 4 + 1) {
        assert_eq!(row.first(), Some(&0));
        for pixel in row.get(1..).expect("one filter byte").as_chunks::<4>().0 {
            match pixel {
                [0, 0, 0, 0] => transparent += 1,
                [0x12, 0x34, 0x56, 255] => painted += 1,
                _ => panic!(
                    "PNG paper pixels must be exact ink or transparency"
                ),
            }
        }
    }
    assert!(painted > 5_000, "real grid marks are painted");
    assert!(transparent > painted, "most paper remains transparent");
    assert_eq!(bytes, render_png(options, 150).expect("deterministic PNG"));
}

// Pin independently exported pre-row-span native PNG output at maximum
// admitted A5 DPI. Real grid, ruled and dot styles exercise both stroked
// rectangles and square dots without importing the same raster algorithm.
#[test]
fn a5_high_dpi_paper_marks_keep_previous_encoded_pixels() {
    use atrament_profile_entry_sha256::sha256_profile_bytes;
    use std::fmt::Write as _;

    let cases = [
        (Style::Grid,
            "395ab3f7aca3bf122e216151e904be39f8f1513ff430eb48acd7b9c63e162ecb"),
        (Style::Ruled,
            "7097b55812beebb3231b15744f464e9b1b40d69e215a8dfd18401f2b02c71f7e"),
        (Style::Dot,
            "e474ab1e6f05e54e0e13992eae31161fa5a582d449806b7f400ce1def260051e"),
    ];
    for (style, expected) in cases {
        let png = render_png(Options {
            color: PdfRgb { red: 0xAB, green: 0xCD, blue: 0xEF },
            style,
            width_um: 148_000,
            height_um: 210_000,
            ..Options::default()
        }, 300).expect("300-DPI A5 physical paper");
        let mut digest = String::with_capacity(64);
        for byte in sha256_profile_bytes(&png).bytes() {
            write!(&mut digest, "{byte:02x}").expect("hex digest");
        }
        assert_eq!(digest, expected, "physical paper PNG output changed");
    }
}

// These are deterministic *candidate* image goldens, not an owner's
// visually approved reference. Pin real compressed PNG, decoded RGBA and
// vector PDF separately so encoder and physical ink drift are visible.
#[test]
fn ruled_and_grid_page_candidate_golden_bytes_and_pixels_are_stable() {
    use atrament_profile_entry_sha256::sha256_profile_bytes;
    use std::fs;
    use std::fmt::Write as _;
    use std::path::Path;

    let reference_dir = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../../tests/backend/notebook-paper-cli/composition");
    let hex_digest = |source: &[u8]| -> String {
        let mut hex = String::with_capacity(64);
        for byte in sha256_profile_bytes(source).bytes() {
            write!(&mut hex, "{byte:02x}").expect("String formatting");
        }
        hex
    };
    let cases = [
        (Style::Ruled, "ruled",
            "fa59a88a53ffba9a11b8f7dd1416101bf3cd9b6d63693fed6b5692b5899bad8e",
            "d775db518f8cba371e5af6cf1608aef7ad38c33d1ec8d37ab6bda71700c50aaf",
            "27b9e33ced9c7b89e27a768ea30dc127bc82368ac4535a29856c46418bb81254",
            34_500),
        (Style::Grid, "grid",
            "6bbf7d74ded9f62210b45f04fb1b5c48a51422cc4f315019de1b45c16e10b773",
            "46e2ff82bda2db65a685836a6a3fa96f8b73e883aabc0571e28127308f270631",
            "db38766c7658e9179277585b5e982a61a61660939fc607afe4a83f3bf902c4be",
            66_468),
    ];
    for (style, name, pdf_golden, png_golden, rgba_golden, ink_golden)
        in cases {
        let reference_pdf = fs::read(reference_dir.join(
            format!("a4-{name}-72dpi.pdf"))).expect("PDF candidate on disk");
        let reference_png = fs::read(reference_dir.join(
            format!("a4-{name}-72dpi.png"))).expect("PNG candidate on disk");
        let options = Options { style, ..Options::default() };
        let pdf = render_pages(options, 1).expect("real A4 vector paper");
        assert!(pdf.starts_with(b"%PDF-1.4"));
        assert_eq!(hex_digest(&pdf), pdf_golden, "vector page golden");
        assert!(pdf == reference_pdf,
            "native PDF differs from inspectable vector candidate");
        let png = render_png(options, 72).expect("real transparent paper");
        assert_eq!(hex_digest(&png), png_golden, "encoded PNG golden");
        assert!(png == reference_png,
            "native PNG differs from inspectable raster candidate");
        let (width, height, raw) = stored_png_pixels(&png);
        assert_eq!((width, height), (596, 842), "A4 at 72 DPI");
        let mut rgba = Vec::with_capacity(width * height * 4);
        for row in raw.chunks_exact(width * 4 + 1) {
            assert_eq!(row.first(), Some(&0), "unfiltered PNG scanline");
            rgba.extend_from_slice(row.get(1..).expect("pixel row"));
        }
        assert_eq!(rgba.len(), width * height * 4);
        let painted = rgba.as_chunks::<4>().0.iter()
            .filter(|pixel| **pixel == [197, 212, 231, 255]).count();
        assert_eq!(painted, ink_golden, "exact visible paper pixels");
        assert_eq!(hex_digest(&rgba), rgba_golden, "decoded RGBA golden");
        let changed = render_png(Options {
            color: PdfRgb { red: 123, green: 212, blue: 231 },
            ..options
        }, 72).expect("same geometry in new ink color");
        assert_ne!(hex_digest(&changed), png_golden,
            "ink changes must not falsely match the candidate golden");
    }
}

#[test]
fn transparent_blank_has_no_ink_and_paper_variants_do() {
    let basic = Options {
        width_um: 100_000,
        height_um: 140_000,
        ..Options::default()
    };
    for style in [Style::Blank, Style::Ruled, Style::Grid, Style::Dot] {
        let options = Options { style, ..basic };
        let png = render_png(options, 72).expect("raster of all paper styles");
        let (width, height, raw) = stored_png_pixels(&png);
        let mut ink_count = 0usize;
        for row in raw.chunks_exact(width * 4 + 1).take(height) {
            for pixel in row.get(1..).expect("filter byte").as_chunks::<4>().0 {
                if pixel.get(3) == Some(&255) {
                    ink_count += 1;
                }
            }
        }
        assert_eq!(ink_count == 0, style == Style::Blank);
    }
}

#[test]
fn png_export_flags_and_resource_limits_fail_closed() {
    let invalid = [
        vec!["--format", "tiff"],
        vec!["--format", "png", "--dpi", "71"],
        vec!["--format", "png", "--dpi", "301"],
        vec!["--format", "png", "--dpi", "x"],
        vec!["--dpi", "150"],
        vec!["--format", "png", "--pages", "1"],
        vec!["--format", "png", "--emit-profile"],
        vec!["--format", "png", "--format", "pdf"],
        vec!["--format", "png", "--dpi", "150", "--dpi", "72"],
    ];
    for items in invalid {
        let args = items.into_iter().map(str::to_owned).collect::<Vec<_>>();
        assert!(parse_options(&args).is_err(), "{args:?}");
    }
    let options = Options::default();
    for dpi in [0, 71, 301, u32::MAX] {
        assert!(render_png(options, dpi).is_err());
    }
    let oversized = Options {
        width_um: 500_000,
        height_um: 700_000,
        style: Style::Blank,
        ..options
    };
    assert!(render_png(oversized, 300).is_err(), "bounded 5MP budget");
    let valid = parse_options(&["--format", "png", "--dpi", "150"]
        .map(str::to_owned))
        .expect("PNG options accepted").expect("not help");
    assert_eq!(valid.format, OutputFormat::Png);
    assert_eq!(valid.dpi, 150);
    assert!(execute(valid).expect("PNG on stdout").is_some());
}

#[test]
fn explicit_cpu_png_quality_profiles_preserve_physical_page_geometry() {
    let defaults = Options::default();
    let mut sizes = Vec::new();
    for (quality, dpi) in [("preview", 72), ("final", DEFAULT_PNG_DPI)] {
        let args = ["--format", "png", "--quality", quality]
            .map(str::to_owned);
        let request = parse_options(&args)
            .expect("quality option accepted").expect("not help");
        assert_eq!(request.format, OutputFormat::Png);
        assert_eq!(request.dpi, dpi);
        let png = execute(request).expect("CPU raster accepted")
            .expect("unselected PNG returns bytes");
        assert_eq!(png, render_png(defaults, dpi)
            .expect("same physical paper and raster DPI"));
        assert_eq!(png.get(..8), Some(b"\x89PNG\r\n\x1a\n".as_slice()));
        assert_eq!(png.get(25), Some(&6), "RGBA page always transparent");
        sizes.push(png.len());
    }
    assert!(sizes.first() < sizes.last(),
        "preview produces fewer raster bytes than final");
    let wrong = [
        vec!["--quality", "preview"],
        vec!["--format", "pdf", "--quality", "final"],
        vec!["--format", "png", "--quality", "fast"],
        vec!["--format", "png", "--quality", ""],
        vec!["--format", "png", "--quality", "final",
            "--quality", "preview"],
        vec!["--format", "png", "--quality", "preview",
            "--dpi", "72"],
        vec!["--dpi", "150", "--format", "png",
            "--quality", "final"],
        vec!["--emit-profile", "--quality", "preview"],
    ];
    for tokens in wrong {
        let args = tokens.into_iter().map(str::to_owned)
            .collect::<Vec<_>>();
        assert!(parse_options(&args).is_err(), "{args:?}");
    }
    let oversize = ["--format", "png", "--quality", "final",
        "--width-mm", "500", "--height-mm", "700"]
        .map(str::to_owned);
    let request = parse_options(&oversize)
        .expect("bounded quality request parses").expect("not help");
    assert!(execute(request).is_err(),
        "quality presets never bypass the 5-megapixel budget");
    let explicit = ["--format", "png", "--dpi", "150"]
        .map(str::to_owned);
    let default = ["--format", "png", "--quality", "final"]
        .map(str::to_owned);
    assert_eq!(parse_options(&explicit).expect("dpi valid")
        .expect("not help").dpi,
        parse_options(&default).expect("final valid")
            .expect("not help").dpi);
}

#[test]
fn png_profile_entrypoint_uses_same_canonical_paper_geometry() {
    let options = Options { style: Style::Ruled, ..Options::default() };
    let canonical = encode_profile(options);
    assert_eq!(
        paper_profile_png(canonical.as_bytes(), 72),
        render_png(options, 72),
    );
    let invalid = canonical.replace("atrament.paper.v1", "unknown.version");
    assert!(paper_profile_png(invalid.as_bytes(), 72).is_err());
}

#[cfg(target_os = "linux")]
#[test]
fn selected_png_commit_refuses_unsafe_replace_and_preserves_existing() {
    use std::fs;
    use std::os::unix::fs::symlink;
    use std::process;

    let directory = env::temp_dir().join(format!(
        "atrament-paper-cli-png-save-{}",
        process::id(),
    ));
    fs::create_dir_all(&directory).expect("create test-owned directory");
    let output = directory.join("physical-paper.png");
    let filename = output.to_str().expect("test-owned UTF-8 filename");
    let args = [
        "--format", "png", "--dpi", "72", "--style", "ruled",
        "--output", filename,
    ]
    .map(str::to_owned);
    let request = parse_options(&args)
        .expect("selected PNG options are valid")
        .expect("not help");
    assert_eq!(execute(request.clone()).expect("save new PNG"), None);
    let initial = fs::read(&output).expect("read committed PNG");
    assert!(initial.starts_with(b"\x89PNG\r\n\x1a\n"));
    assert!(execute(request).is_err(), "existing output needs --replace");
    assert_eq!(fs::read(&output).expect("preserved output"), initial);

    let invalid = parse_options(
        &[
            "--format", "png", "--style", "dot", "--spacing-mm", "1",
            "--output", filename, "--replace",
        ]
        .map(str::to_owned),
    )
    .expect("parse invalid geometry")
    .expect("not help");
    assert!(execute(invalid).is_err());
    assert_eq!(fs::read(&output).expect("original preserved"), initial);

    let alias = directory.join("alias.png");
    symlink(&output, &alias).expect("create test-owned symlink");
    let alias_filename = alias.to_str().expect("test-owned UTF-8 alias");
    let alias_request = parse_options(
        &["--format", "png", "--output", alias_filename, "--replace"]
            .map(str::to_owned),
    )
    .expect("alias request parses")
    .expect("not help");
    assert!(execute(alias_request).is_err(), "symlink replacement refused");
    assert_eq!(fs::read(&output).expect("original preserved"), initial);
    fs::remove_dir_all(directory).expect("remove test-owned directory");
}

#[test]
fn date_placeholder_is_physical_and_uses_shared_pdf_png_mark_geometry() {
    let paper = default_paper_profile_json();
    let base = paper_profile_pdf_pages(paper.as_bytes(), 2, false)
        .expect("unmarked paper");
    let left_line = paper_profile_pdf_pages_with_date(
        paper.as_bytes(), 2, false, Some(PaperDatePlaceholder::LeftLine),
    ).expect("left date line");
    let right_line = paper_profile_pdf_pages_with_date(
        paper.as_bytes(), 2, false, Some(PaperDatePlaceholder::RightLine),
    ).expect("right date line");
    let left_box = paper_profile_pdf_pages_with_date(
        paper.as_bytes(), 2, false, Some(PaperDatePlaceholder::LeftBox),
    ).expect("left date box");
    assert!(String::from_utf8_lossy(&base).contains("/Count 2"));
    assert_eq!(String::from_utf8_lossy(&left_line).matches(" m\n").count(), 2);
    assert_eq!(String::from_utf8_lossy(&left_box).matches(" m\n").count(), 8);
    assert_ne!(left_line, right_line, "side changes physical position");
    assert_ne!(left_line, left_box, "box has three more strokes");

    let transparent = paper_profile_png_layer(paper.as_bytes(), 72, false)
        .expect("unmarked PNG");
    let line = paper_profile_png_layer_with_date(
        paper.as_bytes(), 72, false, Some(PaperDatePlaceholder::LeftLine),
    ).expect("line PNG");
    let box_png = paper_profile_png_layer_with_date(
        paper.as_bytes(), 72, false, Some(PaperDatePlaceholder::LeftBox),
    ).expect("box PNG");
    let right_png = paper_profile_png_layer_with_date(
        paper.as_bytes(), 72, false, Some(PaperDatePlaceholder::RightLine),
    ).expect("right PNG");
    assert_ne!(transparent, line);
    assert_ne!(line, box_png);
    assert_ne!(line, right_png);
    let (width, height, raw) = stored_png_pixels(&line);
    assert_eq!((width, height), (596, 842));
    let alpha = |pixels: &[u8], x: usize, y: usize| -> u8 {
        let stride = width.checked_mul(4)
            .and_then(|length| length.checked_add(1))
            .expect("bounded scanline width");
        let index = y.checked_mul(stride)
            .and_then(|offset| x.checked_mul(4)
                .and_then(|column| offset.checked_add(column)))
            .and_then(|offset| offset.checked_add(4))
            .expect("sample lies within the PNG raster");
        *pixels.get(index).expect("pixel alpha channel")
    };
    // A4 left inset 20 mm; the date line is 15 mm from the top edge.
    assert_eq!(alpha(&raw, 71, 42), 255);
    assert_eq!(alpha(&raw, 480, 42), 0);
    let (_, _, box_raw) = stored_png_pixels(&box_png);
    assert_eq!(alpha(&box_raw, 71, 19), 255, "top side of the box");
    assert_eq!(alpha(&raw, 71, 19), 0, "line leaves top clear");
    let (_, _, right_raw) = stored_png_pixels(&right_png);
    assert_eq!(alpha(&right_raw, 480, 42), 255);
}

// Independently derive Gregorian month lengths instead of mirroring the
// date parser's admission expression or inferring a locale for slash labels.
fn reference_gregorian_date(year: u32, month: u32, day: u32) -> bool {
    let leap = year.is_multiple_of(400)
        || (year.is_multiple_of(4) && !year.is_multiple_of(100));
    let days = match month {
        2 if leap => 29,
        2 => 28,
        4 | 6 | 9 | 11 => 30,
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        _ => 0,
    };
    year > 0 && (1..=days).contains(&day)
}

#[test]
fn printed_date_parser_matches_full_century_and_month_boundary_oracle() {
    let check = |year, month, day| {
        let expected = reference_gregorian_date(year, month, day);
        let either_order = expected
            || reference_gregorian_date(year, day, month);
        let iso = format!("{year:04}-{month:02}-{day:02}");
        assert_eq!(
            validate_printed_date_text(&iso).is_ok(),
            expected,
            "ISO Gregorian calendar mismatch: {iso}",
        );
        let slash = format!("{day:02}/{month:02}/{year:04}");
        let dot = format!("{month:02}.{day:02}.{year:04}");
        for label in [slash, dot] {
            assert_eq!(
                validate_printed_date_text(&label).is_ok(),
                either_order,
                "ambiguous-calendar-format mismatch: {label}",
            );
        }
    };
    for year in 0..=9_999 {
        for day in 27..=31 {
            check(year, 2, day);
        }
    }
    for year in [0, 1, 4, 100, 400, 1582, 1700, 1900, 1999, 2000,
        2024, 2026, 2100, 2400, 9999] {
        for month in 0..=13 {
            for day in 0..=32 {
                check(year, month, day);
            }
        }
    }
    assert!(validate_printed_date_text("").is_ok());
    for invalid in [
        "2026-2-3", "2026/11/03", "03/11.2026",
        "\u{663}/\u{661}\u{661}/\u{662}\u{660}\u{662}\u{666}",
    ] {
        assert!(validate_printed_date_text(invalid).is_err(), "{invalid}");
    }
    assert_eq!(
        validate_printed_date_text("2024-2-29").expect_err("short month"),
        "date text must use YYYY-MM-DD, NN/NN/YYYY or NN.NN.YYYY",
    );
}

#[test]
fn printed_date_stays_consistent_across_vector_and_raster_exports() {
    let paper = default_paper_profile_json();
    let date = Some(PaperDatePlaceholder::RightBox);
    let plain_pdf = paper_profile_pdf_pages_with_printed_date(
        paper.as_bytes(), 1, false, date, false, 0, None,
    ).expect("blank date box PDF");
    let plain_png = paper_profile_png_page_with_printed_date(
        paper.as_bytes(), 72, false, date, false, 1, 0, None,
    ).expect("blank date box PNG");
    for label in ["2000-02-29", "29/02/2000", "02.29.2000"] {
        let pdf = paper_profile_pdf_pages_with_printed_date(
            paper.as_bytes(), 1, false, date, false, 0, Some(label),
        ).expect("Gregorian date PDF");
        let png = paper_profile_png_page_with_printed_date(
            paper.as_bytes(), 72, false, date, false, 1, 0, Some(label),
        ).expect("Gregorian date PNG");
        assert_ne!(plain_pdf, pdf, "date must add printed PDF geometry");
        assert_ne!(plain_png, png, "date must add printed PNG pixels");
    }
    for invalid in ["1900-02-29", "29/02/1900", "02.29.1900"] {
        assert!(paper_profile_pdf_pages_with_printed_date(
            paper.as_bytes(), 1, false, date, false, 0, Some(invalid),
        ).is_err(), "invalid printed PDF date {invalid}");
        assert!(paper_profile_png_page_with_printed_date(
            paper.as_bytes(), 72, false, date, false, 1, 0,
            Some(invalid),
        ).is_err(), "invalid printed PNG date {invalid}");
    }
}

#[test]
fn date_placeholder_refuses_without_enough_unruled_margin() {
    let paper = default_paper_profile_json();
    let insufficient_top = paper.replace(
        "\"margin_top_um\": 22000", "\"margin_top_um\": 9000",
    );
    let insufficient_width = paper.replace(
        "\"margin_left_um\": 20000", "\"margin_left_um\": 180000",
    );
    for invalid in [&insufficient_top, &insufficient_width] {
        assert!(paper_profile_pdf_pages_with_date(
            invalid.as_bytes(), 1, false,
            Some(PaperDatePlaceholder::RightBox),
        ).is_err());
        assert!(paper_profile_png_layer_with_date(
            invalid.as_bytes(), 72, false,
            Some(PaperDatePlaceholder::LeftLine),
        ).is_err());
        assert!(paper_profile_pdf_pages(invalid.as_bytes(), 1, false).is_ok());
    }
}

#[test]
fn nested_notebook_paper_layer_export_uses_one_rust_renderer() {
    let grid = default_paper_profile_json();
    let blank = grid.replace("\"style\": \"grid\"", "\"style\": \"blank\"");
    let marked = paper_profile_pdf_pages(grid.as_bytes(), 3, true)
        .expect("three physical paper pages");
    let unmarked = paper_profile_pdf_pages(grid.as_bytes(), 3, false)
        .expect("three preprinted pages");
    let blank_pdf = paper_profile_pdf_pages(blank.as_bytes(), 3, true)
        .expect("unmarked physical pages");
    assert!(String::from_utf8_lossy(&marked).contains("/Count 3"));
    assert_eq!(unmarked, blank_pdf, "preprinted pages skip ruling");
    assert_ne!(marked, unmarked, "plain sheets still print grid marks");
    assert_eq!(
        paper_profile_pdf(grid.as_bytes()).expect("existing PDF API"),
        paper_profile_pdf_pages(grid.as_bytes(), 1, true)
            .expect("one-page PDF API"),
    );
    for count in [0, 33, usize::MAX] {
        assert!(
            paper_profile_pdf_pages(grid.as_bytes(), count, false).is_err()
        );
    }

    let transparent = paper_profile_png_layer(grid.as_bytes(), 72, false)
        .expect("preprinted transparent PNG");
    assert_eq!(
        transparent,
        paper_profile_png(blank.as_bytes(), 72)
            .expect("blank transparent PNG"),
    );
    let (width, height, raw) = stored_png_pixels(&transparent);
    assert_eq!((width, height), (596, 842));
    for row in raw.chunks_exact(width * 4 + 1).take(height) {
        for pixel in row.get(1..).expect("filter byte").as_chunks::<4>().0 {
            assert_eq!(pixel[3], 0, "preprinted PNG has no paper ink");
        }
    }
    assert_ne!(
        paper_profile_png_layer(grid.as_bytes(), 72, true)
            .expect("marked PNG"),
        transparent,
    );
    for dpi in [0, 71, 301] {
        assert!(paper_profile_png_layer(grid.as_bytes(), dpi, false).is_err());
    }
}

#[test]
fn manual_diagonal_pen_ink_is_vector_pdf_and_visible_transparent_png() {
    let paper = default_paper_profile_json();
    let ink = [NotebookInkLine {
        page: 2,
        start_um: (25_400, 25_400),
        end_um: (50_800, 50_800),
        width_um: 700,
        rgb: [0xde, 0x11, 0x44],
    }];
    let baseline = paper_profile_pdf_pages(paper.as_bytes(), 2, false)
        .expect("two blank PDF sheets");
    let pdf = paper_profile_pdf_pages_with_ink(
        paper.as_bytes(), 2, false, None, false, 0, None, &ink,
    ).expect("manual line projects as native PDF ink");
    assert_ne!(pdf, baseline);
    let text = String::from_utf8_lossy(&pdf);
    assert!(text.contains("/Count 2"));
    assert_eq!(text.matches(" m\n").count(), 1,
        "only page 2 includes one explicit diagonal path");
    assert!(text.contains("0.871 0.067 0.267 RG"));
    let png = paper_profile_png_page_with_ink(
        paper.as_bytes(), 72, false, None, false, 2, 0, None, &ink,
    ).expect("same line projects to PNG RGBA pixels");
    let (width, _height, raw) = stored_png_pixels(&png);
    let pixel = |x: usize, y: usize| -> &[u8] {
        let index = y * (width * 4 + 1) + 1 + x * 4;
        raw.get(index..index + 4).expect("physical sample within raster")
    };
    assert_eq!(pixel(100, 100), &[0xde, 0x11, 0x44, 255]);
    assert_eq!(pixel(100, 120), &[0, 0, 0, 0],
        "no rectangular bounding box or opaque backdrop");
    assert_eq!(
        paper_profile_png_page_with_ink(
            paper.as_bytes(), 72, false, None, false, 1, 0, None, &ink,
        ).expect("first selected page has no ink"),
        paper_profile_png_layer(paper.as_bytes(), 72, false)
            .expect("legacy transparent first page"),
    );
}

#[test]
fn authored_horizontal_and_vertical_ink_use_round_png_pen_caps() {
    let paper = default_paper_profile_json();
    let horizontal = NotebookInkLine {
        page: 1,
        start_um: (25_400, 25_400),
        end_um: (50_800, 25_400),
        width_um: 1_500,
        rgb: [0x42, 0x17, 0xc9],
    };
    let vertical = NotebookInkLine {
        start_um: (76_200, 25_400),
        end_um: (76_200, 50_800),
        ..horizontal
    };
    let png = paper_profile_png_page_with_ink(
        paper.as_bytes(), 150, false, None, false, 1, 0, None,
        &[horizontal, vertical],
    ).expect("axis-aligned authored pen paths rasterize");
    let (width, _, raw) = stored_png_pixels(&png);
    let pixel = |x: usize, y: usize| -> &[u8] {
        let index = y * (width * 4 + 1) + 1 + x * 4;
        raw.get(index..index + 4).expect("sample in physical page")
    };
    // Corners of a square-capped rectangular bounding box must be clear.
    // Both strokes start at exactly 150 px and share PDF round-cap geometry.
    assert_eq!(pixel(145, 145), &[0, 0, 0, 0]);
    assert_eq!(pixel(445, 145), &[0, 0, 0, 0]);
    assert_eq!(pixel(145, 150), &[0x42, 0x17, 0xc9, 255]);
    assert_eq!(pixel(450, 145), &[0x42, 0x17, 0xc9, 255]);
    assert_eq!(pixel(300, 150), &[0x42, 0x17, 0xc9, 255]);
    assert_eq!(pixel(450, 300), &[0x42, 0x17, 0xc9, 255]);
}

#[test]
fn five_megapixel_png_boundary_keeps_authored_ink_near_last_margin() {
    // Width and height round up to exactly 2000 x 2500 pixels at 300 DPI.
    // Adding one micrometre to either dimension adds an entire pixel row or
    // column and must not bypass the inclusive five-million-pixel ceiling.
    let page = Options {
        style: Style::Blank,
        width_um: 169_333,
        height_um: 211_666,
        ..Options::default()
    };
    let ink = [NotebookInkLine {
        page: 1,
        start_um: (151_000, 194_000),
        end_um: (154_000, 194_000),
        width_um: 700,
        rgb: [0x7a, 0x26, 0xd4],
    }];
    let selected = encode_profile(page);
    let rendered = paper_profile_png_page_with_ink(
        selected.as_bytes(), 300, false, None, false, 1, 0,
        None, &ink,
    ).expect("five million RGBA pixels with near-edge pen ink");
    let (width, height, raw) = stored_png_pixels(&rendered);
    assert_eq!((width, height), (2_000, 2_500));
    let pixel = |x: usize, y: usize| -> &[u8] {
        let index = y * (width * 4 + 1) + 1 + x * 4;
        raw.get(index..index + 4).expect("5MP raster sample")
    };
    assert_eq!(pixel(1_800, 2_291), &[0x7a, 0x26, 0xd4, 255],
        "last printable-area pen stroke remains visible");
    assert_eq!(pixel(1_999, 2_499), &[0, 0, 0, 0],
        "last physical pixel remains transparent outside the margin");
    assert_eq!(pixel(0, 0), &[0, 0, 0, 0]);
    for oversized in [
        Options { width_um: 169_334, ..page },
        Options { height_um: 211_667, ..page },
    ] {
        let refused = paper_profile_png_page_with_ink(
            encode_profile(oversized).as_bytes(), 300, false, None,
            false, 1, 0, None, &ink,
        ).expect_err("one extra output row or column exceeds 5MP");
        assert!(refused.contains("PNG pixel budget"), "{refused}");
    }
}

#[test]
fn maximum_dpi_a5_png_preserves_round_ink_caps_and_transparent_diagonals() {
    let paper = encode_profile(Options {
        style: Style::Blank,
        width_um: 148_000,
        height_um: 210_000,
        ..Options::default()
    });
    let horizontal = NotebookInkLine {
        page: 1,
        start_um: (25_400, 25_400),
        end_um: (50_800, 25_400),
        width_um: 1_500,
        rgb: [0x42, 0x17, 0xc9],
    };
    let diagonal = NotebookInkLine {
        start_um: (76_200, 25_400),
        end_um: (101_600, 50_800),
        width_um: 700,
        rgb: [0xde, 0x11, 0x44],
        ..horizontal
    };
    let png = paper_profile_png_page_with_ink(
        paper.as_bytes(), 300, false, None, false, 1, 0,
        None, &[horizontal, diagonal],
    ).expect("two physical 300-DPI A5 pen paths");
    let (width, height, raw) = stored_png_pixels(&png);
    assert_eq!((width, height), (1_749, 2_481));
    let pixel = |x: usize, y: usize| -> &[u8] {
        let index = y * (width * 4 + 1) + 1 + x * 4;
        raw.get(index..index + 4).expect("A5 raster sample")
    };
    assert_eq!(pixel(290, 300), &[0, 0, 0, 0],
        "rounded cap has no opaque enclosing square");
    assert_eq!(pixel(291, 300), &[0x42, 0x17, 0xc9, 255],
        "round cap includes the near-edge horizontal pixel");
    assert_eq!(pixel(291, 291), &[0, 0, 0, 0],
        "horizontal cap corner remains clear");
    assert_eq!(pixel(450, 300), &[0x42, 0x17, 0xc9, 255]);
    assert_eq!(pixel(1_050, 450), &[0xde, 0x11, 0x44, 255]);
    assert_eq!(pixel(1_020, 480), &[0, 0, 0, 0],
        "diagonal does not paint its axis-aligned bounding box");
    assert_eq!(pixel(0, 0), &[0, 0, 0, 0]);
}

#[test]
fn maximum_dpi_a5_png_keeps_ink_over_the_rounded_paper_border() {
    let paper = encode_profile(Options {
        style: Style::Blank,
        width_um: 148_000,
        height_um: 210_000,
        ..Options::default()
    });
    let ink = [NotebookInkLine {
        page: 1,
        start_um: (70_000, 22_750),
        end_um: (100_000, 22_750),
        width_um: 1_500,
        rgb: [0xdf, 0x26, 0x53],
    }];
    let bordered = paper_profile_png_page_with_ink(
        paper.as_bytes(), 300, false, None, false, 1, 5_000,
        None, &[],
    ).expect("border without any pen ink");
    let overlaid = paper_profile_png_page_with_ink(
        paper.as_bytes(), 300, false, None, false, 1, 5_000,
        None, &ink,
    ).expect("ink above rounded A5 border");
    let (width, height, bare) = stored_png_pixels(&bordered);
    let (overlaid_width, overlaid_height, raw) = stored_png_pixels(&overlaid);
    assert_eq!((width, height), (1_749, 2_481));
    assert_eq!((overlaid_width, overlaid_height), (width, height));
    let offset = |x: usize, y: usize| -> usize {
        y * (width * 4 + 1) + 1 + x * 4
    };
    let pixel = |data: &[u8], x: usize, y: usize| -> [u8; 4] {
        data.get(offset(x, y)..offset(x, y) + 4)
            .expect("physical A5 pixel within raster")
            .try_into().expect("RGBA pixel has four channels")
    };
    assert_eq!(pixel(&raw, 945, 260), [0xdf, 0x26, 0x53, 255],
        "authored red ink covers the selected border edge");
    assert_ne!(pixel(&bare, 945, 260), pixel(&raw, 945, 260),
        "same pixel previously belonged to paper outline");
    assert_eq!(pixel(&raw, 945, 245), [0, 0, 0, 0],
        "paper remains transparent above the top border");
    assert_eq!(pixel(&raw, 237, 480), pixel(&bare, 237, 480),
        "ink does not change the remote left border");
}

#[test]
fn transparent_png_draws_authored_ink_above_rounded_paper_border() {
    let paper = default_paper_profile_json();
    let ink = [NotebookInkLine {
        page: 1,
        start_um: (70_000, 22_750),
        end_um: (100_000, 22_750),
        width_um: 1_500,
        rgb: [0xdf, 0x26, 0x53],
    }];
    // The vector PDF adds the border before ink, so overlapping pixels in
    // the transparent PNG must display the caller's ink, not the border.
    let png = paper_profile_png_page_with_ink(
        paper.as_bytes(), 150, false, None, false, 1, 5_000,
        None, &ink,
    ).expect("rounded border and authored ink");
    let (width, _, raw) = stored_png_pixels(&png);
    let pixel = |x: usize, y: usize| -> &[u8] {
        let index = y * (width * 4 + 1) + 1 + x * 4;
        raw.get(index..index + 4).expect("sample in page")
    };
    assert_eq!(pixel(472, 130), &[0xdf, 0x26, 0x53, 255],
        "ink has visual precedence over the paper border");
    assert_eq!(pixel(472, 128), &[0, 0, 0, 0],
        "no opaque backdrop above the paper border");
}

#[test]
fn manual_ink_refuses_overflow_and_missing_printable_clearance() {
    let paper = default_paper_profile_json();
    let good = NotebookInkLine {
        page: 1,
        start_um: (21_000, 23_000),
        end_um: (26_000, 27_000),
        width_um: 200,
        rgb: [10, 20, 30],
    };
    let invalid = [
        NotebookInkLine { page: 0, ..good },
        NotebookInkLine { page: 33, ..good },
        NotebookInkLine { start_um: good.end_um, ..good },
        NotebookInkLine { width_um: 0, ..good },
        NotebookInkLine { width_um: 1_501, ..good },
        NotebookInkLine { start_um: (0, 23_000), ..good },
        NotebookInkLine { end_um: (210_000, 27_000), ..good },
        NotebookInkLine { end_um: (26_000, 297_000), ..good },
    ];
    for bad in invalid {
        assert!(paper_profile_pdf_pages_with_ink(
            paper.as_bytes(), 1, false, None, false, 0, None, &[bad],
        ).is_err(), "invalid PDF ink {bad:?}");
        assert!(paper_profile_png_page_with_ink(
            paper.as_bytes(), 72, false, None, false, 1, 0, None, &[bad],
        ).is_err(), "invalid PNG ink {bad:?}");
    }
    let off_page = [NotebookInkLine { page: 2, ..good }];
    assert!(paper_profile_pdf_pages_with_ink(
        paper.as_bytes(), 1, false, None, false, 0, None, &off_page,
    ).is_err(), "PDF cannot drop unrequested page ink");
    assert!(paper_profile_png_page_with_ink(
        paper.as_bytes(), 72, false, None, false, 1, 0, None, &off_page,
    ).is_ok(), "PNG may select one of multiple valid notebook pages");
    let oversized = vec![good; MAX_NOTEBOOK_INK_LINES + 1];
    assert!(paper_profile_pdf_pages_with_ink(
        paper.as_bytes(), 1, false, None, false, 0, None, &oversized,
    ).is_err());
    assert!(paper_profile_png_page_with_ink(
        paper.as_bytes(), 72, false, None, false, 1, 0, None, &oversized,
    ).is_err());
}

#[test]
#[expect(
    clippy::integer_division,
    clippy::integer_division_remainder_used,
    reason = "test-only fixed grid indexing across 32 paper pages",
)]
fn selected_png_renders_only_its_page_but_validates_all_4096_ink_lines() {
    let paper = default_paper_profile_json();
    let ink = (0..MAX_NOTEBOOK_INK_LINES).map(|index| {
        let column = u32::try_from(index / 32 % 32)
            .expect("bounded paper column");
        let row = u32::try_from(index / 1_024)
            .expect("bounded paper row");
        let x = 25_000 + column * 4_000;
        let y = 25_000 + row * 8_000;
        NotebookInkLine {
            page: index % 32 + 1,
            start_um: (x, y),
            end_um: (x + 800, y + 1_000),
            width_um: 250,
            rgb: [25, 45, 100],
        }
    }).collect::<Vec<_>>();
    let selected = ink.iter().copied()
        .filter(|line| line.page == 1)
        .collect::<Vec<_>>();
    assert_eq!(selected.len(), 128);
    let output = |lines: &[NotebookInkLine]| {
        paper_profile_png_page_with_ink(
            paper.as_bytes(), 72, false, None, false, 1, 0, None,
            lines,
        )
    };
    assert_eq!(
        output(&ink).expect("all 32 page groups admitted"),
        output(&selected).expect("selected 128 paths alone"),
        "other-page strokes never paint the selected PNG",
    );

    let mut invalid = ink;
    invalid.get_mut(31).expect("bounded 4,096-line fixture").start_um =
        (0, 25_000);
    assert_eq!(
        output(&invalid).expect_err("invalid page 32 ink cannot be dropped"),
        "page 32 ink needs printable endpoints and valid width",
    );
}

#[test]
#[expect(
    clippy::integer_division,
    clippy::integer_division_remainder_used,
    reason = "test-only fixed grid indexing across 32 columns and three pages",
)]
fn multilingual_sized_ink_batch_renders_pdf_and_selected_png() {
    let paper = default_paper_profile_json();
    let ink = (0..2738usize).map(|index| {
        let page = 1 + index % 3;
        let cell = index / 3;
        let x = 25_000 + u32::try_from(page)
            .expect("bounded page") * 1_500
            + u32::try_from(cell % 32)
                .expect("bounded column") * 4_300;
        let y = 25_000 + u32::try_from(cell / 32)
            .expect("bounded row") * 7_000;
        NotebookInkLine {
            page, start_um: (x, y), end_um: (x + 600, y + 1100),
            width_um: 250, rgb: [25, 45, 100],
        }
    }).collect::<Vec<_>>();
    assert_eq!(ink.len(), 2738);
    let pdf = paper_profile_pdf_pages_with_ink(
        paper.as_bytes(), 3, false, None, false, 0, None, &ink,
    ).expect("bilingual-sized multisegment PDF");
    assert!(pdf.starts_with(b"%PDF-1.4"));
    assert_eq!(pdf, paper_profile_pdf_pages_with_ink(
        paper.as_bytes(), 3, false, None, false, 0, None, &ink,
    ).expect("repeated PDF exact bytes"));
    let png = paper_profile_png_page_with_ink(
        paper.as_bytes(), 72, false, None, false, 2, 0, None, &ink,
    ).expect("selected transparent PNG of larger export");
    assert!(png.starts_with(b"\x89PNG\r\n\x1a\n"));
    assert_eq!(png, paper_profile_png_page_with_ink(
        paper.as_bytes(), 72, false, None, false, 2, 0, None, &ink,
    ).expect("selected PNG replay"));
    let other = paper_profile_png_page_with_ink(
        paper.as_bytes(), 72, false, None, false, 1, 0, None, &ink,
    ).expect("different selected PNG page");
    assert_ne!(png, other, "distinct physical pages isolate authored ink");
}

#[test]
fn degenerate_paper_stroke_refuses_png_instead_of_painting_a_square() {
    let point = PdfPoint { x_um: 25_000, y_um: 25_000 };
    let path = PdfVectorPath {
        commands: vec![
            PdfPathCommand::MoveTo(point),
            PdfPathCommand::LineTo(point),
        ],
        paint: PdfPathPaint::Stroke {
            color: DEFAULT_COLOR,
            width_um: 120,
        },
    };
    let error = png::render_paper_png(
        210_000, 297_000, 72, &[path], 1, None,
    ).expect_err("zero-length paper stroke must not be a painted dot");
    assert_eq!(error, "invalid zero-length paper stroke");
}

#[test]
fn reversed_pen_segment_has_identical_png_pixels_at_supported_dpi() {
    let paper = encode_profile(Options {
        style: Style::Blank,
        width_um: 70_000,
        height_um: 100_000,
        ..Options::default()
    });
    for dpi in [72, 150, 300] {
        for (start, end) in [
            ((25_000, 25_000), (35_000, 25_000)),
            ((25_000, 25_000), (25_000, 45_000)),
            ((25_000, 25_000), (35_000, 45_000)),
            ((25_000, 45_000), (35_000, 25_000)),
            ((25_000, 25_000), (35_000, 27_500)),
        ] {
            let line = NotebookInkLine {
                page: 1,
                start_um: start,
                end_um: end,
                width_um: 700,
                rgb: [0x58, 0x21, 0xc4],
            };
            let reverse = NotebookInkLine {
                start_um: end,
                end_um: start,
                ..line
            };
            let render = |ink: NotebookInkLine| {
                paper_profile_png_page_with_ink(
                    paper.as_bytes(), dpi, false, None, false, 1, 0,
                    None, &[ink],
                ).expect("bounded line in a selected transparent page")
            };
            assert_eq!(render(line), render(reverse),
                "physical pen bitmap must be direction-neutral: {dpi} DPI, \
                 {start:?} -> {end:?}");
        }
    }
}

#[test]
fn edge_crossing_paper_marks_refuse_both_pdf_and_png() {
    let options = Options {
        margin_left_um: 0,
        margin_top_um: 0,
        ..Options::default()
    };
    assert!(render_pages(options, 1).is_err(),
        "PDF must not silently clip the edge-crossing paper marks");
    assert!(render_png(options, 72).is_err(),
        "PNG must share the PDF printable sheet clipping refusal");
}

#[test]
fn direct_png_path_bounds_keep_fills_and_half_strokes_on_sheet() {
    let page_width = 70_000;
    let page_height = 100_000;
    let stroke = |first: PdfPoint, last: PdfPoint, width_um| {
        PdfVectorPath {
            commands: vec![
                PdfPathCommand::MoveTo(first),
                PdfPathCommand::LineTo(last),
            ],
            paint: PdfPathPaint::Stroke {
                color: DEFAULT_COLOR,
                width_um,
            },
        }
    };
    let render = |path: PdfVectorPath| {
        png::render_paper_png(
            page_width, page_height, 72, &[path], 1, None,
        )
    };
    let center = PdfPoint { x_um: 25_000, y_um: 25_000 };
    assert!(render(stroke(
        PdfPoint { x_um: 60, y_um: 60 }, center, 120,
    )).is_ok(), "inclusive half-stroke page boundary is valid");
    assert!(render(stroke(
        PdfPoint { x_um: 59, y_um: 60 }, center, 120,
    )).is_err(), "a single micrometre beyond the left edge refuses");
    assert!(render(stroke(
        PdfPoint { x_um: 60, y_um: 59 }, center, 120,
    )).is_err(), "a single micrometre above the top edge refuses");
    assert!(render(stroke(
        center, PdfPoint { x_um: 69_941, y_um: 99_940 }, 120,
    )).is_err(), "a one-micrometre overrun at the far edge refuses");
    assert!(render(stroke(center, center, 0)).is_err(),
        "zero-width pen path refuses before pixel allocation");
    assert!(render(stroke(
        PdfPoint { x_um: -1, y_um: 25_000 }, center, 120,
    )).is_err(), "negative physical coordinates refuse");

    let fill = PdfVectorPath {
        commands: vec![
            PdfPathCommand::MoveTo(PdfPoint { x_um: 0, y_um: 0 }),
            PdfPathCommand::LineTo(PdfPoint { x_um: 100, y_um: 0 }),
            PdfPathCommand::LineTo(PdfPoint { x_um: 100, y_um: 100 }),
            PdfPathCommand::LineTo(PdfPoint { x_um: 0, y_um: 100 }),
            PdfPathCommand::Close,
        ],
        paint: PdfPathPaint::Fill {
            color: DEFAULT_COLOR,
            rule: PdfFillRule::NonZero,
        },
    };
    assert!(render(fill.clone()).is_ok(),
        "a filled dot may touch the page edge without stroke clearance");
    let mut outside = fill;
    *outside.commands.get_mut(2).expect("test fill corner exists") =
        PdfPathCommand::LineTo(
            PdfPoint { x_um: 70_001, y_um: 100 },
        );
    assert!(render(outside).is_err(),
        "a filled polygon cannot paint outside the physical page");
}

#[test]
fn zero_margin_blank_and_dotted_paper_remain_exportable_in_both_formats() {
    for style in [Style::Blank, Style::Dot] {
        let options = Options {
            style,
            margin_left_um: 0,
            margin_top_um: 0,
            ..Options::default()
        };
        let pdf = render_pages(options, 1).expect(
            "blank pages and edge-touching filled dots have no stroke overrun",
        );
        let png = render_png(options, 72).expect(
            "PNG must preserve admitted zero-margin filled paper",
        );
        assert!(pdf.starts_with(b"%PDF-1.4"));
        assert!(png.starts_with(b"\x89PNG\r\n\x1a\n"));
    }
}

// Check both faces of the PDF half-stroke clearance at the admitted PNG
// resolution extremes, including all four sheet edges and dot fills.
#[test]
fn paper_margin_clearance_matches_pdf_at_both_png_dpi_extremes() {
    let margins = [0, 59, 60, 61, 90, 121];
    for style in [Style::Blank, Style::Dot, Style::Grid, Style::Ruled] {
        let (width_um, height_um) = (40_000, 60_000);
        for margin in margins {
            for edge in 0u8..4u8 {
                let mut options = Options {
                    style,
                    width_um,
                    height_um,
                    margin_left_um: 0,
                    margin_right_um: 0,
                    margin_top_um: 0,
                    margin_bottom_um: 0,
                    ..Options::default()
                };
                match edge {
                    0 => options.margin_left_um = margin,
                    1 => options.margin_right_um = margin,
                    2 => options.margin_top_um = margin,
                    _ => options.margin_bottom_um = margin,
                }
                let pdf = render_pages(options, 1);
                for dpi in [72, 300] {
                    let png = render_png(options, dpi);
                    assert_eq!(
                        pdf.is_ok(),
                        png.is_ok(),
                        "{style:?} {width_um}x{height_um}, edge {edge}, \
                         margin {margin}, {dpi} DPI: PDF {:?}, PNG {:?}",
                        pdf.as_ref().err(),
                        png.as_ref().err(),
                    );
                }
            }
        }
    }
}
