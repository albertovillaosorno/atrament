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
