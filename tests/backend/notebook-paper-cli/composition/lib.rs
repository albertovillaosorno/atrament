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
//   - Native paper CLI geometry and argument regression fixtures.
// - Must-Not:
//   - Claim handwriting, profiles, images, or full notebook PDF rendering.
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
