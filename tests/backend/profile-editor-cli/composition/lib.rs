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
fn handwriting_preflight_rejects_invalid_flags_before_profile_io() {
    for (flags, diagnostic) in [
        (["--profile", "unopened.json", "--profile", "other.json"]
            .as_slice(), "--profile must appear once, before edits"),
        (["--profile", "unopened.json", "--seed", "-1"]
            .as_slice(), "invalid seed"),
        (["--profile", "unopened.json", "--seed", "+11"]
            .as_slice(), "invalid seed"),
        (["--profile", "unopened.json", "--set-global",
            "not-a-dimension", "1", "2", "1", "1", "0"]
            .as_slice(), "unknown statistics dimension"),
        (["--profile", "unopened.json", "--set-global", "width",
            "2000", "1000", "1500", "1500", "0"]
            .as_slice(), "statistics range refused: InvertedEnvelope"),
        (["--profile", "unopened.json", "--set-global", "width",
            "2000", "3000", "2500", "2500", "600"]
            .as_slice(), "statistics range refused: DeviationImpossible"),
        (["--profile", "unopened.json", "--set-glyph", "ñ", "width",
            "2000", "3000", "5000", "2500", "0"]
            .as_slice(), "statistics range refused: MeanOutsideEnvelope"),
        (["--profile", "unopened.json", "--remove-glyph-dimension",
            "a", "invalid-dimension"].as_slice(), "unknown dimension"),
        (["--profile", "unopened.json", "--preview-glyph", "a", "-1"]
            .as_slice(), "preview occurrence requires an unsigned integer"),
        (["--profile", "unopened.json", "--preview-glyph", "ab", "0"]
            .as_slice(), "numeric preview requires one exact"),
        (["--profile", "unopened.json", "--set-glyph", "ab",
            "width", "1", "2", "1", "1", "0"]
            .as_slice(), "glyph edit requires one exact"),
        (["--profile", "unopened.json", "--remove-glyph", "ab"]
            .as_slice(), "glyph removal requires one exact"),
        (["--profile", "unopened.json", "--remove-glyph-dimension", "",
            "width"].as_slice(), "glyph removal requires one exact"),
        (["--profile", "unopened.json", "--inspect", "--output", "out.json"]
            .as_slice(), "--inspect is stdout only"),
        (["--profile", "unopened.json", "--inventory-text", "a",
            "--output", "out.json"].as_slice(),
            "--inventory-text is stdout only"),
        (["--profile", "unopened.json", "--preview-glyph", "a", "1",
            "--inspect"].as_slice(), "--preview-glyph is stdout-only"),
        (["--profile", "unopened.json", "--replace"]
            .as_slice(), "--replace requires --output"),
        (["--profile", "unopened.json", "--replace", "--replace"]
            .as_slice(), "--replace may appear only once"),
        (["--profile", "unopened.json", "--output", "a.json",
            "--output", "b.json"].as_slice(),
            "--output requires one explicit filename"),
        (["--profile", "unopened.json", "--output", "-"]
            .as_slice(), "--output requires one explicit filename"),
        (["--profile", "unopened.json", "--unexpected"]
            .as_slice(), "unknown handwriting option"),
    ] {
        let error = preflight_handwriting_options(&args(flags))
            .expect_err("invalid deferred flags must refuse before source I/O");
        assert!(error.contains(diagnostic), "{error}");
    }
    for flags in [
        ["--profile", "unopened.json", "--preview-glyph", "ñ", "0"]
            .as_slice(),
        ["--profile", "unopened.json", "--set-glyph", "n\u{303}",
            "width", "1000", "2000", "1500", "1500", "0"].as_slice(),
        ["--profile", "unopened.json", "--remove-glyph", "-"]
            .as_slice(),
        ["--profile", "unopened.json", "--inventory-text", "--output"]
            .as_slice(),
        ["--profile", "unopened.json", "--preview-glyph", "-",
            "42"].as_slice(),
        ["--set-global", "width", "1000", "2000", "1500", "1500", "0"]
            .as_slice(),
    ] {
        // A flag-shaped operand is not a second option selection.
        // Scalar ranges and key shape are now validated before source reads.
        preflight_handwriting_options(&args(flags))
            .expect("complete literal operands should survive early scanning");
    }
}

#[test]
fn handwriting_statistics_cli_requires_exact_decimal_scalar_text() {
    let canonical = ["width", "2000", "3000", "2500", "2500", "0"];
    for field in 1..canonical.len() {
        for invalid in ["+2000", "2e3", "0x7d0", " 2000", "2000 "] {
            let mut fields = canonical;
            fields[field] = invalid;
            let mut flags = args(&[
                "--profile", "unopened.json", "--set-global",
            ]);
            flags.extend(fields.map(str::to_owned));
            let error = preflight_handwriting_options(&flags)
                .expect_err("coerced scalar must refuse before profile read");
            assert!(error.contains("decimal"), "{invalid}: {error}");
            assert!(parse_handwriting_options(&args(&[
                "--set-global", fields[0], fields[1], fields[2],
                fields[3], fields[4], fields[5],
            ])).is_err(), "final parser accepted {invalid}");
        }
    }
    let signed = args(&[
        "--set-global", "slant", "-1000", "1000", "-100", "-100", "0",
    ]);
    preflight_handwriting_options(&signed)
        .expect("negative decimal statistics remain valid");
    let _admitted = parse_handwriting_options(&signed)
        .expect("negative decimal statistics survive final CLI admission");
}

#[test]
fn glyph_and_handwriting_profile_paths_need_explicit_spellings() {
    for source in ["", "-", "--inspect", "--output", "--profile"] {
        let error = preflight_handwriting_options(&args(&[
            "--profile", source,
        ])).expect_err("invalid handwriting source must refuse before IO");
        assert!(error.contains("explicit"), "{source:?}: {error}");
    }
    for source in ["./--inspect", "./-", "profile.json"] {
        preflight_handwriting_options(&args(&["--profile", source]))
            .expect("explicit handwriting profile path is valid");
    }
    for source in ["", "--inspect-glyph", "--output", "--profile"] {
        let error = preflight_glyph_profile_sources(&args(&[
            "--profile", source,
        ])).expect_err("invalid glyph file source must refuse before IO");
        assert!(error.contains("explicit"), "{source:?}: {error}");
    }
    for source in ["./--output", "./-", "glyphs.tsv", "-"] {
        preflight_glyph_profile_sources(&args(&["--profile", source]))
            .expect("explicit glyph library path or stdin is valid");
    }
}

#[test]
fn glyph_library_preflight_rejects_second_source_without_input_io() {
    for flags in [
        ["--profile", "-", "--profile", "missing.tsv"].as_slice(),
        ["--profile", "-", "--profile", "-"].as_slice(),
        ["--profile", "-", "--glyph-stroke", "a", "500", "500",
            "1800", "3000", "250", "#123456", "--profile", "missing.tsv"]
            .as_slice(),
    ] {
        let error = preflight_glyph_profile_sources(&args(flags))
            .expect_err("duplicate library must refuse before source I/O");
        assert!(error.contains("may appear only once"), "{error}");
    }
    for flags in [
        ["--audit-text", "--profile", "--profile", "-"].as_slice(),
        ["--profile", "-", "--audit-text", "--profile"].as_slice(),
        ["--glyph-stroke", "a", "500", "500", "1800", "3000",
            "250", "#123456", "--audit-text", "--profile"].as_slice(),
    ] {
        preflight_glyph_profile_sources(&args(flags))
            .expect("literal option-like operands do not select sources");
    }
}

#[test]
fn glyph_library_preflight_refuses_missing_operands_before_source_io() {
    for (flags, diagnostic) in [
        (["--profile", "-", "--audit-text"].as_slice(),
            "--audit-text requires UTF-8 text"),
        (["--profile", "-", "--inspect-glyph"].as_slice(),
            "inspection requires an exact grapheme"),
        (["--profile", "-", "--inspect-paths"].as_slice(),
            "inspection requires an exact grapheme"),
        (["--profile", "-", "--remove-glyph"].as_slice(),
            "--remove-glyph requires an exact grapheme"),
        (["--profile", "-", "--copy-glyph", "a"].as_slice(),
            "--copy-glyph requires 2 operands"),
        (["--profile", "-", "--rotate-glyph", "a", "10", "200"]
            .as_slice(), "--rotate-glyph requires 4 operands"),
        (["--profile", "-", "--move-stroke", "a", "1"]
            .as_slice(), "--move-stroke requires 3 operands"),
        (["--profile", "-", "--set-stroke", "a", "1", "500",
            "500", "1800", "3000", "250"].as_slice(),
            "--set-stroke requires 8 operands"),
        (["--profile", "-", "--glyph-stroke", "a", "500",
            "500", "1800", "3000", "250"].as_slice(),
            "--glyph-stroke requires 7 operands"),
        (["--profile", "-", "--scale-glyph-ink", "a"].as_slice(),
            "--scale-glyph-ink requires 2 operands"),
    ] {
        let error = preflight_glyph_profile_sources(&args(flags))
            .expect_err("missing glyph operand must refuse before stdin");
        assert_eq!(error, diagnostic, "{flags:?}");
    }
    let error = preflight_glyph_profile_sources(&args(&[
        "--profile", "-", "--copy-glyph", "--output", "--replace",
    ])).expect_err("flag-shaped copy keys are operands, then invalid keys");
    assert!(error.contains("glyph copy needs two distinct exact"));
}

#[test]
fn glyph_library_preflights_typed_stroke_geometry_before_source_io() {
    for (stroke, diagnostic) in [
        (["a", "500", "500", "500", "500", "250", "#123456"],
            "glyph stroke must have two distinct points"),
        (["a", "500", "500", "1800", "3000", "79", "#123456"],
            "glyph ink width requires 80..1500"),
        (["a", "500", "500", "1800", "3000", "250", "#GGGGGG"],
            "glyph color requires #RRGGBB"),
        (["a", "3501", "500", "1800", "3000", "250", "#123456"],
            "glyph x1 requires 0..3500"),
    ] {
        let mut flags = args(&["--profile", "-", "--glyph-stroke"]);
        flags.extend(stroke.iter().map(|value| (*value).to_owned()));
        let error = preflight_glyph_profile_sources(&flags)
            .expect_err("invalid authored geometry must refuse before stdin");
        assert!(error.contains(diagnostic), "{error}");
    }
    preflight_glyph_profile_sources(&args(&[
        "--profile", "-", "--glyph-stroke", "ñ", "500", "500",
        "1800", "3000", "250", "#123456",
    ])).expect("valid exact Unicode shape should survive syntax admission");
}

#[test]
fn native_ink_and_heading_colors_require_exact_ascii_hex_pairs() {
    for color in ["#+A1122", "#11+A22", "#1122+A", "#11G222"] {
        let mut glyph = args(&[
            "--profile", "-", "--glyph-stroke", "ñ", "500", "500",
            "1800", "3000", "250",
        ]);
        glyph.push(color.into());
        let error = preflight_glyph_profile_sources(&glyph)
            .expect_err("signed glyph channel refuses before source read");
        assert!(error.contains("glyph color requires #RRGGBB"), "{error}");

        let mut manual = args(&[
            "--ink-file", "-", "--ink-line", "1", "30000", "40000",
            "32000", "42000", "250",
        ]);
        manual.push(color.into());
        manual.extend(args(&["--export-paper", "pdf"]));
        let error = preflight_notebook_sources(&manual)
            .expect_err("signed manual ink channel refuses before source read");
        assert!(error.contains("ink color must be #RRGGBB"), "{error}");

        for option in ["--heading-color", "--heading-ink-color"] {
            let flags = args(&[
                "--flow-markdown-file", "-", option, color,
                "--export-paper", "pdf",
            ]);
            let error = preflight_notebook_sources(&flags)
                .expect_err(
                "signed heading channel refuses before source read"
            );
            assert!(
                error.contains(&format!("{option} requires #RRGGBB")),
                "{option}: {error}"
            );
        }
    }
    let valid = ["#012345", "#AabbCC"];
    for color in valid {
        let glyph = args(&[
            "--profile", "-", "--glyph-stroke", "ñ", "500", "500",
            "1800", "3000", "250", color,
        ]);
        preflight_glyph_profile_sources(&glyph)
            .expect("exact six-digit hex is valid");
    }
}

#[test]
fn glyph_library_preflights_typed_stroke_and_path_edits() {
    for (tail, diagnostic) in [
        (["--reverse-stroke", "a", "0"].as_slice(),
            "stroke index requires 1..4096"),
        (["--join-strokes", "a", "-1"].as_slice(),
            "stroke index requires 1..4096"),
        (["--move-stroke", "a", "1", "5000"].as_slice(),
            "stroke index requires 1..4096"),
        (["--set-stroke", "a", "1", "500", "500", "500", "500",
            "250", "#123456"].as_slice(),
            "glyph stroke must have two distinct points"),
        (["--set-stroke", "a", "0", "500", "500", "1800", "3000",
            "250", "#123456"].as_slice(),
            "stroke index requires 1..4096"),
        (["--reverse-path", "a", "0"].as_slice(),
            "path index requires 1..4096"),
        (["--move-path", "a", "1", "5000"].as_slice(),
            "path index requires 1..4096"),
        (["--rotate-path", "a", "1", "0"].as_slice(),
            "path start stroke requires 1..4096"),
        (["--translate-path", "a", "1", "5000", "0"].as_slice(),
            "path dx outside native bounds"),
        (["--translate-path", "a", "1", "0", "0"].as_slice(),
            "path shift needs nonzero bounded whole um"),
        (["--copy-path", "a", "0", "ñ"].as_slice(),
            "source path index requires 1..4096"),
        (["--copy-path", "a", "4097", "ñ"].as_slice(),
            "source path index requires 1..4096"),
    ] {
        let mut flags = args(&["--profile", "-"]);
        flags.extend(tail.iter().map(|value| (*value).to_owned()));
        let error = preflight_glyph_profile_sources(&flags)
            .expect_err("invalid edit operands must refuse before stdin");
        assert!(error.contains(diagnostic), "{tail:?}: {error}");
    }
    for tail in [
        ["--reverse-stroke", "ñ", "1"].as_slice(),
        ["--set-stroke", "ñ", "1", "500", "500", "1800", "3000",
            "250", "#123456"].as_slice(),
        ["--translate-path", "ñ", "1", "-2", "2"].as_slice(),
        ["--copy-path", "n\u{303}", "1", "ñ"].as_slice(),
    ] {
        let mut flags = args(&["--profile", "-"]);
        flags.extend(tail.iter().map(|value| (*value).to_owned()));
        preflight_glyph_profile_sources(&flags)
            .expect("valid edit syntax must defer imported-library state");
    }
}

#[test]
fn glyph_library_preflights_typed_transformation_values() {
    for (tail, diagnostic) in [
        (["--rotate-glyph", "a", "21", "1750", "2250"].as_slice(),
            "rotation angle outside native bounds"),
        (["--rotate-glyph", "a", "10", "1750", "4501"].as_slice(),
            "rotation Y pivot requires 0..4500"),
        (["--translate-glyph", "a", "-3501", "0"].as_slice(),
            "glyph dx outside native bounds"),
        (["--shear-glyph", "a", "21", "2250"].as_slice(),
            "italic angle requires 0..20"),
        (["--scale-glyph-height", "a", "151", "2250"].as_slice(),
            "glyph height percent requires 50..150"),
        (["--scale-glyph-width", "a", "100", "3501"].as_slice(),
            "glyph width anchor requires 0..3500"),
        (["--scale-glyph-ink", "a", "49"].as_slice(),
            "glyph ink percent requires 50..150"),
    ] {
        let mut flags = args(&["--profile", "-"]);
        flags.extend(tail.iter().map(|value| (*value).to_owned()));
        let error = preflight_glyph_profile_sources(&flags)
            .expect_err("invalid transformation must refuse before stdin");
        assert!(error.contains(diagnostic), "{tail:?}: {error}");
    }
    for tail in [
        ["--rotate-glyph", "ñ", "-10", "1750", "2250"].as_slice(),
        ["--translate-glyph", "ñ", "0", "0"].as_slice(),
        ["--scale-glyph-ink", "ñ", "100"].as_slice(),
    ] {
        let mut flags = args(&["--profile", "-"]);
        flags.extend(tail.iter().map(|value| (*value).to_owned()));
        preflight_glyph_profile_sources(&flags)
            .expect("valid transformation syntax defers shape checks");
    }
}

#[test]
fn glyph_library_preflights_exact_mutation_and_inspection_keys() {
    for (tail, diagnostic) in [
        (["--copy-glyph", "a", "a"].as_slice(),
            "glyph copy needs two distinct exact visible graphemes"),
        (["--copy-glyph", "a", "ab"].as_slice(),
            "glyph copy needs two distinct exact visible graphemes"),
        (["--copy-glyph", " ", "ñ"].as_slice(),
            "glyph copy needs two distinct exact visible graphemes"),
        (["--copy-path", "a", "1", "a"].as_slice(),
            "glyph copy needs two distinct exact visible graphemes"),
        (["--copy-path", "a", "1", "bc"].as_slice(),
            "glyph copy needs two distinct exact visible graphemes"),
        (["--inspect-glyph", "ab"].as_slice(),
            "inspection requires one exact visible grapheme"),
        (["--inspect-paths", ""].as_slice(),
            "inspection requires one exact visible grapheme"),
        (["--inspect-paths", "\n"].as_slice(),
            "inspection requires one exact visible grapheme"),
        (["--remove-glyph", "ab"].as_slice(),
            "glyph removal requires one exact visible grapheme"),
        (["--remove-glyph", " "].as_slice(),
            "glyph removal requires one exact visible grapheme"),
        (["--remove-glyph", ""].as_slice(),
            "glyph removal requires one exact visible grapheme"),
    ] {
        let mut flags = args(&["--profile", "-"]);
        flags.extend(tail.iter().map(|value| (*value).to_owned()));
        let error = preflight_glyph_profile_sources(&flags)
            .expect_err("bad glyph keys must refuse before stdin");
        assert!(error.contains(diagnostic), "{tail:?}: {error}");
    }
    for tail in [
        ["--copy-glyph", "n\u{303}", "ñ"].as_slice(),
        ["--copy-path", "n\u{303}", "1", "ñ"].as_slice(),
        ["--inspect-glyph", "ñ"].as_slice(),
        ["--inspect-paths", "n\u{303}"].as_slice(),
        ["--remove-glyph", "ñ"].as_slice(),
        ["--remove-glyph", "n\u{303}"].as_slice(),
    ] {
        let mut flags = args(&["--profile", "-"]);
        flags.extend(tail.iter().map(|value| (*value).to_owned()));
        preflight_glyph_profile_sources(&flags)
            .expect("valid exact Unicode keys defer imported-library checks");
    }
}

#[test]
fn glyph_library_preflights_audit_text_before_stdin_source_io() {
    let over_limit = "ñ".repeat(2049);
    for input in ["", "  \n", "\u{feff}", "a\u{b}", "a\rb",
        over_limit.as_str()] {
        let flags = args(&["--profile", "-", "--audit-text", input]);
        let error = preflight_glyph_profile_sources(&flags)
            .expect_err("bad literal audit text must refuse before stdin");
        assert!(error.contains("glyph audit needs 1..4096 UTF-8 bytes"),
            "{input:?}: {error}");
    }
    for input in ["ñ", "--profile", "a\tb", "n\u{303} ∞"] {
        preflight_glyph_profile_sources(&args(&[
            "--profile", "-", "--audit-text", input,
        ])).expect("visible Unicode and literal options remain accepted");
    }
}

#[test]
fn glyph_library_preflight_refuses_deferred_flag_conflicts() {
    for (flags, diagnostic) in [
        (["--profile", "-", "--audit-text", "a", "--audit-text", "b"]
            .as_slice(), "--audit-text may appear only once"),
        (["--profile", "-", "--audit-baseline", "--audit-text", "a"]
            .as_slice(), "--audit-text may appear only once"),
        (["--profile", "-", "--inspect-paths", "a", "--inspect-glyph",
            "b"].as_slice(), "glyph inspection may appear only once"),
        (["--profile", "-", "--output", "one.tsv", "--output",
            "two.tsv"].as_slice(), "--output requires one explicit filename"),
        (["--profile", "-", "--replace", "--replace"].as_slice(),
            "--replace may appear only once"),
        (["--profile", "-", "--audit-baseline", "--inspect-glyph",
            "a"].as_slice(), "glyph inspection cannot accompany a glyph audit"),
        (["--profile", "-", "--audit-text", "a", "--output",
            "one.tsv"].as_slice(), "--audit-text is stdout-only"),
        (["--profile", "-", "--inspect-glyph", "a", "--output",
            "one.tsv"].as_slice(), "glyph inspection is stdout-only"),
        (["--profile", "-", "--replace"].as_slice(),
            "--replace requires --output"),
    ] {
        let error = preflight_glyph_profile_sources(&args(flags))
            .expect_err("invalid late glyph flags must refuse before stdin");
        assert!(error.contains(diagnostic), "{error}");
    }
    for flags in [
        ["--audit-text", "--inspect-glyph", "--profile", "-"].as_slice(),
        ["--profile", "-", "--audit-text", "--output"].as_slice(),
    ] {
        preflight_glyph_profile_sources(&args(flags))
            .expect("literal argument values must not select flags");
    }
}

#[test]
fn early_output_selection_requires_a_literal_filename() {
    for flags in [
        ["--profile", "-", "--output", ""].as_slice(),
        ["--profile", "-", "--output", "-"].as_slice(),
        ["--profile", "-", "--output", "--replace"].as_slice(),
    ] {
        let error = preflight_glyph_profile_sources(&args(flags))
            .expect_err("invalid selected glyph output must refuse before IO");
        assert_eq!(error, "--output requires one explicit filename");
    }
    for flags in [
        ["--ink-file", "-", "--output", ""].as_slice(),
        ["--glyph-file", "-", "--output", "-"].as_slice(),
        ["--flow-text-file", "-", "--output", "--replace"].as_slice(),
    ] {
        let error = preflight_notebook_sources(&args(flags))
            .expect_err("invalid notebook output must refuse before IO");
        assert_eq!(error, "--output requires one explicit filename");
    }
    assert_eq!(preflight_notebook_sources(&args(&[
        "--ink-file", "-", "--output",
    ])).expect_err("missing file must refuse"),
    "--output requires a filename");
    assert_eq!(preflight_glyph_profile_sources(&args(&[
        "--profile", "-", "--output",
    ])).expect_err("missing file must refuse"),
    "--output requires a filename");
    preflight_notebook_sources(&args(&[
        "--flow-text", "--output", "--export-paper", "pdf",
        "--glyph-file", "unopened.tsv",
    ])).expect("flag-like flow text is not an output selection");
}

#[test]
fn notebook_preflight_requires_selected_file_operands_before_stdin() {
    for (option, diagnostic) in [
        ("--glyph-file", "--glyph-file requires a library path"),
        ("--ink-file", "--ink-file requires a file"),
        ("--profile", "--profile requires a file"),
        ("--paper-profile", "--paper-profile requires a file"),
        ("--flow-text-file",
            "flow needs a UTF-8 text value or a selected file path"),
        ("--flow-markdown-file",
            "flow needs a UTF-8 text value or a selected file path"),
    ] {
        for prefix in ["--ink-file", "--glyph-file", "--flow-text-file"] {
            // An earlier selected source may remain unclosed indefinitely.
            if (prefix.starts_with("--flow") && option.starts_with("--flow"))
                || (prefix == "--glyph-file" && option == "--glyph-file")
            {
                continue;
            }
            let mut flags = args(&[prefix, "-", "--export-paper", "pdf",
                option]);
            for selected in [None, Some("")] {
                if let Some(value) = selected { flags.push(value.into()); }
                let error = preflight_notebook_sources(&flags)
                    .expect_err("missing or empty path must refuse first");
                assert_eq!(error, diagnostic, "{flags:?}");
                flags.truncate(5);
            }
        }
    }
    for option in ["--glyph-file", "--ink-file", "--profile",
        "--paper-profile", "--flow-text-file", "--flow-markdown-file"] {
        let error = preflight_notebook_sources(&args(&["--ink-file", "-",
            option, "--output", "--export-paper", "pdf", "--glyph-stroke",
            "a", "500", "500", "1800", "3000", "250", "#123456",
            "--place-text", "1", "30000", "40000", "a"]))
            .expect_err("option-shaped source is not an implicit filename");
        assert!(error.contains("explicit"), "{option}: {error}");
        preflight_notebook_sources(&args(&["--ink-file", "-", option,
            "./--output", "--export-paper", "pdf", "--glyph-stroke",
            "a", "500", "500", "1800", "3000", "250", "#123456",
            "--place-text", "1", "30000", "40000", "a"]))
            .expect("explicitly relative dash-prefixed source is accepted");
    }
}

#[test]
fn notebook_preflight_checks_default_paper_inline_ink_clearance() {
    for (x1, y1, x2, y2, width) in [
        (19_999, 40_000, 70_000, 41_000, 250),
        (30_000, 21_999, 70_000, 41_000, 250),
        (195_876, 40_000, 190_000, 41_000, 250),
        (30_000, 280_876, 70_000, 275_000, 250),
        (20_124, 40_000, 70_000, 41_000, 250),
    ] {
        let flags = args(&[
            "--ink-file", "-", "--export-paper", "pdf",
            "--ink-line", "1", &x1.to_string(), &y1.to_string(),
            &x2.to_string(), &y2.to_string(), &width.to_string(),
            "#123456",
        ]);
        let error = preflight_notebook_sources(&flags)
            .expect_err("default paper margin refuses before stdin");
        assert!(error.contains("page 1 ink needs printable endpoints"),
            "{error}");
    }
    preflight_notebook_sources(&args(&[
        "--ink-file", "-", "--ink-line", "1", "20125", "22125",
        "195875", "280875", "250", "#123456",
        "--export-paper", "pdf",
    ])).expect("ink radius exactly on default printable boundary is valid");
    for source in ["--paper-profile", "--profile"] {
        let mut flags = args(&["--ink-file", "-", source, "unopened.json"]);
        flags.extend(args(&["--ink-line", "1", "10000", "30000",
            "11000", "31000", "250", "#123456",
            "--export-paper", "pdf"]));
        preflight_notebook_sources(&flags)
            .expect("selected paper may have different valid margins");
    }
}

#[test]
fn notebook_preflight_checks_default_paper_authored_glyph_clearance() {
    for (page, x, y) in [
        (1, 19_000, 40_000),
        (2, 30_000, 21_000),
        (3, 195_000, 40_000),
        (4, 30_000, 280_000),
    ] {
        let flags = args(&[
            "--ink-file", "-", "--glyph-stroke", "a", "500", "500",
            "1800", "3000", "250", "#123456", "--place-text",
            &page.to_string(), &x.to_string(), &y.to_string(), "a",
            "--export-paper", "png", "--png-page", "1",
        ]);
        let error = preflight_notebook_sources(&flags)
            .expect_err("default paper glyph clearance refuses before stdin");
        assert!(error.contains(&format!(
            "page {page} ink needs printable endpoints and valid width"
        )), "{error}");
    }
    let _error = preflight_notebook_sources(&args(&[
        "--ink-file", "-", "--glyph-stroke", "a", "500", "500",
        "1800", "3000", "250", "#123456", "--place-text",
        "1", "19625", "21624", "a", "--export-paper", "pdf",
    ])).expect_err("ink crosses the printable inset by half its width");
    preflight_notebook_sources(&args(&[
        "--ink-file", "-", "--glyph-stroke", "a", "500", "500",
        "1800", "3000", "250", "#123456", "--place-text",
        "1", "19625", "22125", "a", "--export-paper", "pdf",
    ])).expect("glyph ink touching default margin is valid");
    for selected in ["--paper-profile", "--profile"] {
        preflight_notebook_sources(&args(&[
            "--ink-file", "-", selected, "unopened.json",
            "--glyph-stroke", "a", "500", "500", "1800", "3000",
            "250", "#123456", "--place-text", "1", "10000", "40000",
            "a", "--export-paper", "pdf",
        ])).expect("caller-selected paper may admit different margins");
    }
    preflight_notebook_sources(&args(&[
        "--ink-file", "-", "--handwriting-profile", "unopened.json",
        "--glyph-stroke", "a", "500", "500", "1800", "3000",
        "250", "#123456", "--place-text", "1", "30000", "40000",
        "a", "--export-paper", "pdf",
    ])).expect("mean handwriting metrics require selected source");
}

#[test]
fn notebook_preflight_refuses_unplaceable_ink_before_selected_source() {
    for flags in [
        ["--glyph-file", "-", "--export-paper", "pdf"].as_slice(),
        ["--glyph-file", "-", "--emit-ink-file"].as_slice(),
        ["--glyph-file", "-", "--ink-line", "1", "70000", "40000",
            "71000", "41000", "250", "#123456", "--export-paper",
            "pdf"].as_slice(),
    ] {
        let error = preflight_notebook_sources(&args(flags))
            .expect_err("unplaced glyph library must refuse before stdin");
        assert!(error.contains("--glyph-stroke and --place-text require"),
            "{flags:?}: {error}");
    }
    let error = preflight_notebook_sources(&args(&[
        "--profile", "unopened.json", "--emit-ink-file",
    ])).expect_err("empty ink export must refuse before opening profile");
    assert_eq!(error, "page ink export needs at least one physical segment");

    for flags in [
        ["--glyph-file", "-", "--place-text", "1", "30000", "40000",
            "a", "--emit-ink-file"].as_slice(),
        ["--ink-file", "-", "--emit-ink-file"].as_slice(),
    ] {
        preflight_notebook_sources(&args(flags))
            .expect("selected input can supply placed physical ink");
    }
}

#[test]
fn selected_paper_cannot_defer_known_invalid_notebook_date_metadata() {
    for (flags, expected) in [
        (["--ink-file", "-", "--paper-profile", "unopened-paper.json",
            "--date-style", "line", "--export-paper", "pdf"].as_slice(),
            "date style and placement must both be none or set"),
        (["--ink-file", "-", "--paper-profile", "unopened-paper.json",
            "--date-text", "2026-11-11", "--export-paper", "pdf"].as_slice(),
            "printed date text requires a date placeholder"),
    ] {
        let error = preflight_notebook_sources(&args(flags))
            .expect_err("known metadata must refuse before source read");
        assert_eq!(error, expected);
    }
    preflight_notebook_sources(&args(&[
        "--ink-file", "-", "--paper-profile", "unopened-paper.json",
        "--date-style", "line", "--date-placement", "top-left",
        "--date-text", "2026-11-11", "--export-paper", "pdf",
    ])).expect("valid metadata may await the selected paper geometry");
    preflight_notebook_sources(&args(&[
        "--ink-file", "-", "--profile", "unopened-notebook.json",
        "--date-style", "line", "--export-paper", "pdf",
    ])).expect("a saved notebook may supply the missing date placement");
}

#[test]
fn notebook_preflight_detects_invalid_sources_without_input_io() {
    for flags in [
        ["--ink-file", "-", "--glyph-file", "-"].as_slice(),
        ["--glyph-file", "-", "--flow-text-file", "-"].as_slice(),
        ["--flow-markdown-file", "-", "--ink-file", "-"].as_slice(),
        ["--ink-file", "-", "--ink-file", "-"].as_slice(),
    ] {
        let error = preflight_notebook_sources(&args(flags))
            .expect_err("multiple stdin consumers must refuse before read");
        assert_eq!(error, "standard input may be selected only once");
    }
    for (flags, diagnostic) in [
        (["--glyph-file", "-", "--glyph-file", "missing.tsv"].as_slice(),
            "must precede authored strokes and may appear only once"),
        (["--glyph-file", "-", "--place-text", "1", "30000", "40000",
            "a", "--glyph-file", "missing.tsv"].as_slice(),
            "must precede authored strokes and may appear only once"),
        (["--flow-text-file", "-", "--flow-markdown", "hello"].as_slice(),
            "only one flow source may be supplied"),
        (["--flow-markdown-file", "-", "--flow-text-file", "text.md"]
            .as_slice(), "only one flow source may be supplied"),
        (["--ink-file", "-", "--paper-profile", "first.json",
            "--paper-profile", "second.json"].as_slice(),
            "--paper-profile may appear only once"),
        (["--flow-text-file", "-", "--profile", "one.json",
            "--profile", "two.json"].as_slice(),
            "--profile must appear once, before edits"),
        (["--glyph-file", "-", "--date-style", "line",
            "--profile", "notebook.json"].as_slice(),
            "--profile must appear once, before edits"),
        (["--ink-file", "-", "--handwriting-profile", "one.json",
            "--handwriting-profile", "two.json"].as_slice(),
            "--handwriting-profile may appear only once"),
        (["--ink-file", "-", "--handwriting-profile", ""].as_slice(),
            "invalid handwriting profile path"),
        (["--glyph-file", "-", "--handwriting-profile", "-"].as_slice(),
            "invalid handwriting profile path"),
        (["--flow-text-file", "-", "--handwriting-profile", "--replace"]
            .as_slice(), "invalid handwriting profile path"),
        (["--ink-file", "-", "--handwriting-profile"].as_slice(),
            "--handwriting-profile requires a saved profile path"),
        (["--flow-text-file", "-", "--flow-region", "20000",
            "30000", "40000", "50000", "--flow-region", "10000",
            "20000", "30000", "40000"].as_slice(),
            "--flow-region may appear only once"),
        (["--glyph-file", "-", "--output", "one.pdf",
            "--output", "two.pdf"].as_slice(),
            "--output requires one explicit filename"),
    ] {
        let error = preflight_notebook_sources(&args(flags))
            .expect_err("late invalid source must refuse before read");
        assert!(error.contains(diagnostic), "{error}");
    }
    for (flags, diagnostic) in [
        (["--ink-file", "-", "--inspect", "--inspect"].as_slice(),
            "--inspect may appear only once"),
        (["--glyph-file", "-", "--replace", "--replace"].as_slice(),
            "--replace may appear only once"),
        (["--ink-file", "-", "--export-paper", "pdf",
            "--export-paper", "png"].as_slice(),
            "--export-paper may appear only once"),
        (["--ink-file", "-", "--emit-ink-file",
            "--export-paper", "pdf"].as_slice(),
            "--export-paper may appear only once"),
        (["--ink-file", "-", "--export-paper", "pdf",
            "--emit-ink-file"].as_slice(),
            "--emit-ink-file may appear only once"),
        (["--ink-file", "-", "--pages", "1", "--pages", "2",
            "--export-paper", "pdf"].as_slice(),
            "--pages may appear only once"),
        (["--ink-file", "-", "--dpi", "150", "--dpi", "72",
            "--export-paper", "png"].as_slice(),
            "--dpi may appear only once"),
        (["--ink-file", "-", "--quality", "preview", "--quality",
            "final", "--export-paper", "png"].as_slice(),
            "--quality may appear only once"),
        (["--ink-file", "-", "--png-page", "1", "--png-page", "2",
            "--export-paper", "png"].as_slice(),
            "--png-page may appear only once"),
        (["--ink-file", "-", "--inspect", "--output", "out.pdf"]
            .as_slice(), "--inspect is stdout only"),
        (["--ink-file", "-", "--inspect", "--emit-ink-file"].as_slice(),
            "--inspect cannot export paper or page ink"),
        (["--ink-file", "-", "--replace"].as_slice(),
            "--replace requires --output"),
        (["--ink-file", "-", "--pages", "2", "--emit-ink-file"]
            .as_slice(), "--pages requires --export-paper pdf"),
        (["--ink-file", "-", "--dpi", "150", "--export-paper", "pdf"]
            .as_slice(), "require --export-paper png"),
        (["--ink-file", "-", "--quality", "preview", "--dpi", "150",
            "--export-paper", "png"].as_slice(),
            "--quality and --dpi are mutually exclusive"),
        (["--ink-file", "-", "--pages", "0", "--export-paper",
            "pdf"].as_slice(), "--pages requires 1..32"),
        (["--glyph-file", "-", "--pages", "33", "--export-paper",
            "pdf"].as_slice(), "--pages requires 1..32"),
        (["--ink-file", "-", "--dpi", "71", "--export-paper",
            "png"].as_slice(), "--dpi requires 72..300"),
        (["--flow-text-file", "-", "--dpi", "301", "--export-paper",
            "png"].as_slice(), "--dpi requires 72..300"),
        (["--ink-file", "-", "--png-page", "0", "--export-paper",
            "png"].as_slice(), "--png-page requires 1..32"),
        (["--glyph-file", "-", "--png-page", "33", "--export-paper",
            "png"].as_slice(), "--png-page requires 1..32"),
        (["--ink-file", "-", "--quality", "draft", "--export-paper",
            "png"].as_slice(), "--quality requires preview or final"),
        (["--ink-file", "-", "--quality"].as_slice(),
            "--quality requires preview or final"),
        (["--ink-file", "-", "--export-paper", "invalid"].as_slice(),
            "--export-paper requires pdf or png"),
        (["--flow-text-file", "-", "--flow-region", "-1",
            "30000", "40000", "50000", "--export-paper", "pdf"]
            .as_slice(), "flow region left must be unsigned um"),
        (["--flow-markdown-file", "-", "--flow-region", "20000",
            "30000", "0", "50000", "--export-paper", "pdf"]
            .as_slice(), "flow region width and height must be positive"),
        (["--flow-text-file", "-", "--flow-region", "20000",
            "30000", "40000", "0", "--export-paper", "pdf"]
            .as_slice(), "flow region width and height must be positive"),
        (["--flow-text-file", "-", "--flow-region", "20000",
            "4294967296", "40000", "50000", "--export-paper", "pdf"]
            .as_slice(), "flow region top exceeds u32 micrometres"),
        (["--flow-text-file", "-", "--glyph-file", "unopened.tsv",
            "--flow-region", "10000", "40000", "50000", "30000",
            "--export-paper", "pdf"].as_slice(),
            "flow region must fit within printable paper margins"),
        (["--flow-markdown-file", "-", "--glyph-file",
            "unopened.tsv", "--flow-region", "30000", "40000",
            "4294967295", "50000", "--export-paper", "pdf"]
            .as_slice(), "flow region right edge overflows"),
        (["--flow-markdown-file", "-", "--flow-region", "20000",
            "30000", "40000"].as_slice(),
            "--flow-region requires LEFT TOP WIDTH HEIGHT in um"),
        (["--flow-markdown-file", "-", "--flow-region",
            "--flow-region", "20000", "30000", "40000"].as_slice(),
            "flow region left must be unsigned um"),
        (["--ink-file", "-", "--ink-line", "0", "30000",
            "40000", "32000", "42000", "250", "#123456"].as_slice(),
            "ink page requires 1..32"),
        (["--ink-file", "-", "--ink-line", "6", "70000",
            "40000", "71000", "41000", "250", "#123456",
            "--pages", "5", "--export-paper", "pdf"].as_slice(),
            "manual ink requires 6 physical pages"),
        (["--glyph-file", "-", "--place-text", "3", "30000",
            "40000", "a", "--pages", "2", "--export-paper", "pdf"]
            .as_slice(), "manual ink requires 3 physical pages"),
        (["--flow-markdown-file", "-", "--glyph-stroke", "a",
            "500", "500", "1800", "3000", "250", "#123456",
            "--place-text", "4", "30000", "40000", "a",
            "--pages", "3", "--export-paper", "pdf"].as_slice(),
            "flow and manual ink require 4 physical pages"),
        (["--glyph-file", "-", "--ink-line", "1", "700001",
            "40000", "32000", "42000", "250", "#123456"].as_slice(),
            "ink x1 requires 0..700000"),
        (["--flow-text-file", "-", "--ink-line", "1", "30000",
            "40000", "32000", "42000", "79", "#123456"].as_slice(),
            "ink width requires 80..1500"),
        (["--ink-file", "-", "--ink-line", "2", "30000",
            "40000", "30000", "40000", "250", "#123456"].as_slice(),
            "page 2 ink needs printable endpoints and valid width"),
        (["--glyph-file", "-", "--ink-line", "31", "70000",
            "40000", "70000", "40000", "250", "#123456"].as_slice(),
            "page 31 ink needs printable endpoints and valid width"),
        (["--ink-file", "-", "--paper-profile", "unopened.json",
            "--ink-line", "1", "0", "40000", "32000", "42000",
            "250", "#123456", "--export-paper", "pdf"].as_slice(),
            "page 1 ink needs printable endpoints and valid width"),
        (["--flow-text-file", "-", "--ink-line", "3", "30000",
            "124", "32000", "42000", "250", "#123456"].as_slice(),
            "page 3 ink needs printable endpoints and valid width"),
        (["--flow-markdown-file", "-", "--ink-line", "1", "30000",
            "40000", "32000", "42000", "250", "#GGGGGG"].as_slice(),
            "ink color must be #RRGGBB"),
        (["--flow-text-file", "-", "--ink-line", "1", "30000",
            "40000", "32000", "42000", "250", "--ink-file"].as_slice(),
            "ink color must be #RRGGBB"),
        (["--ink-file", "-", "--date-style", "unknown",
            "--export-paper", "pdf"].as_slice(),
            "date style must be none, line or box"),
        (["--glyph-file", "-", "--date-placement", "bottom",
            "--export-paper", "pdf"].as_slice(),
            "date placement must be none, top-left or top-right"),
        (["--flow-text-file", "-", "--border-radius-um", "-1",
            "--export-paper", "pdf"].as_slice(),
            "border radius requires unsigned integer micrometres"),
        (["--ink-file", "-", "--border-radius-um", "4294967296",
            "--export-paper", "pdf"].as_slice(),
            "invalid border radius"),
        (["--flow-markdown-file", "-", "--page-numbers", "yes",
            "--export-paper", "pdf"].as_slice(),
            "--page-numbers requires true or false"),
        (["--ink-file", "-", "--printed-sheet", "1",
            "--export-paper", "pdf"].as_slice(),
            "--printed-sheet requires true or false"),
        (["--ink-file", "-", "--date-text", "2026-02-29",
            "--export-paper", "pdf"].as_slice(),
            "invalid printed Gregorian date"),
        (["--flow-text-file", "-", "--date-text", "31/02/2026",
            "--export-paper", "png"].as_slice(),
            "invalid printed Gregorian date"),
        (["--glyph-file", "-", "--date-text", "2026/11/11",
            "--export-paper", "pdf"].as_slice(),
            "invalid printed Gregorian date"),
        (["--ink-file", "-", "--date-text", "2026-11-11",
            "--export-paper", "pdf"].as_slice(),
            "printed date text requires a date placeholder"),
        (["--ink-file", "-", "--date-style", "line",
            "--export-paper", "pdf"].as_slice(),
            "date style and placement must both be none or set"),
        (["--flow-text-file", "-", "--glyph-file", "unopened.tsv",
            "--date-placement", "top-right", "--export-paper", "pdf"]
            .as_slice(),
            "date style and placement must both be none or set"),
        (["--flow-markdown-file", "-", "--heading-style", "marker",
            "--heading-style", "underline"].as_slice(),
            "--heading-style may appear only once"),
        (["--flow-markdown-file", "-", "--heading-color", "#112233",
            "--heading-color", "#332211"].as_slice(),
            "--heading-color may appear only once"),
        (["--glyph-file", "-", "--heading-ink-color", "#123456",
            "--heading-ink-color", "#654321"].as_slice(),
            "--heading-ink-color may appear only once"),
        (["--ink-file", "-", "--heading-marker-scale", "80",
            "--heading-marker-scale", "90"].as_slice(),
            "--heading-marker-scale repeated"),
        (["--ink-file", "-", "--heading-ink-scale", "150",
            "--heading-ink-scale", "170"].as_slice(),
            "--heading-ink-scale repeated"),
        (["--flow-markdown-file", "-"].as_slice(),
            "flow source needs --export-paper pdf or png"),
        (["--ink-file", "-", "--flow-region", "20000", "30000",
            "40000", "50000", "--export-paper", "pdf"].as_slice(),
            "--flow-region requires a text or Markdown flow"),
        (["--flow-text-file", "-", "--heading-style", "marker",
            "--export-paper", "pdf"].as_slice(),
            "heading customization requires --flow-markdown"),
        (["--flow-markdown-file", "-", "--heading-marker-scale",
            "90", "--export-paper", "pdf"].as_slice(),
            "--heading-marker-scale requires --heading-style marker"),
        (["--flow-markdown-file", "-", "--heading-style", "none",
            "--heading-color", "#123456", "--export-paper", "pdf"]
            .as_slice(), "--heading-color cannot accompany"),
        (["--glyph-file", "-", "--heading-ink-scale", "99",
            "--export-paper", "pdf"].as_slice(),
            "heading ink scale must be 100..250"),
        (["--flow-markdown-file", "-", "--heading-color", "#GGGGGG",
            "--export-paper", "pdf"].as_slice(),
            "--heading-color requires #RRGGBB"),
    ] {
        let error = preflight_notebook_sources(&args(flags))
            .expect_err("late export conflict must refuse before read");
        assert!(error.contains(diagnostic), "{error}");
    }
    for flags in [
        ["--profile", "unopened-notebook.json", "--ink-file", "-",
            "--date-text", "2026-11-11", "--export-paper", "pdf"].as_slice(),
        ["--paper-profile", "unopened-paper.json", "--ink-file", "-",
            "--date-style", "line", "--date-placement", "top-left",
            "--date-text", "2026-11-11", "--export-paper", "pdf"].as_slice(),
        ["--ink-file", "-", "--date-style", "line",
            "--date-placement", "top-left", "--date-text", "2026-11-11",
            "--export-paper", "pdf"].as_slice(),
        ["--ink-file", "-", "--ink-line", "2", "70000",
            "40000", "71000", "41000", "250", "#123456",
            "--pages", "2", "--export-paper", "pdf"].as_slice(),
        ["--glyph-file", "-", "--place-text", "3", "30000",
            "40000", "a", "--pages", "3", "--export-paper", "pdf"]
            .as_slice(),
    ] {
        preflight_notebook_sources(&args(flags))
            .expect("valid or deferred date metadata is source-safe");
    }
    for flags in [
        ["--flow-text", "--ink-file", "--ink-file", "-",
            "--export-paper", "pdf", "--glyph-file", "unopened.tsv"]
            .as_slice(),
        ["--glyph-file", "-", "--place-text", "1", "30000", "40000",
            "--ink-file", "--export-paper", "pdf"].as_slice(),
        ["--flow-markdown", "--export-paper", "--export-paper",
            "pdf", "--pages", "1", "--glyph-file", "unopened.tsv"]
            .as_slice(),
        ["--flow-markdown", "--heading-style", "--heading-style",
            "marker", "--export-paper", "pdf", "--glyph-file",
            "unopened.tsv"].as_slice(),
        ["--flow-markdown-file", "-", "--heading-style", "marker",
            "--heading-marker-scale", "95", "--emit-ink-file",
            "--glyph-file", "unopened.tsv"].as_slice(),
        ["--flow-markdown", "# a", "--date-style", "line",
            "--date-placement", "top-left", "--date-text", "2026-11-11",
            "--export-paper", "pdf", "--glyph-file", "unopened.tsv"]
            .as_slice(),
        ["--flow-text-file", "-", "--flow-region", "30000",
            "40000", "40000", "50000", "--export-paper", "pdf",
            "--glyph-file", "unopened.tsv"].as_slice(),
        ["--profile", "unopened-notebook.json", "--flow-text-file",
            "-", "--flow-region", "10000", "30000", "40000",
            "50000", "--export-paper", "pdf", "--glyph-file",
            "unopened.tsv"].as_slice(),
        ["--flow-markdown-file", "-", "--paper-profile",
            "unopened-paper.json", "--flow-region", "10000", "30000",
            "40000", "50000", "--export-paper", "pdf",
            "--glyph-file", "unopened.tsv"].as_slice(),
        ["--ink-file", "-", "--export-paper", "png", "--dpi",
            "150", "--png-page", "1", "--output", "out.png",
            "--replace"].as_slice(),
        ["--ink-file", "-", "--handwriting-profile", "valid.json",
            "--export-paper", "pdf", "--glyph-stroke", "a", "500",
            "500", "1800", "3000", "250", "#123456",
            "--place-text", "1", "30000", "40000", "a"].as_slice(),
    ] {
        preflight_notebook_sources(&args(flags))
            .expect("option-looking values are not source flags");
    }
}

#[test]
fn notebook_preflight_refuses_unrenderable_overlays_before_source_io() {
    for (flags, expected) in [
        (["--flow-text-file", "-", "--export-paper", "pdf"].as_slice(),
            "flow requires authored glyph shapes"),
        (["--flow-markdown-file", "-", "--ink-file", "empty.tsv",
            "--export-paper", "png"].as_slice(),
            "flow requires authored glyph shapes"),
        (["--ink-file", "-", "--handwriting-profile", "unopened.json",
            "--export-paper", "pdf"].as_slice(),
            "--handwriting-profile requires both"),
        (["--ink-file", "-", "--glyph-variation", "1", "1", "1",
            "1", "1", "1", "1", "--export-paper", "pdf"].as_slice(),
            "--glyph-variation requires placed text"),
    ] {
        let error = preflight_notebook_sources(&args(flags))
            .expect_err("impossible ink must refuse before source I/O");
        assert!(error.contains(expected), "{flags:?}: {error}");
    }
    preflight_notebook_sources(&args(&[
        "--glyph-file", "unopened.tsv", "--flow-text-file", "-",
        "--export-paper", "pdf",
    ])).expect("imported library may provide the missing shapes");
    for (flags, expected) in [
        (["--ink-file", "-", "--glyph-stroke", "a", "500", "500",
            "1800", "3000", "250", "#123456", "--export-paper", "pdf"]
            .as_slice(), "--glyph-stroke and --place-text require one another"),
        (["--ink-file", "-", "--place-text", "1", "30000", "40000",
            "a", "--export-paper", "pdf"].as_slice(),
            "--glyph-stroke and --place-text require one another"),
        (["--ink-file", "-", "--join-text-pair", "a", "b",
            "--export-paper", "pdf"].as_slice(),
            "--join-text-pair requires placed text"),
        (["--glyph-file", "-", "--join-text-pair", "a", "b",
            "--export-paper", "pdf"].as_slice(),
            "--join-text-pair requires placed text"),
        (["--ink-file", "-", "--fallback-glyph", "ñ", "a",
            "--export-paper", "pdf"].as_slice(),
            "--fallback-glyph requires authored glyphs and placed text"),
        (["--glyph-file", "-", "--fallback-glyph", "ñ", "a",
            "--export-paper", "pdf"].as_slice(),
            "--fallback-glyph requires authored glyphs and placed text"),
    ] {
        let error = preflight_notebook_sources(&args(flags))
            .expect_err("impossible glyph pairing must refuse before I/O");
        assert!(error.contains(expected), "{flags:?}: {error}");
    }
    preflight_notebook_sources(&args(&[
        "--glyph-file", "unopened.tsv", "--place-text", "1", "30000",
        "40000", "a", "--export-paper", "pdf",
    ])).expect("imported library may supply the missing glyph strokes");
}

#[test]
fn notebook_preflight_rejects_inline_ink_above_budget_before_stdin() {
    let line = args(&[
        "--ink-line", "1", "70000", "40000", "71000", "41000",
        "250", "#123456",
    ]);
    let mut request = args(&["--export-paper", "pdf"]);
    for _ in 0..MAX_NOTEBOOK_INK_LINES {
        request.extend(line.iter().cloned());
    }
    preflight_notebook_sources(&request)
        .expect("4096 inline lines without imports remain admitted");
    for imported_first in [true, false] {
        let mut importing = args(&["--ink-file", "-"]);
        if imported_first {
            importing.extend(request.iter().cloned());
        } else {
            importing = request.clone();
            importing.extend(args(&["--ink-file", "-"]));
        }
        assert_eq!(
            preflight_notebook_sources(&importing).expect_err(
                "a nonempty imported ink file needs one reserved line",
            ),
            "ink exceeds 4096 explicit line segments",
        );
    }
    let mut one_reserved = request.iter().take(
        request.len().saturating_sub(line.len()),
    ).cloned().collect::<Vec<_>>();
    one_reserved.extend(args(&["--ink-file", "-"]));
    preflight_notebook_sources(&one_reserved)
        .expect("4095 inline lines plus a minimum imported line fit");
    request.extend(line);
    assert_eq!(
        preflight_notebook_sources(&request).expect_err(
            "4097th inline line must refuse before opening stdin",
        ),
        "ink exceeds 4096 explicit line segments",
    );
}

#[test]
fn notebook_preflight_requires_export_for_inline_ink_and_glyphs() {
    let line = args(&[
        "--ink-line", "1", "70000", "40000", "71000", "41000",
        "250", "#123456",
    ]);
    let selected = args(&["--ink-file", "-"]);
    let mut invalid = selected.clone();
    invalid.extend(line);
    assert_eq!(
        preflight_notebook_sources(&invalid).expect_err(
            "inline ink without export must refuse before stdin",
        ),
        concat!(
            "ink and glyph overlays need ",
            "--export-paper pdf or png or --emit-ink-file",
        ),
    );
    let mut valid = invalid;
    valid.extend(args(&["--export-paper", "pdf"]));
    preflight_notebook_sources(&valid).expect("explicit PDF export is valid");

    for flags in [
        ["--glyph-file", "-", "--place-text", "1", "30000",
            "40000", "a"].as_slice(),
        ["--ink-file", "-", "--glyph-variation", "1", "1",
            "1", "1", "1", "1", "1"].as_slice(),
    ] {
        let error = preflight_notebook_sources(&args(flags))
            .expect_err("inline glyph intent needs explicit output mode");
        assert!(error.contains("ink and glyph overlays need"), "{error}");
    }
    for source in [
        ["--ink-file", "-"].as_slice(),
        ["--glyph-file", "-"].as_slice(),
        ["--ink-file", "missing-ink.tsv"].as_slice(),
        ["--glyph-file", "missing-glyph.tsv"].as_slice(),
    ] {
        let error = preflight_notebook_sources(&args(source))
            .expect_err("imported overlays cannot be saved in notebook v1");
        assert!(error.contains("ink and glyph overlays need"), "{error}");
    }
    for source in [
        ["--ink-file", "-", "--emit-ink-file"].as_slice(),
        ["--glyph-file", "-", "--place-text", "1", "30000", "40000",
            "a", "--export-paper", "pdf"].as_slice(),
    ] {
        preflight_notebook_sources(&args(source))
            .expect("selected ink or placed glyphs can feed exports");
    }
}

#[test]
fn notebook_preflights_exact_typed_glyph_options_without_opening_sources() {
    for (flags, diagnostic) in [
        (["--glyph-file", "-", "--glyph-stroke", "a", "3501", "500",
            "1800", "3000", "250", "#123456"].as_slice(),
            "glyph x1 requires 0..3500"),
        (["--flow-text-file", "-", "--glyph-stroke", "ñ", "500",
            "500", "500", "500", "250", "#123456"].as_slice(),
            "glyph stroke must have two distinct points"),
        (["--ink-file", "-", "--glyph-variation", "42", "0",
            "0", "0", "0", "0", "151"].as_slice(),
            "variation regularity requires 0..150"),
        (["--glyph-file", "-", "--place-text", "0", "30000",
            "40000", "a"].as_slice(), "text page requires 1..32"),
        (["--flow-text-file", "-", "--fallback-glyph", "a", "a"]
            .as_slice(), "fallback requires two distinct exact"),
        (["--ink-file", "-", "--join-text-anchors", "a", "b",
            "0", "1"].as_slice(), "join exit stroke requires 1..4096"),
        (["--glyph-file", "-", "--join-text-pair", "a", "b",
            "--join-text-pair", "a", "b"].as_slice(),
            "duplicate text join pair refused"),
        (["--glyph-file", "-", "--glyph-variation", "42", "0",
            "0", "0", "0", "0", "0", "--glyph-variation", "42",
            "0", "0", "0", "0", "0", "0"].as_slice(),
            "--glyph-variation may appear only once"),
    ] {
        let error = preflight_notebook_sources(&args(flags))
            .expect_err("bad typed glyph option must refuse before source IO");
        assert!(error.contains(diagnostic), "{error}");
    }
    for flags in [
        ["--glyph-file", "-", "--place-text", "1", "30000", "40000",
            "--glyph-stroke", "--export-paper", "pdf"].as_slice(),
        ["--flow-markdown-file", "-", "--glyph-stroke", "n\u{303}",
            "500", "500", "1800", "3000", "250", "#123456",
            "--join-text-pair", "n\u{303}", "n\u{303}",
            "--export-paper", "pdf"].as_slice(),
        ["--ink-file", "-", "--fallback-glyph", "ñ", "n\u{303}",
            "--glyph-stroke", "n\u{303}", "500", "500", "1800",
            "3000", "250", "#123456", "--place-text", "1", "30000",
            "40000", "ñ", "--export-paper", "pdf"].as_slice(),
    ] {
        preflight_notebook_sources(&args(flags))
            .expect("valid option-like and decomposed Unicode operands");
    }
}

#[test]
fn inline_flow_is_admitted_before_earlier_selected_stdin_sources() {
    let overlong = "x".repeat(4097);
    let expanded = "x\t".repeat(1025);
    for (flags, diagnostic) in [
        (["--glyph-file", "-", "--flow-text", ""].as_slice(),
            "flow text needs 1..4096 UTF-8 bytes"),
        (["--ink-file", "-", "--flow-markdown", "  \n"].as_slice(),
            "flow text needs 1..4096 UTF-8 bytes"),
        (["--glyph-file", "-", "--flow-text", "a\rb"].as_slice(),
            "flow text needs 1..4096 UTF-8 bytes"),
        (["--ink-file", "-", "--flow-markdown", "a\u{b}"]
            .as_slice(), "flow text needs 1..4096 UTF-8 bytes"),
        (["--ink-file", "-", "--flow-text", overlong.as_str()]
            .as_slice(), "flow text needs 1..4096 UTF-8 bytes"),
        (["--glyph-file", "-", "--flow-text", expanded.as_str()]
            .as_slice(), "expanded flow text exceeds 4096 UTF-8 bytes"),
    ] {
        let error = preflight_notebook_sources(&args(flags))
            .expect_err("invalid literal flow must refuse before source IO");
        assert!(error.contains(diagnostic), "{error}");
    }
    for flags in [
        ["--glyph-file", "-", "--flow-text", "a\tb",
            "--export-paper", "pdf"].as_slice(),
        ["--ink-file", "-", "--flow-markdown", "# ñ\r\n¡a!",
            "--emit-ink-file", "--glyph-stroke", "a", "500", "500",
            "1800", "3000", "250", "#123456"].as_slice(),
        ["--flow-text", "--output", "--export-paper", "pdf",
            "--glyph-file", "unopened.tsv"].as_slice(),
    ] {
        preflight_notebook_sources(&args(flags)).unwrap_or_else(|error| {
            panic!("valid inline flow {flags:?} refused: {error}")
        });
    }
}

#[test]
fn physical_ink_needs_upper_canvas_clearance_before_source_io() {
    // The pen extends beyond both endpoints. A nominal endpoint on the
    // 700-mm canvas edge must not survive the reusable ink export gate.
    for invalid in [
        ("699876", "50000", "699875", "51000"),
        ("50000", "699876", "51000", "699875"),
        ("700000", "50000", "699875", "51000"),
        ("50000", "700000", "51000", "699875"),
    ] {
        let flags = args(&[
            "--ink-file", "unopened.tsv", "--ink-line", "1",
            invalid.0, invalid.1, invalid.2, invalid.3, "250", "#123456",
            "--emit-ink-file",
        ]);
        let error = preflight_notebook_sources(&flags)
            .expect_err("upper pen clearance must refuse before file read");
        assert!(error.contains("printable endpoints"), "{error}");
    }
    let admitted = NotebookInkLine {
        page: 1, start_um: (699_875, 50_000),
        end_um: (699_875, 51_000), width_um: 250,
        rgb: [0x12, 0x34, 0x56],
    };
    validate_native_ink_line(&admitted)
        .expect("centerline exactly one half-width inside limit");
}

#[test]
fn invalid_page_ink_tsv_does_not_append_any_valid_prefix() {
    let file = test_path().with_extension("tsv");
    let name = file.to_str().expect("owned TSV path");
    let original = NotebookInkLine {
        page: 3, start_um: (70_000, 45_000),
        end_um: (71_000, 46_000), width_um: 250,
        rgb: [0x12, 0x34, 0x56],
    };
    let source = concat!(
        "atrament.page-ink.v1\n",
        "1\t80000\t50000\t81000\t51000\t250\t#ABCDEF\n",
        "2\t90000\t50000\t91000\t51000\t300\t#123456\n",
    );
    let invalid = [
        ("atrament.page-ink.v1\n".to_owned(),
            "at least one physical segment"),
        (format!("{source}not a TSV row\n"),
            "line 4 needs seven tab-separated fields"),
        (format!("{source}4\t80000\t50000\t81000\t51000\t0\t#ABCDEF\n"),
            "line 4 refused: ink width"),
        (format!("{source}4\t699876\t50000\t699875\t51000\t250\t#ABCDEF\n"),
            "line 4 refused: page 4 ink needs printable endpoints"),
    ];
    for (contents, diagnostic) in &invalid {
        fs::write(&file, contents).expect("owned invalid file");
        let mut ink = vec![original];
        let error = load_page_ink_file(name, &mut ink)
            .expect_err("invalid physical ink file must refuse");
        assert!(error.contains(diagnostic), "unexpected error: {error}");
        assert_eq!(ink, [original],
            "failed file import retained its valid prefix");
    }
    fs::write(&file, source).expect("owned valid file");
    let mut almost_full = vec![original; MAX_NOTEBOOK_INK_LINES - 1];
    let error = load_page_ink_file(name, &mut almost_full)
        .expect_err("combined manual and file ink must fit budget");
    assert!(error.contains("4096 explicit line segments"), "{error}");
    assert_eq!(almost_full, vec![original; MAX_NOTEBOOK_INK_LINES - 1],
        "over-budget source must not append even its first valid row");

    let mut ink = vec![original];
    load_page_ink_file(name, &mut ink).expect("complete valid TSV accepted");
    assert_eq!(ink.len(), 3);
    assert_eq!(ink[0], original);
    assert_eq!(ink[1].page, 1);
    assert_eq!(ink[1].rgb, [0xab, 0xcd, 0xef]);
    assert_eq!(ink[2].page, 2);
    let accepted = ink.clone();
    fs::write(&file, format!("{source}broken\n"))
        .expect("owned late malformed TSV");
    assert!(load_page_ink_file(name, &mut ink).is_err());
    assert_eq!(ink, accepted,
        "failed second file must preserve previously appended layers");
    fs::remove_file(file).expect("remove owned test TSV");
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
    for seed in ["0", "18446744073709551615"] {
        let bytes = execute_handwriting(&args(&["--seed", seed]))
            .expect("full unsigned seed range admitted")
            .expect("canonical seed JSON");
        let round_trip = decode_statistics_json(
            &bytes, &UnicodeGraphemeSegmentation,
        ).expect("reopening preserves full seed precision");
        assert_eq!(round_trip.seed.to_string(), seed);
    }
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
        args(&["--seed", "+11"]),
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
fn named_notebook_png_quality_preserves_ink_and_atomic_refusal() {
    let output = test_path();
    let filename = output.to_str().expect("test-owned output name");
    let ink = args(&[
        "--printed-sheet", "true", "--export-paper", "png",
        "--png-page", "2", "--ink-line", "2", "25000", "30000",
        "50000", "40000", "300", "#123456",
    ]);
    for (quality, dpi) in [("preview", "72"), ("final", "150")] {
        let mut named = ink.clone();
        named.extend(args(&["--quality", quality]));
        let mut numeric = ink.clone();
        numeric.extend(args(&["--dpi", dpi]));
        let selected = execute_notebook(&named)
            .expect("named authored-ink preview").expect("PNG bytes");
        let expected = execute_notebook(&numeric)
            .expect("numeric authored-ink preview").expect("PNG bytes");
        assert_eq!(selected, expected,
            "named quality never changes authored marks or paper geometry");
        assert!(selected.starts_with(b"\x89PNG\r\n\x1a\n"));
        assert_eq!(selected.get(25), Some(&6), "transparent RGBA");
    }
    fs::write(&output, b"existing export").expect("output sentinel");
    for invalid in [
        args(&["--quality", "preview"]),
        args(&["--export-paper", "pdf", "--quality", "final"]),
        args(&["--export-paper", "png", "--quality", "unknown"]),
        args(&["--export-paper", "png", "--quality", ""]),
        args(&["--export-paper", "png", "--quality", "preview",
            "--quality", "final"]),
        args(&["--export-paper", "png", "--quality", "preview",
            "--dpi", "72"]),
        args(&["--export-paper", "png", "--dpi", "150",
            "--quality", "final"]),
    ] {
        let mut attempt = invalid;
        attempt.extend(args(&["--output", filename, "--replace"]));
        assert!(execute_notebook(&attempt).is_err(), "{attempt:?}");
        assert_eq!(fs::read(&output).expect("existing PNG unchanged"),
            b"existing export");
    }
    fs::remove_file(output).expect("remove test-owned sentinel");
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
fn inventory_cli_reports_exact_unicode_spaces_and_statistics_overrides() {
    let sample = "a\u{a0}a\u{2009}a\u{202f}a a\ta\na";
    let report = String::from_utf8(execute_handwriting(&args(&[
        "--set-glyph", "\u{2009}", "width", "1600", "2600", "2200",
        "2200", "200", "--inventory-text", sample,
    ])).expect("Unicode space inventory admitted")
        .expect("read-only report"))
        .expect("report is UTF-8");
    assert!(report.contains("Distinct visible graphemes: 4"),
        "{report}");
    assert!(report.contains("Outside English/Spanish text baseline: 3"),
        "{report}");
    for codepoint in ["U+00A0", "U+2009", "U+202F"] {
        assert!(report.contains(&format!("[{codepoint}]")),
            "the exact Unicode space must be listed: {report}");
    }
    assert!(report.contains(
        "outside-text-baseline / statistical-override [U+2009]"
    ), "{report}");
    assert!(!report.contains("[U+0020]"), "ASCII space is implicit");
    assert!(!report.contains("[U+0009]"), "tab is source structure");
    assert!(!report.contains("[U+000A]"), "newline is source structure");

    let single = String::from_utf8(execute_handwriting(&args(&[
        "--inventory-text", "\u{a0}",
    ])).expect("single authored space is auditable")
        .expect("report"))
        .expect("UTF-8 report");
    assert!(single.contains("Distinct visible graphemes: 1"));
    assert!(single.contains("Outside English/Spanish text baseline: 1"));
    assert!(single.contains("[U+00A0]"));

    // A full bounded source must not repeatedly rescan the original
    // grapheme prefix; every repeated exact space is counted only once.
    let long = "\u{2009}".repeat(1365);
    assert_eq!(long.len(), 4095);
    let large = String::from_utf8(execute_handwriting(&args(&[
        "--inventory-text", &long,
    ])).expect("bounded repeated spaces admitted")
        .expect("report"))
        .expect("UTF-8 report");
    assert!(large.contains("Distinct visible graphemes: 1"));
    assert!(large.contains("[U+2009]"));
}

#[test]
fn initial_source_bom_is_not_ink_in_unicode_coverage_reports() {
    let sample = "\u{feff}\u{f1}\u{2028}n\u{303}";
    for report in [
        execute_glyph_library(&args(&["--audit-text", sample])),
        execute_handwriting(&args(&["--inventory-text", sample])),
    ] {
        let text = String::from_utf8(report.expect("BOM source admitted")
            .expect("read-only report"))
            .expect("UTF-8 report");
        assert!(text.contains("Distinct visible graphemes: 2"),
            "{text}");
        assert!(!text.contains("U+FEFF"), "leading BOM is not ink: {text}");
        assert!(!text.contains("U+2028"), "separator is not ink: {text}");
    }

    // BOM is metadata only at byte zero. A later exact U+FEFF grapheme
    // is not silently normalized away, even on a BOM-prefixed source.
    for sample in ["n\u{feff}n", "\u{feff}\u{feff}n"] {
        for report in [
            execute_glyph_library(&args(&["--audit-text", sample])),
            execute_handwriting(&args(&["--inventory-text", sample])),
        ] {
            let text = String::from_utf8(report.expect("source admitted")
                .expect("read-only report"))
                .expect("UTF-8 report");
            assert!(text.contains("Distinct visible graphemes: 2"),
                "{text}");
            assert!(text.contains("U+FEFF"),
                "interior Unicode identity retained: {text}");
        }
    }

    for structural in ["\u{feff}", "\u{feff}\u{2028}",
        "\u{feff} \u{2029}\t\r\n"] {
        assert!(execute_glyph_library(&args(&[
            "--audit-text", structural,
        ])).is_err(), "BOM-only glyph audit: {structural:?}");
        assert!(execute_handwriting(&args(&[
            "--inventory-text", structural,
        ])).is_err(), "BOM-only inventory: {structural:?}");
    }

    // The original UTF-8 byte budget includes even an ignored BOM.
    let accepted = format!("\u{feff}{}", "a".repeat(4093));
    let refused = format!("\u{feff}{}", "a".repeat(4094));
    for flag in ["--audit-text", "--inventory-text"] {
        let result = if flag == "--audit-text" {
            execute_glyph_library(&args(&[flag, &accepted]))
        } else {
            execute_handwriting(&args(&[flag, &accepted]))
        };
        assert!(result.is_ok(), "admitted 4096 bytes: {flag}");
        let result = if flag == "--audit-text" {
            execute_glyph_library(&args(&[flag, &refused]))
        } else {
            execute_handwriting(&args(&[flag, &refused]))
        };
        assert!(result.is_err(), "refuse 4097 bytes: {flag}");
    }
}

#[test]
fn unicode_line_and_paragraph_separators_are_audit_structure() {
    // Native flow normalizes these Unicode separators into physical rows,
    // not missing or outside-baseline authored glyphs.
    let source = "\u{f1}\u{2028}n\u{303}\u{2029}\u{f1}";
    let glyph = String::from_utf8(execute_glyph_library(&args(&[
        "--audit-text", source,
    ])).expect("Unicode separators may delimit an audit")
        .expect("read-only glyph report"))
        .expect("UTF-8 report");
    assert!(glyph.contains("Distinct visible graphemes: 2"), "{glyph}");
    assert!(glyph.contains("Missing authored glyphs: 2"), "{glyph}");
    for codepoint in ["U+00F1", "U+006E U+0303"] {
        assert!(glyph.contains(codepoint), "{glyph}");
    }
    for boundary in ["U+2028", "U+2029"] {
        assert!(!glyph.contains(boundary), "{glyph}");
    }

    let inventory = String::from_utf8(execute_handwriting(&args(&[
        "--inventory-text", source,
    ])).expect("Unicode separators may delimit an inventory")
        .expect("read-only inventory report"))
        .expect("UTF-8 report");
    assert!(inventory.contains("Distinct visible graphemes: 2"),
        "{inventory}");
    assert!(inventory.contains("Outside English/Spanish text baseline: 0"),
        "{inventory}");
    for boundary in ["U+2028", "U+2029"] {
        assert!(!inventory.contains(boundary), "{inventory}");
    }

    // Unicode spaces may have exact authored strokes; unlike a pasted
    // line or paragraph separator, they must stay in both reports.
    let spaced = "a\u{2028}\u{a0}\u{2029}\u{2009}";
    for report in [
        execute_glyph_library(&args(&["--audit-text", spaced])),
        execute_handwriting(&args(&["--inventory-text", spaced])),
    ] {
        let printed = String::from_utf8(report.expect("spaced source admitted")
            .expect("read-only report"))
            .expect("UTF-8 report");
        assert!(printed.contains("Distinct visible graphemes: 3"),
            "{printed}");
        assert!(printed.contains("U+00A0"), "{printed}");
        assert!(printed.contains("U+2009"), "{printed}");
    }

    for structural in ["\u{2028}", "\u{2029}",
        " \u{2028}\r\n\u{2029}\t"] {
        assert!(execute_glyph_library(&args(&[
            "--audit-text", structural,
        ])).is_err(), "empty glyph audit: {structural:?}");
        assert!(execute_handwriting(&args(&[
            "--inventory-text", structural,
        ])).is_err(), "empty inventory: {structural:?}");
    }
}

#[test]
fn unicode_coverage_audits_reject_nonstructural_control_breaks() {
    // An otherwise visible source must not conceal control breaks that the
    // native placed-text compositor refuses. CRLF, LF and tabs are structure.
    for text in [
        "a\u{85}b", "a\u{b}b", "a\u{c}b", "a\rb",
        "\ra", "a\r", "a\r\r\nb",
    ] {
        let glyph = execute_glyph_library(&args(&[
            "--audit-text", &text,
        ]));
        assert!(glyph.is_err(), "glyph audit admitted {text:?}");
        let inventory = execute_handwriting(&args(&[
            "--inventory-text", &text,
        ]));
        assert!(inventory.is_err(), "inventory admitted {text:?}");
    }
    for flag in ["--audit-text", "--inventory-text"] {
        let result = if flag == "--audit-text" {
            execute_glyph_library(&args(&[flag, "a\r\nb\tc"]))
        } else {
            execute_handwriting(&args(&[flag, "a\r\nb\tc"]))
        }.expect("CR/LF/tab source structure accepted")
            .expect("coverage report");
        let report = String::from_utf8(result).expect("readable report");
        assert!(report.contains("Distinct visible graphemes: 3"),
            "{flag}: {report}");
    }
}

#[test]
fn inventory_cli_refuses_unsafe_or_conflicting_requests_without_file_effects() {
    let target = test_path();
    fs::write(&target, b"unchanged").expect("write owned sentinel");
    let filename = target.to_str().expect("UTF-8 path");
    for invalid in [
        args(&["--inventory-text"]),
        args(&["--inventory-text", ""]),
        args(&["--inventory-text", " "]),
        args(&["--inventory-text", "\t"]),
        args(&["--inventory-text", " \t\n"]),
        args(&["--inventory-text", "\u{85}"]),
        args(&["--inventory-text", "\u{b}"]),
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
            "--pages",
            "1",
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
fn long_exact_unicode_audit_reads_each_decomposed_grapheme() {
    let mut commands = args(&[
        "--glyph-stroke", "ó", "250", "500", "1500", "3200",
        "250", "#113355",
        "--glyph-stroke", "o\u{301}", "250", "500", "1500", "3200",
        "250", "#224466",
        "--glyph-stroke", "a", "250", "500", "1500", "3200",
        "250", "#335577",
        "--glyph-stroke", "👩\u{200d}🏫", "250", "500", "1500",
        "3200", "250", "#446688",
    ]);
    let source = format!(
        "{}óa👩\u{200d}🏫", "o\u{301}".repeat(1_024),
    );
    commands.extend(args(&["--audit-text", &source]));
    let report = execute_glyph_library(&commands)
        .expect("long bounded audit accepted").expect("audit report");
    let report = String::from_utf8(report).expect("UTF-8 glyph audit");
    assert!(report.contains("Distinct visible graphemes: 4"), "{report}");
    assert!(report.contains("Missing authored glyphs: 0"), "{report}");
    for glyph in ["ó", "o\u{301}", "a"] {
        assert!(report.contains(glyph), "exact {glyph} preserved");
    }
    assert!(report.contains("U+1F469 U+200D U+1F3EB"),
        "joined Unicode scalars remain one exact glyph");
}

#[test]
fn unicode_suffix_placement_matches_explicit_pdf_and_png_strokes() {
    let sample = "óo\u{301}a👩\u{200d}🏫 ".repeat(7);
    let shapes = [
        ("ó", "#113355"), ("o\u{301}", "#224466"),
        ("a", "#335577"), ("👩\u{200d}🏫", "#446688"),
    ];
    for (format, opts) in [
        ("pdf", vec![]), ("png", args(&["--dpi", "72"])),
    ] {
        let mut glyph = args(&[
            "--printed-sheet", "true", "--export-paper", format,
        ]);
        glyph.extend(opts.clone());
        let mut manual = glyph.clone();
        for (key, color) in shapes {
            glyph.extend(args(&[
                "--glyph-stroke", key, "250", "500", "1500",
                "3200", "250", color,
            ]));
        }
        for row in 0..7 {
            for (slot, (_, color)) in shapes.iter().enumerate() {
                let position = 25_000 + (row * 5 + slot) * 4_000;
                manual.extend(args(&[
                    "--ink-line", "1", &(position + 250).to_string(),
                    "30500", &(position + 1_500).to_string(), "33200",
                    "250", color,
                ]));
            }
        }
        glyph.extend(args(&[
            "--place-text", "1", "25000", "30000", &sample,
        ]));
        let output = execute_notebook(&glyph)
            .expect("exact Unicode native placement")
            .expect("physical export");
        let manual_ink = execute_notebook(&manual)
            .expect("manual native strokes").expect("physical export");
        assert!(output == manual_ink,
            "{format}: placement {} bytes differs from manual {} bytes",
            output.len(), manual_ink.len());
    }
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
    for _ in 0..43 {
        excessive.extend(args(&[
            "--glyph-stroke", "ñ", "0", "0", "3000", "3000",
            "250", "#123456",
        ]));
    }
    excessive.extend(args(&[
        "--place-text", "1", "25000", "30000", &"ñ".repeat(96),
        "--output", name, "--replace",
    ]));
    let error = execute_notebook(&excessive).expect_err("more than 4096");
    assert!(error.contains("4096"), "{error}");
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
        b"atrament.glyph-strokes.v1\n".to_vec(),
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
fn large_unicode_glyph_audit_keeps_exact_source_order_and_counts()
-> Result<(), String> {
    // 1024 distinct keys and 4096 interleaved authored strokes exercise
    // both report uniqueness and the full admitted inventory boundary.
    // Exact NFD, NFC and an ink-bearing marked space are independent keys.
    let mut keys = vec![
        "ñ".to_owned(), "n\u{303}".to_owned(), " \u{301}".to_owned(),
    ];
    for ordinal in 0..1021u32 {
        keys.push(char::from_u32(0x4E00 + ordinal)
            .ok_or("missing unique test CJK scalar")?.to_string());
    }
    let mut overlay = GlyphOverlay::default();
    for stroke_ordinal in 0..4usize {
        for key in &keys {
            overlay.strokes.push(GlyphStroke {
                grapheme: key.clone(),
                start_um: (500, 600 + u32::try_from(stroke_ordinal)
                    .map_err(|error| error.to_string())? * 100),
                end_um: (1800, 3000),
                width_um: 250,
                rgb: [18, 52, 86],
            });
        }
    }
    let mut source = keys.concat();
    source.push_str(&keys.iter().rev().take(256)
        .map(String::as_str).collect::<String>());
    source.push('¿');
    if source.len() > 4096 || overlay.strokes.len() != 4096 {
        return Err("test exceeded native audit bounds".into());
    }
    let report = String::from_utf8(overlay.audit_text(&source)?)
        .map_err(|error| error.to_string())?;
    let mut expected = String::from(concat!(
        "Exact authored glyph audit (not a font or sampler)\n",
        "Distinct visible graphemes: 1025\n",
        "Missing authored glyphs: 1\n",
    ));
    for key in &keys {
        let codes = key.chars().map(|ch| format!(
            "U+{:04X}", u32::from(ch),
        )).collect::<Vec<_>>().join(" ");
        expected.push_str(&format!(
            "\"{}\" [{codes}]: 4 authored strokes\n",
            key.escape_debug(),
        ));
    }
    expected.push_str("\"¿\" [U+00BF]: missing authored strokes\n");
    if report != expected {
        return Err("large exact audit lost first-use order or counts".into());
    }
    if overlay.audit_text(&source)? != report.as_bytes() {
        return Err("large exact audit is not reproducible".into());
    }
    Ok(())
}

#[test]
fn saved_full_unicode_glyph_library_audit_replays_exact_cli_report()
-> Result<(), String> {
    // Exercise the real TSV loader and public stdout audit, not only the
    // typed in-memory overlay. An exact accented pair, a visible base with
    // an attached nonmarking mark and its bare base must remain distinct.
    let keys = ["n\u{303}", "ñ", "a\u{34f}", "a"];
    let mut source = String::from("atrament.glyph-strokes.v1\n");
    for _ in 0..1024 {
        for key in keys {
            source.push_str(&format!(
                "{key}\t500\t500\t1800\t3000\t250\t#123456\n"
            ));
        }
    }
    let path = test_path().with_extension("tsv");
    fs::write(&path, source.as_bytes())
        .map_err(|error| error.to_string())?;
    let file = path.to_str().ok_or("invalid test-owned library path")?;
    let selected = format!(
        "{} {} {} {} ∞ {}",
        keys[0], keys[1], keys[2], keys[3], keys[0],
    );
    let mut expected = String::from(concat!(
        "Exact authored glyph audit (not a font or sampler)\n",
        "Distinct visible graphemes: 5\n",
        "Missing authored glyphs: 1\n",
    ));
    for key in keys {
        let codepoints = key.chars().map(|ch| {
            format!("U+{:04X}", u32::from(ch))
        }).collect::<Vec<_>>().join(" ");
        expected.push_str(&format!(
            "\"{}\" [{codepoints}]: 1024 authored strokes\n",
            key.escape_debug(),
        ));
    }
    expected.push_str("\"∞\" [U+221E]: missing authored strokes\n");
    for selected in [selected.clone(), format!("\u{feff}{selected}")] {
        let report = execute_glyph_library(&args(&[
            "--profile", file, "--audit-text", &selected,
        ]))?.ok_or("read-only CLI audit returned no report")?;
        if report != expected.as_bytes() {
            return Err("full TSV CLI audit lost exact report ordering".into());
        }
    }
    let reopened = execute_glyph_library(&args(&["--profile", file]))?
        .ok_or("real glyph TSV readback returned no bytes")?;
    if reopened != source.as_bytes()
        || fs::read(&path).map_err(|error| error.to_string())?
            != source.as_bytes() {
        return Err("read-only audit changed selected TSV bytes".into());
    }
    fs::remove_file(path).map_err(|error| error.to_string())?;
    Ok(())
}

#[test]
fn glyph_audit_accepts_whitespace_only_when_a_glyph_can_be_authored() {
    for glyph in ["\u{a0}", "\u{2009}", "\u{202f}"] {
        let unicode = u32::from(glyph.chars().next().expect("one codepoint"));
        let missing = String::from_utf8(execute_glyph_library(&args(&[
            "--audit-text", glyph,
        ])).expect("Unicode-only source auditable")
            .expect("read-only glyph audit"))
            .expect("UTF-8 output");
        assert!(missing.contains("Distinct visible graphemes: 1"),
            "{missing}");
        assert!(missing.contains("Missing authored glyphs: 1"),
            "{missing}");
        assert!(missing.contains(&format!("[U+{unicode:04X}]")),
            "{missing}");

        let authored = String::from_utf8(execute_glyph_library(&args(&[
            "--glyph-stroke", glyph, "500", "500", "1800", "3000",
            "250", "#123456", "--audit-text", glyph,
        ])).expect("single exact space shape auditable")
            .expect("read-only glyph audit"))
            .expect("UTF-8 output");
        assert!(authored.contains("Missing authored glyphs: 0"),
            "{authored}");
    }
    for structural in [" ", "\t", "\n", " \n\t ", "\u{85}", "\u{b}"] {
        let error = execute_glyph_library(&args(&[
            "--audit-text", structural,
        ])).expect_err("implicit source spaces are not authored ink");
        assert!(error.contains("visible text"), "{error}");
    }
}

#[test]
fn structural_unicode_separators_cannot_be_authored_as_glyph_ink() {
    // Read-only reports and --flow-markdown treat these as row structure.
    // They must not become hidden physical pen marks via --place-text.
    for separator in ["\u{2028}", "\u{2029}"] {
        let stroke = [
            "--glyph-stroke", separator, "500", "500", "1800",
            "3000", "250", "#123456",
        ];
        assert!(execute_glyph_library(&args(&stroke)).is_err(),
            "source row separators are not authorable visible glyphs");

        let target = test_path().with_extension("pdf");
        fs::write(&target, b"existing selected PDF")
            .expect("test-owned output sentinel");
        let mut command = args(&[
            "--export-paper", "pdf", "--printed-sheet", "false",
            "--glyph-stroke", "a", "500", "500", "1800", "3000",
            "250", "#123456",
        ]);
        command.extend(args(&stroke));
        command.extend(args(&[
            "--place-text", "1", "30000", "40000",
            &format!("a{separator}a"), "--output",
            target.to_str().expect("target path"), "--replace",
        ]));
        assert!(execute_notebook(&command).is_err(),
            "invisible row separators cannot create physical glyph ink");
        let text = format!("a{separator}a");
        let place_without_separator_shape = args(&[
            "--export-paper", "pdf", "--printed-sheet", "false",
            "--glyph-stroke", "a", "500", "500", "1800", "3000",
            "250", "#123456", "--place-text", "1", "30000",
            "40000", &text, "--output",
            target.to_str().expect("target path"), "--replace",
        ]);
        let error = execute_notebook(&place_without_separator_shape)
            .expect_err("row separators must refuse before glyph lookup");
        assert!(error.contains("structural separators"), "{error}");
        assert_eq!(fs::read(&target).expect("unchanged target"),
            b"existing selected PDF");
        fs::remove_file(&target).expect("remove own target");
    }
}

#[test]
fn saved_glyph_library_rejects_structural_separators_before_pdf_replacement() {
    let source = test_path().with_extension("tsv");
    let target = test_path().with_extension("pdf");
    let filename = source.to_str().expect("selected library path");
    let destination = target.to_str().expect("selected PDF path");
    fs::write(&target, b"previous accepted document")
        .expect("test-owned PDF sentinel");
    for separator in ["\u{2028}", "\u{2029}"] {
        let bytes = format!(
            concat!(
                "atrament.glyph-strokes.v1\n",
                "{}\t500\t500\t1800\t3000\t250\t#123456\n",
            ),
            separator
        );
        fs::write(&source, bytes.as_bytes())
            .expect("test-owned selected TSV");
        let refused = execute_glyph_library(&args(&[
            "--profile", filename, "--audit-text", "a",
        ])).expect_err("structural separator TSV cannot be loaded");
        assert!(refused.contains("visible grapheme"), "{refused}");
        let error = execute_notebook(&args(&[
            "--export-paper", "pdf", "--printed-sheet", "false",
            "--glyph-file", filename, "--place-text", "1",
            "30000", "40000", "a", "--output", destination,
            "--replace",
        ])).expect_err("invalid TSV must not reach PDF rendering");
        assert!(error.contains("visible grapheme"), "{error}");
        assert_eq!(fs::read(&target).expect("unchanged PDF"),
            b"previous accepted document");
        assert_eq!(fs::read(&source).expect("unchanged TSV"),
            bytes.as_bytes());
    }
    fs::remove_file(&source).expect("remove test-owned TSV");
    fs::remove_file(&target).expect("remove test-owned PDF");
}

#[test]
fn reserved_or_nonmarking_unicode_keys_cannot_name_ink() {
    let source = test_path().with_extension("tsv");
    let target = test_path().with_extension("pdf");
    let filename = source.to_str().expect("selected TSV path");
    let destination = target.to_str().expect("selected PDF path");
    fs::write(&target, b"existing selected PDF")
        .expect("write test-owned PDF sentinel");
    for invisible in [
        "\u{00ad}", "\u{034f}", "\u{061c}", "\u{180e}",
        "\u{0600}", "\u{0605}", "\u{06dd}",
        "\u{070f}", "\u{0890}", "\u{0891}", "\u{08e2}",
        "\u{110bd}", "\u{110cd}", "\u{13430}", "\u{1343f}",
        "\u{13440}", "\u{13441}", "\u{13442}",
        "\u{13447}", "\u{13455}",
        "\u{200b}", "\u{200c}", "\u{200d}", "\u{200e}",
        "\u{200f}", "\u{202a}", "\u{202e}", "\u{2060}",
        "\u{2061}", "\u{2062}", "\u{2063}", "\u{2064}",
        "\u{2066}", "\u{2069}", "\u{206f}", "\u{fe00}",
        "\u{fe0e}", "\u{fe0f}", "\u{feff}",
        "\u{e0100}", "\u{e01ef}", "\u{115f}", "\u{1160}",
        "\u{17b4}", "\u{17b5}", "\u{180b}", "\u{180f}",
        "\u{3164}", "\u{ffa0}", "\u{2800}", "\u{fff0}", "\u{fff8}",
        "\u{fff9}", "\u{fffa}", "\u{fffb}",
        "\u{1bca0}", "\u{1bca3}", "\u{1d173}", "\u{1d17a}",
        "\u{e0001}", "\u{e0067}", "\u{e007f}",
        "\u{fdd0}", "\u{fdef}", "\u{fffe}", "\u{ffff}",
        "\u{1fffe}", "\u{1ffff}", "\u{10fffe}", "\u{10ffff}",
        "\u{fdd0}\u{301}", "\u{1fffe}\u{e0100}",
        "\u{fe0e}\u{fe0f}", "\u{200d}\u{fe0f}",
        "\u{2800}\u{fe0f}", "\u{0600}\u{200d}",
        "\u{034f}\u{fe0e}", "\u{e0067}\u{e007f}",
    ] {
        let ink = [
            "--glyph-stroke", invisible, "500", "500", "1800",
            "3000", "250", "#123456",
        ];
        let error = execute_glyph_library(&args(&ink))
            .expect_err("reserved or nonmarking key cannot draw ink");
        assert!(error.contains("visible grapheme"), "{error}");
        let bytes = format!(
            concat!(
                "atrament.glyph-strokes.v1\n",
                "{}\t500\t500\t1800\t3000\t250\t#123456\n",
            ),
            invisible
        );
        fs::write(&source, bytes.as_bytes())
            .expect("write test-owned invalid glyph library");
        let error = execute_glyph_library(&args(&[
            "--profile", filename,
        ])).expect_err("native TSV import must refuse hidden glyph keys");
        assert!(error.contains("visible grapheme"), "{error}");
        let error = execute_notebook(&args(&[
            "--export-paper", "pdf", "--printed-sheet", "false",
            "--glyph-file", filename, "--place-text", "1",
            "30000", "40000", "a", "--output", destination,
            "--replace",
        ])).expect_err("invalid TSV must not replace selected PDF");
        assert!(error.contains("visible grapheme"), "{error}");
        assert_eq!(fs::read(&target).expect("unchanged PDF"),
            b"existing selected PDF");
        assert_eq!(fs::read(&source).expect("unchanged TSV"),
            bytes.as_bytes());
    }
    for visible in ["👩\u{200d}💻", "❤\u{fe0f}",
        "\u{0600}\u{0661}", "\u{06dd}\u{0661}",
        "\u{0890}\u{0661}", "\u{0891}\u{0661}",
        "a\u{034f}", "a\u{e0100}", "⠁", "\u{2800}\u{301}",
        "\u{13000}\u{13440}", "\u{13000}\u{13447}",
        "\u{13443}", "\u{e000}", "\u{10fffd}",
        "\u{fffd}", "\u{fdcf}", "\u{fdf0}",
        "🏴\u{e0067}\u{e0062}\u{e0065}\u{e006e}\u{e0067}\u{e007f}",
        "\u{a0}"] {
        let text = execute_glyph_library(&args(&[
            "--glyph-stroke", visible, "500", "500", "1800",
            "3000", "250", "#123456",
        ]));
        assert!(text.is_ok(), "visible grapheme refused: {visible:?}");
    }
    fs::remove_file(&source).expect("remove own TSV");
    fs::remove_file(&target).expect("remove own PDF");
}

#[test]
fn every_unicode_noncharacter_refuses_direct_glyph_authoring() {
    let mut reserved = (0xfdd0..=0xfdef).collect::<Vec<u32>>();
    for plane in 0..=16 {
        reserved.extend([plane * 0x10000 + 0xfffe,
            plane * 0x10000 + 0xffff]);
    }
    assert_eq!(reserved.len(), 66);
    for scalar in reserved {
        let key = char::from_u32(scalar)
            .expect("reserved scalar is in Unicode range").to_string();
        let error = execute_glyph_library(&args(&[
            "--glyph-stroke", &key, "500", "500", "1800",
            "3000", "250", "#123456",
        ])).expect_err("noncharacters cannot name authored glyph ink");
        assert!(error.contains("visible grapheme"), "U+{scalar:04X}: {error}");
    }
}

#[test]
fn glyph_audit_reports_unicode_whitespace_requiring_authored_ink() {
    let sample = "a\u{a0}a\u{2009}a\u{202f}a";
    let base = args(&[
        "--glyph-stroke", "a", "500", "500", "1800", "3000",
        "250", "#123456",
    ]);
    let mut audit_command = base.clone();
    audit_command.extend(args(&["--audit-text", sample]));
    let bytes = execute_glyph_library(&audit_command)
        .expect("non-ASCII spaces are auditable").expect("report bytes");
    let report = String::from_utf8(bytes).expect("audit is UTF-8");
    assert!(report.contains("Distinct visible graphemes: 4"), "{report}");
    assert!(report.contains("Missing authored glyphs: 3"), "{report}");
    for codepoint in ["U+00A0", "U+2009", "U+202F"] {
        assert!(report.contains(&format!(
            "[{codepoint}]: missing authored strokes"
        )), "missing exact space {codepoint}: {report}");
    }
    let mut ascii_audit = base.clone();
    ascii_audit.extend(args(&["--audit-text", "a a\ta\na"]));
    let ascii = execute_glyph_library(&ascii_audit)
        .expect("source whitespace remains auditable")
        .expect("report bytes");
    let ascii = String::from_utf8(ascii).expect("ASCII report");
    assert!(ascii.contains("Missing authored glyphs: 0"), "{ascii}");

    for format in ["pdf", "png"] {
        let target = test_path().with_extension(format);
        let path = target.to_str().expect("test output path");
        fs::write(&target, b"accepted native output sentinel")
            .expect("test-owned output");
        let mut export = args(&[
            "--export-paper", format, "--printed-sheet", "false",
            "--glyph-stroke", "a", "500", "500", "1800", "3000",
            "250", "#123456", "--place-text", "1", "30000", "40000",
            sample, "--output", path, "--replace",
        ]);
        if format == "png" {
            export.extend(args(&["--png-page", "1", "--dpi", "72"]));
        }
        let error = execute_notebook(&export)
            .expect_err("unauthored Unicode space must refuse");
        for codepoint in ["U+00A0", "U+2009", "U+202F"] {
            assert!(error.contains(codepoint), "{error}");
        }
        assert_eq!(fs::read(&target).expect("retained output"),
            b"accepted native output sentinel");
        fs::remove_file(&target).expect("remove test-owned output");
    }
}

#[test]
fn explicitly_authored_unicode_whitespace_has_real_native_pdf_ink() {
    // These exact Unicode spaces may deliberately carry caller-authored ink.
    let sample = "a\u{a0}a\u{2009}a\u{202f}a";
    let base = args(&[
        "--glyph-stroke", "a", "500", "500", "1800", "3000",
        "250", "#123456",
    ]);
    // The audit must count them, not normalize them to ASCII advances.
    let mut authored = base;
    for whitespace in ["\u{a0}", "\u{2009}", "\u{202f}"] {
        authored.extend(args(&[
            "--glyph-stroke", whitespace, "500", "500", "1800",
            "3000", "250", "#123456",
        ]));
    }
    let mut authored_audit = authored.clone();
    authored_audit.extend(args(&["--audit-text", sample]));
    let report = execute_glyph_library(&authored_audit)
        .expect("independently authored whitespace audited")
        .expect("report bytes");
    assert!(String::from_utf8_lossy(&report)
        .contains("Missing authored glyphs: 0"));
    let mut accepted = args(&[
        "--export-paper", "pdf", "--printed-sheet", "false",
    ]);
    accepted.extend(authored);
    accepted.extend(args(&[
        "--place-text", "1", "30000", "40000", sample,
    ]));
    let pdf = execute_notebook(&accepted)
        .expect("selected exact Unicode whitespace shapes render")
        .expect("real PDF bytes");
    assert!(pdf.starts_with(b"%PDF-1.4"));
    assert_eq!(String::from_utf8_lossy(&pdf)
        .matches("0.071 0.204 0.337 RG").count(), 7,
        "four authored letters and three exact Unicode space strokes");
}

#[test]
fn path_inspector_groups_only_exact_directed_same_pen_segments() {
    let file = test_path().with_extension("tsv");
    let name = file.to_str().expect("owned TSV");
    let strokes = args(&[
        "--glyph-stroke", "n\u{303}", "100", "200", "200", "300",
        "250", "#102030",
        "--glyph-stroke", "ñ", "100", "100", "200", "200",
        "300", "#AABBCC",
        "--glyph-stroke", "n\u{303}", "200", "300", "400", "500",
        "250", "#102030",
        "--glyph-stroke", "n\u{303}", "400", "500", "450", "600",
        "300", "#102030",
        "--glyph-stroke", "n\u{303}", "450", "600", "500", "700",
        "300", "#102030",
        "--glyph-stroke", "n\u{303}", "550", "700", "600", "800",
        "300", "#102030",
        "--glyph-stroke", "n\u{303}", "600", "800", "700", "900",
        "300", "#304050",
    ]);
    let library = execute_glyph_library(&strokes)
        .expect("authored library").expect("canonical TSV");
    fs::write(&file, &library).expect("own source file");
    let report = String::from_utf8(execute_glyph_library(&args(&[
        "--profile", name, "--inspect-paths", "n\u{303}",
    ])).expect("read-only path inspection").expect("stdout report"))
        .expect("UTF-8 report");
    let expected = concat!(
        "Candidate continuous pen paths (exact authored order)\n",
        "Glyph: \"ñ\" [U+006E U+0303]\n",
        "Strokes: 6\nPaths: 4\nPen lifts: 3\n",
        "Path 1: strokes 1..2 (2 segments) ",
        "(100, 200) -> (400, 500) width 250 um color #102030\n",
        "Path 2: strokes 3..4 (2 segments) ",
        "(400, 500) -> (500, 700) width 300 um color #102030\n",
        "Path 3: strokes 5..5 (1 segment) ",
        "(550, 700) -> (600, 800) width 300 um color #102030\n",
        "Path 4: strokes 6..6 (1 segment) ",
        "(600, 800) -> (700, 900) width 300 um color #304050\n",
    );
    assert_eq!(report, expected);
    let mut inline = strokes.clone();
    inline.extend(args(&["--inspect-paths", "n\u{303}"]));
    assert_eq!(execute_glyph_library(&inline)
        .expect("inline inspector").expect("report"),
        report.as_bytes());
    let nfc = String::from_utf8(execute_glyph_library(&args(&[
        "--profile", name, "--inspect-paths", "ñ",
    ])).expect("NFC paths").expect("report"))
        .expect("UTF-8");
    assert!(nfc.contains("Glyph: \"ñ\" [U+00F1]"), "{nfc}");
    assert!(nfc.contains("Strokes: 1\nPaths: 1\nPen lifts: 0"));
    assert!(!nfc.contains("(400, 500)"));
    assert_eq!(fs::read(&file).expect("canonical unchanged"), library);
    fs::remove_file(file).expect("remove owned source");
}

#[test]
fn explicit_join_and_reversal_change_candidate_path_boundaries() {
    let base = args(&[
        "--glyph-stroke", "n\u{303}", "500", "600", "800",
        "1000", "250", "#102030",
        "--glyph-stroke", "ñ", "1000", "1000", "2000",
        "2000", "300", "#AABBCC",
        "--glyph-stroke", "n\u{303}", "1100", "1200", "1600",
        "1500", "250", "#102030",
    ]);
    let mut report = base.clone();
    report.extend(args(&["--inspect-paths", "n\u{303}"]));
    let before = String::from_utf8(execute_glyph_library(&report)
        .expect("unconnected exact glyph").expect("report"))
        .expect("UTF-8");
    assert!(before.contains("Strokes: 2\nPaths: 2\nPen lifts: 1"));
    let mut connected = base.clone();
    connected.extend(args(&[
        "--join-strokes", "n\u{303}", "1",
        "--inspect-paths", "n\u{303}",
    ]));
    let joined = String::from_utf8(execute_glyph_library(&connected)
        .expect("explicit joined glyph").expect("report"))
        .expect("UTF-8");
    assert!(joined.contains("Strokes: 3\nPaths: 1\nPen lifts: 0"));
    assert!(joined.contains("Path 1: strokes 1..3 (3 segments)"));
    let mut reversed = base;
    reversed.extend(args(&[
        "--join-strokes", "n\u{303}", "1",
        "--reverse-stroke", "n\u{303}", "2",
        "--inspect-paths", "n\u{303}",
    ]));
    let inverted = String::from_utf8(execute_glyph_library(&reversed)
        .expect("directed inverse").expect("report"))
        .expect("UTF-8");
    assert!(inverted.contains("Strokes: 3\nPaths: 3\nPen lifts: 2"));
    let mut moved = args(&[
        "--glyph-stroke", "n\u{303}", "500", "600", "800",
        "1000", "250", "#102030",
        "--glyph-stroke", "n\u{303}", "1100", "1200", "1600",
        "1500", "250", "#102030",
    ]);
    moved.extend(args(&[
        "--join-strokes", "n\u{303}", "1",
        "--move-stroke", "n\u{303}", "3", "1",
        "--inspect-paths", "n\u{303}",
    ]));
    let reordered = String::from_utf8(execute_glyph_library(&moved)
        .expect("reorder authored pen sequence").expect("report"))
        .expect("UTF-8");
    assert!(reordered.contains("Strokes: 3\nPaths: 2\nPen lifts: 1"));
    assert!(reordered.contains("Path 2: strokes 2..3 (2 segments)"));
}

#[test]
fn native_path_edits_reorder_and_reverse_full_exact_unicode_groups() {
    let source = test_path().with_extension("tsv");
    let revised = test_path().with_extension("tsv");
    let source_name = source.to_str().expect("test source");
    let revised_name = revised.to_str().expect("test output");
    let original = execute_glyph_library(&args(&[
        "--glyph-stroke", "n\u{303}", "100", "100", "400", "300",
        "250", "#102030",
        "--glyph-stroke", "ñ", "500", "600", "700", "800",
        "350", "#AABBCC",
        "--glyph-stroke", "n\u{303}", "400", "300", "700", "500",
        "250", "#102030",
        "--glyph-stroke", "∞", "500", "1000", "900", "1200",
        "250", "#223344",
        "--glyph-stroke", "n\u{303}", "900", "800", "1100", "1200",
        "300", "#334455",
        "--glyph-stroke", "n\u{303}", "1100", "1200", "1300", "1600",
        "300", "#334455",
        "--glyph-stroke", "n\u{303}", "1300", "1600", "1500", "1800",
        "300", "#334455",
        "--glyph-stroke", "ñ", "900", "1000", "1200", "1300",
        "350", "#AABBCC",
        "--glyph-stroke", "n\u{303}", "2000", "2000", "2300", "2400",
        "400", "#667788",
    ])).expect("initial glyph library").expect("canonical TSV");
    fs::write(&source, &original).expect("source file");
    let mut edited = args(&[
        "--profile", source_name, "--move-path", "n\u{303}", "1", "2",
        "--reverse-path", "n\u{303}", "2",
    ]);
    let actual = execute_glyph_library(&edited)
        .expect("complete path edit").expect("new TSV");
    let expected = concat!(
        "atrament.glyph-strokes.v1\n",
        "ñ\t900\t800\t1100\t1200\t300\t#334455\n",
        "ñ\t500\t600\t700\t800\t350\t#AABBCC\n",
        "ñ\t1100\t1200\t1300\t1600\t300\t#334455\n",
        "∞\t500\t1000\t900\t1200\t250\t#223344\n",
        "ñ\t1300\t1600\t1500\t1800\t300\t#334455\n",
        "ñ\t700\t500\t400\t300\t250\t#102030\n",
        "ñ\t400\t300\t100\t100\t250\t#102030\n",
        "ñ\t900\t1000\t1200\t1300\t350\t#AABBCC\n",
        "ñ\t2000\t2000\t2300\t2400\t400\t#667788\n",
    );
    assert_eq!(actual, expected.as_bytes(),
        "all interleaved NFC/math slots and complete path interiors remain");
    let mut inspected = edited.clone();
    inspected.extend(args(&["--inspect-paths", "n\u{303}"]));
    let report = String::from_utf8(execute_glyph_library(&inspected)
        .expect("inspector after edit").expect("report"))
        .expect("UTF-8 report");
    assert!(report.contains("Strokes: 6\nPaths: 3\nPen lifts: 2"),
        "{report}");
    assert!(report.contains(
        "Path 1: strokes 1..3 (3 segments) (900, 800) -> (1500, 1800)"
    ), "{report}");
    assert!(report.contains(
        "Path 2: strokes 4..5 (2 segments) (700, 500) -> (100, 100)"
    ), "{report}");
    let mut restored = edited.clone();
    restored.extend(args(&[
        "--reverse-path", "n\u{303}", "2",
        "--move-path", "n\u{303}", "2", "1",
    ]));
    assert_eq!(execute_glyph_library(&restored)
        .expect("reversible path edits").expect("TSV"), original);

    edited.extend(args(&["--output", revised_name]));
    assert!(execute_glyph_library(&edited)
        .expect("save edited library").is_none());
    assert_eq!(fs::read(&revised).expect("read revised"), actual);
    assert_eq!(fs::read(&source).expect("original remains"), original);
    assert_eq!(execute_glyph_library(&args(&[
        "--profile", revised_name,
    ])).expect("reload").expect("bytes"), actual);

    for format in ["pdf", "png"] {
        let mut selected = args(&[
            "--export-paper", format, "--printed-sheet", "true",
            "--glyph-file", revised_name,
            "--place-text", "1", "30000", "45000", "n\u{303}ñ∞",
        ]);
        let mut independent = args(&[
            "--export-paper", format, "--printed-sheet", "true",
            "--glyph-stroke", "n\u{303}", "900", "800", "1100",
            "1200", "300", "#334455",
            "--glyph-stroke", "n\u{303}", "1100", "1200", "1300",
            "1600", "300", "#334455",
            "--glyph-stroke", "n\u{303}", "1300", "1600", "1500",
            "1800", "300", "#334455",
            "--glyph-stroke", "n\u{303}", "700", "500", "400",
            "300", "250", "#102030",
            "--glyph-stroke", "n\u{303}", "400", "300", "100",
            "100", "250", "#102030",
            "--glyph-stroke", "n\u{303}", "2000", "2000", "2300",
            "2400", "400", "#667788",
            "--glyph-stroke", "ñ", "500", "600", "700",
            "800", "350", "#AABBCC",
            "--glyph-stroke", "ñ", "900", "1000", "1200",
            "1300", "350", "#AABBCC",
            "--glyph-stroke", "∞", "500", "1000", "900",
            "1200", "250", "#223344",
            "--place-text", "1", "30000", "45000", "n\u{303}ñ∞",
        ]);
        if format == "png" {
            selected.extend(args(&["--dpi", "72"]));
            independent.extend(args(&["--dpi", "72"]));
        }
        assert_eq!(execute_notebook(&selected)
            .expect("saved path render").expect("bytes"),
            execute_notebook(&independent)
            .expect("independent pen lines").expect("bytes"),
            "ordered path edits produce the same physical {format} ink");
    }
    fs::remove_file(source).expect("remove test source");
    fs::remove_file(revised).expect("remove test output");
}

#[test]
fn native_path_edits_refuse_invalid_or_nonadjacent_paths_atomically() {
    let source = test_path().with_extension("tsv");
    let target = test_path().with_extension("tsv");
    let source_name = source.to_str().expect("source path");
    let target_name = target.to_str().expect("destination path");
    let original = execute_glyph_library(&args(&[
        "--glyph-stroke", "n\u{303}", "100", "100", "200", "200",
        "250", "#112233",
        "--glyph-stroke", "ñ", "100", "100", "200", "200",
        "250", "#112233",
        "--glyph-stroke", "n\u{303}", "200", "200", "300", "300",
        "250", "#112233",
        "--glyph-stroke", "n\u{303}", "500", "500", "600", "600",
        "300", "#223344",
        "--glyph-stroke", "n\u{303}", "800", "800", "900", "900",
        "400", "#334455",
    ])).expect("source").expect("TSV");
    fs::write(&source, &original).expect("source file");
    fs::write(&target, b"preserved destination").expect("sentinel");
    for invalid in [
        args(&["--reverse-path"]),
        args(&["--reverse-path", "n\u{303}"]),
        args(&["--reverse-path", "n\u{303}", "0"]),
        args(&["--reverse-path", "n\u{303}", "-1"]),
        args(&["--reverse-path", "n\u{303}", "1.0"]),
        args(&["--reverse-path", "n\u{303}", "4097"]),
        args(&["--reverse-path", "n\u{303}", "4"]),
        args(&["--reverse-path", "n\u{303}", "2"]),
        args(&["--reverse-path", "ñ", "1"]),
        args(&["--reverse-path", "n", "1"]),
        args(&["--reverse-path", "ab", "1"]),
        args(&["--move-path"]),
        args(&["--move-path", "n\u{303}"]),
        args(&["--move-path", "n\u{303}", "1"]),
        args(&["--move-path", "n\u{303}", "1", "1"]),
        args(&["--move-path", "n\u{303}", "1", "3"]),
        args(&["--move-path", "n\u{303}", "0", "1"]),
        args(&["--move-path", "n\u{303}", "1", "4"]),
        args(&["--move-path", "ñ", "1", "2"]),
        args(&["--move-path", "n", "1", "2"]),
        args(&["--move-path", "n\u{303}", "2", "4097"]),
        args(&["--reverse-path", "n\u{303}", "1",
            "--move-path", "n\u{303}", "1", "3"]),
    ] {
        let mut command = args(&["--profile", source_name]);
        command.extend(invalid);
        command.extend(args(&["--output", target_name, "--replace"]));
        assert!(execute_glyph_library(&command).is_err(),
            "invalid path edits must refuse: {command:?}");
        assert_eq!(fs::read(&source).expect("source unchanged"), original);
        assert_eq!(fs::read(&target).expect("target unchanged"),
            b"preserved destination");
    }
    fs::remove_file(source).expect("remove source");
    fs::remove_file(target).expect("remove target");
}

#[test]
fn native_closed_path_start_rotation_preserves_exact_glyph_slots() {
    let nfd = "o\u{301}";
    let source = args(&[
        "--glyph-stroke", nfd, "400", "500", "1400", "500",
        "250", "#223344",
        "--glyph-stroke", "ó", "600", "700", "800", "900",
        "250", "#556677",
        "--glyph-stroke", nfd, "1400", "500", "1400", "1500",
        "250", "#223344",
        "--glyph-stroke", "∞", "500", "600", "700", "800",
        "250", "#111111",
        "--glyph-stroke", nfd, "1400", "1500", "400", "1500",
        "250", "#223344",
        "--glyph-stroke", nfd, "400", "1500", "400", "500",
        "250", "#223344",
        "--glyph-stroke", nfd, "2000", "2000", "2300", "2400",
        "300", "#334455",
    ]);
    let original = execute_glyph_library(&source)
        .expect("initial geometry").expect("TSV");
    let mut rotated = source.clone();
    rotated.extend(args(&["--rotate-path", nfd, "1", "3"]));
    let bytes = execute_glyph_library(&rotated)
        .expect("closed loop rotation").expect("TSV");
    let expected = concat!(
        "atrament.glyph-strokes.v1\n",
        "ó\t1400\t1500\t400\t1500\t250\t#223344\n",
        "ó\t600\t700\t800\t900\t250\t#556677\n",
        "ó\t400\t1500\t400\t500\t250\t#223344\n",
        "∞\t500\t600\t700\t800\t250\t#111111\n",
        "ó\t400\t500\t1400\t500\t250\t#223344\n",
        "ó\t1400\t500\t1400\t1500\t250\t#223344\n",
        "ó\t2000\t2000\t2300\t2400\t300\t#334455\n",
    );
    assert_eq!(bytes, expected.as_bytes(),
        "only closed path starting segment changes, not geometry");
    let mut inspected = rotated.clone();
    inspected.extend(args(&["--inspect-paths", nfd]));
    let report = String::from_utf8(execute_glyph_library(&inspected)
        .expect("inspect rotated paths").expect("report"))
        .expect("UTF-8 paths");
    assert!(report.contains("Strokes: 5\nPaths: 2\nPen lifts: 1"),
        "{report}");
    assert!(report.contains(
        "Path 1: strokes 1..4 (4 segments) (1400, 1500) -> (1400, 1500)"
    ), "{report}");
    assert!(report.contains("width 250 um color #223344 [closed loop]"),
        "{report}");
    assert!(!report.contains("color #334455 [closed loop]"),
        "an open accent must not be labeled as a closed pen loop");
    rotated.extend(args(&["--rotate-path", nfd, "1", "3"]));
    assert_eq!(execute_glyph_library(&rotated)
        .expect("inverse rotation").expect("TSV"), original);

    let source_file = test_path().with_extension("tsv");
    let target_file = test_path().with_extension("tsv");
    fs::write(&source_file, &original).expect("owned input");
    fs::write(&target_file, b"keep prior output").expect("sentinel");
    let input = source_file.to_str().expect("input name");
    let output = target_file.to_str().expect("target name");
    for invalid in [
        args(&["--rotate-path"]),
        args(&["--rotate-path", nfd, "1"]),
        args(&["--rotate-path", nfd, "0", "2"]),
        args(&["--rotate-path", nfd, "1", "0"]),
        args(&["--rotate-path", nfd, "1", "1"]),
        args(&["--rotate-path", nfd, "1", "5"]),
        args(&["--rotate-path", nfd, "2", "2"]),
        args(&["--rotate-path", "ó", "1", "2"]),
        args(&["--rotate-path", "o", "1", "2"]),
        args(&["--rotate-path", "oo", "1", "2"]),
        args(&["--rotate-path", nfd, "1", "2.5"]),
        args(&["--rotate-path", nfd, "1", "4097"]),
    ] {
        let mut command = args(&["--profile", input]);
        command.extend(invalid);
        command.extend(args(&["--output", output, "--replace"]));
        assert!(execute_glyph_library(&command).is_err(),
            "invalid rotation must refuse: {command:?}");
        assert_eq!(fs::read(&source_file).expect("input"), original);
        assert_eq!(fs::read(&target_file).expect("output"),
            b"keep prior output");
    }
    let mut open = source.clone();
    open.extend(args(&[
        "--set-stroke", nfd, "4", "400", "1500", "600", "600",
        "250", "#223344", "--rotate-path", nfd, "1", "2",
    ]));
    assert!(execute_glyph_library(&open).is_err(),
        "an open path cannot be rotated as a closed contour");
    fs::remove_file(source_file).expect("owned input removed");
    fs::remove_file(target_file).expect("owned output removed");
}

#[test]
fn native_explicit_path_closure_preserves_unicode_and_refuses_atomically() {
    let nfd = "o\u{301}";
    let geometry = args(&[
        "--glyph-stroke", nfd, "1500", "2200", "1600", "2300",
        "300", "#112233",
        "--glyph-stroke", "ó", "500", "600", "800", "900",
        "250", "#556677",
        "--glyph-stroke", nfd, "400", "500", "1400", "500",
        "250", "#243647",
        "--glyph-stroke", "∞", "900", "1000", "1200", "1400",
        "250", "#111111",
        "--glyph-stroke", nfd, "1400", "500", "1400", "1500",
        "250", "#243647",
        "--glyph-stroke", nfd, "1400", "1500", "600", "1500",
        "250", "#243647",
    ]);
    let original = execute_glyph_library(&geometry)
        .expect("open source accepted").expect("TSV");
    let mut close = geometry.clone();
    close.extend(args(&["--close-path", nfd, "2"]));
    let result = execute_glyph_library(&close)
        .expect("explicit closure accepted").expect("TSV");
    let expected = concat!(
        "atrament.glyph-strokes.v1\n",
        "ó\t1500\t2200\t1600\t2300\t300\t#112233\n",
        "ó\t500\t600\t800\t900\t250\t#556677\n",
        "ó\t400\t500\t1400\t500\t250\t#243647\n",
        "∞\t900\t1000\t1200\t1400\t250\t#111111\n",
        "ó\t1400\t500\t1400\t1500\t250\t#243647\n",
        "ó\t1400\t1500\t600\t1500\t250\t#243647\n",
        "ó\t600\t1500\t400\t500\t250\t#243647\n",
    );
    assert_eq!(result, expected.as_bytes(),
        "only a terminal same-pen bridge is inserted");
    assert_eq!(execute_glyph_library(&geometry)
        .expect("unchanged source").expect("source TSV"), original);
    let mut report = close.clone();
    report.extend(args(&["--inspect-paths", nfd]));
    let text = String::from_utf8(execute_glyph_library(&report)
        .expect("path inspection").expect("inspection bytes"))
        .expect("UTF-8 report");
    assert!(text.contains("Strokes: 5\nPaths: 2\nPen lifts: 1"),
        "{text}");
    assert!(text.contains(
        "Path 2: strokes 2..5 (4 segments) (400, 500) -> (400, 500)"
    ), "{text}");
    assert!(text.contains("width 250 um color #243647 [closed loop]"),
        "{text}");

    let source_path = test_path().with_extension("tsv");
    let destination_path = test_path().with_extension("tsv");
    fs::write(&source_path, &original).expect("test-owned source");
    fs::write(&destination_path, b"preserved output")
        .expect("test-owned destination");
    let source = source_path.to_str().expect("source path");
    let destination = destination_path.to_str().expect("output path");
    let mut save = args(&["--profile", source]);
    save.extend(args(&["--close-path", nfd, "2",
        "--output", destination, "--replace"]));
    assert_eq!(execute_glyph_library(&save)
        .expect("save closure"), None);
    assert_eq!(fs::read(&destination_path)
        .expect("read saved bridge"), result);
    assert_eq!(fs::read(&source_path)
        .expect("read unmodified source"), original);

    fs::write(&destination_path, b"preserved output")
        .expect("reset sentinel");
    let invalid = [
        args(&["--close-path"]),
        args(&["--close-path", nfd]),
        args(&["--close-path", nfd, "0"]),
        args(&["--close-path", nfd, "4"]),
        args(&["--close-path", nfd, "1"]),
        args(&["--close-path", "ó", "1"]),
        args(&["--close-path", "o", "1"]),
        args(&["--close-path", "oo", "1"]),
        args(&["--close-path", nfd, "2.5"]),
        args(&["--close-path", nfd, "4097"]),
        args(&["--close-path", nfd, "2",
            "--close-path", nfd, "2"]),
        args(&["--set-stroke", nfd, "2", "0", "500",
            "1400", "500", "250", "#243647",
            "--close-path", nfd, "2"]),
    ];
    for edit in invalid {
        let mut command = args(&["--profile", source]);
        command.extend(edit);
        command.extend(args(&["--output", destination, "--replace"]));
        assert!(execute_glyph_library(&command).is_err(),
            "invalid closure must refuse: {command:?}");
        assert_eq!(fs::read(&source_path).expect("input"), original);
        assert_eq!(fs::read(&destination_path).expect("output"),
            b"preserved output");
    }
    for (end_x, permitted) in [(1699, true), (1700, true), (1701, false)] {
        let proposed_closure = args(&[
            "--glyph-stroke",
            "o",
            "500",
            "800",
            "900",
            "1200",
            "250",
            "#243647",
            "--glyph-stroke",
            "o",
            "900",
            "1200",
            &end_x.to_string(),
            "800",
            "250",
            "#243647",
            "--close-path",
            "o",
            "1",
        ]);
        let result = execute_glyph_library(&proposed_closure);
        assert_eq!(
            result.is_ok(),
            permitted,
            "closure gap {} um must match the admission boundary",
            end_x - 500
        );
    }
    let tangent = args(&[
        "--glyph-stroke", "o", "125", "800", "900", "1200",
        "250", "#243647",
        "--glyph-stroke", "o", "900", "1200", "500", "800",
        "250", "#243647", "--close-path", "o", "1",
    ]);
    let admitted = execute_glyph_library(&tangent)
        .expect("inclusive physical pen tangent accepted")
        .expect("closure ink bytes");
    assert!(String::from_utf8(admitted).expect("UTF-8").contains(
        "o\t500\t800\t125\t800\t250\t#243647\n"));
    // The closing bridge cannot bypass the shared v1 4096-row budget.
    let mut full = args(&[
        "--glyph-stroke", "o", "500", "800", "900", "1200",
        "250", "#243647",
        "--glyph-stroke", "o", "900", "1200", "1700", "800",
        "250", "#243647",
    ]);
    for _ in 0..4094 {
        full.extend(args(&[
            "--glyph-stroke", "∞", "100", "200", "300", "400",
            "250", "#112233",
        ]));
    }
    let mut beyond = full.clone();
    beyond.extend(args(&["--close-path", "o", "1",
        "--output", destination, "--replace"]));
    assert!(execute_glyph_library(&beyond)
        .expect_err("4097th glyph stroke refuses")
        .contains("4096 authored strokes"));
    assert_eq!(fs::read(&destination_path).expect("prior output"),
        b"preserved output", "stroke cap refusal is file-atomic");
    assert!(execute_glyph_library(&full)
        .expect("the original 4096 glyphs remain valid").is_some());
    fs::remove_file(source_path).expect("remove test input");
    fs::remove_file(destination_path).expect("remove test output");
}

#[test]
fn native_join_next_path_preserves_unicode_and_native_pdf_png_ink() {
    let nfd = "n\u{303}";
    let source = test_path().with_extension("tsv");
    let saved = test_path().with_extension("tsv");
    let source_name = source.to_str().expect("original path");
    let saved_name = saved.to_str().expect("saved path");
    let geometry = args(&[
        "--glyph-stroke", nfd, "500", "700", "850", "750",
        "250", "#243647",
        "--glyph-stroke", "ñ", "600", "700", "650", "1000",
        "251", "#112233",
        "--glyph-stroke", nfd, "850", "750", "1100", "850",
        "250", "#243647",
        "--glyph-stroke", "∞", "500", "300", "600", "400",
        "251", "#445566",
        "--glyph-stroke", nfd, "1350", "900", "1600", "1050",
        "250", "#243647",
        "--glyph-stroke", nfd, "1600", "1050", "1900", "1150",
        "250", "#243647",
    ]);
    let original = execute_glyph_library(&geometry)
        .expect("original glyph library").expect("original TSV");
    fs::write(&source, &original).expect("source library");
    let edited = execute_glyph_library(&args(&[
        "--profile", source_name, "--join-next-path", nfd, "1",
    ])).expect("valid short pen bridge").expect("new TSV");
    let expected = concat!(
        "atrament.glyph-strokes.v1\n",
        "ñ\t500\t700\t850\t750\t250\t#243647\n",
        "ñ\t600\t700\t650\t1000\t251\t#112233\n",
        "ñ\t850\t750\t1100\t850\t250\t#243647\n",
        "ñ\t1100\t850\t1350\t900\t250\t#243647\n",
        "∞\t500\t300\t600\t400\t251\t#445566\n",
        "ñ\t1350\t900\t1600\t1050\t250\t#243647\n",
        "ñ\t1600\t1050\t1900\t1150\t250\t#243647\n",
    );
    assert_eq!(edited, expected.as_bytes());
    let mut save = args(&[
        "--profile", source_name, "--join-next-path", nfd, "1",
        "--output", saved_name,
    ]);
    assert_eq!(execute_glyph_library(&save)
        .expect("deliberate file save"), None);
    assert_eq!(fs::read(&saved).expect("saved bytes"), edited);
    assert_eq!(fs::read(&source).expect("untouched source"), original);
    let report = String::from_utf8(execute_glyph_library(&args(&[
        "--profile", saved_name, "--inspect-paths", nfd,
    ])).expect("inspected merged path").expect("inspector bytes"))
        .expect("UTF-8 path report");
    assert!(report.contains("Strokes: 5\nPaths: 1\nPen lifts: 0"),
        "{report}");
    assert!(report.contains(
        "Path 1: strokes 1..5 (5 segments) (500, 700) -> (1900, 1150)"
    ), "{report}");

    for format in ["pdf", "png"] {
        let mut imported = args(&[
            "--export-paper", format, "--printed-sheet", "true",
            "--glyph-file", saved_name,
            "--place-text", "1", "30000", "45000", nfd,
        ]);
        let mut inline = args(&[
            "--export-paper", format, "--printed-sheet", "true",
            "--glyph-stroke", nfd, "500", "700", "850", "750",
            "250", "#243647",
            "--glyph-stroke", nfd, "850", "750", "1100", "850",
            "250", "#243647",
            "--glyph-stroke", nfd, "1100", "850", "1350", "900",
            "250", "#243647",
            "--glyph-stroke", nfd, "1350", "900", "1600", "1050",
            "250", "#243647",
            "--glyph-stroke", nfd, "1600", "1050", "1900", "1150",
            "250", "#243647",
            "--place-text", "1", "30000", "45000", nfd,
        ]);
        if format == "png" {
            imported.extend(args(&["--dpi", "72"]));
            inline.extend(args(&["--dpi", "72"]));
        }
        let actual = execute_notebook(&imported)
            .expect("native glyph-file ink").expect("rendered bytes");
        let expected_bytes = execute_notebook(&inline)
            .expect("inline native ink").expect("rendered inline bytes");
        assert_eq!(actual, expected_bytes,
            "joining must reach the same native {format} renderer");
        assert!(if format == "pdf" {
            actual.starts_with(b"%PDF-1.4")
        } else { actual.starts_with(b"\x89PNG\r\n\x1a\n") });
    }

    fs::write(&saved, b"accepted output").expect("sentinel");
    for rejected in [
        args(&["--join-next-path"]),
        args(&["--join-next-path", nfd]),
        args(&["--join-next-path", nfd, "0"]),
        args(&["--join-next-path", nfd, "3"]),
        args(&["--join-next-path", "ñ", "1"]),
        args(&["--join-next-path", "nn", "1"]),
        args(&["--join-next-path", nfd, "1.5"]),
        args(&["--join-next-path", nfd, "4097"]),
        args(&["--join-next-path", nfd, "1",
            "--join-next-path", nfd, "1"]),
        args(&["--set-stroke", nfd, "3", "1350", "900",
            "1600", "1050", "300", "#243647",
            "--join-next-path", nfd, "1"]),
        args(&["--set-stroke", nfd, "3", "3400", "900",
            "1600", "1050", "250", "#243647",
            "--join-next-path", nfd, "1"]),
    ] {
        let mut command = args(&["--profile", source_name]);
        command.extend(rejected);
        command.extend(args(&["--output", saved_name, "--replace"]));
        assert!(execute_glyph_library(&command).is_err(),
            "invalid joined path must refuse: {command:?}");
        assert_eq!(fs::read(&saved).expect("unmodified sentinel"),
            b"accepted output");
        assert_eq!(fs::read(&source).expect("unmodified source"), original);
    }
    for (x, allowed) in [(2000, true), (2001, false)] {
        let coordinate = x.to_string();
        let proposed = args(&[
            "--glyph-stroke", nfd, "500", "700", "800", "850",
            "250", "#243647",
            "--glyph-stroke", nfd, &coordinate, "850", "2200", "950",
            "250", "#243647",
            "--join-next-path", nfd, "1",
        ]);
        assert_eq!(execute_glyph_library(&proposed).is_ok(), allowed,
            "a {x} µm second-path start must honor the 1200 µm limit");
    }
    let tangent = args(&[
        "--glyph-stroke", nfd, "300", "700", "125", "850",
        "250", "#243647",
        "--glyph-stroke", nfd, "500", "850", "650", "950",
        "250", "#243647",
        "--join-next-path", nfd, "1",
    ]);
    assert!(String::from_utf8(execute_glyph_library(&tangent)
        .expect("round pen edge tangent is valid")
        .expect("TSV bytes")).expect("UTF-8").contains(
        "n\u{303}\t125\t850\t500\t850\t250\t#243647\n"
    ));
    // Edge-valid source endpoints can still be too close to the cell edge
    // for the added *round pen*. The original TSV is not clipped on import.
    let clipped = args(&[
        "--glyph-stroke", nfd, "250", "700", "50", "850",
        "250", "#243647",
        "--glyph-stroke", nfd, "250", "850", "450", "950",
        "250", "#243647",
        "--join-next-path", nfd, "1",
    ]);
    assert!(execute_glyph_library(&clipped)
        .expect_err("round pen clips cell").contains("clips"));
    // The last permitted authored stroke cannot allocate a bridge.
    let mut full = String::from("atrament.glyph-strokes.v1\n");
    full.push_str("n\u{303}\t500\t700\t850\t750\t250\t#243647\n");
    full.push_str("n\u{303}\t1350\t900\t1600\t1050\t250\t#243647\n");
    full.push_str(&"∞\t500\t300\t600\t400\t251\t#445566\n".repeat(4094));
    fs::write(&source, &full).expect("4096-row source");
    save = args(&[
        "--profile", source_name, "--join-next-path", nfd, "1",
        "--output", saved_name, "--replace",
    ]);
    assert!(execute_glyph_library(&save)
        .expect_err("4097th bridge refused")
        .contains("4096 authored strokes"));
    assert_eq!(fs::read(&saved).expect("preserved sentinel"),
        b"accepted output");
    fs::remove_file(source).expect("remove source");
    fs::remove_file(saved).expect("remove saved output");
}

#[test]
fn native_curved_join_replays_exact_unicode_pdf_png_and_rejects_bad_ink() {
    let nfd = "n\u{303}";
    let source = test_path().with_extension("tsv");
    let saved = test_path().with_extension("tsv");
    let source_name = source.to_str().expect("original library path");
    let saved_name = saved.to_str().expect("saved library path");
    let geometry = args(&[
        "--glyph-stroke", nfd, "500", "700", "850", "750",
        "250", "#243647",
        "--glyph-stroke", "ñ", "600", "700", "650", "1000",
        "251", "#112233",
        "--glyph-stroke", nfd, "850", "750", "1100", "850",
        "250", "#243647",
        "--glyph-stroke", nfd, "1350", "900", "1600", "1050",
        "250", "#243647",
    ]);
    let original = execute_glyph_library(&geometry)
        .expect("original ink").expect("original bytes");
    fs::write(&source, &original).expect("original source file");
    let edited = execute_glyph_library(&args(&[
        "--profile", source_name, "--join-next-path-curved",
        nfd, "1", "0", "-200",
    ])).expect("accepted curved ink").expect("edited TSV");
    let expected = concat!(
        "atrament.glyph-strokes.v1\n",
        "ñ\t500\t700\t850\t750\t250\t#243647\n",
        "ñ\t600\t700\t650\t1000\t251\t#112233\n",
        "ñ\t850\t750\t1100\t850\t250\t#243647\n",
        "ñ\t1100\t850\t1163\t788\t250\t#243647\n",
        "ñ\t1163\t788\t1225\t775\t250\t#243647\n",
        "ñ\t1225\t775\t1288\t813\t250\t#243647\n",
        "ñ\t1288\t813\t1350\t900\t250\t#243647\n",
        "ñ\t1350\t900\t1600\t1050\t250\t#243647\n",
    );
    assert_eq!(edited, expected.as_bytes());
    assert_eq!(execute_glyph_library(&args(&[
        "--profile", source_name, "--join-next-path-curved",
        nfd, "1", "0", "-200", "--output", saved_name,
    ])).expect("explicit save"), None);
    assert_eq!(fs::read(&saved).expect("saved bytes"), edited);
    assert_eq!(fs::read(&source).expect("unchanged original"), original);
    let inspected = String::from_utf8(execute_glyph_library(&args(&[
        "--profile", saved_name, "--inspect-paths", nfd,
    ])).expect("path inspection").expect("path report"))
        .expect("UTF-8 report");
    assert!(inspected.contains("Strokes: 7\nPaths: 1\nPen lifts: 0"),
        "{inspected}");
    for format in ["pdf", "png"] {
        let mut imported = args(&[
            "--export-paper", format, "--printed-sheet", "true",
            "--glyph-file", saved_name,
            "--place-text", "1", "30000", "45000", nfd,
        ]);
        let mut inline = args(&[
            "--export-paper", format, "--printed-sheet", "true",
            "--glyph-stroke", nfd, "500", "700", "850", "750",
            "250", "#243647",
            "--glyph-stroke", nfd, "850", "750", "1100", "850",
            "250", "#243647",
            "--glyph-stroke", nfd, "1100", "850", "1163", "788",
            "250", "#243647",
            "--glyph-stroke", nfd, "1163", "788", "1225", "775",
            "250", "#243647",
            "--glyph-stroke", nfd, "1225", "775", "1288", "813",
            "250", "#243647",
            "--glyph-stroke", nfd, "1288", "813", "1350", "900",
            "250", "#243647",
            "--glyph-stroke", nfd, "1350", "900", "1600", "1050",
            "250", "#243647",
            "--place-text", "1", "30000", "45000", nfd,
        ]);
        if format == "png" {
            imported.extend(args(&["--dpi", "72"]));
            inline.extend(args(&["--dpi", "72"]));
        }
        let actual = execute_notebook(&imported)
            .expect("rendered curved library").expect("rendered bytes");
        let expected_bytes = execute_notebook(&inline)
            .expect("rendered inline strokes").expect("rendered bytes");
        assert_eq!(actual, expected_bytes, "curved {format} geometry parity");
    }
    fs::write(&saved, b"sentinel").expect("existing output sentinel");
    for rejected in [
        args(&["--join-next-path-curved"]),
        args(&["--join-next-path-curved", nfd, "1", "0"]),
        args(&["--join-next-path-curved", nfd, "0", "0", "-200"]),
        args(&["--join-next-path-curved", nfd, "1", "0", "0"]),
        args(&["--join-next-path-curved", nfd, "1", "601", "0"]),
        args(&["--join-next-path-curved", nfd, "1", "0", "-601"]),
        args(&["--join-next-path-curved", nfd, "1", "1.5", "0"]),
        args(&["--join-next-path-curved", "nn", "1", "0", "-200"]),
        args(&["--join-next-path-curved", nfd, "3", "0", "-200"]),
        args(&["--join-next-path-curved", nfd, "1", "250", "50"]),
        args(&["--set-stroke", nfd, "3", "1350", "900",
            "1600", "1050", "300", "#243647",
            "--join-next-path-curved", nfd, "1", "0", "-200"]),
    ] {
        let mut command = args(&["--profile", source_name]);
        command.extend(rejected);
        command.extend(args(&["--output", saved_name, "--replace"]));
        assert!(execute_glyph_library(&command).is_err(),
            "invalid curve must refuse: {command:?}");
        assert_eq!(fs::read(&saved).expect("preserved sentinel"),
            b"sentinel");
        assert_eq!(fs::read(&source).expect("unchanged original"), original);
    }
    let mut clipped = args(&[
        "--glyph-stroke", nfd, "250", "700", "125", "850",
        "250", "#243647",
        "--glyph-stroke", nfd, "500", "850", "650", "950",
        "250", "#243647", "--join-next-path-curved", nfd,
        "1", "-600", "50",
    ]);
    assert!(execute_glyph_library(&clipped)
        .expect_err("clipped curved line refused").contains("glyph cell"));
    clipped = args(&[
        "--glyph-stroke", nfd, "500", "700", "800", "850",
        "250", "#243647", "--glyph-stroke", nfd, "2001", "850",
        "2200", "950", "250", "#243647",
        "--join-next-path-curved", nfd, "1", "0", "-200",
    ]);
    assert!(execute_glyph_library(&clipped)
        .expect_err("over-limit gap refused").contains("1200"));
    let mut near = String::from("atrament.glyph-strokes.v1\n");
    near.push_str("n\u{303}\t500\t700\t850\t750\t250\t#243647\n");
    near.push_str("n\u{303}\t1350\t900\t1600\t1050\t250\t#243647\n");
    near.push_str(&"∞\t500\t300\t600\t400\t251\t#445566\n".repeat(4090));
    fs::write(&source, &near).expect("4092-stroke input");
    let completed = execute_glyph_library(&args(&[
        "--profile", source_name, "--join-next-path-curved",
        nfd, "1", "0", "-200",
    ])).expect("exactly four free slots").expect("full TSV bytes");
    assert_eq!(completed.iter().filter(|byte| **byte == b'\n').count(),
        4097, "header and 4096 authored strokes");
    near.push_str("∞\t500\t300\t600\t400\t251\t#445566\n");
    fs::write(&source, &near).expect("4093-stroke input");
    assert!(execute_glyph_library(&args(&[
        "--profile", source_name, "--join-next-path-curved",
        nfd, "1", "0", "-200",
    ])).expect_err("insufficient room for four curves")
        .contains("4096 authored strokes"));
    fs::remove_file(source).expect("remove original");
    fs::remove_file(saved).expect("remove saved");
}

#[test]
fn native_whole_glyph_shear_keeps_exact_unicode_slots_and_pen_paths() {
    let source = test_path().with_extension("tsv");
    let saved = test_path().with_extension("tsv");
    let source_name = source.to_str().expect("source path");
    let saved_name = saved.to_str().expect("destination path");
    let nfd = "a\u{301}";
    let original = execute_glyph_library(&args(&[
        "--glyph-stroke", nfd, "450", "1000", "850", "1600",
        "250", "#243647",
        "--glyph-stroke", "á", "600", "1200", "900", "1550",
        "250", "#123456",
        "--glyph-stroke", nfd, "850", "1600", "1300", "2200",
        "250", "#243647",
        "--glyph-stroke", "∞", "500", "900", "700", "1100",
        "250", "#112233",
        "--glyph-stroke", nfd, "1100", "200", "1700", "600",
        "250", "#334455",
    ])).expect("valid source").expect("source bytes");
    fs::write(&source, &original).expect("test source");
    let edited = execute_glyph_library(&args(&[
        "--profile", source_name, "--shear-glyph", nfd, "12", "3550",
    ])).expect("italic edit accepted").expect("italic TSV");
    assert_eq!(edited, concat!(
        "atrament.glyph-strokes.v1\n",
        "á\t992\t1000\t1264\t1600\t250\t#243647\n",
        "á\t600\t1200\t900\t1550\t250\t#123456\n",
        "á\t1264\t1600\t1587\t2200\t250\t#243647\n",
        "∞\t500\t900\t700\t1100\t250\t#112233\n",
        "á\t1812\t200\t2327\t600\t250\t#334455\n",
    ).as_bytes());
    assert_eq!(fs::read(&source).expect("source read"), original);
    let inspected = execute_glyph_library(&args(&[
        "--profile", source_name, "--shear-glyph", nfd, "12", "3550",
        "--inspect-paths", nfd,
    ])).expect("inspect italic path").expect("report");
    assert!(String::from_utf8_lossy(&inspected)
        .contains("Strokes: 3\nPaths: 2\nPen lifts: 1"));
    let wrote = execute_glyph_library(&args(&[
        "--profile", source_name, "--shear-glyph", nfd, "12", "3550",
        "--output", saved_name,
    ])).expect("save italic source");
    assert!(wrote.is_none());
    assert_eq!(fs::read(&saved).expect("saved TSV"), edited);
    assert_eq!(execute_glyph_library(&args(&[
        "--profile", saved_name,
    ])).expect("reopen saved").expect("bytes"), edited);
    assert_eq!(execute_glyph_library(&args(&[
        "--profile", source_name, "--shear-glyph", nfd, "0", "3550",
    ])).expect("zero angle is a valid identity").expect("TSV"), original);
    fs::remove_file(source).expect("remove source");
    fs::remove_file(saved).expect("remove saved");
}

#[test]
fn native_whole_glyph_shear_refuses_clipping_and_invalid_edits_atomically() {
    let source = test_path().with_extension("tsv");
    let output = test_path().with_extension("tsv");
    let source_name = source.to_str().expect("source path");
    let output_name = output.to_str().expect("destination path");
    let nfd = "a\u{301}";
    let original = execute_glyph_library(&args(&[
        "--glyph-stroke", nfd, "450", "1000", "850", "1600",
        "251", "#243647",
        "--glyph-stroke", "á", "600", "1200", "900", "1550",
        "250", "#123456",
        "--glyph-stroke", nfd, "850", "1600", "3250", "2200",
        "251", "#243647",
    ])).expect("valid source").expect("source bytes");
    fs::write(&source, &original).expect("own source");
    fs::write(&output, b"approved prior glyph file")
        .expect("test-owned destination");
    let invalid = [
        args(&["--shear-glyph"]),
        args(&["--shear-glyph", nfd]),
        args(&["--shear-glyph", nfd, "12"]),
        args(&["--shear-glyph", nfd, "-1", "3550"]),
        args(&["--shear-glyph", nfd, "21", "3550"]),
        args(&["--shear-glyph", nfd, "1.5", "3550"]),
        args(&["--shear-glyph", nfd, "+12", "3550"]),
        args(&["--shear-glyph", nfd, "12", "-1"]),
        args(&["--shear-glyph", nfd, "12", "4501"]),
        args(&["--shear-glyph", "aa", "12", "3550"]),
        args(&["--shear-glyph", "a", "12", "3550"]),
        args(&["--shear-glyph", "ó", "12", "3550"]),
        args(&["--shear-glyph", nfd, "20", "4500"]),
        args(&["--shear-glyph", nfd, "12", "3550",
            "--shear-glyph", nfd, "20", "4500"]),
    ];
    for edit in invalid {
        let mut command = args(&["--profile", source_name]);
        command.extend(edit);
        command.extend(args(&["--output", output_name, "--replace"]));
        assert!(execute_glyph_library(&command).is_err(),
            "invalid italic stroke edit must refuse: {command:?}");
        assert_eq!(fs::read(&source).expect("source unchanged"), original);
        assert_eq!(fs::read(&output).expect("prior output unchanged"),
            b"approved prior glyph file");
    }
    // An odd-width pen must not extend over the right edge even though
    // its rounded endpoint itself remains within 0..3500 micrometres.
    let clipped = execute_glyph_library(&args(&[
        "--glyph-stroke", "o", "3375", "100", "3374", "200",
        "251", "#102030", "--shear-glyph", "o", "0", "4500",
    ]));
    assert!(clipped.expect_err("odd-width pen clips the cell")
        .contains("italic ink and pen clip"));
    fs::remove_file(source).expect("remove source");
    fs::remove_file(output).expect("remove saved");
}

#[test]
fn native_glyph_height_scales_exact_unicode_pen_paths_without_other_edits() {
    let source = test_path().with_extension("tsv");
    let saved = test_path().with_extension("tsv");
    let source_name = source.to_str().expect("source path");
    let saved_name = saved.to_str().expect("destination path");
    let nfd = "a\u{301}";
    let original = execute_glyph_library(&args(&[
        "--glyph-stroke", nfd, "450", "1000", "850", "1600",
        "250", "#243647",
        "--glyph-stroke", "á", "600", "1200", "900", "1550",
        "250", "#123456",
        "--glyph-stroke", nfd, "850", "1600", "1300", "2200",
        "250", "#243647",
        "--glyph-stroke", "∞", "500", "900", "700", "1100",
        "250", "#112233",
        "--glyph-stroke", nfd, "1100", "600", "1700", "1000",
        "250", "#334455",
    ])).expect("valid source").expect("source bytes");
    fs::write(&source, &original).expect("test source");
    let edited = execute_glyph_library(&args(&[
        "--profile", source_name, "--scale-glyph-height", nfd,
        "110", "2250",
    ])).expect("height edit accepted").expect("TSV");
    assert_eq!(edited, concat!(
        "atrament.glyph-strokes.v1\n",
        "á\t450\t875\t850\t1535\t250\t#243647\n",
        "á\t600\t1200\t900\t1550\t250\t#123456\n",
        "á\t850\t1535\t1300\t2195\t250\t#243647\n",
        "∞\t500\t900\t700\t1100\t250\t#112233\n",
        "á\t1100\t435\t1700\t875\t250\t#334455\n",
    ).as_bytes());
    assert_eq!(fs::read(&source).expect("source preserved"), original);
    let inspector = execute_glyph_library(&args(&[
        "--profile", source_name, "--scale-glyph-height", nfd,
        "110", "2250", "--inspect-paths", nfd,
    ])).expect("inspect scaled paths").expect("report");
    assert!(String::from_utf8_lossy(&inspector)
        .contains("Strokes: 3\nPaths: 2\nPen lifts: 1"));
    assert!(execute_glyph_library(&args(&[
        "--profile", source_name, "--scale-glyph-height", nfd,
        "110", "2250", "--output", saved_name,
    ])).expect("explicit save").is_none());
    assert_eq!(fs::read(&saved).expect("saved TSV"), edited);
    assert_eq!(execute_glyph_library(&args(&[
        "--profile", saved_name,
    ])).expect("reopen").expect("bytes"), edited);
    assert_eq!(execute_glyph_library(&args(&[
        "--profile", source_name, "--scale-glyph-height", nfd,
        "100", "2250",
    ])).expect("identity").expect("TSV"), original);
    fs::remove_file(source).expect("remove source");
    fs::remove_file(saved).expect("remove saved");
}

#[test]
fn native_glyph_height_refuses_clipped_collapsed_and_invalid_edits() {
    let source = test_path().with_extension("tsv");
    let output = test_path().with_extension("tsv");
    let nfd = "a\u{301}";
    let original = execute_glyph_library(&args(&[
        "--glyph-stroke", nfd, "450", "1000", "850", "1600",
        "251", "#243647",
        "--glyph-stroke", "á", "600", "1200", "900", "1550",
        "250", "#123456",
        "--glyph-stroke", nfd, "850", "1600", "1300", "2200",
        "251", "#243647",
    ])).expect("source admitted").expect("bytes");
    fs::write(&source, &original).expect("source");
    fs::write(&output, b"approved prior output").expect("output");
    let source_name = source.to_str().expect("source path");
    let output_name = output.to_str().expect("output path");
    for edit in [
        args(&["--scale-glyph-height"]),
        args(&["--scale-glyph-height", nfd]),
        args(&["--scale-glyph-height", nfd, "110"]),
        args(&["--scale-glyph-height", nfd, "49", "2250"]),
        args(&["--scale-glyph-height", nfd, "151", "2250"]),
        args(&["--scale-glyph-height", nfd, "110.5", "2250"]),
        args(&["--scale-glyph-height", nfd, "+110", "2250"]),
        args(&["--scale-glyph-height", nfd, "110", "-1"]),
        args(&["--scale-glyph-height", nfd, "110", "4501"]),
        args(&["--scale-glyph-height", "aa", "110", "2250"]),
        args(&["--scale-glyph-height", "ñ", "110", "2250"]),
        args(&["--scale-glyph-height", nfd, "150", "4500"]),
    ] {
        let mut command = args(&["--profile", source_name]);
        command.extend(edit);
        command.extend(args(&["--output", output_name, "--replace"]));
        assert!(execute_glyph_library(&command).is_err(),
            "invalid height edit must refuse: {command:?}");
        assert_eq!(fs::read(&source).expect("source"), original);
        assert_eq!(fs::read(&output).expect("output"),
            b"approved prior output");
    }
    for (y1, y2, percent, pivot, expected) in [
        ("125", "300", "100", "2250", "resized ink and pen clip"),
        ("200", "600", "150", "2250", "resized ink and pen clip"),
        ("2001", "2002", "50", "2000", "collapses a pen segment"),
    ] {
        let result = execute_glyph_library(&args(&[
            "--glyph-stroke", "o", "450", y1, "450", y2,
            "251", "#102030", "--scale-glyph-height", "o",
            percent, pivot,
        ]));
        assert!(result.expect_err("must refuse").contains(expected));
    }
    let bad_x = execute_glyph_library(&args(&[
        "--glyph-stroke", "o", "125", "1000", "850", "1600",
        "251", "#102030", "--scale-glyph-height", "o",
        "100", "2250",
    ]));
    assert!(bad_x.expect_err("odd pen crosses X edge")
        .contains("resized ink and pen clip"));
    let negative_half = execute_glyph_library(&args(&[
        "--glyph-stroke", "o", "450", "1999", "850", "2001",
        "250", "#102030", "--scale-glyph-height", "o",
        "50", "2000",
    ])).expect("negative half rounded").expect("TSV");
    assert!(String::from_utf8_lossy(&negative_half)
        .contains("o\t450\t2000\t850\t2001\t250"));
    fs::remove_file(source).expect("remove source");
    fs::remove_file(output).expect("remove output");
}

#[test]
fn native_path_translation_round_trips_exact_unicode_pen_geometry() {
    let source = test_path().with_extension("tsv");
    let revised = test_path().with_extension("tsv");
    let source_name = source.to_str().expect("test source");
    let revised_name = revised.to_str().expect("test output");
    let original = execute_glyph_library(&args(&[
        "--glyph-stroke", "n\u{303}", "500", "650", "850",
        "950", "250", "#243647",
        "--glyph-stroke", "ñ", "100", "200", "300",
        "400", "250", "#121212",
        "--glyph-stroke", "n\u{303}", "850", "950", "1300",
        "1400", "250", "#243647",
        "--glyph-stroke", "∞", "500", "600", "700",
        "800", "250", "#121212",
        "--glyph-stroke", "n\u{303}", "1600", "2500", "2000",
        "2900", "250", "#B03040",
    ])).expect("source library").expect("original bytes");
    fs::write(&source, &original).expect("own source");
    let mut commands = args(&[
        "--profile", source_name,
        "--translate-path", "n\u{303}", "1", "300", "-250",
        "--translate-path", "n\u{303}", "2", "-100", "200",
    ]);
    let changed = execute_glyph_library(&commands)
        .expect("valid path shifts").expect("TSV bytes");
    assert_eq!(changed, concat!(
        "atrament.glyph-strokes.v1\n",
        "ñ\t800\t400\t1150\t700\t250\t#243647\n",
        "ñ\t100\t200\t300\t400\t250\t#121212\n",
        "ñ\t1150\t700\t1600\t1150\t250\t#243647\n",
        "∞\t500\t600\t700\t800\t250\t#121212\n",
        "ñ\t1500\t2700\t1900\t3100\t250\t#B03040\n",
    ).as_bytes());
    assert_eq!(fs::read(&source).expect("source still original"), original);
    let mut inspected = commands.clone();
    inspected.extend(args(&["--inspect-paths", "n\u{303}"]));
    let report = String::from_utf8(execute_glyph_library(&inspected)
        .expect("path inspector").expect("report")).expect("UTF-8");
    assert!(report.contains("Strokes: 3\nPaths: 2\nPen lifts: 1"));
    assert!(report.contains(
        "Path 1: strokes 1..2 (2 segments) (800, 400) -> (1600, 1150)"
    ), "{report}");
    assert!(report.contains(
        "Path 2: strokes 3..3 (1 segment) (1500, 2700) -> (1900, 3100)"
    ), "{report}");
    commands.extend(args(&[
        "--translate-path", "n\u{303}", "1", "-300", "250",
        "--translate-path", "n\u{303}", "2", "100", "-200",
    ]));
    assert_eq!(execute_glyph_library(&commands)
        .expect("inverse native shifts").expect("restored"), original);
    let save = execute_glyph_library(&args(&[
        "--profile", source_name,
        "--translate-path", "n\u{303}", "1", "300", "-250",
        "--translate-path", "n\u{303}", "2", "-100", "200",
        "--output", revised_name,
    ])).expect("explicit saved output");
    assert!(save.is_none());
    assert_eq!(fs::read(&revised).expect("saved"), changed);
    assert_eq!(execute_glyph_library(&args(&[
        "--profile", revised_name,
    ])).expect("reload").expect("bytes"), changed);
    for format in ["pdf", "png"] {
        let mut selected = args(&[
            "--export-paper", format, "--printed-sheet", "true",
            "--glyph-file", revised_name,
            "--place-text", "1", "30000", "45000", "n\u{303}",
        ]);
        let mut independent = args(&[
            "--export-paper", format, "--printed-sheet", "true",
            "--glyph-stroke", "n\u{303}", "800", "400", "1150",
            "700", "250", "#243647",
            "--glyph-stroke", "n\u{303}", "1150", "700", "1600",
            "1150", "250", "#243647",
            "--glyph-stroke", "n\u{303}", "1500", "2700", "1900",
            "3100", "250", "#B03040",
            "--place-text", "1", "30000", "45000", "n\u{303}",
        ]);
        if format == "png" {
            selected.extend(args(&["--dpi", "72"]));
            independent.extend(args(&["--dpi", "72"]));
        }
        assert_eq!(execute_notebook(&selected)
            .expect("native saved translated ink").expect("bytes"),
            execute_notebook(&independent)
            .expect("independent pen ink").expect("bytes"),
            "native translated {format} ink order and geometry");
    }
    fs::remove_file(source).expect("remove source");
    fs::remove_file(revised).expect("remove output");
}

#[test]
fn native_path_translation_refuses_invalid_offsets_atomically() {
    let source = test_path().with_extension("tsv");
    let output = test_path().with_extension("tsv");
    let source_name = source.to_str().expect("source path");
    let output_name = output.to_str().expect("output path");
    let original = execute_glyph_library(&args(&[
        "--glyph-stroke", "n\u{303}", "200", "200", "600",
        "600", "251", "#112233",
        "--glyph-stroke", "ñ", "200", "200", "600",
        "600", "250", "#112233",
        "--glyph-stroke", "n\u{303}", "600", "600", "800",
        "800", "251", "#112233",
        "--glyph-stroke", "n\u{303}", "2000", "3000", "2500",
        "3500", "250", "#445566",
    ])).expect("library").expect("original bytes");
    fs::write(&source, &original).expect("source file");
    fs::write(&output, b"preserve target").expect("target file");
    let invalid = [
        args(&["--translate-path"]),
        args(&["--translate-path", "n\u{303}"]),
        args(&["--translate-path", "n\u{303}", "1"]),
        args(&["--translate-path", "n\u{303}", "1", "100"]),
        args(&["--translate-path", "n\u{303}", "0", "100", "0"]),
        args(&["--translate-path", "n\u{303}", "3", "100", "0"]),
        args(&["--translate-path", "n\u{303}", "1", "0", "0"]),
        args(&["--translate-path", "n\u{303}", "1", "+1", "0"]),
        args(&["--translate-path", "n\u{303}", "1", "1.0", "0"]),
        args(&["--translate-path", "n\u{303}", "1", "3501", "0"]),
        args(&["--translate-path", "n\u{303}", "1", "-3501", "0"]),
        args(&["--translate-path", "n\u{303}", "1", "0", "-4501"]),
        args(&["--translate-path", "n\u{303}", "1", "-100", "0"]),
        args(&["--translate-path", "n\u{303}", "1", "0", "-100"]),
        args(&["--translate-path", "n\u{303}", "1", "3200", "0"]),
        args(&["--translate-path", "n\u{303}", "1", "0", "4000"]),
        args(&["--translate-path", "ñ", "2", "100", "0"]),
        args(&["--translate-path", "n", "1", "100", "0"]),
        args(&["--translate-path", "ab", "1", "100", "0"]),
        args(&["--translate-path", "n\u{303}", "1",
            "1", "0", "--translate-path", "n\u{303}", "2",
            "0", "-4000"]),
    ];
    for flags in invalid {
        let mut command = args(&["--profile", source_name]);
        command.extend(flags);
        command.extend(args(&["--output", output_name, "--replace"]));
        assert!(execute_glyph_library(&command).is_err(),
            "invalid pen translation must refuse: {command:?}");
        assert_eq!(fs::read(&source).expect("source untouched"), original);
        assert_eq!(fs::read(&output).expect("target untouched"),
            b"preserve target");
    }
    fs::remove_file(source).expect("remove source");
    fs::remove_file(output).expect("remove output");
}

#[test]
fn path_inspection_can_report_the_full_bounded_library() {
    let mut source = String::from("atrament.glyph-strokes.v1\n");
    for index in 0..4096 {
        let (start_x, end_x) = if index % 2 == 0 {
            (100, 200)
        } else {
            (200, 100)
        };
        source.push_str(&format!(
            "n\u{303}\t{start_x}\t100\t{end_x}\t200\t250\t#102030\n",
        ));
    }
    let mut overlay = GlyphOverlay::default();
    overlay.load_file_bytes(source.as_bytes())
        .expect("4096 authored strokes within bounded file");
    // Repeated example alternates horizontal endpoints but the y direction
    // does not match; every segment therefore starts a candidate new path.
    let report = String::from_utf8(overlay.inspect_paths("n\u{303}")
        .expect("bounded per-stroke diagnostic"))
        .expect("UTF-8 report");
    assert!(report.contains("Strokes: 4096\nPaths: 4096"));
    assert!(report.contains("Pen lifts: 4095"));
    assert!(report.contains("Path 4096: strokes 4096..4096"));
}

#[test]
fn failed_glyph_tsv_load_does_not_retain_valid_prefix_ink()
-> Result<(), String> {
    let header = "atrament.glyph-strokes.v1\n";
    let valid = "ñ\t100\t200\t600\t900\t250\t#112233\n";
    let accepted = format!("{header}{valid}");
    let mut empty = GlyphOverlay::default();
    let error = empty.load_file_bytes(header.as_bytes())
        .expect_err("header-only TSV has no authored shapes");
    if !error.contains("at least one authored stroke")
        || empty.has_any() || empty.loaded_file {
        return Err(format!("header-only TSV changed draft: {error}"));
    }
    empty.load_file_bytes(accepted.as_bytes())?;
    if empty.encode_file()? != accepted.as_bytes() {
        return Err("header-only TSV refusal changed later exact ink".into());
    }
    let malformed = [
        format!("{accepted}no tab separators\n"),
        format!("{accepted}\u{200d}\t100\t200\t600\t900\t250\t#112233\n"),
        format!("{accepted}a\t100\t200\t600\t900\t0\t#112233\n"),
    ];
    for source in &malformed {
        let mut overlay = GlyphOverlay::default();
        let error = overlay.load_file_bytes(source.as_bytes())
            .expect_err("late invalid TSV row must refuse the entire file");
        if !error.contains("glyph library line 3") {
            return Err(format!("wrong late-line diagnostic: {error}"));
        }
        if overlay.has_any() || overlay.loaded_file {
            return Err("failed TSV retained prefix ink or file flag".into());
        }
        overlay.load_file_bytes(accepted.as_bytes())?;
        if overlay.encode_file()? != accepted.as_bytes()
            || !overlay.loaded_file || overlay.strokes.len() != 1 {
            return Err("retry after refusal changed exact ink".into());
        }
    }

    // Also refuse the 4097th row without retaining the 4096 valid prefix.
    let overflowing = format!("{header}{}", valid.repeat(4097));
    let mut overlay = GlyphOverlay::default();
    let error = overlay.load_file_bytes(overflowing.as_bytes())
        .expect_err("oversized stroke count must refuse atomically");
    if !error.contains("4096 strokes")
        || overlay.has_any() || overlay.loaded_file {
        return Err(format!("overfull TSV retained partial glyph ink: {error}"));
    }
    overlay.load_file_bytes(accepted.as_bytes())?;
    if overlay.encode_file()? != accepted.as_bytes() {
        return Err("overfull TSV refusal changed later exact ink".into());
    }
    Ok(())
}

#[test]
fn path_inspection_refuses_invalid_modes_without_writing_files() {
    let source = test_path().with_extension("tsv");
    let output = test_path().with_extension("tsv");
    let name = source.to_str().expect("owned source");
    let destination = output.to_str().expect("owned target");
    let library = execute_glyph_library(&args(&[
        "--glyph-stroke", "ñ", "100", "200", "800",
        "1200", "250", "#112233",
    ])).expect("source library").expect("TSV");
    fs::write(&source, &library).expect("source");
    fs::write(&output, b"preserve target sentinel").expect("sentinel");
    for options in [
        args(&["--inspect-paths"]),
        args(&["--inspect-paths", ""]),
        args(&["--inspect-paths", " "]),
        args(&["--inspect-paths", "n\u{303}"]),
        args(&["--inspect-paths", "ab"]),
        args(&["--inspect-paths", "ñ", "--inspect-paths", "ñ"]),
        args(&["--inspect-paths", "ñ", "--inspect-glyph", "ñ"]),
        args(&["--inspect-paths", "ñ", "--audit-baseline"]),
        args(&["--inspect-paths", "ñ", "--audit-text", "ñ"]),
        args(&["--inspect-paths", "ñ", "--replace"]),
        args(&["--inspect-paths", "ñ", "--output", destination]),
        args(&["--inspect-paths", "ñ", "--output", destination,
            "--replace"]),
    ] {
        let mut command = args(&["--profile", name]);
        command.extend(options);
        assert!(execute_glyph_library(&command).is_err(),
            "inspection cannot mutate source or output: {command:?}");
        assert_eq!(fs::read(&source).expect("source unchanged"), library);
        assert_eq!(fs::read(&output).expect("target unchanged"),
            b"preserve target sentinel");
    }
    fs::remove_file(source).expect("remove source");
    fs::remove_file(output).expect("remove sentinel");
}

#[test]
fn glyph_inspection_preserves_exact_unicode_order_and_ink_direction() {
    let file = test_path().with_extension("tsv");
    let path = file.to_str().expect("owned library filename");
    let template = args(&[
        "--glyph-stroke", "n\u{303}", "100", "200", "800", "1200",
        "250", "#112233",
        "--glyph-stroke", "ñ", "900", "1500", "1000", "2000",
        "250", "#445566",
        "--glyph-stroke", "n\u{303}", "800", "1200", "350", "4200",
        "300", "#AABBCC",
    ]);
    let original = execute_glyph_library(&template)
        .expect("ordered canonical glyphs").expect("TSV bytes");
    fs::write(&file, &original).expect("owned stroke source");
    let expected = concat!(
        "Exact authored stroke sequence (not inferred cursive)\n",
        "Glyph: \"ñ\" [U+006E U+0303]\n",
        "Strokes: 2\n",
        "1: (100, 200) -> (800, 1200) width 250 um color #112233\n",
        "2: (800, 1200) -> (350, 4200) width 300 um color #AABBCC\n",
    );
    let from_disk = execute_glyph_library(&args(&[
        "--profile", path, "--inspect-glyph", "n\u{303}",
    ])).expect("inspect loaded NFD glyph").expect("stdout report");
    assert_eq!(from_disk, expected.as_bytes());
    let mut inline = template;
    inline.extend(args(&["--inspect-glyph", "n\u{303}"]));
    assert_eq!(execute_glyph_library(&inline)
        .expect("inspect inline geometry").expect("stdout"), from_disk);
    let nfc = String::from_utf8(execute_glyph_library(&args(&[
        "--profile", path, "--inspect-glyph", "ñ",
    ])).expect("inspect NFC glyph").expect("report"))
        .expect("UTF-8 report");
    assert!(nfc.contains("[U+00F1]"));
    assert!(nfc.contains("Strokes: 1"));
    assert!(nfc.contains("1: (900, 1500) -> (1000, 2000)"));
    assert!(!nfc.contains("(100, 200)"),
        "NFC and NFD remain independent authored identities");
    assert_eq!(fs::read(&file).expect("read-only source"), original);

    let pdf = execute_notebook(&args(&[
        "--printed-sheet", "true", "--export-paper", "pdf",
        "--glyph-file", path, "--place-text", "1", "30000",
        "45000", "n\u{303}ñ",
    ])).expect("same authored strokes used for native PDF")
        .expect("vector PDF");
    let content = String::from_utf8_lossy(&pdf);
    let first = content.find("0.067 0.133 0.200 RG")
        .expect("first NFD pen color");
    let second = content.find("0.667 0.733 0.800 RG")
        .expect("second NFD pen color");
    let third = content.find("0.267 0.333 0.400 RG")
        .expect("NFC pen color");
    assert!(first < second && second < third,
        "native compositing preserves per-glyph segment order");
    fs::remove_file(file).expect("remove test-owned library");
}

#[test]
fn glyph_inspection_is_stdout_only_and_refuses_invalid_selection() {
    let library = test_path().with_extension("tsv");
    let output = test_path().with_extension("tsv");
    let name = library.to_str().expect("library filename");
    let destination = output.to_str().expect("output filename");
    let source = execute_glyph_library(&args(&[
        "--glyph-stroke", "ñ", "100", "200", "800", "1200",
        "250", "#112233",
    ])).expect("original glyph").expect("TSV");
    fs::write(&library, &source).expect("owned source file");
    fs::write(&output, b"preserve selected target")
        .expect("owned sentinel");
    let invalid = [
        args(&["--inspect-glyph"]),
        args(&["--inspect-glyph", "ab"]),
        args(&["--inspect-glyph", "n\u{303}"]),
        args(&["--inspect-glyph", " "]),
        args(&["--inspect-glyph", "\n"]),
        args(&["--inspect-glyph", ""]),
        args(&["--inspect-glyph", "ñ", "--inspect-glyph", "ñ"]),
        args(&["--inspect-glyph", "ñ", "--audit-text", "ñ"]),
        args(&["--audit-baseline", "--inspect-glyph", "ñ"]),
        args(&["--inspect-glyph", "ñ", "--replace"]),
        args(&["--inspect-glyph", "ñ", "--output", destination]),
    ];
    for flags in invalid {
        let mut command = args(&["--profile", name]);
        command.extend(flags);
        let rejected = execute_glyph_library(&command);
        assert!(rejected.is_err(), "{command:?}");
        assert_eq!(fs::read(&library).expect("source unaffected"), source);
        assert_eq!(fs::read(&output).expect("target unaffected"),
            b"preserve selected target");
    }
    let rejected = execute_glyph_library(&args(&[
        "--profile", name, "--inspect-glyph", "ñ",
        "--output", destination, "--replace",
    ]));
    assert!(rejected.is_err(), "inspection never writes");
    assert_eq!(fs::read(&output).expect("unchanged output"),
        b"preserve selected target");
    fs::remove_file(library).expect("remove owned input");
    fs::remove_file(output).expect("remove owned sentinel");
}

#[test]
fn glyph_edits_shift_only_exact_unicode_slots_and_render_in_new_order() {
    let source = test_path().with_extension("tsv");
    let revised = test_path().with_extension("tsv");
    let source_name = source.to_str().expect("owned source path");
    let revised_name = revised.to_str().expect("owned revised path");
    let original = execute_glyph_library(&args(&[
        "--glyph-stroke", "n\u{303}", "500", "700", "900", "1300",
        "250", "#102030",
        "--glyph-stroke", "ñ", "1100", "900", "2200", "2700",
        "300", "#445566",
        "--glyph-stroke", "n\u{303}", "900", "1300", "1800", "2600",
        "350", "#708090",
        "--glyph-stroke", "∞", "1200", "600", "2300", "1700",
        "400", "#AA5500",
        "--glyph-stroke", "n\u{303}", "1800", "2600", "3000", "3900",
        "450", "#BBCCDD",
    ])).expect("initial canonical library").expect("original TSV");
    fs::write(&source, &original).expect("owned source file");

    let updated = execute_glyph_library(&args(&[
        "--profile", source_name,
        "--move-stroke", "n\u{303}", "1", "3",
        "--reverse-stroke", "n\u{303}", "2",
        "--move-stroke", "n\u{303}", "3", "2",
    ])).expect("moves forward/backward and reverses one NFD segment")
        .expect("edited TSV stdout");
    let expected = concat!(
        "atrament.glyph-strokes.v1\n",
        "ñ\t900\t1300\t1800\t2600\t350\t#708090\n",
        "ñ\t1100\t900\t2200\t2700\t300\t#445566\n",
        "ñ\t500\t700\t900\t1300\t250\t#102030\n",
        "∞\t1200\t600\t2300\t1700\t400\t#AA5500\n",
        "ñ\t3000\t3900\t1800\t2600\t450\t#BBCCDD\n",
    );
    assert_eq!(updated, expected.as_bytes(),
        "NFC and math records retain their original TSV slots and bytes");
    assert_eq!(fs::read(&source).expect("no implicit source write"),
        original);
    let report = execute_glyph_library(&args(&[
        "--profile", source_name,
        "--move-stroke", "n\u{303}", "1", "3",
        "--reverse-stroke", "n\u{303}", "2",
        "--move-stroke", "n\u{303}", "3", "2",
        "--inspect-glyph", "n\u{303}",
    ])).expect("inspect edited in-memory draft").expect("stdout report");
    let text = String::from_utf8(report).expect("readable report");
    assert!(text.contains(
        "1: (900, 1300) -> (1800, 2600) width 350 um color #708090"
    ), "{text}");
    assert!(text.contains(
        "2: (500, 700) -> (900, 1300) width 250 um color #102030"
    ), "{text}");
    assert!(text.contains(
        "3: (3000, 3900) -> (1800, 2600) width 450 um color #BBCCDD"
    ), "{text}");

    assert!(execute_glyph_library(&args(&[
        "--profile", source_name,
        "--move-stroke", "n\u{303}", "1", "3",
        "--reverse-stroke", "n\u{303}", "2",
        "--move-stroke", "n\u{303}", "3", "2",
        "--output", revised_name,
    ])).expect("explicit save creates a separate library").is_none());
    assert_eq!(fs::read(&revised).expect("native saved TSV"), updated);
    assert_eq!(execute_glyph_library(&args(&[
        "--profile", revised_name,
    ])).expect("reopen edited library").expect("TSV bytes"), updated);

    for format in ["pdf", "png"] {
        let mut selected = args(&[
            "--export-paper", format, "--printed-sheet", "true",
            "--glyph-file", revised_name, "--place-text", "1",
            "30000", "45000", "n\u{303}ñ∞",
        ]);
        let mut explicit = args(&[
            "--export-paper", format, "--printed-sheet", "true",
            "--glyph-stroke", "n\u{303}", "900", "1300", "1800",
            "2600", "350", "#708090",
            "--glyph-stroke", "n\u{303}", "500", "700", "900",
            "1300", "250", "#102030",
            "--glyph-stroke", "n\u{303}", "3000", "3900", "1800",
            "2600", "450", "#BBCCDD",
            "--glyph-stroke", "ñ", "1100", "900", "2200", "2700",
            "300", "#445566",
            "--glyph-stroke", "∞", "1200", "600", "2300", "1700",
            "400", "#AA5500",
            "--place-text", "1", "30000", "45000", "n\u{303}ñ∞",
        ]);
        if format == "png" {
            selected.extend(args(&["--dpi", "72"]));
            explicit.extend(args(&["--dpi", "72"]));
        }
        assert_eq!(execute_notebook(&selected)
            .expect("saved reordering renders")
            .expect("native bytes"), execute_notebook(&explicit)
            .expect("inline order renders").expect("native bytes"),
            "saved edit changes the actual physical {format} ink sequence");
    }
    assert_eq!(fs::read(&source).expect("source stayed immutable"),
        original);
    fs::remove_file(source).expect("remove test source");
    fs::remove_file(revised).expect("remove test revision");
}

#[test]
fn glyph_stroke_edits_refuse_invalid_indices_without_touching_files() {
    let library = test_path().with_extension("tsv");
    let target = test_path().with_extension("tsv");
    let source_name = library.to_str().expect("owned source path");
    let output_name = target.to_str().expect("owned destination path");
    let original = execute_glyph_library(&args(&[
        "--glyph-stroke", "n\u{303}", "500", "700", "900", "1300",
        "250", "#102030",
        "--glyph-stroke", "ñ", "1100", "900", "2200", "2700",
        "300", "#445566",
        "--glyph-stroke", "n\u{303}", "900", "1300", "1800",
        "2600", "350", "#708090",
    ])).expect("canonical fixture").expect("TSV");
    fs::write(&library, &original).expect("owned source library");
    fs::write(&target, b"preserve existing destination")
        .expect("owned sentinel file");
    let invalid = [
        args(&["--reverse-stroke"]),
        args(&["--reverse-stroke", "n\u{303}"]),
        args(&["--reverse-stroke", "n\u{303}", "0"]),
        args(&["--reverse-stroke", "n\u{303}", "-1"]),
        args(&["--reverse-stroke", "n\u{303}", "1.0"]),
        args(&["--reverse-stroke", "n\u{303}", "4097"]),
        args(&["--reverse-stroke", "n\u{303}", "3"]),
        args(&["--reverse-stroke", "ñ", "2"]),
        args(&["--reverse-stroke", "n", "1"]),
        args(&["--reverse-stroke", " ", "1"]),
        args(&["--reverse-stroke", "ab", "1"]),
        args(&["--move-stroke"]),
        args(&["--move-stroke", "n\u{303}", "1"]),
        args(&["--move-stroke", "n\u{303}", "0", "1"]),
        args(&["--move-stroke", "n\u{303}", "1", "3"]),
        args(&["--move-stroke", "n\u{303}", "2", "2"]),
        args(&["--move-stroke", "ñ", "1", "2"]),
        args(&["--move-stroke", "n", "1", "2"]),
        args(&["--move-stroke", "n\u{303}", "1", "4294967295"]),
        args(&[
            "--reverse-stroke", "n\u{303}", "1",
            "--move-stroke", "n\u{303}", "1", "3",
        ]),
    ];
    for flags in invalid {
        let mut candidate = args(&["--profile", source_name]);
        candidate.extend(flags);
        candidate.extend(args(&["--output", output_name, "--replace"]));
        assert!(execute_glyph_library(&candidate).is_err(),
            "invalid stroke edits must refuse: {candidate:?}");
        assert_eq!(fs::read(&library).expect("source unchanged"), original);
        assert_eq!(fs::read(&target).expect("destination unchanged"),
            b"preserve existing destination");
    }
    fs::remove_file(library).expect("remove test library");
    fs::remove_file(target).expect("remove test sentinel");
}

#[test]
fn explicit_source_glyph_copy_derives_simple_a_without_changing_o() {
    let source = test_path().with_extension("tsv");
    let revised = test_path().with_extension("tsv");
    let source_name = source.to_str().expect("owned source");
    let revised_name = revised.to_str().expect("owned derived library");
    // Caller-authored sample segments, not a mathematically exact circle.
    let original = execute_glyph_library(&args(&[
        "--glyph-stroke", "o", "1750", "500", "3000", "2300",
        "250", "#203040",
        "--glyph-stroke", "n\u{303}", "1000", "600", "2000",
        "2600", "350", "#CC9911",
        "--glyph-stroke", "o", "3000", "2300", "1750", "4000",
        "250", "#203040",
        "--glyph-stroke", "o", "1750", "4000", "500", "2300",
        "250", "#203040",
        "--glyph-stroke", "o", "500", "2300", "1750", "500",
        "250", "#203040",
    ])).expect("original loop sample").expect("canonical TSV");
    fs::write(&source, &original).expect("write original source");
    let options = args(&[
        "--profile", source_name, "--copy-glyph", "o", "a",
        "--glyph-stroke", "a", "2900", "850", "2900", "3950",
        "300", "#203040",
        "--set-stroke", "a", "1", "1750", "550", "3000",
        "2300", "275", "#556677",
    ]);
    let updated = execute_glyph_library(&options)
        .expect("copy and separately edit loop for a")
        .expect("derived library");
    let mut expected = String::from_utf8(original.clone())
        .expect("initial UTF-8 glyph TSV");
    expected.push_str(concat!(
        "a\t1750\t550\t3000\t2300\t275\t#556677\n",
        "a\t3000\t2300\t1750\t4000\t250\t#203040\n",
        "a\t1750\t4000\t500\t2300\t250\t#203040\n",
        "a\t500\t2300\t1750\t500\t250\t#203040\n",
        "a\t2900\t850\t2900\t3950\t300\t#203040\n",
    ));
    assert_eq!(updated, expected.as_bytes());
    assert_eq!(fs::read(&source).expect("source stays untouched"),
        original);
    let report = execute_glyph_library(&args(&[
        "--profile", source_name, "--copy-glyph", "o", "a",
        "--inspect-glyph", "a",
    ])).expect("inspect independent copy").expect("report");
    let report = String::from_utf8(report).expect("inspect UTF-8");
    assert!(report.contains("Glyph: \"a\" [U+0061]"), "{report}");
    assert!(report.contains("Strokes: 4"), "{report}");
    assert!(report.contains("4: (500, 2300) -> (1750, 500)"),
        "{report}");

    let mut saving = options;
    saving.extend(args(&["--output", revised_name]));
    assert!(execute_glyph_library(&saving)
        .expect("explicit independent derived save").is_none());
    assert_eq!(fs::read(&revised).expect("saved TSV"), updated);
    for format in ["pdf", "png"] {
        let mut stored = args(&[
            "--export-paper", format, "--printed-sheet", "true",
            "--glyph-file", revised_name,
            "--place-text", "1", "30000", "45000", "oa",
        ]);
        let mut inline = args(&[
            "--export-paper", format, "--printed-sheet", "true",
            "--glyph-stroke", "o", "1750", "500", "3000", "2300",
            "250", "#203040",
            "--glyph-stroke", "o", "3000", "2300", "1750", "4000",
            "250", "#203040",
            "--glyph-stroke", "o", "1750", "4000", "500", "2300",
            "250", "#203040",
            "--glyph-stroke", "o", "500", "2300", "1750", "500",
            "250", "#203040",
            "--glyph-stroke", "a", "1750", "550", "3000", "2300",
            "275", "#556677",
            "--glyph-stroke", "a", "3000", "2300", "1750", "4000",
            "250", "#203040",
            "--glyph-stroke", "a", "1750", "4000", "500", "2300",
            "250", "#203040",
            "--glyph-stroke", "a", "500", "2300", "1750", "500",
            "250", "#203040",
            "--glyph-stroke", "a", "2900", "850", "2900", "3950",
            "300", "#203040",
            "--place-text", "1", "30000", "45000", "oa",
        ]);
        if format == "png" {
            stored.extend(args(&["--dpi", "72"]));
            inline.extend(args(&["--dpi", "72"]));
        }
        assert_eq!(execute_notebook(&stored).expect("derived library renders")
            .expect("native bytes"), execute_notebook(&inline)
            .expect("same inline glyphs render").expect("native bytes"),
            "derived loop and stem reach the native {format} compositor");
    }
    fs::remove_file(source).expect("remove test input");
    fs::remove_file(revised).expect("remove derived library");
}

#[test]
fn native_path_copy_appends_exact_unicode_and_renders_pdf_png() {
    let source = test_path().with_extension("tsv");
    let revision = test_path().with_extension("tsv");
    let source_name = source.to_str().expect("source path");
    let revision_name = revision.to_str().expect("revision path");
    let original = execute_glyph_library(&args(&[
        "--glyph-stroke", "n\u{303}", "500", "700", "1100", "1500",
        "200", "#112233",
        "--glyph-stroke", "∞", "800", "800", "2000", "2700",
        "300", "#445566",
        "--glyph-stroke", "ñ", "2200", "600", "2200", "3300",
        "300", "#778899",
        "--glyph-stroke", "n\u{303}", "1100", "1500", "1700", "2500",
        "200", "#112233",
        "--glyph-stroke", "n\u{303}", "2100", "700", "2700", "1100",
        "220", "#99AABB",
    ])).expect("source library admitted").expect("original bytes");
    fs::write(&source, &original).expect("owned source");
    let mut copy = args(&[
        "--profile", source_name,
        "--copy-path", "n\u{303}", "1", "ñ",
        "--copy-path", "n\u{303}", "2", "á",
    ]);
    let output = execute_glyph_library(&copy)
        .expect("independent path copies").expect("canonical copied TSV");
    let mut expected = original.clone();
    expected.extend_from_slice(concat!(
        "ñ\t500\t700\t1100\t1500\t200\t#112233\n",
        "ñ\t1100\t1500\t1700\t2500\t200\t#112233\n",
        "á\t2100\t700\t2700\t1100\t220\t#99AABB\n",
    ).as_bytes());
    assert_eq!(output, expected);
    assert_eq!(fs::read(&source).expect("source remains unchanged"),
        original);
    copy.extend(args(&["--output", revision_name]));
    assert!(execute_glyph_library(&copy)
        .expect("explicit save accepted").is_none());
    assert_eq!(fs::read(&revision).expect("saved UTF-8 TSV"), output);
    let report = String::from_utf8(execute_glyph_library(&args(&[
        "--profile", revision_name, "--inspect-paths", "ñ",
    ])).expect("inspect saved ink").expect("path report"))
        .expect("UTF-8 inspection");
    assert!(report.contains("Strokes: 3"), "{report}");
    assert!(report.contains("Paths: 2"), "{report}");
    for format in ["pdf", "png"] {
        let mut saved = args(&[
            "--export-paper", format, "--printed-sheet", "true",
            "--glyph-file", revision_name,
            "--place-text", "1", "30000", "45000", "ñá",
        ]);
        let mut inline = args(&[
            "--export-paper", format, "--printed-sheet", "true",
            "--glyph-stroke", "ñ", "2200", "600", "2200", "3300",
            "300", "#778899",
            "--glyph-stroke", "ñ", "500", "700", "1100", "1500",
            "200", "#112233",
            "--glyph-stroke", "ñ", "1100", "1500", "1700", "2500",
            "200", "#112233",
            "--glyph-stroke", "á", "2100", "700", "2700", "1100",
            "220", "#99AABB",
            "--place-text", "1", "30000", "45000", "ñá",
        ]);
        if format == "png" {
            saved.extend(args(&["--dpi", "72"]));
            inline.extend(args(&["--dpi", "72"]));
        }
        let actual = execute_notebook(&saved)
            .expect("saved path renders").expect("rendered bytes");
        let expectation = execute_notebook(&inline)
            .expect("inline path renders").expect("inline bytes");
        assert_eq!(actual, expectation,
            "copied path must render same {format} as explicit source ink");
        if format == "pdf" {
            assert!(actual.starts_with(b"%PDF-1.4"));
        } else {
            assert!(actual.starts_with(b"\x89PNG\r\n\x1a\n"));
        }
    }
    fs::remove_file(source).expect("remove source");
    fs::remove_file(revision).expect("remove revision");
}

#[test]
fn native_path_removal_preserves_interleaved_unicode_and_refuses_last_ink() {
    let source = test_path().with_extension("tsv");
    let target = test_path().with_extension("tsv");
    let source_name = source.to_str().expect("path");
    let target_name = target.to_str().expect("path");
    let original = execute_glyph_library(&args(&[
        "--glyph-stroke", "n\u{303}", "500", "600", "1000", "1200",
        "251", "#112233",
        "--glyph-stroke", "ñ", "1100", "600", "1400", "1200",
        "251", "#ABCDEF",
        "--glyph-stroke", "n\u{303}", "1000", "1200", "1400", "1900",
        "251", "#112233",
        "--glyph-stroke", "∞", "300", "400", "600", "800",
        "200", "#556677",
        "--glyph-stroke", "n\u{303}", "1800", "2500", "2300",
        "3000", "200", "#445566",
    ])).expect("source").expect("TSV");
    fs::write(&source, &original).expect("source");
    fs::write(&target, b"target sentinel").expect("sentinel");
    let revised = execute_glyph_library(&args(&[
        "--profile", source_name,
        "--remove-path", "n\u{303}", "1",
    ])).expect("remove two connected segments").expect("output");
    assert_eq!(revised, concat!(
        "atrament.glyph-strokes.v1\n",
        "ñ\t1100\t600\t1400\t1200\t251\t#ABCDEF\n",
        "∞\t300\t400\t600\t800\t200\t#556677\n",
        "n\u{303}\t1800\t2500\t2300\t3000\t200\t#445566\n",
    ).as_bytes());
    assert_eq!(fs::read(&source).expect("source"), original);
    assert!(execute_glyph_library(&args(&[
        "--profile", source_name, "--remove-path", "n\u{303}", "1",
        "--output", target_name, "--replace",
    ])).expect("save selected target").is_none());
    assert_eq!(fs::read(&target).expect("reopen"), revised);
    let inspected = execute_glyph_library(&args(&[
        "--profile", target_name, "--inspect-paths", "n\u{303}",
    ])).expect("inspection").expect("UTF-8");
    assert!(String::from_utf8_lossy(&inspected).contains("Paths: 1"));
    for (index, rejected) in [
        args(&["--remove-path"]),
        args(&["--remove-path", "n\u{303}"]),
        args(&["--remove-path", "n\u{303}", "0"]),
        args(&["--remove-path", "n\u{303}", "4097"]),
        args(&["--remove-path", "n\u{303}", "3"]),
        args(&["--remove-path", "ab", "1"]),
        args(&["--remove-path", " ", "1"]),
    ].into_iter().enumerate() {
        let mut edit = args(&["--profile", source_name]);
        edit.extend(rejected.clone());
        edit.extend(args(&["--output", target_name, "--replace"]));
        assert!(execute_glyph_library(&edit).is_err(),
            "invalid path removal {rejected:?}");
        assert_eq!(fs::read(&target).expect("sentinel"), revised);
        let mut piped = args(&["--profile", "-"]);
        piped.extend(rejected);
        let preflight = preflight_glyph_profile_sources(&piped);
        if index == 4 {
            preflight.expect("missing path requires source inspection");
        } else {
            assert!(preflight.is_err(), "{preflight:?}");
        }
    }
    let alone = execute_glyph_library(&args(&[
        "--glyph-stroke", "ñ", "500", "600", "900", "1000",
        "200", "#112233",
        "--remove-path", "ñ", "1",
    ])).expect_err("cannot delete final native pen path");
    assert!(alone.contains("final authored pen path"), "{alone}");
    fs::remove_file(source).expect("remove source");
    fs::remove_file(target).expect("remove target");
}

#[test]
fn native_path_pen_style_round_trips_unicode_and_physical_exports() {
    let source = test_path().with_extension("tsv");
    let revision = test_path().with_extension("tsv");
    let source_name = source.to_str().expect("source filename");
    let revision_name = revision.to_str().expect("revision filename");
    let original = execute_glyph_library(&args(&[
        "--glyph-stroke", "n\u{303}", "500", "700", "1000", "1400",
        "251", "#112233",
        "--glyph-stroke", "ñ", "2200", "600", "2200", "3300",
        "300", "#778899",
        "--glyph-stroke", "n\u{303}", "1000", "1400", "1500", "2200",
        "251", "#112233",
        "--glyph-stroke", "∞", "800", "800", "1700", "1500",
        "200", "#445566",
        "--glyph-stroke", "n\u{303}", "2200", "2500", "2700", "3000",
        "200", "#445566",
    ])).expect("source admitted").expect("source bytes");
    fs::write(&source, &original).expect("selected source");
    let revised = execute_glyph_library(&args(&[
        "--profile", source_name,
        "--style-path", "n\u{303}", "1", "501", "#abcdef",
        "--style-path", "n\u{303}", "2", "125", "#010203",
    ])).expect("restyle both candidate paths").expect("TSV bytes");
    let expected = concat!(
        "atrament.glyph-strokes.v1\n",
        "n\u{303}\t500\t700\t1000\t1400\t501\t#ABCDEF\n",
        "ñ\t2200\t600\t2200\t3300\t300\t#778899\n",
        "n\u{303}\t1000\t1400\t1500\t2200\t501\t#ABCDEF\n",
        "∞\t800\t800\t1700\t1500\t200\t#445566\n",
        "n\u{303}\t2200\t2500\t2700\t3000\t125\t#010203\n",
    ).as_bytes();
    assert_eq!(revised, expected);
    assert_eq!(fs::read(&source).expect("unchanged source"), original);
    assert!(execute_glyph_library(&args(&[
        "--profile", source_name, "--style-path", "n\u{303}",
        "1", "501", "#abcdef", "--style-path", "n\u{303}",
        "2", "125", "#010203", "--output", revision_name,
    ])).expect("explicit file output").is_none());
    assert_eq!(fs::read(&revision).expect("saved revision"), expected);
    assert_eq!(execute_glyph_library(&args(&[
        "--profile", revision_name,
    ])).expect("reopened").expect("exact bytes"), expected);
    let report = String::from_utf8(execute_glyph_library(&args(&[
        "--profile", revision_name, "--inspect-paths", "n\u{303}",
    ])).expect("path inspection").expect("report")).expect("UTF-8");
    assert!(report.contains("Strokes: 3\nPaths: 2\nPen lifts: 1"),
        "{report}");
    assert!(report.contains("width 501 um color #ABCDEF"), "{report}");
    for format in ["pdf", "png"] {
        let mut saved = args(&[
            "--export-paper", format, "--printed-sheet", "true",
            "--glyph-file", revision_name,
            "--place-text", "1", "30000", "45000", "ñn\u{303}",
        ]);
        let mut direct = args(&[
            "--export-paper", format, "--printed-sheet", "true",
            "--glyph-stroke", "ñ", "2200", "600", "2200", "3300",
            "300", "#778899",
            "--glyph-stroke", "n\u{303}", "500", "700", "1000",
            "1400", "501", "#ABCDEF",
            "--glyph-stroke", "n\u{303}", "1000", "1400", "1500",
            "2200", "501", "#ABCDEF",
            "--glyph-stroke", "n\u{303}", "2200", "2500", "2700",
            "3000", "125", "#010203",
            "--place-text", "1", "30000", "45000", "ñn\u{303}",
        ]);
        if format == "png" {
            saved.extend(args(&["--dpi", "72"]));
            direct.extend(args(&["--dpi", "72"]));
        }
        let actual = execute_notebook(&saved)
            .expect("saved ink").expect("rendered bytes");
        let expected_ink = execute_notebook(&direct)
            .expect("direct ink").expect("rendered bytes");
        assert_eq!(actual, expected_ink, "path style parity in {format}");
        if format == "pdf" {
            assert!(actual.starts_with(b"%PDF-1.4"));
        } else {
            assert!(actual.starts_with(b"\x89PNG\r\n\x1a\n"));
        }
    }
    fs::remove_file(source).expect("remove owned source");
    fs::remove_file(revision).expect("remove owned revision");
}

#[test]
fn native_path_pen_style_refuses_invalid_inputs_without_file_writes() {
    let source = test_path().with_extension("tsv");
    let output = test_path().with_extension("tsv");
    let source_name = source.to_str().expect("source");
    let output_name = output.to_str().expect("output");
    let original = execute_glyph_library(&args(&[
        "--glyph-stroke", "n\u{303}", "125", "400", "900",
        "1500", "250", "#112233",
        "--glyph-stroke", "ñ", "2200", "600", "2400",
        "3300", "300", "#778899",
    ])).expect("source").expect("TSV");
    fs::write(&source, &original).expect("own source");
    fs::write(&output, b"target sentinel").expect("own output");
    for (index, rejected) in [
        args(&["--style-path"]),
        args(&["--style-path", "n\u{303}"]),
        args(&["--style-path", "n\u{303}", "1"]),
        args(&["--style-path", "n\u{303}", "1", "250"]),
        args(&["--style-path", "n\u{303}", "0", "250", "#102030"]),
        args(&["--style-path", "n\u{303}", "2", "250", "#102030"]),
        args(&["--style-path", "n\u{303}", "1", "79", "#102030"]),
        args(&["--style-path", "n\u{303}", "1", "1501", "#102030"]),
        args(&["--style-path", "n\u{303}", "1", "+250", "#102030"]),
        args(&["--style-path", "n\u{303}", "1", "1.5", "#102030"]),
        args(&["--style-path", "n\u{303}", "1", "250", "102030"]),
        args(&["--style-path", "n\u{303}", "1", "250", "#GGGGGG"]),
        args(&["--style-path", "ab", "1", "250", "#102030"]),
        args(&["--style-path", " ", "1", "250", "#102030"]),
        args(&["--style-path", "n\u{303}", "1", "250", "#112233"]),
        args(&["--style-path", "n\u{303}", "1", "250", "#112234"]),
        args(&["--style-path", "n\u{303}", "1", "301", "#102030"]),
    ].into_iter().enumerate() {
        let mut command = args(&["--profile", source_name]);
        command.extend(rejected.clone());
        command.extend(args(&["--output", output_name, "--replace"]));
        let result = execute_glyph_library(&command);
        if index == 15 {
            // Recolor-only remains admissible even close to a cell edge.
            assert!(result.is_ok(), "{result:?}");
            fs::write(&output, b"target sentinel").expect("restore sentinel");
            continue;
        }
        assert!(result.is_err(), "unsafe edit accepted: {rejected:?}");
        assert_eq!(fs::read(&source).expect("source"), original);
        assert_eq!(fs::read(&output).expect("target"), b"target sentinel");
        let mut piped = args(&["--profile", "-"]);
        piped.extend(rejected.clone());
        let preflight = preflight_glyph_profile_sources(&piped);
        // Path existence, unchanged style and clipping depend on source ink.
        if matches!(index, 5 | 14 | 16) {
            assert!(preflight.is_ok(), "{rejected:?}: {preflight:?}");
        } else {
            assert!(preflight.is_err(),
                "{rejected:?}: invalid syntax must reject stdin");
        }
    }
    fs::remove_file(source).expect("remove source");
    fs::remove_file(output).expect("remove target");
}

#[test]
fn native_shifted_path_copy_saves_reopens_and_renders_both_formats() {
    let source = test_path().with_extension("tsv");
    let revision = test_path().with_extension("tsv");
    let source_name = source.to_str().expect("source filename");
    let revision_name = revision.to_str().expect("revision filename");
    let original = execute_glyph_library(&args(&[
        "--glyph-stroke", "n\u{303}", "500", "700", "1000", "1400",
        "251", "#112233",
        "--glyph-stroke", "ñ", "2200", "600", "2200", "3300",
        "300", "#778899",
        "--glyph-stroke", "n\u{303}", "1000", "1400", "1500", "2200",
        "251", "#112233",
        "--glyph-stroke", "∞", "800", "800", "1700", "1500",
        "200", "#445566",
    ])).expect("source admitted").expect("source bytes");
    fs::write(&source, &original).expect("write selected source");
    let mut commands = args(&[
        "--profile", source_name,
        "--copy-path-shifted", "n\u{303}", "1", "ñ", "-100", "200",
        "--copy-path-shifted", "n\u{303}", "1", "n\u{303}",
        "300", "-200",
    ]);
    let output = execute_glyph_library(&commands)
        .expect("two shifted copies").expect("revised TSV");
    let mut expected = original.clone();
    expected.extend_from_slice(concat!(
        "ñ\t400\t900\t900\t1600\t251\t#112233\n",
        "ñ\t900\t1600\t1400\t2400\t251\t#112233\n",
        "n\u{303}\t800\t500\t1300\t1200\t251\t#112233\n",
        "n\u{303}\t1300\t1200\t1800\t2000\t251\t#112233\n",
    ).as_bytes());
    assert_eq!(output, expected);
    assert_eq!(fs::read(&source).expect("unchanged source"), original);
    commands.extend(args(&["--output", revision_name]));
    assert!(execute_glyph_library(&commands)
        .expect("selected output saved").is_none());
    assert_eq!(fs::read(&revision).expect("saved source"), expected);
    let roundtrip = execute_glyph_library(&args(&[
        "--profile", revision_name,
    ])).expect("reimported library").expect("reimported bytes");
    assert_eq!(roundtrip, expected);
    let report = String::from_utf8(execute_glyph_library(&args(&[
        "--profile", revision_name, "--inspect-paths", "ñ",
    ])).expect("inspect path").expect("report"))
        .expect("UTF-8 report");
    assert!(report.contains("Strokes: 3"), "{report}");
    for format in ["pdf", "png"] {
        let mut saved = args(&[
            "--export-paper", format, "--printed-sheet", "true",
            "--glyph-file", revision_name,
            "--place-text", "1", "30000", "45000", "ñn\u{303}",
        ]);
        let mut direct = args(&[
            "--export-paper", format, "--printed-sheet", "true",
            "--glyph-stroke", "ñ", "2200", "600", "2200", "3300",
            "300", "#778899",
            "--glyph-stroke", "ñ", "400", "900", "900", "1600",
            "251", "#112233",
            "--glyph-stroke", "ñ", "900", "1600", "1400", "2400",
            "251", "#112233",
            "--glyph-stroke", "n\u{303}", "500", "700", "1000", "1400",
            "251", "#112233",
            "--glyph-stroke", "n\u{303}", "1000", "1400", "1500",
            "2200", "251", "#112233",
            "--glyph-stroke", "n\u{303}", "800", "500", "1300",
            "1200", "251", "#112233",
            "--glyph-stroke", "n\u{303}", "1300", "1200", "1800",
            "2000", "251", "#112233",
            "--place-text", "1", "30000", "45000", "ñn\u{303}",
        ]);
        if format == "png" {
            saved.extend(args(&["--dpi", "72"]));
            direct.extend(args(&["--dpi", "72"]));
        }
        let actual = execute_notebook(&saved)
            .expect("saved shifted ink accepted").expect("native bytes");
        let expected_ink = execute_notebook(&direct)
            .expect("independent ink accepted").expect("native bytes");
        assert_eq!(actual, expected_ink,
            "shifted ink must render identically in {format}");
        if format == "pdf" {
            assert!(actual.starts_with(b"%PDF-1.4"));
        } else {
            assert!(actual.starts_with(b"\x89PNG\r\n\x1a\n"));
        }
    }
    fs::remove_file(source).expect("remove original");
    fs::remove_file(revision).expect("remove revision");
}

#[test]
fn shifted_path_copy_preflights_and_refuses_atomically() {
    let source = test_path().with_extension("tsv");
    let output = test_path().with_extension("tsv");
    let source_name = source.to_str().expect("source path");
    let output_name = output.to_str().expect("target path");
    let original = execute_glyph_library(&args(&[
        "--glyph-stroke", "n\u{303}", "125", "700", "1000", "1400",
        "251", "#112233",
        "--glyph-stroke", "ñ", "2200", "600", "2200", "3300",
        "300", "#778899",
    ])).expect("source admitted").expect("source bytes");
    fs::write(&source, &original).expect("source saved");
    fs::write(&output, b"selected output sentinel")
        .expect("output sentinel");
    let invalid = [
        args(&["--copy-path-shifted"]),
        args(&["--copy-path-shifted", "n\u{303}"]),
        args(&["--copy-path-shifted", "n\u{303}", "1"]),
        args(&["--copy-path-shifted", "n\u{303}", "1", "ñ"]),
        args(&["--copy-path-shifted", "n\u{303}", "1", "ñ", "0"]),
        args(&["--copy-path-shifted", "n\u{303}", "0", "ñ", "1", "2"]),
        args(&["--copy-path-shifted", "n\u{303}", "2", "ñ", "1", "2"]),
        args(&["--copy-path-shifted", "n\u{303}", "1", "ñ", "0", "0"]),
        args(&["--copy-path-shifted", "n\u{303}", "1", "ñ", "1.0", "2"]),
        args(&["--copy-path-shifted", "n\u{303}", "1", "ñ", "+1", "2"]),
        args(&["--copy-path-shifted", "n\u{303}", "1", "ñ", "3501", "0"]),
        args(&["--copy-path-shifted", "n\u{303}", "1", "ñ", "1", "-4501"]),
        args(&["--copy-path-shifted", "n\u{303}", "1", "ñ", "-1", "0"]),
        args(&["--copy-path-shifted", "n\u{303}", "1", "ñ", "0", "-600"]),
        args(&["--copy-path-shifted", "n\u{303}", "1", "ab", "1", "2"]),
        args(&["--copy-path-shifted", " ", "1", "ñ", "1", "2"]),
        args(&["--copy-path-shifted", "n\u{303}", "1", " ", "1", "2"]),
    ];
    for (index, rejected) in invalid.into_iter().enumerate() {
        let mut command = args(&["--profile", source_name]);
        command.extend(rejected.clone());
        command.extend(["--output", output_name, "--replace"]
            .map(str::to_owned));
        assert!(execute_glyph_library(&command).is_err(),
            "invalid shifted copy accepted: {rejected:?}");
        assert_eq!(fs::read(&source).expect("source unchanged"), original);
        assert_eq!(fs::read(&output).expect("sentinel unchanged"),
            b"selected output sentinel");
        // All malformed operands must be rejected before opening stdin.
        let mut piped = args(&["--profile", "-"]);
        piped.extend(rejected);
        let preflight = preflight_glyph_profile_sources(&piped);
        if matches!(index, 6 | 12 | 13) {
            preflight.expect("path existence and cell clipping need ink");
        } else {
            let _error = preflight.expect_err(
                "malformed arguments must refuse before IO");
        }
    }
    let full = String::from("atrament.glyph-strokes.v1\n")
        + &"n\u{303}\t500\t700\t1100\t1500\t251\t#112233\n"
            .repeat(4096);
    fs::write(&source, full.as_bytes()).expect("full source");
    let refused = execute_glyph_library(&args(&[
        "--profile", source_name,
        "--copy-path-shifted", "n\u{303}", "1", "ñ", "1", "1",
        "--output", output_name, "--replace",
    ])).expect_err("capacity refusal");
    assert!(refused.contains("4096"), "{refused}");
    assert_eq!(fs::read(&output).expect("sentinel"),
        b"selected output sentinel");
    fs::remove_file(source).expect("remove source");
    fs::remove_file(output).expect("remove output");
}

#[test]
fn native_path_copy_refuses_unsafe_inputs_and_preserves_files() {
    let source = test_path().with_extension("tsv");
    let target = test_path().with_extension("tsv");
    let source_name = source.to_str().expect("source path");
    let target_name = target.to_str().expect("target path");
    let original = execute_glyph_library(&args(&[
        "--glyph-stroke", "n\u{303}", "500", "700", "1100", "1500",
        "200", "#112233",
        "--glyph-stroke", "ñ", "2200", "600", "2200", "3300",
        "300", "#778899",
    ])).expect("valid library").expect("canonical TSV");
    fs::write(&source, &original).expect("owned source");
    fs::write(&target, b"unchanged target sentinel").expect("sentinel");
    for rejected in [
        args(&["--copy-path"]),
        args(&["--copy-path", "n\u{303}"]),
        args(&["--copy-path", "n\u{303}", "1"]),
        args(&["--copy-path", "n\u{303}", "0", "á"]),
        args(&["--copy-path", "n\u{303}", "-1", "á"]),
        args(&["--copy-path", "n\u{303}", "1.0", "á"]),
        args(&["--copy-path", "n\u{303}", "4097", "á"]),
        args(&["--copy-path", "n\u{303}", "2", "á"]),
        args(&["--copy-path", "n", "1", "á"]),
        args(&["--copy-path", "n\u{303}", "1", "n\u{303}"]),
        args(&["--copy-path", "n\u{303}", "1", "ab"]),
        args(&["--copy-path", " ", "1", "á"]),
        args(&["--copy-path", "n\u{303}", "1", " "]),
        args(&["--copy-path", "n\u{303}", "1", "\n"]),
    ] {
        let mut command = args(&["--profile", source_name]);
        command.extend(rejected);
        command.extend(args(&["--output", target_name, "--replace"]));
        assert!(execute_glyph_library(&command).is_err(),
            "invalid path copy accepted: {command:?}");
        assert_eq!(fs::read(&source).expect("source bytes"), original);
        assert_eq!(fs::read(&target).expect("target bytes"),
            b"unchanged target sentinel");
    }
    let full = String::from("atrament.glyph-strokes.v1\n")
        + &"n\u{303}\t500\t700\t1100\t1500\t200\t#112233\n".repeat(4096);
    fs::write(&source, full.as_bytes()).expect("full source library");
    let error = execute_glyph_library(&args(&[
        "--profile", source_name, "--copy-path", "n\u{303}", "1", "á",
        "--output", target_name, "--replace",
    ])).expect_err("over-budget copy refuses");
    assert!(error.contains("4096 authored strokes"), "{error}");
    assert_eq!(fs::read(&source).expect("full source"), full.as_bytes());
    assert_eq!(fs::read(&target).expect("sentinel"),
        b"unchanged target sentinel");
    fs::remove_file(source).expect("remove source");
    fs::remove_file(target).expect("remove sentinel");
}

#[test]
fn explicit_glyph_copy_refuses_ambiguous_targets_and_stroke_budget() {
    let source = test_path().with_extension("tsv");
    let target = test_path().with_extension("tsv");
    let source_name = source.to_str().expect("owned source");
    let output_name = target.to_str().expect("owned target");
    let original = execute_glyph_library(&args(&[
        "--glyph-stroke", "o", "1000", "500", "2000", "3500",
        "250", "#203040",
        "--glyph-stroke", "a", "1000", "500", "2000", "3500",
        "250", "#556677",
        "--glyph-stroke", "n\u{303}", "1000", "500", "2000",
        "3500", "250", "#AABBCC",
    ])).expect("three authored graphemes").expect("TSV");
    fs::write(&source, &original).expect("test source");
    fs::write(&target, b"preserve prior target")
        .expect("test sentinel");
    let invalid = [
        args(&["--copy-glyph"]),
        args(&["--copy-glyph", "o"]),
        args(&["--copy-glyph", "o", "o"]),
        args(&["--copy-glyph", "o", "a"]),
        args(&["--copy-glyph", "a", "o"]),
        args(&["--copy-glyph", "n", "b"]),
        args(&["--copy-glyph", "b", "o"]),
        args(&["--copy-glyph", "o", "ab"]),
        args(&["--copy-glyph", "o", " "]),
        args(&["--copy-glyph", "\n", "b"]),
        args(&["--copy-glyph", "o", "\n"]),
        args(&["--copy-glyph", "o", "b",
            "--copy-glyph", "o", "b"]),
        args(&["--copy-glyph", "o", "b",
            "--copy-glyph", "b", "o"]),
    ];
    for flags in invalid {
        let mut command = args(&["--profile", source_name]);
        command.extend(flags);
        command.extend(args(&["--output", output_name, "--replace"]));
        assert!(execute_glyph_library(&command).is_err(),
            "invalid copy must refuse: {command:?}");
        assert_eq!(fs::read(&source).expect("unchanged source"), original);
        assert_eq!(fs::read(&target).expect("unchanged target"),
            b"preserve prior target");
    }
    let permitted = execute_glyph_library(&args(&[
        "--profile", source_name, "--copy-glyph", "n\u{303}", "ñ",
        "--copy-glyph", "ñ", "b",
    ])).expect("explicit successive independent copies").expect("TSV");
    let permitted = String::from_utf8(permitted).expect("valid UTF-8");
    assert!(permitted.contains("ñ\t1000\t500\t2000\t3500"));
    assert!(permitted.contains("b\t1000\t500\t2000\t3500"));
    assert!(permitted.as_bytes().starts_with(&original),
        "explicit copies preserve all previous authored records");
    assert_eq!(fs::read(&source).expect("no implicit source write"),
        original);

    let big = String::from("atrament.glyph-strokes.v1\n")
        + &"o\t1000\t500\t2000\t3500\t250\t#203040\n".repeat(2049);
    fs::write(&source, big.as_bytes()).expect("bounded large library");
    let error = execute_glyph_library(&args(&[
        "--profile", source_name, "--copy-glyph", "o", "a",
        "--output", output_name, "--replace",
    ])).expect_err("copy across whole-file segment budget must refuse");
    assert!(error.contains("4096 authored strokes"), "{error}");
    assert_eq!(fs::read(&source).expect("large source stays unchanged"),
        big.as_bytes());
    assert_eq!(fs::read(&target).expect("sentinel stays unchanged"),
        b"preserve prior target");
    fs::remove_file(source).expect("remove test source");
    fs::remove_file(target).expect("remove test target");
}

#[test]
fn targeted_stroke_geometry_and_deletion_round_trip_into_native_exports() {
    let source = test_path().with_extension("tsv");
    let revised = test_path().with_extension("tsv");
    let original_name = source.to_str().expect("test source");
    let revised_name = revised.to_str().expect("test revision");
    let original = execute_glyph_library(&args(&[
        "--glyph-stroke", "n\u{303}", "500", "700", "900", "1300",
        "250", "#102030",
        "--glyph-stroke", "ñ", "1100", "900", "2200", "2700",
        "300", "#445566",
        "--glyph-stroke", "n\u{303}", "900", "1300", "1800",
        "2600", "350", "#708090",
        "--glyph-stroke", "∞", "1200", "600", "2300", "1700",
        "400", "#AA5500",
        "--glyph-stroke", "n\u{303}", "1800", "2600", "3000",
        "3900", "450", "#BBCCDD",
    ])).expect("initial canonical TSV").expect("bytes");
    fs::write(&source, &original).expect("test source saved");
    let mut changes = args(&[
        "--profile", original_name,
        "--set-stroke", "n\u{303}", "2", "1000", "500", "2900",
        "3300", "500", "#123456",
        "--remove-stroke", "n\u{303}", "1",
        "--move-stroke", "n\u{303}", "2", "1",
        "--reverse-stroke", "n\u{303}", "2",
    ]);
    let updated = execute_glyph_library(&changes)
        .expect("native geometry edit").expect("updated TSV");
    let expected = concat!(
        "atrament.glyph-strokes.v1\n",
        "ñ\t1100\t900\t2200\t2700\t300\t#445566\n",
        "ñ\t1800\t2600\t3000\t3900\t450\t#BBCCDD\n",
        "∞\t1200\t600\t2300\t1700\t400\t#AA5500\n",
        "ñ\t2900\t3300\t1000\t500\t500\t#123456\n",
    );
    assert_eq!(updated, expected.as_bytes());
    assert_eq!(fs::read(&source).expect("no implicit write"), original);
    changes.extend(args(&["--inspect-glyph", "n\u{303}"]));
    let report = String::from_utf8(execute_glyph_library(&changes)
        .expect("inspect changed draft").expect("report"))
        .expect("UTF-8 inspection");
    assert!(report.contains("Strokes: 2"), "{report}");
    assert!(report.contains("1: (1800, 2600) -> (3000, 3900)"),
        "{report}");
    assert!(report.contains(
        "2: (2900, 3300) -> (1000, 500) width 500 um color #123456"
    ), "{report}");

    assert!(execute_glyph_library(&args(&[
        "--profile", original_name,
        "--set-stroke", "n\u{303}", "2", "1000", "500", "2900",
        "3300", "500", "#123456",
        "--remove-stroke", "n\u{303}", "1",
        "--move-stroke", "n\u{303}", "2", "1",
        "--reverse-stroke", "n\u{303}", "2",
        "--output", revised_name,
    ])).expect("explicit save").is_none());
    assert_eq!(fs::read(&revised).expect("saved native library"), updated);

    for format in ["pdf", "png"] {
        let mut stored = args(&[
            "--export-paper", format, "--printed-sheet", "true",
            "--glyph-file", revised_name, "--place-text", "1",
            "30000", "45000", "n\u{303}ñ∞",
        ]);
        let mut inline = args(&[
            "--export-paper", format, "--printed-sheet", "true",
            "--glyph-stroke", "ñ", "1100", "900", "2200", "2700",
            "300", "#445566",
            "--glyph-stroke", "n\u{303}", "1800", "2600", "3000",
            "3900", "450", "#BBCCDD",
            "--glyph-stroke", "n\u{303}", "2900", "3300", "1000",
            "500", "500", "#123456",
            "--glyph-stroke", "∞", "1200", "600", "2300", "1700",
            "400", "#AA5500",
            "--place-text", "1", "30000", "45000", "n\u{303}ñ∞",
        ]);
        if format == "png" {
            stored.extend(args(&["--dpi", "72"]));
            inline.extend(args(&["--dpi", "72"]));
        }
        assert_eq!(execute_notebook(&stored).expect("saved authored ink")
            .expect("native bytes"), execute_notebook(&inline)
            .expect("inline authored ink").expect("native bytes"),
            "edited segment's order and direction must reach {format}");
    }
    fs::remove_file(source).expect("remove test source");
    fs::remove_file(revised).expect("remove test revision");
}

#[test]
fn selected_text_join_matches_explicit_page_ink_in_pdf_and_png() {
    // Two independently authored NFC/NFD shapes: their physically placed
    // pen endpoints are 1000 um apart on the selected output row.
    let library = test_path().with_extension("tsv");
    let name = library.to_str().expect("owned TSV path");
    let original = execute_glyph_library(&args(&[
        "--glyph-stroke", "n\u{303}", "500", "1000", "3200", "2200",
        "250", "#102030",
        "--glyph-stroke", "ó", "200", "2200", "1200", "1000",
        "250", "#102030",
    ])).expect("authored exact ink").expect("TSV bytes");
    fs::write(&library, &original).expect("own canonical library");
    for format in ["pdf", "png"] {
        let mut connected = args(&[
            "--printed-sheet", "true", "--export-paper", format,
            "--glyph-file", name,
            "--place-text", "1", "30000", "45000", "n\u{303}ó",
            "--join-text-pair", "n\u{303}", "ó",
        ]);
        let mut manual = args(&[
            "--printed-sheet", "true", "--export-paper", format,
            "--ink-line", "1", "30500", "46000", "33200",
            "47200", "250", "#102030",
            "--ink-line", "1", "33200", "47200", "34200",
            "47200", "250", "#102030",
            "--ink-line", "1", "34200", "47200", "35200",
            "46000", "250", "#102030",
        ]);
        let mut independent = args(&[
            "--printed-sheet", "true", "--export-paper", format,
            "--glyph-file", name,
            "--place-text", "1", "30000", "45000", "n\u{303}ó",
        ]);
        if format == "png" {
            for flags in [&mut connected, &mut manual, &mut independent] {
                flags.extend(args(&["--dpi", "72"]));
            }
        }
        let actual = execute_notebook(&connected).expect("joined export")
            .expect("export bytes");
        assert_eq!(actual, execute_notebook(&manual)
            .expect("authored independent connector")
            .expect("export bytes"),
            "real {format} must draw connector in pen stroke order");
        assert_ne!(actual, execute_notebook(&independent)
            .expect("ordinary glyphs")
            .expect("export bytes"),
            "join is not silently present without opt-in");
        if format == "pdf" {
            assert!(actual.starts_with(b"%PDF-"));
        } else {
            assert!(actual.starts_with(b"\x89PNG\r\n\x1a\n"));
        }
    }
    assert_eq!(fs::read(&library).expect("canonical source"), original);
    fs::remove_file(library).expect("remove owned TSV");
}

#[test]
fn selected_text_joins_reset_at_spaces_runs_and_page_boundaries() {
    let mut overlay = GlyphOverlay::default();
    overlay.parse("--glyph-stroke", &mut args(&[
        "n\u{303}", "500", "1000", "3200", "2200",
        "250", "#102030",
    ]).iter()).expect("source glyph");
    overlay.parse("--glyph-stroke", &mut args(&[
        "ó", "200", "2200", "1200", "1000",
        "250", "#102030",
    ]).iter()).expect("next glyph");
    overlay.add_text_join_pair(&mut args(&["n\u{303}", "ó"]).iter())
        .expect("selected Unicode join");
    overlay.parse("--place-text", &mut args(&[
        "1", "30000", "45000", "n\u{303}ó n\u{303}ó",
    ]).iter()).expect("same page repeated pair with space");
    overlay.parse("--place-text", &mut args(&[
        "2", "30000", "45000", "n\u{303}ó",
    ]).iter()).expect("second page");
    let expanded = overlay.expand(&[], None).expect("both pages rendered");
    assert_eq!(expanded.len(), 9,
        "three pairs of authored glyphs plus three explicit bridges");
    let bridges = expanded.iter().filter(|line| {
        line.start_um == (33_200, 47_200)
            && line.end_um == (34_200, 47_200)
    }).collect::<Vec<_>>();
    assert_eq!(bridges.len(), 2,
        "same row bridges on both pages, with no cross-run connector");
    assert_eq!(bridges[0].page, 1);
    assert_eq!(bridges[1].page, 2);
    assert!(expanded.iter().any(|line| {
        line.page == 1 && line.start_um == (45_200, 47_200)
            && line.end_um == (46_200, 47_200)
    }), "the second same-row pair is connected at its actual advance");
    assert!(expanded.iter().all(|line| line.width_um == 250));

    // A single selected pair split by a space or by a run must refuse,
    // not silently claim a connected word or connect two physical pages.
    for runs in [
        args(&["--place-text", "1", "30000", "45000", "n\u{303} ó"]),
        args(&["--place-text", "1", "30000", "45000", "n\u{303}",
            "--place-text", "1", "34000", "45000", "ó"]),
        args(&["--place-text", "1", "30000", "45000", "n\u{303}",
            "--place-text", "2", "34000", "45000", "ó"]),
    ] {
        let mut command = args(&[
            "--export-paper", "pdf",
            "--glyph-stroke", "n\u{303}", "500", "1000", "3200",
            "2200", "250", "#102030",
            "--glyph-stroke", "ó", "200", "2200", "1200",
            "1000", "250", "#102030",
            "--join-text-pair", "n\u{303}", "ó",
        ]);
        command.extend(runs);
        assert!(execute_notebook(&command).is_err(),
            "no adjacent glyphs within a single authored run: {command:?}");
    }
}

#[test]
fn selected_text_join_uses_seeded_physical_pen_not_nominal_geometry() {
    let mut overlay = GlyphOverlay::default();
    overlay.parse("--glyph-stroke", &mut args(&[
        "n\u{303}", "500", "1000", "3200", "2200",
        "250", "#102030",
    ]).iter()).expect("authored left letter");
    overlay.parse("--glyph-stroke", &mut args(&[
        "ó", "200", "2200", "1200", "1000",
        "250", "#102030",
    ]).iter()).expect("authored right letter");
    overlay.parse("--place-text", &mut args(&[
        "1", "30000", "45000", "n\u{303}ó",
    ]).iter()).expect("one physical text run");
    overlay.parse("--glyph-variation", &mut args(&[
        "42", "2", "2", "0", "1", "10", "10",
    ]).iter()).expect("bounded seeded variation");
    let canonical = overlay.encode_file().expect("authored TSV");
    let original = overlay.expand(&[], None).expect("unjoined varied ink");
    assert_eq!(original.len(), 2);
    overlay.add_text_join_pair(&mut args(&["n\u{303}", "ó"]).iter())
        .expect("explicit join pair");
    let connected = overlay.expand(&[], None).expect("connected varied ink");
    assert_eq!(connected.len(), 3);
    assert_eq!(connected[0], original[0]);
    assert_eq!(connected[2], original[1]);
    assert_eq!(connected[1].start_um, connected[0].end_um);
    assert_eq!(connected[1].end_um, connected[2].start_um);
    assert_eq!(connected[1].width_um, connected[0].width_um);
    assert_eq!(connected[1].rgb, connected[2].rgb);
    assert_ne!(connected[1].start_um, (33_200, 47_200),
        "connector follows real seeded endpoints, not ideal coordinates");
    assert_eq!(connected, overlay.expand(&[], None)
        .expect("bit-identical seeded replay"));
    assert_eq!(overlay.encode_file().expect("unmodified source"), canonical,
        "text-level joins must never insert new strokes into glyph TSV");
}

#[test]
fn selected_text_join_supports_native_measured_two_row_flow() {
    for format in ["pdf", "png"] {
        let mut connected = args(&[
            "--printed-sheet", "true", "--export-paper", format,
            "--glyph-stroke", "n\u{303}", "500", "1000", "3200",
            "2200", "250", "#102030",
            "--glyph-stroke", "ó", "200", "2200", "1200",
            "1000", "250", "#102030",
            "--flow-text", "n\u{303}ó\nn\u{303}ó",
            "--join-text-pair", "n\u{303}", "ó",
        ]);
        if format == "png" {
            connected.extend(args(&["--dpi", "72"]));
        }
        let joined = execute_notebook(&connected).expect("measured native flow")
            .expect("real output");
        assert!(if format == "pdf" {
            joined.starts_with(b"%PDF-")
        } else {
            joined.starts_with(b"\x89PNG\r\n\x1a\n")
        });
        let mut unjoined = connected.clone();
        let position = unjoined.iter().position(|flag| {
            flag == "--join-text-pair"
        }).expect("explicit join option");
        let _removed = unjoined.drain(position..position + 3)
            .collect::<Vec<_>>();
        assert_ne!(joined, execute_notebook(&unjoined)
            .expect("original unjoined flow").expect("real output"));
    }
    // A forced newline creates a new physical run, not a cursive link.
    let refused = execute_notebook(&args(&[
        "--export-paper", "pdf",
        "--glyph-stroke", "n\u{303}", "500", "1000", "3200",
        "2200", "250", "#102030",
        "--glyph-stroke", "ó", "200", "2200", "1200",
        "1000", "250", "#102030",
        "--flow-text", "n\u{303}\nó",
        "--join-text-pair", "n\u{303}", "ó",
    ])).expect_err("explicit line boundary must not silently connect");
    assert!(refused.contains("not adjacent"), "{refused}");
}

#[test]
fn selected_text_pair_accepts_exact_gap_limit_and_refuses_mismatched_pen() {
    let output = test_path().with_extension("pdf");
    let name = output.to_str().expect("owned output path");
    fs::write(&output, b"preserve original PDF")
        .expect("test sentinel");
    let compose = |last_x: &str, next_width: &str, next_rgb: &str| {
        args(&[
            "--printed-sheet", "true", "--export-paper", "pdf",
            "--glyph-stroke", "a", "200", "1000", last_x,
            "2200", "250", "#102030",
            "--glyph-stroke", "o", "200", "2200", "1200",
            "1000", next_width, next_rgb,
            "--place-text", "1", "30000", "45000", "ao",
            "--join-text-pair", "a", "o",
        ])
    };
    let allowed = execute_notebook(&compose("3000", "250", "#102030"))
        .expect("exact 1200 um gap must be allowed").expect("real PDF");
    assert!(allowed.starts_with(b"%PDF-"));
    for (last_x, width, rgb, problem) in [
        ("2999", "250", "#102030", "text join gap exceeds 1200"),
        ("3000", "251", "#102030", "matching physical pen"),
        ("3000", "250", "#304050", "matching physical pen"),
    ] {
        let mut command = compose(last_x, width, rgb);
        command.extend(args(&["--output", name, "--replace"]));
        let error = execute_notebook(&command).expect_err("unsafe join");
        assert!(error.contains(problem), "{error}");
        assert_eq!(fs::read(&output).expect("selected PDF unchanged"),
            b"preserve original PDF");
    }
    fs::remove_file(output).expect("remove owned sentinel");
}

#[test]
fn selected_text_pair_budget_is_bounded_before_any_export() {
    let mut overlay = GlyphOverlay::default();
    for target in "ABCDEFGHIJKLMNOPQRSTUVWXYZ012345".chars() {
        let right = target.to_string();
        overlay.add_text_join_pair(&mut args(&["a", &right]).iter())
            .expect("up to 32 distinct Unicode pairs");
    }
    assert_eq!(overlay.text_joins.len(), 32);
    let error = overlay.add_text_join_pair(&mut args(&["a", "6"]).iter())
        .expect_err("33rd pair refuses before export");
    assert!(error.contains("32 selected pairs"), "{error}");
    assert_eq!(overlay.text_joins.len(), 32,
        "invalid pair never partially changes the selection");
}

#[test]
fn selected_text_join_refusals_preserve_prior_pdf_and_glyph_source() {
    let source = test_path().with_extension("tsv");
    let output = test_path().with_extension("pdf");
    let source_name = source.to_str().expect("owned source");
    let output_name = output.to_str().expect("owned output");
    let original = execute_glyph_library(&args(&[
        "--glyph-stroke", "a", "200", "1000", "2000", "2200",
        "250", "#102030",
        "--glyph-stroke", "o", "200", "2200", "1200", "1000",
        "250", "#102030",
    ])).expect("canonical glyphs").expect("TSV");
    fs::write(&source, &original).expect("test source");
    fs::write(&output, b"retain original PDF sentinel")
        .expect("test output");
    let base = args(&[
        "--printed-sheet", "true", "--export-paper", "pdf",
        "--glyph-file", source_name,
        "--place-text", "1", "30000", "45000", "ao",
    ]);
    let variants = [
        args(&["--join-text-pair"]),
        args(&["--join-text-pair", "a"]),
        args(&["--join-text-pair", "ab", "o"]),
        args(&["--join-text-pair", "a", "o o"]),
        args(&["--join-text-pair", "a", " "]),
        args(&["--join-text-pair", "a", "\n"]),
        args(&["--join-text-pair", "a", "o",
            "--join-text-pair", "a", "o"]),
        args(&["--join-text-pair", "o", "a"]),
        args(&["--join-text-pair", "a", "o"]),
    ];
    for flags in variants {
        let mut command = base.clone();
        command.extend(flags);
        command.extend(args(&["--output", output_name, "--replace"]));
        assert!(execute_notebook(&command).is_err(),
            "invalid or unsafe text join must refuse: {command:?}");
        assert_eq!(fs::read(&source).expect("unchanged glyph source"),
            original);
        assert_eq!(fs::read(&output).expect("output sentinel"),
            b"retain original PDF sentinel");
    }
    // This 2200-um pen gap refuses only opted-in joins: plain
    // independent glyph placement must remain valid and unchanged.
    let plain = execute_notebook(&base).expect("ordinary unjoined output")
        .expect("printable PDF");
    assert!(plain.starts_with(b"%PDF-"));
    fs::remove_file(source).expect("remove source");
    fs::remove_file(output).expect("remove sentinel");
}

#[test]
fn explicit_joins_connect_one_exact_glyph_in_ink_and_native_exports() {
    let source = test_path().with_extension("tsv");
    let target = test_path().with_extension("tsv");
    let source_name = source.to_str().expect("join source");
    let target_name = target.to_str().expect("join output");
    let original = execute_glyph_library(&args(&[
        "--glyph-stroke", "n\u{303}", "500", "600", "1100",
        "1400", "250", "#102030",
        "--glyph-stroke", "\u{f1}", "1000", "1000", "1200",
        "2200", "350", "#AABBCC",
        "--glyph-stroke", "n\u{303}", "1500", "1800", "2200",
        "2600", "250", "#102030",
    ])).expect("authored ink").expect("source TSV");
    fs::write(&source, &original).expect("test source");
    let joined = execute_glyph_library(&args(&[
        "--profile", source_name, "--join-strokes", "n\u{303}", "1",
    ])).expect("explicit pen-down bridge").expect("joined TSV");
    let exact = concat!(
        "atrament.glyph-strokes.v1\n",
        "n\u{303}\t500\t600\t1100\t1400\t250\t#102030\n",
        "n\u{303}\t1100\t1400\t1500\t1800\t250\t#102030\n",
        "\u{f1}\t1000\t1000\t1200\t2200\t350\t#AABBCC\n",
        "n\u{303}\t1500\t1800\t2200\t2600\t250\t#102030\n",
    );
    assert_eq!(joined, exact.as_bytes());
    assert_eq!(fs::read(&source).expect("canonical unchanged"), original);

    let inspection = execute_glyph_library(&args(&[
        "--profile", source_name, "--join-strokes", "n\u{303}", "1",
        "--inspect-glyph", "n\u{303}",
    ])).expect("inspect pen sequence").expect("report");
    let report = String::from_utf8(inspection).expect("UTF-8 report");
    assert!(report.contains("Strokes: 3"), "{report}");
    assert!(report.contains("2: (1100, 1400) -> (1500, 1800)"),
        "{report}");
    assert!(!report.contains("Glyph: \"\u{f1}\""));

    assert!(execute_glyph_library(&args(&[
        "--profile", source_name, "--join-strokes", "n\u{303}", "1",
        "--output", target_name,
    ])).expect("save connected exact shape").is_none());
    assert_eq!(fs::read(&target).expect("saved file"), joined);
    for format in ["pdf", "png"] {
        let mut stored = args(&[
            "--export-paper", format, "--printed-sheet", "true",
            "--glyph-file", target_name,
            "--place-text", "1", "30000", "45000", "n\u{303}\u{f1}",
        ]);
        let mut inline = args(&[
            "--export-paper", format, "--printed-sheet", "true",
            "--glyph-stroke", "n\u{303}", "500", "600", "1100",
            "1400", "250", "#102030",
            "--glyph-stroke", "n\u{303}", "1100", "1400", "1500",
            "1800", "250", "#102030",
            "--glyph-stroke", "\u{f1}", "1000", "1000", "1200",
            "2200", "350", "#AABBCC",
            "--glyph-stroke", "n\u{303}", "1500", "1800", "2200",
            "2600", "250", "#102030",
            "--place-text", "1", "30000", "45000", "n\u{303}\u{f1}",
        ]);
        if format == "png" {
            stored.extend(args(&["--dpi", "72"]));
            inline.extend(args(&["--dpi", "72"]));
        }
        assert_eq!(execute_notebook(&stored)
            .expect("saved joined ink renders").expect("ink bytes"),
            execute_notebook(&inline)
                .expect("explicit segments render").expect("ink bytes"),
            "native {format} must paint exact connector and original ink");
    }
    fs::remove_file(source).expect("remove source");
    fs::remove_file(target).expect("remove target");
}

#[test]
fn explicit_joins_obey_physical_gap_boundary_and_pen_direction() {
    let base = args(&[
        "--glyph-stroke", "\u{f3}", "500", "500", "1000", "1000",
        "250", "#204060",
        "--glyph-stroke", "\u{f3}", "2300", "1200", "2200", "1000",
        "250", "#204060",
    ]);
    let mut selected = base.clone();
    selected.extend(args(&[
        "--reverse-stroke", "\u{f3}", "2", "--join-strokes", "\u{f3}", "1",
    ]));
    let joined = execute_glyph_library(&selected)
        .expect("caller-directed exact 1200 um bridge")
        .expect("canonical joined strokes");
    assert!(String::from_utf8(joined).expect("UTF-8").contains(
        "\u{f3}\t1000\t1000\t2200\t1000\t250\t#204060\n"));

    let mut wrong_direction = base;
    wrong_direction.extend(args(&["--join-strokes", "\u{f3}", "1"]));
    assert!(execute_glyph_library(&wrong_direction).is_err(),
        "never infer pen direction from the shorter endpoint");
    let outside = args(&[
        "--glyph-stroke", "\u{f3}", "500", "500", "1000", "1000",
        "250", "#204060",
        "--glyph-stroke", "\u{f3}", "2201", "1000", "2300", "1200",
        "250", "#204060", "--join-strokes", "\u{f3}", "1",
    ]);
    assert!(execute_glyph_library(&outside).is_err(),
        "1201 um gap is outside inclusive 1200 um bound");
    let clipped = args(&[
        "--glyph-stroke", "ó", "500", "600", "0", "1000",
        "250", "#204060",
        "--glyph-stroke", "ó", "500", "1000", "900", "1400",
        "250", "#204060", "--join-strokes", "ó", "1",
    ]);
    let error = execute_glyph_library(&clipped)
        .expect_err("an imported edge centerline cannot insert clipped ink");
    assert!(error.contains("pen bridge clips glyph cell"), "{error}");
    let tangent = args(&[
        "--glyph-stroke", "ó", "500", "600", "125", "1000",
        "250", "#204060",
        "--glyph-stroke", "ó", "425", "1000", "900", "1400",
        "250", "#204060", "--join-strokes", "ó", "1",
    ]);
    let admitted = execute_glyph_library(&tangent)
        .expect("round pen tangent to cell edge fits")
        .expect("three authored rows");
    assert!(String::from_utf8(admitted).expect("UTF-8").contains(
        "ó\t125\t1000\t425\t1000\t250\t#204060\n"));
}

#[test]
fn explicit_joins_refuse_unsafe_gaps_colors_and_indices_atomically() {
    let source = test_path().with_extension("tsv");
    let target = test_path().with_extension("tsv");
    let source_name = source.to_str().expect("join source");
    let target_name = target.to_str().expect("join target");
    let original = execute_glyph_library(&args(&[
        "--glyph-stroke", "\u{f1}", "500", "600", "1100", "1400",
        "250", "#102030",
        "--glyph-stroke", "\u{f1}", "1500", "1800", "2200", "2600",
        "250", "#102030",
        "--glyph-stroke", "a", "500", "600", "1100", "1400",
        "250", "#AABBCC",
    ])).expect("original glyphs").expect("original TSV");
    fs::write(&source, &original).expect("source TSV");
    fs::write(&target, b"preserve selected output")
        .expect("target sentinel");
    let invalid = [
        args(&["--join-strokes"]),
        args(&["--join-strokes", "\u{f1}"]),
        args(&["--join-strokes", "\u{f1}", "0"]),
        args(&["--join-strokes", "\u{f1}", "2"]),
        args(&["--join-strokes", "\u{f1}", "4097"]),
        args(&["--join-strokes", "n", "1"]),
        args(&["--join-strokes", "a", "1"]),
        args(&["--join-strokes", "\u{f1}", "1",
            "--join-strokes", "\u{f1}", "1"]),
        args(&["--set-stroke", "\u{f1}", "2", "1100", "1400",
            "2200", "2600", "250", "#102030",
            "--join-strokes", "\u{f1}", "1"]),
        args(&["--set-stroke", "\u{f1}", "2", "3000", "3000",
            "3200", "3500", "250", "#102030",
            "--join-strokes", "\u{f1}", "1"]),
        args(&["--set-stroke", "\u{f1}", "2", "1500", "1800",
            "2200", "2600", "350", "#102030",
            "--join-strokes", "\u{f1}", "1"]),
        args(&["--set-stroke", "\u{f1}", "2", "1500", "1800",
            "2200", "2600", "250", "#AABBCC",
            "--join-strokes", "\u{f1}", "1"]),
        args(&["--set-stroke", "\u{f1}", "1", "500", "600",
            "0", "1400", "250", "#102030",
            "--set-stroke", "\u{f1}", "2", "500", "1800",
            "2200", "2600", "250", "#102030",
            "--join-strokes", "\u{f1}", "1"]),
    ];
    for flags in invalid {
        let mut command = args(&["--profile", source_name]);
        command.extend(flags);
        command.extend(args(&["--output", target_name, "--replace"]));
        assert!(execute_glyph_library(&command).is_err(),
            "invalid join refuses: {command:?}");
        assert_eq!(fs::read(&source).expect("source unchanged"), original);
        assert_eq!(fs::read(&target).expect("output unchanged"),
            b"preserve selected output");
    }
    let complete = test_path().with_extension("tsv");
    let complete_name = complete.to_str().expect("full library name");
    let full = String::from("atrament.glyph-strokes.v1\n")
        + &"\u{f1}\t500\t600\t1100\t1400\t250\t#102030\n".repeat(4096);
    fs::write(&complete, full).expect("maximum authored shapes");
    let error = execute_glyph_library(&args(&[
        "--profile", complete_name, "--join-strokes", "\u{f1}", "1",
        "--output", target_name, "--replace",
    ])).expect_err("capacity cannot be exceeded");
    assert!(error.contains("4096"), "{error}");
    assert_eq!(fs::read(&target).expect("output unchanged"),
        b"preserve selected output");
    fs::remove_file(source).expect("remove source");
    fs::remove_file(target).expect("remove target");
    fs::remove_file(complete).expect("remove full library");
}

#[test]
fn native_midpoint_split_matches_browser_tsv_paths_and_rendered_ink() {
    let source = test_path().with_extension("tsv");
    let revised = test_path().with_extension("tsv");
    let source_name = source.to_str().expect("split source");
    let revised_name = revised.to_str().expect("split output");
    let nfd = "n\u{303}";
    let original = execute_glyph_library(&args(&[
        "--glyph-stroke", nfd, "1000", "900", "1601", "1702",
        "251", "#112233",
        "--glyph-stroke", "ñ", "400", "800", "500", "1200",
        "250", "#445566",
        "--glyph-stroke", nfd, "1601", "1702", "2000", "2200",
        "251", "#112233",
        "--glyph-stroke", "∞", "600", "600", "900", "900",
        "300", "#AABBCC",
    ])).expect("original interleaved glyphs").expect("source bytes");
    fs::write(&source, &original).expect("source saved");
    let mut edits = args(&[
        "--profile", source_name,
        "--split-stroke", nfd, "1",
        "--split-stroke", nfd, "3",
    ]);
    let split = execute_glyph_library(&edits)
        .expect("two native splits").expect("TSV bytes");
    let expected = concat!(
        "atrament.glyph-strokes.v1\n",
        "ñ\t1000\t900\t1301\t1301\t251\t#112233\n",
        "ñ\t1301\t1301\t1601\t1702\t251\t#112233\n",
        "ñ\t400\t800\t500\t1200\t250\t#445566\n",
        "ñ\t1601\t1702\t1801\t1951\t251\t#112233\n",
        "ñ\t1801\t1951\t2000\t2200\t251\t#112233\n",
        "∞\t600\t600\t900\t900\t300\t#AABBCC\n",
    );
    assert_eq!(split, expected.as_bytes(),
        "midpoints use browser half-up rounding without moving other rows");
    assert_eq!(fs::read(&source).expect("source remains"), original);
    edits.extend(args(&["--inspect-paths", nfd]));
    let report = String::from_utf8(execute_glyph_library(&edits)
        .expect("read native paths").expect("report"))
        .expect("inspector UTF-8");
    assert!(report.contains("Strokes: 4\nPaths: 1\nPen lifts: 0"),
        "{report}");
    assert!(report.contains(
        "Path 1: strokes 1..4 (4 segments) (1000, 900) -> (2000, 2200)"
    ), "{report}");
    assert!(execute_glyph_library(&args(&[
        "--profile", source_name, "--split-stroke", nfd, "1",
        "--split-stroke", nfd, "3", "--output", revised_name,
    ])).expect("explicit revised save").is_none());
    assert_eq!(fs::read(&revised).expect("saved revision"), split);
    assert_eq!(execute_glyph_library(&args(&[
        "--profile", revised_name,
    ])).expect("native reload").expect("TSV"), split);

    for format in ["pdf", "png"] {
        let mut stored = args(&[
            "--export-paper", format, "--printed-sheet", "true",
            "--glyph-file", revised_name,
            "--place-text", "1", "30000", "45000", "n\u{303}ñ∞",
        ]);
        let mut independent = args(&[
            "--export-paper", format, "--printed-sheet", "true",
            "--glyph-stroke", nfd, "1000", "900", "1301", "1301",
            "251", "#112233",
            "--glyph-stroke", nfd, "1301", "1301", "1601", "1702",
            "251", "#112233",
            "--glyph-stroke", "ñ", "400", "800", "500", "1200",
            "250", "#445566",
            "--glyph-stroke", nfd, "1601", "1702", "1801", "1951",
            "251", "#112233",
            "--glyph-stroke", nfd, "1801", "1951", "2000", "2200",
            "251", "#112233",
            "--glyph-stroke", "∞", "600", "600", "900", "900",
            "300", "#AABBCC",
            "--place-text", "1", "30000", "45000", "n\u{303}ñ∞",
        ]);
        if format == "png" {
            stored.extend(args(&["--dpi", "72"]));
            independent.extend(args(&["--dpi", "72"]));
        }
        assert_eq!(execute_notebook(&stored)
            .expect("saved native ink renders").expect("bytes"),
            execute_notebook(&independent)
                .expect("independent ink renders").expect("bytes"),
            "split directed ink differs from native {format} geometry");
    }
    fs::remove_file(source).expect("remove source");
    fs::remove_file(revised).expect("remove revision");
}

#[test]
fn native_midpoint_split_refuses_invalid_or_unsafe_edits_without_file_writes() {
    for flags in [
        args(&["--profile", "-", "--split-stroke"]),
        args(&["--profile", "-", "--split-stroke", "ñ"]),
    ] {
        let error = preflight_glyph_profile_sources(&flags)
            .expect_err("incomplete split refuses before stdin");
        assert!(error.contains("--split-stroke requires 2 operands"),
            "{error}");
    }
    for index in ["0", "-1", "1.0", "4097", "not-a-number"] {
        let error = preflight_glyph_profile_sources(&args(&[
            "--profile", "-", "--split-stroke", "ñ", index,
        ])).expect_err("invalid ordinal refuses before stdin");
        assert!(error.contains("stroke index"), "{index}: {error}");
    }
    let invalid = preflight_glyph_profile_sources(&args(&[
        "--profile", "-", "--split-stroke", "ab", "1",
    ])).expect_err("multiple graphemes refuse before stdin");
    assert!(invalid.contains("exact visible grapheme"), "{invalid}");

    let source = test_path().with_extension("tsv");
    let target = test_path().with_extension("tsv");
    let source_name = source.to_str().expect("split source");
    let target_name = target.to_str().expect("split target");
    let original = execute_glyph_library(&args(&[
        "--glyph-stroke", "ñ", "1000", "1200", "1001", "1200",
        "250", "#102030",
        "--glyph-stroke", "n\u{303}", "1100", "1200", "1100", "1201",
        "250", "#102030",
        "--glyph-stroke", "∞", "700", "600", "1800", "2000",
        "250", "#102030",
    ])).expect("short strokes admitted").expect("TSV bytes");
    fs::write(&source, &original).expect("saved source");
    fs::write(&target, b"do not replace accepted ink")
        .expect("output sentinel");
    for (glyph, index, diagnostic) in [
        ("ñ", "1", "too short"),
        ("n\u{303}", "1", "too short"),
        ("n", "1", "no authored strokes"),
        ("ñ", "2", "exceeds glyph"),
        ("∞", "2", "exceeds glyph"),
    ] {
        let error = execute_glyph_library(&args(&[
            "--profile", source_name, "--split-stroke", glyph, index,
            "--output", target_name, "--replace",
        ])).expect_err("bad split leaves output intact");
        assert!(error.contains(diagnostic), "{error}");
        assert_eq!(fs::read(&source).expect("source intact"), original);
        assert_eq!(fs::read(&target).expect("output intact"),
            b"do not replace accepted ink");
    }
    let full = test_path().with_extension("tsv");
    let full_name = full.to_str().expect("full library");
    let full_tsv = String::from("atrament.glyph-strokes.v1\n")
        + &"∞\t700\t600\t1800\t2000\t250\t#102030\n".repeat(4096);
    fs::write(&full, full_tsv.as_bytes()).expect("full source");
    let error = execute_glyph_library(&args(&[
        "--profile", full_name, "--split-stroke", "∞", "1",
        "--output", target_name, "--replace",
    ])).expect_err("bounded stroke library refuses extra line");
    assert!(error.contains("4096"), "{error}");
    assert_eq!(fs::read(&full).expect("full source unchanged"),
        full_tsv.as_bytes());
    assert_eq!(fs::read(&target).expect("target unchanged"),
        b"do not replace accepted ink");

    // A two-micrometre directed line has a distinct integer midpoint.
    let admitted = execute_glyph_library(&args(&[
        "--glyph-stroke", "ñ", "1000", "1200", "1002", "1200",
        "250", "#102030", "--split-stroke", "ñ", "1",
    ])).expect("two-um line can split").expect("updated ink");
    assert!(String::from_utf8(admitted).expect("UTF-8").contains(
        concat!(
            "ñ\t1000\t1200\t1001\t1200\t250\t#102030\n",
            "ñ\t1001\t1200\t1002\t1200\t250\t#102030\n",
        )
    ));
    fs::remove_file(source).expect("remove source");
    fs::remove_file(target).expect("remove target");
    fs::remove_file(full).expect("remove full source");
}


fn assert_merged_ink_export_matches_authored(
    revised_name: &str, nfd: &str,
) {
    for format in ["pdf", "png"] {
        let mut imported = args(&[
            "--export-paper", format, "--printed-sheet", "true",
            "--glyph-file", revised_name,
            "--place-text", "1", "30000", "45000", "n\u{303}\u{f1}\u{221e}",
        ]);
        let mut independently_authored = args(&[
            "--export-paper", format, "--printed-sheet", "true",
            "--glyph-stroke", nfd, "800", "900", "2000", "2100",
            "250", "#112233",
            "--glyph-stroke", "\u{f1}", "500", "800", "500", "1600",
            "250", "#445566",
            "--glyph-stroke", "\u{221e}", "600", "600", "900", "900",
            "300", "#AABBCC",
            "--glyph-stroke", nfd, "1600", "2400", "2100", "2400",
            "250", "#112233",
            "--place-text", "1", "30000", "45000", "n\u{303}\u{f1}\u{221e}",
        ]);
        if format == "png" {
            imported.extend(args(&["--dpi", "72"]));
            independently_authored.extend(args(&["--dpi", "72"]));
        }
        assert_eq!(execute_notebook(&imported)
            .expect("imported glyphs render").expect("render bytes"),
            execute_notebook(&independently_authored)
                .expect("authored glyphs render").expect("render bytes"),
            "merged TSV and direct {format} ink differ");
    }
}

#[test]
fn native_lossless_merge_preserves_exact_glyph_rows_and_export_geometry() {
    let source = test_path().with_extension("tsv");
    let revised = test_path().with_extension("tsv");
    let source_name = source.to_str().expect("original TSV path");
    let revised_name = revised.to_str().expect("merged TSV path");
    let nfd = "n\u{303}";
    let original = execute_glyph_library(&args(&[
        "--glyph-stroke", nfd, "800", "900", "1200", "1300",
        "250", "#112233",
        "--glyph-stroke", "\u{f1}", "500", "800", "500", "1600",
        "250", "#445566",
        "--glyph-stroke", nfd, "1200", "1300", "1600", "1700",
        "250", "#112233",
        "--glyph-stroke", "\u{221e}", "600", "600", "900", "900",
        "300", "#AABBCC",
        "--glyph-stroke", nfd, "1600", "1700", "2000", "2100",
        "250", "#112233",
        "--glyph-stroke", nfd, "1600", "2400", "2100", "2400",
        "250", "#112233",
    ])).expect("authored exact glyphs").expect("source bytes");
    fs::write(&source, &original).expect("save source");
    let expected = concat!(
        "atrament.glyph-strokes.v1\n",
        "n\u{303}\t800\t900\t2000\t2100\t250\t#112233\n",
        "\u{f1}\t500\t800\t500\t1600\t250\t#445566\n",
        "\u{221e}\t600\t600\t900\t900\t300\t#AABBCC\n",
        "n\u{303}\t1600\t2400\t2100\t2400\t250\t#112233\n",
    );
    let edit = args(&[
        "--profile", source_name,
        "--merge-strokes", nfd, "1",
        "--merge-strokes", nfd, "1",
    ]);
    let merged = execute_glyph_library(&edit)
        .expect("merge exact same-glyph neighbors").expect("merged TSV");
    assert_eq!(merged, expected.as_bytes(),
        "merge retains interleaved NFC and unrelated glyph slots");
    assert_eq!(fs::read(&source).expect("source intact"), original);
    let roundtrip = execute_glyph_library(&args(&[
        "--profile", source_name,
        "--split-stroke", nfd, "1",
        "--merge-strokes", nfd, "1",
    ])).expect("split/merge replay").expect("roundtrip bytes");
    assert_eq!(roundtrip, original,
        "exact line subdivision must be recoverable without rounding drift");

    let mut inspect = edit.clone();
    inspect.extend(args(&["--inspect-paths", nfd]));
    let report = String::from_utf8(execute_glyph_library(&inspect)
        .expect("inspect merged glyph").expect("path report"))
        .expect("UTF-8 report");
    assert!(report.contains("Strokes: 2\nPaths: 2\nPen lifts: 1"),
        "{report}");
    assert!(report.contains(
        "Path 1: strokes 1..1 (1 segment) (800, 900) -> (2000, 2100)"
    ), "{report}");
    let mut save = edit;
    save.extend(args(&["--output", revised_name]));
    assert!(execute_glyph_library(&save)
        .expect("save merged TSV").is_none());
    assert_eq!(fs::read(&revised).expect("saved merged ink"), merged);
    assert_eq!(execute_glyph_library(&args(&[
        "--profile", revised_name,
    ])).expect("read merged TSV").expect("bytes"), merged);

    assert_merged_ink_export_matches_authored(revised_name, nfd);
    fs::remove_file(source).expect("remove source");
    fs::remove_file(revised).expect("remove merged file");
}

#[test]
fn native_lossless_merge_refuses_nonidentical_ink_without_clobbering() {
    for flags in [
        args(&["--profile", "-", "--merge-strokes"]),
        args(&["--profile", "-", "--merge-strokes", "\u{f1}"]),
    ] {
        let failure = preflight_glyph_profile_sources(&flags)
            .expect_err("incomplete operands refuse before stdin");
        assert!(failure.contains("--merge-strokes requires 2 operands"),
            "{failure}");
    }
    for index in ["0", "-1", "1.0", "4097", "not-a-number"] {
        let failure = preflight_glyph_profile_sources(&args(&[
            "--profile", "-", "--merge-strokes", "\u{f1}", index,
        ])).expect_err("invalid ordinal refuses before stdin");
        assert!(failure.contains("stroke index"), "{index}: {failure}");
    }
    let failure = preflight_glyph_profile_sources(&args(&[
        "--profile", "-", "--merge-strokes", "n\u{303}a", "1",
    ])).expect_err("multiple graphemes refuse");
    assert!(failure.contains("exact visible grapheme"), "{failure}");

    let source = test_path().with_extension("tsv");
    let target = test_path().with_extension("tsv");
    let source_name = source.to_str().expect("source TSV path");
    let target_name = target.to_str().expect("target TSV path");
    fs::write(&target, b"do not replace accepted output")
        .expect("sentinel target");
    let base = "n\u{303}\t1000\t1000\t1300\t1300\t250\t#102030\n";
    for (scenario, next, diagnostic) in [
        ("pen lift", "n\u{303}\t1301\t1300\t1600\t1600\t250\t#102030\n",
            "touching"),
        ("turn", "n\u{303}\t1300\t1300\t1400\t1600\t250\t#102030\n",
            "straight"),
        ("reverse", "n\u{303}\t1300\t1300\t1200\t1200\t250\t#102030\n",
            "co-directed"),
        ("width", "n\u{303}\t1300\t1300\t1600\t1600\t251\t#102030\n",
            "matching pen"),
        ("color", "n\u{303}\t1300\t1300\t1600\t1600\t250\t#102031\n",
            "matching pen"),
        ("different glyph", "\u{f1}\t1300\t1300\t1600\t1600\t250\t#102030\n",
            "following stroke"),
        ("rounded bend", "n\u{303}\t1300\t1300\t1601\t1602\t250\t#102030\n",
            "straight"),
    ] {
        let content = format!("atrament.glyph-strokes.v1\n{base}{next}");
        fs::write(&source, content.as_bytes()).expect("valid case TSV");
        let failure = execute_glyph_library(&args(&[
            "--profile", source_name, "--merge-strokes", "n\u{303}", "1",
            "--output", target_name, "--replace",
        ])).expect_err("nonlossless merge refuses");
        assert!(failure.contains(diagnostic),
            "{scenario} refusal: {failure}");
        assert_eq!(fs::read(&source).expect("source untouched"),
            content.as_bytes());
        assert_eq!(fs::read(&target).expect("target untouched"),
            b"do not replace accepted output", "{scenario}");
    }
    let rounded = concat!(
        "atrament.glyph-strokes.v1\n",
        "n\u{303}\t1000\t900\t1301\t1301\t251\t#112233\n",
        "n\u{303}\t1301\t1301\t1601\t1702\t251\t#112233\n",
    );
    fs::write(&source, rounded).expect("rounded midpoint source");
    let failure = execute_glyph_library(&args(&[
        "--profile", source_name, "--merge-strokes", "n\u{303}", "1",
        "--output", target_name, "--replace",
    ])).expect_err("rounded midpoint can be noncollinear");
    assert!(failure.contains("straight"), "{failure}");

    // Combining two strokes must also work at the full 4096-row cap.
    let full = test_path().with_extension("tsv");
    let full_name = full.to_str().expect("full TSV path");
    let mut source_text = String::from("atrament.glyph-strokes.v1\n");
    source_text.push_str("n\u{303}\t800\t900\t1200\t1300\t250\t#112233\n");
    source_text.push_str("n\u{303}\t1200\t1300\t1600\t1700\t250\t#112233\n");
    source_text.push_str(
        &"\u{221e}\t600\t600\t900\t900\t250\t#112233\n".repeat(4094)
    );
    fs::write(&full, &source_text).expect("bounded full TSV");
    let reduced = execute_glyph_library(&args(&[
        "--profile", full_name, "--merge-strokes", "n\u{303}", "1",
    ])).expect("full library can shrink").expect("shrunk TSV");
    assert!(String::from_utf8(reduced).expect("UTF-8").contains(
        "n\u{303}\t800\t900\t1600\t1700\t250\t#112233\n"));
    fs::remove_file(source).expect("remove source");
    fs::remove_file(target).expect("remove target");
    fs::remove_file(full).expect("remove full fixture");
}

#[test]
fn native_reflection_preserves_directed_unicode_paths_and_pdf_png_ink() {
    let source = test_path().with_extension("tsv");
    let mirrored = test_path().with_extension("tsv");
    let source_name = source.to_str().expect("reflection source");
    let mirrored_name = mirrored.to_str().expect("reflection output");
    let nfd = "o\u{301}";
    let original = execute_glyph_library(&args(&[
        "--glyph-stroke", nfd, "450", "1000", "850", "1600",
        "251", "#243647",
        "--glyph-stroke", "ó", "200", "1000", "550", "1500",
        "250", "#102030",
        "--glyph-stroke", nfd, "850", "1600", "1300", "2200",
        "251", "#243647",
        "--glyph-stroke", "∞", "300", "1700", "500", "1900",
        "250", "#AABBCC",
    ])).expect("original library").expect("source TSV");
    fs::write(&source, &original).expect("save reflection source");
    let mirror_x = execute_glyph_library(&args(&[
        "--profile", source_name,
        "--reflect-glyph", nfd, "x", "1750",
    ])).expect("native horizontal reflection").expect("reflected bytes");
    let expected_x = concat!(
        "atrament.glyph-strokes.v1\n",
        "ó\t3050\t1000\t2650\t1600\t251\t#243647\n",
        "ó\t200\t1000\t550\t1500\t250\t#102030\n",
        "ó\t2650\t1600\t2200\t2200\t251\t#243647\n",
        "∞\t300\t1700\t500\t1900\t250\t#AABBCC\n",
    );
    assert_eq!(mirror_x, expected_x.as_bytes(),
        "mirrored directed segments keep interleaved NFC/math slots");
    let double_x = execute_glyph_library(&args(&[
        "--profile", source_name,
        "--reflect-glyph", nfd, "x", "1750",
        "--reflect-glyph", nfd, "x", "1750",
    ])).expect("double reflection").expect("original bytes");
    assert_eq!(double_x, original,
        "reflection is an exact involution, not a rounded approximation");
    let mirror_y = execute_glyph_library(&args(&[
        "--profile", source_name,
        "--reflect-glyph", nfd, "y", "2250",
    ])).expect("native vertical reflection").expect("vertical TSV");
    let expected_y = concat!(
        "atrament.glyph-strokes.v1\n",
        "ó\t450\t3500\t850\t2900\t251\t#243647\n",
        "ó\t200\t1000\t550\t1500\t250\t#102030\n",
        "ó\t850\t2900\t1300\t2300\t251\t#243647\n",
        "∞\t300\t1700\t500\t1900\t250\t#AABBCC\n",
    );
    assert_eq!(mirror_y, expected_y.as_bytes());
    assert_eq!(execute_glyph_library(&args(&[
        "--profile", source_name,
        "--reflect-glyph", nfd, "y", "2250",
        "--reflect-glyph", nfd, "y", "2250",
    ])).expect("double vertical reflection").expect("bytes"), original);
    let report = String::from_utf8(execute_glyph_library(&args(&[
        "--profile", source_name,
        "--reflect-glyph", nfd, "x", "1750",
        "--inspect-paths", nfd,
    ])).expect("inspect mirrored path").expect("report"))
        .expect("UTF-8 inspector");
    assert!(report.contains("Strokes: 2\nPaths: 1\nPen lifts: 0"),
        "{report}");
    assert!(report.contains(
        "Path 1: strokes 1..2 (2 segments) (3050, 1000) -> (2200, 2200)"
    ), "{report}");
    assert!(execute_glyph_library(&args(&[
        "--profile", source_name,
        "--reflect-glyph", nfd, "x", "1750",
        "--output", mirrored_name,
    ])).expect("save reflected TSV").is_none());
    assert_eq!(fs::read(&source).expect("original unchanged"), original);
    assert_eq!(fs::read(&mirrored).expect("mirrored saved"), mirror_x);
    assert_eq!(execute_glyph_library(&args(&[
        "--profile", mirrored_name,
    ])).expect("reopen").expect("TSV"), mirror_x);
    for format in ["pdf", "png"] {
        let mut saved = args(&[
            "--export-paper", format, "--printed-sheet", "true",
            "--glyph-file", mirrored_name,
            "--place-text", "1", "30000", "45000", "o\u{301}ó∞",
        ]);
        let mut manual = args(&[
            "--export-paper", format, "--printed-sheet", "true",
            "--glyph-stroke", nfd, "3050", "1000", "2650", "1600",
            "251", "#243647",
            "--glyph-stroke", "ó", "200", "1000", "550", "1500",
            "250", "#102030",
            "--glyph-stroke", nfd, "2650", "1600", "2200", "2200",
            "251", "#243647",
            "--glyph-stroke", "∞", "300", "1700", "500", "1900",
            "250", "#AABBCC",
            "--place-text", "1", "30000", "45000", "o\u{301}ó∞",
        ]);
        if format == "png" {
            saved.extend(args(&["--dpi", "72"]));
            manual.extend(args(&["--dpi", "72"]));
        }
        assert_eq!(execute_notebook(&saved)
            .expect("saved reflected ink").expect("bytes"),
            execute_notebook(&manual)
                .expect("independent authored ink").expect("bytes"),
            "native {format} must reflect directed source ink exactly");
    }
    fs::remove_file(source).expect("remove source");
    fs::remove_file(mirrored).expect("remove reflected output");
}

#[test]
fn native_reflection_refuses_bad_parameters_and_clipping_atomically() {
    for flags in [
        args(&["--profile", "-", "--reflect-glyph"]),
        args(&["--profile", "-", "--reflect-glyph", "ó"]),
        args(&["--profile", "-", "--reflect-glyph", "ó", "x"]),
    ] {
        let error = preflight_glyph_profile_sources(&flags)
            .expect_err("incomplete reflection must refuse before stdin");
        assert!(error.contains("--reflect-glyph requires 3 operands"),
            "{error}");
    }
    for (glyph, axis, pivot, diagnostic) in [
        ("ó", "X", "1750", "axis"),
        ("ó", "z", "1750", "axis"),
        ("ó", "", "1750", "axis"),
        ("ab", "x", "1750", "exact visible"),
        ("ó", "x", "-1", "reflection pivot"),
        ("ó", "x", "1.5", "reflection pivot"),
        ("ó", "x", "3501", "reflection pivot"),
        ("ó", "y", "4501", "reflection pivot"),
        ("ó", "y", "not-a-number", "reflection pivot"),
    ] {
        let error = preflight_glyph_profile_sources(&args(&[
            "--profile", "-", "--reflect-glyph", glyph, axis, pivot,
        ])).expect_err("bad typed operand must refuse before file read");
        assert!(error.contains(diagnostic), "{error}");
    }
    let source = test_path().with_extension("tsv");
    let target = test_path().with_extension("tsv");
    let original = execute_glyph_library(&args(&[
        "--glyph-stroke", "ó", "450", "1000", "850", "1600",
        "251", "#243647",
        "--glyph-stroke", "∞", "600", "1200", "800", "1800",
        "250", "#AABBCC",
    ])).expect("source glyph library").expect("source TSV");
    fs::write(&source, &original).expect("saved source");
    fs::write(&target, b"retain accepted reflection")
        .expect("target sentinel");
    let source_name = source.to_str().expect("source path");
    let target_name = target.to_str().expect("target path");
    for (glyph, axis, pivot, diagnostic) in [
        ("ó", "x", "0", "clips"),
        ("ó", "y", "0", "clips"),
        ("a", "x", "1750", "no authored strokes"),
    ] {
        let error = execute_glyph_library(&args(&[
            "--profile", source_name,
            "--reflect-glyph", glyph, axis, pivot,
            "--output", target_name, "--replace",
        ])).expect_err("unsafe reflection cannot replace accepted file");
        assert!(error.contains(diagnostic), "{error}");
        assert_eq!(fs::read(&source).expect("unchanged source"), original);
        assert_eq!(fs::read(&target).expect("unchanged target"),
            b"retain accepted reflection");
    }
    // A y reflection must reject a preexisting x-cap clip too. The
    // browser checks both axes after reflecting, not only the edited one.
    let cross_axis = execute_glyph_library(&args(&[
        "--glyph-stroke", "ó", "136", "708", "1446", "2720",
        "508", "#243647",
        "--reflect-glyph", "ó", "y", "2372",
    ])).expect_err("unreflected X pen cap must still be checked");
    assert!(cross_axis.contains("clips"), "{cross_axis}");
    // The physical pen radius is half a micrometre for odd pen widths:
    // 2*x >= width and 2*x <= 7000-width must both hold.
    let tangent = execute_glyph_library(&args(&[
        "--glyph-stroke", "ó", "126", "1000", "850", "1600",
        "251", "#243647", "--reflect-glyph", "ó", "x", "1750",
    ])).expect("odd-width reflected edge is admitted").expect("TSV");
    assert!(String::from_utf8(tangent).expect("UTF-8")
        .contains("ó\t3374\t1000\t2650\t1600\t251\t#243647"));
    let error = execute_glyph_library(&args(&[
        "--glyph-stroke", "ó", "125", "1000", "850", "1600",
        "251", "#243647", "--reflect-glyph", "ó", "x", "1750",
    ])).expect_err("odd-width reflected contour must clip");
    assert!(error.contains("clips"), "{error}");
    fs::remove_file(source).expect("remove source");
    fs::remove_file(target).expect("remove target");
}

#[test]
fn targeted_stroke_edits_refuse_bad_shapes_and_leave_files_untouched() {
    let source = test_path().with_extension("tsv");
    let target = test_path().with_extension("tsv");
    let source_name = source.to_str().expect("test source");
    let output_name = target.to_str().expect("test output");
    let original = execute_glyph_library(&args(&[
        "--glyph-stroke", "ñ", "500", "700", "900", "1300",
        "250", "#102030",
        "--glyph-stroke", "n\u{303}", "500", "700", "900",
        "1300", "250", "#708090",
    ])).expect("valid original").expect("TSV");
    fs::write(&source, &original).expect("test input");
    fs::write(&target, b"preserve already accepted output")
        .expect("test sentinel");
    let invalid = [
        args(&["--set-stroke"]),
        args(&["--set-stroke", "ñ"]),
        args(&["--set-stroke", "ñ", "0"]),
        args(&["--set-stroke", "n\u{303}", "2"]),
        args(&["--set-stroke", "n", "1", "500", "700", "900",
            "1300", "250", "#102030"]),
        args(&["--set-stroke", "ñ", "1", "500", "700", "500",
            "700", "250", "#102030"]),
        args(&["--set-stroke", "ñ", "1", "500", "700", "900",
            "1300", "79", "#102030"]),
        args(&["--set-stroke", "ñ", "1", "500", "700", "900",
            "1300", "1501", "#102030"]),
        args(&["--set-stroke", "ñ", "1", "3501", "700", "900",
            "1300", "250", "#102030"]),
        args(&["--set-stroke", "ñ", "1", "500", "4501", "900",
            "1300", "250", "#102030"]),
        args(&["--set-stroke", "ñ", "1", "500", "700", "900",
            "1300", "250", "#GGGGGG"]),
        args(&["--set-stroke", "ñ", "1", "500", "700", "900",
            "1300", "250"]),
        args(&["--remove-stroke"]),
        args(&["--remove-stroke", "ñ"]),
        args(&["--remove-stroke", "ñ", "0"]),
        args(&["--remove-stroke", "ñ", "2"]),
        args(&["--remove-stroke", "n", "1"]),
        args(&["--remove-stroke", "ñ", "4097"]),
        args(&["--remove-stroke", "ñ", "1",
            "--remove-stroke", "ñ", "1"]),
        args(&["--remove-stroke", "ñ", "1",
            "--set-stroke", "ñ", "1", "500", "700", "900",
            "1300", "250", "#102030"]),
    ];
    for flags in invalid {
        let mut command = args(&["--profile", source_name]);
        command.extend(flags);
        command.extend(args(&["--output", output_name, "--replace"]));
        assert!(execute_glyph_library(&command).is_err(),
            "invalid shape edits must refuse: {command:?}");
        assert_eq!(fs::read(&source).expect("source untouched"), original);
        assert_eq!(fs::read(&target).expect("target untouched"),
            b"preserve already accepted output");
    }
    // Removing a library's only stroke also refuses without saving anything.
    let last = test_path().with_extension("tsv");
    let last_name = last.to_str().expect("owned empty-library target");
    let error = execute_glyph_library(&args(&[
        "--glyph-stroke", "ñ", "500", "700", "900", "1300",
        "250", "#102030", "--remove-stroke", "ñ", "1",
        "--output", last_name,
    ])).expect_err("library cannot be saved without any strokes");
    assert!(error.contains("at least one authored stroke"), "{error}");
    assert!(!last.exists(), "no target committed after last stroke removal");
    fs::remove_file(source).expect("remove test source");
    fs::remove_file(target).expect("remove test target");
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
    let markdown = format!("# a\n\na a a\n- a\n{}", "a  \n".repeat(34));
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
fn markdown_soft_wrap_preserves_unicode_and_structural_block_boundaries() {
    let text = "# a\na \nn\u{303}\n- a\na\n\nn\u{303}";
    let source = FlowSource::new(FlowFormat::Markdown, text)
        .expect("admitted Markdown");
    let (runs, ornaments, pages) = layout_flow(
        &source, &NotebookProfile::default(), None,
        HeadingDecoration::default(),
    ).expect("measured Markdown paragraphs");
    assert_eq!(pages, 1);
    assert_eq!(runs.iter().map(|run| run.text.as_str())
        .collect::<Vec<_>>(), ["a", "a n\u{303}", "a", "a", "n\u{303}"]);
    assert_eq!(ornaments.len(), 2, "heading underline and list bullet");
    assert_eq!(runs[1].origin_um.1 - runs[0].origin_um.1, 9_000);
    assert_eq!(runs[2].origin_um.1 - runs[1].origin_um.1, 9_000);
    assert_eq!(runs[4].origin_um.1 - runs[3].origin_um.1, 13_000,
        "blank Markdown line adds a paragraph gap");

    let mut glyphs = GlyphOverlay::default();
    for grapheme in ["a", "n\u{303}"] {
        glyphs.parse("--glyph-stroke", &mut args(&[
            grapheme, "0", "0", "3500", "4500", "250", "#204060",
        ]).iter()).expect("authored exact NFC/NFD shape");
    }
    glyphs.runs = runs;
    assert_eq!(glyphs.expand(&[], None)
        .expect("all exact graphemes render").len(), 6);

    let plain = FlowSource::new(FlowFormat::Plain, "a\nn\u{303}")
        .expect("plain text source");
    let (plain_runs, _, _) = layout_flow(
        &plain, &NotebookProfile::default(), None,
        HeadingDecoration::default(),
    ).expect("plain newlines remain physical rows");
    assert_eq!(plain_runs.iter().map(|run| run.text.as_str())
        .collect::<Vec<_>>(), ["a", "n\u{303}"]);
}

#[test]
fn markdown_quote_paragraphs_keep_authored_text_and_physical_bars() {
    let source = FlowSource::new(
        FlowFormat::Markdown, "> a\n>a\n> a  \n> a\na\n> a",
    ).expect("single-level Markdown quotes");
    let (runs, bars, pages) = layout_flow(
        &source, &NotebookProfile::default(), None,
        HeadingDecoration::default(),
    ).expect("quoted and ordinary paragraphs measured separately");
    assert_eq!(pages, 1);
    assert_eq!(runs.iter().map(|run| run.text.as_str())
        .collect::<Vec<_>>(), ["a a a", "a", "a", "a"]);
    assert_eq!(runs.iter().map(|run| run.origin_um.0)
        .collect::<Vec<_>>(), [28_500, 28_500, 22_500, 28_500]);
    assert_eq!(bars.len(), 3, "one quote bar per visible quote row");
    for (bar, row) in bars.iter().zip([&runs[0], &runs[1], &runs[3]]) {
        assert_eq!(bar.page, row.page);
        assert_eq!(bar.start_um, (25_500, row.origin_um.1 + 500));
        assert_eq!(bar.end_um, (25_500, row.origin_um.1 + 8_400));
        assert_eq!(bar.width_um, 400);
        assert_eq!(bar.rgb, [30, 48, 74]);
    }

    let long_quote = format!("> {}", "a ".repeat(26));
    let source = FlowSource::new(FlowFormat::Markdown, &long_quote)
        .expect("bounded long quote");
    let (wrapped, marks, pages) = layout_flow(
        &source, &NotebookProfile::default(), None,
        HeadingDecoration::default(),
    ).expect("grapheme-aware quote continuation");
    assert_eq!(pages, 1);
    assert!(wrapped.len() >= 2, "quote wraps within physical margins");
    assert_eq!(wrapped.len(), marks.len(),
        "every continuation gets its own quote rule");
    assert!(wrapped.iter().all(|row| row.origin_um.0 == 28_500));
    assert!(marks.iter().all(|rule| rule.start_um.0 == 25_500));
}

#[test]
fn markdown_quote_soft_breaks_replay_real_pdf_and_png_ink() {
    let base = args(&[
        "--printed-sheet", "true", "--glyph-stroke", "a", "0", "0",
        "3500", "4500", "250", "#204060",
    ]);
    for format in ["pdf", "png"] {
        let render = |markdown: &str| {
            let mut command = base.clone();
            command.extend(args(&[
                "--export-paper", format, "--flow-markdown", markdown,
            ]));
            if format == "png" {
                command.extend(args(&["--dpi", "72"]));
            }
            execute_notebook(&command)
                .expect("native quote export").expect("image bytes")
        };
        let soft = render("> a\n>a");
        let joined = render("> a a");
        let hard = render("> a  \n> a");
        let plain = render("a a");
        assert_eq!(soft, joined, "quoted soft line replays identically");
        assert_ne!(soft, hard, "hard quote break makes a second row");
        assert_ne!(soft, plain, "quote rule is real page ink");
        if format == "pdf" {
            assert!(soft.starts_with(b"%PDF-1.4"));
        } else {
            assert!(soft.starts_with(b"\x89PNG\r\n\x1a\n"));
        }
    }
    let unquoted = execute_notebook(&args(&[
        "--export-paper", "pdf", "--glyph-stroke", "a", "0", "0",
        "3500", "4500", "250", "#204060", "--flow-text", "> a",
    ])).expect_err("plain text > is not Markdown syntax");
    assert!(unquoted.contains("U+003E"), "{unquoted}");
}

#[test]
fn markdown_quote_bars_follow_real_page_breaks_and_refuse_empty_blocks() {
    let markdown = "> a  \n".repeat(36);
    let source = FlowSource::new(FlowFormat::Markdown, &markdown)
        .expect("bounded quote rows");
    let (runs, bars, pages) = layout_flow(
        &source, &NotebookProfile::default(), None,
        HeadingDecoration::default(),
    ).expect("two-page quoted notebook");
    assert_eq!(pages, 2);
    assert_eq!(runs.len(), 36);
    assert_eq!(bars.len(), 36);
    assert!(bars.iter().any(|bar| bar.page == 2));
    assert!(runs.iter().zip(&bars).all(|(run, bar)| run.page == bar.page));

    let mut command = args(&[
        "--printed-sheet", "true", "--export-paper", "pdf",
        "--glyph-stroke", "a", "0", "0", "3500", "4500",
        "250", "#204060", "--flow-markdown", &markdown,
    ]);
    let pdf = execute_notebook(&command)
        .expect("real two-page quote PDF").expect("PDF bytes");
    let text = String::from_utf8_lossy(&pdf);
    let streams = text.split("\nstream\n").skip(1).map(|section| {
        section.split("\nendstream").next().unwrap_or("")
    }).collect::<Vec<_>>();
    assert_eq!(streams.len(), 2);
    assert!(streams.iter().all(|stream| stream.contains(" m\n")));
    command[3] = "png".into();
    command.extend(args(&["--dpi", "72", "--png-page", "2"]));
    let png = execute_notebook(&command)
        .expect("page two transparent quote PNG").expect("PNG bytes");
    assert!(png.starts_with(b"\x89PNG\r\n\x1a\n"));

    let target = test_path().with_extension("pdf");
    fs::write(&target, b"existing PDF").expect("test-owned sentinel");
    let name = target.to_str().expect("selected PDF path");
    for invalid in [">", "> ", ">    ", "a\n> "] {
        let error = execute_notebook(&args(&[
            "--export-paper", "pdf", "--glyph-stroke", "a", "0", "0",
            "3500", "4500", "250", "#204060", "--flow-markdown",
            invalid, "--output", name, "--replace",
        ])).expect_err("empty quoted block refused without overwrite");
        assert!(error.contains("empty Markdown block refused"), "{error}");
        assert_eq!(fs::read(&target).expect("sentinel intact"),
            b"existing PDF");
    }
    fs::remove_file(target).expect("remove test-owned sentinel");
}

#[test]
fn markdown_syntax_markers_do_not_change_statistical_row_pitch() {
    let source = FlowSource::new(
        FlowFormat::Markdown, "# a\n- a\n* a\n+ a\n> a\na",
    ).expect("mixed visible Markdown blocks");
    let stats = execute_handwriting(&args(&[
        "--set-glyph", "#", "height", "20000", "20000", "20000",
        "20000", "0",
        "--set-glyph", "-", "height", "21000", "21000", "21000",
        "21000", "0",
        "--set-glyph", "*", "height", "22000", "22000", "22000",
        "22000", "0",
        "--set-glyph", "+", "height", "23000", "23000", "23000",
        "23000", "0",
        "--set-glyph", ">", "height", "24000", "24000", "24000",
        "24000", "0",
        "--set-glyph", "1", "height", "25000", "25000", "25000",
        "25000", "0",
    ])).expect("valid but tall syntax glyph controls").expect("profile bytes");
    let model = decode_statistics_json(&stats, &UnicodeGraphemeSegmentation)
        .expect("reopen valid metrics");
    let (runs, ornaments, pages) = layout_flow(
        &source, &NotebookProfile::default(), Some(&model),
        HeadingDecoration::default(),
    ).expect("only visible authored text sets row pitch");
    assert_eq!(pages, 1);
    assert_eq!(runs.iter().map(|run| run.text.as_str())
        .collect::<Vec<_>>(), ["a", "a", "a", "a", "a", "a"]);
    assert_eq!(runs.iter().map(|run| run.origin_um.1)
        .collect::<Vec<_>>(), [24_500, 33_500, 42_500,
            51_500, 60_500, 69_500]);
    assert_eq!(ornaments.len(), 5);

    let literal = FlowSource::new(FlowFormat::Plain, ">\na")
        .expect("literal > in plain text");
    let (plain, _, _) = layout_flow(
        &literal, &NotebookProfile::default(), Some(&model),
        HeadingDecoration::default(),
    ).expect("plain glyph metrics remain active");
    assert_eq!(plain[1].origin_um.1 - plain[0].origin_um.1, 28_000,
        "the same > remains an authored glyph in plain text");

    let numbered = FlowSource::new(FlowFormat::Markdown, "1. a\n2. a")
        .expect("numbered labels");
    let (numbers, _, _) = layout_flow(
        &numbered, &NotebookProfile::default(), Some(&model),
        HeadingDecoration::default(),
    ).expect("authored numeral label stays visible");
    assert_eq!(numbers[1].origin_um.1 - numbers[0].origin_um.1, 29_000,
        "an actually drawn number still changes measured row pitch");
}

#[test]
fn markdown_syntax_height_overrides_leave_pdf_and_png_byte_identical() {
    let base_bytes = execute_handwriting(&args(&["--seed", "19"]))
        .expect("baseline statistics").expect("statistics JSON");
    let syntax_bytes = execute_handwriting(&args(&[
        "--seed", "19", "--set-glyph", "#", "height",
        "20000", "20000", "20000", "20000", "0",
        "--set-glyph", "-", "height", "21000", "21000",
        "21000", "21000", "0",
        "--set-glyph", ">", "height", "22000", "22000",
        "22000", "22000", "0",
    ])).expect("tall syntax overrides").expect("statistics JSON");
    let baseline = test_path();
    let syntax = test_path();
    fs::write(&baseline, &base_bytes).expect("test baseline file");
    fs::write(&syntax, &syntax_bytes).expect("test syntax file");
    let baseline_path = baseline.to_str().expect("baseline filename");
    let syntax_path = syntax.to_str().expect("syntax filename");
    for format in ["pdf", "png"] {
        let render = |profile: &str| {
            let mut command = args(&[
                "--export-paper", format, "--printed-sheet", "true",
                "--glyph-stroke", "a", "0", "0", "3500", "4500",
                "250", "#204060", "--flow-markdown", "# a\n- a\n> a",
                "--handwriting-profile", profile,
            ]);
            if format == "png" {
                command.extend(args(&["--dpi", "72"]));
            }
            execute_notebook(&command)
                .expect("native Markdown export").expect("image bytes")
        };
        let standard = render(baseline_path);
        assert_eq!(standard, render(syntax_path),
            "unrendered marker statistics must not move ink");
    }
    fs::remove_file(baseline).expect("remove test baseline");
    fs::remove_file(syntax).expect("remove test syntax profile");
}

#[test]
fn markdown_backslash_breaks_match_two_spaces_across_native_pages() {
    for prefix in ["", "> "] {
        let slash = format!("{prefix}a\\\n").repeat(36);
        let spaces = format!("{prefix}a  \n").repeat(36);
        let soft = format!("{prefix}a\n").repeat(36);
        let source = FlowSource::new(FlowFormat::Markdown, &slash)
            .expect("backslash hard breaks admitted");
        let (runs, bars, pages) = layout_flow(
            &source, &NotebookProfile::default(), None,
            HeadingDecoration::default(),
        ).expect("every hard break reserves a physical row");
        assert_eq!((runs.len(), pages), (36, 2));
        assert!(runs.iter().all(|run| run.text == "a"));
        assert_eq!(bars.len(), if prefix.is_empty() { 0 } else { 36 });
        for format in ["pdf", "png"] {
            let render = |markdown: &str, page: &str| {
                let mut command = args(&[
                    "--export-paper", format, "--printed-sheet", "true",
                    "--glyph-stroke", "a", "0", "0", "3500", "4500",
                    "250", "#204060", "--flow-markdown", markdown,
                ]);
                if format == "png" {
                    command.extend(args(&[
                        "--dpi", "72", "--png-page", page,
                    ]));
                }
                execute_notebook(&command)
                    .expect("real native handwritten output")
                    .expect("PDF or PNG bytes")
            };
            let actual = render(&slash, "2");
            assert_eq!(actual, render(&spaces, "2"),
                "single backslash and two-space breaks have identical ink");
            if format == "pdf" {
                assert!(actual.starts_with(b"%PDF-1.4"));
                assert!(String::from_utf8_lossy(&actual)
                    .contains("/Count 2"));
            } else {
                assert!(actual.starts_with(b"\x89PNG\r\n\x1a\n"));
                assert_ne!(actual, render(&slash, "1"),
                    "page 2 carries different physical ink");
            }
            assert_ne!(actual, render(&soft, "1"),
                "soft lines have different physical row placements");
        }
    }
}

#[test]
fn markdown_backslash_literal_and_empty_refusals_are_atomic() {
    let source = FlowSource::new(
        FlowFormat::Markdown, "a\\\\\na",
    ).expect("doubled literal backslash is text");
    let (runs, _, pages) = layout_flow(
        &source, &NotebookProfile::default(), None,
        HeadingDecoration::default(),
    ).expect("paired slashes are not hard breaks");
    assert_eq!(pages, 1);
    assert_eq!(runs.len(), 1);
    assert_eq!(runs[0].text, "a\\\\ a");
    let target = test_path().with_extension("pdf");
    fs::write(&target, b"existing valid PDF").expect("test sentinel");
    let path = target.to_str().expect("test path");
    for input in ["\\", "> \\", "a\\\\\na", "a\\\n\\"] {
        let error = execute_notebook(&args(&[
            "--export-paper", "pdf", "--glyph-stroke", "a",
            "0", "0", "3500", "4500", "250", "#204060",
            "--flow-markdown", input, "--output", path, "--replace",
        ])).expect_err("empty or unowned backslashes must refuse");
        assert!(error.contains("empty Markdown block refused")
            || error.contains("U+005C"), "{error}");
        assert_eq!(fs::read(&target).expect("sentinel intact"),
            b"existing valid PDF");
    }
    let plain = execute_notebook(&args(&[
        "--export-paper", "pdf", "--glyph-stroke", "a",
        "0", "0", "3500", "4500", "250", "#204060",
        "--flow-text", "a\\\na",
    ])).expect_err("plain backslash retains literal identity");
    assert!(plain.contains("U+005C"), "{plain}");
    fs::remove_file(target).expect("remove test-owned PDF");
}

#[test]
fn markdown_backslash_control_does_not_inflate_statistical_pitch() {
    let statistics = execute_handwriting(&args(&[
        "--set-glyph", "\\", "height", "25000", "25000",
        "25000", "25000", "0",
    ])).expect("tall slash metrics admitted").expect("profile bytes");
    let model = decode_statistics_json(
        &statistics, &UnicodeGraphemeSegmentation,
    )
        .expect("read tall slash metrics");
    let slash = FlowSource::new(FlowFormat::Markdown, "a\\\na")
        .expect("Markdown hard break");
    let spaces = FlowSource::new(FlowFormat::Markdown, "a  \na")
        .expect("two-space break");
    let (slash_runs, _, _) = layout_flow(&slash,
        &NotebookProfile::default(), Some(&model),
        HeadingDecoration::default()).expect("slash is syntax only");
    let (space_runs, _, _) = layout_flow(&spaces,
        &NotebookProfile::default(), Some(&model),
        HeadingDecoration::default()).expect("same visible text");
    assert_eq!(slash_runs.len(), space_runs.len());
    for (slash_run, space_run) in slash_runs.iter().zip(&space_runs) {
        assert_eq!(slash_run.text, space_run.text);
        assert_eq!(slash_run.origin_um, space_run.origin_um);
        assert_eq!(slash_run.page, space_run.page);
    }
    assert_eq!(slash_runs[1].origin_um.1 - slash_runs[0].origin_um.1,
        9_000);
    let literal = FlowSource::new(FlowFormat::Plain, "a\\\na")
        .expect("literal slashes in plain text");
    let (literal_runs, _, _) = layout_flow(&literal,
        &NotebookProfile::default(), Some(&model),
        HeadingDecoration::default()).expect("plain slash measured");
    assert_eq!(literal_runs[1].origin_um.1 - literal_runs[0].origin_um.1,
        29_000);
}

#[test]
fn markdown_thematic_separators_have_identical_native_page_ink() {
    let mut reference = Vec::new();
    for marker in ["---", "***", "___", "-----"] {
        let markdown = format!("a\n\n{marker}\na");
        let source = FlowSource::new(FlowFormat::Markdown, &markdown)
            .expect("standalone Markdown separator syntax");
        let (runs, marks, pages) = layout_flow(
            &source, &NotebookProfile::default(), None,
            HeadingDecoration::default(),
        ).expect("rule consumes one physical row");
        assert_eq!(pages, 1);
        assert_eq!(runs.iter().map(|run| run.text.as_str())
            .collect::<Vec<_>>(), ["a", "a"]);
        assert_eq!(runs[0].origin_um, (22_500, 24_500));
        assert_eq!(runs[1].origin_um, (22_500, 46_500));
        assert_eq!(marks.len(), 1, "one ink line, no delimiter glyphs");
        assert_eq!(marks[0].page, 1);
        assert_eq!(marks[0].start_um, (22_500, 42_000));
        assert_eq!(marks[0].end_um, (82_500, 42_000));
        assert_eq!(marks[0].width_um, 400);
        assert_eq!(marks[0].rgb, [30, 48, 74]);

        let mut rendered = Vec::new();
        for format in ["pdf", "png"] {
            let mut cli = args(&[
                "--export-paper", format, "--printed-sheet", "true",
                "--glyph-stroke", "a", "0", "0", "3500", "4500",
                "250", "#204060", "--flow-markdown", &markdown,
            ]);
            if format == "png" {
                cli.extend(args(&["--dpi", "72"]));
            }
            let bytes = execute_notebook(&cli)
                .expect("real native Markdown separator")
                .expect("PDF/PNG bytes");
            if format == "pdf" {
                assert!(bytes.starts_with(b"%PDF-1.4"));
            } else {
                assert!(bytes.starts_with(b"\x89PNG\r\n\x1a\n"));
            }
            rendered.push(bytes);
        }
        if reference.is_empty() {
            reference = rendered;
        } else {
            assert_eq!(rendered, reference,
                "three thematic spellings and length must replay same ink");
        }
    }
    let plain = execute_notebook(&args(&[
        "--export-paper", "pdf", "--glyph-stroke", "a", "0",
        "0", "3500", "4500", "250", "#204060",
        "--flow-text", "---",
    ])).expect_err("plain hyphens still need authored shapes");
    assert!(plain.contains("U+002D"), "{plain}");
}

#[test]
fn markdown_thematic_rule_follows_page_and_blank_paragraph_budgets() {
    let source = FlowSource::new(FlowFormat::Markdown, "a\n\n---\n\na")
        .expect("blank lines surrounding a thematic rule");
    let (runs, rules, pages) = layout_flow(
        &source, &NotebookProfile::default(), None,
        HeadingDecoration::default(),
    ).expect("blank-line paragraph spacing on either side");
    assert_eq!(pages, 1);
    assert_eq!(rules.len(), 1);
    assert_eq!(rules[0].start_um.1, 42_000);
    assert_eq!(runs[1].origin_um.1 - runs[0].origin_um.1, 26_000);

    let markdown = format!("{}---\na", "a  \n".repeat(28));
    let source = FlowSource::new(FlowFormat::Markdown, &markdown)
        .expect("two-page mixed text and separator");
    let (runs, separators, pages) = layout_flow(
        &source, &NotebookProfile::default(), None,
        HeadingDecoration::default(),
    ).expect("separator spans physical page boundary");
    assert_eq!(pages, 2);
    assert_eq!(runs.len(), 29);
    assert_eq!(separators.len(), 1);
    assert_eq!(separators[0].page, 2);
    assert_eq!(separators[0].start_um, (22_500, 29_000));
    assert_eq!(runs[28].page, 2);
    assert_eq!(runs[28].origin_um.1, 33_500);

    let mut cli = args(&[
        "--export-paper", "pdf", "--printed-sheet", "true",
        "--glyph-stroke", "a", "0", "0", "3500", "4500",
        "250", "#204060", "--flow-markdown", &markdown,
    ]);
    let pdf = execute_notebook(&cli)
        .expect("two-page native PDF").expect("PDF bytes");
    assert_eq!(String::from_utf8_lossy(&pdf)
        .matches("\nstream\n").count(), 2);
    cli[1] = "png".into();
    cli.extend(args(&["--dpi", "72", "--png-page", "2"]));
    let png = execute_notebook(&cli)
        .expect("second-page separator PNG").expect("PNG bytes");
    assert!(png.starts_with(b"\x89PNG\r\n\x1a\n"));
    assert_ne!(png, {
        let mut first = cli.clone();
        let page = first.last_mut().expect("PNG selected page");
        *page = "1".into();
        execute_notebook(&first).expect("page 1")
            .expect("page 1 native ink")
    });
}

#[test]
fn markdown_thematic_rule_refusals_preserve_selected_pdf() {
    let alone = FlowSource::new(FlowFormat::Markdown, "---")
        .expect("standalone rule syntax");
    let error = layout_flow(&alone, &NotebookProfile::default(), None,
        HeadingDecoration::default())
        .expect_err("notebook text flow needs at least one text block");
    assert!(error.contains("one visible text block"), "{error}");

    let destination = test_path().with_extension("pdf");
    fs::write(&destination, b"existing native PDF")
        .expect("test-owned PDF sentinel");
    let filename = destination.to_str().expect("test PDF path");
    for (markdown, message) in [
        ("---", "one visible text block"),
        (&format!("{}---", "a  \n".repeat(128)),
            "128 measured lines"),
        ("a\n- -", "U+002D"),
        ("a\n**", "U+002A"),
        ("a\n__", "U+005F"),
        (&format!("{}---\na", "a  \n".repeat(28)),
            "requires 2 physical pages"),
    ] {
        let error = execute_notebook(&args(&[
            "--export-paper", "pdf", "--glyph-stroke", "a",
            "0", "0", "3500", "4500", "250", "#204060",
            "--flow-markdown", markdown, "--pages", "1",
            "--output", filename, "--replace",
        ])).expect_err("unsupported or over-budget rule must refuse");
        assert!(error.contains(message), "{error}");
        assert_eq!(fs::read(&destination).expect("sentinel persists"),
            b"existing native PDF");
    }
    fs::remove_file(destination).expect("remove test-owned PDF");
}

#[test]
fn markdown_heading_keeps_following_thematic_rule_on_next_page() {
    let markdown = format!("{}# a\n---\na", "a  \n".repeat(27));
    let source = FlowSource::new(FlowFormat::Markdown, &markdown)
        .expect("near-bottom heading with physical rule");
    let (runs, rules, pages) = layout_flow(
        &source, &NotebookProfile::default(), None,
        HeadingDecoration::default(),
    ).expect("heading and rule placed together");
    assert_eq!(pages, 2);
    assert_eq!(runs.len(), 29);
    assert_eq!(rules.len(), 2, "heading underline and separator");
    assert_eq!(runs[27].page, 2, "heading moves off orphan row");
    assert_eq!(runs[27].origin_um, (22_500, 24_500));
    assert_eq!(rules[0].page, 2);
    assert_eq!(rules[1].page, 2);
    assert_eq!(rules[1].start_um, (22_500, 38_000));
    assert_eq!(runs[28].page, 2);
    assert_eq!(runs[28].origin_um, (22_500, 42_500));
    let pdf = execute_notebook(&args(&[
        "--export-paper", "pdf", "--printed-sheet", "true",
        "--glyph-stroke", "a", "0", "0", "3500", "4500",
        "250", "#204060", "--flow-markdown", &markdown,
    ])).expect("two-page PDF").expect("ink bytes");
    let text = String::from_utf8_lossy(&pdf);
    let pages = text.split("\nstream\n").skip(1)
        .map(|part| part.split("\nendstream").next().unwrap_or(""))
        .collect::<Vec<_>>();
    assert_eq!(pages.len(), 2);
    assert!(!pages[0].contains("0.118 0.188 0.290 RG"),
        "heading/rule ink must not stay on previous page");
    assert!(pages[1].contains("0.118 0.188 0.290 RG"),
        "native title and separator ink stay together");
}

#[test]
fn markdown_mixed_crlf_and_lf_replay_identically_in_native_exports() {
    let lf = "# a\n> a\\\n> a\n---\na  \na";
    let crlf = lf.replace('\n', "\r\n");
    let normalized = FlowSource::new(FlowFormat::Markdown, &crlf)
        .expect("CRLF normalizes on admission");
    assert_eq!(normalized.text, lf);
    for format in ["pdf", "png"] {
        let render = |markdown: &str| {
            let mut cli = args(&[
                "--export-paper", format, "--printed-sheet", "true",
                "--glyph-stroke", "a", "0", "0", "3500", "4500",
                "250", "#204060", "--flow-markdown", markdown,
            ]);
            if format == "png" {
                cli.extend(args(&["--dpi", "72"]));
            }
            execute_notebook(&cli).expect("native source export")
                .expect("image bytes")
        };
        assert_eq!(render(lf), render(&crlf),
            "LF and CRLF must compose identical page-local ink");
    }
}

#[test]
fn leading_utf8_bom_keeps_real_plain_and_markdown_pdf_png_identical() {
    let file = test_path().with_extension("md");
    for (format, flag, text) in [
        (FlowFormat::Plain, "--flow-text", "a\na"),
        (FlowFormat::Markdown, "--flow-markdown",
            "# a\n> a  \n> a\n---\na"),
    ] {
        let bom = format!("\u{feff}{text}");
        let bom_crlf = bom.replace('\n', "\r\n");
        fs::write(&file, bom_crlf.as_bytes())
            .expect("test-owned UTF-8 BOM source");
        let read = FlowSource::from_file(format,
            file.to_str().expect("local source path"))
            .expect("BOM and CRLF imported as normal source");
        assert_eq!(read.text, text);
        assert_eq!(read.format, format);
        for paper in ["pdf", "png"] {
            let render = |source: &str| {
                let mut cli = args(&[
                    "--export-paper", paper, "--printed-sheet", "true",
                    "--glyph-stroke", "a", "0", "0", "3500", "4500",
                    "250", "#204060", flag, source,
                ]);
                if paper == "png" {
                    cli.extend(args(&["--dpi", "72"]));
                }
                execute_notebook(&cli).expect("native source export")
                    .expect("native page bytes")
            };
            let baseline = render(text);
            assert_eq!(baseline, render(&bom),
                "initial transport signature is not authored ink");
            assert_eq!(baseline, render(&bom_crlf),
                "BOM and CRLF normalize before rendering");
        }
    }
    fs::remove_file(file).expect("remove test source file");
}

#[test]
fn utf8_bom_only_strips_at_first_position_and_keeps_byte_cap() {
    for invalid in [
        "\u{feff}".to_owned(),
        format!("\u{feff}{}", "a".repeat(4_094)),
    ] {
        let error = FlowSource::new(FlowFormat::Markdown, &invalid)
            .expect_err("empty or oversized BOM input");
        assert!(error.contains("1..4096 UTF-8 bytes"), "{error}");
    }
    let initial_two = format!("\u{feff}\u{feff}a");
    let after_first = FlowSource::new(FlowFormat::Markdown, &initial_two)
        .expect("one BOM stripped, other remains authored text");
    assert_eq!(after_first.text, "\u{feff}a");
    let embedded = format!("a\u{feff}a");
    let target = test_path().with_extension("pdf");
    fs::write(&target, b"original PDF").expect("test sentinel");
    let output = target.to_str().expect("test PDF path");
    for invalid in [initial_two.as_str(), embedded.as_str()] {
        let error = execute_notebook(&args(&[
            "--export-paper", "pdf", "--glyph-stroke", "a",
            "0", "0", "3500", "4500", "250", "#204060",
            "--flow-markdown", invalid, "--output", output, "--replace",
        ])).expect_err("only first BOM is transport metadata");
        assert!(error.contains("U+FEFF"), "{error}");
        assert_eq!(fs::read(&target).expect("unchanged output"),
            b"original PDF");
    }
    fs::remove_file(target).expect("remove test sentinel");
}

#[test]
fn markdown_soft_and_hard_breaks_change_measured_page_count() {
    let soft = FlowSource::new(FlowFormat::Markdown, &"a\n".repeat(40))
        .expect("soft-wrapped paragraph");
    let hard = FlowSource::new(FlowFormat::Markdown, &"a  \n".repeat(40))
        .expect("hard breaks");
    let plain = FlowSource::new(FlowFormat::Plain, &"a\n".repeat(40))
        .expect("plain lines");
    let profile = NotebookProfile::default();
    let counts = [soft, hard, plain].map(|source| {
        let (runs, _, pages) = layout_flow(
            &source, &profile, None, HeadingDecoration::default(),
        ).expect("bounded physical flow");
        (runs.len(), pages)
    });
    assert_eq!(counts[0].1, 1, "soft wraps consume only measured rows");
    assert!(counts[0].0 < 40);
    assert_eq!(counts[1], (40, 2), "two spaces request hard rows");
    assert_eq!(counts[2], (40, 2), "plain text keeps authored rows");
}

#[test]
fn markdown_soft_wrap_exports_same_real_pages_as_explicit_paragraph() {
    let base = args(&[
        "--printed-sheet", "true", "--glyph-stroke", "a",
        "0", "0", "3500", "4500", "250", "#123456",
        "--glyph-stroke", "n\u{303}", "0", "0", "3500", "4500",
        "250", "#123456",
    ]);
    for format in ["pdf", "png"] {
        let mut option = base.clone();
        option.extend(args(&["--export-paper", format]));
        if format == "png" {
            option.extend(args(&["--dpi", "72"]));
        }
        let render = |text: &str| {
            let mut command = option.clone();
            command.extend(args(&["--flow-markdown", text]));
            execute_notebook(&command)
                .expect("native rendered Markdown").expect("image bytes")
        };
        let soft = render("a\nn\u{303}");
        let joined = render("a n\u{303}");
        let hard = render("a  \nn\u{303}");
        assert_eq!(soft, joined, "soft break matches explicit space");
        assert_ne!(soft, hard, "hard row break changes physical ink");
        if format == "pdf" {
            assert!(soft.starts_with(b"%PDF-1.4"));
        } else {
            assert!(soft.starts_with(b"\x89PNG\r\n\x1a\n"));
        }
    }
}

#[test]
fn markdown_blank_runs_collapse_and_plain_blanks_keep_physical_rows() {
    let profile = NotebookProfile::default();
    let single = FlowSource::new(FlowFormat::Markdown, "a\n\na")
        .expect("one Markdown blank");
    let many = FlowSource::new(FlowFormat::Markdown,
        "\n\na\n\n\n\n\na\n\n")
        .expect("leading, repeated and trailing Markdown blanks");
    let (one, marks, pages) = layout_flow(&single, &profile, None,
        HeadingDecoration::default()).expect("one paragraph gap");
    let (collapsed, other_marks, collapsed_pages) = layout_flow(
        &many, &profile, None, HeadingDecoration::default(),
    ).expect("one collapsed paragraph gap");
    assert_eq!((pages, collapsed_pages), (1, 1));
    assert_eq!(one.iter().map(|run| (run.page, run.origin_um, &run.text))
        .collect::<Vec<_>>(), collapsed.iter().map(|run| {
        (run.page, run.origin_um, &run.text)
    }).collect::<Vec<_>>(), "empty Markdown runs add only one gap");
    assert_eq!(marks, other_marks);
    assert_eq!(one[1].origin_um.1 - one[0].origin_um.1, 13_000);
    let plain = FlowSource::new(FlowFormat::Plain, "a\n\n\na")
        .expect("two explicit empty rows");
    let (runs, _, _) = layout_flow(&plain, &profile, None,
        HeadingDecoration::default()).expect("blank physical rows");
    assert_eq!(runs.len(), 2);
    assert_eq!(runs[1].origin_um.1 - runs[0].origin_um.1, 27_000,
        "both empty plain-text lines consume a full 9-mm writing row");
    // These are the same actual page bytes for any number of intervening
    // Markdown empty lines, and remain different from explicit plain rows.
    for format in ["pdf", "png"] {
        let render = |input: &str, markdown: bool| {
            let mut cli = args(&[
                "--export-paper", format, "--printed-sheet", "true",
                "--glyph-stroke", "a", "0", "0", "3500", "4500",
                "250", "#204060", if markdown { "--flow-markdown" }
                    else { "--flow-text" }, input,
            ]);
            if format == "png" { cli.extend(args(&["--dpi", "72"])); }
            execute_notebook(&cli).expect("native blank-space export")
                .expect("physical output")
        };
        let canonical = render("a\n\na", true);
        assert_eq!(canonical, render("\n\na\n\n\n\n\na\n\n", true));
        assert_ne!(canonical, render("a\n\n\na", false));
        if format == "png" {
            assert!(canonical.starts_with(b"\x89PNG\r\n\x1a\n"));
        } else {
            assert!(canonical.starts_with(b"%PDF-1.4"));
        }
    }
}

#[test]
fn plain_blank_rows_paginate_and_obey_row_and_page_limits_atomically() {
    let long = format!("a\n{}a", "\n".repeat(27));
    let source = FlowSource::new(FlowFormat::Plain, &long)
        .expect("bounded plain source");
    let (runs, marks, pages) = layout_flow(
        &source, &NotebookProfile::default(), None,
        HeadingDecoration::default(),
    ).expect("blank rows carry physical pagination");
    assert_eq!(pages, 2, "29 rows no longer collapse onto page 1");
    assert_eq!(runs.len(), 2, "no invisible glyphs are synthesized");
    assert!(marks.is_empty());
    assert_eq!((runs[0].page, runs[1].page), (1, 2));
    assert_eq!(runs[0].origin_um, runs[1].origin_um,
        "page 2 resumes at the selected physical top inset");
    let mut cli = args(&[
        "--export-paper", "pdf", "--printed-sheet", "true",
        "--glyph-stroke", "a", "0", "0", "3500", "4500",
        "250", "#204060", "--flow-text", &long,
    ]);
    let pdf = execute_notebook(&cli).expect("two-page native PDF")
        .expect("PDF bytes");
    assert_eq!(String::from_utf8_lossy(&pdf)
        .matches("\nstream\n").count(), 2);
    cli[1] = "png".into();
    cli.extend(args(&["--png-page", "2", "--dpi", "72"]));
    let png = execute_notebook(&cli).expect("physical second-page PNG")
        .expect("PNG bytes");
    assert!(png.starts_with(b"\x89PNG\r\n\x1a\n"));
    let target = test_path();
    let filename = target.to_str().expect("test-owned output path");
    fs::write(&target, b"existing valid PDF").expect("sentinel");
    for (plain, expected) in [
        (long.as_str(), "requires 2 physical pages"),
        (&format!("a\n{}a", "\n".repeat(128)),
            "flow exceeds 128 measured lines"),
    ] {
        let error = execute_notebook(&args(&[
            "--export-paper", "pdf", "--printed-sheet", "true",
            "--glyph-stroke", "a", "0", "0", "3500", "4500",
            "250", "#204060", "--flow-text", plain,
            "--pages", "1", "--output", filename, "--replace",
        ])).expect_err("insufficient page or row budget must refuse");
        assert!(error.contains(expected), "{error}");
        assert_eq!(fs::read(&target).expect("preserved PDF"),
            b"existing valid PDF");
    }
    fs::remove_file(target).expect("remove test-owned sentinel");
}

#[test]
fn markdown_invalid_structural_block_preserves_existing_output() {
    let target = test_path();
    let filename = target.to_str().expect("test-owned output path");
    fs::write(&target, b"existing PDF").expect("sentinel");
    for source in ["a\n- [ ]", "a\n* ", "a\n+ ", "a\n# ", "a\n## ", "a\n### "] {
        let error = execute_notebook(&args(&[
            "--export-paper", "pdf", "--glyph-stroke", "a", "0", "0",
            "3500", "4500", "250", "#123456", "--flow-markdown",
            source, "--output", filename, "--replace",
        ])).expect_err("empty structural Markdown block must refuse");
        assert!(error.contains("empty Markdown block refused"), "{error}");
        assert_eq!(fs::read(&target).expect("preserved sentinel"),
            b"existing PDF");
    }
    fs::remove_file(target).expect("remove test-owned output");
}

#[test]
fn plus_markdown_bullets_reuse_existing_real_ink_in_pdf_and_png() {
    let profile = NotebookProfile::default();
    let sources = ["- a a\na", "* a a\na", "+ a a\na"];
    let mut reference = Vec::new();
    for source in sources {
        let admitted = FlowSource::new(FlowFormat::Markdown, source)
            .expect("Markdown source accepted");
        let (runs, marks, pages) = layout_flow(
            &admitted, &profile, None, HeadingDecoration::default(),
        ).expect("uniform unordered list ink");
        assert_eq!(pages, 1);
        assert_eq!(runs.iter().map(|run| run.text.as_str())
            .collect::<Vec<_>>(), ["a a", "a"],
            "following body paragraph must not join a bullet item");
        assert_eq!(marks.len(), 1, "one synthetic bullet, not a + glyph");
        assert_eq!(marks[0].width_um, 450);
        assert_eq!(marks[0].start_um.0 + 1_700,
            marks[0].end_um.0);
        assert_eq!(runs[0].origin_um.0, runs[1].origin_um.0 + 6_000);
        let mut png_bytes = Vec::new();
        for format in ["pdf", "png"] {
            let mut command = args(&[
                "--export-paper", format, "--printed-sheet", "true",
                "--glyph-stroke", "a", "0", "0", "3500", "4500",
                "250", "#204060", "--flow-markdown", source,
            ]);
            if format == "png" {
                command.extend(args(&["--dpi", "72"]));
            }
            let bytes = execute_notebook(&command)
                .expect("native plus bullet export").expect("output bytes");
            if format == "png" {
                assert!(bytes.starts_with(b"\x89PNG\r\n\x1a\n"));
            } else {
                assert!(bytes.starts_with(b"%PDF-1.4"));
            }
            png_bytes.push(bytes);
        }
        if reference.is_empty() {
            reference = png_bytes;
        } else {
            assert_eq!(png_bytes, reference,
                "all three Markdown bullet spellings render identical ink");
        }
    }
    let plain = execute_notebook(&args(&[
        "--export-paper", "pdf", "--glyph-stroke", "a", "0", "0",
        "3500", "4500", "250", "#204060",
        "--flow-text", "+ a a",
    ])).expect_err("plus is ordinary text in plain mode");
    assert!(plain.contains("U+002B"), "{plain}");
}

#[test]
fn ordered_markdown_list_numbers_remain_authored_ink_without_bullets() {
    let source = FlowSource::new(
        FlowFormat::Markdown, "# a\n1. a\n2) a\n- a",
    ).expect("bounded mixed Markdown");
    let (runs, ornaments, pages) = layout_flow(
        &source, &NotebookProfile::default(), None,
        HeadingDecoration::default(),
    ).expect("ordered list compiled");
    assert_eq!(pages, 1);
    assert_eq!(runs.iter().map(|run| run.text.as_str())
        .collect::<Vec<_>>(), ["a", "1. a", "2) a", "a"]);
    assert_eq!(ornaments.len(), 2, "heading and unordered bullet only");
    assert_eq!(runs[1].origin_um.0, runs[2].origin_um.0);
    assert_eq!(runs[1].origin_um.0, runs[3].origin_um.0);
    assert_eq!(runs[1].origin_um.0 - runs[0].origin_um.0, 6_000);
    let mut glyphs = GlyphOverlay::default();
    let mut command = args(&[
        "--printed-sheet", "true", "--export-paper", "pdf",
        "--flow-markdown", "# a\n1. a\n2) a\n- a",
    ]);
    for grapheme in ["a", "1", ".", "2", ")"] {
        let values = args(&[
            grapheme, "0", "0", "3500", "4500", "250", "#204060",
        ]);
        glyphs.parse("--glyph-stroke", &mut values.iter())
            .expect("one native authored ink shape");
        command.extend(args(&["--glyph-stroke", grapheme, "0", "0",
            "3500", "4500", "250", "#204060"]));
    }
    glyphs.runs = runs;
    assert_eq!(glyphs.expand(&[], None).expect("authored ordered ink").len(),
        8, "numbers and separators are real authored glyphs");
    let pdf = execute_notebook(&command).expect("native PDF")
        .expect("PDF bytes");
    assert!(pdf.starts_with(b"%PDF-1.4"));
    let format_at = command.iter().position(|item| item == "pdf")
        .expect("format option");
    command[format_at] = "png".into();
    command.extend(args(&["--dpi", "72"]));
    let png = execute_notebook(&command).expect("native PNG")
        .expect("PNG bytes");
    assert!(png.starts_with(b"\x89PNG\r\n\x1a\n"));
}

#[test]
fn ordered_markdown_wrap_does_not_repeat_number_on_continuation() {
    let source = FlowSource::new(
        FlowFormat::Markdown, &format!("1. {}", "a ".repeat(50)),
    ).expect("bounded long list item");
    let (runs, marks, _) = layout_flow(
        &source, &NotebookProfile::default(), None,
        HeadingDecoration::default(),
    ).expect("wrapped ordered item");
    assert!(runs.len() > 1);
    assert!(runs[0].text.starts_with("1. "));
    assert!(runs.iter().skip(1).all(|run| !run.text.contains("1.")));
    assert!(runs.iter().skip(1).all(|run| {
        run.origin_um.0 == runs[0].origin_um.0 + 12_000
    }), "continuations align under the first item glyph, not its number");
    assert!(marks.is_empty(), "ordered numeral is authored, not a bullet");
    let unrecognized = FlowSource::new(
        FlowFormat::Markdown, "1234567890. a",
    ).expect("ten-digit marker treated as text");
    let (runs, _, _) = layout_flow(
        &unrecognized, &NotebookProfile::default(), None,
        HeadingDecoration::default(),
    ).expect("unrecognized numbered marker is ordinary text");
    assert_eq!(runs[0].origin_um.0 + 6_000,
        28_500, "ordinary long number has no list indentation");
}

#[test]
fn ordered_markdown_hanging_indent_tracks_label_and_item_spacing() {
    let markdown = format!(
        "1. {}\n12)   {}", "a".repeat(65), "a ".repeat(55),
    );
    let source = FlowSource::new(FlowFormat::Markdown, &markdown)
        .expect("bounded numbered Markdown");
    let (runs, ornaments, pages) = layout_flow(
        &source, &NotebookProfile::default(), None,
        HeadingDecoration::default(),
    ).expect("measured hanging-indent list");
    assert_eq!(pages, 1);
    assert!(ornaments.is_empty(), "numbered labels are authored glyphs");
    let second_index = runs.iter().position(|run| run.text.starts_with("12)"))
        .expect("second numbered item");
    assert!(second_index >= 2, "first long word must wrap");
    assert!(runs.len() > second_index + 1, "second item must wrap");
    assert!(runs[0].text.starts_with("1. a"),
        "the label must share its first row with an item glyph");
    assert!(runs[second_index].text.starts_with("12)   a"),
        "the wider marker must keep authored separator spaces");
    for run in &runs[1..second_index] {
        assert_eq!(run.origin_um.0, runs[0].origin_um.0 + 12_000,
            "one digit, delimiter and space form the hanging indent");
        assert!(!run.text.starts_with("1."), "no duplicated marker");
    }
    for run in &runs[second_index + 1..] {
        assert_eq!(run.origin_um.0,
            runs[second_index].origin_um.0 + 24_000,
            "two digits, delimiter and three spaces form the indent");
        assert!(!run.text.starts_with("12)"), "no duplicated marker");
    }
    // Compare actual physical ink, not only TextRun origins. The first
    // body glyph sits after the label and matches the continuation column.
    let mut glyphs = GlyphOverlay::default();
    for grapheme in ["1", ".", "2", ")", "a"] {
        glyphs.parse("--glyph-stroke", &mut args(&[
            grapheme, "0", "0", "3500", "4500", "250", "#204060",
        ]).iter()).expect("authored glyph");
    }
    let first_row_ink_count = runs[0].text.chars()
        .filter(|ch| *ch != ' ').count();
    glyphs.runs = runs;
    let ink = glyphs.expand(&[], None).expect("native ink placement");
    assert_eq!(ink[2].start_um.0, ink[first_row_ink_count].start_um.0,
        "first item glyph and first continuation glyph align");
    // The label is still drawn in both native physical output formats.
    let mut cli = args(&[
        "--printed-sheet", "true", "--export-paper", "pdf",
        "--flow-markdown", &markdown,
    ]);
    for grapheme in ["1", ".", "2", ")", "a"] {
        cli.extend(args(&[
            "--glyph-stroke", grapheme, "0", "0", "3500", "4500",
            "250", "#204060",
        ]));
    }
    let pdf = execute_notebook(&cli).expect("native paginated PDF")
        .expect("PDF bytes");
    assert!(pdf.starts_with(b"%PDF-1.4"));
    cli[3] = "png".into();
    cli.extend(args(&["--dpi", "72"]));
    let png = execute_notebook(&cli).expect("same native PNG")
        .expect("PNG bytes");
    assert!(png.starts_with(b"\x89PNG\r\n\x1a\n"));
}

#[test]
fn oversized_ordered_marker_refuses_before_inventing_orphan_line() {
    let mut paper = NotebookProfile::default();
    paper.paper_json = paper.paper_json.replace(
        "\"margin_right_um\": 14000", "\"margin_right_um\": 149000",
    );
    assert!(paper.printable_rect_um().is_ok(),
        "narrow A4 printable region is still valid paper");
    let source = FlowSource::new(FlowFormat::Markdown,
        "123456789. a").expect("source accepted");
    let error = layout_flow(&source, &paper, None,
        HeadingDecoration::default()).expect_err("label cannot fit");
    assert!(error.contains("ordered list label and first glyph exceed"),
        "{error}");
    let normal = FlowSource::new(FlowFormat::Markdown, "1. a")
        .expect("small ordered marker");
    assert!(layout_flow(&normal, &paper, None,
        HeadingDecoration::default()).is_ok(),
        "short label still admits physical text");
}

#[test]
fn ordered_markdown_refuses_unwritten_numbers_and_empty_items_atomically() {
    let target = test_path();
    let filename = target.to_str().expect("owned path");
    fs::write(&target, b"preserved PDF").expect("sentinel");
    for (markdown, expected) in [
        ("a\n1. ", "empty Markdown block"),
        ("a\n2) ", "empty Markdown block"),
        ("1. a", "U+0031"),
    ] {
        let error = execute_notebook(&args(&[
            "--export-paper", "pdf", "--glyph-stroke", "a", "0", "0",
            "3500", "4500", "250", "#123456", "--flow-markdown",
            markdown, "--output", filename, "--replace",
        ])).expect_err("invalid ordered list refuses");
        assert!(error.contains(expected), "{error}");
        assert_eq!(fs::read(&target).expect("original output"),
            b"preserved PDF");
    }
    fs::remove_file(target).expect("remove owned output");
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
fn bilingual_punctuation_wrapping_matches_direct_native_pdf_and_png_ink() {
    let paper = NotebookProfile::default();
    let region = FlowRegion {
        left: 30_000, top: 40_000, width: 32_000, height: 36_000,
    };
    let glyphs = ["a", "b", "c", "d", "e", "f", "g", "¿", "¡",
        "?", "!", ".", "ñ", "n\u{303}", "«", "»", "“", "”",
        "‘", "’", "(", ")", "[", "]", "{", "}"];
    for (format, text) in [
        (FlowFormat::Plain, "abcde¿n\u{303}g"),
        (FlowFormat::Markdown, "abcdef?ñ"),
        (FlowFormat::Plain, "abcde¡fg"),
        (FlowFormat::Markdown, "abcdef.!"),
        (FlowFormat::Plain, "abcde«fg"),
        (FlowFormat::Markdown, "abcdef”g"),
        (FlowFormat::Plain, "abcde“n\u{303}g"),
        (FlowFormat::Markdown, "abcdef’ñ"),
        (FlowFormat::Plain, "abcde(fg"),
        (FlowFormat::Markdown, "abcdef)g"),
    ] {
        let source = FlowSource::new(format, text)
            .expect("bilingual source admitted");
        let (rows, ornaments, pages) = layout_flow_with_region(
            &source, &paper, None, HeadingDecoration::default(),
            Some(region),
        ).expect("bounded Unicode line breaking");
        assert_eq!(pages, 1);
        assert!(ornaments.is_empty());
        assert_eq!(rows.len(), 2);
        assert_eq!(rows.iter().map(|row| row.text.as_str())
            .collect::<String>(), text, "exact source ink retained");
        if text.contains("n\u{303}") {
            assert!(rows.iter().any(|row| row.text.contains("n\u{303}")),
                "decomposed accent cannot break between physical rows");
        }
        for export_format in ["pdf", "png"] {
            let mut baseline = args(&[
                "--printed-sheet", "true", "--export-paper", export_format,
            ]);
            if export_format == "png" {
                baseline.extend(args(&["--dpi", "72"]));
            }
            for glyph in glyphs {
                baseline.extend(args(&[
                    "--glyph-stroke", glyph, "500", "500", "1800",
                    "2800", "250", "#204060",
                ]));
            }
            let mut flowing = baseline.clone();
            flowing.extend(args(&[
                if format == FlowFormat::Plain {
                    "--flow-text"
                } else {
                    "--flow-markdown"
                },
                text,
                "--flow-region", "30000", "40000", "32000", "36000",
            ]));
            let mut direct = baseline;
            for row in &rows {
                direct.extend(args(&[
                    "--place-text", &row.page.to_string(),
                    &row.origin_um.0.to_string(),
                    &row.origin_um.1.to_string(), &row.text,
                ]));
            }
            let actual = execute_notebook(&flowing)
                .expect("real bilingual flow export").expect("physical bytes");
            let expected = execute_notebook(&direct)
                .expect("independent direct ink export")
                .expect("physical bytes");
            assert_eq!(actual, expected, "{export_format} exact ink parity");
            assert_eq!(actual, execute_notebook(&flowing)
                .expect("deterministic replay").expect("physical bytes"));
        }
    }
}

#[test]
fn bilingual_punctuation_flow_refuses_missing_ink_before_replacing_exports() {
    for (text, missing) in [
        ("abcde¿fg", "U+00BF"), ("abcde«fg", "U+00AB"),
    ] {
        for format in ["pdf", "png"] {
            let output = test_path().with_extension(format);
            let name = output.to_str().expect("owned export path");
            fs::write(&output, b"preserve previously accepted page")
                .expect("selected output sentinel");
            let mut command = args(&[
                "--printed-sheet", "true", "--export-paper", format,
                "--flow-text", text,
                "--flow-region", "30000", "40000", "32000", "36000",
            ]);
            if format == "png" {
                command.extend(args(&["--dpi", "72"]));
            }
            for glyph in ["a", "b", "c", "d", "e", "f", "g"] {
                command.extend(args(&[
                    "--glyph-stroke", glyph, "500", "500", "1800", "2800",
                    "250", "#204060",
                ]));
            }
            command.extend(args(&["--output", name, "--replace"]));
            let error = execute_notebook(&command)
                .expect_err("missing quoted punctuation cannot invent ink");
            assert!(error.contains(missing), "{error}");
            assert_eq!(fs::read(&output).expect("target unchanged"),
                b"preserve previously accepted page");
            fs::remove_file(output).expect("remove test-owned target");
        }
    }
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
fn space_attached_combining_grapheme_survives_native_flow_and_ink() {
    let source_text = "a \u{0301}b";
    let region = FlowRegion {
        left: 30_000, top: 40_000, width: 14_000, height: 32_000,
    };
    for format in [FlowFormat::Plain, FlowFormat::Markdown] {
        let source = FlowSource::new(format, source_text)
            .expect("source has one exact space-attached glyph");
        let (runs, ornaments, pages) = layout_flow_with_region(
            &source, &NotebookProfile::default(), None,
            HeadingDecoration::default(), Some(region),
        ).expect("space-attached combining mark is measurable");
        assert_eq!(pages, 1);
        assert!(ornaments.is_empty());
        assert_eq!(runs.iter().map(|run| run.text.as_str())
            .collect::<String>(), source_text);
        assert!(runs.len() >= 2, "selected physical region wraps ink");

        let mut glyphs = GlyphOverlay::default();
        for (key, color) in [
            ("a", "#204060"),
            (" \u{0301}", "#D02040"),
            ("b", "#206040"),
        ] {
            glyphs.parse("--glyph-stroke", &mut args(&[
                key, "200", "500", "1500", "3000", "250", color,
            ]).iter()).expect("exact authored Unicode shape");
        }
        glyphs.runs = runs;
        let marks = glyphs.expand(&[], None)
            .expect("all three exact shapes render in physical rows");
        assert_eq!(marks.len(), 3);
        assert_eq!(marks.iter().map(|mark| mark.rgb)
            .collect::<Vec<_>>(), [
                [0x20, 0x40, 0x60],
                [0xD0, 0x20, 0x40],
                [0x20, 0x60, 0x40],
            ]);
        assert!(marks.iter().all(|mark| mark.page == 1));
        assert_ne!(marks[0].start_um, marks[1].start_um);
        assert_ne!(marks[1].start_um, marks[2].start_um);
    }

    // Both real native output paths include the attached mark's unique
    // red stroke; it is never omitted as an ordinary ASCII space.
    for (kind, options) in [
        ("pdf", vec![]),
        ("png", args(&["--dpi", "72", "--png-page", "1"])),
    ] {
        let mut command = args(&[
            "--printed-sheet", "true", "--export-paper", kind,
            "--flow-markdown", source_text,
            "--flow-region", "30000", "40000", "14000", "32000",
        ]);
        command.extend(options);
        for (key, color) in [
            ("a", "#204060"),
            (" \u{0301}", "#D02040"),
            ("b", "#206040"),
        ] {
            command.extend(args(&[
                "--glyph-stroke", key, "200", "500",
                "1500", "3000", "250", color,
            ]));
        }
        let bytes = execute_notebook(&command)
            .expect("native export with space-attached ink")
            .expect("physical output bytes");
        if kind == "pdf" {
            assert!(bytes.starts_with(b"%PDF-1.4"));
            let content = String::from_utf8_lossy(&bytes);
            assert_eq!(content.matches(" m\n").count(), 3);
        } else {
            assert!(bytes.starts_with(b"\x89PNG\r\n\x1a\n"));
        }
        assert_eq!(bytes, execute_notebook(&command)
            .expect("deterministic replay").expect("replayed output"));
    }
}

#[test]
fn native_seeded_variation_matches_browser_accents_three_times() {
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
        args(&[
            "--glyph-variation", "4294967296", "0", "0",
            "0", "0", "0", "0",
        ]),
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
    let markdown = format!("# á\n\n- á á\n{}", "á  \n".repeat(34));
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
fn indexed_unicode_stroke_lookup_preserves_interleaved_library_order()
-> Result<(), String> {
    // A full native library can interleave matching strokes among thousands
    // of unrelated exact glyphs. Lookup must preserve authored stroke order,
    // fallback identity and per-occurrence variation (not hash-map order).
    let mut dense = GlyphOverlay::default();
    let mut sparse = GlyphOverlay::default();
    for index in 0..4096usize {
        let matching = matches!(index, 0 | 2048 | 4095);
        let grapheme = if matching {
            "a".to_owned()
        } else {
            char::from_u32(0x4E00 + u32::try_from(index)
                .map_err(|error| error.to_string())?)
                .ok_or("test CJK glyph out of range")?.to_string()
        };
        let shape = GlyphStroke {
            grapheme,
            start_um: (600 + u32::from(matching), 550),
            end_um: (2000, 2800),
            width_um: 250,
            rgb: [u8::try_from(index % 255)
                .map_err(|error| error.to_string())?, 48, 70],
        };
        if matching {
            sparse.strokes.push(shape.clone());
        }
        dense.strokes.push(shape);
    }
    for overlay in [&mut dense, &mut sparse] {
        overlay.fallbacks.push(GlyphFallback {
            missing: "ó".into(), authored: "a".into(),
        });
        overlay.runs.push(TextRun {
            heading_ink: None, origin_um: (25000, 30000),
            page: 1, text: "aóa aó".into(),
        });
    }
    let ordinary = sparse.expand(&[], None)?;
    if ordinary.len() != 15 || dense.expand(&[], None)? != ordinary {
        return Err("full library changed exact authored stroke order".into());
    }
    for overlay in [&mut dense, &mut sparse] {
        overlay.parse("--glyph-variation", &mut args(&[
            "42", "4", "4", "5", "3", "50", "15",
        ]).iter())?;
    }
    let sampled = sparse.expand(&[], None)?;
    if sampled.len() != ordinary.len()
        || sampled == ordinary || dense.expand(&[], None)? != sampled {
        return Err("full library changed seeded exact fallback ink".into());
    }
    if dense.expand(&[], None)? != sampled {
        return Err("indexed Unicode ink is not deterministic".into());
    }
    Ok(())
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
    let model = decode_statistics_json(
        &statistics, &UnicodeGraphemeSegmentation,
    ).expect("reopen exact grapheme means");
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
fn typed_placed_text_runs_keep_cli_admission_at_ink_boundary()
-> Result<(), String> {
    let mut glyphs = GlyphOverlay::default();
    glyphs.parse("--glyph-stroke", &mut args(&[
        "a", "600", "500", "1800", "3000", "250", "#123456",
    ]).iter())?;
    glyphs.parse("--place-text", &mut args(&[
        "1", "25000", "30000", "a",
    ]).iter())?;
    let baseline = glyphs.expand(&[], None)?;
    let valid = glyphs.runs[0].clone();
    let mut beyond_page = valid.clone();
    beyond_page.page = 33;
    let mut beyond_origin = valid.clone();
    beyond_origin.origin_um.0 = 700_001;
    let mut no_text = valid.clone();
    no_text.text.clear();
    let mut spaces = valid.clone();
    spaces.text = "  ".into();
    let mut long_text = valid.clone();
    long_text.text = "a".repeat(97);
    let mut oversized_utf8 = valid.clone();
    oversized_utf8.text = "a\u{301}\u{302}\u{303}\u{304}".repeat(43);
    let mut control_text = valid.clone();
    control_text.text = "a\n".into();
    let mut separator_text = valid.clone();
    separator_text.text = "a\u{2028}".into();
    let mut zero_page = valid.clone();
    zero_page.page = 0;
    let mut too_low_heading = valid.clone();
    too_low_heading.heading_ink = Some(HeadingInk {
        color: None, scale_percent: 99,
    });
    let mut too_high_heading = valid.clone();
    too_high_heading.heading_ink = Some(HeadingInk {
        color: None, scale_percent: 251,
    });
    for (name, invalid) in [
        ("page 33", beyond_page),
        ("page 0", zero_page),
        ("unbounded origin", beyond_origin),
        ("empty text", no_text),
        ("space-only text", spaces),
        ("97 graphemes", long_text),
        ("384-byte source limit", oversized_utf8),
        ("embedded newline", control_text),
        ("Unicode line separator", separator_text),
        ("heading scale below range", too_low_heading),
        ("heading scale above range", too_high_heading),
    ] {
        glyphs.runs[0] = invalid;
        if glyphs.expand(&[], None).is_ok() {
            return Err(format!("typed {name} escaped run admission"));
        }
        glyphs.runs[0] = valid.clone();
        if glyphs.expand(&[], None)? != baseline {
            return Err(format!("{name} altered accepted render replay"));
        }
    }
    Ok(())
}

#[test]
fn typed_glyph_audit_obeys_native_text_admission()
-> Result<(), String> {
    let mut glyphs = GlyphOverlay::default();
    glyphs.parse("--glyph-stroke", &mut args(&[
        "á", "600", "500", "1800", "3000", "250", "#123456",
    ]).iter())?;
    let baseline = glyphs.audit_text("á")?;
    let maximum = "á".repeat(2_048);
    if !String::from_utf8(glyphs.audit_text(&maximum)?)
        .map_err(|error| error.to_string())?
        .contains("Missing authored glyphs: 0") {
        return Err("4096-byte Unicode audit boundary was refused".into());
    }
    for (name, invalid) in [
        ("empty source", String::new()),
        ("only structural source", "  \r\n ".into()),
        ("oversized source", "á".repeat(2_049)),
        ("unpaired carriage return", "á\rx".into()),
        ("control byte", "á\u{0001}".into()),
    ] {
        if glyphs.audit_text(&invalid).is_ok() {
            return Err(format!("typed {name} escaped glyph audit bounds"));
        }
        if glyphs.audit_text("á")? != baseline {
            return Err(format!("rejected {name} changed accepted audit"));
        }
    }
    Ok(())
}

#[test]
fn typed_glyph_inspection_refuses_invalid_unselected_inventory()
-> Result<(), String> {
    let mut glyphs = GlyphOverlay::default();
    for grapheme in ["a", "b"] {
        glyphs.parse("--glyph-stroke", &mut args(&[
            grapheme, "600", "500", "1800", "3000", "250", "#123456",
        ]).iter())?;
    }
    let paths = glyphs.inspect_paths("a")?;
    let order = glyphs.inspect_stroke_order("a")?;
    let original = glyphs.strokes[1].clone();
    let mut collapsed = original.clone();
    collapsed.end_um = collapsed.start_um;
    let mut invalid_key = original.clone();
    invalid_key.grapheme = "bc".into();
    let mut clipped = original.clone();
    clipped.end_um.1 = 4501;
    for (name, invalid) in [
        ("collapsed unused stroke", collapsed),
        ("invalid unused key", invalid_key),
        ("clipped unused stroke", clipped),
    ] {
        glyphs.strokes[1] = invalid;
        if glyphs.inspect_paths("a").is_ok()
            || glyphs.inspect_stroke_order("a").is_ok() {
            return Err(format!("inspection hid {name}"));
        }
        glyphs.strokes[1] = original.clone();
        if glyphs.inspect_paths("a")? != paths
            || glyphs.inspect_stroke_order("a")? != order {
            return Err(format!("rejected {name} changed inspection replay"));
        }
    }
    glyphs.strokes.resize(4097, original);
    if glyphs.inspect_paths("a").is_ok()
        || glyphs.inspect_stroke_order("a").is_ok() {
        return Err("inspection accepted over-budget typed library".into());
    }
    glyphs.strokes.truncate(2);
    if glyphs.inspect_paths("a")? != paths
        || glyphs.inspect_stroke_order("a")? != order {
        return Err("over-budget refusal changed valid inspection".into());
    }
    Ok(())
}

#[test]
fn typed_glyph_mutations_refuse_invalid_unselected_inventory()
-> Result<(), String> {
    let mut glyphs = GlyphOverlay::default();
    for (grapheme, x1, y1, x2, y2) in [
        ("a", "600", "500", "1000", "1000"),
        ("a", "1200", "1000", "1800", "3000"),
        ("b", "600", "500", "1800", "3000"),
    ] {
        glyphs.parse("--glyph-stroke", &mut args(&[
            grapheme, x1, y1, x2, y2, "250", "#123456",
        ]).iter())?;
    }
    let accepted = glyphs.encode_file()?;
    let original = glyphs.strokes[2].clone();
    // Each operation is valid against the original library. None may
    // mutate an unrelated invalid typed stroke or create further edits
    // before the complete authored inventory has been admitted.
    let mutations: [(&str, fn(&mut GlyphOverlay) -> Result<(), String>); 10] = [
        ("copy glyph", |overlay| overlay.copy_glyph("a", "c")),
        ("copy path", |overlay| overlay.copy_path("a", 1, "c")),
        ("remove glyph", |overlay| overlay.remove_glyph("a")),
        ("remove stroke", |overlay| overlay.remove_stroke("a", 1)),
        ("reverse stroke", |overlay| overlay.reverse_stroke("a", 1)),
        ("move stroke", |overlay| overlay.move_stroke("a", 1, 2)),
        ("join strokes", |overlay| overlay.join_strokes("a", 1)),
        ("replace stroke", |overlay| {
            let replacement = overlay.strokes[0].clone();
            overlay.set_stroke("a", 1, replacement)
        }),
        ("scale glyph", |overlay| overlay.scale_glyph_ink("a", 100)),
        ("translate glyph", |overlay| {
            overlay.translate_glyph("a", 10, 0)
        }),
    ];
    let mut collapsed = original.clone();
    collapsed.end_um = collapsed.start_um;
    let mut invalid_key = original.clone();
    invalid_key.grapheme = "ab".into();
    let mut outside = original.clone();
    outside.end_um.1 = 4501;
    let mut invalid_width = original.clone();
    invalid_width.width_um = 1501;
    for (case, invalid) in [
        ("collapsed unused stroke", collapsed),
        ("invalid unused key", invalid_key),
        ("unused stroke outside cell", outside),
        ("invalid unused pen width", invalid_width),
    ] {
        glyphs.strokes[2] = invalid;
        let rejected_state = format!("{:?}", glyphs.strokes);
        for (name, edit) in mutations {
            if edit(&mut glyphs).is_ok() {
                return Err(format!("{name} accepted {case}"));
            }
            if format!("{:?}", glyphs.strokes) != rejected_state {
                return Err(format!("{name} mutated {case}"));
            }
        }
    }
    glyphs.strokes[2] = original.clone();
    glyphs.strokes.resize(4097, original);
    let rejected_state = format!("{:?}", glyphs.strokes);
    for (name, edit) in mutations {
        if edit(&mut glyphs).is_ok() {
            return Err(format!("{name} accepted over-budget typed library"));
        }
        if format!("{:?}", glyphs.strokes) != rejected_state {
            return Err(format!("{name} mutated over-budget typed library"));
        }
    }
    glyphs.strokes.truncate(3);
    if glyphs.encode_file()? != accepted {
        return Err("rejected edits changed valid library replay".into());
    }
    for (name, edit) in mutations {
        let mut valid = GlyphOverlay {
            strokes: glyphs.strokes.clone(), ..GlyphOverlay::default()
        };
        if let Err(error) = edit(&mut valid) {
            return Err(format!("valid {name} refused: {error}"));
        }
        let _encoded = valid.encode_file()?;
    }
    Ok(())
}

#[test]
fn typed_glyph_append_refuses_invalid_existing_library()
-> Result<(), String> {
    let mut glyphs = GlyphOverlay::default();
    glyphs.parse("--glyph-stroke", &mut args(&[
        "á", "600", "500", "1800", "3000", "250", "#123456",
    ]).iter())?;
    let baseline = glyphs.encode_file()?;
    let original = glyphs.strokes[0].clone();
    let mut collapsed = original.clone();
    collapsed.end_um = collapsed.start_um;
    glyphs.strokes[0] = collapsed;
    if glyphs.parse("--glyph-stroke", &mut args(&[
        "ñ", "600", "500", "1800", "3000", "250", "#123456",
    ]).iter()).is_ok() {
        return Err("typed glyph append accepted invalid existing ink".into());
    }
    if glyphs.strokes.len() != 1 {
        return Err("rejected typed append mutated existing inventory".into());
    }
    glyphs.strokes[0] = original;
    if glyphs.encode_file()? != baseline {
        return Err("rejected append changed valid stroke replay".into());
    }
    glyphs.parse("--glyph-stroke", &mut args(&[
        "ñ", "600", "500", "1800", "3000", "250", "#123456",
    ]).iter())?;
    if glyphs.strokes.len() != 2 {
        return Err("valid second glyph append was refused".into());
    }
    Ok(())
}

#[test]
fn typed_glyph_audit_rejects_unrenderable_inventory()
-> Result<(), String> {
    let mut glyphs = GlyphOverlay::default();
    glyphs.parse("--glyph-stroke", &mut args(&[
        "á", "600", "500", "1800", "3000", "250", "#123456",
    ]).iter())?;
    let original = glyphs.strokes[0].clone();
    let baseline = glyphs.audit_text("á")?;
    let mut collapsed = original.clone();
    collapsed.end_um = collapsed.start_um;
    let mut invalid_key = original.clone();
    invalid_key.grapheme = "ab".into();
    let mut outside = original.clone();
    outside.start_um.0 = 3501;
    for (name, invalid) in [
        ("collapsed shape", collapsed),
        ("invalid key", invalid_key),
        ("out-of-cell shape", outside),
    ] {
        glyphs.strokes[0] = invalid;
        if glyphs.audit_text("á").is_ok() {
            return Err(format!("audit accepted {name} as coverage"));
        }
        glyphs.strokes[0] = original.clone();
        if glyphs.audit_text("á")? != baseline {
            return Err(format!("rejected {name} changed valid audit"));
        }
    }
    glyphs.strokes.resize(4097, original);
    if glyphs.audit_text("á").is_ok() {
        return Err("audit accepted over-budget stroke inventory".into());
    }
    glyphs.strokes.truncate(1);
    if glyphs.audit_text("á")? != baseline {
        return Err("over-budget audit mutated valid inventory".into());
    }
    Ok(())
}

#[test]
fn typed_glyph_expansion_limits_positioned_runs()
-> Result<(), String> {
    let mut glyphs = GlyphOverlay::default();
    glyphs.parse("--glyph-stroke", &mut args(&[
        "a", "600", "500", "1800", "3000", "250", "#123456",
    ]).iter())?;
    glyphs.parse("--place-text", &mut args(&[
        "1", "25000", "30000", "a",
    ]).iter())?;
    let valid = glyphs.runs[0].clone();
    glyphs.runs.resize(16, valid.clone());
    let at_limit = glyphs.expand(&[], None)?;
    if at_limit.len() != 16 {
        return Err("16 typed runs must render exact authored strokes".into());
    }
    glyphs.runs.push(valid);
    let Err(error) = glyphs.expand(&[], None) else {
        return Err("typed 17th text run bypassed CLI admission".into());
    };
    if !error.contains("16") {
        return Err(format!("unexpected positioned run refusal: {error}"));
    }
    let (with_flow, boundary) = glyphs.expand_with_run_boundary(
        &[], None, 1,
    )?;
    if with_flow.len() != 17 || boundary != 1 {
        return Err(
            "flow prefix plus 16 explicit runs must remain valid".into(),
        );
    }
    let Err(error) = glyphs.expand_with_run_boundary(&[], None, 18) else {
        return Err("typed out-of-range flow boundary was accepted".into());
    };
    if !error.contains("flow run boundary") {
        return Err(format!("unexpected flow boundary refusal: {error}"));
    }
    let _removed = glyphs.runs.pop().ok_or("missing rejected run")?;
    if glyphs.expand(&[], None)? != at_limit {
        return Err("rejected run changed accepted physical ink replay".into());
    }
    Ok(())
}

#[test]
fn typed_glyph_expansion_limits_selected_text_join_pairs()
-> Result<(), String> {
    let mut glyphs = GlyphOverlay::default();
    let letters = "abcdefghijklmnopqrstuvwxyzABCDEFGH";
    let keys = letters.chars().map(|ch| ch.to_string())
        .collect::<Vec<_>>();
    for key in &keys {
        glyphs.parse("--glyph-stroke", &mut args(&[
            key, "125", "2250", "3375", "2250", "250", "#123456",
        ]).iter())?;
    }
    glyphs.parse("--place-text", &mut args(&[
        "1", "25000", "30000", letters,
    ]).iter())?;
    for pair in keys.windows(2).take(32) {
        glyphs.add_text_join_pair(&mut args(&[
            &pair[0], &pair[1],
        ]).iter())?;
    }
    let baseline = glyphs.expand(&[], None)?;
    if baseline.len() != 66 {
        return Err("expected 34 glyphs and 32 explicit bridges".into());
    }
    glyphs.text_joins.push(TextJoin {
        left: keys[32].clone(), right: keys[33].clone(), anchors: None,
    });
    let Err(error) = glyphs.expand(&[], None) else {
        return Err("typed 33rd text join bypassed CLI limit".into());
    };
    if !error.contains("32") {
        return Err(format!("unexpected join limit refusal: {error}"));
    }
    let _removed = glyphs.text_joins.pop()
        .ok_or("missing rejected join")?;
    if glyphs.expand(&[], None)? != baseline {
        return Err("rejected join changed accepted physical ink".into());
    }
    Ok(())
}

#[test]
fn typed_glyph_expansion_refuses_unselected_invalid_ink()
-> Result<(), String> {
    let mut glyphs = GlyphOverlay::default();
    for glyph in ["a", "b"] {
        glyphs.parse("--glyph-stroke", &mut args(&[
            glyph, "600", "500", "1800", "3000", "250", "#123456",
        ]).iter())?;
    }
    glyphs.parse("--place-text", &mut args(&[
        "1", "25000", "30000", "a",
    ]).iter())?;
    let baseline = glyphs.expand(&[], None)?;
    let original = glyphs.strokes[1].clone();
    let mut hidden = original.clone();
    hidden.grapheme = "\u{00ad}".into();
    let mut collapsed = original.clone();
    collapsed.end_um = collapsed.start_um;
    let mut clipped = original.clone();
    clipped.end_um.0 = 3501;
    let mut invalid_width = original.clone();
    invalid_width.width_um = 1501;
    for (case, invalid) in [
        ("non-ink glyph key", hidden),
        ("collapsed unused segment", collapsed),
        ("unused segment beyond cell", clipped),
        ("unused pen outside bounds", invalid_width),
    ] {
        glyphs.strokes[1] = invalid;
        if glyphs.expand(&[], None).is_ok() {
            return Err(format!("render ignored {case}"));
        }
        glyphs.strokes[1] = original.clone();
        if glyphs.expand(&[], None)? != baseline {
            return Err(format!("{case} altered accepted replay"));
        }
    }
    glyphs.strokes.resize(4097, original);
    if glyphs.expand(&[], None).is_ok() {
        return Err("render accepted an oversized typed glyph library".into());
    }
    glyphs.strokes.truncate(2);
    if glyphs.expand(&[], None)? != baseline {
        return Err("oversized library altered accepted replay".into());
    }
    Ok(())
}

#[test]
fn typed_glyph_library_never_serializes_unreloadable_strokes()
-> Result<(), String> {
    let mut glyphs = GlyphOverlay::default();
    glyphs.parse("--glyph-stroke", &mut args(&[
        "ñ", "600", "500", "1800", "3000", "250", "#123456",
    ]).iter())?;
    let accepted = glyphs.encode_file()?;
    let original = glyphs.strokes.first().cloned()
        .ok_or("missing valid original stroke")?;
    let mut invalid_key = original.clone();
    invalid_key.grapheme = "ab".into();
    let mut zero_length = original.clone();
    zero_length.end_um = zero_length.start_um;
    let mut out_of_cell = original.clone();
    out_of_cell.start_um.1 = 4501;
    let mut oversized_pen = original.clone();
    oversized_pen.width_um = 1501;
    for (label, invalid) in [
        ("key", invalid_key),
        ("zero length", zero_length),
        ("outside cell", out_of_cell),
        ("pen width", oversized_pen),
    ] {
        glyphs.strokes[0] = invalid;
        if glyphs.encode_file().is_ok() {
            return Err(format!("invalid typed {label} serialized"));
        }
        glyphs.strokes[0] = original.clone();
        if glyphs.encode_file()? != accepted {
            return Err(format!("invalid {label} changed canonical replay"));
        }
    }
    glyphs.strokes.resize(4097, original);
    if glyphs.encode_file().is_ok() {
        return Err("typed library over 4096 strokes serialized".into());
    }
    glyphs.strokes.truncate(1);
    if glyphs.encode_file()? != accepted {
        return Err("over-budget library changed canonical replay".into());
    }
    Ok(())
}

#[test]
fn typed_stroke_replacement_refuses_invalid_geometry_atomically()
-> Result<(), String> {
    let mut glyphs = GlyphOverlay::default();
    glyphs.parse("--glyph-stroke", &mut args(&[
        "a", "600", "500", "1800", "3000", "250", "#123456",
    ]).iter())?;
    let accepted = glyphs.encode_file()?;
    let original = glyphs.strokes.first().cloned()
        .ok_or("missing accepted typed shape")?;
    let mut collapsed = original.clone();
    collapsed.end_um = collapsed.start_um;
    let mut outside = original.clone();
    outside.end_um.0 = 3501;
    let mut too_thin = original.clone();
    too_thin.width_um = 79;
    let mut too_thick = original;
    too_thick.width_um = 1501;
    for (description, replacement) in [
        ("collapsed", collapsed),
        ("outside cell", outside),
        ("too thin", too_thin),
        ("too thick", too_thick),
    ] {
        if glyphs.set_stroke("a", 1, replacement).is_ok() {
            return Err(format!("{description} typed stroke was accepted"));
        }
        if glyphs.encode_file()? != accepted {
            return Err(format!("{description} poisoned the canonical library"));
        }
    }
    Ok(())
}

#[test]
fn typed_manual_ink_matches_cli_line_admission() -> Result<(), String> {
    let glyphs = GlyphOverlay::default();
    let accepted = NotebookInkLine {
        page: 1,
        start_um: (25_000, 30_000),
        end_um: (27_000, 32_000),
        width_um: 250,
        rgb: [0x12, 0x34, 0x56],
    };
    let baseline = glyphs.expand(&[accepted], None)?;
    if baseline != [accepted] {
        return Err("valid typed manual ink did not replay exactly".into());
    }
    let mut zero_page = accepted;
    zero_page.page = 0;
    let mut extra_page = accepted;
    extra_page.page = 33;
    let mut outside = accepted;
    outside.end_um.0 = 700_001;
    let mut too_close = accepted;
    too_close.start_um.1 = 124;
    let mut collapsed = accepted;
    collapsed.end_um = collapsed.start_um;
    let mut thin = accepted;
    thin.width_um = 79;
    let mut thick = accepted;
    thick.width_um = 1501;
    for (name, invalid) in [
        ("zero page", zero_page),
        ("page above 32", extra_page),
        ("out-of-range coordinate", outside),
        ("pen crossing page origin", too_close),
        ("zero-length stroke", collapsed),
        ("pen below 80 um", thin),
        ("pen above 1500 um", thick),
    ] {
        if glyphs.expand(&[invalid], None).is_ok() {
            return Err(format!("typed manual {name} bypassed CLI admission"));
        }
        if glyphs.expand(&[accepted], None)? != baseline {
            return Err(format!("refused {name} changed accepted ink replay"));
        }
    }
    Ok(())
}

#[test]
fn typed_glyph_expansion_refuses_over_budget_manual_ink()
-> Result<(), String> {
    let mut glyphs = GlyphOverlay::default();
    let valid = NotebookInkLine {
        page: 1,
        start_um: (25_000, 30_000),
        end_um: (27_000, 32_000),
        width_um: 250,
        rgb: [0x12, 0x34, 0x56],
    };
    let within_limit = vec![valid; MAX_NOTEBOOK_INK_LINES];
    if glyphs.expand(&within_limit, None)? != within_limit {
        return Err("boundary-sized manual ink was changed".into());
    }
    let mut over_limit = within_limit.clone();
    over_limit.push(valid);
    let Err(error) = glyphs.expand(&over_limit, None) else {
        return Err("over-budget typed manual ink was admitted".into());
    };
    if !error.contains("4096") {
        return Err(format!("unexpected ink budget refusal: {error}"));
    }
    glyphs.parse("--glyph-stroke", &mut args(&[
        "a", "600", "500", "1800", "3000", "250", "#123456",
    ]).iter())?;
    glyphs.parse("--place-text", &mut args(&[
        "1", "25000", "30000", "a",
    ]).iter())?;
    let Err(error) = glyphs.expand(&within_limit, None) else {
        return Err("authored glyph overflowed typed manual ink budget".into());
    };
    if !error.contains("4096") {
        return Err(format!("unexpected glyph budget refusal: {error}"));
    }
    Ok(())
}

#[test]
fn typed_glyph_expansion_refuses_unvalidated_statistics()
-> Result<(), String> {
    use atrament_handwriting_statistics::{
        Dimension, GlyphStatistics, HandwritingStatisticsProfile,
    };

    const OUTSIDE_BOUNDS: i32 = 1_000_000;
    let mut overlay = GlyphOverlay::default();
    overlay.parse("--glyph-stroke", &mut args(&[
        "a", "600", "500", "1800", "3000", "250", "#123456",
    ]).iter())?;
    overlay.parse("--place-text", &mut args(&[
        "1", "25000", "30000", "a",
    ]).iter())?;
    let baseline = HandwritingStatisticsProfile::baseline(42);
    let admitted = overlay.expand(&[], Some(&baseline))?;
    if admitted.len() != 1 {
        return Err("valid typed expansion lost authored pen ink".into());
    }

    let mut stale = baseline.clone();
    stale.version = "unsupported-statistics-version".into();
    let inherited = baseline.globals.first().copied()
        .ok_or("missing baseline statistics")?;
    let mut duplicate = baseline.clone();
    duplicate.globals.push(inherited);
    let mut outside = baseline.clone();
    let width = outside.globals.iter_mut()
        .find(|entry| entry.dimension == Dimension::Width)
        .ok_or("missing baseline width")?;
    width.statistics.minimum = OUTSIDE_BOUNDS;
    width.statistics.maximum = OUTSIDE_BOUNDS;
    width.statistics.mean = OUTSIDE_BOUNDS;
    width.statistics.median = OUTSIDE_BOUNDS;
    let mut malformed_key = baseline;
    malformed_key.glyphs.push(GlyphStatistics {
        grapheme: "ab".into(),
        parameters: vec![inherited],
    });
    for (label, invalid, expected) in [
        ("version", stale, "UnsupportedVersion"),
        ("duplicate", duplicate, "DuplicateGlobal"),
        ("width", outside, "OutsideDimensionBounds"),
        ("grapheme", malformed_key, "InvalidGrapheme"),
    ] {
        let Err(error) = overlay.expand(&[], Some(&invalid)) else {
            return Err(format!("unvalidated {label} profile rendered ink"));
        };
        if !error.contains("invalid glyph handwriting statistics")
            || !error.contains(expected) {
            return Err(format!("unexpected {label} refusal: {error}"));
        }
    }
    if overlay.expand(&[], Some(&HandwritingStatisticsProfile::baseline(42)))?
        != admitted {
        return Err("invalid typed expansion changed replay".into());
    }
    Ok(())
}

#[test]
fn typed_fallbacks_require_unique_exact_graphemes()
-> Result<(), String> {
    let mut overlay = GlyphOverlay::default();
    overlay.parse("--glyph-stroke", &mut args(&[
        "a", "600", "500", "1800", "3000", "250", "#123456",
    ]).iter())?;
    overlay.parse("--fallback-glyph", &mut args(&[
        "\u{f3}", "a",
    ]).iter())?;
    overlay.parse("--place-text", &mut args(&[
        "1", "25000", "30000", "\u{f3}",
    ]).iter())?;
    let accepted = overlay.expand(&[], None)?;
    if accepted.len() != 1 {
        return Err("valid exact fallback lost authored ink".into());
    }
    for (missing, expected) in [
        ("ab", "fallback requires two distinct exact visible graphemes"),
        ("\u{f3}", "duplicate exact glyph fallback"),
    ] {
        overlay.fallbacks.push(GlyphFallback {
            authored: "a".into(), missing: missing.into(),
        });
        let outcome = overlay.expand(&[], None);
        drop(overlay.fallbacks.pop());
        let Err(error) = outcome else {
            return Err(format!("invalid typed fallback {missing:?} rendered"));
        };
        if error != expected {
            return Err(format!("unexpected fallback refusal: {error}"));
        }
    }
    overlay.fallbacks.resize_with(65, || GlyphFallback {
        authored: "a".into(), missing: "\u{f3}".into(),
    });
    let Err(error) = overlay.expand(&[], None) else {
        return Err("typed fallback count bypassed native limit".into());
    };
    if error != "more than 64 glyph fallbacks refused" {
        return Err(format!("unexpected fallback count refusal: {error}"));
    }
    overlay.fallbacks.truncate(1);
    if overlay.expand(&[], None)? != accepted {
        return Err("invalid fallback changed valid replay".into());
    }
    Ok(())
}

#[test]
fn double_underlined_headings_keep_tiered_authored_text_and_native_ink() {
    let markdown = "# a\n## a\n### a\n> a\n---\na";
    let source = FlowSource::new(FlowFormat::Markdown, markdown)
        .expect("tiered headings and physical decorations");
    let mut decoration = HeadingDecoration::default();
    decoration.parse(
        "--heading-style", &mut args(&["double-underline"]).iter(),
    ).expect("explicit double underline");
    decoration.parse(
        "--heading-color", &mut args(&["#64A3C5"]).iter(),
    ).expect("explicit custom title ink");
    let (runs, ornaments, pages) = layout_flow(
        &source, &NotebookProfile::default(), None, decoration,
    ).expect("physical heading hierarchy with real ornaments");
    assert_eq!(pages, 1);
    assert_eq!(runs.len(), 5);
    assert!(runs.iter().all(|run| run.text == "a"));
    assert_eq!(ornaments.len(), 8,
        "two lines per heading, quote bar, one separator");
    for (index, width) in [550, 420, 300].iter().enumerate() {
        let heading = &runs[index];
        for (line, rise) in [5_800, 6_700].iter().enumerate() {
            let ornament = &ornaments[index * 2 + line];
            assert_eq!(ornament.page, 1);
            assert_eq!(ornament.start_um,
                (heading.origin_um.0, heading.origin_um.1 + rise));
            assert_eq!(ornament.end_um,
                (heading.origin_um.0 + 4_000,
                    heading.origin_um.1 + rise));
            assert_eq!(ornament.width_um, *width);
            assert_eq!(ornament.rgb, [0x64, 0xA3, 0xC5]);
        }
    }
    assert_eq!(ornaments[6].rgb, [30, 48, 74],
        "quotations keep their own rule color");
    assert_eq!(ornaments[7].rgb, [30, 48, 74],
        "thematic rules remain independent from title tint");

    for format in ["pdf", "png"] {
        let mut base = args(&[
            "--export-paper", format, "--printed-sheet", "true",
            "--glyph-stroke", "a", "0", "0", "3500", "4500",
            "250", "#204060", "--flow-markdown", markdown,
        ]);
        if format == "png" {
            base.extend(args(&["--dpi", "72"]));
        }
        let default = execute_notebook(&base)
            .expect("original single underline").expect("image bytes");
        base.extend(args(&[
            "--heading-style", "double-underline",
            "--heading-color", "#64A3C5",
        ]));
        let decorated = execute_notebook(&base)
            .expect("double underline native export").expect("image bytes");
        assert_ne!(decorated, default);
        assert_eq!(decorated, execute_notebook(&base)
            .expect("deterministic layout").expect("same image bytes"));
        if format == "pdf" {
            assert!(decorated.starts_with(b"%PDF-1.4"));
            assert_eq!(String::from_utf8_lossy(&decorated)
                .matches("0.392 0.639 0.773 RG").count(), 6,
                "two colored strokes for each heading level");
        } else {
            assert!(decorated.starts_with(b"\x89PNG\r\n\x1a\n"));
        }
    }
}

#[test]
fn double_underline_and_following_text_stay_on_page_two() {
    let markdown = format!("{}# a\na", "a  \n".repeat(27));
    let source = FlowSource::new(FlowFormat::Markdown, &markdown)
        .expect("heading after last safe row");
    let mut decoration = HeadingDecoration::default();
    decoration.parse(
        "--heading-style", &mut args(&["double-underline"]).iter(),
    ).expect("double underline style");
    let (runs, ornaments, pages) = layout_flow(
        &source, &NotebookProfile::default(), None, decoration,
    ).expect("keep heading and follower together");
    assert_eq!(pages, 2);
    assert_eq!(runs.len(), 29);
    assert_eq!(runs[27].page, 2);
    assert_eq!(runs[28].page, 2);
    assert_eq!(ornaments.len(), 2);
    assert!(ornaments.iter().all(|line| line.page == 2));
    assert_eq!(ornaments[0].start_um, (22_500, 30_300));
    assert_eq!(ornaments[1].start_um, (22_500, 31_200));
    let mut cli = args(&[
        "--export-paper", "pdf", "--printed-sheet", "true",
        "--glyph-stroke", "a", "0", "0", "3500", "4500",
        "250", "#204060", "--flow-markdown", &markdown,
        "--heading-style", "double-underline",
    ]);
    let pdf = execute_notebook(&cli)
        .expect("PDF with second-page double ornament")
        .expect("PDF bytes");
    let streams = String::from_utf8_lossy(&pdf)
        .split("\nstream\n").skip(1).map(|part| {
            part.split("\nendstream").next().unwrap_or("").to_owned()
        }).collect::<Vec<_>>();
    assert_eq!(streams.len(), 2);
    assert_eq!(streams[0].matches("0.118 0.188 0.290 RG").count(), 0);
    assert_eq!(streams[1].matches("0.118 0.188 0.290 RG").count(), 2);
    cli[1] = "png".into();
    cli.extend(args(&["--dpi", "72", "--png-page", "2"]));
    let png = execute_notebook(&cli)
        .expect("page two raster").expect("PNG bytes");
    assert!(png.starts_with(b"\x89PNG\r\n\x1a\n"));
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
fn markdown_heading_keeps_next_visible_row_on_its_physical_page() {
    let preceding = "a  \n".repeat(27);
    for (heading, following, gap) in [
        ("# a", "\na", 0u32),
        ("## a", "\n\n\na", 4_000),
        ("### a", "\n\n\n\na", 4_000),
    ] {
        let markdown = format!("{preceding}{heading}{following}");
        let source = FlowSource::new(FlowFormat::Markdown, &markdown)
            .expect("bounded heading at final available row");
        let (runs, ornaments, pages) = layout_flow(
            &source, &NotebookProfile::default(), None,
            HeadingDecoration::default(),
        ).expect("heading and body fitted on one page");
        assert_eq!(pages, 2);
        assert_eq!(runs.len(), 29);
        assert_eq!(runs[26].page, 1);
        assert_eq!(runs[27].page, 2, "heading moves off orphan row");
        assert_eq!(runs[28].page, 2, "first body row follows heading");
        assert_eq!(runs[27].origin_um, (22_500, 24_500));
        assert_eq!(runs[28].origin_um.1 - runs[27].origin_um.1,
            9_000 + gap);
        assert_eq!(ornaments.len(), 1);
        assert_eq!(ornaments[0].page, 2,
            "native underline stays with its heading");
        let mut cli = args(&[
            "--printed-sheet", "true", "--export-paper", "pdf",
            "--glyph-stroke", "a", "0", "0", "3500", "4500",
            "250", "#204060", "--flow-markdown", &markdown,
        ]);
        let pdf = execute_notebook(&cli).expect("real two-page PDF")
            .expect("PDF bytes");
        let content = String::from_utf8_lossy(&pdf);
        let streams = content.split("\nstream\n").skip(1)
            .map(|part| part.split("\nendstream").next().unwrap_or(""))
            .collect::<Vec<_>>();
        assert_eq!(streams.len(), 2);
        assert!(!streams[0].contains("0.118 0.188 0.290 RG"),
            "no heading underline on previous page");
        assert!(streams[1].contains("0.118 0.188 0.290 RG"),
            "heading underline is native page-two ink");
        cli[3] = "png".into();
        cli.extend(args(&["--png-page", "2", "--dpi", "72"]));
        let png = execute_notebook(&cli).expect("page-two PNG")
            .expect("PNG bytes");
        assert!(png.starts_with(b"\x89PNG\r\n\x1a\n"));
    }
    let ending = format!("{preceding}# a");
    let end_source = FlowSource::new(FlowFormat::Markdown, &ending)
        .expect("a final heading is allowed");
    let (runs, ornaments, pages) = layout_flow(
        &end_source, &NotebookProfile::default(), None,
        HeadingDecoration::default(),
    ).expect("no invisible following row reserved");
    assert_eq!(pages, 1);
    assert_eq!(runs[27].page, 1);
    assert_eq!(ornaments[0].page, 1);
}

#[test]
fn heading_keep_with_next_degrades_on_one_row_printable_sheet() {
    let mut profile = NotebookProfile::default();
    profile.paper_json = profile.paper_json.replace(
        "\"margin_bottom_um\": 16000", "\"margin_bottom_um\": 258000",
    );
    assert!(profile.printable_rect_um().is_ok());
    let source = FlowSource::new(FlowFormat::Markdown, "# a\na")
        .expect("heading and follower");
    let (runs, ornaments, pages) = layout_flow(
        &source, &profile, None, HeadingDecoration::default(),
    ).expect("single-row sheet supports a heading and following page");
    assert_eq!(pages, 2);
    assert_eq!((runs[0].page, runs[1].page), (1, 2));
    assert_eq!(ornaments[0].page, 1);
}

#[test]
fn markdown_heading_levels_have_distinct_native_ink_weights() {
    let source = FlowSource::new(
        FlowFormat::Markdown, "# a\n## a\n### a\n- a",
    ).expect("three Markdown levels and list item");
    let profile = NotebookProfile::default();
    let (runs, underline, pages) = layout_flow(
        &source, &profile, None, HeadingDecoration::default(),
    ).expect("level-aware underline ink");
    assert_eq!(pages, 1);
    assert_eq!(runs.iter().map(|run| run.text.as_str())
        .collect::<Vec<_>>(), ["a", "a", "a", "a"]);
    assert_eq!(underline.len(), 4, "three headings and one bullet");
    assert_eq!(underline.iter().map(|line| line.width_um)
        .collect::<Vec<_>>(), [550, 420, 300, 450]);
    assert!(underline.iter().take(3).all(|line| line.rgb == [30, 48, 74]));

    let mut marker = HeadingDecoration::default();
    marker.parse("--heading-style", &mut args(&["marker"]).iter())
        .expect("explicit marker style");
    marker.parse("--heading-color", &mut args(&["#A1B2C3"]).iter())
        .expect("exact marker pigment");
    let (marked_runs, marks, _) = layout_flow(
        &source, &profile, None, marker,
    ).expect("three distinct native markers");
    assert_eq!(marked_runs.iter().map(|run| run.text.as_str())
        .collect::<Vec<_>>(), ["a", "a", "a", "a"],
        "heading decoration does not modify authored glyphs");
    assert_eq!(marks.iter().map(|line| line.width_um)
        .collect::<Vec<_>>(),
        [1_400, 1_400, 1_400, 1_400,
         1_150, 1_150, 1_150, 950, 950, 450]);
    for (level, (start, end, offsets)) in [
        (0usize, (0usize, 4usize, &[900, 1900, 2900, 3900][..])),
        (1, (4, 7, &[1000, 2000, 3000][..])),
        (2, (7, 9, &[1400, 2400][..])),
    ] {
        for (ink, offset) in marks[start..end].iter().zip(offsets) {
            assert_eq!(ink.page, 1);
            assert_eq!(ink.start_um.0, marked_runs[level].origin_um.0);
            assert_eq!(ink.start_um.1,
                marked_runs[level].origin_um.1 + offset);
            assert_eq!(ink.rgb, [0xA1, 0xB2, 0xC3]);
        }
    }
    assert_eq!(marks.last().expect("bullet remains").rgb, [30, 48, 74]);

    let mut none = HeadingDecoration::default();
    none.parse("--heading-style", &mut args(&["none"]).iter())
        .expect("no heading ornament");
    let (_, bare, _) = layout_flow(&source, &profile, None, none)
        .expect("headings remain valid without decoration");
    assert_eq!(bare.len(), 1, "none removes only heading ink");

    let mut cli = args(&[
        "--printed-sheet", "true", "--export-paper", "pdf",
        "--glyph-stroke", "a", "900", "1200", "2400", "3200",
        "250", "#123456", "--flow-markdown", "# a\n## a\n### a",
        "--heading-style", "marker", "--heading-color", "#A1B2C3",
    ]);
    let pdf = execute_notebook(&cli).expect("level markers in PDF")
        .expect("PDF bytes");
    assert!(pdf.starts_with(b"%PDF-1.4"));
    assert_eq!(String::from_utf8_lossy(&pdf)
        .matches("0.631 0.698 0.765 RG").count(), 9,
        "PDF paints nine level-dependent marker lines");
    cli[3] = "png".into();
    cli.extend(args(&["--dpi", "72"]));
    let png = execute_notebook(&cli).expect("level markers in PNG")
        .expect("transparent PNG bytes");
    assert!(png.starts_with(b"\x89PNG\r\n\x1a\n"));
    assert_eq!(png, execute_notebook(&cli)
        .expect("same level-aware raster").expect("PNG bytes"));
    let mut undecorated = cli.clone();
    undecorated.truncate(undecorated.len() - 6);
    assert_ne!(png, execute_notebook(&undecorated)
        .expect("ordinary headings").expect("PNG bytes"),
        "actual raster content must change with selected heading hierarchy");
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
fn heading_lettering_styles_only_markdown_heading_glyph_ink() {
    let source = FlowSource::new(
        FlowFormat::Markdown, "# a\n\na\n## a\n- a\n### a",
    ).expect("Markdown heading and body with a list");
    let mut heading = HeadingDecoration::default();
    heading.parse(
        "--heading-ink-color", &mut args(&["#BD376A"]).iter(),
    ).expect("explicit title glyph pigment");
    heading.parse(
        "--heading-ink-scale", &mut args(&["180"]).iter(),
    ).expect("explicit 180 percent heading pen weight");
    let (runs, ornaments, pages) = layout_flow(
        &source, &NotebookProfile::default(), None, heading,
    ).expect("measured physical heading ink");
    assert_eq!(pages, 1);
    assert_eq!(runs.iter().map(|run| run.heading_ink.is_some())
        .collect::<Vec<_>>(), [true, false, true, false, true]);
    assert_eq!(ornaments.len(), 4, "default underlines plus bullet");
    assert!(ornaments.iter().all(|line| line.rgb != [0xBD, 0x37, 0x6A]),
        "styling glyph ink does not recolor title ornaments");

    let mut authored = GlyphOverlay::default();
    authored.parse("--glyph-stroke", &mut args(&[
        "a", "900", "1200", "2400", "3200", "250", "#123456",
    ]).iter()).expect("one canonical authored lowercase glyph");
    let canonical = authored.strokes.clone();
    authored.runs = runs;
    let physical = authored.expand(&ornaments, None)
        .expect("same native PDF/PNG ink segment pipeline");
    let glyph_lines = &physical[ornaments.len()..];
    assert_eq!(glyph_lines.len(), 5);
    assert_eq!(glyph_lines.iter().map(|line| (line.width_um, line.rgb))
        .collect::<Vec<_>>(), [
        (450, [0xBD, 0x37, 0x6A]), (250, [0x12, 0x34, 0x56]),
        (450, [0xBD, 0x37, 0x6A]), (250, [0x12, 0x34, 0x56]),
        (450, [0xBD, 0x37, 0x6A]),
    ]);
    assert_eq!(authored.strokes.first().expect("canonical unchanged").rgb,
        canonical[0].rgb);
    assert_eq!(authored.strokes[0].width_um, 250);

    let command = args(&[
        "--printed-sheet", "true", "--export-paper", "pdf",
        "--glyph-stroke", "a", "900", "1200", "2400", "3200",
        "250", "#123456", "--flow-markdown",
        "# a\n\na\n## a\n- a\n### a",
        "--heading-ink-color", "#BD376A", "--heading-ink-scale", "180",
    ]);
    let pdf = execute_notebook(&command).expect("native styled PDF")
        .expect("real vector bytes");
    let vector = String::from_utf8_lossy(&pdf);
    assert_eq!(vector.matches("0.741 0.216 0.416 RG").count(), 3,
        "only the three title glyphs use chosen pigment");
    assert_eq!(vector.matches("0.071 0.204 0.337 RG").count(), 2,
        "body and bullet text remain author-colored");
    assert_eq!(vector.matches("0.450 w").count(), 0,
        "PDF physical widths use points, not micrometres");
    assert_eq!(pdf, execute_notebook(&command).expect("PDF replay")
        .expect("exact bytes"));
    let mut png_command = command.clone();
    png_command[3] = "png".into();
    png_command.extend(args(&["--dpi", "72"]));
    let png = execute_notebook(&png_command).expect("native PNG")
        .expect("RGBA bytes");
    assert!(png.starts_with(b"\x89PNG\r\n\x1a\n"));
    assert_eq!(png, execute_notebook(&png_command).expect("PNG replay")
        .expect("stable PNG bytes"));
    let original = command[..command.len() - 4].to_vec();
    assert_ne!(pdf, execute_notebook(&original).expect("plain heading ink")
        .expect("PDF bytes"));
    let mut plain_png = original;
    plain_png[3] = "png".into();
    plain_png.extend(args(&["--dpi", "72"]));
    assert_ne!(png, execute_notebook(&plain_png).expect("plain PNG")
        .expect("RGBA bytes"));
}

#[test]
fn heading_lettering_rejects_invalid_options_and_oversized_pen_atomically() {
    let target = test_path();
    let filename = target.to_str().expect("test-owned output name");
    fs::write(&target, b"keep-previous-styled-notebook")
        .expect("sentinel output");
    let base = args(&[
        "--printed-sheet", "true", "--export-paper", "pdf",
        "--glyph-stroke", "a", "900", "1200", "2400", "3200",
        "700", "#123456", "--flow-markdown", "# a\n\na",
        "--output", filename, "--replace",
    ]);
    let invalid = [
        args(&["--heading-ink-scale", "99"]),
        args(&["--heading-ink-scale", "251"]),
        args(&["--heading-ink-scale", "-1"]),
        args(&["--heading-ink-scale", "100.5"]),
        args(&["--heading-ink-scale", "4294967296"]),
        args(&["--heading-ink-scale", "150", "--heading-ink-scale", "200"]),
        args(&["--heading-ink-scale"]),
        args(&["--heading-ink-color", "#12345Z"]),
        args(&["--heading-ink-color", "#12345"]),
        args(&[
            "--heading-ink-color", "#abcdef",
            "--heading-ink-color", "#fedcba",
        ]),
        args(&["--heading-ink-color"]),
        args(&["--heading-ink-scale", "250"]),
    ];
    for options in invalid {
        let mut attempt = base.clone();
        attempt.extend(options);
        assert!(execute_notebook(&attempt).is_err(), "{attempt:?}");
        assert_eq!(fs::read(&target).expect("existing output preserved"),
            b"keep-previous-styled-notebook");
    }
    for flag in ["--heading-ink-color", "--heading-ink-scale"] {
        let argument = if flag.ends_with("color") { "#123456" } else { "150" };
        assert!(execute_notebook(&args(&[
            "--printed-sheet", "true", "--export-paper", "pdf",
            "--glyph-stroke", "a", "900", "1200", "2400", "3200",
            "250", "#123456", "--flow-text", "a",
            flag, argument,
        ])).is_err(), "heading styling requires Markdown flow");
    }
    fs::remove_file(target).expect("remove owned sentinel");
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
        args(&[
            "--heading-style", "double-underline",
            "--heading-style", "underline",
        ]),
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

#[test]
fn marker_headings_refuse_bounded_ink_overflow_atomically() {
    // With 49 caller-authored pen segments per letter, 80 normal
    // headings need 4000 ink segments including their underlines.
    // Four-band marker ink requires 4240 and must refuse atomically.
    let target = test_path();
    let filename = target.to_str().expect("test-owned PDF path");
    let markdown = "# a\n".repeat(80);
    let mut command = args(&[
        "--printed-sheet", "true", "--export-paper", "pdf",
        "--flow-markdown", &markdown,
    ]);
    for _ in 0..49 {
        command.extend(args(&[
            "--glyph-stroke", "a", "900", "1200", "2400", "3200",
            "250", "#123456",
        ]));
    }
    assert!(execute_notebook(&command).expect("original underline fits")
        .expect("native PDF bytes").starts_with(b"%PDF-1.4"));
    fs::write(&target, b"keep-previous-notebook")
        .expect("save owned sentinel");
    command.extend(args(&[
        "--heading-style", "marker", "--output", filename, "--replace",
    ]));
    let error = execute_notebook(&command).expect_err("marker ink budget");
    assert!(error.contains("4096 total pen strokes"), "{error}");
    assert_eq!(fs::read(&target).expect("unchanged saved file"),
        b"keep-previous-notebook");
    fs::remove_file(target).expect("remove test-owned sentinel");
}

#[test]
fn selected_flow_region_repeats_on_real_pdf_and_transparent_png_pages() {
    let region = FlowRegion {
        left: 30_000, top: 40_000, width: 50_000, height: 30_000,
    };
    let source = FlowSource::new(
        FlowFormat::Markdown, "# a\n- a\na  \na",
    ).expect("bounded heading, list and hard-broken paragraph");
    let (runs, marks, pages) = layout_flow_with_region(
        &source, &NotebookProfile::default(), None,
        HeadingDecoration::default(), Some(region),
    ).expect("selected two-row region repeats on each sheet");
    assert_eq!(pages, 2);
    assert_eq!(runs.iter().map(|run| (run.page, run.origin_um))
        .collect::<Vec<_>>(), [
            (1, (32_500, 42_500)), (1, (38_500, 51_500)),
            (2, (32_500, 42_500)), (2, (32_500, 51_500)),
        ]);
    assert_eq!(marks.len(), 2, "heading and bullet remain native ink");
    assert!(marks.iter().all(|mark| mark.page == 1
        && mark.start_um.0 >= region.left
        && mark.end_um.0 <= region.left + region.width
        && mark.start_um.1 >= region.top
        && mark.end_um.1 <= region.top + region.height));
    let mut cli = args(&[
        "--printed-sheet", "true", "--export-paper", "pdf",
        "--glyph-stroke", "a", "0", "0", "3500", "4500",
        "250", "#204060", "--flow-markdown", "# a\n- a\na  \na",
        "--flow-region", "30000", "40000", "50000", "30000",
    ]);
    let pdf = execute_notebook(&cli).expect("bounded real PDF")
        .expect("PDF bytes");
    assert!(pdf.starts_with(b"%PDF-1.4"));
    let streams = String::from_utf8_lossy(&pdf)
        .split("\nstream\n").skip(1)
        .map(|entry| entry.split("\nendstream").next().unwrap_or("")
            .to_owned()).collect::<Vec<_>>();
    assert_eq!(streams.len(), 2);
    assert!(streams.iter().all(|stream| stream.contains(" m\n")));
    assert_eq!(pdf, execute_notebook(&cli)
        .expect("deterministic PDF replay").expect("PDF bytes"));
    drop(cli.splice(2..4, args(&["--export-paper", "png", "--dpi", "72",
        "--png-page", "2"])));
    let second = execute_notebook(&cli).expect("page-two PNG")
        .expect("PNG bytes");
    assert!(second.starts_with(b"\x89PNG\r\n\x1a\n"));
    assert_eq!(second, execute_notebook(&cli)
        .expect("deterministic transparent page-two PNG")
        .expect("PNG bytes"));
    let selected = cli.iter_mut().find(|value| value.as_str() == "2")
        .expect("PNG page-two selector");
    *selected = "1".into();
    assert_ne!(second, execute_notebook(&cli)
        .expect("page-one PNG").expect("PNG bytes"));
}

#[test]
fn selected_flow_region_refuses_slanted_ink_outside_bounds_atomically() {
    let stats_path = test_path();
    let output = test_path().with_extension("pdf");
    let stats_name = stats_path.to_str().expect("statistics filename");
    let output_name = output.to_str().expect("output filename");
    let statistics = execute_handwriting(&args(&[
        "--set-global", "slant", "-85000", "85000", "50000",
        "50000", "0",
    ])).expect("bounded high-slant statistics")
        .expect("canonical statistics bytes");
    fs::write(&stats_path, statistics).expect("test-only statistics file");
    fs::write(&output, b"preserve bounded page")
        .expect("test-only original PDF");
    let mut command = args(&[
        "--printed-sheet", "true", "--export-paper", "pdf",
        "--glyph-stroke", "a", "3000", "1000", "3500", "500",
        "250", "#204060", "--handwriting-profile", stats_name,
        "--flow-markdown", "aaaaaaaa",
    ]);
    let unrestricted = execute_notebook(&command)
        .expect("same slanted ink remains within normal paper margins")
        .expect("genuine PDF");
    assert!(unrestricted.starts_with(b"%PDF-1.4"));
    command.extend(args(&[
        "--flow-region", "30000", "40000", "25000", "30000",
        "--output", output_name, "--replace",
    ]));
    let error = execute_notebook(&command)
        .expect_err("slanted glyph outside chosen flow rectangle must refuse");
    assert!(error.contains("flow region"), "{error}");
    assert_eq!(fs::read(&output).expect("original PDF preserved"),
        b"preserve bounded page");
    fs::remove_file(stats_path).expect("remove owned statistics file");
    fs::remove_file(output).expect("remove owned PDF");
}

#[test]
fn fixed_region_checks_unselected_png_pages_and_leaves_manual_ink_free() {
    let stats_path = test_path();
    let output = test_path().with_extension("png");
    let stats_name = stats_path.to_str().expect("statistics path");
    let output_name = output.to_str().expect("owned PNG output path");
    let statistics = execute_handwriting(&args(&[
        "--set-global", "slant", "-85000", "85000", "50000",
        "50000", "0",
    ])).expect("slanted statistics profile").expect("stats JSON");
    fs::write(&stats_path, statistics).expect("owned statistics fixture");
    fs::write(&output, b"preserve preceding page one PNG")
        .expect("owned PNG sentinel");
    let command = args(&[
        "--printed-sheet", "true", "--export-paper", "png",
        "--dpi", "72", "--png-page", "1",
        "--glyph-stroke", "a", "3000", "1000", "3500", "500",
        "250", "#204060", "--handwriting-profile", stats_name,
        "--flow-markdown", "a  \na  \naaaaaaaa",
        "--flow-region", "30000", "40000", "25000", "30000",
        "--output", output_name, "--replace",
    ]);
    let error = execute_notebook(&command)
        .expect_err("page-two overflow must refuse page-one PNG export");
    assert!(error.contains("flow region"), "{error}");
    assert!(error.contains("page 2"), "{error}");
    assert_eq!(fs::read(&output).expect("PNG output preserved"),
        b"preserve preceding page one PNG");
    fs::remove_file(stats_path).expect("remove owned statistics fixture");
    fs::remove_file(output).expect("remove owned PNG sentinel");

    // A caller's manual page annotation is not the typed flow region;
    // validate only the generated heading, bullet and glyph ink.
    let manual = args(&[
        "--printed-sheet", "true", "--export-paper", "pdf",
        "--glyph-stroke", "a", "900", "1100", "2400", "3500",
        "250", "#204060", "--flow-markdown", "a",
        "--flow-region", "30000", "40000", "25000", "30000",
        "--ink-line", "1", "70000", "45000", "72000", "45000",
        "250", "#112233",
    ]);
    let pdf = execute_notebook(&manual)
        .expect("manual annotation outside flow remains permitted")
        .expect("native PDF ink");
    assert!(pdf.starts_with(b"%PDF-1.4"));
    assert!(String::from_utf8_lossy(&pdf).contains("0.067 0.133 0.200 RG"),
        "manual annotation remains in physical PDF, not ignored");
}

#[test]
fn fixed_flow_region_refuses_invalid_bounds_and_overflow_atomically() {
    let target = test_path().with_extension("pdf");
    fs::write(&target, b"previous valid notebook")
        .expect("test-owned original PDF");
    let name = target.to_str().expect("test output path");
    let base = args(&[
        "--printed-sheet", "true", "--export-paper", "pdf",
        "--glyph-stroke", "a", "0", "0", "3500", "4500",
        "250", "#204060", "--flow-markdown", "a  \na  \na",
    ]);
    let cases: [(&[&str], &str); 11] = [
        (&["10000", "40000", "50000", "30000"], "printable"),
        (&["30000", "10000", "50000", "30000"], "printable"),
        (&["180000", "40000", "50000", "30000"], "printable"),
        (&["30000", "260000", "50000", "30000"], "printable"),
        (&["30000", "40000", "0", "30000"], "positive"),
        (&["30000", "40000", "50000", "0"], "positive"),
        (&["4294967295", "40000", "50000", "30000"], "overflows"),
        (&["30000", "40000", "50000", "4294967296"], "u32"),
        (&["-1", "40000", "50000", "30000"], "unsigned"),
        (&["30000", "40000", "6000", "30000"], "too narrow"),
        (&["30000", "40000", "50000", "9000"], "vertical"),
    ];
    for (region, expected) in cases {
        let mut cli = base.clone();
        cli.push("--flow-region".into());
        cli.extend(args(region));
        cli.extend(args(&["--output", name, "--replace"]));
        let error = execute_notebook(&cli).expect_err("region must refuse");
        assert!(error.contains(expected), "{region:?}: {error}");
        assert_eq!(fs::read(&target).expect("original PDF preserved"),
            b"previous valid notebook");
    }
    for extra in [
        args(&["--flow-region", "30000", "40000", "50000"]),
        args(&["--flow-region", "30000", "40000", "50000", "30000",
            "--flow-region", "30000", "40000", "50000", "30000"]),
    ] {
        let mut cli = base.clone();
        cli.extend(extra);
        assert!(execute_notebook(&cli).is_err());
    }
    assert!(execute_notebook(&args(&[
        "--flow-region", "30000", "40000", "50000", "30000",
    ])).expect_err("region requires text source").contains("requires"));
    let mut shortage = base;
    shortage.extend(args(&[
        "--flow-region", "30000", "40000", "50000", "30000",
        "--pages", "1", "--output", name, "--replace",
    ]));
    assert!(execute_notebook(&shortage)
        .expect_err("three rows cannot fit on one two-row page")
        .contains("requires 2 physical pages"));
    assert_eq!(fs::read(&target).expect("unchanged PDF"),
        b"previous valid notebook");
    fs::remove_file(target).expect("remove only test-owned PDF");
}

#[test]
fn large_authored_glyph_flow_renders_exact_4096_segments_and_refuses_extra() {
    let mut command = args(&[
        "--printed-sheet", "true", "--export-paper", "pdf",
        "--flow-text", &"a".repeat(128),
    ]);
    // One deliberately authored 32-segment glyph repeated 128 times.
    // This tests real text expansion rather than only a manual-ink vector.
    for index in 0..32u32 {
        let x = 850 + index * 40;
        command.extend(args(&[
            "--glyph-stroke", "a", &x.to_string(), "1000",
            &(x + 250).to_string(), "3500", "250", "#243647",
        ]));
    }
    let pdf = execute_notebook(&command)
        .expect("expanded 4096-stroke glyph PDF")
        .expect("real PDF bytes");
    assert!(pdf.starts_with(b"%PDF-1.4"));
    assert!(pdf.len() < MAX_PAPER_OUTPUT_BYTES);
    assert_eq!(pdf, execute_notebook(&command)
        .expect("exact replay of large glyph layout")
        .expect("PDF bytes"));
    let mut png_command = command.clone();
    let format = png_command.iter().position(|value| value == "pdf")
        .expect("PDF output selection");
    *png_command.get_mut(format).expect("format value") = "png".into();
    png_command.extend(args(&["--dpi", "72"]));
    let png = execute_notebook(&png_command)
        .expect("real large transparent PNG").expect("PNG bytes");
    assert!(png.starts_with(b"\x89PNG\r\n\x1a\n"));
    assert_eq!(png, execute_notebook(&png_command)
        .expect("exact large PNG replay").expect("PNG bytes"));

    let target = test_path().with_extension("pdf");
    fs::write(&target, b"previous complete PDF")
        .expect("owned existing output");
    let mut over_limit = command;
    let source = over_limit.iter_mut().find(|item| item.len() == 128)
        .expect("flow text with 128 a glyphs");
    source.push('a');
    over_limit.extend(args(&[
        "--output", target.to_str().expect("owned path"), "--replace",
    ]));
    let error = execute_notebook(&over_limit)
        .expect_err("4097th requested segment refuses");
    assert!(error.contains("4096 total pen strokes"), "{error}");
    assert_eq!(fs::read(&target).expect("original output preserved"),
        b"previous complete PDF");
    fs::remove_file(target).expect("remove only owned output");
}

#[test]
fn native_glyph_width_preserves_exact_unicode_and_refuses_clipped_ink() {
    let original = execute_glyph_library(&args(&[
        "--glyph-stroke", "o\u{301}", "450", "1000", "850", "1600",
        "250", "#243647",
        "--glyph-stroke", "ó", "300", "1000", "500", "1500",
        "250", "#112233",
        "--glyph-stroke", "o\u{301}", "850", "1600", "1300", "2200",
        "250", "#243647",
    ])).expect("source admitted").expect("TSV");
    let source = test_path().with_extension("tsv");
    let output = test_path().with_extension("tsv");
    fs::write(&source, &original).expect("write source");
    fs::write(&output, b"unchanged destination").expect("prior output");
    let source_name = source.to_str().expect("path");
    let output_name = output.to_str().expect("path");
    let edited = execute_glyph_library(&args(&[
        "--profile", source_name, "--scale-glyph-width", "o\u{301}",
        "110", "1750",
    ])).expect("resize").expect("TSV");
    assert_eq!(edited, concat!(
        "atrament.glyph-strokes.v1\n",
        "ó\t320\t1000\t760\t1600\t250\t#243647\n",
        "ó\t300\t1000\t500\t1500\t250\t#112233\n",
        "ó\t760\t1600\t1255\t2200\t250\t#243647\n",
    ).as_bytes());
    assert_eq!(execute_glyph_library(&args(&[
        "--profile", source_name, "--scale-glyph-width", "o\u{301}",
        "100", "1750",
    ])).expect("identity").expect("TSV"), original);
    let report = execute_glyph_library(&args(&[
        "--profile", source_name, "--scale-glyph-width", "o\u{301}",
        "110", "1750", "--inspect-paths", "o\u{301}",
    ])).expect("inspect").expect("report");
    assert!(String::from_utf8_lossy(&report)
        .contains("Strokes: 2\nPaths: 1\nPen lifts: 0"));
    for (glyph, percent, anchor) in [
        ("o\u{301}", "49", "1750"),
        ("o\u{301}", "151", "1750"),
        ("o\u{301}", "110.5", "1750"),
        ("o\u{301}", "110", "3501"),
        ("ó", "150", "3500"),
        ("ñ", "110", "1750"),
        ("ab", "110", "1750"),
    ] {
        assert!(execute_glyph_library(&args(&[
            "--profile", source_name, "--scale-glyph-width", glyph,
            percent, anchor, "--output", output_name, "--replace",
        ])).is_err());
        assert_eq!(fs::read(&output).expect("prior output"),
            b"unchanged destination");
    }
    let collapse = execute_glyph_library(&args(&[
        "--glyph-stroke", "o", "1749", "2000", "1750", "2000",
        "250", "#112233", "--scale-glyph-width", "o", "50", "1750",
    ]));
    assert!(collapse.expect_err("short segment collapses")
        .contains("collapses a pen segment"));
    let negative_half = execute_glyph_library(&args(&[
        "--glyph-stroke", "o", "1749", "2000", "1751", "2100",
        "250", "#112233", "--scale-glyph-width", "o", "50", "1750",
    ])).expect("ties round upward").expect("TSV");
    assert!(String::from_utf8_lossy(&negative_half)
        .contains("o\t1750\t2000\t1751\t2100"));
    assert_eq!(fs::read(&source).expect("source unchanged"), original);
    fs::remove_file(source).expect("remove source");
    fs::remove_file(output).expect("remove output");
}

#[test]
fn native_glyph_ink_scales_one_exact_unicode_pen_and_refuses_atomically() {
    let nfd = "o\u{301}";
    let original = execute_glyph_library(&args(&[
        "--glyph-stroke", nfd, "500", "800", "850", "1200",
        "251", "#243647",
        "--glyph-stroke", "ó", "300", "1000", "500", "1500",
        "250", "#112233",
        "--glyph-stroke", nfd, "850", "1200", "1100", "1700",
        "251", "#243647",
    ])).expect("original admitted").expect("TSV");
    let source = test_path().with_extension("tsv");
    let output = test_path().with_extension("tsv");
    fs::write(&source, &original).expect("source written");
    fs::write(&output, b"preserved destination").expect("existing output");
    let source_name = source.to_str().expect("path");
    let output_name = output.to_str().expect("path");
    let edited = execute_glyph_library(&args(&[
        "--profile", source_name, "--scale-glyph-ink", nfd, "110",
    ])).expect("edit admitted").expect("TSV");
    assert_eq!(edited, concat!(
        "atrament.glyph-strokes.v1\n",
        "ó\t500\t800\t850\t1200\t276\t#243647\n",
        "ó\t300\t1000\t500\t1500\t250\t#112233\n",
        "ó\t850\t1200\t1100\t1700\t276\t#243647\n",
    ).as_bytes());
    assert_eq!(execute_glyph_library(&args(&[
        "--profile", source_name, "--scale-glyph-ink", nfd, "100",
    ])).expect("identity").expect("TSV"), original);
    let half = execute_glyph_library(&args(&[
        "--profile", source_name, "--scale-glyph-ink", nfd, "50",
    ])).expect("half-up tie").expect("TSV");
    assert!(String::from_utf8_lossy(&half)
        .contains("ó\t500\t800\t850\t1200\t126\t#243647"));
    let paths = execute_glyph_library(&args(&[
        "--profile", source_name, "--scale-glyph-ink", nfd, "110",
        "--inspect-paths", nfd,
    ])).expect("inspect revised paths").expect("report");
    assert!(String::from_utf8_lossy(&paths)
        .contains("Strokes: 2\nPaths: 1\nPen lifts: 0"));
    for (glyph, percent) in [
        (nfd, "49"), (nfd, "151"), (nfd, "110.5"),
        (nfd, "+110"), ("ñ", "110"), ("ab", "110"),
    ] {
        assert!(execute_glyph_library(&args(&[
            "--profile", source_name, "--scale-glyph-ink", glyph,
            percent, "--output", output_name, "--replace",
        ])).is_err());
        assert_eq!(fs::read(&output).expect("existing output"),
            b"preserved destination");
    }
    for (x, y, width, percent) in [
        ("125", "800", "251", "110"),
        ("500", "4400", "251", "110"),
        ("500", "800", "80", "50"),
        ("1000", "800", "1500", "150"),
    ] {
        let refusal = execute_glyph_library(&args(&[
            "--glyph-stroke", "o", x, y, "850", "1200", width,
            "#102030", "--scale-glyph-ink", "o", percent,
            "--output", output_name, "--replace",
        ]));
        assert!(refusal.is_err(), "invalid ink must refuse: {x} {y}");
        assert_eq!(fs::read(&output).expect("existing output"),
            b"preserved destination");
    }
    assert_eq!(fs::read(&source).expect("original source"), original);
    fs::remove_file(source).expect("remove owned source");
    fs::remove_file(output).expect("remove owned output");
}

#[test]
fn native_whole_glyph_translation_preserves_paths_and_refuses_clipping() {
    let nfd = "o\u{301}";
    let original = execute_glyph_library(&args(&[
        "--glyph-stroke", nfd, "450", "1000", "850", "1600",
        "251", "#243647",
        "--glyph-stroke", "ó", "300", "1000", "500", "1500",
        "250", "#112233",
        "--glyph-stroke", nfd, "850", "1600", "1300", "2200",
        "251", "#243647",
    ])).expect("source admitted").expect("TSV");
    let source = test_path().with_extension("tsv");
    let output = test_path().with_extension("tsv");
    fs::write(&source, &original).expect("owned source");
    fs::write(&output, b"prior destination").expect("sentinel");
    let source_name = source.to_str().expect("source path");
    let output_name = output.to_str().expect("output path");
    let shifted = execute_glyph_library(&args(&[
        "--profile", source_name, "--translate-glyph", nfd, "-200", "175",
    ])).expect("move admitted").expect("TSV bytes");
    assert_eq!(shifted, concat!(
        "atrament.glyph-strokes.v1\n",
        "ó\t250\t1175\t650\t1775\t251\t#243647\n",
        "ó\t300\t1000\t500\t1500\t250\t#112233\n",
        "ó\t650\t1775\t1100\t2375\t251\t#243647\n",
    ).as_bytes());
    assert_eq!(execute_glyph_library(&args(&[
        "--profile", source_name, "--translate-glyph", nfd, "0", "0",
    ])).expect("identity accepted").expect("TSV"), original);
    let report = execute_glyph_library(&args(&[
        "--profile", source_name, "--translate-glyph", nfd, "-200", "175",
        "--inspect-paths", nfd,
    ])).expect("inspect moved contours").expect("report");
    assert!(String::from_utf8_lossy(&report)
        .contains("Strokes: 2\nPaths: 1\nPen lifts: 0"));
    let mut authored = args(&["--profile", source_name,
        "--translate-glyph", nfd, "-200", "175",
        "--output", output_name, "--replace"]);
    assert!(execute_glyph_library(&authored)
        .expect("explicit replace allowed").is_none());
    assert_eq!(fs::read(&output).expect("saved TSV"), shifted);
    fs::write(&output, b"prior destination").expect("reset sentinel");
    for values in [
        (nfd, "-326", "175"), (nfd, "0", "-1000"),
        (nfd, "3501", "0"), (nfd, "0", "4501"),
        (nfd, "1.0", "0"), (nfd, "+1", "0"),
        ("ó", "-300", "0"), ("ñ", "0", "1"),
        ("ab", "0", "1"),
    ] {
        authored = args(&[
            "--profile", source_name, "--translate-glyph",
            values.0, values.1, values.2,
            "--output", output_name, "--replace",
        ]);
        assert!(execute_glyph_library(&authored).is_err(), "{values:?}");
        assert_eq!(fs::read(&output).expect("unchanged output"),
            b"prior destination");
    }
    assert_eq!(fs::read(&source).expect("source remains"), original);
    fs::remove_file(source).expect("remove owned source");
    fs::remove_file(output).expect("remove owned output");
}

#[test]
fn native_whole_glyph_rotation_is_bounded_and_preserves_exact_identity() {
    let nfd = "o\u{301}";
    let original = execute_glyph_library(&args(&[
        "--glyph-stroke", nfd, "450", "1000", "850", "1600",
        "251", "#243647",
        "--glyph-stroke", "ó", "600", "1200", "900", "1550",
        "250", "#112233",
        "--glyph-stroke", nfd, "850", "1600", "1300", "2200",
        "251", "#243647",
    ])).expect("source admitted").expect("TSV");
    let source = test_path().with_extension("tsv");
    let output = test_path().with_extension("tsv");
    fs::write(&source, &original).expect("owned original");
    fs::write(&output, b"prior approved TSV").expect("sentinel");
    let source_name = source.to_str().expect("source path");
    let output_name = output.to_str().expect("output path");
    let identity = execute_glyph_library(&args(&[
        "--profile", source_name, "--rotate-glyph", nfd, "0", "0", "0",
    ])).expect("zero admitted").expect("TSV");
    assert_eq!(identity, original);
    let rotated = execute_glyph_library(&args(&[
        "--profile", source_name, "--rotate-glyph",
        nfd, "10", "1750", "2250",
    ])).expect("rotation admitted").expect("TSV");
    assert_ne!(rotated, original);
    assert!(String::from_utf8_lossy(&rotated)
        .contains("ó\t600\t1200\t900\t1550\t250\t#112233"));
    let inspector = execute_glyph_library(&args(&[
        "--profile", source_name, "--rotate-glyph",
        nfd, "10", "1750", "2250", "--inspect-paths", nfd,
    ])).expect("inspector").expect("report");
    assert!(String::from_utf8_lossy(&inspector)
        .contains("Strokes: 2\nPaths: 1\nPen lifts: 0"));
    for (glyph, angle, x, y) in [
        (nfd, "21", "1750", "2250"),
        (nfd, "-21", "1750", "2250"),
        (nfd, "1.5", "1750", "2250"),
        (nfd, "10", "3501", "2250"),
        (nfd, "10", "1750", "4501"),
        (nfd, "20", "0", "0"),
        ("ñ", "10", "1750", "2250"),
        ("ab", "10", "1750", "2250"),
    ] {
        assert!(execute_glyph_library(&args(&[
            "--profile", source_name, "--rotate-glyph",
            glyph, angle, x, y, "--output", output_name, "--replace",
        ])).is_err(), "{glyph} {angle} {x} {y}");
        assert_eq!(fs::read(&output).expect("sentinel remains"),
            b"prior approved TSV");
    }
    assert!(execute_glyph_library(&args(&[
        "--glyph-stroke", nfd, "1300", "1626", "1301", "1626",
        "250", "#243647",
        "--rotate-glyph", nfd, "20", "1750", "2250",
    ])).is_err(), "rounded rotation must reject a collapsed pen line");
    assert_eq!(fs::read(&source).expect("original stays"), original);
    fs::remove_file(source).expect("cleanup source");
    fs::remove_file(output).expect("cleanup destination");
}

#[test]
fn opt_in_line_drift_is_bounded_smooth_per_run_and_seed_replayable()
-> Result<(), String> {
    let mut glyphs = GlyphOverlay::default();
    glyphs.parse("--glyph-stroke", &mut args(&[
        "\u{e1}", "600", "1000", "1600", "2800", "250", "#123456",
    ]).iter())?;
    for y in [30_000u32, 40_000u32] {
        glyphs.parse("--place-text", &mut args(&[
            "1", "25000", &y.to_string(), &"\u{e1}".repeat(16),
        ]).iter())?;
    }
    glyphs.parse("--glyph-variation", &mut args(&[
        "42", "0", "0", "0", "0", "0", "0",
    ]).iter())?;
    let legacy = glyphs.expand(&[], None)?;
    glyphs.parse("--glyph-line-drift", &mut args(&["150"]).iter())?;
    let drifted = glyphs.expand(&[], None)?;
    if drifted.len() != 32 || drifted == legacy {
        return Err("opt-in line drift did not change both rows".into());
    }
    for run in 0..2usize {
        let start = run * 16;
        let baseline = if run == 0 { 31_000i64 } else { 41_000i64 };
        let mut previous: Option<i64> = None;
        for (index, segment) in drifted.get(start..start + 16)
            .ok_or("bounded run span missing")?.iter().enumerate() {
            let delta = i64::from(segment.start_um.1) - baseline;
            if delta.unsigned_abs() > 150 {
                return Err(format!(
                    "run {run} col {index} drift {delta} exceeds cap"
                ));
            }
            if let Some(earlier) = previous
                && (delta - earlier).unsigned_abs() > 38 {
                return Err(format!("run {run} col {index} drift jumped"));
            }
            previous = Some(delta);
            if segment.end_um.1 != segment.start_um.1 + 1800 {
                return Err("line drift changed authored segment shape".into());
            }
        }
    }
    if drifted.get(..16).ok_or("first run missing")?.iter()
        .zip(drifted.get(16..).ok_or("second run missing")?)
        .all(|(left, right)| left.start_um.1 + 10_000 == right.start_um.1) {
        return Err("different lines unexpectedly share the same drift".into());
    }
    if glyphs.expand(&[], None)? != drifted {
        return Err("replayed line drift was not identical".into());
    }
    glyphs.line_drift_um = Some(0);
    if glyphs.expand(&[], None)? != legacy {
        return Err("zero line drift changed legacy ink".into());
    }
    Ok(())
}

#[test]
fn line_drift_cli_refuses_invalid_options_before_overwriting_output()
-> Result<(), String> {
    let target = test_path();
    fs::write(&target, b"preserve").map_err(|error| error.to_string())?;
    let name = target.to_str().ok_or("target needs UTF-8")?;
    let valid = args(&[
        "--export-paper", "pdf", "--glyph-stroke", "\u{e1}", "600",
        "1000", "1600", "2800", "250", "#123456",
        "--place-text", "1", "25000", "30000", "\u{e1}\u{e1}",
        "--output", name, "--replace",
    ]);
    for rejected in [
        args(&["--glyph-line-drift", "-1"]),
        args(&["--glyph-line-drift", "151"]),
        args(&["--glyph-line-drift", "4294967296"]),
        args(&["--glyph-line-drift", "150"]),
        args(&[
            "--glyph-variation", "42", "0", "0", "0", "0", "0", "0",
            "--glyph-line-drift", "5", "--glyph-line-drift", "6",
        ]),
    ] {
        let mut command = valid.clone();
        command.extend(rejected);
        if execute_notebook(&command).is_ok() {
            return Err(format!("invalid line drift accepted: {command:?}"));
        }
        if fs::read(&target).map_err(|error| error.to_string())?
            != b"preserve" {
            return Err("refused line drift changed accepted output".into());
        }
    }
    let mut command = valid;
    command.extend(args(&[
        "--glyph-line-drift", "150", "--glyph-variation",
        "42", "0", "0", "0", "0", "0", "0",
    ]));
    let output = execute_notebook(&command)?;
    if output.is_some() {
        return Err("explicit output unexpectedly wrote stdout".into());
    }
    let bytes = fs::read(&target).map_err(|error| error.to_string())?;
    if !bytes.starts_with(b"%PDF-1.4") || bytes == b"preserve" {
        return Err("admitted line drift did not produce PDF".into());
    }
    fs::remove_file(&target).map_err(|error| error.to_string())?;
    Ok(())
}


#[test]
fn line_drift_changes_native_flow_pdf_and_png_with_stable_seed()
-> Result<(), String> {
    let source = format!(
        "{}  \n{} {}", "\u{e1}".repeat(12), "\u{e1}",
        "\u{e1}".repeat(11),
    );
    let bare = args(&[
        "--printed-sheet", "true", "--glyph-stroke", "\u{e1}", "600",
        "1000", "1600", "2800", "250", "#123456",
        "--flow-markdown", &source,
        "--glyph-variation", "42", "0", "0", "0", "0", "0", "0",
    ]);
    let run = |format: &str, extra: &[&str]| -> Result<Vec<u8>, String> {
        let mut command = bare.clone();
        command.extend(args(&["--export-paper", format]));
        if format == "png" {
            command.extend(args(&["--dpi", "72"]));
        }
        command.extend(args(extra));
        execute_notebook(&command)?
            .ok_or_else(|| "native output bytes missing".into())
    };
    for format in ["pdf", "png"] {
        let original = run(format, &[])?;
        let zero = run(format, &["--glyph-line-drift", "0"])?;
        let changed = run(format, &["--glyph-line-drift", "150"])?;
        if original != zero || original == changed {
            return Err(format!(
                "{format} lost optional line-drift compatibility"
            ));
        }
        if changed != run(format, &["--glyph-line-drift", "150"])? {
            return Err(format!("{format} drift did not replay"));
        }
        let different = {
            let mut alternate = bare.clone();
            let seed = alternate.iter()
                .position(|arg| arg == "--glyph-variation")
                .ok_or("variation flag missing")?;
            *alternate.get_mut(seed + 1).ok_or("seed missing")? = "43".into();
            let mut command = alternate;
            command.extend(args(&["--export-paper", format,
                "--glyph-line-drift", "150"]));
            if format == "png" {
                command.extend(args(&["--dpi", "72"]));
            }
            execute_notebook(&command)?.ok_or("alternate output missing")?
        };
        if changed == different {
            return Err(format!("{format} did not respond to drift seed"));
        }
        if format == "pdf" && !changed.starts_with(b"%PDF-1.4") {
            return Err("drifted PDF header missing".into());
        }
        if format == "png" && !changed.starts_with(b"\x89PNG\r\n\x1a\n") {
            return Err("drifted PNG header missing".into());
        }
    }
    Ok(())
}

#[test]
fn opt_in_word_drift_is_shared_per_exact_unicode_word_and_run()
-> Result<(), String> {
    let glyph = "n\u{303}";
    let mut overlay = GlyphOverlay::default();
    overlay.parse("--glyph-stroke", &mut args(&[
        glyph, "600", "1000", "1600", "2800", "250", "#123456",
    ]).iter())?;
    let text = format!("{glyph}{glyph}  {glyph} {glyph}{glyph}");
    for y in [30_000u32, 40_000u32] {
        overlay.parse("--place-text", &mut args(&[
            "1", "25000", &y.to_string(), &text,
        ]).iter())?;
    }
    overlay.parse("--glyph-variation", &mut args(&[
        "42", "0", "0", "0", "0", "0", "0",
    ]).iter())?;
    let original = overlay.expand(&[], None)?;
    overlay.parse("--glyph-word-drift", &mut args(&["150"]).iter())?;
    let changed = overlay.expand(&[], None)?;
    if changed.len() != 10 || changed == original {
        return Err("word drift did not change ten exact graphemes".into());
    }
    let mut shifts = Vec::new();
    for (before, after) in original.iter().zip(&changed) {
        let dx = i64::from(after.start_um.0) - i64::from(before.start_um.0);
        if dx.unsigned_abs() > 150
            || i64::from(after.end_um.0) - i64::from(before.end_um.0) != dx
            || before.start_um.1 != after.start_um.1
            || before.end_um.1 != after.end_um.1
            || before.width_um != after.width_um
        {
            return Err("word drift changed pen shape or left its cap".into());
        }
        shifts.push(dx);
    }
    for row in 0..2usize {
        let start = row * 5;
        let [first, second, _, fourth, fifth] = shifts
            .get(start..start + 5).ok_or("word shift row missing")? else {
            return Err("five glyph shifts required per row".into());
        };
        if first != second || fourth != fifth {
            return Err("adjacent letters lost shared word offset".into());
        }
    }
    let first_shift = shifts.first().ok_or("word shifts missing")?;
    if shifts.iter().all(|shift| shift == first_shift) {
        return Err("all seeded word offsets unexpectedly coincide".into());
    }
    if overlay.expand(&[], None)? != changed {
        return Err("word drift lost seed replay".into());
    }
    overlay.word_drift_um = Some(0);
    if overlay.expand(&[], None)? != original {
        return Err("zero word offset changed legacy ink".into());
    }
    overlay.word_drift_um = Some(150);
    overlay.line_drift_um = Some(120);
    let combined = overlay.expand(&[], None)?;
    overlay.word_drift_um = Some(0);
    let vertical = overlay.expand(&[], None)?;
    for ((both, horizontal), vertical_line) in combined.iter()
        .zip(&changed).zip(&vertical) {
        if both.start_um.0 != horizontal.start_um.0
            || both.end_um.0 != horizontal.end_um.0
            || both.start_um.1 != vertical_line.start_um.1
            || both.end_um.1 != vertical_line.end_um.1 {
            return Err("word and line rhythm did not compose".into());
        }
    }
    Ok(())
}

#[test]
fn seeded_word_drift_preserves_short_selected_intra_word_joins()
-> Result<(), String> {
    let mut glyphs = GlyphOverlay::default();
    glyphs.parse("--glyph-stroke", &mut args(&[
        "\u{e1}", "500", "1200", "3300", "1200", "250", "#123456",
    ]).iter())?;
    glyphs.parse("--place-text", &mut args(&[
        "1", "25000", "30000", "\u{e1}\u{e1} \u{e1}\u{e1}",
    ]).iter())?;
    glyphs.parse("--glyph-variation", &mut args(&[
        "42", "0", "0", "0", "0", "0", "0",
    ]).iter())?;
    glyphs.add_text_join_pair(&mut args(&["\u{e1}", "\u{e1}"]).iter())?;
    let plain = glyphs.expand(&[], None)?;
    glyphs.parse("--glyph-word-drift", &mut args(&["150"]).iter())?;
    let drifted = glyphs.expand(&[], None)?;
    if drifted.len() != 6 || plain.len() != 6 {
        return Err("two selected intra-word bridges missing".into());
    }
    for (group, start) in [0usize, 3].into_iter().enumerate() {
        let [first, bridge, last] = drifted
            .get(start..start + 3).ok_or("joined word missing")? else {
            return Err("joined word needs three pen segments".into());
        };
        let original = plain.get(start..start + 3)
            .ok_or("original word missing")?;
        let original_first = original.first().ok_or("first pen missing")?;
        let offset = i64::from(first.start_um.0)
            - i64::from(original_first.start_um.0);
        if offset.unsigned_abs() > 150 {
            return Err(format!("word {group} offset exceeds bound"));
        }
        for (before, after) in original.iter().zip([first, bridge, last]) {
            if i64::from(after.start_um.0)
                - i64::from(before.start_um.0) != offset
                || i64::from(after.end_um.0)
                    - i64::from(before.end_um.0) != offset
                || after.start_um.1 != before.start_um.1 {
                return Err("word shift disconnected the selected join".into());
            }
        }
        if first.end_um != bridge.start_um
            || bridge.end_um != last.start_um {
            return Err("short join no longer touches its pen endpoints".into());
        }
    }
    Ok(())
}

#[test]
fn word_drift_native_pdf_png_replay_and_refusal_are_atomic()
-> Result<(), String> {
    let prefix = args(&[
        "--printed-sheet", "true", "--glyph-stroke", "\u{e1}", "600",
        "1000", "1600", "2800", "250", "#123456",
        "--flow-markdown", concat!(
            "\u{e1}\u{e1} \u{e1} \u{e1}\u{e1}\n\n",
            "\u{e1}\u{e1} \u{e1}\u{e1}",
        ),
        "--glyph-variation", "42", "0", "0", "0", "0", "0", "0",
    ]);
    for format in ["pdf", "png"] {
        let render = |extra: &[&str]| -> Result<Vec<u8>, String> {
            let mut command = prefix.clone();
            command.extend(args(&["--export-paper", format]));
            if format == "png" {
                command.extend(args(&["--dpi", "72"]));
            }
            command.extend(args(extra));
            execute_notebook(&command)?
                .ok_or_else(|| "missing native output".into())
        };
        let baseline = render(&[])?;
        if baseline != render(&["--glyph-word-drift", "0"])? {
            return Err(format!("{format} zero word drift changed bytes"));
        }
        let shifted = render(&["--glyph-word-drift", "150"])?;
        if shifted == baseline
            || shifted != render(&["--glyph-word-drift", "150"])? {
            return Err(format!("{format} failed seeded word replay"));
        }
        if format == "pdf" && !shifted.starts_with(b"%PDF-1.4") {
            return Err("word-drifted PDF header missing".into());
        }
        if format == "png" && !shifted.starts_with(b"\x89PNG\r\n\x1a\n") {
            return Err("word-drifted PNG signature missing".into());
        }
    }
    let target = test_path();
    fs::write(&target, b"accepted output").map_err(|error| error.to_string())?;
    let name = target.to_str().ok_or("output path needs UTF-8")?;
    for invalid in [
        args(&["--glyph-word-drift", "-1"]),
        args(&["--glyph-word-drift", "151"]),
        args(&["--glyph-word-drift", "4294967296"]),
        args(&["--glyph-word-drift", "150", "--glyph-word-drift", "5"]),
    ] {
        let mut command = prefix.clone();
        command.extend(args(&["--export-paper", "pdf", "--output", name,
            "--replace"]));
        command.extend(invalid);
        if execute_notebook(&command).is_ok()
            || fs::read(&target).map_err(|error| error.to_string())?
                != b"accepted output" {
            return Err("invalid word drift modified existing export".into());
        }
    }
    let mut unpaired = prefix;
    let option = unpaired.iter().position(|value| value == "--glyph-variation")
        .ok_or("expected variation option")?;
    unpaired.drain(option..option + 8).for_each(drop);
    unpaired.extend(args(&["--glyph-word-drift", "150", "--export-paper",
        "pdf", "--output", name, "--replace"]));
    if execute_notebook(&unpaired).is_ok()
        || fs::read(&target).map_err(|error| error.to_string())?
            != b"accepted output" {
        return Err("unpaired word drift modified existing export".into());
    }
    fs::remove_file(&target).map_err(|error| error.to_string())?;
    Ok(())
}

#[test]
fn opt_in_word_rise_is_shared_by_exact_words_and_composes_with_line_rhythm()
-> Result<(), String> {
    let glyph = "n\u{303}";
    let mut overlay = GlyphOverlay::default();
    overlay.parse("--glyph-stroke", &mut args(&[
        glyph, "600", "1000", "1600", "2800", "250", "#123456",
    ]).iter())?;
    let text = format!("{glyph}{glyph}  {glyph} {glyph}{glyph}");
    for y in [30_000u32, 40_000u32] {
        overlay.parse("--place-text", &mut args(&[
            "1", "25000", &y.to_string(), &text,
        ]).iter())?;
    }
    overlay.parse("--glyph-variation", &mut args(&[
        "42", "0", "0", "0", "0", "0", "0",
    ]).iter())?;
    let original = overlay.expand(&[], None)?;
    overlay.parse("--glyph-word-rise", &mut args(&["150"]).iter())?;
    let raised = overlay.expand(&[], None)?;
    if raised.len() != 10 || raised == original {
        return Err("word rise did not affect ten exact graphemes".into());
    }
    let mut shifts = Vec::new();
    for (before, after) in original.iter().zip(&raised) {
        let dy = i64::from(after.start_um.1) - i64::from(before.start_um.1);
        if dy.unsigned_abs() > 150
            || i64::from(after.end_um.1) - i64::from(before.end_um.1) != dy
            || before.start_um.0 != after.start_um.0
            || before.end_um.0 != after.end_um.0
            || before.width_um != after.width_um
            || before.rgb != after.rgb {
            return Err("word rise changed ink or left its bound".into());
        }
        shifts.push(dy);
    }
    for run in 0..2usize {
        let start = run * 5;
        let [first, second, _, fourth, fifth] = shifts.get(start..start + 5)
            .ok_or("five grapheme word span missing")? else {
            return Err("five grapheme word span needed".into());
        };
        if first != second || fourth != fifth {
            return Err("word mates received different vertical offsets".into());
        }
    }
    // Independently calculated uint32 avalanche/xorshift samples for the
    // seed-42 streams: word 0/1/2 on physical rows 0 and 1.
    if shifts != [-59, -59, 126, 117, 117, 116, 116, -38, 73, 73] {
        return Err(format!("word rise stream changed: {shifts:?}"));
    }
    if overlay.expand(&[], None)? != raised {
        return Err("word rise seed replay changed".into());
    }
    overlay.word_rise_um = Some(0);
    if overlay.expand(&[], None)? != original {
        return Err("zero rise is not legacy ink".into());
    }
    overlay.word_rise_um = Some(150);
    overlay.line_drift_um = Some(120);
    overlay.word_drift_um = Some(90);
    let combined = overlay.expand(&[], None)?;
    overlay.word_rise_um = Some(0);
    let without_rise = overlay.expand(&[], None)?;
    overlay.word_rise_um = Some(150);
    overlay.line_drift_um = Some(0);
    overlay.word_drift_um = Some(0);
    let rise_only = overlay.expand(&[], None)?;
    for (((both, base), only), legacy) in combined.iter()
        .zip(&without_rise).zip(&rise_only).zip(&original) {
        if both.start_um.0 != base.start_um.0
            || both.end_um.0 != base.end_um.0
            || i64::from(both.start_um.1) - i64::from(base.start_um.1)
                != i64::from(only.start_um.1)
                    - i64::from(legacy.start_um.1)
            || i64::from(both.end_um.1) - i64::from(base.end_um.1)
                != i64::from(only.end_um.1)
                    - i64::from(legacy.end_um.1) {
            return Err("word rise did not compose independently".into());
        }
    }
    overlay.word_rise_um = Some(151);
    if overlay.expand(&[], None).is_ok() {
        return Err("typed out-of-range word rise accepted".into());
    }
    overlay.word_rise_um = Some(150);
    overlay.variation = None;
    if overlay.expand(&[], None).is_ok() {
        return Err("typed unpaired word rise accepted".into());
    }
    Ok(())
}

#[test]
fn seeded_word_rise_preserves_directed_short_intra_word_bridges()
-> Result<(), String> {
    let mut overlay = GlyphOverlay::default();
    overlay.parse("--glyph-stroke", &mut args(&[
        "\u{e1}", "500", "1200", "3300", "1200", "250", "#123456",
    ]).iter())?;
    overlay.parse("--place-text", &mut args(&[
        "1", "25000", "30000", "\u{e1}\u{e1}  \u{e1}\u{e1}",
    ]).iter())?;
    overlay.parse("--glyph-variation", &mut args(&[
        "42", "0", "0", "0", "0", "0", "0",
    ]).iter())?;
    overlay.add_text_join_pair(&mut args(&["\u{e1}", "\u{e1}"]).iter())?;
    let baseline = overlay.expand(&[], None)?;
    overlay.parse("--glyph-word-rise", &mut args(&["150"]).iter())?;
    let changed = overlay.expand(&[], None)?;
    if baseline.len() != 6 || changed.len() != 6 {
        return Err("two joined words must emit six pen segments".into());
    }
    for start in [0usize, 3] {
        let first = changed.get(start).ok_or("first pen missing")?;
        let bridge = changed.get(start + 1).ok_or("bridge missing")?;
        let last = changed.get(start + 2).ok_or("last pen missing")?;
        if first.end_um != bridge.start_um
            || bridge.end_um != last.start_um {
            return Err("word rise disconnected a directed join".into());
        }
        let old = baseline.get(start).ok_or("old pen missing")?;
        let dy = i64::from(first.start_um.1) - i64::from(old.start_um.1);
        if dy.unsigned_abs() > 150 {
            return Err("joined word rise exceeded cap".into());
        }
        for (before, after) in baseline.get(start..start + 3)
            .ok_or("old group missing")?.iter().zip([first, bridge, last]) {
            if before.start_um.0 != after.start_um.0
                || before.end_um.0 != after.end_um.0
                || i64::from(after.start_um.1)
                    - i64::from(before.start_um.1) != dy
                || i64::from(after.end_um.1)
                    - i64::from(before.end_um.1) != dy {
                return Err("joined word did not translate uniformly".into());
            }
        }
    }
    Ok(())
}

#[test]
fn word_rise_native_pdf_png_replay_and_refusal_preserve_output()
-> Result<(), String> {
    let prefix = args(&[
        "--printed-sheet", "true", "--glyph-stroke", "\u{e1}", "600",
        "1000", "1600", "2800", "250", "#123456",
        "--flow-markdown", concat!(
            "\u{e1}\u{e1}  \u{e1} \u{e1}\u{e1}\n\n",
            "\u{e1}\u{e1} \u{e1}\u{e1}",
        ),
        "--glyph-variation", "42", "0", "0", "0", "0", "0", "0",
    ]);
    for format in ["pdf", "png"] {
        let render = |extra: &[&str], seed: &str| -> Result<Vec<u8>, String> {
            let mut command = prefix.clone();
            let seed_index = command.iter()
                .position(|arg| arg == "--glyph-variation")
                .ok_or("variation missing")? + 1;
            *command.get_mut(seed_index).ok_or("seed missing")? = seed.into();
            command.extend(args(&["--export-paper", format]));
            if format == "png" {
                command.extend(args(&["--dpi", "72"]));
            }
            command.extend(args(extra));
            execute_notebook(&command)?
                .ok_or_else(|| "native output missing".into())
        };
        let baseline = render(&[], "42")?;
        if baseline != render(&["--glyph-word-rise", "0"], "42")? {
            return Err(format!("{format} zero rise changed native bytes"));
        }
        let raised = render(&["--glyph-word-rise", "150"], "42")?;
        if raised == baseline
            || raised != render(&["--glyph-word-rise", "150"], "42")?
            || raised == render(&["--glyph-word-rise", "150"], "43")? {
            return Err(format!("{format} rise seed replay invalid"));
        }
        if format == "pdf" && !raised.starts_with(b"%PDF-1.4") {
            return Err("word-rise PDF header missing".into());
        }
        if format == "png" && !raised.starts_with(b"\x89PNG\r\n\x1a\n") {
            return Err("word-rise PNG signature missing".into());
        }
    }
    let target = test_path();
    fs::write(&target, b"accepted bytes").map_err(|error| error.to_string())?;
    let path = target.to_str().ok_or("output path must be UTF-8")?;
    for invalid in [
        args(&["--glyph-word-rise", "-1"]),
        args(&["--glyph-word-rise", "151"]),
        args(&["--glyph-word-rise", "4294967296"]),
        args(&["--glyph-word-rise", "120", "--glyph-word-rise", "150"]),
    ] {
        let mut command = prefix.clone();
        command.extend(args(&["--export-paper", "pdf", "--output", path,
            "--replace"]));
        command.extend(invalid);
        if execute_notebook(&command).is_ok()
            || fs::read(&target).map_err(|error| error.to_string())?
                != b"accepted bytes" {
            return Err("invalid rise overwrote accepted output".into());
        }
    }
    let mut unpaired = prefix;
    let option = unpaired.iter().position(|arg| arg == "--glyph-variation")
        .ok_or("expected variation")?;
    unpaired.drain(option..option + 8).for_each(drop);
    unpaired.extend(args(&["--glyph-word-rise", "150", "--export-paper",
        "pdf", "--output", path, "--replace"]));
    if execute_notebook(&unpaired).is_ok()
        || fs::read(&target).map_err(|error| error.to_string())?
            != b"accepted bytes" {
        return Err("unpaired rise overwrote accepted output".into());
    }
    fs::remove_file(&target).map_err(|error| error.to_string())?;
    Ok(())
}

#[test]
fn seeded_word_pressure_shares_physical_width_without_moving_authored_ink()
-> Result<(), String> {
    let glyph = "n\u{303}";
    let mut overlay = GlyphOverlay::default();
    overlay.parse("--glyph-stroke", &mut args(&[
        glyph, "600", "1000", "1600", "2800", "250", "#123456",
    ]).iter())?;
    let text = format!("{glyph}{glyph}  {glyph} {glyph}{glyph}");
    for y in [30_000u32, 40_000u32] {
        overlay.parse("--place-text", &mut args(&[
            "1", "25000", &y.to_string(), &text,
        ]).iter())?;
    }
    overlay.parse("--glyph-variation", &mut args(&[
        "42", "0", "0", "0", "0", "0", "0",
    ]).iter())?;
    let baseline = overlay.expand(&[], None)?;
    overlay.parse("--glyph-word-pressure", &mut args(&["20"]).iter())?;
    let pressured = overlay.expand(&[], None)?;
    let widths = pressured.iter().map(|ink| ink.width_um)
        .collect::<Vec<_>>();
    // Independent uint32 avalanche/xorshift oracle: word indices 0, 1, 2
    // for each distinct run, using seed 42 and max 20 percent.
    if widths != [285, 285, 203, 285, 285, 205, 205, 225, 208, 208] {
        return Err(format!("word-pressure sample changed: {widths:?}"));
    }
    for (plain, varied) in baseline.iter().zip(&pressured) {
        if plain.start_um != varied.start_um
            || plain.end_um != varied.end_um || plain.rgb != varied.rgb
            || !(200..=300).contains(&varied.width_um) {
            return Err("word pressure moved ink or left width bounds".into());
        }
    }
    if overlay.expand(&[], None)? != pressured {
        return Err("word-pressure replay changed physical ink".into());
    }
    overlay.word_pressure_percent = Some(0);
    if overlay.expand(&[], None)? != baseline {
        return Err("zero word pressure differs from legacy ink".into());
    }
    overlay.word_pressure_percent = Some(20);
    overlay.word_rise_um = Some(150);
    overlay.line_drift_um = Some(120);
    overlay.word_drift_um = Some(90);
    let combined = overlay.expand(&[], None)?;
    overlay.word_pressure_percent = Some(0);
    let unpressured = overlay.expand(&[], None)?;
    for (pressure, unchanged) in combined.iter().zip(&unpressured) {
        if pressure.start_um != unchanged.start_um
            || pressure.end_um != unchanged.end_um
            || pressure.rgb != unchanged.rgb {
            return Err("word pressure changed independent rhythm".into());
        }
    }
    overlay.word_pressure_percent = Some(21);
    if overlay.expand(&[], None).is_ok() {
        return Err("typed out-of-range word pressure accepted".into());
    }
    overlay.word_pressure_percent = Some(20);
    overlay.variation = None;
    if overlay.expand(&[], None).is_ok() {
        return Err("typed unpaired word pressure accepted".into());
    }
    Ok(())
}

#[test]
fn seeded_word_pressure_preserves_same_pen_directed_intra_word_bridges()
-> Result<(), String> {
    let mut overlay = GlyphOverlay::default();
    overlay.parse("--glyph-stroke", &mut args(&[
        "\u{e1}", "500", "1200", "3300", "1200", "250", "#123456",
    ]).iter())?;
    overlay.parse("--place-text", &mut args(&[
        "1", "25000", "30000", "\u{e1}\u{e1} \u{e1}\u{e1}",
    ]).iter())?;
    overlay.parse("--glyph-variation", &mut args(&[
        "42", "0", "0", "0", "0", "0", "0",
    ]).iter())?;
    overlay.add_text_join_pair(
        &mut args(&["\u{e1}", "\u{e1}"]).iter(),
    )?;
    let original = overlay.expand(&[], None)?;
    overlay.parse("--glyph-word-pressure", &mut args(&["20"]).iter())?;
    let actual = overlay.expand(&[], None)?;
    if original.len() != 6 || actual.len() != 6 {
        return Err("two joined word groups must contain six lines".into());
    }
    for (offset, expected_width) in [(0usize, 285u32), (3, 203)] {
        let [first, bridge, last] = actual.get(offset..offset + 3)
            .ok_or("word group missing")? else {
            return Err("word group must contain three strokes".into());
        };
        if first.end_um != bridge.start_um
            || bridge.end_um != last.start_um
            || first.width_um != expected_width
            || bridge.width_um != expected_width
            || last.width_um != expected_width {
            return Err("word pressure broke the selected pen join".into());
        }
    }
    for (before, after) in original.iter().zip(&actual) {
        if before.start_um != after.start_um
            || before.end_um != after.end_um
            || before.rgb != after.rgb {
            return Err("word pressure moved a directed join".into());
        }
    }
    Ok(())
}

#[test]
fn word_pressure_pdf_png_and_ink_file_replay_with_atomic_refusal()
-> Result<(), String> {
    let prefix = args(&[
        "--printed-sheet", "true",
        "--glyph-stroke", "\u{e1}", "600", "1000", "1600", "2800",
        "250", "#123456",
        "--flow-markdown", concat!(
            "\u{e1}\u{e1} \u{e1} \u{e1}\u{e1}\n\n",
            "\u{e1}\u{e1} \u{e1}\u{e1}",
        ),
        "--glyph-variation", "42", "0", "0", "0", "0", "0", "0",
    ]);
    for format in ["pdf", "png", "ink"] {
        let render = |seed: &str, extras: &[&str]|
            -> Result<Vec<u8>, String> {
            let mut command = prefix.clone();
            let seed_index = command.iter().position(|value| {
                value == "--glyph-variation"
            }).ok_or("variation flag missing")? + 1;
            *command.get_mut(seed_index).ok_or("seed missing")? = seed.into();
            if format == "ink" {
                command.push("--emit-ink-file".into());
            } else {
                command.extend(args(&["--export-paper", format]));
            }
            if format == "png" {
                command.extend(args(&["--dpi", "72"]));
            }
            command.extend(args(extras));
            execute_notebook(&command)?.ok_or("export bytes missing".into())
        };
        let original = render("42", &[])?;
        if original != render("42", &["--glyph-word-pressure", "0"])? {
            return Err(format!("{format} zero pressure changed bytes"));
        }
        let changed = render("42", &["--glyph-word-pressure", "20"])?;
        if changed == original
            || changed != render("42", &["--glyph-word-pressure", "20"])?
            || changed == render("43", &["--glyph-word-pressure", "20"])? {
            return Err(format!("{format} word-pressure replay invalid"));
        }
        if format == "pdf" && !changed.starts_with(b"%PDF-1.4") {
            return Err("word-pressure PDF header missing".into());
        }
        if format == "png" && !changed.starts_with(b"\x89PNG\r\n\x1a\n") {
            return Err("word-pressure PNG signature missing".into());
        }
        if format == "ink"
            && !changed.starts_with(b"atrament.page-ink.v1\n") {
            return Err("word-pressure ink format header missing".into());
        }
    }
    let target = test_path();
    fs::write(&target, b"accepted bytes").map_err(|e| e.to_string())?;
    let path = target.to_str().ok_or("non-UTF8 output path")?;
    for invalid in [
        args(&["--glyph-word-pressure", "-1"]),
        args(&["--glyph-word-pressure", "21"]),
        args(&["--glyph-word-pressure", "4294967296"]),
        args(&["--glyph-word-pressure", "20",
            "--glyph-word-pressure", "1"]),
    ] {
        let mut command = prefix.clone();
        command.extend(args(&[
            "--export-paper", "pdf", "--output", path, "--replace",
        ]));
        command.extend(invalid);
        if execute_notebook(&command).is_ok()
            || fs::read(&target).map_err(|e| e.to_string())?
                != b"accepted bytes" {
            return Err("invalid word pressure replaced output".into());
        }
    }
    let mut unpaired = prefix;
    let at = unpaired.iter().position(|value| {
        value == "--glyph-variation"
    }).ok_or("variation flag missing")?;
    unpaired.drain(at..at + 8).for_each(drop);
    unpaired.extend(args(&[
        "--glyph-word-pressure", "20", "--export-paper", "pdf",
        "--output", path, "--replace",
    ]));
    if execute_notebook(&unpaired).is_ok()
        || fs::read(&target).map_err(|e| e.to_string())?
            != b"accepted bytes" {
        return Err("unpaired word pressure replaced output".into());
    }
    fs::remove_file(&target).map_err(|e| e.to_string())?;
    Ok(())
}

#[test]
fn word_pressure_refuses_physical_pen_limits_and_scales_heading_ink()
-> Result<(), String> {
    let mut overlay = GlyphOverlay::default();
    overlay.parse("--glyph-stroke", &mut args(&[
        "\u{e1}", "1000", "1000", "2000", "2000", "250", "#123456",
    ]).iter())?;
    overlay.parse("--place-text", &mut args(&[
        "1", "25000", "30000", "\u{e1}\u{e1}",
    ]).iter())?;
    overlay.parse("--glyph-variation", &mut args(&[
        "42", "0", "0", "0", "0", "0", "0",
    ]).iter())?;
    overlay.parse("--glyph-word-pressure", &mut args(&["20"]).iter())?;
    let first = overlay.expand(&[], None)?;
    let run = overlay.runs.get_mut(0).ok_or("text run missing")?;
    run.heading_ink = Some(HeadingInk {
        color: Some([0x45, 0x67, 0x89]), scale_percent: 150,
    });
    let themed = overlay.expand(&[], None)?;
    if first.len() != 2 || first[0].width_um != 285
        || themed.len() != 2 || themed[0].width_um != 428
        || themed[1].width_um != 428
        || themed[0].rgb != [0x45, 0x67, 0x89]
        || themed[0].start_um != first[0].start_um {
        return Err("combined heading/pressure width is incorrect".into());
    }

    let target = test_path();
    fs::write(&target, b"accepted pdf").map_err(|e| e.to_string())?;
    let path = target.to_str().ok_or("non-UTF8 output path")?;
    for (width, text, start, end) in [
        ("80", "\u{e1} \u{e1}", "600", "1600"),
        ("1500", "\u{e1}", "1000", "2000"),
    ] {
        let command = args(&[
            "--printed-sheet", "true", "--glyph-stroke", "\u{e1}",
            start, "1000", end, "2000", width, "#123456",
            "--place-text", "1", "25000", "30000", text,
            "--glyph-variation", "42", "0", "0", "0", "0", "0", "0",
            "--glyph-word-pressure", "20", "--export-paper", "pdf",
            "--output", path, "--replace",
        ]);
        let failure = execute_notebook(&command);
        if !failure.as_ref().err().is_some_and(|error| {
            error.contains("scaled glyph ink width exceeds 80..1500 um")
        }) || fs::read(&target).map_err(|e| e.to_string())?
            != b"accepted pdf" {
            return Err(format!(
                "invalid physical pressure committed: {failure:?}"
            ));
        }
    }
    fs::remove_file(&target).map_err(|e| e.to_string())?;
    Ok(())
}

#[test]
fn seeded_word_slant_shares_shear_per_unicode_word_and_preserves_zero_bytes()
-> Result<(), String> {
    let glyph = "n\u{303}";
    let mut overlay = GlyphOverlay::default();
    overlay.parse("--glyph-stroke", &mut args(&[
        glyph, "600", "1000", "1600", "2800", "250", "#123456",
    ]).iter())?;
    let text = format!("{glyph}{glyph}  {glyph} {glyph}{glyph}");
    for y in [30_000u32, 40_000u32] {
        overlay.parse("--place-text", &mut args(&[
            "1", "25000", &y.to_string(), &text,
        ]).iter())?;
    }
    overlay.parse("--glyph-variation", &mut args(&[
        "42", "0", "0", "0", "0", "0", "0",
    ]).iter())?;
    let baseline = overlay.expand(&[], None)?;
    overlay.parse("--glyph-word-slant", &mut args(&["5000"]).iter())?;
    let slanted = overlay.expand(&[], None)?;
    // Independent uint32 avalanche/xorshift samples for seed 42, two runs
    // and three exact ASCII-space-delimited words. The expected shifts use
    // lower-baseline tangent shear with Rust's nearest-integer rounding.
    let shifts = [
        (272i64, 132i64), (-250, -121), (122, 59),
        (287, 139), (-214, -104), (-156, -76),
    ];
    let positions = [25_000i64, 29_000, 41_000, 49_000, 53_000];
    for (run_index, row) in slanted.as_chunks::<5>().0.iter().enumerate() {
        let row_y = if run_index == 0 { 30_000u32 } else { 40_000u32 };
        for (index, stroke) in row.iter().enumerate() {
            let word = match index { 0 | 1 => 0, 2 => 1, _ => 2 };
            let (top, bottom) = *shifts.get(run_index * 3 + word)
                .ok_or("missing seeded word fixture")?;
            let x = *positions.get(index)
                .ok_or("missing word position fixture")?;
            let start_x = u32::try_from(x + 600 + top)
                .map_err(|error| error.to_string())?;
            let end_x = u32::try_from(x + 1600 + bottom)
                .map_err(|error| error.to_string())?;
            if stroke.start_um != (start_x, row_y + 1000)
                || stroke.end_um != (end_x, row_y + 2800)
                || stroke.width_um != 250 || stroke.rgb != [0x12, 0x34, 0x56] {
                return Err(format!(
                    "incorrect shared word slant at run {run_index} {index}: \
                     {stroke:?}"
                ));
            }
        }
    }
    if slanted.len() != 10 || slanted == baseline
        || slanted != overlay.expand(&[], None)? {
        return Err("word slant must change and replay ten strokes".into());
    }
    overlay.word_slant_millidegrees = Some(0);
    if baseline != overlay.expand(&[], None)? {
        return Err("zero word slant changed legacy ink".into());
    }
    overlay.word_slant_millidegrees = Some(5_000);
    overlay.word_drift_um = Some(90);
    overlay.word_rise_um = Some(90);
    overlay.line_drift_um = Some(120);
    overlay.word_pressure_percent = Some(20);
    let composed = overlay.expand(&[], None)?;
    overlay.word_slant_millidegrees = Some(0);
    let without_slant = overlay.expand(&[], None)?;
    for (index, (before, after)) in without_slant.iter()
        .zip(&composed).enumerate() {
        let run_index = index.div_euclid(5);
        let word = match index.rem_euclid(5) { 0 | 1 => 0, 2 => 1, _ => 2 };
        let (top, bottom) = *shifts.get(run_index * 3 + word)
                .ok_or("missing seeded word fixture")?;
        if i64::from(after.start_um.0) - i64::from(before.start_um.0)
            != top || i64::from(after.end_um.0)
            - i64::from(before.end_um.0) != bottom
            || after.start_um.1 != before.start_um.1
            || after.end_um.1 != before.end_um.1
            || after.width_um != before.width_um {
            return Err("word shear changed independent rhythm".into());
        }
    }
    overlay.word_slant_millidegrees = Some(5_001);
    if overlay.expand(&[], None).is_ok() {
        return Err("typed out-of-range word slant accepted".into());
    }
    overlay.word_slant_millidegrees = Some(5_000);
    overlay.variation = None;
    if overlay.expand(&[], None).is_ok() {
        return Err("typed unpaired word slant accepted".into());
    }
    Ok(())
}

#[test]
fn seeded_word_slant_keeps_same_pen_directed_word_bridges()
-> Result<(), String> {
    let mut overlay = GlyphOverlay::default();
    overlay.parse("--glyph-stroke", &mut args(&[
        "\u{e1}", "500", "1200", "3300", "1200", "250", "#123456",
    ]).iter())?;
    overlay.parse("--place-text", &mut args(&[
        "1", "25000", "30000", "\u{e1}\u{e1} \u{e1}\u{e1}",
    ]).iter())?;
    overlay.parse("--glyph-variation", &mut args(&[
        "42", "0", "0", "0", "0", "0", "0",
    ]).iter())?;
    overlay.add_text_join_pair(
        &mut args(&["\u{e1}", "\u{e1}"]).iter(),
    )?;
    overlay.parse("--glyph-word-slant", &mut args(&["5000"]).iter())?;
    let joined = overlay.expand(&[], None)?;
    if joined.len() != 6 {
        return Err("word slant lost a selected join".into());
    }
    for offset in [0usize, 3] {
        let [first, bridge, last] = joined.get(offset..offset + 3)
            .ok_or("missing joined word")? else {
            return Err("joined word must contain three lines".into());
        };
        if first.end_um != bridge.start_um
            || bridge.end_um != last.start_um
            || first.width_um != bridge.width_um
            || bridge.width_um != last.width_um
            || bridge.end_um.0 - bridge.start_um.0 != 1_200
            || bridge.start_um.1 != bridge.end_um.1 {
            return Err("shared word slant broke the directed join".into());
        }
    }
    if joined != overlay.expand(&[], None)? {
        return Err("joined slanted word changed on replay".into());
    }
    Ok(())
}

#[test]
fn word_slant_composes_with_statistical_mean_in_physical_engine()
-> Result<(), String> {
    let stats_bytes = execute_handwriting(&args(&[
        "--set-glyph", "\u{e1}", "width", "3500", "3500", "3500",
        "3500", "0",
        "--set-glyph", "\u{e1}", "height", "4500", "4500", "4500",
        "4500", "0",
        "--set-glyph", "\u{e1}", "slant", "3000", "3000", "3000",
        "3000", "0",
    ]))?.ok_or("missing statistical profile")?;
    let statistics = decode_statistics_json(
        &stats_bytes, &UnicodeGraphemeSegmentation,
    ).map_err(|error| format!("statistics: {error:?}"))?;
    let mut overlay = GlyphOverlay::default();
    overlay.parse("--glyph-stroke", &mut args(&[
        "\u{e1}", "600", "1000", "1600", "2800", "250", "#123456",
    ]).iter())?;
    overlay.parse("--place-text", &mut args(&[
        "1", "25000", "30000", "\u{e1}",
    ]).iter())?;
    let default = overlay.expand(&[], Some(&statistics))?;
    overlay.parse("--glyph-variation", &mut args(&[
        "42", "0", "0", "0", "0", "0", "0",
    ]).iter())?;
    overlay.parse("--glyph-word-slant", &mut args(&["5000"]).iter())?;
    let combined = overlay.expand(&[], Some(&statistics))?;
    // Mean 3000 mdeg plus independently seeded 4445 mdeg for run 0,
    // word 0. The core rounds (4500-Y)*tan(7.445 degrees) on each point.
    let original = default.first().ok_or("missing original ink")?;
    let varied = combined.first().ok_or("missing varied ink")?;
    if default.len() != 1 || combined.len() != 1
        || original.start_um != (25_783, 31_000)
        || original.end_um != (26_689, 32_800)
        || varied.start_um != (26_057, 31_000)
        || varied.end_um != (26_822, 32_800)
        || varied.width_um != 250 {
        return Err(format!(
            "word and mean slant did not compose: {default:?} {combined:?}"
        ));
    }
    overlay.word_slant_millidegrees = Some(0);
    if overlay.expand(&[], Some(&statistics))? != default {
        return Err("zero word shear changed mean slant".into());
    }
    Ok(())
}

#[test]
fn word_slant_pdf_png_ink_replay_and_atomic_refusals()
-> Result<(), String> {
    let prefix = args(&[
        "--printed-sheet", "true",
        "--glyph-stroke", "\u{e1}", "600", "1000", "1600", "2800",
        "250", "#123456",
        "--flow-markdown", concat!(
            "\u{e1}\u{e1} \u{e1} \u{e1}\u{e1}\n\n",
            "\u{e1}\u{e1} \u{e1}\u{e1}",
        ),
        "--glyph-variation", "42", "0", "0", "0", "0", "0", "0",
    ]);
    for format in ["pdf", "png", "ink"] {
        let render = |seed: &str, extras: &[&str]|
            -> Result<Vec<u8>, String> {
            let mut command = prefix.clone();
            let at = command.iter().position(|value| {
                value == "--glyph-variation"
            }).ok_or("variation flag missing")? + 1;
            *command.get_mut(at).ok_or("seed missing")? = seed.into();
            if format == "ink" {
                command.push("--emit-ink-file".into());
            } else {
                command.extend(args(&["--export-paper", format]));
            }
            if format == "png" {
                command.extend(args(&["--dpi", "72"]));
            }
            command.extend(args(extras));
            execute_notebook(&command)?.ok_or_else(|| "missing output".into())
        };
        let original = render("42", &[])?;
        if original != render("42", &["--glyph-word-slant", "0"])? {
            return Err(format!("{format} zero word shear changed output"));
        }
        let changed = render("42", &["--glyph-word-slant", "5000"])?;
        if changed == original
            || changed != render("42", &["--glyph-word-slant", "5000"])?
            || changed == render("43", &["--glyph-word-slant", "5000"])? {
            return Err(format!("{format} word-shear replay failed"));
        }
        if format == "pdf" && !changed.starts_with(b"%PDF-1.4") {
            return Err("sheared PDF signature missing".into());
        }
        if format == "png" && !changed.starts_with(b"\x89PNG\r\n\x1a\n") {
            return Err("sheared PNG signature missing".into());
        }
        if format == "ink"
            && !changed.starts_with(b"atrament.page-ink.v1\n") {
            return Err("sheared ink manifest missing".into());
        }
    }
    let target = test_path();
    fs::write(&target, b"accepted bytes").map_err(|error| error.to_string())?;
    let path = target.to_str().ok_or("non-UTF8 output path")?;
    for invalid in [
        args(&["--glyph-word-slant", "-1"]),
        args(&["--glyph-word-slant", "5001"]),
        args(&["--glyph-word-slant", "4294967296"]),
        args(&["--glyph-word-slant", "5000",
            "--glyph-word-slant", "1"]),
    ] {
        let mut command = prefix.clone();
        command.extend(args(&[
            "--export-paper", "pdf", "--output", path, "--replace",
        ]));
        command.extend(invalid);
        if execute_notebook(&command).is_ok()
            || fs::read(&target).map_err(|error| error.to_string())?
                != b"accepted bytes" {
            return Err("invalid word slant replaced output".into());
        }
    }
    let mut unpaired = prefix;
    let at = unpaired.iter().position(|value| {
        value == "--glyph-variation"
    }).ok_or("variation flag missing")?;
    unpaired.drain(at..at + 8).for_each(drop);
    unpaired.extend(args(&[
        "--glyph-word-slant", "5000", "--export-paper", "pdf",
        "--output", path, "--replace",
    ]));
    if execute_notebook(&unpaired).is_ok()
        || fs::read(&target).map_err(|error| error.to_string())?
            != b"accepted bytes" {
        return Err("unpaired word slant replaced output".into());
    }
    fs::remove_file(&target).map_err(|error| error.to_string())?;
    Ok(())
}

#[test]
fn word_slant_refuses_off_page_shear_before_replacing_existing_export()
-> Result<(), String> {
    let prefix = args(&[
        "--printed-sheet", "true",
        "--glyph-stroke", "\u{e1}", "200", "1000", "1600", "2800",
        "250", "#123456",
        "--place-text", "1", "20000", "30000", "\u{e1}",
        "--glyph-variation", "7", "0", "0", "0", "0", "0", "0",
    ]);
    let mut plain = prefix.clone();
    plain.extend(args(&["--export-paper", "pdf"]));
    if !execute_notebook(&plain)?.is_some_and(|bytes| {
        bytes.starts_with(b"%PDF-1.4")
    }) {
        return Err("boundary fixture must render without word shear".into());
    }
    let target = test_path();
    fs::write(&target, b"accepted export").map_err(|error| error.to_string())?;
    let path = target.to_str().ok_or("non-UTF8 output path")?;
    let mut refused = prefix;
    refused.extend(args(&[
        "--glyph-word-slant", "5000", "--export-paper", "pdf",
        "--output", path, "--replace",
    ]));
    let refusal = execute_notebook(&refused)
        .expect_err("seed-7 negative word shear must move ink off-page");
    if !refusal.contains("page 1 ink needs printable endpoints")
        || fs::read(&target).map_err(|error| error.to_string())?
            != b"accepted export" {
        return Err(format!(
            "unprintable word slant changed existing output: {refusal}"
        ));
    }
    fs::remove_file(&target).map_err(|error| error.to_string())?;
    Ok(())
}

#[test]
fn seeded_word_width_shares_letter_scale_and_preserves_zero_bytes()
-> Result<(), String> {
    let glyph = "n\u{303}";
    let mut overlay = GlyphOverlay::default();
    overlay.parse("--glyph-stroke", &mut args(&[
        glyph, "600", "1000", "1600", "2800", "250", "#123456",
    ]).iter())?;
    let text = format!("{glyph}{glyph}  {glyph} {glyph}{glyph}");
    for y in [30_000u32, 40_000u32] {
        overlay.parse("--place-text", &mut args(&[
            "1", "25000", &y.to_string(), &text,
        ]).iter())?;
    }
    overlay.parse("--glyph-variation", &mut args(&[
        "42", "0", "0", "0", "0", "0", "0",
    ]).iter())?;
    let original = overlay.expand(&[], None)?;
    overlay.parse("--glyph-word-width", &mut args(&["15"]).iter())?;
    let widened = overlay.expand(&[], None)?;
    // Independent unsigned avalanche/xorshift samples for two physical
    // runs and three exact words each (seed 42, maximum 15 percent).
    let percent = [107u32, 108, 92, 107, 90, 96];
    let offsets = [0u32, 4_000, 16_000, 24_000, 28_000];
    for (run_index, row) in widened.as_chunks::<5>().0.iter().enumerate() {
        let origin_y = if run_index == 0 { 30_000 } else { 40_000 };
        for (index, stroke) in row.iter().enumerate() {
            let word = match index { 0 | 1 => 0, 2 => 1, _ => 2 };
            let chosen = percent[run_index * 3 + word];
            let target_width = (3_500 * chosen + 50) / 100;
            // The independent physical reference performs both specified
            // integer roundings: word cell then authored local coordinate.
            let scaled = |x: u32| -> u32 {
                (x * target_width + 1_750) / 3_500
            };
            let x = 25_000 + offsets[index];
            if stroke.start_um != (x + scaled(600), origin_y + 1_000)
                || stroke.end_um != (x + scaled(1600), origin_y + 2_800)
                || stroke.width_um != 250 || stroke.rgb != [0x12, 0x34, 0x56] {
                return Err(format!(
                    "word width changed at {run_index} {index}: {stroke:?}"
                ));
            }
        }
    }
    if widened.len() != 10 || widened == original
        || widened != overlay.expand(&[], None)? {
        return Err("word-width seed replay or glyph count changed".into());
    }
    overlay.word_width_percent = Some(0);
    if overlay.expand(&[], None)? != original {
        return Err("zero word width changed previous output".into());
    }
    overlay.word_width_percent = Some(15);
    overlay.word_drift_um = Some(90);
    overlay.word_rise_um = Some(90);
    overlay.line_drift_um = Some(120);
    overlay.word_slant_millidegrees = Some(3_000);
    overlay.word_pressure_percent = Some(15);
    let combined = overlay.expand(&[], None)?;
    overlay.word_width_percent = Some(0);
    let before = overlay.expand(&[], None)?;
    for (index, (old, updated)) in before.iter().zip(&combined).enumerate() {
        let word = match index % 5 { 0 | 1 => 0, 2 => 1, _ => 2 };
        let chosen = percent[index / 5 * 3 + word];
        let width = (3_500 * chosen + 50) / 100;
        let scaled = |x: u32| (x * width + 1_750) / 3_500;
        if i64::from(updated.start_um.0) - i64::from(old.start_um.0)
            != i64::from(scaled(600)) - 600
            || i64::from(updated.end_um.0) - i64::from(old.end_um.0)
                != i64::from(scaled(1600)) - 1600
            || updated.start_um.1 != old.start_um.1
            || updated.end_um.1 != old.end_um.1
            || updated.width_um != old.width_um
            || updated.rgb != old.rgb {
            return Err("word width changed independent ink rhythms".into());
        }
    }
    overlay.word_width_percent = Some(16);
    if overlay.expand(&[], None).is_ok() {
        return Err("typed out-of-range word width accepted".into());
    }
    overlay.word_width_percent = Some(15);
    overlay.variation = None;
    if overlay.expand(&[], None).is_ok() {
        return Err("typed unpaired word width accepted".into());
    }
    Ok(())
}

#[test]
fn word_width_keeps_joined_words_continuous_and_mean_scaling_explicit()
-> Result<(), String> {
    let mut overlay = GlyphOverlay::default();
    overlay.parse("--glyph-stroke", &mut args(&[
        "\u{e1}", "150", "1200", "3370", "1200", "250", "#123456",
    ]).iter())?;
    overlay.parse("--place-text", &mut args(&[
        "1", "25000", "30000", "\u{e1}\u{e1} \u{e1}\u{e1}",
    ]).iter())?;
    overlay.parse("--glyph-variation", &mut args(&[
        "42", "0", "0", "0", "0", "0", "0",
    ]).iter())?;
    overlay.add_text_join_pair(
        &mut args(&["\u{e1}", "\u{e1}"]).iter(),
    )?;
    overlay.parse("--glyph-word-width", &mut args(&["15"]).iter())?;
    let joined = overlay.expand(&[], None)?;
    if joined.len() != 6 {
        return Err(format!("width variation lost directed joins: {joined:?}"));
    }
    for offset in [0usize, 3] {
        let [first, bridge, last] = joined.get(offset..offset + 3)
            .ok_or("missing joined word")? else {
            return Err("joined word must have exactly three strokes".into());
        };
        if first.end_um != bridge.start_um
            || bridge.end_um != last.start_um
            || first.width_um != bridge.width_um
            || bridge.width_um != last.width_um
            || bridge.start_um.1 != bridge.end_um.1
            || bridge.end_um.0 <= bridge.start_um.0
            || bridge.end_um.0 - bridge.start_um.0 > 1_200 {
            return Err(format!("scaled word join changed pen: {joined:?}"));
        }
    }

    let profile = execute_handwriting(&args(&[
        "--set-glyph", "\u{e1}", "width", "3900", "3900",
        "3900", "3900", "0",
    ]))?.ok_or("missing statistical profile")?;
    let statistics = decode_statistics_json(
        &profile, &UnicodeGraphemeSegmentation,
    ).map_err(|error| format!("statistics: {error:?}"))?;
    overlay.text_joins.clear();
    overlay.runs.clear();
    overlay.parse("--place-text", &mut args(&[
        "1", "25000", "30000", "\u{e1}",
    ]).iter())?;
    let scaled = overlay.expand(&[], Some(&statistics))?;
    let stroke = scaled.first().ok_or("mean and width must render")?;
    // Target cell 3900 * 107%, rounded, is 4173 um. The mean
    // projector independently rounds each authored local X coordinate.
    if scaled.len() != 1
        || stroke.start_um != (25_179, 30_800)
        || stroke.end_um != (29_018, 30_800)
        || stroke.width_um != 250 {
        return Err(format!("word width did not compose with mean: {scaled:?}"));
    }
    overlay.word_width_percent = Some(0);
    let baseline = overlay.expand(&[], Some(&statistics))?;
    if baseline.len() != 1
        || baseline[0].start_um != (25_167, 30_800)
        || baseline[0].end_um != (28_755, 30_800) {
        return Err(format!("zero word width modified mean: {baseline:?}"));
    }
    Ok(())
}

#[test]
fn word_width_pdf_png_ink_replay_and_atomic_option_refusals()
-> Result<(), String> {
    let prefix = args(&[
        "--printed-sheet", "true",
        "--glyph-stroke", "\u{e1}", "600", "1000", "1600", "2800",
        "250", "#123456",
        "--flow-markdown", concat!(
            "\u{e1}\u{e1} \u{e1} \u{e1}\u{e1}\n\n",
            "\u{e1}\u{e1} \u{e1}\u{e1}",
        ),
        "--glyph-variation", "42", "0", "0", "0", "0", "0", "0",
    ]);
    for format in ["pdf", "png", "ink"] {
        let render = |seed: &str, extras: &[&str]|
            -> Result<Vec<u8>, String> {
            let mut command = prefix.clone();
            let seed_index = command.iter()
                .position(|value| value == "--glyph-variation")
                .ok_or("variation flag missing")? + 1;
            *command.get_mut(seed_index).ok_or("seed missing")? = seed.into();
            if format == "ink" {
                command.push("--emit-ink-file".into());
            } else {
                command.extend(args(&["--export-paper", format]));
            }
            if format == "png" {
                command.extend(args(&["--dpi", "72"]));
            }
            command.extend(args(extras));
            execute_notebook(&command)?.ok_or("missing native output".into())
        };
        let original = render("42", &[])?;
        if original != render("42", &["--glyph-word-width", "0"])? {
            return Err(format!("{format} zero word width changed bytes"));
        }
        let changed = render("42", &["--glyph-word-width", "15"])?;
        if changed == original
            || changed != render("42", &["--glyph-word-width", "15"])?
            || changed == render("43", &["--glyph-word-width", "15"])? {
            return Err(format!("{format} width replay/seed failed"));
        }
        let signature = match format {
            "pdf" => b"%PDF-1.4".as_slice(),
            "png" => b"\x89PNG\r\n\x1a\n".as_slice(),
            _ => b"atrament.page-ink.v1\n".as_slice(),
        };
        if !changed.starts_with(signature) {
            return Err(format!("{format} native output header missing"));
        }
    }

    let target = test_path();
    fs::write(&target, b"accepted export").map_err(|e| e.to_string())?;
    let path = target.to_str().ok_or("non-UTF8 path")?;
    for invalid in [
        args(&["--glyph-word-width", "-1"]),
        args(&["--glyph-word-width", "16"]),
        args(&["--glyph-word-width", "4294967296"]),
        args(&["--glyph-word-width", "15",
            "--glyph-word-width", "1"]),
    ] {
        let mut command = prefix.clone();
        command.extend(args(&[
            "--export-paper", "pdf", "--output", path, "--replace",
        ]));
        command.extend(invalid);
        if execute_notebook(&command).is_ok()
            || fs::read(&target).map_err(|e| e.to_string())?
                != b"accepted export" {
            return Err("invalid width flag changed accepted output".into());
        }
    }
    let mut unpaired = prefix;
    let at = unpaired.iter().position(|value| value == "--glyph-variation")
        .ok_or("missing variation")?;
    unpaired.drain(at..at + 8).for_each(drop);
    unpaired.extend(args(&[
        "--glyph-word-width", "15", "--export-paper", "pdf",
        "--output", path, "--replace",
    ]));
    if execute_notebook(&unpaired).is_ok()
        || fs::read(&target).map_err(|e| e.to_string())?
            != b"accepted export" {
        return Err("unpaired word width changed accepted output".into());
    }
    fs::remove_file(&target).map_err(|e| e.to_string())?;
    Ok(())
}

#[test]
fn word_width_refuses_off_printable_region_without_replacing_pdf_or_png()
-> Result<(), String> {
    // A4's writable right edge is 196000 um. At this position, the
    // original 1600um endpoint and 125um half-pen fit; seed 42 widens
    // the same first word to 107%, crossing the printable edge.
    let prefix = args(&[
        "--printed-sheet", "true",
        "--glyph-stroke", "\u{e1}", "600", "1000", "1600", "2800",
        "250", "#123456",
        "--place-text", "1", "194200", "30000", "\u{e1}",
        "--glyph-variation", "42", "0", "0", "0", "0", "0", "0",
    ]);
    let target = test_path();
    let path = target.to_str().ok_or("non-UTF8 output path")?;
    for format in ["pdf", "png"] {
        let mut valid = prefix.clone();
        valid.extend(args(&["--export-paper", format]));
        if format == "png" {
            valid.extend(args(&["--dpi", "72"]));
        }
        let output = execute_notebook(&valid)?.ok_or("missing baseline bytes")?;
        if !output.starts_with(if format == "pdf" {
            b"%PDF-1.4"
        } else {
            b"\x89PNG\r\n\x1a\n"
        }) {
            return Err("baseline near-margin export is invalid".into());
        }
        fs::write(&target, b"accepted export")
            .map_err(|error| error.to_string())?;
        let mut invalid = prefix.clone();
        invalid.extend(args(&[
            "--glyph-word-width", "15", "--export-paper", format,
            "--output", path, "--replace",
        ]));
        if format == "png" {
            invalid.extend(args(&["--dpi", "72"]));
        }
        let refused = execute_notebook(&invalid)
            .expect_err("word width should cross writable boundary");
        if !refused.contains("page 1 ink needs printable endpoints")
            || fs::read(&target).map_err(|error| error.to_string())?
                != b"accepted export" {
            return Err(format!("word width clipped or overwrote: {refused}"));
        }
    }
    fs::remove_file(&target).map_err(|error| error.to_string())?;
    Ok(())
}

#[test]
fn seeded_word_height_scales_unicode_words_around_the_lower_baseline()
-> Result<(), String> {
    let glyph = "n\u{303}";
    let mut overlay = GlyphOverlay::default();
    overlay.parse("--glyph-stroke", &mut args(&[
        glyph, "600", "1000", "1600", "2800", "250", "#123456",
    ]).iter())?;
    let line = format!("{glyph}{glyph}  {glyph} {glyph}{glyph}");
    for y in [30_000u32, 40_000u32] {
        overlay.parse("--place-text", &mut args(&[
            "1", "25000", &y.to_string(), &line,
        ]).iter())?;
    }
    overlay.parse("--glyph-variation", &mut args(&[
        "42", "0", "0", "0", "0", "0", "0",
    ]).iter())?;
    let original = overlay.expand(&[], None)?;
    overlay.parse("--glyph-word-height", &mut args(&["15"]).iter())?;
    let varied = overlay.expand(&[], None)?;
    // Independently fixed xorshift percentages for exact UTF-8 NFD words.
    // Spaces reset the word; two repeated spaces do not create a new word.
    let percent = [89i64, 101, 102, 93, 93, 99];
    let offsets = [0u32, 4_000, 16_000, 24_000, 28_000];
    if varied.len() != 10 || varied == original
        || varied != overlay.expand(&[], None)? {
        return Err("word height must replay ten distinct strokes".into());
    }
    for (run, row) in varied.chunks_exact(5).enumerate() {
        let origin_y = if run == 0 { 30_000i64 } else { 40_000i64 };
        for (index, stroke) in row.iter().enumerate() {
            let word = match index { 0 | 1 => 0, 2 => 1, _ => 2 };
            let chosen = percent[run * 3 + word];
            let height = (4_500 * chosen + 50) / 100;
            let place = |local_y: i64| -> Result<u32, String> {
                u32::try_from(origin_y + 4_500 - height
                    + (local_y * height + 2_250) / 4_500)
                    .map_err(|error| error.to_string())
            };
            let x = 25_000 + offsets[index];
            if stroke.start_um != (x + 600, place(1_000)?)
                || stroke.end_um != (x + 1600, place(2_800)?)
                || stroke.width_um != 250 || stroke.rgb != [0x12, 0x34, 0x56] {
                return Err(format!(
                    "wrong seeded height in run {run}, glyph {index}: \
                     {stroke:?}"
                ));
            }
        }
    }
    overlay.word_height_percent = Some(0);
    if overlay.expand(&[], None)? != original {
        return Err("zero word height changed legacy ink".into());
    }
    overlay.word_height_percent = Some(15);
    overlay.word_drift_um = Some(90);
    overlay.word_rise_um = Some(90);
    overlay.line_drift_um = Some(120);
    overlay.word_width_percent = Some(12);
    overlay.word_pressure_percent = Some(15);
    let combined = overlay.expand(&[], None)?;
    overlay.word_height_percent = Some(0);
    let without_height = overlay.expand(&[], None)?;
    for (index, (before, after)) in without_height.iter().zip(&combined)
        .enumerate() {
        let word = match index % 5 { 0 | 1 => 0, 2 => 1, _ => 2 };
        let height = (4_500 * percent[index / 5 * 3 + word] + 50) / 100;
        let delta = |local_y: i64| (4_500 - height)
            + (local_y * height + 2_250) / 4_500 - local_y;
        if i64::from(after.start_um.1) - i64::from(before.start_um.1)
            != delta(1_000)
            || i64::from(after.end_um.1) - i64::from(before.end_um.1)
                != delta(2_800)
            || after.start_um.0 != before.start_um.0
            || after.end_um.0 != before.end_um.0
            || after.width_um != before.width_um || after.rgb != before.rgb {
            return Err("word height modified independent word rhythms".into());
        }
    }
    overlay.word_height_percent = Some(16);
    if overlay.expand(&[], None).is_ok() {
        return Err("typed out-of-range word height accepted".into());
    }
    overlay.word_height_percent = Some(15);
    overlay.variation = None;
    if overlay.expand(&[], None).is_ok() {
        return Err("typed unpaired word height accepted".into());
    }
    Ok(())
}

#[test]
fn seeded_word_height_preserves_joins_and_statistical_mean_baselines()
-> Result<(), String> {
    let mut overlay = GlyphOverlay::default();
    overlay.parse("--glyph-stroke", &mut args(&[
        "á", "150", "1200", "3370", "1200", "250", "#123456",
    ]).iter())?;
    overlay.parse("--place-text", &mut args(&[
        "1", "25000", "30000", "áá áá",
    ]).iter())?;
    overlay.parse("--glyph-variation", &mut args(&[
        "42", "0", "0", "0", "0", "0", "0",
    ]).iter())?;
    overlay.add_text_join_pair(&mut args(&["á", "á"]).iter())?;
    overlay.parse("--glyph-word-height", &mut args(&["15"]).iter())?;
    let joined = overlay.expand(&[], None)?;
    if joined.len() != 6 {
        return Err(format!("word height lost directed joins: {joined:?}"));
    }
    for start in [0usize, 3] {
        let [first, bridge, second] = joined.get(start..start + 3)
            .ok_or("missing joined word")? else {
            return Err("joined word must have three segments".into());
        };
        if first.end_um != bridge.start_um
            || bridge.end_um != second.start_um
            || first.width_um != bridge.width_um
            || bridge.width_um != second.width_um
            || bridge.start_um.1 != bridge.end_um.1
            || bridge.end_um.0 <= bridge.start_um.0
            || bridge.end_um.0 - bridge.start_um.0 > 1_200 {
            return Err(format!("word height changed joins: {joined:?}"));
        }
    }
    let profile = execute_handwriting(&args(&[
        "--set-glyph", "á", "height", "5000", "5000", "5000",
        "5000", "0", "--set-glyph", "á", "width", "3500",
        "3500", "3500", "3500", "0",
    ]))?.ok_or("missing statistical profile")?;
    let statistics = decode_statistics_json(
        &profile, &UnicodeGraphemeSegmentation,
    ).map_err(|error| format!("statistics: {error:?}"))?;
    overlay.runs.clear();
    overlay.text_joins.clear();
    overlay.parse("--place-text", &mut args(&[
        "1", "25000", "30000", "á",
    ]).iter())?;
    let mean = overlay.expand(&[], Some(&statistics))?;
    if mean.len() != 1
        || mean[0].start_um != (25_150, 31_737)
        || mean[0].end_um != (28_370, 31_737) {
        return Err(format!("word height and mean disagreed: {mean:?}"));
    }
    overlay.word_height_percent = Some(0);
    let baseline = overlay.expand(&[], Some(&statistics))?;
    if baseline.len() != 1
        || baseline[0].start_um != (25_150, 31_333)
        || baseline[0].end_um != (28_370, 31_333) {
        return Err(format!("zero word height changed mean: {baseline:?}"));
    }
    Ok(())
}

#[test]
fn word_height_pdf_png_and_page_ink_replay_with_atomic_option_refusals()
-> Result<(), String> {
    let prefix = args(&[
        "--printed-sheet", "true",
        "--glyph-stroke", "á", "600", "1000", "1600", "2800",
        "250", "#123456",
        "--flow-markdown", "áá á áá\n\náá áá",
        "--glyph-variation", "42", "0", "0", "0", "0", "0", "0",
    ]);
    let render = |format: &str, seed: &str, extras: &[&str]|
        -> Result<Vec<u8>, String> {
        let mut command = prefix.clone();
        let position = command.iter().position(|v| v == "--glyph-variation")
            .ok_or("missing variation seed")? + 1;
        *command.get_mut(position).ok_or("seed missing")? = seed.into();
        if format == "ink" {
            command.push("--emit-ink-file".into());
        } else {
            command.extend(args(&["--export-paper", format]));
        }
        if format == "png" {
            command.extend(args(&["--dpi", "72"]));
        }
        command.extend(args(extras));
        execute_notebook(&command)?.ok_or("native export missing".into())
    };
    for format in ["pdf", "png", "ink"] {
        let base = render(format, "42", &[])?;
        if base != render(format, "42", &["--glyph-word-height", "0"])? {
            return Err(format!("zero height changed {format} bytes"));
        }
        let altered = render(format, "42", &["--glyph-word-height", "15"])?;
        if altered == base
            || altered != render(format, "42", &["--glyph-word-height", "15"])?
            || altered == render(
                format, "43", &["--glyph-word-height", "15"],
            )? {
            return Err(format!("word height {format} replay failed"));
        }
        let signature = match format {
            "pdf" => b"%PDF-1.4".as_slice(),
            "png" => b"\x89PNG\r\n\x1a\n".as_slice(),
            _ => b"atrament.page-ink.v1\n".as_slice(),
        };
        if !altered.starts_with(signature) {
            return Err(format!("word height {format} signature missing"));
        }
    }
    let source = test_path().with_extension("tsv");
    let ink = render("ink", "42", &["--glyph-word-height", "15"])?;
    fs::write(&source, &ink).map_err(|error| error.to_string())?;
    let source_name = source.to_str().ok_or("non-UTF8 ink path")?;
    for format in ["pdf", "png"] {
        let mut replay = args(&[
            "--printed-sheet", "true", "--ink-file", source_name,
            "--export-paper", format,
        ]);
        if format == "png" {
            replay.extend(args(&["--dpi", "72"]));
        }
        if execute_notebook(&replay)?
            != Some(render(format, "42", &["--glyph-word-height", "15"])?) {
            return Err(format!("word height {format} differs from TSV replay"));
        }
    }
    fs::remove_file(source).map_err(|error| error.to_string())?;

    let target = test_path();
    fs::write(&target, b"preserved PDF").map_err(|error| error.to_string())?;
    let name = target.to_str().ok_or("non-UTF8 output path")?;
    for invalid in [
        args(&["--glyph-word-height", "-1"]),
        args(&["--glyph-word-height", "16"]),
        args(&["--glyph-word-height", "4294967296"]),
        args(&["--glyph-word-height", "15",
            "--glyph-word-height", "1"]),
    ] {
        let mut command = prefix.clone();
        command.extend(args(&[
            "--export-paper", "pdf", "--output", name, "--replace",
        ]));
        command.extend(invalid);
        if execute_notebook(&command).is_ok()
            || fs::read(&target).map_err(|error| error.to_string())?
                != b"preserved PDF" {
            return Err("invalid height options overwrote PDF".into());
        }
    }
    let mut unpaired = prefix;
    let index = unpaired.iter().position(|v| v == "--glyph-variation")
        .ok_or("missing variation")?;
    unpaired.drain(index..index + 8).for_each(drop);
    unpaired.extend(args(&[
        "--glyph-word-height", "15", "--export-paper", "pdf",
        "--output", name, "--replace",
    ]));
    if execute_notebook(&unpaired).is_ok()
        || fs::read(&target).map_err(|error| error.to_string())?
            != b"preserved PDF" {
        return Err("unpaired word height overwrote PDF".into());
    }
    fs::remove_file(target).map_err(|error| error.to_string())?;
    Ok(())
}

#[test]
fn word_height_refuses_top_margin_clipping_without_changing_exports()
-> Result<(), String> {
    // A4 top margin is 22 mm. The 125-um half-pen clears it at 22150 um;
    // seed 48 chooses 115% height, moving this upper vertex 525 um up.
    let prefix = args(&[
        "--printed-sheet", "true",
        "--glyph-stroke", "á", "600", "1000", "1600", "2800",
        "250", "#123456",
        "--place-text", "1", "25000", "21150", "á",
        "--glyph-variation", "48", "0", "0", "0", "0", "0", "0",
    ]);
    let destination = test_path();
    let name = destination.to_str().ok_or("non-UTF8 output path")?;
    for format in ["pdf", "png"] {
        let mut original = prefix.clone();
        original.extend(args(&["--export-paper", format]));
        if format == "png" {
            original.extend(args(&["--dpi", "72"]));
        }
        if execute_notebook(&original)?.is_none() {
            return Err("valid near-top baseline has no ink export".into());
        }
        fs::write(&destination, b"accepted export")
            .map_err(|error| error.to_string())?;
        let mut invalid = original;
        invalid.extend(args(&[
            "--glyph-word-height", "15", "--output", name, "--replace",
        ]));
        let refused = execute_notebook(&invalid)
            .expect_err("higher word must leave printable region");
        if !refused.contains("page 1 ink needs printable endpoints")
            || fs::read(&destination).map_err(|error| error.to_string())?
                != b"accepted export" {
            return Err(format!(
                "clipped word height overwrote {format}: {refused}"
            ));
        }
    }
    fs::remove_file(destination).map_err(|error| error.to_string())?;
    Ok(())
}

#[test]
fn word_tracking_replays_exact_unicode_positions_and_resets_at_spaces()
-> Result<(), String> {
    let glyph = "n\u{303}";
    let mut overlay = GlyphOverlay::default();
    overlay.parse("--glyph-stroke", &mut args(&[
        glyph, "600", "1000", "1600", "2800", "250", "#123456",
    ]).iter())?;
    let line = format!("{glyph}{glyph}  {glyph} {glyph}{glyph}");
    for y in [30_000u32, 40_000u32] {
        overlay.parse("--place-text", &mut args(&[
            "1", "25000", &y.to_string(), &line,
        ]).iter())?;
    }
    overlay.parse("--glyph-variation", &mut args(&[
        "42", "0", "0", "0", "0", "0", "0",
    ]).iter())?;
    let original = overlay.expand(&[], None)?;
    overlay.parse("--glyph-word-tracking", &mut args(&["150"]).iter())?;
    let changed = overlay.expand(&[], None)?;
    // Independent avalanche/xorshift seed oracle for three words per run.
    // Consecutive glyphs in one word take the same signed 1-column step.
    let steps = [89i64, 36, -76, -9, -91, -133];
    let advances = [0u32, 4_000, 16_000, 24_000, 28_000];
    let position = [0i64, 1, 0, 0, 1];
    if changed.len() != 10 || original == changed
        || changed != overlay.expand(&[], None)? {
        return Err("word tracking did not deterministically place ink".into());
    }
    for (run, row) in changed.chunks_exact(5).enumerate() {
        let y = if run == 0 { 30_000u32 } else { 40_000u32 };
        for (index, stroke) in row.iter().enumerate() {
            let word = match index { 0 | 1 => 0, 2 => 1, _ => 2 };
            let drift = steps[run * 3 + word] * position[index];
            let base = i64::from(25_000 + advances[index]) + drift;
            let expected_start = (
                u32::try_from(base + 600).map_err(|e| e.to_string())?,
                y + 1_000,
            );
            let expected_end = (
                u32::try_from(base + 1_600).map_err(|e| e.to_string())?,
                y + 2_800,
            );
            if stroke.start_um != expected_start
                || stroke.end_um != expected_end
                || stroke.width_um != 250 || stroke.rgb != [0x12, 0x34, 0x56] {
                return Err(format!(
                    "word tracking moved wrong glyph {run}/{index}: {stroke:?}"
                ));
            }
        }
    }
    overlay.word_tracking_um = Some(0);
    if overlay.expand(&[], None)? != original {
        return Err("zero tracking changed legacy ink".into());
    }
    overlay.word_tracking_um = Some(150);
    overlay.word_drift_um = Some(90);
    overlay.word_rise_um = Some(90);
    overlay.line_drift_um = Some(120);
    overlay.word_width_percent = Some(12);
    overlay.word_height_percent = Some(12);
    overlay.word_slant_millidegrees = Some(3_000);
    overlay.word_pressure_percent = Some(15);
    let combined = overlay.expand(&[], None)?;
    overlay.word_tracking_um = Some(0);
    let without = overlay.expand(&[], None)?;
    for (index, (baseline, tracked)) in without.iter().zip(&combined)
        .enumerate() {
        let run = index / 5;
        let word = match index % 5 { 0 | 1 => 0, 2 => 1, _ => 2 };
        let delta = steps[run * 3 + word] * position[index % 5];
        if i64::from(tracked.start_um.0)
            - i64::from(baseline.start_um.0) != delta
            || i64::from(tracked.end_um.0)
                - i64::from(baseline.end_um.0) != delta
            || tracked.start_um.1 != baseline.start_um.1
            || tracked.end_um.1 != baseline.end_um.1
            || tracked.width_um != baseline.width_um
            || tracked.rgb != baseline.rgb {
            return Err("tracking changed independent word treatments".into());
        }
    }
    overlay.word_tracking_um = Some(151);
    if overlay.expand(&[], None).is_ok() {
        return Err("typed out-of-range tracking was accepted".into());
    }
    overlay.word_tracking_um = Some(150);
    overlay.variation = None;
    if overlay.expand(&[], None).is_ok() {
        return Err("typed unpaired tracking was accepted".into());
    }
    Ok(())
}

#[test]
fn word_tracking_preserves_directed_joins_and_statistical_mean_spacing()
-> Result<(), String> {
    let mut overlay = GlyphOverlay::default();
    overlay.parse("--glyph-stroke", &mut args(&[
        "á", "150", "1200", "3370", "1200", "250", "#123456",
    ]).iter())?;
    overlay.parse("--place-text", &mut args(&[
        "1", "25000", "30000", "áá áá",
    ]).iter())?;
    overlay.parse("--glyph-variation", &mut args(&[
        "42", "0", "0", "0", "0", "0", "0",
    ]).iter())?;
    overlay.add_text_join_pair(&mut args(&["á", "á"]).iter())?;
    overlay.parse("--glyph-word-tracking", &mut args(&["150"]).iter())?;
    let joined = overlay.expand(&[], None)?;
    if joined.len() != 6 {
        return Err(format!(
            "word tracking dropped joined segments: {joined:?}"
        ));
    }
    for offset in [0usize, 3] {
        let [left, bridge, right] = joined.get(offset..offset + 3)
            .ok_or("missing directed join")? else {
            return Err("expected three strokes per joined word".into());
        };
        if left.end_um != bridge.start_um
            || bridge.end_um != right.start_um
            || bridge.width_um != left.width_um
            || bridge.width_um != right.width_um
            || bridge.start_um.1 != bridge.end_um.1
            || bridge.end_um.0 <= bridge.start_um.0
            || bridge.end_um.0 - bridge.start_um.0 > 1_200 {
            return Err(format!(
                "word tracking changed directed pen: {joined:?}"
            ));
        }
    }
    if joined[2].start_um.0 != 29_239
        || joined[5].start_um.0 != 41_186 {
        return Err(format!("wrong tracking join endpoints: {joined:?}"));
    }

    let profile = execute_handwriting(&args(&[
        "--set-glyph", "á", "width", "3900", "3900", "3900",
        "3900", "0",
    ]))?.ok_or("missing statistical profile")?;
    let statistics = decode_statistics_json(
        &profile, &UnicodeGraphemeSegmentation,
    ).map_err(|error| format!("statistics: {error:?}"))?;
    overlay.text_joins.clear();
    overlay.runs.clear();
    overlay.parse("--place-text", &mut args(&[
        "1", "25000", "30000", "áá",
    ]).iter())?;
    let tracked = overlay.expand(&[], Some(&statistics))?;
    overlay.word_tracking_um = Some(0);
    let baseline = overlay.expand(&[], Some(&statistics))?;
    if baseline.len() != 2 || tracked.len() != 2
        || baseline[0] != tracked[0]
        || i64::from(tracked[1].start_um.0)
            - i64::from(baseline[1].start_um.0) != 89
        || i64::from(tracked[1].end_um.0)
            - i64::from(baseline[1].end_um.0) != 89
        || tracked[1].start_um.1 != baseline[1].start_um.1
        || tracked[1].end_um.1 != baseline[1].end_um.1
        || tracked[1].width_um != baseline[1].width_um {
        return Err(format!("tracking violated statistical means: {tracked:?}"));
    }
    Ok(())
}

#[test]
fn word_tracking_pdf_png_and_ink_replay_with_atomic_option_refusal()
-> Result<(), String> {
    let prefix = args(&[
        "--printed-sheet", "true",
        "--glyph-stroke", "á", "600", "1000", "1600", "2800",
        "250", "#123456",
        "--flow-markdown", "áá á áá\n\náá áá",
        "--glyph-variation", "42", "0", "0", "0", "0", "0", "0",
    ]);
    let render = |format: &str, seed: &str, extras: &[&str]|
        -> Result<Vec<u8>, String> {
        let mut command = prefix.clone();
        let position = command.iter().position(|v| v == "--glyph-variation")
            .ok_or("missing variation seed")? + 1;
        *command.get_mut(position).ok_or("missing seed")? = seed.into();
        if format == "ink" {
            command.push("--emit-ink-file".into());
        } else {
            command.extend(args(&["--export-paper", format]));
        }
        if format == "png" {
            command.extend(args(&["--dpi", "72"]));
        }
        command.extend(args(extras));
        execute_notebook(&command)?.ok_or("missing native export".into())
    };
    for format in ["pdf", "png", "ink"] {
        let original = render(format, "42", &[])?;
        if original != render(format, "42", &[
            "--glyph-word-tracking", "0",
        ])? {
            return Err(format!("zero tracking changed {format} bytes"));
        }
        let tracked = render(format, "42", &[
            "--glyph-word-tracking", "150",
        ])?;
        if tracked == original
            || tracked != render(format, "42", &[
                "--glyph-word-tracking", "150",
            ])?
            || tracked == render(format, "43", &[
                "--glyph-word-tracking", "150",
            ])? {
            return Err(format!("word tracking {format} replay failed"));
        }
        let signature = match format {
            "pdf" => b"%PDF-1.4".as_slice(),
            "png" => b"\x89PNG\r\n\x1a\n".as_slice(),
            _ => b"atrament.page-ink.v1\n".as_slice(),
        };
        if !tracked.starts_with(signature) {
            return Err(format!("missing {format} native signature"));
        }
    }
    let ink = render("ink", "42", &["--glyph-word-tracking", "150"])?;
    let source = test_path().with_extension("tsv");
    fs::write(&source, ink).map_err(|error| error.to_string())?;
    let source_name = source.to_str().ok_or("non-UTF8 source path")?;
    for format in ["pdf", "png"] {
        let mut command = args(&[
            "--printed-sheet", "true", "--ink-file", source_name,
            "--export-paper", format,
        ]);
        if format == "png" {
            command.extend(args(&["--dpi", "72"]));
        }
        if execute_notebook(&command)? != Some(render(
            format, "42", &["--glyph-word-tracking", "150"],
        )?) {
            return Err(format!("tracking {format} changed TSV replay"));
        }
    }
    fs::remove_file(source).map_err(|error| error.to_string())?;

    let target = test_path();
    fs::write(&target, b"preserved ink").map_err(|error| error.to_string())?;
    let name = target.to_str().ok_or("non-UTF8 target path")?;
    for invalid in [
        args(&["--glyph-word-tracking", "-1"]),
        args(&["--glyph-word-tracking", "151"]),
        args(&["--glyph-word-tracking", "4294967296"]),
        args(&["--glyph-word-tracking", "150",
            "--glyph-word-tracking", "1"]),
    ] {
        let mut command = prefix.clone();
        command.extend(args(&[
            "--export-paper", "pdf", "--output", name, "--replace",
        ]));
        command.extend(invalid);
        if execute_notebook(&command).is_ok()
            || fs::read(&target).map_err(|error| error.to_string())?
                != b"preserved ink" {
            return Err("invalid tracking replaced accepted file".into());
        }
    }
    let mut unpaired = prefix;
    let index = unpaired.iter().position(|v| v == "--glyph-variation")
        .ok_or("missing variation")?;
    unpaired.drain(index..index + 8).for_each(drop);
    unpaired.extend(args(&[
        "--glyph-word-tracking", "150", "--export-paper", "pdf",
        "--output", name, "--replace",
    ]));
    if execute_notebook(&unpaired).is_ok()
        || fs::read(&target).map_err(|error| error.to_string())?
            != b"preserved ink" {
        return Err("unpaired tracking replaced accepted file".into());
    }
    fs::remove_file(target).map_err(|error| error.to_string())?;
    Ok(())
}

#[test]
fn word_tracking_refuses_right_margin_clipping_in_pdf_and_png()
-> Result<(), String> {
    // A4 right printable edge is 196 mm. The baseline right endpoint
    // 195800 um clears a 125-um half-pen; seed 42 adds 89 um at glyph 2.
    let prefix = args(&[
        "--printed-sheet", "true",
        "--glyph-stroke", "á", "600", "1000", "1600", "2800",
        "250", "#123456",
        "--place-text", "1", "190200", "30000", "áá",
        "--glyph-variation", "42", "0", "0", "0", "0", "0", "0",
    ]);
    let target = test_path();
    let name = target.to_str().ok_or("non-UTF8 target path")?;
    for format in ["pdf", "png"] {
        let mut valid = prefix.clone();
        valid.extend(args(&["--export-paper", format]));
        if format == "png" {
            valid.extend(args(&["--dpi", "72"]));
        }
        if execute_notebook(&valid)?.is_none() {
            return Err("original right-margin stroke did not export".into());
        }
        fs::write(&target, b"preserved output")
            .map_err(|error| error.to_string())?;
        valid.extend(args(&[
            "--glyph-word-tracking", "150", "--output", name, "--replace",
        ]));
        let error = execute_notebook(&valid)
            .expect_err("tracked ink should cross right printable margin");
        if !error.contains("page 1 ink needs printable endpoints")
            || fs::read(&target).map_err(|e| e.to_string())?
                != b"preserved output" {
            return Err(format!(
                "word tracking incorrectly accepted {format}: {error}"
            ));
        }
    }
    fs::remove_file(target).map_err(|error| error.to_string())?;
    Ok(())
}

#[test]
fn word_baseline_step_replays_exact_unicode_words_and_composes_independently()
-> Result<(), String> {
    let glyph = "n\u{303}";
    let mut overlay = GlyphOverlay::default();
    overlay.parse("--glyph-stroke", &mut args(&[
        glyph, "600", "1000", "1600", "2800", "250", "#123456",
    ]).iter())?;
    let text = format!("{glyph}{glyph}  {glyph} {glyph}{glyph}");
    for y in [30_000u32, 40_000u32] {
        overlay.parse("--place-text", &mut args(&[
            "1", "25000", &y.to_string(), &text,
        ]).iter())?;
    }
    overlay.parse("--glyph-variation", &mut args(&[
        "42", "0", "0", "0", "0", "0", "0",
    ]).iter())?;
    let original = overlay.expand(&[], None)?;
    overlay.parse("--glyph-word-baseline-step",
        &mut args(&["150"]).iter())?;
    let changed = overlay.expand(&[], None)?;
    // Independent uint32 avalanche/xorshift oracle. Repeated ASCII
    // spaces cause one boundary, and runs draw different word samples.
    let steps = [122i64, 142, -62, -149, 129, 81];
    let ordinal = [0i64, 1, 0, 0, 1];
    if changed.len() != 10 || original == changed
        || changed != overlay.expand(&[], None)? {
        return Err("word baseline step did not deterministically draw".into());
    }
    for (run, row) in changed.chunks_exact(5).enumerate() {
        let y = if run == 0 { 30_000i64 } else { 40_000i64 };
        for (index, stroke) in row.iter().enumerate() {
            let word = match index { 0 | 1 => 0, 2 => 1, _ => 2 };
            let shift = steps[run * 3 + word] * ordinal[index];
            let expected_y = u32::try_from(y + 1_000 + shift)
                .map_err(|error| error.to_string())?;
            let expected_end_y = u32::try_from(y + 2_800 + shift)
                .map_err(|error| error.to_string())?;
            if stroke.start_um.1 != expected_y
                || stroke.end_um.1 != expected_end_y
                || stroke.start_um.0 != original[run * 5 + index].start_um.0
                || stroke.end_um.0 != original[run * 5 + index].end_um.0
                || stroke.width_um != 250
                || stroke.rgb != [0x12, 0x34, 0x56] {
                return Err(format!(
                    "word baseline step ink mismatch {run}/{index}: {stroke:?}"
                ));
            }
        }
    }
    overlay.word_baseline_step_um = Some(0);
    if overlay.expand(&[], None)? != original {
        return Err("zero baseline step changed native ink".into());
    }
    overlay.word_baseline_step_um = Some(150);
    overlay.word_drift_um = Some(90);
    overlay.word_rise_um = Some(90);
    overlay.line_drift_um = Some(120);
    overlay.word_tracking_um = Some(100);
    overlay.word_width_percent = Some(12);
    overlay.word_height_percent = Some(12);
    overlay.word_slant_millidegrees = Some(3_000);
    overlay.word_pressure_percent = Some(15);
    let combined = overlay.expand(&[], None)?;
    overlay.word_baseline_step_um = Some(0);
    let without = overlay.expand(&[], None)?;
    for (index, (baseline, inclined)) in without.iter().zip(&combined)
        .enumerate() {
        let run = index / 5;
        let word = match index % 5 { 0 | 1 => 0, 2 => 1, _ => 2 };
        let shift = steps[run * 3 + word] * ordinal[index % 5];
        if i64::from(inclined.start_um.1)
            - i64::from(baseline.start_um.1) != shift
            || i64::from(inclined.end_um.1)
                - i64::from(baseline.end_um.1) != shift
            || inclined.start_um.0 != baseline.start_um.0
            || inclined.end_um.0 != baseline.end_um.0
            || inclined.width_um != baseline.width_um
            || inclined.rgb != baseline.rgb {
            return Err("baseline slope changed other word rhythms".into());
        }
    }
    overlay.word_baseline_step_um = Some(151);
    if overlay.expand(&[], None).is_ok() {
        return Err("typed baseline step out of range accepted".into());
    }
    overlay.word_baseline_step_um = Some(150);
    overlay.variation = None;
    if overlay.expand(&[], None).is_ok() {
        return Err("unpaired typed baseline step accepted".into());
    }
    Ok(())
}

#[test]
fn word_baseline_step_preserves_directed_joins_and_statistical_baselines()
-> Result<(), String> {
    let mut overlay = GlyphOverlay::default();
    overlay.parse("--glyph-stroke", &mut args(&[
        "á", "150", "1200", "3370", "1200", "250", "#123456",
    ]).iter())?;
    overlay.parse("--place-text", &mut args(&[
        "1", "25000", "30000", "áá áá",
    ]).iter())?;
    overlay.parse("--glyph-variation", &mut args(&[
        "42", "0", "0", "0", "0", "0", "0",
    ]).iter())?;
    overlay.add_text_join_pair(&mut args(&["á", "á"]).iter())?;
    overlay.parse("--glyph-word-baseline-step",
        &mut args(&["150"]).iter())?;
    let joined = overlay.expand(&[], None)?;
    if joined.len() != 6 {
        return Err(format!("word baseline step omitted bridges: {joined:?}"));
    }
    for (offset, expected_rise) in [(0usize, 122i64), (3, 142)] {
        let [left, bridge, right] = joined.get(offset..offset + 3)
            .ok_or("missing joined word")? else {
            return Err("expected two strokes and one directed bridge".into());
        };
        if left.end_um != bridge.start_um
            || bridge.end_um != right.start_um
            || i64::from(bridge.end_um.1)
                - i64::from(bridge.start_um.1) != expected_rise
            || bridge.start_um.0 >= bridge.end_um.0
            || bridge.width_um != left.width_um
            || bridge.width_um != right.width_um {
            return Err(format!(
                "baseline trend broke physical pen join: {joined:?}"
            ));
        }
    }
    let profile = execute_handwriting(&args(&[
        "--set-glyph", "á", "baseline_offset",
        "500", "500", "500", "500", "0",
    ]))?.ok_or("missing statistical profile")?;
    let statistics = decode_statistics_json(
        &profile, &UnicodeGraphemeSegmentation,
    ).map_err(|error| format!("statistics: {error:?}"))?;
    overlay.text_joins.clear();
    overlay.runs.clear();
    overlay.parse("--place-text", &mut args(&[
        "1", "25000", "30000", "áá",
    ]).iter())?;
    let inclined = overlay.expand(&[], Some(&statistics))?;
    overlay.word_baseline_step_um = Some(0);
    let straight = overlay.expand(&[], Some(&statistics))?;
    if inclined.len() != 2 || straight.len() != 2
        || inclined[0] != straight[0]
        || i64::from(inclined[1].start_um.1)
            - i64::from(straight[1].start_um.1) != 122
        || i64::from(inclined[1].end_um.1)
            - i64::from(straight[1].end_um.1) != 122
        || inclined[1].start_um.0 != straight[1].start_um.0
        || inclined[1].end_um.0 != straight[1].end_um.0 {
        return Err(format!(
            "baseline slope lost saved statistical mean: {inclined:?}"
        ));
    }
    Ok(())
}

#[test]
fn word_baseline_step_pdf_png_and_page_ink_replay_and_invalid_options()
-> Result<(), String> {
    let prefix = args(&[
        "--printed-sheet", "true",
        "--glyph-stroke", "á", "600", "1000", "1600", "2800",
        "250", "#123456",
        "--flow-markdown", "áá á áá\n\náá áá",
        "--glyph-variation", "42", "0", "0", "0", "0", "0", "0",
    ]);
    let render = |format: &str, seed: &str, options: &[&str]|
        -> Result<Vec<u8>, String> {
        let mut command = prefix.clone();
        let index = command.iter().position(|v| v == "--glyph-variation")
            .ok_or("missing variation seed")? + 1;
        *command.get_mut(index).ok_or("missing seed")? = seed.into();
        if format == "ink" {
            command.push("--emit-ink-file".into());
        } else {
            command.extend(args(&["--export-paper", format]));
        }
        if format == "png" {
            command.extend(args(&["--dpi", "72"]));
        }
        command.extend(args(options));
        execute_notebook(&command)?.ok_or("missing native export".into())
    };
    for (format, signature) in [
        ("pdf", b"%PDF-1.4".as_slice()),
        ("png", b"\x89PNG\r\n\x1a\n".as_slice()),
        ("ink", b"atrament.page-ink.v1\n".as_slice()),
    ] {
        let original = render(format, "42", &[])?;
        if original != render(format, "42", &[
            "--glyph-word-baseline-step", "0",
        ])? {
            return Err(format!("zero slope changed {format} bytes"));
        }
        let inclined = render(format, "42", &[
            "--glyph-word-baseline-step", "150",
        ])?;
        if inclined == original || !inclined.starts_with(signature)
            || inclined != render(format, "42", &[
                "--glyph-word-baseline-step", "150",
            ])? || inclined == render(format, "43", &[
                "--glyph-word-baseline-step", "150",
            ])? {
            return Err(format!("{format} word baseline replay failed"));
        }
    }
    let page_ink = render("ink", "42", &[
        "--glyph-word-baseline-step", "150",
    ])?;
    let source = test_path().with_extension("tsv");
    fs::write(&source, page_ink).map_err(|error| error.to_string())?;
    let source_path = source.to_str().ok_or("non-UTF8 ink source")?;
    for format in ["pdf", "png"] {
        let mut command = args(&[
            "--printed-sheet", "true", "--ink-file", source_path,
            "--export-paper", format,
        ]);
        if format == "png" {
            command.extend(args(&["--dpi", "72"]));
        }
        if execute_notebook(&command)? != Some(render(
            format, "42", &["--glyph-word-baseline-step", "150"],
        )?) {
            return Err(format!("{format} baseline TSV reimport changed ink"));
        }
    }
    fs::remove_file(source).map_err(|error| error.to_string())?;

    let target = test_path();
    fs::write(&target, b"preserved export")
        .map_err(|error| error.to_string())?;
    let name = target.to_str().ok_or("non-UTF8 target path")?;
    for invalid in [
        args(&["--glyph-word-baseline-step", "-1"]),
        args(&["--glyph-word-baseline-step", "151"]),
        args(&["--glyph-word-baseline-step", "4294967296"]),
        args(&["--glyph-word-baseline-step", "150",
            "--glyph-word-baseline-step", "1"]),
    ] {
        let mut command = prefix.clone();
        command.extend(args(&[
            "--export-paper", "pdf", "--output", name, "--replace",
        ]));
        command.extend(invalid);
        if execute_notebook(&command).is_ok()
            || fs::read(&target).map_err(|error| error.to_string())?
                != b"preserved export" {
            return Err("invalid baseline option replaced export".into());
        }
    }
    let mut unpaired = prefix;
    let index = unpaired.iter().position(|v| v == "--glyph-variation")
        .ok_or("missing variation")?;
    unpaired.drain(index..index + 8).for_each(drop);
    unpaired.extend(args(&[
        "--glyph-word-baseline-step", "150", "--export-paper", "pdf",
        "--output", name, "--replace",
    ]));
    if execute_notebook(&unpaired).is_ok()
        || fs::read(&target).map_err(|error| error.to_string())?
            != b"preserved export" {
        return Err("unpaired baseline option replaced export".into());
    }
    fs::remove_file(target).map_err(|error| error.to_string())?;
    Ok(())
}

#[test]
fn word_baseline_step_refuses_bottom_margin_overshoot_in_pdf_and_png()
-> Result<(), String> {
    // An A4 page has a 16 mm bottom margin: 281 mm is the ink boundary.
    // Seed 42 moves the second letter down by 122 um; its last pen edge
    // crosses 281000 um while the unchanged word remains printable.
    let prefix = args(&[
        "--printed-sheet", "true",
        "--glyph-stroke", "á", "600", "1000", "1600", "1200",
        "250", "#123456",
        "--place-text", "1", "25000", "279580", "áá",
        "--glyph-variation", "42", "0", "0", "0", "0", "0", "0",
    ]);
    let target = test_path();
    let name = target.to_str().ok_or("non-UTF8 output path")?;
    for format in ["pdf", "png"] {
        let mut valid = prefix.clone();
        valid.extend(args(&["--export-paper", format]));
        if format == "png" {
            valid.extend(args(&["--dpi", "72"]));
        }
        if execute_notebook(&valid)?.is_none() {
            return Err("near-bottom baseline must be printable".into());
        }
        fs::write(&target, b"preserved page")
            .map_err(|error| error.to_string())?;
        valid.extend(args(&[
            "--glyph-word-baseline-step", "150",
            "--output", name, "--replace",
        ]));
        let error = execute_notebook(&valid)
            .expect_err("word baseline slope must cross printable margin");
        if !error.contains("page 1 ink needs printable endpoints")
            || fs::read(&target).map_err(|e| e.to_string())?
                != b"preserved page" {
            return Err(format!(
                "baseline overshoot unexpectedly changed {format}: {error}"
            ));
        }
    }
    fs::remove_file(target).map_err(|error| error.to_string())?;
    Ok(())
}

#[test]
fn word_baseline_step_rejects_overlong_directed_bridges_atomically()
-> Result<(), String> {
    // The initial same-height gap is 1195 um, just inside the 1200-um
    // physical join limit. The second glyph's seeded +122 um baseline
    // step increases the exact squared distance to 1_442_909 > 1_440_000.
    let prefix = args(&[
        "--printed-sheet", "true",
        "--glyph-stroke", "á", "150", "1200", "2955", "1200",
        "250", "#123456",
        "--place-text", "1", "25000", "30000", "áá",
        "--join-text-pair", "á", "á",
        "--glyph-variation", "42", "0", "0", "0", "0", "0", "0",
    ]);
    let target = test_path();
    let name = target.to_str().ok_or("non-UTF8 output path")?;
    for format in ["pdf", "png"] {
        let mut original = prefix.clone();
        original.extend(args(&["--export-paper", format]));
        if format == "png" {
            original.extend(args(&["--dpi", "72"]));
        }
        let accepted = execute_notebook(&original)?
            .ok_or("valid straight short join missing export")?;
        if accepted.is_empty() {
            return Err("accepted join export must not be empty".into());
        }
        fs::write(&target, b"preserved output")
            .map_err(|error| error.to_string())?;
        original.extend(args(&[
            "--glyph-word-baseline-step", "150",
            "--output", name, "--replace",
        ]));
        let refusal = execute_notebook(&original)
            .expect_err("inclined bridge exceeds inclusive join limit");
        if !refusal.contains("gap")
            || fs::read(&target).map_err(|error| error.to_string())?
                != b"preserved output" {
            return Err(format!(
                "invalid inclined join changed {format}: {refusal}"
            ));
        }
    }
    fs::remove_file(target).map_err(|error| error.to_string())?;
    Ok(())
}

#[test]
fn word_pigment_has_exact_seeded_unicode_word_samples_and_independent_ink()
-> Result<(), String> {
    let glyph = "n\u{303}";
    let mut overlay = GlyphOverlay::default();
    overlay.parse("--glyph-stroke", &mut args(&[
        glyph, "500", "1000", "1600", "2800", "250", "#123456",
    ]).iter())?;
    let text = format!("{glyph}{glyph}  {glyph} {glyph}{glyph}");
    for y in [30_000u32, 40_000u32] {
        overlay.parse("--place-text", &mut args(&[
            "1", "25000", &y.to_string(), &text,
        ]).iter())?;
    }
    overlay.parse("--glyph-variation", &mut args(&[
        "42", "0", "0", "0", "0", "0", "0",
    ]).iter())?;
    let original = overlay.expand(&[], None)?;
    overlay.parse("--glyph-word-pigment", &mut args(&["20"]).iter())?;
    let changed = overlay.expand(&[], None)?;
    // Independent uint32 stream oracle. Three words per run, with repeated
    // ASCII spaces creating exactly one word boundary.
    let colors = [
        [[23, 56, 89], [25, 58, 91], [35, 66, 98]],
        [[32, 64, 96], [32, 64, 96], [15, 43, 71]],
    ];
    if changed.len() != 10 || original.len() != 10
        || changed == original || changed != overlay.expand(&[], None)? {
        return Err("seeded word pigment did not replay".into());
    }
    for (run, row) in changed.chunks_exact(5).enumerate() {
        for (index, line) in row.iter().enumerate() {
            let word = match index { 0 | 1 => 0, 2 => 1, _ => 2 };
            let baseline = &original[run * 5 + index];
            if line.rgb != colors[run][word]
                || line.start_um != baseline.start_um
                || line.end_um != baseline.end_um
                || line.width_um != baseline.width_um {
                return Err(format!(
                    "word pigment mismatch at {run}/{index}: {line:?}"
                ));
            }
        }
    }
    overlay.word_pigment_percent = Some(0);
    if overlay.expand(&[], None)? != original {
        return Err("zero word pigment changed legacy bytes".into());
    }
    overlay.word_pigment_percent = Some(20);
    overlay.word_rise_um = Some(90);
    overlay.word_width_percent = Some(10);
    overlay.word_baseline_step_um = Some(90);
    let combined = overlay.expand(&[], None)?;
    overlay.word_pigment_percent = Some(0);
    let without = overlay.expand(&[], None)?;
    for (tinted, untouched) in combined.iter().zip(&without) {
        if tinted.start_um != untouched.start_um
            || tinted.end_um != untouched.end_um
            || tinted.width_um != untouched.width_um {
            return Err("word pigment altered independent geometry".into());
        }
    }
    overlay.word_pigment_percent = Some(21);
    if overlay.expand(&[], None).is_ok() {
        return Err("typed out-of-range word pigment accepted".into());
    }
    overlay.word_pigment_percent = Some(20);
    overlay.variation = None;
    if overlay.expand(&[], None).is_ok() {
        return Err("typed unpaired word pigment accepted".into());
    }
    Ok(())
}

#[test]
fn word_pigment_preserves_directed_joins_and_heading_color_override()
-> Result<(), String> {
    let mut overlay = GlyphOverlay::default();
    overlay.parse("--glyph-stroke", &mut args(&[
        "á", "500", "1200", "3300", "1200", "250", "#123456",
    ]).iter())?;
    overlay.parse("--place-text", &mut args(&[
        "1", "25000", "30000", "áá áá",
    ]).iter())?;
    overlay.parse("--glyph-variation", &mut args(&[
        "42", "0", "0", "0", "0", "0", "0",
    ]).iter())?;
    overlay.add_text_join_pair(&mut args(&["á", "á"]).iter())?;
    overlay.parse("--glyph-word-pigment", &mut args(&["20"]).iter())?;
    let joined = overlay.expand(&[], None)?;
    if joined.len() != 6 { return Err("two joins need six strokes".into()); }
    for (start, expected) in [
        (0usize, [23, 56, 89]), (3, [25, 58, 91]),
    ] {
        let [first, bridge, last] = joined.get(start..start + 3)
            .ok_or("joined word missing")? else {
            return Err("joined word needs three strokes".into());
        };
        if first.end_um != bridge.start_um
            || bridge.end_um != last.start_um
            || [first.rgb, bridge.rgb, last.rgb] != [expected; 3]
            || [first.width_um, bridge.width_um, last.width_um]
                != [250; 3] {
            return Err(format!("pigment broke same-pen join: {joined:?}"));
        }
    }
    let run = overlay.runs.get_mut(0).ok_or("missing heading run")?;
    run.heading_ink = Some(HeadingInk {
        color: Some([0x45, 0x67, 0x89]), scale_percent: 150,
    });
    let heading = overlay.expand(&[], None)?;
    for (line, other) in heading.iter().zip(&joined) {
        if line.rgb == other.rgb || line.start_um != other.start_um
            || line.end_um != other.end_um
            || line.width_um != 375 {
            return Err("word tint and heading override did not compose".into());
        }
    }
    for group in heading.chunks_exact(3) {
        if group[0].rgb != group[1].rgb
            || group[1].rgb != group[2].rgb {
            return Err("heading tint disconnected physical pen".into());
        }
    }
    Ok(())
}

#[test]
fn word_pigment_native_pdf_png_tsv_replay_and_invalid_output_are_atomic()
-> Result<(), String> {
    let source = args(&[
        "--printed-sheet", "true",
        "--glyph-stroke", "á", "500", "1200", "3300", "1200",
        "250", "#123456",
        "--flow-text", "áá áá\náá áá",
        "--glyph-variation", "42", "0", "0", "0", "0", "0", "0",
    ]);
    let render = |format: &str, seed: &str, options: &[&str]|
        -> Result<Vec<u8>, String> {
        let mut command = source.clone();
        let index = command.iter().position(|arg| arg == "--glyph-variation")
            .ok_or("missing variation option")? + 1;
        *command.get_mut(index).ok_or("missing seed")? = seed.into();
        if format == "ink" {
            command.push("--emit-ink-file".into());
        } else {
            command.extend(args(&["--export-paper", format]));
        }
        if format == "png" { command.extend(args(&["--dpi", "72"])); }
        command.extend(args(options));
        execute_notebook(&command)?.ok_or("missing native export".into())
    };
    for (format, signature) in [
        ("pdf", b"%PDF-1.4".as_slice()),
        ("png", b"\x89PNG\r\n\x1a\n".as_slice()),
        ("ink", b"atrament.page-ink.v1\n".as_slice()),
    ] {
        let plain = render(format, "42", &[])?;
        if plain != render(format, "42", &[
            "--glyph-word-pigment", "0",
        ])? {
            return Err(format!("zero pigment changed {format} bytes"));
        }
        let shaded = render(format, "42", &[
            "--glyph-word-pigment", "20",
        ])?;
        if !shaded.starts_with(signature) || shaded == plain
            || shaded != render(format, "42", &[
                "--glyph-word-pigment", "20",
            ])? {
            return Err(format!("seeded {format} pigment cannot replay"));
        }
        if shaded == render(format, "43", &[
            "--glyph-word-pigment", "20",
        ])? {
            return Err(format!("different seed unchanged {format}"));
        }
        if format == "ink" && !shaded.windows(7)
            .any(|bytes| bytes == b"#173859") {
            return Err("native page-ink TSV lacks physical tint".into());
        }
    }
    let target = test_path();
    fs::write(&target, b"accepted export")
        .map_err(|error| error.to_string())?;
    let name = target.to_str().ok_or("non-UTF8 output path")?;
    for format in ["pdf", "png"] {
        for invalid in [
            args(&["--glyph-word-pigment", "-1"]),
            args(&["--glyph-word-pigment", "21"]),
            args(&["--glyph-word-pigment", "20",
                "--glyph-word-pigment", "1"]),
        ] {
            let mut command = source.clone();
            command.extend(args(&["--export-paper", format]));
            if format == "png" {
                command.extend(args(&["--dpi", "72"]));
            }
            command.extend(args(&["--output", name, "--replace"]));
            command.extend(invalid);
            if execute_notebook(&command).is_ok()
                || fs::read(&target).map_err(|error| error.to_string())?
                    != b"accepted export" {
                return Err(format!("invalid pigment modified {format}"));
            }
        }
    }
    let mut unpaired = source;
    let index = unpaired.iter().position(|arg| arg == "--glyph-variation")
        .ok_or("missing variation")?;
    unpaired.drain(index..index + 8).for_each(drop);
    unpaired.extend(args(&[
        "--glyph-word-pigment", "20", "--export-paper", "pdf",
        "--output", name, "--replace",
    ]));
    if execute_notebook(&unpaired).is_ok()
        || fs::read(&target).map_err(|error| error.to_string())?
            != b"accepted export" {
        return Err("unpaired pigment modified output".into());
    }
    fs::remove_file(target).map_err(|error| error.to_string())?;
    Ok(())
}

#[test]
fn selected_flow_pitch_measures_unicode_lines_and_preserves_legacy_layout() {
    let source = FlowSource::new(FlowFormat::Plain,
        "n\u{303}\nn\u{303}\nn\u{303}\nn\u{303}")
        .expect("exact decomposed graphemes");
    let paper = NotebookProfile::default();
    let region = FlowRegion {
        left: 30_000, top: 40_000, width: 50_000, height: 45_000,
    };
    let default = layout_flow_with_region(
        &source, &paper, None, HeadingDecoration::default(), Some(region),
    ).expect("legacy 9-mm layout");
    let minimum = |pitch| layout_flow_with_min_pitch(
        &source, &paper, None, HeadingDecoration::default(), Some(region),
        Some(pitch),
    );
    assert_eq!(default.2, 1);
    assert_eq!(default.0.iter().map(|run| (run.page, run.origin_um.1))
        .collect::<Vec<_>>(), [
            (1, 42_500), (1, 51_500), (1, 60_500), (1, 69_500),
        ]);
    assert_eq!(minimum(9_000).expect("explicit default").0.iter()
        .map(|run| (run.page, run.origin_um, run.text.as_str()))
        .collect::<Vec<_>>(), default.0.iter()
        .map(|run| (run.page, run.origin_um, run.text.as_str()))
        .collect::<Vec<_>>());
    let (runs, marks, pages) = minimum(18_000)
        .expect("minimum physical row leading");
    assert_eq!(pages, 2);
    assert!(marks.is_empty());
    assert_eq!(runs.iter().map(|run| (run.page, run.origin_um.1))
        .collect::<Vec<_>>(), [
            (1, 42_500), (1, 60_500), (2, 42_500), (2, 60_500),
        ]);
    assert!(runs.iter().all(|run| run.text == "n\u{303}"));
    assert_eq!(minimum(30_000).expect("maximum accepted pitch").2, 4);
    for invalid in [0, 8_999, 30_001, u32::MAX] {
        let error = minimum(invalid)
            .expect_err("typed pitch bypass must be rejected");
        assert_eq!(error, "flow pitch must be within 9000..30000 um");
    }
}

#[test]
fn selected_flow_pitch_exports_replayable_pdf_png_and_page_ink() {
    let base = args(&[
        "--printed-sheet", "true",
        "--glyph-stroke", "a", "900", "900", "2200", "3400",
        "250", "#204060", "--flow-text", "a\na\na\na",
        "--flow-region", "30000", "40000", "50000", "60000",
        "--flow-pitch-um", "18000",
    ]);
    let mut pdf_command = base.clone();
    pdf_command.extend(args(&["--export-paper", "pdf"]));
    let pdf = execute_notebook(&pdf_command)
        .expect("two-page pitched PDF").expect("native PDF bytes");
    assert!(pdf.starts_with(b"%PDF-1.4"));
    let streams = String::from_utf8_lossy(&pdf)
        .split("\nstream\n").skip(1).count();
    assert_eq!(streams, 2, "18-mm pitch requires two pages");
    assert!(execute_notebook(&pdf_command)
        .expect("PDF replay").expect("PDF bytes") == pdf);

    // An explicit default is byte-for-byte identical to omitting pitch.
    let mut unselected = pdf_command.clone();
    let position = unselected.iter().position(|arg| arg == "--flow-pitch-um")
        .expect("selected pitch flag");
    drop(unselected.drain(position..position + 2));
    let mut original = unselected.clone();
    original.extend(args(&["--flow-pitch-um", "9000"]));
    assert!(execute_notebook(&unselected).expect("default PDF")
        == execute_notebook(&original).expect("explicit 9-mm PDF"));

    let mut png_command = base.clone();
    png_command.extend(args(&[
        "--export-paper", "png", "--dpi", "72", "--png-page", "2",
    ]));
    let second = execute_notebook(&png_command)
        .expect("second transparent physical PNG")
        .expect("PNG bytes");
    assert!(second.starts_with(b"\x89PNG\r\n\x1a\n"));
    assert!(execute_notebook(&png_command)
        .expect("PNG replay").expect("PNG bytes") == second);
    let selected = png_command.last_mut().expect("selected page number");
    *selected = "1".into();
    let first = execute_notebook(&png_command)
        .expect("first physical PNG").expect("PNG bytes");
    assert!(first != second, "pagination changes visible ink");

    let mut ink_command = base;
    ink_command.push("--emit-ink-file".into());
    let bytes = execute_notebook(&ink_command)
        .expect("physical page ink TSV").expect("TSV bytes");
    assert_eq!(bytes, execute_notebook(&ink_command)
        .expect("TSV replay").expect("TSV bytes"));
    let ink = String::from_utf8(bytes).expect("UTF-8 physical ink");
    let rows = ink.lines().collect::<Vec<_>>();
    assert_eq!(rows[0], "atrament.page-ink.v1");
    assert_eq!(rows.len(), 5, "four physical authored glyph strokes");
    assert_eq!(rows.iter().skip(1).map(|row| row.split('\t')
        .next().expect("page field")).collect::<Vec<_>>(),
        ["1", "1", "1", "2"]);
}

#[test]
fn invalid_or_unsupported_flow_pitch_refuses_before_source_io_and_commit() {
    let before_io = args(&[
        "--flow-text-file", "-", "--glyph-file", "not-opened.tsv",
        "--export-paper", "pdf",
    ]);
    for (values, diagnostic) in [
        (vec!["--flow-pitch-um", "8999"], "within 9000..30000"),
        (vec!["--flow-pitch-um", "30001"], "within 9000..30000"),
        (vec!["--flow-pitch-um", "-9000"], "within 9000..30000"),
        (vec!["--flow-pitch-um", "+9000"], "within 9000..30000"),
        (vec!["--flow-pitch-um", "4294967296"], "within 9000..30000"),
        (vec!["--flow-pitch-um", ""], "within 9000..30000"),
        (vec!["--flow-pitch-um"], "requires 9000..30000"),
        (vec!["--flow-pitch-um", "18000", "--flow-pitch-um", "9000"],
            "may appear only once"),
    ] {
        let mut command = before_io.clone();
        command.extend(args(&values));
        let error = preflight_notebook_sources(&command)
            .expect_err("invalid late pitch must refuse before stdin or files");
        assert!(error.contains(diagnostic), "{values:?}: {error}");
    }
    let error = preflight_notebook_sources(&args(&[
        "--ink-file", "-", "--export-paper", "pdf",
        "--flow-pitch-um", "18000",
    ])).expect_err("flow-only pitch must refuse before stdin read");
    assert_eq!(error, "--flow-pitch-um requires a text or Markdown flow");

    let target = test_path().with_extension("pdf");
    fs::write(&target, b"keep-accepted-pdf")
        .expect("owned original PDF sentinel");
    let mut command = args(&[
        "--export-paper", "pdf", "--pages", "1",
        "--glyph-stroke", "a", "900", "900", "2200", "3400",
        "250", "#204060", "--flow-text", "a\na\na\na",
        "--flow-region", "30000", "40000", "50000", "45000",
        "--flow-pitch-um", "18000",
    ]);
    command.extend(args(&[
        "--output", target.to_str().expect("owned path"), "--replace",
    ]));
    let error = execute_notebook(&command)
        .expect_err("requested one-page output cannot fit chosen leading");
    assert!(error.contains("requires 2 physical pages"), "{error}");
    assert_eq!(fs::read(&target).expect("sentinel unchanged"),
        b"keep-accepted-pdf");
    fs::remove_file(target).expect("remove test-owned file");
}

#[test]
fn selected_minimum_pitch_cannot_compress_measured_handwriting_height() {
    let statistics_json = execute_handwriting(&args(&[
        "--set-glyph", "a", "height", "20000", "20000",
        "20000", "20000", "0",
    ])).expect("valid 20-mm glyph height").expect("canonical statistics");
    let statistics = decode_statistics_json(
        &statistics_json, &UnicodeGraphemeSegmentation,
    ).expect("reopen selected exact height");
    let paper = NotebookProfile::default();
    let source = FlowSource::new(FlowFormat::Plain, "a\na")
        .expect("two authored physical rows");
    let render = |minimum| layout_flow_with_min_pitch(
        &source, &paper, Some(&statistics), HeadingDecoration::default(),
        None, minimum,
    );
    let (legacy, _, _) = render(None).expect("glyph-height-driven pitch");
    let (small, _, _) = render(Some(18_000))
        .expect("explicit minimum does not shrink tall glyphs");
    let (large, _, _) = render(Some(30_000))
        .expect("wider minimum may enlarge tall-glyph pitch");
    assert_eq!(legacy[1].origin_um.1 - legacy[0].origin_um.1, 24_000);
    assert_eq!(small[1].origin_um.1 - small[0].origin_um.1,
        24_000, "measured contour remains authoritative");
    assert_eq!(large[1].origin_um.1 - large[0].origin_um.1, 30_000);
}

#[test]
fn selected_pitch_preserves_markdown_decorations_across_region_page_breaks() {
    let paper = NotebookProfile::default();
    let source = FlowSource::new(FlowFormat::Markdown,
        "# a\n\n> a\n\n- a\n\n1. a")
        .expect("heading, quote, bullet and number source");
    let region = FlowRegion {
        left: 30_000, top: 40_000, width: 80_000, height: 95_000,
    };
    let render = |pitch| layout_flow_with_min_pitch(
        &source, &paper, None, HeadingDecoration::default(),
        Some(region), Some(pitch),
    );
    let (normal, normal_marks, normal_pages) = render(9_000)
        .expect("default Markdown annotations fit one page");
    let (wide, marks, pages) = render(30_000)
        .expect("large leading paginates Markdown decorations");
    assert_eq!(normal_pages, 1);
    assert!(pages > normal_pages, "larger leading should add a page");
    let visible = |runs: &[TextRun]| runs.iter()
        .map(|run| run.text.clone()).collect::<Vec<_>>();
    assert_eq!(visible(&normal), visible(&wide),
        "a spacing override must not alter Markdown content");
    assert_eq!(normal_marks.len(), marks.len(),
        "the same semantic decorations must survive pagination");
    assert!(marks.len() >= 3,
        "heading, blockquote and bullet all retain physical ink");
    for run in &wide {
        assert!((1..=pages).contains(&run.page));
        assert!((42_500..137_500).contains(&run.origin_um.1));
    }
    for mark in &marks {
        assert!((1..=pages).contains(&mark.page));
        assert!(mark.start_um.1 >= region.top);
        assert!(mark.end_um.1 <= region.top + region.height);
    }
    assert_eq!(render(30_000).expect("replay").1, marks);
}

#[test]
fn selected_markdown_paragraph_gap_preserves_default_and_changes_page_flow() {
    let source = FlowSource::new(FlowFormat::Markdown, "a\n\nb\n\nc")
        .expect("three paragraphs");
    let paper = NotebookProfile::default();
    let region = FlowRegion {
        left: 30_000, top: 40_000, width: 50_000, height: 45_000,
    };
    let render = |gap| layout_flow_with_spacing(
        &source, &paper, None, HeadingDecoration::default(),
        Some(region), None, gap,
    );
    let legacy = layout_flow_with_region(
        &source, &paper, None, HeadingDecoration::default(), Some(region),
    ).expect("default Markdown layout");
    let normalize = |(runs, marks, pages):
        (Vec<TextRun>, Vec<NotebookInkLine>, usize)| {
        (runs.into_iter().map(|run| (run.page, run.origin_um,
            run.text, run.heading_ink)).collect::<Vec<_>>(), marks, pages)
    };
    assert_eq!(normalize(render(None).expect("implicit default")),
        normalize(layout_flow_with_region(&source, &paper, None,
            HeadingDecoration::default(), Some(region))
            .expect("default layout replay")));
    assert_eq!(normalize(render(Some(4_000)).expect("explicit default")),
        normalize(legacy));
    let origins = |gap| {
        let (lines, marks, pages) = render(Some(gap)).expect("gap layout");
        assert!(marks.is_empty());
        (lines.into_iter().map(|line| (line.page, line.origin_um.1))
            .collect::<Vec<_>>(), pages)
    };
    assert_eq!(origins(0), (
        vec![(1, 42_500), (1, 51_500), (1, 60_500)], 1,
    ));
    assert_eq!(origins(4_000), (
        vec![(1, 42_500), (1, 55_500), (1, 68_500)], 1,
    ));
    assert_eq!(origins(12_000), (
        vec![(1, 42_500), (1, 63_500), (2, 42_500)], 2,
    ));
    for invalid in [12_001, u32::MAX] {
        assert_eq!(render(Some(invalid)).expect_err("typed bound"),
            "flow paragraph gap must be within 0..12000 um");
    }
    let plain = FlowSource::new(FlowFormat::Plain, "a\n\nb")
        .expect("plain physical rows");
    assert_eq!(layout_flow_with_spacing(
        &plain, &paper, None, HeadingDecoration::default(),
        Some(region), None, Some(0),
    ).expect_err("typed Markdown-only gate"),
        "flow paragraph gap requires Markdown flow");
    let collapsed = FlowSource::new(FlowFormat::Markdown, "a\n\n\n\nb")
        .expect("multiple blank lines");
    let one = FlowSource::new(FlowFormat::Markdown, "a\n\nb")
        .expect("single blank line");
    let layout = |source: &FlowSource| layout_flow_with_spacing(
        source, &paper, None, HeadingDecoration::default(),
        Some(region), None, Some(12_000),
    ).expect("collapsed gap layout");
    assert_eq!(normalize(layout(&collapsed)), normalize(layout(&one)));
}

#[test]
fn selected_paragraph_gap_keeps_decorated_heading_and_rule_chains() {
    let paper = NotebookProfile::default();
    let region = FlowRegion {
        left: 30_000, top: 40_000, width: 80_000, height: 70_000,
    };
    let source = FlowSource::new(FlowFormat::Markdown,
        "a  \na  \na\n\n---\n\n# a\n\n## a\n\n> a")
        .expect("body and decorated title chain");
    let render = |gap| layout_flow_with_spacing(
        &source, &paper, None, HeadingDecoration::default(),
        Some(region), None, Some(gap),
    );
    for gap in [0, 4_000, 12_000] {
        let (runs, marks, pages) = render(gap)
            .expect("fitting decorated layout");
        assert!(pages >= 1);
        if gap == 12_000 {
            assert!(pages >= 2, "wider gaps must paginate a compact region");
        }
        assert_eq!(runs.iter().map(|run| run.text.as_str())
            .collect::<Vec<_>>(), ["a", "a", "a", "a", "a", "a"]);
        assert!(marks.len() >= 3, "rule, titles and quote need ink");
        assert_eq!(render(gap).expect("identical replay").1, marks);
        assert!(runs.iter().all(|run| run.origin_um.1 >= 42_500
            && run.origin_um.1 < 107_500));
        assert!(marks.iter().all(|mark| mark.start_um.1 >= region.top
            && mark.end_um.1 <= region.top + region.height));
    }
}

#[test]
fn selected_paragraph_gap_exports_real_pdf_png_and_page_ink() {
    let base = args(&[
        "--printed-sheet", "true",
        "--glyph-stroke", "a", "900", "900", "2200", "3400",
        "250", "#204060",
        "--glyph-stroke", "b", "900", "900", "2200", "3400",
        "250", "#204060",
        "--glyph-stroke", "c", "900", "900", "2200", "3400",
        "250", "#204060",
        "--flow-markdown", "a\n\nb\n\nc",
        "--flow-region", "30000", "40000", "50000", "45000",
    ]);
    let export = |gap: Option<&str>, format: &str, page: Option<&str>| {
        let mut command = base.clone();
        if let Some(gap) = gap {
            command.extend(args(&["--flow-paragraph-gap-um", gap]));
        }
        if format == "ink" {
            command.push("--emit-ink-file".into());
        } else {
            command.extend(args(&["--export-paper", format]));
        }
        if format == "png" {
            command.extend(args(&[
                "--dpi", "72", "--png-page", page.unwrap_or("1"),
            ]));
        }
        execute_notebook(&command)
            .expect("selected native export").expect("native bytes")
    };
    for format in ["pdf", "png", "ink"] {
        let legacy = export(None, format, None);
        assert_eq!(legacy, export(Some("4000"), format, None),
            "explicit 4-mm default changes {format}");
        assert_eq!(export(Some("12000"), format, None),
            export(Some("12000"), format, None),
            "selected {format} must replay byte for byte");
        assert_ne!(legacy, export(Some("12000"), format, None),
            "selected {format} must change actual physical output");
    }
    let pdf = export(Some("12000"), "pdf", None);
    assert!(pdf.starts_with(b"%PDF-1.4"));
    assert_eq!(String::from_utf8_lossy(&pdf)
        .split("\nstream\n").skip(1).count(), 2);
    let second = export(Some("12000"), "png", Some("2"));
    assert!(second.starts_with(b"\x89PNG\r\n\x1a\n"));
    let first = export(Some("12000"), "png", Some("1"));
    assert_ne!(first, second);
    let ink = String::from_utf8(export(Some("12000"), "ink", None))
        .expect("valid page-ink TSV");
    let pages = ink.lines().skip(1)
        .map(|line| line.split('\t').next().unwrap_or(""))
        .collect::<Vec<_>>();
    assert_eq!(pages, ["1", "1", "2"]);
}

#[test]
fn invalid_paragraph_gap_refuses_before_input_and_preserves_existing_pdf() {
    let source = args(&[
        "--flow-markdown-file", "-", "--glyph-file", "not-opened.tsv",
        "--export-paper", "pdf",
    ]);
    for (tokens, diagnostic) in [
        (vec!["--flow-paragraph-gap-um", "12001"], "within 0..12000"),
        (vec!["--flow-paragraph-gap-um", "-1"], "within 0..12000"),
        (vec!["--flow-paragraph-gap-um", "+0"], "within 0..12000"),
        (vec!["--flow-paragraph-gap-um", "4294967296"],
            "within 0..12000"),
        (vec!["--flow-paragraph-gap-um", ""], "within 0..12000"),
        (vec!["--flow-paragraph-gap-um"], "requires 0..12000"),
        (vec!["--flow-paragraph-gap-um", "0",
            "--flow-paragraph-gap-um", "1"], "may appear only once"),
    ] {
        let mut command = source.clone();
        command.extend(args(&tokens));
        let error = preflight_notebook_sources(&command)
            .expect_err("invalid late gap must be rejected before stdin");
        assert!(error.contains(diagnostic), "{tokens:?}: {error}");
    }
    for (flag, value) in [
        ("--flow-text", "a"), ("--ink-file", "not-opened.tsv"),
    ] {
        let mut command = args(&[
            "--export-paper", "pdf", "--glyph-file", "not-opened.tsv",
            flag, value,
        ]);
        command.extend(args(&["--flow-paragraph-gap-um", "0"]));
        let error = preflight_notebook_sources(&command)
            .expect_err("only Markdown may select paragraph gap");
        assert_eq!(error,
            "--flow-paragraph-gap-um requires a Markdown flow");
    }

    let target = test_path().with_extension("pdf");
    fs::write(&target, b"keep-old-PDF").expect("sentinel PDF");
    let mut command = args(&[
        "--export-paper", "pdf", "--pages", "1",
        "--glyph-stroke", "a", "900", "900", "2200", "3400",
        "250", "#204060", "--flow-markdown", "a\n\na\n\na",
        "--flow-region", "30000", "40000", "50000", "45000",
        "--flow-paragraph-gap-um", "12000",
    ]);
    command.extend(args(&[
        "--output", target.to_str().expect("test path"), "--replace",
    ]));
    let error = execute_notebook(&command)
        .expect_err("a 12-mm paragraph gap needs a second page");
    assert!(error.contains("requires 2 physical pages"), "{error}");
    assert_eq!(fs::read(&target).expect("sentinel survives"), b"keep-old-PDF");
    fs::remove_file(target).expect("delete test-owned sentinel");
}

#[test]
fn selected_markdown_body_inset_moves_only_flowing_prose_rows() {
    let source = FlowSource::new(FlowFormat::Markdown,
        "# a\n\na a a  \na\n\n- a\n\n> a\n\n    a\n\n~~~\na\n~~~")
        .expect("mixed Markdown");
    let paper = NotebookProfile::default();
    let region = FlowRegion {
        left: 30_000, top: 40_000, width: 52_000, height: 185_000,
    };
    let render = |inset| layout_flow_with_typography(
        &source, &paper, None, HeadingDecoration::default(),
        Some(region), FlowTypography { body_inset: inset,
            ..FlowTypography::default() },
    );
    let legacy = layout_flow_with_spacing(
        &source, &paper, None, HeadingDecoration::default(),
        Some(region), None, None,
    ).expect("original layout");
    let zero = render(Some(0)).expect("explicit zero");
    let signature = |(runs, marks, pages):
        (Vec<TextRun>, Vec<NotebookInkLine>, usize)| {
        (runs.into_iter().map(|run| (run.page, run.origin_um, run.text))
            .collect::<Vec<_>>(), marks, pages)
    };
    assert_eq!(signature(render(None).expect("implicit zero")),
        signature(render(Some(0)).expect("explicit zero")));
    assert_eq!(signature(legacy), signature(zero));
    let (baseline, marks, _) = render(None).expect("original layout");
    let (shifted, new_marks, _) = render(Some(6_000))
        .expect("selected body inset");
    assert_eq!(marks, new_marks, "list and quote ornaments do not move");
    assert_eq!(baseline.len(), shifted.len(), "this source does not reflow");
    let first_list = baseline.iter().position(|run| run.origin_um.0 == 38_500)
        .expect("first visible list row");
    for (index, (old, new)) in baseline.iter().zip(&shifted).enumerate() {
        assert_eq!(old.text, new.text);
        assert_eq!(old.page, new.page);
        assert_eq!(old.origin_um.1, new.origin_um.1);
        assert_eq!(new.origin_um.0 - old.origin_um.0,
            if (1..first_list).contains(&index) { 6_000 } else { 0 },
            "{old:?}");
    }
    assert!(shifted.iter().any(|run| run.text.starts_with("a a")
        && run.origin_um.0 == 38_500));
    assert_eq!(shifted.iter().filter(|run| run.text == "    a").count(), 1);
}

#[test]
fn selected_markdown_body_inset_measures_wrapping_and_refuses_narrow_rows() {
    let source = FlowSource::new(FlowFormat::Markdown,
        "a a a a a a a a a a a a").expect("visible prose");
    let paper = NotebookProfile::default();
    let region = FlowRegion {
        left: 30_000, top: 40_000, width: 32_000, height: 32_000,
    };
    let render = |inset, fixed| layout_flow_with_typography(
        &source, &paper, None, HeadingDecoration::default(),
        Some(fixed), FlowTypography { body_inset: inset,
            ..FlowTypography::default() },
    );
    let (legacy, _, _) = render(None, region).expect("default wrap");
    let (shifted, _, pages) = render(Some(10_000), region)
        .expect("inset wraps onto multiple pages");
    assert!(shifted.len() > legacy.len(), "inset reduces physical capacity");
    assert!(pages >= 2, "narrow inset should paginate");
    assert!(shifted.iter().all(|row| row.origin_um.0 == 42_500));
    let narrow = FlowRegion {
        left: 30_000, top: 40_000, width: 9_000, height: 32_000,
    };
    assert_eq!(render(Some(3_000), narrow)
        .expect_err("body inset cannot consume entire writing row"),
        "flow body inset leaves no writing row");
    for invalid in [12_001, u32::MAX] {
        assert_eq!(render(Some(invalid), region)
            .expect_err("typed maximum"),
            "flow body inset must be within 0..12000 um");
    }
    let plain = FlowSource::new(FlowFormat::Plain, "a").expect("plain text");
    assert_eq!(layout_flow_with_typography(
        &plain, &paper, None, HeadingDecoration::default(),
        Some(region), FlowTypography { body_inset: Some(0),
            ..FlowTypography::default() },
    ).expect_err("plain text cannot opt into Markdown prose inset"),
        "flow body inset requires Markdown flow");
}

#[test]
fn selected_markdown_body_inset_exports_replayable_pdf_png_and_ink() {
    let base = args(&[
        "--printed-sheet", "true",
        "--glyph-stroke", "a", "900", "900", "2200", "3400",
        "250", "#204060",
        "--flow-markdown", "a a a a a\n\n# a\n\n- a\n\n> a",
        "--flow-region", "30000", "40000", "52000", "100000",
    ]);
    let export = |inset: Option<&str>, format: &str| {
        let mut command = base.clone();
        if let Some(value) = inset {
            command.extend(args(&["--flow-body-inset-um", value]));
        }
        if format == "ink" {
            command.push("--emit-ink-file".into());
        } else {
            command.extend(args(&["--export-paper", format]));
        }
        if format == "png" {
            command.extend(args(&["--dpi", "72"]));
        }
        execute_notebook(&command).expect("native export")
            .expect("export bytes")
    };
    for format in ["pdf", "png", "ink"] {
        let original = export(None, format);
        assert_eq!(original, export(Some("0"), format),
            "zero inset must retain exact original {format} bytes");
        let selected = export(Some("6000"), format);
        assert_eq!(selected, export(Some("6000"), format),
            "selected {format} must be byte-deterministic");
        assert_ne!(original, selected,
            "selected inset must change native {format} ink");
        if format == "pdf" {
            assert!(selected.starts_with(b"%PDF-1.4"));
        } else if format == "png" {
            assert!(selected.starts_with(b"\x89PNG\r\n\x1a\n"));
        } else {
            let text = String::from_utf8(selected).expect("versioned TSV");
            assert!(text.starts_with("atrament.page-ink.v1\n"));
            assert!(text.lines().skip(1)
                .any(|line| line.starts_with("1\t39400\t")));
        }
    }
}

#[test]
fn invalid_markdown_body_inset_preflight_and_export_are_atomic() {
    let source = args(&[
        "--flow-markdown-file", "-", "--glyph-file", "not-opened.tsv",
        "--export-paper", "pdf",
    ]);
    for (tokens, diagnostic) in [
        (vec!["--flow-body-inset-um", "12001"], "within 0..12000"),
        (vec!["--flow-body-inset-um", "-1"], "within 0..12000"),
        (vec!["--flow-body-inset-um", "+0"], "within 0..12000"),
        (vec!["--flow-body-inset-um", "4294967296"], "within 0..12000"),
        (vec!["--flow-body-inset-um", ""], "within 0..12000"),
        (vec!["--flow-body-inset-um"], "requires 0..12000"),
        (vec!["--flow-body-inset-um", "0",
            "--flow-body-inset-um", "1"], "may appear only once"),
    ] {
        let mut command = source.clone();
        command.extend(args(&tokens));
        let error = preflight_notebook_sources(&command)
            .expect_err("late invalid inset cannot consume selected stdin");
        assert!(error.contains(diagnostic), "{tokens:?}: {error}");
    }
    for (flag, value) in [
        ("--flow-text", "a"), ("--ink-file", "not-opened.tsv"),
    ] {
        let mut command = args(&[
            "--export-paper", "pdf", "--glyph-file", "not-opened.tsv",
            flag, value,
        ]);
        command.extend(args(&["--flow-body-inset-um", "0"]));
        assert_eq!(preflight_notebook_sources(&command)
            .expect_err("body inset requires Markdown"),
            "--flow-body-inset-um requires a Markdown flow");
    }

    let target = test_path().with_extension("pdf");
    fs::write(&target, b"keep-existing-PDF").expect("sentinel");
    let mut command = args(&[
        "--printed-sheet", "true", "--export-paper", "pdf",
        "--glyph-stroke", "a", "900", "900", "2200", "3400",
        "250", "#204060", "--flow-markdown", "a",
        "--flow-region", "30000", "40000", "9000", "32000",
        "--flow-body-inset-um", "3000",
    ]);
    command.extend(args(&[
        "--output", target.to_str().expect("test filename"), "--replace",
    ]));
    let error = execute_notebook(&command)
        .expect_err("inset removes usable text width");
    assert_eq!(error, "flow body inset leaves no writing row");
    assert_eq!(fs::read(&target).expect("preserved output"),
        b"keep-existing-PDF");
    fs::remove_file(target).expect("test-owned file cleanup");
}

#[test]
fn selected_markdown_body_right_inset_preserves_structural_anchors() {
    let source = FlowSource::new(FlowFormat::Markdown,
        "# a\n\na a  \na\n\n- a\n\n> a\n\n    a\n\n~~~\na\n~~~")
        .expect("mixed Markdown");
    let paper = NotebookProfile::default();
    let region = FlowRegion {
        left: 30_000, top: 40_000, width: 52_000, height: 185_000,
    };
    let render = |left, right| layout_flow_with_typography(
        &source, &paper, None, HeadingDecoration::default(),
        Some(region), FlowTypography {
            body_inset: left, body_right_inset: right,
            ..FlowTypography::default()
        },
    ).expect("admitted physical layout");
    let baseline = render(None, None);
    let explicit_zero = render(Some(0), Some(0));
    let signature = |(runs, marks, pages):
        (Vec<TextRun>, Vec<NotebookInkLine>, usize)| {
        (runs.into_iter().map(|row| (row.page, row.origin_um, row.text))
            .collect::<Vec<_>>(), marks, pages)
    };
    assert_eq!(signature(baseline), signature(explicit_zero),
        "explicit zero must keep legacy row/ornament bytes");
    let (original, marks, pages) = render(None, None);
    let (right, right_marks, right_pages) = render(None, Some(6_000));
    assert_eq!(pages, right_pages);
    assert_eq!(marks, right_marks);
    assert_eq!(original.len(), right.len());
    for (before, after) in original.iter().zip(&right) {
        assert_eq!((before.page, before.origin_um, &before.text),
            (after.page, after.origin_um, &after.text),
            "right margin only changes width, never the writing origin");
    }
    let (both, both_marks, _) = render(Some(4_000), Some(6_000));
    assert_eq!(marks, both_marks);
    for (index, (before, after)) in original.iter().zip(&both).enumerate() {
        assert_eq!((&before.text, before.page, before.origin_um.1),
            (&after.text, after.page, after.origin_um.1));
        let prose = matches!(index, 1 | 2);
        assert_eq!(after.origin_um.0 - before.origin_um.0,
            if prose { 4_000 } else { 0 });
    }
}

#[test]
fn selected_markdown_body_right_inset_measures_width_and_both_edges() {
    let source = FlowSource::new(FlowFormat::Markdown,
        "a a a a a a a a a a a a").expect("visible prose");
    let paper = NotebookProfile::default();
    let region = FlowRegion {
        left: 30_000, top: 40_000, width: 32_000, height: 32_000,
    };
    let render = |left, right, fixed| layout_flow_with_typography(
        &source, &paper, None, HeadingDecoration::default(),
        Some(fixed), FlowTypography {
            body_inset: left, body_right_inset: right,
            ..FlowTypography::default()
        },
    );
    let (baseline, _, _) = render(None, None, region).expect("default");
    let (right, _, right_pages) = render(None, Some(10_000), region)
        .expect("right margin wraps onto more rows");
    assert!(right.len() > baseline.len());
    assert!(right_pages >= 2);
    assert!(right.iter().all(|row| row.origin_um.0 == 32_500));
    let (both, _, _) = render(Some(6_000), Some(10_000), region)
        .expect("combined inset");
    assert!(both.len() > right.len(), "both edges reduce usable width");
    assert!(both.iter().all(|row| row.origin_um.0 == 38_500));
    let narrow = FlowRegion {
        left: 30_000, top: 40_000, width: 9_000, height: 32_000,
    };
    assert_eq!(render(None, Some(2_000), narrow)
        .expect_err("right inset leaves <4mm"),
        "flow body right inset leaves no writing row");
    let both_narrow = FlowRegion { width: 11_000, ..narrow };
    assert_eq!(render(Some(2_000), Some(2_000), both_narrow)
        .expect_err("combined insets leave <4mm"),
        "flow body right inset leaves no writing row");
    for invalid in [12_001, u32::MAX] {
        assert_eq!(render(None, Some(invalid), region)
            .expect_err("typed maximum"),
            "flow body right inset must be within 0..12000 um");
    }
    let plain = FlowSource::new(FlowFormat::Plain, "a").expect("plain");
    assert_eq!(layout_flow_with_typography(
        &plain, &paper, None, HeadingDecoration::default(),
        Some(region), FlowTypography {
            body_right_inset: Some(0), ..FlowTypography::default()
        },
    ).expect_err("plain flow cannot request right Markdown margin"),
        "flow body right inset requires Markdown flow");
}

#[test]
fn selected_markdown_body_right_inset_native_exports_replay() {
    let base = args(&[
        "--printed-sheet", "true",
        "--glyph-stroke", "a", "900", "900", "2200", "3400",
        "250", "#204060",
        "--flow-markdown", "a a a a a a a a a a a a a a\n\n# a\n\n- a\n\n> a",
        "--flow-region", "30000", "40000", "52000", "100000",
    ]);
    let export = |right: Option<&str>, format: &str| {
        let mut command = base.clone();
        if let Some(value) = right {
            command.extend(args(&["--flow-body-right-inset-um", value]));
        }
        if format == "ink" {
            command.push("--emit-ink-file".into());
        } else {
            command.extend(args(&["--export-paper", format]));
        }
        if format == "png" {
            command.extend(args(&["--dpi", "72"]));
        }
        execute_notebook(&command).expect("native export")
            .expect("PDF/PNG/ink bytes")
    };
    for format in ["pdf", "png", "ink"] {
        let original = export(None, format);
        assert_eq!(original, export(Some("0"), format),
            "explicit zero must keep legacy {format} bytes");
        let selected = export(Some("9000"), format);
        assert_eq!(selected, export(Some("9000"), format),
            "native {format} replay must be deterministic");
        assert_ne!(original, selected,
            "selected right margin must reflow real {format} ink");
        if format == "pdf" {
            assert!(selected.starts_with(b"%PDF-1.4"));
        } else if format == "png" {
            assert!(selected.starts_with(b"\x89PNG\r\n\x1a\n"));
        } else {
            let rows = String::from_utf8(selected).expect("versioned TSV");
            assert!(rows.starts_with("atrament.page-ink.v1\n"));
            assert!(rows.lines().skip(1)
                .any(|line| line.starts_with("1\t")));
        }
    }
}

#[test]
fn invalid_markdown_body_right_inset_preflight_is_atomic() {
    let source = args(&[
        "--flow-markdown-file", "-", "--glyph-file", "not-opened.tsv",
        "--export-paper", "pdf",
    ]);
    for (tokens, diagnostic) in [
        (vec!["--flow-body-right-inset-um", "12001"], "within 0..12000"),
        (vec!["--flow-body-right-inset-um", "-1"], "within 0..12000"),
        (vec!["--flow-body-right-inset-um", "+0"], "within 0..12000"),
        (vec!["--flow-body-right-inset-um", "4294967296"],
            "within 0..12000"),
        (vec!["--flow-body-right-inset-um", ""], "within 0..12000"),
        (vec!["--flow-body-right-inset-um"], "requires 0..12000"),
        (vec!["--flow-body-right-inset-um", "0",
            "--flow-body-right-inset-um", "1"], "may appear only once"),
    ] {
        let mut command = source.clone();
        command.extend(args(&tokens));
        let error = preflight_notebook_sources(&command)
            .expect_err("invalid right margin cannot read selected stdin");
        assert!(error.contains(diagnostic), "{tokens:?}: {error}");
    }
    for (flag, value) in [
        ("--flow-text", "a"), ("--ink-file", "not-opened.tsv"),
    ] {
        let mut command = args(&[
            "--export-paper", "pdf", "--glyph-file", "not-opened.tsv",
            flag, value,
        ]);
        command.extend(args(&["--flow-body-right-inset-um", "0"]));
        assert_eq!(preflight_notebook_sources(&command)
            .expect_err("right margin requires Markdown flow"),
            "--flow-body-right-inset-um requires a Markdown flow");
    }
    let target = test_path().with_extension("pdf");
    fs::write(&target, b"keep-existing-PDF").expect("sentinel");
    let mut command = args(&[
        "--printed-sheet", "true", "--export-paper", "pdf",
        "--glyph-stroke", "a", "900", "900", "2200", "3400",
        "250", "#204060", "--flow-markdown", "a",
        "--flow-region", "30000", "40000", "9000", "32000",
        "--flow-body-right-inset-um", "2000",
    ]);
    command.extend(args(&[
        "--output", target.to_str().expect("test filename"), "--replace",
    ]));
    assert_eq!(execute_notebook(&command)
        .expect_err("right margin removes usable text width"),
        "flow body right inset leaves no writing row");
    assert_eq!(fs::read(&target).expect("output preserved"),
        b"keep-existing-PDF");
    fs::remove_file(target).expect("test-owned sentinel cleanup");
}


#[test]
fn selected_markdown_list_quote_inset_preserves_other_anchors() {
    let source = FlowSource::new(FlowFormat::Markdown,
        "a\n\n- a\n\n1. a\n\n- [x] a\n\n> a\n\n# a\n\n    a\n\n~~~\na\n~~~")
        .expect("mixed Markdown blocks");
    let region = FlowRegion {
        left: 30_000, top: 40_000, width: 52_000, height: 180_000,
    };
    let paper = NotebookProfile::default();
    let render = |extra| layout_flow_with_typography(
        &source, &paper, None, HeadingDecoration::default(),
        Some(region), FlowTypography {
            list_quote_inset: extra, ..FlowTypography::default()
        },
    ).expect("measured Markdown rows");
    let baseline = render(None);
    let explicit_zero = render(Some(0));
    let run_signature = |rows: &[TextRun]| rows.iter()
        .map(|row| (row.page, row.origin_um, row.text.clone()))
        .collect::<Vec<_>>();
    assert_eq!(run_signature(&baseline.0), run_signature(&explicit_zero.0));
    assert_eq!(baseline.1, explicit_zero.1);
    assert_eq!(baseline.2, explicit_zero.2);
    let (rows, marks, pages) = baseline;
    let (shifted, moved_marks, moved_pages) = render(Some(4_000));
    assert_eq!(pages, moved_pages);
    assert_eq!(rows.len(), shifted.len());
    assert_eq!(marks.len(), moved_marks.len());
    assert_eq!(rows.len(), 8);
    for (index, (original, moved)) in rows.iter().zip(&shifted).enumerate() {
        assert_eq!((&original.text, original.page, original.origin_um.1),
            (&moved.text, moved.page, moved.origin_um.1));
        let structural = matches!(index, 1 | 2 | 3 | 4);
        assert_eq!(moved.origin_um.0 - original.origin_um.0,
            if structural { 4_000 } else { 0 },
            "only numbered/bullet/task/quote text moves");
    }
    // The title underline is last; list marks and the quote rule move
    // with their text. Marker widths/colors and page positions do not.
    for (index, (original, moved)) in marks.iter()
        .zip(&moved_marks).enumerate() {
        let shift = if index + 1 == marks.len() { 0 } else { 4_000 };
        assert_eq!(moved.page, original.page);
        assert_eq!(moved.width_um, original.width_um);
        assert_eq!(moved.rgb, original.rgb);
        assert_eq!(moved.start_um.1, original.start_um.1);
        assert_eq!(moved.end_um.1, original.end_um.1);
        assert_eq!(moved.start_um.0 - original.start_um.0, shift);
        assert_eq!(moved.end_um.0 - original.end_um.0, shift);
    }
}

#[test]
fn selected_markdown_list_quote_inset_wraps_and_refuses_narrow_regions() {
    let source = FlowSource::new(FlowFormat::Markdown,
        "- a a a a a a a a a a a a a a").expect("long bullet");
    let paper = NotebookProfile::default();
    let region = FlowRegion {
        left: 30_000, top: 40_000, width: 28_000, height: 32_000,
    };
    let render = |source: &FlowSource, inset, region| {
        layout_flow_with_typography(
            source, &paper, None, HeadingDecoration::default(),
            Some(region), FlowTypography {
                list_quote_inset: inset, ..FlowTypography::default()
            },
        )
    };
    let (legacy, old_marks, _) = render(&source, None, region)
        .expect("legacy wrapping");
    let (rows, new_marks, pages) = render(&source, Some(9_000), region)
        .expect("extra structural inset wraps more");
    assert!(rows.len() > legacy.len());
    assert!(pages >= 2);
    assert!(rows.iter().all(|row| row.origin_um.0 == 47_500));
    assert_eq!(new_marks.len(), 1, "continuations have no bullets");
    assert_eq!(new_marks[0].start_um.0 - old_marks[0].start_um.0,
        9_000);
    let narrow = FlowRegion { width: 14_000, ..region };
    assert_eq!(render(&source, Some(1_000), narrow)
        .expect_err("selected structural ink must leave a full row"),
        "flow horizontal printable region too narrow");
    for bad in [12_001, u32::MAX] {
        assert_eq!(render(&source, Some(bad), region)
            .expect_err("typed maximum"),
            "flow list/quote inset must be within 0..12000 um");
    }
    let plain = FlowSource::new(FlowFormat::Plain, "a")
        .expect("plain source");
    assert_eq!(render(&plain, Some(0), region)
        .expect_err("plain cannot request a structural Markdown inset"),
        "flow list/quote inset requires Markdown flow");
}

#[test]
fn selected_markdown_list_quote_inset_native_exports_replay() {
    let base = args(&[
        "--printed-sheet", "true", "--glyph-stroke", "a",
        "900", "900", "2200", "3400", "250", "#204060",
        "--flow-markdown",
        "- a a a a a a a a a a a a\n\n> a a a\n\n- [x] a\n\n# a",
        "--flow-region", "30000", "40000", "32000", "90000",
    ]);
    let export = |inset: Option<&str>, format: &str| {
        let mut command = base.clone();
        if let Some(value) = inset {
            command.extend(args(&["--flow-list-quote-inset-um", value]));
        }
        if format == "ink" {
            command.push("--emit-ink-file".into());
        } else {
            command.extend(args(&["--export-paper", format]));
        }
        if format == "png" {
            command.extend(args(&["--dpi", "72"]));
        }
        execute_notebook(&command).expect("native export")
            .expect("selected output bytes")
    };
    for format in ["pdf", "png", "ink"] {
        let original = export(None, format);
        assert_eq!(original, export(Some("0"), format),
            "explicit zero keeps {format} byte identity");
        let shifted = export(Some("6000"), format);
        assert_eq!(shifted, export(Some("6000"), format),
            "selected {format} must replay deterministically");
        assert_ne!(original, shifted, "measured ink shifts in {format}");
        if format == "pdf" {
            assert!(shifted.starts_with(b"%PDF-1.4"));
        } else if format == "png" {
            assert!(shifted.starts_with(b"\x89PNG\r\n\x1a\n"));
        } else {
            let tsv = String::from_utf8(shifted).expect("versioned ink");
            assert!(tsv.starts_with("atrament.page-ink.v1\n"));
            assert!(tsv.lines().skip(1).any(|line| line.starts_with("1\t")));
        }
    }
}

#[test]
fn invalid_markdown_list_quote_inset_preflight_is_atomic() {
    let source = args(&[
        "--flow-markdown-file", "-", "--glyph-file", "unopened.tsv",
        "--export-paper", "pdf",
    ]);
    for (tokens, expected) in [
        (vec!["--flow-list-quote-inset-um", "12001"], "within 0..12000"),
        (vec!["--flow-list-quote-inset-um", "-1"], "within 0..12000"),
        (vec!["--flow-list-quote-inset-um", "+0"], "within 0..12000"),
        (vec!["--flow-list-quote-inset-um", "4294967296"],
            "within 0..12000"),
        (vec!["--flow-list-quote-inset-um", ""], "within 0..12000"),
        (vec!["--flow-list-quote-inset-um"], "requires 0..12000"),
        (vec!["--flow-list-quote-inset-um", "0",
            "--flow-list-quote-inset-um", "1"], "may appear only once"),
    ] {
        let mut command = source.clone();
        command.extend(args(&tokens));
        let error = preflight_notebook_sources(&command)
            .expect_err("invalid option cannot read selected stdin");
        assert!(error.contains(expected), "{tokens:?}: {error}");
    }
    let mut plain = args(&[
        "--flow-text", "a", "--export-paper", "pdf",
        "--glyph-file", "unopened.tsv",
    ]);
    plain.extend(args(&["--flow-list-quote-inset-um", "0"]));
    assert_eq!(preflight_notebook_sources(&plain)
        .expect_err("list/quote inset needs Markdown"),
        "--flow-list-quote-inset-um requires a Markdown flow");

    let target = test_path().with_extension("pdf");
    fs::write(&target, b"keep-existing-PDF").expect("sentinel");
    let mut command = args(&[
        "--printed-sheet", "true", "--export-paper", "pdf",
        "--glyph-stroke", "a", "900", "900", "2200", "3400",
        "250", "#204060", "--flow-markdown", "- a",
        "--flow-region", "30000", "40000", "14000", "32000",
        "--flow-list-quote-inset-um", "1000",
    ]);
    command.extend(args(&[
        "--output", target.to_str().expect("test filename"), "--replace",
    ]));
    assert_eq!(execute_notebook(&command)
        .expect_err("structural inset removes usable row"),
        "flow horizontal printable region too narrow");
    assert_eq!(fs::read(&target).expect("preserved existing PDF"),
        b"keep-existing-PDF");
    fs::remove_file(target).expect("test-owned file cleanup");
}


#[test]
fn selected_markdown_list_quote_right_inset_preserves_marks() {
    let source = FlowSource::new(FlowFormat::Markdown,
        concat!("a\n\n- a a\n\n1. a a\n\n- [x] a\n\n> a a\n\n# a\n\n",
            "    a\n\n~~~\na\n~~~"))
        .expect("mixed structural Markdown");
    let region = FlowRegion {
        left: 30_000, top: 40_000, width: 62_000, height: 180_000,
    };
    let paper = NotebookProfile::default();
    let render = |extra| layout_flow_with_typography(
        &source, &paper, None, HeadingDecoration::default(),
        Some(region), FlowTypography {
            list_quote_right_inset: extra, ..FlowTypography::default()
        },
    ).expect("admitted structural rows");
    let original = render(None);
    let zero = render(Some(0));
    let signature = |rows: &[TextRun]| rows.iter()
        .map(|row| (row.page, row.origin_um, row.text.clone()))
        .collect::<Vec<_>>();
    assert_eq!(signature(&original.0), signature(&zero.0));
    assert_eq!(original.1, zero.1);
    assert_eq!(original.2, zero.2);
    let selected = render(Some(6_000));
    assert_eq!(signature(&original.0), signature(&selected.0),
        "right reserve cannot move short rows or authored anchors");
    assert_eq!(original.1, selected.1,
        "right reserve cannot displace bullets, tasks, bars or titles");
    assert_eq!(original.2, selected.2);
}

#[test]
fn selected_markdown_list_quote_right_inset_wraps_across_pages() {
    let paper = NotebookProfile::default();
    let region = FlowRegion {
        left: 30_000, top: 40_000, width: 36_000, height: 32_000,
    };
    let source = FlowSource::new(FlowFormat::Markdown,
        "a\n\n- a a a a a a a a a a a a a a\n\n> a a a a a a a a")
        .expect("two structural paragraphs");
    let render = |right, left| layout_flow_with_typography(
        &source, &paper, None, HeadingDecoration::default(),
        Some(region), FlowTypography {
            list_quote_inset: left, list_quote_right_inset: right,
            ..FlowTypography::default()
        },
    );
    let legacy = render(None, None).expect("legacy physical rows");
    let constrained = render(Some(8_000), Some(4_000))
        .expect("combined structural insets");
    assert!(constrained.0.len() > legacy.0.len(),
        "less writable width must produce more structural rows");
    assert!(constrained.2 > legacy.2,
        "extra wraps consume more physical pages");
    assert_eq!(constrained.0[0].page, legacy.0[0].page);
    assert_eq!(constrained.0[0].origin_um, legacy.0[0].origin_um);
    let structural = constrained.0.iter().filter(|row| row.text != "a")
        .collect::<Vec<_>>();
    assert!(structural.len() >= 3);
    assert!(structural.iter().all(|row| row.origin_um.0 == 42_500),
        "right reserve only removes capacity; left inset moves anchors");
    assert!(constrained.1.iter().all(|mark|
        mark.start_um.0 > region.left && mark.end_um.0 > region.left));
    assert_eq!(render(Some(12_001), Some(0))
        .expect_err("typed upper bound"),
        "flow list/quote right inset must be within 0..12000 um");
    let narrow = FlowRegion { width: 22_000, ..region };
    let source = FlowSource::new(FlowFormat::Markdown, "> a")
        .expect("quoted ink");
    assert_eq!(layout_flow_with_typography(
        &source, &paper, None, HeadingDecoration::default(),
        Some(narrow), FlowTypography {
            list_quote_inset: Some(6_000),
            list_quote_right_inset: Some(8_000),
            ..FlowTypography::default()
        },
    ).expect_err("combined structural insets must leave a writing row"),
        "flow list/quote right inset leaves no writing row");
    let plain = FlowSource::new(FlowFormat::Plain, "a")
        .expect("plain source");
    assert_eq!(layout_flow_with_typography(
        &plain, &paper, None, HeadingDecoration::default(),
        Some(region), FlowTypography {
            list_quote_right_inset: Some(0), ..FlowTypography::default()
        },
    ).expect_err("typed option requires Markdown"),
        "flow list/quote right inset requires Markdown flow");
}

#[test]
fn selected_markdown_list_quote_right_inset_native_exports_replay() {
    let base = args(&[
        "--printed-sheet", "true", "--glyph-stroke", "a",
        "900", "900", "2200", "3400", "250", "#204060",
        "--flow-markdown",
        "- a a a a a a a a a a a a a\n\n> a a a a\n\n- [x] a",
        "--flow-region", "30000", "40000", "32000", "90000",
    ]);
    let export = |selected: Option<&str>, format: &str| {
        let mut command = base.clone();
        if let Some(right) = selected {
            command.extend(args(&[
                "--flow-list-quote-right-inset-um", right,
            ]));
        }
        if format == "ink" {
            command.push("--emit-ink-file".into());
        } else {
            command.extend(args(&["--export-paper", format]));
        }
        if format == "png" {
            command.extend(args(&["--dpi", "72"]));
        }
        execute_notebook(&command).expect("native export")
            .expect("selected output bytes")
    };
    for format in ["pdf", "png", "ink"] {
        let original = export(None, format);
        assert_eq!(original, export(Some("0"), format),
            "explicit zero must preserve {format} bytes");
        let chosen = export(Some("6000"), format);
        assert_eq!(chosen, export(Some("6000"), format),
            "structural right reserve must replay exact {format} bytes");
        assert_ne!(original, chosen, "physical wraps affect {format}");
        if format == "pdf" {
            assert!(chosen.starts_with(b"%PDF-1.4"));
        } else if format == "png" {
            assert!(chosen.starts_with(b"\x89PNG\r\n\x1a\n"));
        } else {
            let tsv = String::from_utf8(chosen).expect("versioned ink");
            assert!(tsv.starts_with("atrament.page-ink.v1\n"));
            assert!(tsv.lines().skip(1).any(|line| line.starts_with("1\t")));
        }
    }
}

#[test]
fn invalid_markdown_list_quote_right_inset_is_preflight_atomic() {
    let source = args(&[
        "--flow-markdown-file", "-", "--glyph-file", "unopened.tsv",
        "--export-paper", "pdf",
    ]);
    for (tokens, expected) in [
        (vec!["--flow-list-quote-right-inset-um", "12001"],
            "within 0..12000"),
        (vec!["--flow-list-quote-right-inset-um", "-1"],
            "within 0..12000"),
        (vec!["--flow-list-quote-right-inset-um", "+0"],
            "within 0..12000"),
        (vec!["--flow-list-quote-right-inset-um", "4294967296"],
            "within 0..12000"),
        (vec!["--flow-list-quote-right-inset-um", ""],
            "within 0..12000"),
        (vec!["--flow-list-quote-right-inset-um"],
            "requires 0..12000"),
        (vec!["--flow-list-quote-right-inset-um", "0",
            "--flow-list-quote-right-inset-um", "1"],
            "may appear only once"),
    ] {
        let mut command = source.clone();
        command.extend(args(&tokens));
        let error = preflight_notebook_sources(&command)
            .expect_err("invalid option refuses before reading stdin");
        assert!(error.contains(expected), "{tokens:?}: {error}");
    }
    let mut plain = args(&[
        "--flow-text", "a", "--export-paper", "pdf",
        "--glyph-file", "unopened.tsv",
    ]);
    plain.extend(args(&["--flow-list-quote-right-inset-um", "0"]));
    assert_eq!(preflight_notebook_sources(&plain)
        .expect_err("right reserve requires Markdown"),
        "--flow-list-quote-right-inset-um requires a Markdown flow");

    let target = test_path().with_extension("pdf");
    fs::write(&target, b"keep-existing-PDF").expect("test-owned sentinel");
    let mut command = args(&[
        "--printed-sheet", "true", "--export-paper", "pdf",
        "--glyph-stroke", "a", "900", "900", "2200", "3400",
        "250", "#204060", "--flow-markdown", "- a",
        "--flow-region", "30000", "40000", "20000", "32000",
        "--flow-list-quote-inset-um", "6000",
        "--flow-list-quote-right-inset-um", "8000",
    ]);
    command.extend(args(&[
        "--output", target.to_str().expect("test path"), "--replace",
    ]));
    assert_eq!(execute_notebook(&command)
        .expect_err("narrow selected region must refuse"),
        "flow list/quote right inset leaves no writing row");
    assert_eq!(fs::read(&target).expect("existing PDF preserved"),
        b"keep-existing-PDF");
    fs::remove_file(target).expect("test-owned cleanup");
}

#[test]
fn markdown_heading_alignment_moves_titles_and_ornaments_only() {
    let source = FlowSource::new(
        FlowFormat::Markdown, "# a\n\n- a\n\n> a\n\na\n===",
    ).expect("ATX, structural rows and Setext heading");
    let region = FlowRegion {
        left: 30_000, top: 40_000, width: 60_000, height: 160_000,
    };
    let paper = NotebookProfile::default();
    let render = |alignment: Option<&str>| {
        let mut heading = HeadingDecoration::default();
        if let Some(alignment) = alignment {
            heading.parse("--heading-align", &mut args(&[alignment]).iter())
                .expect("admitted title alignment");
        }
        layout_flow_with_typography(
            &source, &paper, None, heading, Some(region),
            FlowTypography::default(),
        ).expect("physical heading layout")
    };
    let legacy = render(None);
    let left = render(Some("left"));
    assert_eq!(legacy.0.len(), 4);
    let rows = |runs: &[TextRun]| runs.iter()
        .map(|row| (row.page, row.origin_um, row.text.clone()))
        .collect::<Vec<_>>();
    assert_eq!(rows(&legacy.0), rows(&left.0),
        "explicit left keeps all physical text rows");
    assert_eq!(legacy.1, left.1, "explicit left keeps every pen mark");
    assert_eq!(legacy.2, left.2);
    assert_eq!(legacy.0.iter().map(|row| row.text.as_str())
        .collect::<Vec<_>>(), ["a", "a", "a", "a"]);
    let baseline_x = region.left + 2_500;
    let capacity = region.width - 2_500 - 1_500;
    let heading_ink_width = 4_000;
    for (selected, offset) in [
        ("center", (capacity - heading_ink_width) / 2),
        ("right", capacity - heading_ink_width),
    ] {
        let moved = render(Some(selected));
        assert_eq!(moved.2, legacy.2);
        assert_eq!(moved.0.len(), legacy.0.len());
        for (index, row) in moved.0.iter().enumerate() {
            assert_eq!(row.page, legacy.0[index].page);
            assert_eq!(row.origin_um.1, legacy.0[index].origin_um.1);
            let shift = if matches!(index, 0 | 3) { offset } else { 0 };
            assert_eq!(row.origin_um.0, baseline_x
                + if matches!(index, 1 | 2) { 6_000 } else { shift },
                "row {index} in {selected}");
        }
        assert_eq!(moved.1.len(), legacy.1.len());
        for (original, shifted) in legacy.1.iter().zip(&moved.1) {
            let heading_ornament = original.width_um == 550;
            let shift = if heading_ornament { offset } else { 0 };
            assert_eq!(shifted.start_um.0, original.start_um.0 + shift);
            assert_eq!(shifted.end_um.0, original.end_um.0 + shift);
            assert_eq!(shifted.start_um.1, original.start_um.1);
            assert_eq!(shifted.end_um.1, original.end_um.1);
            assert_eq!(shifted.width_um, original.width_um);
        }
    }
}

#[test]
fn markdown_heading_alignment_keeps_page_turns_and_exact_contours() {
    let paper = NotebookProfile::default();
    let region = FlowRegion {
        left: 30_000, top: 40_000, width: 30_000, height: 19_000,
    };
    let source = FlowSource::new(
        FlowFormat::Markdown, "# a\n\n## a\n\na\n===",
    ).expect("multiple title levels");
    let render = |alignment| {
        let mut heading = HeadingDecoration::default();
        heading.parse("--heading-align", &mut args(&[alignment]).iter())
            .expect("selected alignment");
        layout_flow_with_typography(
            &source, &paper, None, heading, Some(region),
            FlowTypography::default(),
        ).expect("each heading on its own physical page")
    };
    let left = render("left");
    let center = render("center");
    let right = render("right");
    assert_eq!(left.2, 3);
    let baseline_x = region.left + 2_500;
    let cap = region.width - 2_500 - 1_500;
    for (index, row) in left.0.iter().enumerate() {
        assert_eq!(row.page, index + 1);
        assert_eq!(center.0[index].page, row.page);
        assert_eq!(right.0[index].page, row.page);
        assert_eq!(center.0[index].origin_um.0,
            baseline_x + (cap - 4_000) / 2);
        assert_eq!(right.0[index].origin_um.0,
            baseline_x + cap - 4_000);
        assert_eq!(center.0[index].origin_um.1, row.origin_um.1);
        assert_eq!(right.0[index].origin_um.1, row.origin_um.1);
    }
    for (original, changed) in left.1.iter().zip(&right.1) {
        assert_eq!(changed.page, original.page);
        assert_eq!(changed.start_um.0 - original.start_um.0,
            cap - 4_000);
        assert_eq!(changed.end_um.0 - original.end_um.0,
            cap - 4_000);
    }
}

#[test]
fn selected_markdown_heading_alignment_replays_native_exports() {
    let base = args(&[
        "--printed-sheet", "true",
        "--glyph-stroke", "a", "900", "900", "2200", "3400",
        "250", "#204060",
        "--flow-markdown", "# a\n\n## a\n\na\n===",
        "--flow-region", "30000", "40000", "52000", "90000",
        "--heading-style", "marker", "--heading-color", "#c5b4e6",
    ]);
    let export = |align: Option<&str>, format: &str| {
        let mut command = base.clone();
        if let Some(align) = align {
            command.extend(args(&["--heading-align", align]));
        }
        if format == "ink" {
            command.push("--emit-ink-file".into());
        } else {
            command.extend(args(&["--export-paper", format]));
        }
        if format == "png" {
            command.extend(args(&["--dpi", "72"]));
        }
        execute_notebook(&command).expect("native heading export")
            .expect("PDF PNG or TSV bytes")
    };
    for format in ["pdf", "png", "ink"] {
        let original = export(None, format);
        assert_eq!(original, export(Some("left"), format),
            "explicit left preserves {format} bytes");
        let centered = export(Some("center"), format);
        let right = export(Some("right"), format);
        assert_eq!(centered, export(Some("center"), format));
        assert_eq!(right, export(Some("right"), format));
        assert_ne!(centered, original, "center changes {format} ink");
        assert_ne!(right, centered, "right changes {format} ink");
        if format == "pdf" {
            assert!(right.starts_with(b"%PDF-1.4"));
        } else if format == "png" {
            assert!(right.starts_with(b"\x89PNG\r\n\x1a\n"));
        } else {
            let tsv = String::from_utf8(right).expect("valid ink TSV");
            assert!(tsv.starts_with("atrament.page-ink.v1\n"));
            assert!(tsv.lines().skip(1).any(|line| line.starts_with("1\t")));
        }
    }
}

#[test]
fn invalid_heading_alignment_preflights_and_preserves_existing_pdf() {
    let source = args(&[
        "--flow-markdown-file", "-", "--glyph-file", "unopened.tsv",
        "--export-paper", "pdf",
    ]);
    for (tokens, expected) in [
        (vec!["--heading-align", "middle"], "requires left, center or right"),
        (vec!["--heading-align", ""], "requires left, center or right"),
        (vec!["--heading-align"], "requires a heading setting"),
        (vec!["--heading-align", "right", "--heading-align", "left"],
            "may appear only once"),
    ] {
        let mut command = source.clone();
        command.extend(args(&tokens));
        let error = preflight_notebook_sources(&command)
            .expect_err("invalid heading alignment must precede source IO");
        assert!(error.contains(expected), "{tokens:?}: {error}");
    }
    let mut plain = args(&[
        "--flow-text", "a", "--glyph-file", "unopened.tsv",
        "--export-paper", "pdf", "--heading-align", "left",
    ]);
    assert_eq!(preflight_notebook_sources(&plain)
        .expect_err("Markdown-only heading alignment"),
        "heading customization requires --flow-markdown");
    plain[0] = "--flow-markdown".into();
    assert!(preflight_notebook_sources(&plain).is_ok());

    let target = test_path().with_extension("pdf");
    fs::write(&target, b"keep-existing-heading-PDF").expect("sentinel");
    let mut command = args(&[
        "--printed-sheet", "true", "--export-paper", "pdf",
        "--glyph-stroke", "a", "900", "900", "2200", "3400",
        "250", "#204060", "--flow-markdown", "# a a",
        "--flow-region", "30000", "40000", "12000", "32000",
        "--heading-align", "right",
    ]);
    command.extend(args(&[
        "--output", target.to_str().expect("path"), "--replace",
    ]));
    let error = execute_notebook(&command)
        .expect_err("heading must refuse its one-line fixed region");
    assert!(error.contains("heading exceeds one-line fixed region"),
        "{error}");
    assert_eq!(fs::read(&target).expect("preserved PDF"),
        b"keep-existing-heading-PDF");
    fs::remove_file(target).expect("clean test-owned sentinel");
}

#[test]
fn markdown_body_alignment_respects_prose_rows_and_two_insets() {
    let source = FlowSource::new(FlowFormat::Markdown,
        "a a a a a a\n\n# a\n\n- a\n\n> a\n\n    a",
    ).expect("flowing prose, heading, list, quote and literal row");
    let region = FlowRegion {
        left: 30_000, top: 40_000, width: 30_000, height: 180_000,
    };
    let render = |align| layout_flow_with_typography(
        &source, &NotebookProfile::default(), None,
        HeadingDecoration::default(), Some(region),
        FlowTypography {
            body_align: align,
            body_inset: Some(4_000),
            body_right_inset: Some(6_000),
            ..FlowTypography::default()
        },
    ).expect("selected prose stays in the physical region");
    let original = render(None);
    let left = render(Some(BodyAlign::Left));
    assert_eq!(original.0.len(), 7);
    assert_eq!(original.0.iter().map(|row| row.text.as_str())
        .collect::<Vec<_>>(),
        ["a a", "a a", "a a", "a", "a", "a", "    a"]);
    assert_eq!(original.0.iter().map(|row| row.origin_um)
        .collect::<Vec<_>>(),
        left.0.iter().map(|row| row.origin_um)
            .collect::<Vec<_>>());
    assert_eq!(original.1, left.1);
    for (align, shift) in [
        (BodyAlign::Center, 2_000), (BodyAlign::Right, 4_000),
    ] {
        let moved = render(Some(align));
        assert_eq!(moved.2, original.2);
        assert_eq!(moved.1, original.1,
            "only flowing prose shifts, never heading/list/quote ornaments");
        for (index, row) in moved.0.iter().enumerate() {
            let earlier = &original.0[index];
            assert_eq!(row.text, earlier.text);
            assert_eq!(row.page, earlier.page);
            assert_eq!(row.origin_um.1, earlier.origin_um.1);
            let prose = index < 3;
            assert_eq!(row.origin_um.0, earlier.origin_um.0
                + if prose { shift } else { 0 });
        }
    }
}

#[test]
fn selected_markdown_body_alignment_replays_pdf_png_and_ink_tsv() {
    let base = args(&[
        "--printed-sheet", "true",
        "--glyph-stroke", "a", "900", "900", "2200", "3400",
        "250", "#204060",
        "--flow-markdown", "a a a a a a\n\n# a\n\n- a\n\n> a",
        "--flow-region", "30000", "40000", "30000", "90000",
        "--flow-body-inset-um", "4000",
        "--flow-body-right-inset-um", "6000",
    ]);
    let render = |align: Option<&str>, format: &str| {
        let mut command = base.clone();
        if let Some(align) = align {
            command.extend(args(&["--flow-body-align", align]));
        }
        if format == "ink" {
            command.push("--emit-ink-file".into());
        } else {
            command.extend(args(&["--export-paper", format]));
        }
        if format == "png" {
            command.extend(args(&["--dpi", "72"]));
        }
        execute_notebook(&command).expect("selected body export")
            .expect("PDF, PNG or pen TSV bytes")
    };
    for format in ["pdf", "png", "ink"] {
        let original = render(None, format);
        assert_eq!(original, render(Some("left"), format),
            "explicit left leaves native {format} bytes unchanged");
        let center = render(Some("center"), format);
        let right = render(Some("right"), format);
        assert_eq!(center, render(Some("center"), format));
        assert_eq!(right, render(Some("right"), format));
        assert_ne!(center, original, "center moves {format} prose ink");
        assert_ne!(right, center, "right moves {format} prose ink");
        if format == "pdf" {
            assert!(right.starts_with(b"%PDF-1.4"));
        } else if format == "png" {
            assert!(right.starts_with(b"\x89PNG\r\n\x1a\n"));
        } else {
            assert!(right.starts_with(b"atrament.page-ink.v1\n"));
        }
    }
}

#[test]
fn invalid_markdown_body_alignment_preflights_and_preserves_pdf() {
    let source = args(&[
        "--flow-markdown-file", "-", "--glyph-file", "unopened.tsv",
        "--export-paper", "pdf",
    ]);
    for tokens in [
        vec!["--flow-body-align", "middle"],
        vec!["--flow-body-align", ""],
        vec!["--flow-body-align"],
    ] {
        let mut command = source.clone();
        command.extend(args(&tokens));
        assert_eq!(preflight_notebook_sources(&command)
            .expect_err("bad alignment must refuse without opening stdin"),
            "--flow-body-align requires left, center or right");
    }
    let mut duplicate = source.clone();
    duplicate.extend(args(&["--flow-body-align", "left",
        "--flow-body-align", "right"]));
    assert_eq!(preflight_notebook_sources(&duplicate)
        .expect_err("duplicate alignment must refuse without reading file"),
        "--flow-body-align may appear only once");
    let mut plain = args(&[
        "--flow-text", "a", "--export-paper", "pdf",
        "--glyph-file", "unopened.tsv", "--flow-body-align", "left",
    ]);
    assert_eq!(preflight_notebook_sources(&plain)
        .expect_err("body alignment is Markdown-only"),
        "--flow-body-align requires a Markdown flow");
    plain[0] = "--flow-markdown".into();
    assert!(preflight_notebook_sources(&plain).is_ok());

    let target = test_path().with_extension("pdf");
    fs::write(&target, b"keep-existing-body-PDF").expect("test sentinel");
    let mut command = args(&[
        "--printed-sheet", "true", "--export-paper", "pdf",
        "--glyph-stroke", "a", "900", "900", "2200", "3400",
        "250", "#204060", "--flow-markdown", "a a",
        "--flow-region", "30000", "40000", "27000", "32000",
        "--flow-body-inset-um", "12000",
        "--flow-body-right-inset-um", "12000",
        "--flow-body-align", "right",
    ]);
    command.extend(args(&[
        "--output", target.to_str().expect("test path"), "--replace",
    ]));
    let err = execute_notebook(&command)
        .expect_err("unfit body alignment must refuse physical layout");
    assert!(err.contains("flow body inset leaves no writing row")
        || err.contains("flow body right inset leaves no writing row"),
        "{err}");
    assert_eq!(fs::read(&target).expect("unchanged sentinel"),
        b"keep-existing-body-PDF");
    fs::remove_file(target).expect("clean test-owned sentinel");
}

#[test]
fn markdown_body_alignment_measures_compressed_nominal_ink_contours()
-> Result<(), String> {
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
    let source = FlowSource::new(FlowFormat::Markdown, "a a")?;
    let region = FlowRegion {
        left: 30_000, top: 40_000, width: 50_000, height: 40_000,
    };
    for (alignment, shift) in [
        (BodyAlign::Left, 0), (BodyAlign::Center, 17_000),
        (BodyAlign::Right, 34_000),
    ] {
        let (rows, marks, pages) = layout_flow_with_typography(
            &source, &NotebookProfile::default(), Some(&stats),
            HeadingDecoration::default(), Some(region),
            FlowTypography {
                body_align: Some(alignment),
                ..FlowTypography::default()
            },
        )?;
        assert_eq!(pages, 1);
        assert!(marks.is_empty());
        assert_eq!(rows.len(), 2);
        for (index, row) in rows.iter().enumerate() {
            assert_eq!(row.text, "a");
            assert_eq!(row.origin_um, (
                32_500 + shift, 42_500 + 9_000 * index as u32,
            ));
            assert!(row.origin_um.0 + 12_000 <= 80_000,
                "actual 12 mm contour must fit after alignment");
        }
    }
    let plain = FlowSource::new(FlowFormat::Plain, "a")?;
    assert_eq!(layout_flow_with_typography(
        &plain, &NotebookProfile::default(), Some(&stats),
        HeadingDecoration::default(), Some(region),
        FlowTypography {
            body_align: Some(BodyAlign::Left),
            ..FlowTypography::default()
        },
    ).expect_err("typed plain-text body alignment refused"),
        "flow body alignment requires Markdown flow");
    Ok(())
}

#[test]
fn markdown_body_alignment_tracks_wrapped_rows_across_physical_pages() {
    let source = FlowSource::new(FlowFormat::Markdown,
        "a a a a a a a a a a a a",
    ).expect("one long authored paragraph");
    let region = FlowRegion {
        left: 30_000, top: 40_000, width: 30_000, height: 29_000,
    };
    let render = |align| layout_flow_with_typography(
        &source, &NotebookProfile::default(), None,
        HeadingDecoration::default(), Some(region),
        FlowTypography {
            body_align: align,
            body_inset: Some(4_000),
            body_right_inset: Some(6_000),
            ..FlowTypography::default()
        },
    ).expect("bounded multi-page prose");
    let original = render(None);
    assert_eq!(original.2, 3);
    assert_eq!(original.0.len(), 6);
    assert_eq!(original.0.iter().map(|row| row.page)
        .collect::<Vec<_>>(), [1, 1, 2, 2, 3, 3]);
    for (align, shift) in [
        (BodyAlign::Center, 2_000), (BodyAlign::Right, 4_000),
    ] {
        let changed = render(Some(align));
        assert_eq!(changed.2, original.2);
        assert_eq!(changed.1, original.1);
        for (before, after) in original.0.iter().zip(&changed.0) {
            assert_eq!(after.text, before.text);
            assert_eq!(after.page, before.page);
            assert_eq!(after.origin_um.1, before.origin_um.1);
            assert_eq!(after.origin_um.0, before.origin_um.0 + shift);
        }
    }
}

#[test]
fn markdown_first_line_indent_wraps_only_first_prose_row() {
    let source = FlowSource::new(FlowFormat::Markdown,
        "a a a a a a\n\n# a\n\n- a\n\n> a\n\n    a",
    ).expect("prose and structural blocks");
    let region = FlowRegion {
        left: 30_000, top: 40_000, width: 30_000, height: 180_000,
    };
    let render = |indent, align| layout_flow_with_typography(
        &source, &NotebookProfile::default(), None,
        HeadingDecoration::default(), Some(region),
        FlowTypography {
            body_first_line_indent: indent,
            body_align: align,
            body_inset: Some(4_000),
            body_right_inset: Some(6_000),
            ..FlowTypography::default()
        },
    ).expect("bounded first line and structural ink");
    let unchanged = render(None, None);
    assert_eq!(unchanged.0.iter().map(|row| row.text.as_str())
        .collect::<Vec<_>>(),
        ["a a", "a a", "a a", "a", "a", "a", "    a"]);
    let zero = render(Some(0), None);
    assert_eq!(zero.0.iter().map(|row| (row.text.as_str(),
        row.origin_um, row.page)).collect::<Vec<_>>(),
        unchanged.0.iter().map(|row| (row.text.as_str(),
            row.origin_um, row.page)).collect::<Vec<_>>());
    assert_eq!(zero.1, unchanged.1);
    let indented = render(Some(8_000), None);
    assert_eq!(indented.0.iter().map(|row| row.text.as_str())
        .collect::<Vec<_>>(),
        ["a", "a a", "a a", "a", "a", "a", "a", "    a"]);
    assert_eq!(indented.0.iter().map(|row| row.origin_um.0)
        .collect::<Vec<_>>(),
        [44_500, 36_500, 36_500, 36_500, 32_500, 38_500, 38_500,
            32_500]);
    assert_eq!(indented.1.iter().map(|mark| (mark.start_um.0,
        mark.end_um.0, mark.width_um, mark.rgb)).collect::<Vec<_>>(),
        unchanged.1.iter().map(|mark| (mark.start_um.0,
            mark.end_um.0, mark.width_um, mark.rgb))
            .collect::<Vec<_>>(),
        "structural ornaments keep their x anchor and ink style");
    for (align, first, continuation, final_row) in [
        (BodyAlign::Center, 46_500, 38_500, 42_500),
        (BodyAlign::Right, 48_500, 40_500, 48_500),
    ] {
        let moved = render(Some(8_000), Some(align));
        assert_eq!(moved.1, indented.1,
            "alignment must not shift structural decoration");
        assert_eq!(moved.0.iter().map(|row| row.origin_um.0)
            .collect::<Vec<_>>(), [first, continuation, continuation,
                final_row, 32_500, 38_500, 38_500, 32_500]);
        assert_eq!(moved.0.iter().map(|row| row.text.as_str())
            .collect::<Vec<_>>(),
            indented.0.iter().map(|row| row.text.as_str())
                .collect::<Vec<_>>());
    }
}

#[test]
fn markdown_first_line_indent_applies_once_across_page_turns() {
    let source = FlowSource::new(FlowFormat::Markdown,
        "a a a a a a a a a a a a",
    ).expect("multi-page flowing paragraph");
    let region = FlowRegion {
        left: 30_000, top: 40_000, width: 30_000, height: 29_000,
    };
    let (rows, _, pages) = layout_flow_with_typography(
        &source, &NotebookProfile::default(), None,
        HeadingDecoration::default(), Some(region),
        FlowTypography {
            body_first_line_indent: Some(8_000),
            body_inset: Some(4_000),
            body_right_inset: Some(6_000),
            ..FlowTypography::default()
        },
    ).expect("page-boundary paragraph");
    assert!(pages >= 3);
    assert_eq!(rows.first().expect("first physical row").origin_um.0,
        44_500);
    assert_eq!(rows.first().expect("first physical row").text, "a");
    assert!(rows.iter().skip(1).all(|row| row.origin_um.0 == 36_500));
    assert!(rows.iter().any(|row| row.page > 1),
        "paragraph continuation appears on another page");
}

#[test]
fn selected_markdown_first_line_indent_replays_native_outputs() {
    let base = args(&[
        "--printed-sheet", "true",
        "--glyph-stroke", "a", "900", "900", "2200", "3400",
        "250", "#204060",
        "--flow-markdown", "a a a a a a\n\n# a\n\n- a\n\n> a",
        "--flow-region", "30000", "40000", "30000", "90000",
        "--flow-body-inset-um", "4000",
        "--flow-body-right-inset-um", "6000",
        "--flow-body-align", "center",
    ]);
    let render = |indent: Option<&str>, format: &str| {
        let mut command = base.clone();
        if let Some(indent) = indent {
            command.extend(args(&[
                "--flow-body-first-line-indent-um", indent,
            ]));
        }
        if format == "ink" {
            command.push("--emit-ink-file".into());
        } else {
            command.extend(args(&["--export-paper", format]));
        }
        if format == "png" {
            command.extend(args(&["--dpi", "72"]));
        }
        execute_notebook(&command).expect("native first-row export")
            .expect("PDF, PNG or TSV data")
    };
    for format in ["pdf", "png", "ink"] {
        let default = render(None, format);
        assert_eq!(default, render(Some("0"), format),
            "zero first-line indent retains {format} bytes");
        let indented = render(Some("8000"), format);
        assert_eq!(indented, render(Some("8000"), format),
            "first-line indent replays deterministic {format}");
        assert_ne!(indented, default, "first-line indent changes {format}");
        if format == "pdf" {
            assert!(indented.starts_with(b"%PDF-1.4"));
        } else if format == "png" {
            assert!(indented.starts_with(b"\x89PNG\r\n\x1a\n"));
        } else {
            assert!(indented.starts_with(b"atrament.page-ink.v1\n"));
        }
    }
}

#[test]
fn invalid_markdown_first_line_indent_preflights_and_preserves_pdf() {
    let source = args(&[
        "--flow-markdown-file", "-", "--glyph-file", "unopened.tsv",
        "--export-paper", "pdf",
    ]);
    for (tokens, expected) in [
        (vec!["--flow-body-first-line-indent-um", "-1"],
            "flow body first-line indent must be within 0..12000 um"),
        (vec!["--flow-body-first-line-indent-um", "12001"],
            "flow body first-line indent must be within 0..12000 um"),
        (vec!["--flow-body-first-line-indent-um"],
            "--flow-body-first-line-indent-um requires 0..12000 um"),
        (vec!["--flow-body-first-line-indent-um", "4000",
            "--flow-body-first-line-indent-um", "5000"],
            "--flow-body-first-line-indent-um may appear only once"),
    ] {
        let mut command = source.clone();
        command.extend(args(&tokens));
        assert_eq!(preflight_notebook_sources(&command)
            .expect_err("preflight must not read stdin or glyph files"),
            expected);
    }
    let plain = args(&[
        "--flow-text", "a", "--export-paper", "pdf",
        "--flow-body-first-line-indent-um", "0",
    ]);
    assert_eq!(preflight_notebook_sources(&plain)
        .expect_err("Markdown-only first line indentation"),
        "--flow-body-first-line-indent-um requires a Markdown flow");
    let plain_source = FlowSource::new(FlowFormat::Plain, "a")
        .expect("plain source");
    assert_eq!(layout_flow_with_typography(
        &plain_source, &NotebookProfile::default(), None,
        HeadingDecoration::default(), None,
        FlowTypography {
            body_first_line_indent: Some(0),
            ..FlowTypography::default()
        },
    ).expect_err("typed Markdown-only setting"),
        "flow body first-line indent requires Markdown flow");
    let source = FlowSource::new(FlowFormat::Markdown, "a")
        .expect("Markdown source");
    assert_eq!(layout_flow_with_typography(
        &source, &NotebookProfile::default(), None,
        HeadingDecoration::default(), None,
        FlowTypography {
            body_first_line_indent: Some(12_001),
            ..FlowTypography::default()
        },
    ).expect_err("typed first line bound"),
        "flow body first-line indent must be within 0..12000 um");

    let target = test_path().with_extension("pdf");
    fs::write(&target, b"keep-existing-first-line-PDF")
        .expect("test sentinel");
    let mut command = args(&[
        "--printed-sheet", "true", "--export-paper", "pdf",
        "--glyph-stroke", "a", "900", "900", "2200", "3400",
        "250", "#204060", "--flow-markdown", "a a",
        "--flow-region", "30000", "40000", "27000", "32000",
        "--flow-body-inset-um", "8000",
        "--flow-body-right-inset-um", "8000",
        "--flow-body-first-line-indent-um", "8000",
    ]);
    command.extend(args(&[
        "--output", target.to_str().expect("test path"), "--replace",
    ]));
    let err = execute_notebook(&command)
        .expect_err("narrow first-row area must refuse before writing");
    assert!(err.contains("flow body first-line indent leaves no writing row"),
        "{err}");
    assert_eq!(fs::read(&target).expect("PDF preserved"),
        b"keep-existing-first-line-PDF");
    fs::remove_file(target).expect("clean test-owned sentinel");
}

#[test]
fn markdown_first_line_indent_restarts_after_explicit_hard_breaks() {
    let source = FlowSource::new(FlowFormat::Markdown,
        "a a  \na a\n\na a",
    ).expect("hard break and blank separated prose");
    let region = FlowRegion {
        left: 30_000, top: 40_000, width: 30_000, height: 180_000,
    };
    let (rows, marks, pages) = layout_flow_with_typography(
        &source, &NotebookProfile::default(), None,
        HeadingDecoration::default(), Some(region),
        FlowTypography {
            body_first_line_indent: Some(8_000),
            body_inset: Some(4_000),
            body_right_inset: Some(6_000),
            ..FlowTypography::default()
        },
    ).expect("new composed block after hard row break");
    assert_eq!(pages, 1);
    assert!(marks.is_empty());
    assert_eq!(rows.iter().map(|row| row.text.as_str())
        .collect::<Vec<_>>(), ["a", "a", "a", "a", "a", "a"]);
    assert_eq!(rows.iter().map(|row| row.origin_um.0)
        .collect::<Vec<_>>(),
        [44_500, 36_500, 44_500, 36_500, 44_500, 36_500]);
}

#[test]
fn markdown_first_line_indent_alignment_respects_compressed_ink() {
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
            .expect("compressed nominal advance");
    }
    let source = FlowSource::new(FlowFormat::Markdown, "a")
        .expect("single authored contour");
    let region = FlowRegion {
        left: 30_000, top: 40_000, width: 50_000, height: 40_000,
    };
    for (align, origin) in [
        (BodyAlign::Left, 44_500),
        (BodyAlign::Center, 52_500),
        (BodyAlign::Right, 60_500),
    ] {
        let (rows, marks, pages) = layout_flow_with_typography(
            &source, &NotebookProfile::default(), Some(&stats),
            HeadingDecoration::default(), Some(region),
            FlowTypography {
                body_align: Some(align),
                body_first_line_indent: Some(8_000),
                body_inset: Some(4_000),
                body_right_inset: Some(6_000),
                ..FlowTypography::default()
            },
        ).expect("measured ink fits first-row region");
        assert_eq!(pages, 1);
        assert!(marks.is_empty());
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].origin_um.0, origin);
        assert!(rows[0].origin_um.0 + 12_000 <= 72_500,
            "nominal 12 mm contour stays before reserved right margin");
    }
}

#[test]
fn markdown_continuation_indent_shifts_only_wrapped_prose() {
    let source = FlowSource::new(FlowFormat::Markdown,
        "a a a a a a\n\n# a\n\n- a\n\n> a\n\n    a",
    ).expect("prose, heading, list, quote and literal source");
    let region = FlowRegion {
        left: 30_000, top: 40_000, width: 30_000, height: 180_000,
    };
    let render = |indent| layout_flow_with_typography(
        &source, &NotebookProfile::default(), None,
        HeadingDecoration::default(), Some(region),
        FlowTypography {
            body_continuation_indent: indent,
            body_inset: Some(4_000),
            body_right_inset: Some(6_000),
            ..FlowTypography::default()
        },
    ).expect("bounded Markdown prose and structure");
    let baseline = render(None);
    let zero = render(Some(0));
    assert_eq!(baseline.0.iter().map(|row| (
        row.page, row.origin_um, row.text.as_str(), row.heading_ink,
    )).collect::<Vec<_>>(), zero.0.iter().map(|row| (
        row.page, row.origin_um, row.text.as_str(), row.heading_ink,
    )).collect::<Vec<_>>());
    assert_eq!(baseline.1, zero.1);
    assert_eq!(baseline.2, zero.2);
    let moved = render(Some(8_000));
    assert_eq!(moved.2, 1);
    assert_eq!(moved.0.iter().map(|row| row.text.as_str())
        .collect::<Vec<_>>(),
        ["a a", "a", "a", "a", "a", "a", "a", "a", "    a"]);
    assert_eq!(moved.0.iter().map(|row| row.origin_um.0)
        .collect::<Vec<_>>(),
        [36_500, 44_500, 44_500, 44_500, 44_500,
            32_500, 38_500, 38_500, 32_500]);
    assert_eq!(moved.1.iter().map(|mark| (
        mark.start_um.0, mark.end_um.0, mark.width_um, mark.rgb,
    )).collect::<Vec<_>>(),
        baseline.1.iter().map(|mark| (
            mark.start_um.0, mark.end_um.0, mark.width_um, mark.rgb,
        )).collect::<Vec<_>>());
    assert_eq!(moved.0.iter().rev().take(4)
        .map(|row| (row.text.as_str(), row.origin_um.0))
        .collect::<Vec<_>>(),
        baseline.0.iter().rev().take(4)
            .map(|row| (row.text.as_str(), row.origin_um.0))
            .collect::<Vec<_>>(),
        "heading, list, quote and literal ink anchors do not move");
}

#[test]
fn markdown_continuation_indent_composes_with_first_row_and_alignment() {
    let source = FlowSource::new(FlowFormat::Markdown, "a a a a")
        .expect("wrapped prose");
    let region = FlowRegion {
        left: 30_000, top: 40_000, width: 34_000, height: 50_000,
    };
    for (align, first, continuation) in [
        (BodyAlign::Left, 40_500, 44_500),
        (BodyAlign::Center, 42_500, 48_500),
        (BodyAlign::Right, 44_500, 52_500),
    ] {
        let (rows, marks, pages) = layout_flow_with_typography(
            &source, &NotebookProfile::default(), None,
            HeadingDecoration::default(), Some(region),
            FlowTypography {
                body_align: Some(align),
                body_first_line_indent: Some(4_000),
                body_continuation_indent: Some(8_000),
                body_inset: Some(4_000),
                body_right_inset: Some(6_000),
                ..FlowTypography::default()
            },
        ).expect("aligned first and continuation rows");
        assert_eq!(pages, 1);
        assert!(marks.is_empty());
        assert_eq!(rows.iter().map(|row| row.text.as_str())
            .collect::<Vec<_>>(), ["a a", "a", "a"]);
        assert_eq!(rows.iter().map(|row| row.origin_um.0)
            .collect::<Vec<_>>(), [first, continuation, continuation]);
    }
}

#[test]
fn markdown_continuation_indent_survives_page_turns_and_hard_breaks() {
    let region = FlowRegion {
        left: 30_000, top: 40_000, width: 30_000, height: 29_000,
    };
    let source = FlowSource::new(FlowFormat::Markdown,
        "a a a a a a a a a a a a",
    ).expect("long paragraph");
    let typography = FlowTypography {
        body_first_line_indent: Some(4_000),
        body_continuation_indent: Some(8_000),
        body_inset: Some(4_000),
        body_right_inset: Some(6_000),
        ..FlowTypography::default()
    };
    let (rows, _, pages) = layout_flow_with_typography(
        &source, &NotebookProfile::default(), None,
        HeadingDecoration::default(), Some(region), typography,
    ).expect("multi-page hanging paragraph");
    assert!(pages >= 5);
    assert_eq!(rows.first().expect("first row").origin_um.0, 40_500);
    assert!(rows.iter().skip(1).all(|row| row.origin_um.0 == 44_500));
    assert!(rows.iter().any(|row| row.page > 1));
    let hard_breaks = FlowSource::new(FlowFormat::Markdown,
        "a a a  \na a a\n\na a a",
    ).expect("independent hard break and paragraph");
    let (rows, _, _) = layout_flow_with_typography(
        &hard_breaks, &NotebookProfile::default(), None,
        HeadingDecoration::default(), Some(FlowRegion {
            width: 34_000, height: 110_000, ..region
        }), typography,
    ).expect("restarted first row after each hard boundary");
    assert_eq!(rows.iter().map(|row| row.text.as_str())
        .collect::<Vec<_>>(), ["a a", "a", "a a", "a", "a a", "a"]);
    assert_eq!(rows.iter().map(|row| row.origin_um.0)
        .collect::<Vec<_>>(),
        [40_500, 44_500, 40_500, 44_500, 40_500, 44_500]);
}

#[test]
fn selected_markdown_continuation_indent_replays_native_outputs() {
    let base = args(&[
        "--printed-sheet", "true",
        "--glyph-stroke", "a", "900", "900", "2200", "3400",
        "250", "#204060",
        "--flow-markdown", "a a a a a a\n\n# a\n\n- a\n\n> a",
        "--flow-region", "30000", "40000", "30000", "90000",
        "--flow-body-inset-um", "4000",
        "--flow-body-right-inset-um", "6000",
        "--flow-body-align", "center",
    ]);
    let render = |indent: Option<&str>, format: &str| {
        let mut command = base.clone();
        if let Some(indent) = indent {
            command.extend(args(&[
                "--flow-body-continuation-indent-um", indent,
            ]));
        }
        if format == "ink" {
            command.push("--emit-ink-file".into());
        } else {
            command.extend(args(&["--export-paper", format]));
        }
        if format == "png" {
            command.extend(args(&["--dpi", "72"]));
        }
        execute_notebook(&command).expect("native hanging export")
            .expect("PDF, PNG or TSV data")
    };
    for format in ["pdf", "png", "ink"] {
        let default = render(None, format);
        assert_eq!(default, render(Some("0"), format),
            "zero continuation indent preserves {format} bytes");
        let selected = render(Some("8000"), format);
        assert_eq!(selected, render(Some("8000"), format),
            "continuation indent deterministically replays {format}");
        assert_ne!(selected, default,
            "selected continuation indent changes {format}");
        if format == "pdf" {
            assert!(selected.starts_with(b"%PDF-1.4"));
        } else if format == "png" {
            assert!(selected.starts_with(b"\x89PNG\r\n\x1a\n"));
        } else {
            assert!(selected.starts_with(b"atrament.page-ink.v1\n"));
        }
    }
}

#[test]
fn invalid_markdown_continuation_indent_preflights_and_preserves_pdf() {
    let source = args(&[
        "--flow-markdown-file", "-", "--glyph-file", "unopened.tsv",
        "--export-paper", "pdf",
    ]);
    for (tokens, expected) in [
        (vec!["--flow-body-continuation-indent-um", "-1"],
            "flow body continuation indent must be within 0..12000 um"),
        (vec!["--flow-body-continuation-indent-um", "12001"],
            "flow body continuation indent must be within 0..12000 um"),
        (vec!["--flow-body-continuation-indent-um"],
            "--flow-body-continuation-indent-um requires 0..12000 um"),
        (vec!["--flow-body-continuation-indent-um", "4000",
            "--flow-body-continuation-indent-um", "5000"],
            "--flow-body-continuation-indent-um may appear only once"),
    ] {
        let mut command = source.clone();
        command.extend(args(&tokens));
        assert_eq!(preflight_notebook_sources(&command)
            .expect_err("invalid option must refuse before source I/O"),
            expected);
    }
    assert_eq!(preflight_notebook_sources(&args(&[
        "--flow-text", "a", "--export-paper", "pdf",
        "--flow-body-continuation-indent-um", "0",
    ])).expect_err("Markdown-only option"),
        "--flow-body-continuation-indent-um requires a Markdown flow");
    let plain = FlowSource::new(FlowFormat::Plain, "a")
        .expect("plain source");
    assert_eq!(layout_flow_with_typography(
        &plain, &NotebookProfile::default(), None,
        HeadingDecoration::default(), None,
        FlowTypography {
            body_continuation_indent: Some(0),
            ..FlowTypography::default()
        },
    ).expect_err("typed Markdown-only option"),
        "flow body continuation indent requires Markdown flow");
    let source = FlowSource::new(FlowFormat::Markdown, "a")
        .expect("Markdown source");
    assert_eq!(layout_flow_with_typography(
        &source, &NotebookProfile::default(), None,
        HeadingDecoration::default(), None,
        FlowTypography {
            body_continuation_indent: Some(12_001),
            ..FlowTypography::default()
        },
    ).expect_err("typed inset bound"),
        "flow body continuation indent must be within 0..12000 um");
    let target = test_path().with_extension("pdf");
    fs::write(&target, b"preserve-existing-continuation-PDF")
        .expect("test sentinel");
    let mut command = args(&[
        "--printed-sheet", "true", "--export-paper", "pdf",
        "--glyph-stroke", "a", "900", "900", "2200", "3400",
        "250", "#204060", "--flow-markdown", "a a",
        "--flow-region", "30000", "40000", "27000", "32000",
        "--flow-body-inset-um", "8000",
        "--flow-body-right-inset-um", "8000",
        "--flow-body-continuation-indent-um", "8000",
    ]);
    command.extend(args(&[
        "--output", target.to_str().expect("test path"), "--replace",
    ]));
    let err = execute_notebook(&command)
        .expect_err("narrow continuation row refuses before file commit");
    assert!(err.contains("flow body continuation indent leaves no writing row"),
        "{err}");
    assert_eq!(fs::read(&target).expect("original PDF unchanged"),
        b"preserve-existing-continuation-PDF");
    fs::remove_file(target).expect("clean test-owned sentinel");
}

#[test]
fn markdown_continuation_indent_can_exceed_first_row_free_width() {
    let source = FlowSource::new(FlowFormat::Markdown, "a a a a")
        .expect("prose whose first row is narrower than the hang");
    let region = FlowRegion {
        left: 30_000, top: 40_000, width: 34_000, height: 90_000,
    };
    let (rows, marks, pages) = layout_flow_with_typography(
        &source, &NotebookProfile::default(), None,
        HeadingDecoration::default(), Some(region),
        FlowTypography {
            body_first_line_indent: Some(12_000),
            body_continuation_indent: Some(12_000),
            body_inset: Some(4_000),
            body_right_inset: Some(6_000),
            ..FlowTypography::default()
        },
    ).expect("both rows retain seven millimetres of writing width");
    assert_eq!(pages, 1);
    assert!(marks.is_empty());
    assert_eq!(rows.iter().map(|row| row.text.as_str())
        .collect::<Vec<_>>(), ["a", "a", "a", "a"]);
    assert!(rows.iter().all(|row| row.origin_um.0 == 48_500));
}

#[test]
fn selected_markdown_quote_alignment_preserves_rule_gutter_and_prose() {
    let source = FlowSource::new(FlowFormat::Markdown,
        "# a\n\na a\n\n> a a  \n> a\n\n- a\n\n    a",
    ).expect("mixed Markdown rows");
    let region = FlowRegion {
        left: 30_000, top: 40_000, width: 70_000, height: 200_000,
    };
    let base = FlowTypography {
        list_quote_inset: Some(4_000),
        list_quote_right_inset: Some(10_000),
        body_align: Some(BodyAlign::Right),
        ..FlowTypography::default()
    };
    let (left, left_marks, _) = layout_flow_with_typography(
        &source, &NotebookProfile::default(), None,
        HeadingDecoration::default(), Some(region), base,
    ).expect("left-aligned quote baseline");
    let quotes = |align| layout_flow_with_typography(
        &source, &NotebookProfile::default(), None,
        HeadingDecoration::default(), Some(region), FlowTypography {
            quote_align: Some(align), ..base
        },
    ).expect("chosen quote alignment");
    let (explicit_left, left_rules, _) = quotes(BodyAlign::Left);
    assert_eq!(left.iter().map(|row| (&row.text, row.origin_um))
        .collect::<Vec<_>>(), explicit_left.iter()
        .map(|row| (&row.text, row.origin_um)).collect::<Vec<_>>());
    assert_eq!(left_marks, left_rules);
    let (center, center_marks, _) = quotes(BodyAlign::Center);
    let (right, right_marks, _) = quotes(BodyAlign::Right);
    assert_eq!(left_marks, center_marks);
    assert_eq!(left_marks, right_marks);
    assert_eq!(left.iter().map(|row| row.text.as_str())
        .collect::<Vec<_>>(),
        ["a", "a a", "a a", "a", "a", "    a"]);
    assert_eq!(left.iter().map(|row| row.origin_um.0)
        .collect::<Vec<_>>(),
        [32_500, 86_500, 42_500, 42_500, 42_500, 32_500]);
    assert_eq!(center.iter().map(|row| row.origin_um.0)
        .collect::<Vec<_>>(),
        [32_500, 86_500, 59_500, 63_500, 42_500, 32_500]);
    assert_eq!(right.iter().map(|row| row.origin_um.0)
        .collect::<Vec<_>>(),
        [32_500, 86_500, 76_500, 84_500, 42_500, 32_500]);
    let bars = center_marks.iter().filter(|mark| mark.width_um == 400)
        .collect::<Vec<_>>();
    assert_eq!(bars.len(), 2);
    assert!(bars.iter().all(|mark| mark.start_um.0 == 39_500
        && mark.end_um.0 == 39_500));
}

#[test]
fn selected_markdown_quote_alignment_wraps_and_paginates_same_ink() {
    let source = FlowSource::new(FlowFormat::Markdown,
        "> a a a a a a a a a a a a a a a a",
    ).expect("long quoted paragraph");
    let region = FlowRegion {
        left: 30_000, top: 40_000, width: 34_000, height: 25_000,
    };
    let (left, original_bars, pages) = layout_flow_with_typography(
        &source, &NotebookProfile::default(), None,
        HeadingDecoration::default(), Some(region), FlowTypography {
            list_quote_right_inset: Some(6_000),
            ..FlowTypography::default()
        },
    ).expect("quoted wrap across pages");
    let (right, selected_bars, aligned_pages) = layout_flow_with_typography(
        &source, &NotebookProfile::default(), None,
        HeadingDecoration::default(), Some(region), FlowTypography {
            list_quote_right_inset: Some(6_000),
            quote_align: Some(BodyAlign::Right),
            ..FlowTypography::default()
        },
    ).expect("aligned quoted wrap across pages");
    assert!(pages >= 3);
    assert_eq!(pages, aligned_pages);
    assert_eq!(original_bars, selected_bars);
    assert!(left.iter().zip(right.iter()).all(|(ordinary, aligned)| {
        ordinary.page == aligned.page
            && ordinary.text == aligned.text
            && ordinary.origin_um.1 == aligned.origin_um.1
            && ordinary.origin_um.0 <= aligned.origin_um.0
    }));
    assert!(left.iter().zip(right.iter()).any(|(ordinary, aligned)|
        ordinary.origin_um.0 < aligned.origin_um.0));
    assert!(right.iter().any(|row| row.page > 1));
    assert_eq!(selected_bars.len(), right.len());
    assert!(selected_bars.iter().all(|mark| mark.start_um.0 == 35_500));
}

#[test]
fn markdown_quote_alignment_exports_deterministic_native_formats() {
    let source = args(&[
        "--printed-sheet", "true",
        "--glyph-stroke", "a", "900", "900", "2200", "3400",
        "250", "#204060",
        "--flow-markdown", "> a a a a a a a a a a\n\n- a\n\na",
        "--flow-region", "30000", "40000", "30000", "26000",
        "--flow-list-quote-inset-um", "4000",
        "--flow-list-quote-right-inset-um", "6000",
        "--flow-body-align", "center",
    ]);
    let render = |alignment: Option<&str>, format: &str| {
        let mut command = source.clone();
        if let Some(align) = alignment {
            command.extend(args(&["--flow-quote-align", align]));
        }
        if format == "ink" {
            command.push("--emit-ink-file".into());
        } else {
            command.extend(args(&["--export-paper", format]));
        }
        if format == "png" {
            command.extend(args(&["--dpi", "72", "--png-page", "2"]));
        }
        execute_notebook(&command).expect("native quote export")
            .expect("PDF, PNG or ink TSV output")
    };
    for format in ["pdf", "png", "ink"] {
        let baseline = render(None, format);
        assert_eq!(baseline, render(Some("left"), format),
            "explicit quote-left retains {format} bytes");
        for alignment in ["center", "right"] {
            let selected = render(Some(alignment), format);
            assert_eq!(selected, render(Some(alignment), format),
                "deterministic {alignment} quote {format}");
            assert_ne!(selected, baseline,
                "selected {alignment} quote changes {format}");
            match format {
                "pdf" => assert!(selected.starts_with(b"%PDF-1.4")),
                "png" => assert!(selected.starts_with(b"\x89PNG\r\n\x1a\n")),
                _ => assert!(selected.starts_with(b"atrament.page-ink.v1\n")),
            }
        }
    }
}

#[test]
fn invalid_markdown_quote_alignment_preflights_and_preserves_pdf() {
    let source = args(&[
        "--flow-markdown-file", "-", "--glyph-file", "unopened.tsv",
        "--export-paper", "pdf",
    ]);
    for (tokens, expected) in [
        (vec!["--flow-quote-align", "justify"],
            "--flow-quote-align requires left, center or right"),
        (vec!["--flow-quote-align"],
            "--flow-quote-align requires left, center or right"),
        (vec!["--flow-quote-align", "left",
            "--flow-quote-align", "right"],
            "--flow-quote-align may appear only once"),
    ] {
        let mut command = source.clone();
        command.extend(args(&tokens));
        assert_eq!(preflight_notebook_sources(&command)
            .expect_err("invalid option before selected input"), expected);
    }
    assert_eq!(preflight_notebook_sources(&args(&[
        "--flow-text", "a", "--export-paper", "pdf",
        "--flow-quote-align", "center",
    ])).expect_err("Markdown-only quote alignment"),
        "--flow-quote-align requires a Markdown flow");
    let plain = FlowSource::new(FlowFormat::Plain, "a")
        .expect("plain source");
    assert_eq!(layout_flow_with_typography(
        &plain, &NotebookProfile::default(), None,
        HeadingDecoration::default(), None, FlowTypography {
            quote_align: Some(BodyAlign::Center),
            ..FlowTypography::default()
        },
    ).expect_err("typed Markdown-only option"),
        "flow quote alignment requires Markdown flow");
    let target = test_path().with_extension("pdf");
    fs::write(&target, b"preserve-existing-quote-PDF")
        .expect("test sentinel");
    let mut command = args(&[
        "--printed-sheet", "true", "--export-paper", "pdf",
        "--glyph-stroke", "a", "900", "900", "2200", "3400",
        "250", "#204060", "--flow-markdown", "> a a",
        "--flow-region", "30000", "40000", "21000", "26000",
        "--flow-list-quote-right-inset-um", "12000",
        "--flow-quote-align", "right",
    ]);
    command.extend(args(&[
        "--output", target.to_str().expect("test path"), "--replace",
    ]));
    let err = execute_notebook(&command)
        .expect_err("unfit quote row refuses before PDF replacement");
    assert!(err.contains("flow list/quote right inset leaves no writing row"),
        "{err}");
    assert_eq!(fs::read(&target).expect("original PDF"),
        b"preserve-existing-quote-PDF");
    fs::remove_file(target).expect("clean test-owned sentinel");
}

#[test]
fn selected_markdown_bullet_alignment_preserves_marker_gutter_and_other_rows() {
    let source = FlowSource::new(FlowFormat::Markdown,
        "# a\n\na a\n\n- a a\n\n- [x] a\n\n1. a\n\n> a\n\n    a",
    ).expect("mixed Markdown rows");
    let region = FlowRegion {
        left: 30_000, top: 40_000, width: 70_000, height: 200_000,
    };
    let base = FlowTypography {
        list_quote_inset: Some(4_000),
        list_quote_right_inset: Some(10_000),
        body_align: Some(BodyAlign::Right),
        quote_align: Some(BodyAlign::Center),
        ..FlowTypography::default()
    };
    let choose = |align| layout_flow_with_typography(
        &source, &NotebookProfile::default(), None,
        HeadingDecoration::default(), Some(region), FlowTypography {
            bullet_align: align, ..base
        },
    ).expect("selected bullet alignment");
    let (default, marks, pages) = choose(None);
    let (left, left_marks, left_pages) = choose(Some(BodyAlign::Left));
    assert_eq!(pages, left_pages);
    assert_eq!(marks, left_marks);
    assert_eq!(default.iter().map(|row| (&row.text, row.origin_um))
        .collect::<Vec<_>>(), left.iter()
        .map(|row| (&row.text, row.origin_um)).collect::<Vec<_>>());
    let (center, center_marks, _) = choose(Some(BodyAlign::Center));
    let (right, right_marks, _) = choose(Some(BodyAlign::Right));
    assert_eq!(marks, center_marks);
    assert_eq!(marks, right_marks);
    assert_eq!(default.iter().map(|row| row.text.as_str())
        .collect::<Vec<_>>(),
        ["a", "a a", "a a", "a", "1. a", "a", "    a"]);
    for (index, (normal, (middle, end))) in default.iter()
        .zip(center.iter().zip(right.iter())).enumerate() {
        assert_eq!((normal.page, normal.origin_um.1),
            (middle.page, middle.origin_um.1));
        assert_eq!((normal.page, normal.origin_um.1),
            (end.page, end.origin_um.1));
        let (shift_center, shift_right) = match index {
            2 => (17_000, 34_000),
            3 => (21_000, 42_000),
            _ => (0, 0),
        };
        assert_eq!(middle.origin_um.0,
            normal.origin_um.0 + shift_center);
        assert_eq!(end.origin_um.0,
            normal.origin_um.0 + shift_right);
    }
    // Neither the bullet nor the checked box moves with its selected text.
    assert_eq!(marks.iter().filter(|mark| mark.width_um == 450
        && mark.start_um.1 == mark.end_um.1).count(), 1);
    assert!(marks.iter().any(|mark| mark.width_um == 350));
}

#[test]
fn selected_markdown_bullet_alignment_wraps_without_moving_markers() {
    let source = FlowSource::new(FlowFormat::Markdown,
        "- a a a a a a a a a a a a a a a a\n\n- [x] a a a a a a a a",
    ).expect("long bullet and checkbox paragraphs");
    let region = FlowRegion {
        left: 30_000, top: 40_000, width: 34_000, height: 25_000,
    };
    let base = FlowTypography {
        list_quote_right_inset: Some(6_000),
        ..FlowTypography::default()
    };
    let (left, marks, pages) = layout_flow_with_typography(
        &source, &NotebookProfile::default(), None,
        HeadingDecoration::default(), Some(region), base,
    ).expect("wrapped list baseline");
    let (right, selected_marks, selected_pages) = layout_flow_with_typography(
        &source, &NotebookProfile::default(), None,
        HeadingDecoration::default(), Some(region), FlowTypography {
            bullet_align: Some(BodyAlign::Right), ..base
        },
    ).expect("wrapped aligned list");
    assert!(pages >= 3);
    assert_eq!(pages, selected_pages);
    assert_eq!(marks, selected_marks);
    assert!(left.iter().zip(right.iter()).all(|(ordinary, aligned)| {
        ordinary.page == aligned.page
            && ordinary.text == aligned.text
            && ordinary.origin_um.1 == aligned.origin_um.1
            && ordinary.origin_um.0 <= aligned.origin_um.0
    }));
    assert!(left.iter().zip(right.iter()).any(|(ordinary, aligned)|
        ordinary.origin_um.0 < aligned.origin_um.0));
    assert!(right.iter().any(|row| row.page > 1));
    assert_eq!(selected_marks.iter().filter(|mark| mark.width_um == 350)
        .count(), 4, "checkbox outline retains all edges");
}

#[test]
fn markdown_bullet_alignment_exports_deterministic_native_formats() {
    let source = args(&[
        "--printed-sheet", "true",
        "--glyph-stroke", "a", "900", "900", "2200", "3400",
        "250", "#204060",
        "--flow-markdown", "- a a a a a a a a a a\n\n- [x] a\n\n> a\n\na",
        "--flow-region", "30000", "40000", "30000", "26000",
        "--flow-list-quote-inset-um", "4000",
        "--flow-list-quote-right-inset-um", "6000",
        "--flow-body-align", "center",
        "--flow-quote-align", "right",
    ]);
    let render = |alignment: Option<&str>, format: &str| {
        let mut command = source.clone();
        if let Some(align) = alignment {
            command.extend(args(&["--flow-bullet-align", align]));
        }
        if format == "ink" {
            command.push("--emit-ink-file".into());
        } else {
            command.extend(args(&["--export-paper", format]));
        }
        if format == "png" {
            command.extend(args(&["--dpi", "72", "--png-page", "2"]));
        }
        execute_notebook(&command).expect("native bullet export")
            .expect("PDF, PNG or ink TSV output")
    };
    for format in ["pdf", "png", "ink"] {
        let baseline = render(None, format);
        assert_eq!(baseline, render(Some("left"), format),
            "explicit bullet-left retains {format} bytes");
        for alignment in ["center", "right"] {
            let selected = render(Some(alignment), format);
            assert_eq!(selected, render(Some(alignment), format),
                "deterministic {alignment} bullet {format}");
            assert_ne!(selected, baseline,
                "selected {alignment} bullet changes {format}");
            match format {
                "pdf" => assert!(selected.starts_with(b"%PDF-1.4")),
                "png" => assert!(selected.starts_with(b"\x89PNG\r\n\x1a\n")),
                _ => assert!(selected.starts_with(b"atrament.page-ink.v1\n")),
            }
        }
    }
}

#[test]
fn invalid_markdown_bullet_alignment_preflights_and_preserves_pdf() {
    let source = args(&[
        "--flow-markdown-file", "-", "--glyph-file", "unopened.tsv",
        "--export-paper", "pdf",
    ]);
    for (tokens, expected) in [
        (vec!["--flow-bullet-align", "justify"],
            "--flow-bullet-align requires left, center or right"),
        (vec!["--flow-bullet-align"],
            "--flow-bullet-align requires left, center or right"),
        (vec!["--flow-bullet-align", "left",
            "--flow-bullet-align", "right"],
            "--flow-bullet-align may appear only once"),
    ] {
        let mut command = source.clone();
        command.extend(args(&tokens));
        assert_eq!(preflight_notebook_sources(&command)
            .expect_err("invalid option before selected input"), expected);
    }
    assert_eq!(preflight_notebook_sources(&args(&[
        "--flow-text", "a", "--export-paper", "pdf",
        "--flow-bullet-align", "center",
    ])).expect_err("Markdown-only bullet alignment"),
        "--flow-bullet-align requires a Markdown flow");
    let plain = FlowSource::new(FlowFormat::Plain, "a")
        .expect("plain source");
    assert_eq!(layout_flow_with_typography(
        &plain, &NotebookProfile::default(), None,
        HeadingDecoration::default(), None, FlowTypography {
            bullet_align: Some(BodyAlign::Center),
            ..FlowTypography::default()
        },
    ).expect_err("typed Markdown-only option"),
        "flow bullet alignment requires Markdown flow");
    let target = test_path().with_extension("pdf");
    fs::write(&target, b"preserve-existing-bullet-PDF")
        .expect("test sentinel");
    let mut command = args(&[
        "--printed-sheet", "true", "--export-paper", "pdf",
        "--glyph-stroke", "a", "900", "900", "2200", "3400",
        "250", "#204060", "--flow-markdown", "- a a",
        "--flow-region", "30000", "40000", "21000", "26000",
        "--flow-list-quote-right-inset-um", "12000",
        "--flow-bullet-align", "right",
    ]);
    command.extend(args(&[
        "--output", target.to_str().expect("test path"), "--replace",
    ]));
    let err = execute_notebook(&command)
        .expect_err("unfit bullet row refuses before PDF replacement");
    assert!(err.contains("flow list/quote right inset leaves no writing row"),
        "{err}");
    assert_eq!(fs::read(&target).expect("original PDF"),
        b"preserve-existing-bullet-PDF");
    fs::remove_file(target).expect("clean test-owned sentinel");
}

#[test]
fn markdown_task_alignment_overrides_bullet_alignment_without_moving_boxes() {
    let source = FlowSource::new(FlowFormat::Markdown,
        "# a\n\na a\n\n- a a\n\n- [x] a\n\n- [ ] a a\n\n1. a\n\n> a\n\n    a",
    ).expect("mixed Markdown tasks");
    let region = FlowRegion {
        left: 30_000, top: 40_000, width: 70_000, height: 200_000,
    };
    let base = FlowTypography {
        bullet_align: Some(BodyAlign::Right),
        body_align: Some(BodyAlign::Center),
        list_quote_inset: Some(4_000),
        list_quote_right_inset: Some(10_000),
        ..FlowTypography::default()
    };
    let choose = |selection| layout_flow_with_typography(
        &source, &NotebookProfile::default(), None,
        HeadingDecoration::default(), Some(region), FlowTypography {
            task_align: selection, ..base
        },
    ).expect("task alignment");
    let (inherited, marks, pages) = choose(None);
    let (explicit_right, right_marks, right_pages) =
        choose(Some(BodyAlign::Right));
    assert_eq!(pages, right_pages);
    assert_eq!(inherited.iter().map(|r| (&r.text, r.origin_um))
        .collect::<Vec<_>>(), explicit_right.iter()
        .map(|r| (&r.text, r.origin_um)).collect::<Vec<_>>());
    assert_eq!(marks, right_marks);
    let (left, left_marks, _) = choose(Some(BodyAlign::Left));
    let (center, center_marks, _) = choose(Some(BodyAlign::Center));
    assert_eq!(marks, left_marks);
    assert_eq!(marks, center_marks);
    assert_eq!(inherited.iter().map(|r| r.text.as_str()).collect::<Vec<_>>(),
        ["a", "a a", "a a", "a", "a a", "1. a", "a", "    a"]);
    for (index, (original, (unshifted, middle))) in inherited.iter()
        .zip(left.iter().zip(center.iter())).enumerate() {
        assert_eq!((original.page, original.origin_um.1),
            (unshifted.page, unshifted.origin_um.1));
        assert_eq!((original.page, original.origin_um.1),
            (middle.page, middle.origin_um.1));
        if index == 3 || index == 4 {
            let available = if index == 3 { 42_000 } else { 34_000 };
            assert_eq!(original.origin_um.0,
                unshifted.origin_um.0 + available);
            assert_eq!(middle.origin_um.0,
                unshifted.origin_um.0 + available / 2);
        } else {
            assert_eq!(original.origin_um.0, unshifted.origin_um.0);
            assert_eq!(original.origin_um.0, middle.origin_um.0);
        }
    }
    assert_eq!(marks.iter().filter(|mark| mark.width_um == 350).count(),
        8, "both checkbox outlines remain fixed");
}

#[test]
fn markdown_task_alignment_wraps_on_physical_pages_with_fixed_boxes() {
    let source = FlowSource::new(FlowFormat::Markdown,
        "- [x] a a a a a a a a a a a a\n\n- [ ] a a a a a a a a",
    ).expect("wrapped checkbox tasks");
    let region = FlowRegion {
        left: 30_000, top: 40_000, width: 34_000, height: 25_000,
    };
    let base = FlowTypography {
        list_quote_right_inset: Some(6_000),
        ..FlowTypography::default()
    };
    let choose = |task_align| layout_flow_with_typography(
        &source, &NotebookProfile::default(), None,
        HeadingDecoration::default(), Some(region), FlowTypography {
            task_align, ..base
        },
    ).expect("multi-page tasks");
    let (left, marks, pages) = choose(None);
    let (right, shifted_marks, right_pages) =
        choose(Some(BodyAlign::Right));
    assert_eq!(pages, right_pages);
    assert!(pages >= 3);
    assert_eq!(marks, shifted_marks);
    assert_eq!(marks.iter().filter(|mark| mark.width_um == 350).count(),
        8, "checkbox outlines are attached to first physical rows");
    let right_edge = region.left + region.width - 1_500 - 6_000;
    for (normal, aligned) in left.iter().zip(right.iter()) {
        assert_eq!((&normal.text, normal.page, normal.origin_um.1),
            (&aligned.text, aligned.page, aligned.origin_um.1));
        assert!(normal.origin_um.0 <= aligned.origin_um.0);
        let extent = u32::try_from(normal.text.chars().count())
            .expect("bounded row") * 4_000;
        assert_eq!(aligned.origin_um.0 + extent, right_edge);
    }
    assert!(right.iter().any(|row| row.page > 1));
}

#[test]
fn markdown_task_alignment_measures_compressed_glyph_contours() {
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
            .expect("compressed nominal task glyph");
    }
    let source = FlowSource::new(FlowFormat::Markdown,
        "- [x] a\n\n- [ ] a",
    ).expect("authored checkboxes");
    let region = FlowRegion {
        left: 30_000, top: 40_000, width: 50_000, height: 40_000,
    };
    let mut marker_baseline = None;
    for (align, offset) in [
        (BodyAlign::Left, 0),
        (BodyAlign::Center, 12_000),
        (BodyAlign::Right, 24_000),
    ] {
        let (rows, marks, pages) = layout_flow_with_typography(
            &source, &NotebookProfile::default(), Some(&stats),
            HeadingDecoration::default(), Some(region), FlowTypography {
                task_align: Some(align),
                list_quote_right_inset: Some(4_000),
                ..FlowTypography::default()
            },
        ).expect("task ink fits region");
        assert_eq!(pages, 1);
        assert_eq!(rows.len(), 2);
        if let Some(ref baseline) = marker_baseline {
            assert_eq!(&marks, baseline);
        } else {
            marker_baseline = Some(marks.clone());
        }
        assert_eq!(marks.iter().filter(|m| m.width_um == 350).count(), 8);
        for (index, row) in rows.iter().enumerate() {
            assert_eq!(row.text, "a");
            assert_eq!(row.origin_um, (
                38_500 + offset, 42_500 + 13_000 * index as u32,
            ));
            assert!(row.origin_um.0 + 12_000 <= 74_500,
                "compressed advance must not hide the 12 mm ink contour");
        }
    }
}

#[test]
fn markdown_task_alignment_replays_native_pdf_png_and_ink() {
    let source = args(&[
        "--printed-sheet", "true",
        "--glyph-stroke", "a", "900", "900", "2200", "3400",
        "250", "#204060",
        "--flow-markdown",
        "- a a a\n\n- [x] a a a a a a a a\n\n- [ ] a a a\n\n> a",
        "--flow-region", "30000", "40000", "34000", "26000",
        "--flow-list-quote-right-inset-um", "6000",
        "--flow-bullet-align", "right",
    ]);
    let render = |selection: Option<&str>, format: &str| {
        let mut command = source.clone();
        if let Some(align) = selection {
            command.extend(args(&["--flow-task-align", align]));
        }
        if format == "ink" {
            command.push("--emit-ink-file".into());
        } else {
            command.extend(args(&["--export-paper", format]));
        }
        if format == "png" {
            command.extend(args(&["--dpi", "72", "--png-page", "2"]));
        }
        execute_notebook(&command).expect("native checkbox export")
            .expect("PDF, PNG or ink bytes")
    };
    for format in ["pdf", "png", "ink"] {
        let baseline = render(None, format);
        assert_eq!(baseline, render(Some("right"), format),
            "inherit bullet-right when task selection is omitted");
        for alignment in ["left", "center"] {
            let chosen = render(Some(alignment), format);
            assert_eq!(chosen, render(Some(alignment), format),
                "{alignment} task {format} replay");
            assert_ne!(chosen, baseline,
                "{alignment} task must override bullet for {format}");
            match format {
                "pdf" => assert!(chosen.starts_with(b"%PDF-1.4")),
                "png" => assert!(chosen.starts_with(b"\x89PNG\r\n\x1a\n")),
                _ => assert!(chosen.starts_with(b"atrament.page-ink.v1\n")),
            }
        }
    }
}

#[test]
fn markdown_task_alignment_preflights_before_io_and_preserves_pdf() {
    let source = args(&[
        "--flow-markdown-file", "-", "--glyph-file", "unopened.tsv",
        "--export-paper", "pdf",
    ]);
    for (tokens, expected) in [
        (vec!["--flow-task-align", "justify"],
            "--flow-task-align requires left, center or right"),
        (vec!["--flow-task-align"],
            "--flow-task-align requires left, center or right"),
        (vec!["--flow-task-align", "left",
            "--flow-task-align", "right"],
            "--flow-task-align may appear only once"),
    ] {
        let mut command = source.clone();
        command.extend(args(&tokens));
        assert_eq!(preflight_notebook_sources(&command)
            .expect_err("invalid option before selected input"), expected);
    }
    assert_eq!(preflight_notebook_sources(&args(&[
        "--flow-text", "a", "--export-paper", "pdf",
        "--flow-task-align", "center",
    ])).expect_err("Markdown-only task alignment"),
        "--flow-task-align requires a Markdown flow");
    let plain = FlowSource::new(FlowFormat::Plain, "a")
        .expect("plain source");
    assert_eq!(layout_flow_with_typography(
        &plain, &NotebookProfile::default(), None,
        HeadingDecoration::default(), None, FlowTypography {
            task_align: Some(BodyAlign::Center),
            ..FlowTypography::default()
        },
    ).expect_err("typed Markdown-only option"),
        "flow task alignment requires Markdown flow");
    let target = test_path().with_extension("pdf");
    fs::write(&target, b"preserve-existing-task-PDF")
        .expect("test sentinel");
    let mut command = args(&[
        "--printed-sheet", "true", "--export-paper", "pdf",
        "--glyph-stroke", "a", "900", "900", "2200", "3400",
        "250", "#204060", "--flow-markdown", "- [x] a a",
        "--flow-region", "30000", "40000", "21000", "26000",
        "--flow-list-quote-right-inset-um", "12000",
        "--flow-task-align", "right",
    ]);
    command.extend(args(&[
        "--output", target.to_str().expect("test path"), "--replace",
    ]));
    let err = execute_notebook(&command)
        .expect_err("unfit task row refuses before PDF replacement");
    assert!(err.contains("flow list/quote right inset leaves no writing row"),
        "{err}");
    assert_eq!(fs::read(&target).expect("original PDF"),
        b"preserve-existing-task-PDF");
    fs::remove_file(target).expect("clean test-owned sentinel");
}

#[test]
fn selected_markdown_ordered_alignment_preserves_other_rows_and_labels() {
    let source = FlowSource::new(FlowFormat::Markdown,
        "# a\n\na a\n\n1. a a\n\n12) a\n\n- a\n\n> a\n\n    a",
    ).expect("mixed Markdown rows");
    let region = FlowRegion {
        left: 30_000, top: 40_000, width: 70_000, height: 200_000,
    };
    let base = FlowTypography {
        list_quote_inset: Some(4_000),
        list_quote_right_inset: Some(10_000),
        body_align: Some(BodyAlign::Right),
        bullet_align: Some(BodyAlign::Center),
        quote_align: Some(BodyAlign::Right),
        ..FlowTypography::default()
    };
    let choose = |align| layout_flow_with_typography(
        &source, &NotebookProfile::default(), None,
        HeadingDecoration::default(), Some(region), FlowTypography {
            ordered_align: align, ..base
        },
    ).expect("selected ordered alignment");
    let (default, marks, pages) = choose(None);
    let (left, left_marks, left_pages) = choose(Some(BodyAlign::Left));
    assert_eq!(pages, left_pages);
    assert_eq!(marks, left_marks);
    assert_eq!(default.iter().map(|row| (&row.text, row.origin_um))
        .collect::<Vec<_>>(), left.iter()
        .map(|row| (&row.text, row.origin_um)).collect::<Vec<_>>());
    let (center, center_marks, _) = choose(Some(BodyAlign::Center));
    let (right, right_marks, _) = choose(Some(BodyAlign::Right));
    assert_eq!(marks, center_marks);
    assert_eq!(marks, right_marks);
    assert_eq!(default.iter().map(|row| row.text.as_str())
        .collect::<Vec<_>>(),
        ["a", "a a", "1. a a", "12) a", "a", "a", "    a"]);
    for (index, (normal, (middle, end))) in default.iter()
        .zip(center.iter().zip(right.iter())).enumerate() {
        assert_eq!((normal.page, normal.origin_um.1),
            (middle.page, middle.origin_um.1));
        assert_eq!((normal.page, normal.origin_um.1),
            (end.page, end.origin_um.1));
        let (shift_center, shift_right) = match index {
            2 => (11_000, 22_000),
            3 => (13_000, 26_000),
            _ => (0, 0),
        };
        assert_eq!(middle.origin_um.0,
            normal.origin_um.0 + shift_center);
        assert_eq!(end.origin_um.0,
            normal.origin_um.0 + shift_right);
    }
    assert_eq!(default[2].origin_um.0, 42_500);
    assert_eq!(default[3].origin_um.0, 42_500);
}

#[test]
fn selected_markdown_ordered_alignment_respects_hanging_continuations() {
    let source = FlowSource::new(FlowFormat::Markdown,
        "12) a a a a a a a a\n\n1. a a a a a a",
    ).expect("long numbered items");
    let region = FlowRegion {
        left: 30_000, top: 40_000, width: 36_000, height: 25_000,
    };
    let base = FlowTypography {
        list_quote_right_inset: Some(4_000),
        ..FlowTypography::default()
    };
    let choose = |align| layout_flow_with_typography(
        &source, &NotebookProfile::default(), None,
        HeadingDecoration::default(), Some(region), FlowTypography {
            ordered_align: align, ..base
        },
    ).expect("numbered flow with hanging indent");
    let (default, marks, pages) = choose(None);
    let (right, shifted_marks, right_pages) = choose(Some(BodyAlign::Right));
    assert_eq!(pages, right_pages);
    assert!(pages >= 3);
    assert_eq!(marks, shifted_marks);
    assert!(default.iter().any(|row| row.text.starts_with("12) ")));
    assert!(default.iter().any(|row| row.text.starts_with("1. ")));
    assert!(default.iter().any(|row| row.text == "a"));
    let right_edge = region.left + region.width - 1_500 - 4_000;
    for (normal, aligned) in default.iter().zip(&right) {
        assert_eq!((normal.page, &normal.text, normal.origin_um.1),
            (aligned.page, &aligned.text, aligned.origin_um.1));
        assert!(normal.origin_um.0 <= aligned.origin_um.0);
        // Default uncalibrated glyphs advance by one 4-mm cell each.
        let extent = u32::try_from(normal.text.chars().count())
            .expect("bounded row length") * 4_000;
        assert_eq!(aligned.origin_um.0 + extent, right_edge,
            "each numbered row ends at its inset writing edge");
    }
    assert!(right.iter().any(|row| row.page > 1));
}

#[test]
fn markdown_ordered_alignment_exports_deterministic_native_formats() {
    let source = args(&[
        "--printed-sheet", "true",
        "--glyph-stroke", "a", "900", "900", "2200", "3400",
        "250", "#204060",
        "--fallback-glyph", "1", "a",
        "--fallback-glyph", "2", "a",
        "--fallback-glyph", ".", "a",
        "--fallback-glyph", ")", "a",
        "--flow-markdown", "12) a a a a a a a a\n\n1. a a a\n\n- a\n\n> a",
        "--flow-region", "30000", "40000", "36000", "25000",
        "--flow-list-quote-right-inset-um", "4000",
        "--flow-bullet-align", "right",
        "--flow-quote-align", "center",
    ]);
    let render = |alignment: Option<&str>, format: &str| {
        let mut command = source.clone();
        if let Some(align) = alignment {
            command.extend(args(&["--flow-ordered-align", align]));
        }
        if format == "ink" {
            command.push("--emit-ink-file".into());
        } else {
            command.extend(args(&["--export-paper", format]));
        }
        if format == "png" {
            command.extend(args(&["--dpi", "72", "--png-page", "2"]));
        }
        execute_notebook(&command).expect("native numbered export")
            .expect("PDF, PNG or TSV output")
    };
    for format in ["pdf", "png", "ink"] {
        let baseline = render(None, format);
        assert_eq!(baseline, render(Some("left"), format),
            "ordered-left retains prior {format} bytes");
        for alignment in ["center", "right"] {
            let selected = render(Some(alignment), format);
            assert_eq!(selected, render(Some(alignment), format),
                "deterministic {alignment} ordered {format}");
            assert_ne!(selected, baseline,
                "selected {alignment} ordered text changes {format}");
            match format {
                "pdf" => assert!(selected.starts_with(b"%PDF-1.4")),
                "png" => assert!(selected.starts_with(b"\x89PNG\r\n\x1a\n")),
                _ => assert!(selected.starts_with(b"atrament.page-ink.v1\n")),
            }
        }
    }
}

#[test]
fn invalid_markdown_ordered_alignment_preflights_and_preserves_pdf() {
    let source = args(&[
        "--flow-markdown-file", "-", "--glyph-file", "unopened.tsv",
        "--export-paper", "pdf",
    ]);
    for (tokens, expected) in [
        (vec!["--flow-ordered-align", "justify"],
            "--flow-ordered-align requires left, center or right"),
        (vec!["--flow-ordered-align"],
            "--flow-ordered-align requires left, center or right"),
        (vec!["--flow-ordered-align", "left",
            "--flow-ordered-align", "right"],
            "--flow-ordered-align may appear only once"),
    ] {
        let mut command = source.clone();
        command.extend(args(&tokens));
        assert_eq!(preflight_notebook_sources(&command)
            .expect_err("invalid option before selected input"), expected);
    }
    assert_eq!(preflight_notebook_sources(&args(&[
        "--flow-text", "a", "--export-paper", "pdf",
        "--flow-ordered-align", "center",
    ])).expect_err("Markdown-only numbered alignment"),
        "--flow-ordered-align requires a Markdown flow");
    let plain = FlowSource::new(FlowFormat::Plain, "a")
        .expect("plain source");
    assert_eq!(layout_flow_with_typography(
        &plain, &NotebookProfile::default(), None,
        HeadingDecoration::default(), None, FlowTypography {
            ordered_align: Some(BodyAlign::Center),
            ..FlowTypography::default()
        },
    ).expect_err("typed Markdown-only option"),
        "flow ordered alignment requires Markdown flow");
    let target = test_path().with_extension("pdf");
    fs::write(&target, b"preserve-existing-ordered-PDF")
        .expect("test sentinel");
    let mut command = args(&[
        "--printed-sheet", "true", "--export-paper", "pdf",
        "--glyph-stroke", "a", "900", "900", "2200", "3400",
        "250", "#204060",
        "--fallback-glyph", "1", "a",
        "--fallback-glyph", ".", "a",
        "--flow-markdown", "1. a a",
        "--flow-region", "30000", "40000", "21000", "26000",
        "--flow-list-quote-right-inset-um", "12000",
        "--flow-ordered-align", "right",
    ]);
    command.extend(args(&[
        "--output", target.to_str().expect("test path"), "--replace",
    ]));
    let err = execute_notebook(&command)
        .expect_err("unfit ordered row refuses before PDF replacement");
    assert!(err.contains("flow list/quote right inset leaves no writing row"),
        "{err}");
    assert_eq!(fs::read(&target).expect("original PDF"),
        b"preserve-existing-ordered-PDF");
    fs::remove_file(target).expect("clean test-owned sentinel");
}

#[test]
fn markdown_bullet_style_changes_only_unordered_marker_geometry() {
    let source = FlowSource::new(FlowFormat::Markdown,
        "- a\n* a\n+ a\n- [x] a\n1. a\n> a\n\na",
    ).expect("mixed Markdown rows");
    let region = FlowRegion {
        left: 30_000, top: 40_000, width: 80_000, height: 160_000,
    };
    let choose = |bullet_style| layout_flow_with_typography(
        &source, &NotebookProfile::default(), None,
        HeadingDecoration::default(), Some(region), FlowTypography {
            bullet_style,
            bullet_align: Some(BodyAlign::Right),
            list_quote_inset: Some(4_000),
            ..FlowTypography::default()
        },
    ).expect("selected marker geometry");
    let (rows, dashes, pages) = choose(None);
    let (explicit_rows, explicit_dashes, explicit_pages) =
        choose(Some(BulletStyle::Dash));
    assert_eq!(pages, explicit_pages);
    let row_coordinates = |runs: &[TextRun]| runs.iter()
        .map(|run| (run.text.clone(), run.page, run.origin_um))
        .collect::<Vec<_>>();
    assert_eq!(row_coordinates(&rows), row_coordinates(&explicit_rows));
    assert_eq!(dashes, explicit_dashes,
        "default dash must retain every byte of native ornament ink");
    let (dot_rows, dots, dot_pages) = choose(Some(BulletStyle::Dot));
    let (ring_rows, rings, ring_pages) = choose(Some(BulletStyle::Ring));
    assert_eq!((pages, pages), (dot_pages, ring_pages));
    assert_eq!(row_coordinates(&rows), row_coordinates(&dot_rows));
    assert_eq!(row_coordinates(&rows), row_coordinates(&ring_rows));
    assert_eq!(dashes.len(), 10,
        "three dash markers, six checkbox segments and one quote bar");
    assert_eq!(dots.len(), dashes.len());
    assert_eq!(rings.len(), 31,
        "three eight-segment rings, checkbox and quote unchanged");
    let first_dash = dashes.first().expect("first dash");
    let first_dot = dots.first().expect("first dot");
    let first_ring = rings.first().expect("first ring");
    assert_eq!(first_dash.start_um, (38_100, 44_700));
    assert_eq!(first_dash.end_um, (39_800, 44_700));
    assert_eq!(first_dash.width_um, 450);
    assert_eq!(first_dot.start_um, (38_800, 44_700));
    assert_eq!(first_dot.end_um, (39_100, 44_700));
    assert_eq!(first_dot.width_um, 850);
    assert_eq!(first_ring.start_um, (38_950, 43_850));
    assert_eq!(first_ring.end_um, (39_551, 44_099));
    assert_eq!(first_ring.width_um, 350);
    for marker in 0..3 {
        let ring_start = rings.get(marker * 8).expect("ring start");
        let ring_end = rings.get(marker * 8 + 7).expect("ring end");
        let dash = dashes.get(marker).expect("bullet dash");
        assert_eq!(ring_start.start_um, ring_end.end_um,
            "every outlined ring closes without guessing glyph strokes");
        assert_eq!(dash.page, ring_start.page);
    }
    assert_eq!(dashes.get(3..), dots.get(3..),
        "tasks, numbers and quotes keep their native markers");
    assert_eq!(dashes.get(3..), rings.get(24..));
}

#[test]
fn markdown_ring_bullets_do_not_repeat_on_wrapped_page_continuations() {
    let source = FlowSource::new(FlowFormat::Markdown,
        "- a a a a a a a a a a a a\n\n+ a a a a a a a a a a a a",
    ).expect("wrapped bullets");
    let region = FlowRegion {
        left: 30_000, top: 40_000, width: 24_000, height: 25_000,
    };
    let choose = |bullet_style| layout_flow_with_typography(
        &source, &NotebookProfile::default(), None,
        HeadingDecoration::default(), Some(region), FlowTypography {
            bullet_style,
            list_quote_right_inset: Some(1_000),
            ..FlowTypography::default()
        },
    ).expect("paginated bullet markers");
    let (dash_rows, dashes, dash_pages) = choose(None);
    let (ring_rows, rings, ring_pages) = choose(Some(BulletStyle::Ring));
    assert_eq!(dash_pages, ring_pages);
    let row_coordinates = |runs: &[TextRun]| runs.iter()
        .map(|run| (run.text.clone(), run.page, run.origin_um))
        .collect::<Vec<_>>();
    assert_eq!(row_coordinates(&dash_rows), row_coordinates(&ring_rows));
    assert!(ring_pages >= 3, "long list must paginate");
    assert_eq!(dashes.len(), 2, "one synthetic mark per source list item");
    assert_eq!(rings.len(), 16, "eight segments per outlined marker");
    let first_dash = dashes.first().expect("first dash");
    let next_dash = dashes.get(1).expect("next dash");
    let first_ring = rings.first().expect("first ring");
    let next_ring = rings.get(8).expect("next ring");
    assert!(first_dash.page < next_dash.page);
    assert_eq!((first_ring.page, next_ring.page),
        (first_dash.page, next_dash.page));
    assert_eq!(ring_rows.first().expect("first row").page, first_ring.page);
    assert_eq!(ring_rows.last().expect("last continuation").page,
        ring_pages);
}

#[test]
fn markdown_bullet_styles_replay_native_pdf_png_and_ink() {
    let source = args(&[
        "--printed-sheet", "true",
        "--glyph-stroke", "a", "900", "900", "2200", "3400",
        "250", "#204060",
        "--glyph-stroke", "1", "900", "900", "900", "3400",
        "250", "#204060",
        "--glyph-stroke", ".", "1500", "3200", "1650", "3400",
        "250", "#204060",
        "--flow-markdown", "- a a\n\n* a\n\n+ a\n\n- [x] a\n\n1. a",
        "--flow-bullet-align", "center",
    ]);
    let render = |choice: Option<&str>, format: &str| {
        let mut command = source.clone();
        if let Some(style) = choice {
            command.extend(args(&["--flow-bullet-style", style]));
        }
        if format == "ink" {
            command.push("--emit-ink-file".into());
        } else {
            command.extend(args(&["--export-paper", format]));
        }
        if format == "png" {
            command.extend(args(&["--dpi", "72"]));
        }
        execute_notebook(&command).expect("native styled-list export")
            .expect("PDF, PNG or versioned ink bytes")
    };
    for format in ["pdf", "png", "ink"] {
        let original = render(None, format);
        assert_eq!(original, render(Some("dash"), format),
            "explicit dash preserves default {format} bytes");
        for style in ["dot", "ring"] {
            let changed = render(Some(style), format);
            assert_eq!(changed, render(Some(style), format),
                "selected {style} {format} must replay deterministically");
            assert_ne!(changed, original,
                "selected {style} must alter physical {format} ink");
            match format {
                "pdf" => assert!(changed.starts_with(b"%PDF-1.4")),
                "png" => assert!(changed.starts_with(b"\x89PNG\r\n\x1a\n")),
                _ => assert!(changed.starts_with(b"atrament.page-ink.v1\n")),
            }
        }
    }
}

#[test]
fn markdown_bullet_style_rejects_early_and_preserves_selected_output() {
    let source = args(&[
        "--flow-markdown-file", "-", "--glyph-file", "unopened.tsv",
        "--export-paper", "pdf",
    ]);
    for (tokens, expected) in [
        (vec!["--flow-bullet-style", "square"],
            "--flow-bullet-style requires dash, dot or ring"),
        (vec!["--flow-bullet-style"],
            "--flow-bullet-style requires dash, dot or ring"),
        (vec!["--flow-bullet-style", "dot",
            "--flow-bullet-style", "ring"],
            "--flow-bullet-style may appear only once"),
    ] {
        let mut command = source.clone();
        command.extend(args(&tokens));
        assert_eq!(preflight_notebook_sources(&command)
            .expect_err("refuse before reading selected inputs"), expected);
    }
    assert_eq!(preflight_notebook_sources(&args(&[
        "--flow-text", "a", "--export-paper", "pdf",
        "--flow-bullet-style", "ring",
    ])).expect_err("Markdown-only selection"),
        "--flow-bullet-style requires a Markdown flow");
    let plain = FlowSource::new(FlowFormat::Plain, "a")
        .expect("plain text");
    assert_eq!(layout_flow_with_typography(
        &plain, &NotebookProfile::default(), None,
        HeadingDecoration::default(), None, FlowTypography {
            bullet_style: Some(BulletStyle::Ring),
            ..FlowTypography::default()
        },
    ).expect_err("typed Markdown-only selection"),
        "flow bullet style requires Markdown flow");
    let target = test_path().with_extension("pdf");
    fs::write(&target, b"preserve-original-bullet-PDF")
        .expect("sentinel PDF");
    let mut command = args(&[
        "--printed-sheet", "true", "--export-paper", "pdf",
        "--glyph-stroke", "a", "900", "900", "2200", "3400",
        "250", "#204060", "--flow-markdown", "- a a",
        "--flow-region", "30000", "40000", "8000", "10000",
        "--flow-bullet-style", "ring",
    ]);
    command.extend(args(&[
        "--output", target.to_str().expect("test path"), "--replace",
    ]));
    let error = execute_notebook(&command)
        .expect_err("narrow row refuses before replacement");
    assert!(error.contains("writing row")
        || error.contains("printable region"), "{error}");
    assert_eq!(fs::read(&target).expect("unchanged PDF"),
        b"preserve-original-bullet-PDF");
    fs::remove_file(target).expect("remove test-owned sentinel");
}

#[test]
fn markdown_task_diamonds_preserve_other_marks_and_default_ink() {
    let source = FlowSource::new(FlowFormat::Markdown,
        "- [x] a\n* [ ] a\n- a\n1. a\n> a\n\na",
    ).expect("mixed Markdown task blocks");
    let region = FlowRegion {
        left: 30_000, top: 40_000, width: 80_000, height: 160_000,
    };
    let choose = |task_style| layout_flow_with_typography(
        &source, &NotebookProfile::default(), None,
        HeadingDecoration::default(), Some(region), FlowTypography {
            task_style, task_align: Some(BodyAlign::Right),
            bullet_style: Some(BulletStyle::Ring),
            list_quote_inset: Some(4_000),
            ..FlowTypography::default()
        },
    ).expect("task style geometry");
    let (original_rows, original, original_pages) = choose(None);
    let (square_rows, square, square_pages) =
        choose(Some(TaskStyle::Square));
    assert_eq!(original_pages, square_pages);
    let rows = |runs: &[TextRun]| runs.iter()
        .map(|run| (run.text.clone(), run.page, run.origin_um))
        .collect::<Vec<_>>();
    assert_eq!(rows(&original_rows), rows(&square_rows),
        "the explicit square cannot alter the authored text runs");
    assert_eq!(original, square,
        "the default square must remain byte-for-byte identical");
    let (diamond_rows, diamond, diamond_pages) =
        choose(Some(TaskStyle::Diamond));
    assert_eq!(diamond_pages, original_pages);
    assert_eq!(rows(&diamond_rows), rows(&original_rows),
        "diamond task marks must not change row origins or pagination");
    assert_eq!(diamond.len(), original.len(),
        "each checkbox still uses four border and two check strokes");
    assert_eq!(original.len(), 19,
        "checked task six, open task four, ring eight and quote one");
    let expected_first = [
        ((39_150, 43_700), (40_400, 44_950)),
        ((40_400, 44_950), (39_150, 46_200)),
        ((39_150, 46_200), (37_900, 44_950)),
        ((37_900, 44_950), (39_150, 43_700)),
    ];
    for (actual, &(start, end)) in
        diamond.iter().take(4).zip(expected_first.iter()) {
        assert_eq!((actual.start_um, actual.end_um), (start, end));
        assert_eq!(actual.width_um, 350);
    }
    let first_check = diamond.get(4).expect("first check stroke");
    let second_check = diamond.get(5).expect("second check stroke");
    assert_eq!(first_check.start_um, (38_500, 44_950));
    assert_eq!(first_check.end_um, (38_900, 45_300));
    assert_eq!(second_check.start_um, first_check.end_um);
    assert_eq!(second_check.end_um, (39_700, 44_550));
    assert_eq!(
        original.get(10..).expect("unrelated ornaments"),
        diamond.get(10..).expect("unrelated ornaments"),
        "unordered rings, numbers, quote bars and body stay unchanged",
    );
}

#[test]
fn markdown_task_diamonds_do_not_repeat_on_wrapped_pages() {
    let source = FlowSource::new(FlowFormat::Markdown,
        "- [x] a a a a a a a a a a a a\n\n+ [ ] a a a a a a a a a a a a",
    ).expect("wrapped task source");
    let region = FlowRegion {
        left: 30_000, top: 40_000, width: 24_000, height: 25_000,
    };
    let choose = |task_style| layout_flow_with_typography(
        &source, &NotebookProfile::default(), None,
        HeadingDecoration::default(), Some(region), FlowTypography {
            task_style, list_quote_right_inset: Some(1_000),
            ..FlowTypography::default()
        },
    ).expect("task wrapping and page turns");
    let (original_rows, square, original_pages) = choose(None);
    let (diamond_rows, diamond, diamond_pages) =
        choose(Some(TaskStyle::Diamond));
    assert_eq!(original_pages, diamond_pages);
    assert!(original_pages >= 3, "task continuations must paginate");
    let coords = |runs: &[TextRun]| runs.iter()
        .map(|run| (run.text.clone(), run.page, run.origin_um))
        .collect::<Vec<_>>();
    assert_eq!(coords(&original_rows), coords(&diamond_rows));
    assert_eq!(square.len(), 10, "only two first-row checkboxes");
    assert_eq!(diamond.len(), 10, "no marker on continuation pages");
    let first = diamond.first().expect("first diamond");
    let next = diamond.get(6).expect("second diamond");
    assert!(first.page < next.page);
    for &(open, close) in &[(0, 3), (6, 9)] {
        let start = diamond.get(open).expect("diamond opening");
        let end = diamond.get(close).expect("diamond closure");
        assert_eq!(start.start_um, end.end_um,
            "each diamond must close without synthesizing a glyph");
    }
    let first_check = diamond.get(4).expect("first check stroke");
    let second_check = diamond.get(5).expect("second check stroke");
    assert_eq!(first_check.end_um, second_check.start_um,
        "the checked diamond keeps one connected checkmark");
}

#[test]
fn markdown_task_styles_replay_native_pdf_png_and_ink() {
    let source = args(&[
        "--printed-sheet", "true",
        "--glyph-stroke", "a", "900", "900", "2200", "3400",
        "250", "#204060", "--flow-markdown",
        "- [x] a a\n\n+ [ ] a a\n\n* [X] a a",
        "--flow-region", "30000", "40000", "34000", "26000",
        "--flow-task-align", "center",
        "--flow-bullet-style", "ring",
    ]);
    let render = |selection: Option<&str>, format: &str| {
        let mut command = source.clone();
        if let Some(style) = selection {
            command.extend(args(&["--flow-task-style", style]));
        }
        if format == "ink" {
            command.push("--emit-ink-file".into());
        } else {
            command.extend(args(&["--export-paper", format]));
        }
        if format == "png" {
            command.extend(args(&["--dpi", "72", "--png-page", "2"]));
        }
        execute_notebook(&command).expect("native task export")
            .expect("PDF, PNG or physical ink TSV bytes")
    };
    for format in ["pdf", "png", "ink"] {
        let baseline = render(None, format);
        assert_eq!(baseline, render(Some("square"), format),
            "explicit square must replay original {format} bytes");
        let diamond = render(Some("diamond"), format);
        assert_eq!(diamond, render(Some("diamond"), format),
            "diamond task {format} must be deterministic");
        assert_ne!(diamond, baseline,
            "diamond must alter physical {format} ink");
        match format {
            "pdf" => assert!(diamond.starts_with(b"%PDF-1.4")),
            "png" => assert!(diamond.starts_with(b"\x89PNG\r\n\x1a\n")),
            _ => assert!(diamond.starts_with(b"atrament.page-ink.v1\n")),
        }
    }
}

#[test]
fn markdown_task_style_preflights_and_preserves_existing_output() {
    let source = args(&[
        "--flow-markdown-file", "-", "--glyph-file", "unopened.tsv",
        "--export-paper", "pdf",
    ]);
    for (options, expected) in [
        (vec!["--flow-task-style", "circle"],
            "--flow-task-style requires square or diamond"),
        (vec!["--flow-task-style"],
            "--flow-task-style requires square or diamond"),
        (vec!["--flow-task-style", "diamond",
            "--flow-task-style", "square"],
            "--flow-task-style may appear only once"),
    ] {
        let mut command = source.clone();
        command.extend(args(&options));
        assert_eq!(preflight_notebook_sources(&command)
            .expect_err("reject invalid selections before file/stdin"),
            expected);
    }
    assert_eq!(preflight_notebook_sources(&args(&[
        "--flow-text", "a", "--export-paper", "pdf",
        "--flow-task-style", "diamond",
    ])).expect_err("Markdown-only task style"),
        "--flow-task-style requires a Markdown flow");
    let plain = FlowSource::new(FlowFormat::Plain, "a")
        .expect("plain source");
    assert_eq!(layout_flow_with_typography(
        &plain, &NotebookProfile::default(), None,
        HeadingDecoration::default(), None, FlowTypography {
            task_style: Some(TaskStyle::Diamond),
            ..FlowTypography::default()
        },
    ).expect_err("typed Markdown-only task style"),
        "flow task style requires Markdown flow");
    let target = test_path().with_extension("pdf");
    fs::write(&target, b"preserve-selected-task-PDF")
        .expect("test sentinel");
    let mut command = args(&[
        "--printed-sheet", "true", "--export-paper", "pdf",
        "--glyph-stroke", "a", "900", "900", "2200", "3400",
        "250", "#204060", "--flow-markdown", "- [x] a a",
        "--flow-region", "30000", "40000", "8000", "10000",
        "--flow-task-style", "diamond",
    ]);
    command.extend(args(&[
        "--output", target.to_str().expect("test path"), "--replace",
    ]));
    let error = execute_notebook(&command)
        .expect_err("unfit task row refuses before replacement");
    assert!(error.contains("writing row")
        || error.contains("printable region"), "{error}");
    assert_eq!(fs::read(&target).expect("unchanged PDF"),
        b"preserve-selected-task-PDF");
    fs::remove_file(target).expect("clean test sentinel");
}

#[test]
fn markdown_list_marker_colors_change_only_their_own_pen_ink() {
    let source = FlowSource::new(FlowFormat::Markdown,
        "- a\n- [x] a\n+ [ ] a\n1. a\n> a\n\n# a\n\na",
    ).expect("mixed Markdown list and other structural rows");
    let region = FlowRegion {
        left: 30_000, top: 40_000, width: 80_000, height: 160_000,
    };
    let row_contract = |rows: &[TextRun]| rows.iter()
        .map(|run| (run.page, run.origin_um, run.text.clone()))
        .collect::<Vec<_>>();
    for bullet_style in [BulletStyle::Dash, BulletStyle::Dot,
        BulletStyle::Ring] {
        for task_style in [TaskStyle::Square, TaskStyle::Diamond] {
            let choose = |bullet_color, task_color| {
                layout_flow_with_typography(
                    &source, &NotebookProfile::default(), None,
                    HeadingDecoration::default(), Some(region),
                    FlowTypography {
                        bullet_style: Some(bullet_style),
                        task_style: Some(task_style),
                        bullet_color, task_color,
                        bullet_align: Some(BodyAlign::Center),
                        task_align: Some(BodyAlign::Right),
                        ..FlowTypography::default()
                    },
                ).expect("list marker pen colors")
            };
            let (base_rows, base_marks, pages) = choose(None, None);
            let (same_rows, same_marks, same_pages) =
                choose(Some([30, 48, 74]), Some([30, 48, 74]));
            assert_eq!(same_pages, pages);
            assert_eq!(row_contract(&base_rows), row_contract(&same_rows));
            assert_eq!(base_marks, same_marks,
                "explicit historic colors preserve the complete ink");
            let (colored_rows, colored_marks, colored_pages) =
                choose(Some([0, 127, 255]), Some([250, 3, 64]));
            assert_eq!(colored_pages, pages);
            assert_eq!(row_contract(&colored_rows), row_contract(&base_rows));
            assert_eq!(colored_marks.len(), base_marks.len());
            let bullet_segments = if bullet_style == BulletStyle::Ring {
                8
            } else {
                1
            };
            for (index, (old, new)) in base_marks.iter()
                .zip(colored_marks.iter()).enumerate() {
                assert_eq!((old.page, old.start_um, old.end_um, old.width_um),
                    (new.page, new.start_um, new.end_um, new.width_um),
                    "physical marker geometry changed at index {index}");
                let expected_rgb = if index < bullet_segments {
                    [0, 127, 255]
                } else if index < bullet_segments + 10 {
                    [250, 3, 64]
                } else {
                    [30, 48, 74]
                };
                assert_eq!(new.rgb, expected_rgb,
                    "only the selected list marker receives new color");
                assert_eq!(old.rgb, [30, 48, 74]);
            }
        }
    }
}

#[test]
fn markdown_list_marker_colors_stay_on_first_wrapped_page_rows() {
    let source = FlowSource::new(FlowFormat::Markdown,
        concat!(
            "- a a a a a a a a a a a a\n\n",
            "- [x] a a a a a a a a a a a a\n\n",
            "+ [ ] a a a a a a a a a a a a",
        ),
    ).expect("wrapped Markdown list source");
    let region = FlowRegion {
        left: 30_000, top: 40_000, width: 24_000, height: 25_000,
    };
    let choose = |bullet_color, task_color| {
        layout_flow_with_typography(
            &source, &NotebookProfile::default(), None,
            HeadingDecoration::default(), Some(region),
            FlowTypography {
                bullet_color, task_color,
                bullet_style: Some(BulletStyle::Ring),
                task_style: Some(TaskStyle::Diamond),
                list_quote_inset: Some(2_000),
                ..FlowTypography::default()
            },
        ).expect("multi-page colored list")
    };
    let (original_rows, original_marks, original_pages) =
        choose(None, None);
    let (color_rows, color_marks, color_pages) =
        choose(Some([9, 44, 210]), Some([230, 21, 88]));
    assert!(original_pages >= 3);
    assert_eq!(original_pages, color_pages);
    let row_contract = |rows: &[TextRun]| rows.iter()
        .map(|run| (run.page, run.origin_um, run.text.clone()))
        .collect::<Vec<_>>();
    assert_eq!(row_contract(&original_rows), row_contract(&color_rows));
    assert_eq!(original_marks.len(), 18,
        "one ring and two diamonds with one check each");
    assert_eq!(original_marks.len(), color_marks.len());
    for (original, colored) in original_marks.iter()
        .zip(color_marks.iter()) {
        assert_eq!((original.page, original.start_um, original.end_um,
            original.width_um), (colored.page, colored.start_um,
            colored.end_um, colored.width_um));
    }
    let bullet_page = color_marks.first().expect("ring first segment").page;
    let checked_page = color_marks.get(8).expect("checked box").page;
    let open_page = color_marks.get(14).expect("open box").page;
    assert!(bullet_page < checked_page && checked_page < open_page);
    assert!(color_marks.iter().take(8).all(|mark|
        mark.page == bullet_page && mark.rgb == [9, 44, 210]));
    assert!(color_marks.iter().skip(8).take(6).all(|mark|
        mark.page == checked_page && mark.rgb == [230, 21, 88]));
    assert!(color_marks.iter().skip(14).take(4).all(|mark|
        mark.page == open_page && mark.rgb == [230, 21, 88]));
}

#[test]
fn markdown_list_marker_colors_replay_pdf_png_and_ink_bytes() {
    let source = args(&[
        "--printed-sheet", "true",
        "--glyph-stroke", "a", "900", "900", "2200", "3400",
        "250", "#204060", "--flow-markdown",
        "- a\n\n- [x] a\n\n+ [ ] a",
        "--flow-region", "30000", "40000", "34000", "26000",
        "--flow-bullet-style", "ring",
        "--flow-task-style", "diamond",
    ]);
    let render = |colors: Option<(&str, &str)>, format: &str| {
        let mut command = source.clone();
        if let Some((bullet, task)) = colors {
            command.extend(args(&[
                "--flow-bullet-color", bullet,
                "--flow-task-color", task,
            ]));
        }
        if format == "ink" {
            command.push("--emit-ink-file".into());
        } else {
            command.extend(args(&["--export-paper", format]));
        }
        if format == "png" {
            command.extend(args(&["--dpi", "72", "--png-page", "2"]));
        }
        execute_notebook(&command).expect("native colored export")
            .expect("PDF, PNG or ink TSV bytes")
    };
    for format in ["pdf", "png", "ink"] {
        let original = render(None, format);
        assert_eq!(original,
            render(Some(("#1E304A", "#1e304a")), format),
            "explicit historical colors must keep original {format} bytes");
        let altered = render(Some(("#007FFF", "#FA0340")), format);
        assert_eq!(altered,
            render(Some(("#007FFF", "#FA0340")), format),
            "colored {format} must replay exactly");
        assert_ne!(original, altered,
            "selected list mark color should change physical {format}");
        match format {
            "pdf" => assert!(altered.starts_with(b"%PDF-1.4")),
            "png" => assert!(altered.starts_with(b"\x89PNG\r\n\x1a\n")),
            _ => {
                assert!(altered.starts_with(b"atrament.page-ink.v1\n"));
                let ink = str::from_utf8(&altered)
                    .expect("native TSV uses UTF-8");
                assert!(ink.contains("#007FFF"));
                assert!(ink.contains("#FA0340"));
            }
        }
    }
}

#[test]
fn markdown_list_marker_colors_preflight_and_preserve_output() {
    let source = args(&[
        "--flow-markdown-file", "-", "--glyph-file", "unopened.tsv",
        "--export-paper", "pdf",
    ]);
    for (option, valid) in [
        ("--flow-bullet-color", "#FE1234"),
        ("--flow-task-color", "#1234AA"),
    ] {
        for rejected in ["bad", "#12GG00", "#12345", "#1122334",
            "112233", "#\u{2603}0000"] {
            let mut command = source.clone();
            command.extend(args(&[option, rejected]));
            assert_eq!(preflight_notebook_sources(&command)
                .expect_err("reject color before reading source"),
                format!("{option} requires #RRGGBB"));
        }
        let mut missing = source.clone();
        missing.push(option.into());
        assert_eq!(preflight_notebook_sources(&missing)
            .expect_err("missing color"), format!("{option} requires #RRGGBB"));
        let mut duplicate = source.clone();
        duplicate.extend(args(&[option, valid, option, valid]));
        assert_eq!(preflight_notebook_sources(&duplicate)
            .expect_err("duplicate color"),
            format!("{option} may appear only once"));
        let mut plain = args(&[
            "--flow-text", "a", "--export-paper", "pdf",
        ]);
        plain.extend(args(&[option, valid]));
        assert_eq!(preflight_notebook_sources(&plain)
            .expect_err("Markdown-only color"),
            format!("{option} requires a Markdown flow"));
    }
    let plain = FlowSource::new(FlowFormat::Plain, "a")
        .expect("plain source");
    for (selection, expected) in [
        (FlowTypography {
            bullet_color: Some([1, 2, 3]),
            ..FlowTypography::default()
        }, "flow bullet color requires Markdown flow"),
        (FlowTypography {
            task_color: Some([1, 2, 3]),
            ..FlowTypography::default()
        }, "flow task color requires Markdown flow"),
    ] {
        assert_eq!(layout_flow_with_typography(
            &plain, &NotebookProfile::default(), None,
            HeadingDecoration::default(), None, selection,
        ).expect_err("typed Markdown-only color"), expected);
    }
    let target = test_path().with_extension("pdf");
    fs::write(&target, b"preserve-selected-color-PDF")
        .expect("test sentinel");
    let mut command = args(&[
        "--printed-sheet", "true", "--export-paper", "pdf",
        "--glyph-stroke", "a", "900", "900", "2200", "3400",
        "250", "#204060", "--flow-markdown", "- [x] a a",
        "--flow-region", "30000", "40000", "8000", "10000",
        "--flow-bullet-color", "#007FFF",
        "--flow-task-color", "#FA0340",
    ]);
    command.extend(args(&[
        "--output", target.to_str().expect("test path"), "--replace",
    ]));
    let error = execute_notebook(&command)
        .expect_err("unfit color export refuses before replacement");
    assert!(error.contains("writing row")
        || error.contains("printable region"), "{error}");
    assert_eq!(fs::read(&target).expect("unchanged PDF"),
        b"preserve-selected-color-PDF");
    fs::remove_file(target).expect("remove test sentinel");
}

#[test]
fn markdown_quote_color_changes_only_quote_bars() {
    let source = FlowSource::new(FlowFormat::Markdown,
        "# a\n\na a\n\n> a a  \n> a\n\n- a\n\n- [x] a\n\n1. a\n\n---",
    ).expect("mixed Markdown including two quote bars");
    let region = FlowRegion {
        left: 30_000, top: 40_000, width: 70_000, height: 200_000,
    };
    let choose = |quote_color| layout_flow_with_typography(
        &source, &NotebookProfile::default(), None,
        HeadingDecoration::default(), Some(region), FlowTypography {
            quote_color,
            quote_align: Some(BodyAlign::Right),
            list_quote_inset: Some(4_000),
            list_quote_right_inset: Some(6_000),
            bullet_color: Some([20, 200, 40]),
            task_color: Some([220, 30, 60]),
            ..FlowTypography::default()
        },
    ).expect("colored quote bars with other Markdown ornaments");
    let (default_rows, default_marks, default_pages) = choose(None);
    let (old_rows, old_marks, old_pages) =
        choose(Some([30, 48, 74]));
    let rows = |runs: &[TextRun]| runs.iter().map(|run|
        (run.page, run.origin_um, run.text.clone())
    ).collect::<Vec<_>>();
    assert_eq!(default_pages, old_pages);
    assert_eq!(rows(&default_rows), rows(&old_rows));
    assert_eq!(default_marks, old_marks);
    let (chosen_rows, chosen_marks, chosen_pages) =
        choose(Some([30, 120, 240]));
    assert_eq!(default_pages, chosen_pages);
    assert_eq!(rows(&default_rows), rows(&chosen_rows));
    assert_eq!(default_marks.len(), chosen_marks.len());
    assert_eq!(default_marks.len(), 11,
        "heading one, quotes two, bullet one, task six, rule one");
    let mut changed = 0;
    for (original, recolored) in
        default_marks.iter().zip(chosen_marks.iter()) {
        assert_eq!((original.page, original.start_um, original.end_um,
            original.width_um), (recolored.page, recolored.start_um,
            recolored.end_um, recolored.width_um));
        if original.width_um == 400
            && original.start_um.0 == original.end_um.0 {
            assert_eq!(original.rgb, [30, 48, 74]);
            assert_eq!(recolored.rgb, [30, 120, 240]);
            changed += 1;
        } else {
            assert_eq!(original.rgb, recolored.rgb,
                "non-quote ornaments remain independently colored");
        }
    }
    assert_eq!(changed, 2, "only two quote bars receive new ink");
}

#[test]
fn markdown_quote_color_keeps_wrapped_bars_on_their_actual_pages() {
    let source = FlowSource::new(FlowFormat::Markdown,
        "> a a a a a a a a a a a a a a a a",
    ).expect("long quoted paragraph");
    let region = FlowRegion {
        left: 30_000, top: 40_000, width: 34_000, height: 25_000,
    };
    let choose = |quote_color| layout_flow_with_typography(
        &source, &NotebookProfile::default(), None,
        HeadingDecoration::default(), Some(region), FlowTypography {
            quote_color, quote_align: Some(BodyAlign::Center),
            list_quote_right_inset: Some(6_000),
            ..FlowTypography::default()
        },
    ).expect("multi-page quoted paragraph");
    let (original_rows, original_bars, original_pages) = choose(None);
    let (colored_rows, colored_bars, colored_pages) =
        choose(Some([16, 32, 240]));
    assert!(original_pages >= 3);
    assert_eq!(original_pages, colored_pages);
    let rows = |runs: &[TextRun]| runs.iter().map(|run|
        (run.page, run.origin_um, run.text.clone())
    ).collect::<Vec<_>>();
    assert_eq!(rows(&original_rows), rows(&colored_rows));
    assert_eq!(original_bars.len(), original_rows.len(),
        "each physical quoted row owns a bar");
    assert_eq!(original_bars.len(), colored_bars.len());
    for (original, chosen) in original_bars.iter()
        .zip(colored_bars.iter()) {
        assert_eq!((original.page, original.start_um, original.end_um,
            original.width_um), (chosen.page, chosen.start_um,
            chosen.end_um, chosen.width_um),
            "bar placement including balanced page tails is unchanged");
        assert_eq!(original.rgb, [30, 48, 74]);
        assert_eq!(chosen.rgb, [16, 32, 240]);
    }
    assert_eq!(colored_bars.last().map(|mark| mark.page),
        Some(original_pages));
}

#[test]
fn markdown_quote_color_replays_real_pdf_png_and_ink() {
    let source = args(&[
        "--printed-sheet", "true",
        "--glyph-stroke", "a", "900", "900", "2200", "3400",
        "250", "#204060", "--flow-markdown",
        "> a a a a a a a a a a\n\n- a\n\n- [x] a",
        "--flow-region", "30000", "40000", "30000", "26000",
        "--flow-list-quote-inset-um", "4000",
        "--flow-quote-align", "center",
        "--flow-bullet-color", "#006400",
        "--flow-task-color", "#FA0340",
    ]);
    let render = |color: Option<&str>, format: &str| {
        let mut command = source.clone();
        if let Some(value) = color {
            command.extend(args(&["--flow-quote-color", value]));
        }
        if format == "ink" {
            command.push("--emit-ink-file".into());
        } else {
            command.extend(args(&["--export-paper", format]));
        }
        if format == "png" {
            command.extend(args(&["--dpi", "72", "--png-page", "2"]));
        }
        execute_notebook(&command).expect("native colored quote export")
            .expect("physical export bytes")
    };
    for format in ["pdf", "png", "ink"] {
        let default = render(None, format);
        assert_eq!(default, render(Some("#1E304A"), format),
            "historic quote ink remains byte-equivalent in {format}");
        let chosen = render(Some("#1070E0"), format);
        assert_eq!(chosen, render(Some("#1070E0"), format),
            "chosen quote ink is deterministic in {format}");
        assert_ne!(default, chosen,
            "changed quote ink must affect {format} output");
        match format {
            "pdf" => assert!(chosen.starts_with(b"%PDF-1.4")),
            "png" => assert!(chosen.starts_with(b"\x89PNG\r\n\x1a\n")),
            _ => {
                assert!(chosen.starts_with(b"atrament.page-ink.v1\n"));
                let ink = str::from_utf8(&chosen)
                    .expect("physical TSV is UTF-8");
                assert!(ink.contains("#1070E0"));
                assert!(ink.contains("#006400"));
                assert!(ink.contains("#FA0340"));
            }
        }
    }
}

#[test]
fn markdown_quote_color_refuses_early_without_overwriting_pdf() {
    let source = args(&[
        "--flow-markdown-file", "-", "--glyph-file", "unopened.tsv",
        "--export-paper", "pdf",
    ]);
    for (choice, expected) in [
        (vec!["--flow-quote-color", "purple"],
            "--flow-quote-color requires #RRGGBB"),
        (vec!["--flow-quote-color", "#ABCDG1"],
            "--flow-quote-color requires #RRGGBB"),
        (vec!["--flow-quote-color"],
            "--flow-quote-color requires #RRGGBB"),
        (vec!["--flow-quote-color", "#000000",
            "--flow-quote-color", "#FFFFFF"],
            "--flow-quote-color may appear only once"),
    ] {
        let mut command = source.clone();
        command.extend(args(&choice));
        assert_eq!(preflight_notebook_sources(&command)
            .expect_err("invalid quote color before source IO"), expected);
    }
    assert_eq!(preflight_notebook_sources(&args(&[
        "--flow-text", "a", "--export-paper", "pdf",
        "--flow-quote-color", "#123456",
    ])).expect_err("Markdown-only quote color"),
        "--flow-quote-color requires a Markdown flow");
    let plain = FlowSource::new(FlowFormat::Plain, "a")
        .expect("plain source");
    assert_eq!(layout_flow_with_typography(
        &plain, &NotebookProfile::default(), None,
        HeadingDecoration::default(), None, FlowTypography {
            quote_color: Some([1, 2, 3]),
            ..FlowTypography::default()
        },
    ).expect_err("typed Markdown-only quote color"),
        "flow quote color requires Markdown flow");
    let target = test_path().with_extension("pdf");
    fs::write(&target, b"preserve-quote-color-PDF")
        .expect("test sentinel");
    let mut command = args(&[
        "--printed-sheet", "true", "--export-paper", "pdf",
        "--glyph-stroke", "a", "900", "900", "2200", "3400",
        "250", "#204060", "--flow-markdown", "> a a",
        "--flow-region", "30000", "40000", "21000", "26000",
        "--flow-list-quote-right-inset-um", "12000",
        "--flow-quote-color", "#1070E0",
    ]);
    command.extend(args(&[
        "--output", target.to_str().expect("test path"), "--replace",
    ]));
    let error = execute_notebook(&command)
        .expect_err("unfit quote layout cannot replace PDF");
    assert!(error.contains("flow list/quote right inset leaves no writing row"),
        "{error}");
    assert_eq!(fs::read(&target).expect("old PDF"),
        b"preserve-quote-color-PDF");
    fs::remove_file(target).expect("remove test sentinel");
}

#[test]
fn exact_lattice_split_roundtrips_unicode_tsv_and_native_pdf_png() {
    let source = test_path().with_extension("tsv");
    let target = test_path().with_extension("tsv");
    let source_name = source.to_str().expect("exact source path");
    let target_name = target.to_str().expect("exact destination path");
    let nfd = "n\u{303}";
    let original = execute_glyph_library(&args(&[
        "--glyph-stroke", nfd, "1000", "900", "1603", "1704",
        "251", "#123456",
        "--glyph-stroke", "ñ", "500", "700", "800", "900",
        "251", "#ABCDEF",
        "--glyph-stroke", nfd, "3200", "3500", "2597", "2696",
        "251", "#123456",
        "--glyph-stroke", "∞", "600", "600", "800", "900",
        "250", "#234567",
    ])).expect("authored unicode source").expect("source TSV");
    fs::write(&source, &original).expect("write independent source");
    let edit = args(&[
        "--profile", source_name,
        "--split-stroke-exact", nfd, "1",
        "--split-stroke-exact", nfd, "3",
    ]);
    let revised = execute_glyph_library(&edit)
        .expect("two exact lattice splits").expect("TSV bytes");
    let expected = format!(concat!(
        "atrament.glyph-strokes.v1\n",
        "{nfd}\t1000\t900\t1303\t1304\t251\t#123456\n",
        "{nfd}\t1303\t1304\t1603\t1704\t251\t#123456\n",
        "ñ\t500\t700\t800\t900\t251\t#ABCDEF\n",
        "{nfd}\t3200\t3500\t2897\t3096\t251\t#123456\n",
        "{nfd}\t2897\t3096\t2597\t2696\t251\t#123456\n",
        "∞\t600\t600\t800\t900\t250\t#234567\n",
    ), nfd = nfd);
    assert_eq!(revised, expected.as_bytes(),
        "exact line lattice point must agree with browser calculation");
    assert_eq!(fs::read(&source).expect("original remains"), original);
    let merged = execute_glyph_library(&args(&[
        "--profile", source_name,
        "--split-stroke-exact", nfd, "1",
        "--split-stroke-exact", nfd, "3",
        "--merge-strokes", nfd, "1",
        "--merge-strokes", nfd, "2",
    ])).expect("lossless remerge").expect("source ink restored");
    assert_eq!(merged, original,
        "both original directed lines must return byte-for-byte");
    let mut inspect = edit.clone();
    inspect.extend(args(&["--inspect-paths", nfd]));
    let report = String::from_utf8(execute_glyph_library(&inspect)
        .expect("native path inspection").expect("report"))
        .expect("UTF-8 path inspection");
    assert!(report.contains("Strokes: 4\nPaths: 2\nPen lifts: 1"),
        "{report}");
    let mut save = edit;
    save.extend(args(&["--output", target_name]));
    assert!(execute_glyph_library(&save)
        .expect("explicit saved exact split").is_none());
    assert_eq!(fs::read(&target).expect("saved split"), revised);
    assert_eq!(execute_glyph_library(&args(&[
        "--profile", target_name,
    ])).expect("reopened split").expect("TSV"), revised);
    for format in ["pdf", "png"] {
        let mut imported = args(&[
            "--glyph-file", target_name,
            "--place-text", "1", "25000", "30000", "n\u{303}ñ∞",
            "--export-paper", format,
        ]);
        let mut authored = args(&[
            "--glyph-stroke", nfd, "1000", "900", "1303", "1304",
            "251", "#123456",
            "--glyph-stroke", nfd, "1303", "1304", "1603", "1704",
            "251", "#123456",
            "--glyph-stroke", "ñ", "500", "700", "800", "900",
            "251", "#ABCDEF",
            "--glyph-stroke", nfd, "3200", "3500", "2897", "3096",
            "251", "#123456",
            "--glyph-stroke", nfd, "2897", "3096", "2597", "2696",
            "251", "#123456",
            "--glyph-stroke", "∞", "600", "600", "800", "900",
            "250", "#234567",
            "--place-text", "1", "25000", "30000", "n\u{303}ñ∞",
            "--export-paper", format,
        ]);
        if format == "png" {
            imported.extend(args(&["--dpi", "72"]));
            authored.extend(args(&["--dpi", "72"]));
        }
        assert_eq!(execute_notebook(&imported)
            .expect("imported exact ink renders").expect("export"),
            execute_notebook(&authored)
                .expect("independent ink renders").expect("export"),
            "native {format} cannot diverge from independently authored ink");
    }
    fs::remove_file(source).expect("remove exact test source");
    fs::remove_file(target).expect("remove exact test output");
}

#[test]
fn exact_lattice_split_refuses_primitive_and_incomplete_edits_atomically() {
    for flags in [
        args(&["--profile", "-", "--split-stroke-exact"]),
        args(&["--profile", "-", "--split-stroke-exact", "ñ"]),
    ] {
        let error = preflight_glyph_profile_sources(&flags)
            .expect_err("incomplete exact edit must refuse before stdin");
        assert!(error.contains("--split-stroke-exact requires 2 operands"),
            "{error}");
    }
    for index in ["0", "-1", "1.0", "4097", "invalid"] {
        let error = preflight_glyph_profile_sources(&args(&[
            "--profile", "-", "--split-stroke-exact", "ñ", index,
        ])).expect_err("invalid ordinal must refuse before stdin");
        assert!(error.contains("stroke index"), "{index}: {error}");
    }
    let invalid = preflight_glyph_profile_sources(&args(&[
        "--profile", "-", "--split-stroke-exact", "na", "1",
    ])).expect_err("nonexact Unicode key refused before input I/O");
    assert!(invalid.contains("exact visible grapheme"), "{invalid}");

    let source = test_path().with_extension("tsv");
    let target = test_path().with_extension("tsv");
    let source_name = source.to_str().expect("primitive source path");
    let target_name = target.to_str().expect("destination path");
    let original = execute_glyph_library(&args(&[
        "--glyph-stroke", "n\u{303}", "1000", "900", "1601", "1702",
        "251", "#123456",
        "--glyph-stroke", "ñ", "1000", "1000", "1001", "1000",
        "250", "#112233",
    ])).expect("valid primitive source").expect("source bytes");
    fs::write(&source, &original).expect("save source");
    fs::write(&target, b"accepted native output")
        .expect("write output sentinel");
    for (glyph, index, diagnostic) in [
        ("n\u{303}", "1", "strict integer point"),
        ("ñ", "1", "strict integer point"),
        ("n", "1", "no authored strokes"),
        ("ñ", "2", "exceeds glyph"),
    ] {
        let error = execute_glyph_library(&args(&[
            "--profile", source_name,
            "--split-stroke-exact", glyph, index,
            "--output", target_name, "--replace",
        ])).expect_err("rejected exact split cannot replace a file");
        assert!(error.contains(diagnostic), "{error}");
        assert_eq!(fs::read(&source).expect("source unmodified"), original);
        assert_eq!(fs::read(&target).expect("target unmodified"),
            b"accepted native output");
    }
    let full = test_path().with_extension("tsv");
    let full_name = full.to_str().expect("full source path");
    let full_tsv = String::from("atrament.glyph-strokes.v1\n")
        + &"∞\t1000\t1000\t1603\t1704\t250\t#112233\n".repeat(4096);
    fs::write(&full, &full_tsv).expect("save full source");
    let failure = execute_glyph_library(&args(&[
        "--profile", full_name,
        "--split-stroke-exact", "∞", "1",
        "--output", target_name, "--replace",
    ])).expect_err("maximum source cannot grow");
    assert!(failure.contains("4096"), "{failure}");
    assert_eq!(fs::read(&target).expect("target remains"),
        b"accepted native output");
    fs::remove_file(source).expect("remove primitive test source");
    fs::remove_file(target).expect("remove primitive test output");
    fs::remove_file(full).expect("remove full test input");
}

#[test]
fn native_selected_path_scaling_round_trips_unicode_and_physical_ink() {
    let source = test_path().with_extension("tsv");
    let revision = test_path().with_extension("tsv");
    let source_name = source.to_str().expect("source path");
    let revision_name = revision.to_str().expect("revision path");
    let original = execute_glyph_library(&args(&[
        "--glyph-stroke", "n\u{303}", "500", "700", "900", "1200",
        "251", "#112233",
        "--glyph-stroke", "ñ", "2200", "600", "2200", "3300",
        "300", "#778899",
        "--glyph-stroke", "n\u{303}", "900", "1200", "1400", "1900",
        "251", "#112233",
        "--glyph-stroke", "∞", "800", "800", "1700", "1500",
        "200", "#445566",
        "--glyph-stroke", "n\u{303}", "2200", "2500", "2700", "3000",
        "200", "#445566",
    ])).expect("authored ink").expect("source bytes");
    fs::write(&source, &original).expect("own source file");
    let command = args(&[
        "--profile", source_name,
        "--scale-path", "n\u{303}", "1", "125", "75",
    ]);
    let scaled = execute_glyph_library(&command)
        .expect("admitted scaling").expect("scaled TSV");
    let expected = concat!(
        "atrament.glyph-strokes.v1\n",
        "n\u{303}\t500\t700\t1000\t1075\t251\t#112233\n",
        "ñ\t2200\t600\t2200\t3300\t300\t#778899\n",
        "n\u{303}\t1000\t1075\t1625\t1600\t251\t#112233\n",
        "∞\t800\t800\t1700\t1500\t200\t#445566\n",
        "n\u{303}\t2200\t2500\t2700\t3000\t200\t#445566\n",
    ).as_bytes();
    assert_eq!(scaled, expected);
    assert_eq!(fs::read(&source).expect("unchanged source"), original);
    let mut save = command;
    save.extend(args(&["--output", revision_name]));
    assert!(execute_glyph_library(&save).expect("saved revision").is_none());
    assert_eq!(fs::read(&revision).expect("saved bytes"), expected);
    let report = String::from_utf8(execute_glyph_library(&args(&[
        "--profile", revision_name, "--inspect-paths", "n\u{303}",
    ])).expect("inspection").expect("report")).expect("UTF-8");
    assert!(report.contains("Strokes: 3\nPaths: 2\nPen lifts: 1"),
        "{report}");
    for format in ["pdf", "png"] {
        let mut saved = args(&[
            "--export-paper", format, "--printed-sheet", "true",
            "--glyph-file", revision_name,
            "--place-text", "1", "30000", "45000", "ñn\u{303}",
        ]);
        let mut inline = args(&[
            "--export-paper", format, "--printed-sheet", "true",
            "--glyph-stroke", "ñ", "2200", "600", "2200", "3300",
            "300", "#778899",
            "--glyph-stroke", "n\u{303}", "500", "700", "1000",
            "1075", "251", "#112233",
            "--glyph-stroke", "n\u{303}", "1000", "1075", "1625",
            "1600", "251", "#112233",
            "--glyph-stroke", "n\u{303}", "2200", "2500", "2700",
            "3000", "200", "#445566",
            "--place-text", "1", "30000", "45000", "ñn\u{303}",
        ]);
        if format == "png" {
            saved.extend(args(&["--dpi", "72"]));
            inline.extend(args(&["--dpi", "72"]));
        }
        let actual = execute_notebook(&saved)
            .expect("native export").expect("export bytes");
        let direct = execute_notebook(&inline)
            .expect("independent geometry").expect("export bytes");
        assert_eq!(actual, direct, "scaled path {format} compositor parity");
    }
    let negative_tie = execute_glyph_library(&args(&[
        "--glyph-stroke", "á", "1000", "1500", "999", "1499",
        "250", "#112233", "--scale-path", "á", "1", "50", "125",
    ])).expect("negative half tie").expect("TSV");
    assert!(String::from_utf8(negative_tie).expect("UTF-8").contains(
        "á\t1000\t1500\t1000\t1499\t250\t#112233\n"
    ));
    fs::remove_file(source).expect("remove owned source");
    fs::remove_file(revision).expect("remove owned revision");
}

#[test]
fn native_selected_path_scaling_refuses_invalid_edits_without_writes() {
    let source = test_path().with_extension("tsv");
    let output = test_path().with_extension("tsv");
    let source_name = source.to_str().expect("source");
    let output_name = output.to_str().expect("output");
    let original = execute_glyph_library(&args(&[
        "--glyph-stroke", "n\u{303}", "500", "700", "3000", "1200",
        "251", "#112233",
        "--glyph-stroke", "ñ", "2200", "600", "2400", "3300",
        "300", "#778899",
    ])).expect("source").expect("TSV");
    fs::write(&source, &original).expect("source file");
    fs::write(&output, b"destination sentinel").expect("destination file");
    for (index, rejected) in [
        args(&["--scale-path"]),
        args(&["--scale-path", "n\u{303}"]),
        args(&["--scale-path", "n\u{303}", "1"]),
        args(&["--scale-path", "n\u{303}", "1", "125"]),
        args(&["--scale-path", "n\u{303}", "0", "125", "75"]),
        args(&["--scale-path", "n\u{303}", "2", "125", "75"]),
        args(&["--scale-path", "n\u{303}", "1", "49", "100"]),
        args(&["--scale-path", "n\u{303}", "1", "100", "151"]),
        args(&["--scale-path", "n\u{303}", "1", "125.5", "75"]),
        args(&["--scale-path", "n\u{303}", "1", "+125", "75"]),
        args(&["--scale-path", "n\u{303}", "1", "100", "100"]),
        args(&["--scale-path", "n\u{303}", "1", "125", "100"]),
        args(&["--scale-path", "ab", "1", "125", "75"]),
        args(&["--scale-path", "n", "1", "125", "75"]),
    ].into_iter().enumerate() {
        let mut edit = args(&["--profile", source_name]);
        edit.extend(rejected.clone());
        edit.extend(args(&["--output", output_name, "--replace"]));
        let result = execute_glyph_library(&edit);
        assert!(result.is_err(), "{index}: unsafe edit accepted");
        assert_eq!(fs::read(&source).expect("source"), original);
        assert_eq!(fs::read(&output).expect("destination"),
            b"destination sentinel");
        let mut piped = args(&["--profile", "-"]);
        piped.extend(rejected);
        let preflight = preflight_glyph_profile_sources(&piped);
        if matches!(index, 5 | 11 | 13) {
            assert!(preflight.is_ok(), "{index}: {preflight:?}");
        } else {
            assert!(preflight.is_err(), "{index}: {preflight:?}");
        }
    }
    let collapsed = execute_glyph_library(&args(&[
        "--glyph-stroke", "á", "1000", "1500", "999", "1500",
        "250", "#112233", "--scale-path", "á", "1", "50", "100",
    ])).expect_err("negative-tie horizontal segment collapses");
    assert!(collapsed.contains("collapses"), "{collapsed}");
    let no_change = execute_glyph_library(&args(&[
        "--glyph-stroke", "á", "1000", "1500", "1001", "1500",
        "250", "#112233", "--scale-path", "á", "1", "101", "100",
    ])).expect_err("rounded one-um edit must refuse no-op");
    assert!(no_change.contains("does not change"), "{no_change}");
    fs::remove_file(source).expect("remove owned source");
    fs::remove_file(output).expect("remove owned destination");
}

#[test]
fn native_selected_path_scaling_preserves_closed_loops_and_full_budget() {
    let loop_bytes = execute_glyph_library(&args(&[
        "--glyph-stroke", "n\u{303}", "500", "700", "900", "700",
        "251", "#112233",
        "--glyph-stroke", "ñ", "1500", "1500", "2000", "2000",
        "251", "#445566",
        "--glyph-stroke", "n\u{303}", "900", "700", "900", "1000",
        "251", "#112233",
        "--glyph-stroke", "n\u{303}", "900", "1000", "500", "1000",
        "251", "#112233",
        "--glyph-stroke", "n\u{303}", "500", "1000", "500", "700",
        "251", "#112233",
        "--scale-path", "n\u{303}", "1", "125", "75",
    ])).expect("closed native path resize").expect("TSV");
    assert_eq!(loop_bytes, concat!(
        "atrament.glyph-strokes.v1\n",
        "n\u{303}\t500\t700\t1000\t700\t251\t#112233\n",
        "ñ\t1500\t1500\t2000\t2000\t251\t#445566\n",
        "n\u{303}\t1000\t700\t1000\t925\t251\t#112233\n",
        "n\u{303}\t1000\t925\t500\t925\t251\t#112233\n",
        "n\u{303}\t500\t925\t500\t700\t251\t#112233\n",
    ).as_bytes());
    let source = test_path().with_extension("tsv");
    let source_name = source.to_str().expect("test library");
    let mut authored = String::from("atrament.glyph-strokes.v1\n");
    for index in 0..4096 {
        let from = 500 + 2 * (index % 2);
        let to = 500 + 2 * ((index + 1) % 2);
        authored.push_str(&format!(
            "n\u{303}\t{from}\t700\t{to}\t700\t251\t#112233\n",
        ));
    }
    fs::write(&source, authored.as_bytes()).expect("own 4096-row file");
    let scaled = execute_glyph_library(&args(&[
        "--profile", source_name,
        "--scale-path", "n\u{303}", "1", "150", "100",
    ])).expect("4096-segment edit").expect("TSV");
    assert_eq!(scaled.split(|byte| *byte == b'\n').count(), 4098);
    assert!(scaled.starts_with(
        b"atrament.glyph-strokes.v1\nn\xcc\x83\t500\t700\t503\t700\t251\t#112233\n"
    ));
    assert_eq!(fs::read(&source).expect("source unchanged"),
        authored.into_bytes());
    let report = String::from_utf8(execute_glyph_library(&args(&[
        "--profile", source_name,
        "--inspect-paths", "n\u{303}",
    ])).expect("full budget inspected").expect("report"))
        .expect("UTF-8");
    assert!(report.contains("Strokes: 4096\nPaths: 1\nPen lifts: 0"),
        "{report}");
    fs::remove_file(source).expect("remove owned full-budget source");
}

#[test]
fn native_selected_path_reflection_preserves_exact_ink_and_exports() {
    let source = test_path().with_extension("tsv");
    let output = test_path().with_extension("tsv");
    let source_name = source.to_str().expect("source path");
    let output_name = output.to_str().expect("output path");
    let nfd = "n\u{303}";
    let original = execute_glyph_library(&args(&[
        "--glyph-stroke", nfd, "500", "700", "900", "1200",
        "251", "#112233",
        "--glyph-stroke", "ñ", "2200", "600", "2200", "3300",
        "300", "#778899",
        "--glyph-stroke", nfd, "900", "1200", "1400", "1900",
        "251", "#112233",
        "--glyph-stroke", "∞", "800", "800", "1700", "1500",
        "200", "#445566",
        "--glyph-stroke", nfd, "2200", "2500", "2700", "3000",
        "200", "#445566",
    ])).expect("authored source").expect("TSV bytes");
    fs::write(&source, &original).expect("save source");
    let command = args(&[
        "--profile", source_name,
        "--reflect-path", nfd, "1", "x", "1750",
    ]);
    let reflected = execute_glyph_library(&command)
        .expect("reflected path").expect("TSV");
    let expected = concat!(
        "atrament.glyph-strokes.v1\n",
        "n\u{303}\t3000\t700\t2600\t1200\t251\t#112233\n",
        "ñ\t2200\t600\t2200\t3300\t300\t#778899\n",
        "n\u{303}\t2600\t1200\t2100\t1900\t251\t#112233\n",
        "∞\t800\t800\t1700\t1500\t200\t#445566\n",
        "n\u{303}\t2200\t2500\t2700\t3000\t200\t#445566\n",
    );
    assert_eq!(reflected, expected.as_bytes());
    let twice = execute_glyph_library(&args(&[
        "--profile", source_name,
        "--reflect-path", nfd, "1", "x", "1750",
        "--reflect-path", nfd, "1", "x", "1750",
    ])).expect("double reflection").expect("original TSV");
    assert_eq!(twice, original);
    let vertical = execute_glyph_library(&args(&[
        "--profile", source_name,
        "--reflect-path", nfd, "1", "y", "2250",
    ])).expect("vertical reflection").expect("TSV");
    assert!(String::from_utf8(vertical).expect("UTF-8").contains(
        "n\u{303}\t500\t3800\t900\t3300\t251\t#112233\n"));
    let report = String::from_utf8(execute_glyph_library(&args(&[
        "--profile", source_name,
        "--reflect-path", nfd, "1", "x", "1750",
        "--inspect-paths", nfd,
    ])).expect("path inspection").expect("UTF-8"))
        .expect("UTF-8");
    assert!(report.contains("Strokes: 3\nPaths: 2\nPen lifts: 1"),
        "{report}");
    let mut save = command;
    save.extend(args(&["--output", output_name]));
    assert!(execute_glyph_library(&save).expect("saved output").is_none());
    assert_eq!(fs::read(&source).expect("source retained"), original);
    assert_eq!(fs::read(&output).expect("saved revision"), reflected);
    for format in ["pdf", "png"] {
        let mut saved = args(&[
            "--export-paper", format, "--printed-sheet", "true",
            "--glyph-file", output_name,
            "--place-text", "1", "30000", "45000", "ñn\u{303}",
        ]);
        let mut direct = args(&[
            "--export-paper", format, "--printed-sheet", "true",
            "--glyph-stroke", "ñ", "2200", "600", "2200", "3300",
            "300", "#778899",
            "--glyph-stroke", nfd, "3000", "700", "2600", "1200",
            "251", "#112233",
            "--glyph-stroke", nfd, "2600", "1200", "2100", "1900",
            "251", "#112233",
            "--glyph-stroke", nfd, "2200", "2500", "2700", "3000",
            "200", "#445566",
            "--place-text", "1", "30000", "45000", "ñn\u{303}",
        ]);
        if format == "png" {
            saved.extend(args(&["--dpi", "72"]));
            direct.extend(args(&["--dpi", "72"]));
        }
        assert_eq!(execute_notebook(&saved)
            .expect("saved ink export").expect("bytes"),
            execute_notebook(&direct)
                .expect("independent physical ink").expect("bytes"),
            "selected path reflection {format} parity");
    }
    fs::remove_file(source).expect("remove owned source");
    fs::remove_file(output).expect("remove owned output");
}

#[test]
fn native_selected_path_reflection_refuses_without_clobbering() {
    for flags in [
        args(&["--profile", "-", "--reflect-path"]),
        args(&["--profile", "-", "--reflect-path", "ñ"]),
        args(&["--profile", "-", "--reflect-path", "ñ", "1"]),
        args(&["--profile", "-", "--reflect-path", "ñ", "1", "x"]),
    ] {
        let error = preflight_glyph_profile_sources(&flags)
            .expect_err("incomplete option refuses before stdin");
        assert!(error.contains("--reflect-path requires 4 operands"),
            "{error}");
    }
    for (key, path, axis, pivot) in [
        ("ñ", "0", "x", "1750"),
        ("ñ", "4097", "x", "1750"),
        ("ab", "1", "x", "1750"),
        ("ñ", "1", "z", "1750"),
        ("ñ", "1", "X", "1750"),
        ("ñ", "1", "x", "-1"),
        ("ñ", "1", "x", "3501"),
        ("ñ", "1", "y", "4501"),
        ("ñ", "1", "x", "1.5"),
    ] {
        let _refused = preflight_glyph_profile_sources(&args(&[
            "--profile", "-", "--reflect-path", key, path, axis, pivot,
        ])).expect_err("bad typed operand refuses before stdin");
    }
    let source = test_path().with_extension("tsv");
    let output = test_path().with_extension("tsv");
    let source_name = source.to_str().expect("source");
    let output_name = output.to_str().expect("output");
    let original = execute_glyph_library(&args(&[
        "--glyph-stroke", "ñ", "500", "700", "900", "1200",
        "251", "#112233",
        "--glyph-stroke", "∞", "800", "800", "1700", "1500",
        "200", "#445566",
    ])).expect("authored source").expect("TSV");
    fs::write(&source, &original).expect("saved source");
    fs::write(&output, b"accepted output sentinel")
        .expect("saved existing target");
    for (glyph, index, axis, pivot, reason) in [
        ("ñ", "1", "x", "0", "clips"),
        ("ñ", "1", "y", "0", "clips"),
        ("ñ", "2", "x", "1750", "path index"),
        ("á", "1", "x", "1750", "no authored strokes"),
    ] {
        let failure = execute_glyph_library(&args(&[
            "--profile", source_name, "--reflect-path",
            glyph, index, axis, pivot,
            "--output", output_name, "--replace",
        ])).expect_err("invalid edit does not overwrite accepted file");
        assert!(failure.contains(reason), "{failure}");
        assert_eq!(fs::read(&source).expect("source unchanged"), original);
        assert_eq!(fs::read(&output).expect("output unchanged"),
            b"accepted output sentinel");
    }
    let centered = execute_glyph_library(&args(&[
        "--glyph-stroke", "ñ", "1750", "700", "1750", "1200",
        "251", "#112233", "--reflect-path", "ñ", "1", "x", "1750",
    ])).expect_err("unchanged path reflection must refuse");
    assert!(centered.contains("does not change"), "{centered}");
    fs::remove_file(source).expect("remove owned source");
    fs::remove_file(output).expect("remove owned output");
}

#[test]
fn native_path_reflection_keeps_closed_loops_and_full_stroke_budget() {
    let nfd = "n\u{303}";
    let original = execute_glyph_library(&args(&[
        "--glyph-stroke", nfd, "500", "700", "900", "700",
        "251", "#112233",
        "--glyph-stroke", "ñ", "1500", "1500", "2000", "2000",
        "251", "#445566",
        "--glyph-stroke", nfd, "900", "700", "900", "1000",
        "251", "#112233",
        "--glyph-stroke", nfd, "900", "1000", "500", "1000",
        "251", "#112233",
        "--glyph-stroke", nfd, "500", "1000", "500", "700",
        "251", "#112233",
    ])).expect("authored loop").expect("source TSV");
    let mirrored = execute_glyph_library(&args(&[
        "--glyph-stroke", nfd, "500", "700", "900", "700",
        "251", "#112233",
        "--glyph-stroke", "ñ", "1500", "1500", "2000", "2000",
        "251", "#445566",
        "--glyph-stroke", nfd, "900", "700", "900", "1000",
        "251", "#112233",
        "--glyph-stroke", nfd, "900", "1000", "500", "1000",
        "251", "#112233",
        "--glyph-stroke", nfd, "500", "1000", "500", "700",
        "251", "#112233",
        "--reflect-path", nfd, "1", "x", "1750",
        "--inspect-paths", nfd,
    ])).expect("mirrored loop inspector").expect("report");
    let report = String::from_utf8(mirrored).expect("UTF-8");
    assert!(report.contains("Strokes: 4\nPaths: 1\nPen lifts: 0"),
        "{report}");
    assert!(report.contains("(3000, 700) -> (3000, 700)"),
        "mirroring must preserve a closed loop: {report}");
    let source = test_path().with_extension("tsv");
    let source_name = source.to_str().expect("source name");
    let mut authored = String::from("atrament.glyph-strokes.v1\n");
    for index in 0..4096 {
        let from = 500 + 2 * (index % 2);
        let to = 500 + 2 * ((index + 1) % 2);
        authored.push_str(&format!(
            "n\u{303}\t{from}\t700\t{to}\t700\t251\t#112233\n",
        ));
    }
    fs::write(&source, authored.as_bytes()).expect("save full library");
    let result = execute_glyph_library(&args(&[
        "--profile", source_name,
        "--reflect-path", nfd, "1", "x", "1750",
    ])).expect("reflect 4096-path").expect("full mirrored TSV");
    assert_eq!(result.iter().filter(|byte| **byte == b'\n').count(),
        4097, "reflection never adds or drops authored segments");
    assert!(result.starts_with(concat!(
        "atrament.glyph-strokes.v1\n",
        "n\u{303}\t3000\t700\t2998\t700\t251\t#112233\n",
    ).as_bytes()));
    let double = execute_glyph_library(&args(&[
        "--profile", source_name,
        "--reflect-path", nfd, "1", "x", "1750",
        "--reflect-path", nfd, "1", "x", "1750",
    ])).expect("reversed full library").expect("source ink");
    assert_eq!(double, authored.as_bytes());
    assert_eq!(fs::read(&source).expect("source preserved"),
        authored.as_bytes());
    assert!(String::from_utf8(original).expect("UTF-8").contains(
        "ñ\t1500\t1500\t2000\t2000\t251\t#445566\n"));
    fs::remove_file(source).expect("remove test fixture");
}

#[test]
fn native_selected_path_clockwise_rotation_keeps_exact_ink_and_exports() {
    let source = test_path().with_extension("tsv");
    let output = test_path().with_extension("tsv");
    let source_name = source.to_str().expect("source path");
    let output_name = output.to_str().expect("output path");
    let nfd = "n\u{303}";
    let original = execute_glyph_library(&args(&[
        "--glyph-stroke", nfd, "500", "700", "900", "1200",
        "251", "#112233",
        "--glyph-stroke", "ñ", "2200", "600", "2200", "3300",
        "300", "#778899",
        "--glyph-stroke", nfd, "900", "1200", "1400", "1900",
        "251", "#112233",
        "--glyph-stroke", "∞", "800", "800", "1700", "1500",
        "200", "#445566",
        "--glyph-stroke", nfd, "2200", "2500", "2700", "3000",
        "200", "#445566",
    ])).expect("authored source").expect("TSV bytes");
    fs::write(&source, &original).expect("save source");
    let command = args(&[
        "--profile", source_name,
        "--rotate-path-ink", nfd, "1", "20", "1750", "2250",
    ]);
    let rotated = execute_glyph_library(&command)
        .expect("rotated path").expect("TSV");
    let expected = concat!(
        "atrament.glyph-strokes.v1\n",
        "n\u{303}\t1106\t366\t1310\t973\t251\t#112233\n",
        "ñ\t2200\t600\t2200\t3300\t300\t#778899\n",
        "n\u{303}\t1310\t973\t1541\t1801\t251\t#112233\n",
        "∞\t800\t800\t1700\t1500\t200\t#445566\n",
        "n\u{303}\t2200\t2500\t2700\t3000\t200\t#445566\n",
    );
    assert_eq!(rotated, expected.as_bytes());
    let inspector = execute_glyph_library(&args(&[
        "--profile", source_name,
        "--rotate-path-ink", nfd, "1", "20", "1750", "2250",
        "--inspect-paths", nfd,
    ])).expect("inspect rotated path").expect("report");
    assert!(String::from_utf8_lossy(&inspector)
        .contains("Strokes: 3\nPaths: 2\nPen lifts: 1"));
    let mut save = command;
    save.extend(args(&["--output", output_name]));
    assert!(execute_glyph_library(&save).expect("write TSV").is_none());
    assert_eq!(fs::read(&source).expect("source unchanged"), original);
    assert_eq!(fs::read(&output).expect("saved TSV"), rotated);
    for format in ["pdf", "png"] {
        let mut saved = args(&[
            "--export-paper", format, "--printed-sheet", "true",
            "--glyph-file", output_name,
            "--place-text", "1", "30000", "45000", "ñn\u{303}",
        ]);
        let mut direct = args(&[
            "--export-paper", format, "--printed-sheet", "true",
            "--glyph-stroke", "ñ", "2200", "600", "2200", "3300",
            "300", "#778899",
            "--glyph-stroke", nfd, "1106", "366", "1310", "973",
            "251", "#112233",
            "--glyph-stroke", nfd, "1310", "973", "1541", "1801",
            "251", "#112233",
            "--glyph-stroke", nfd, "2200", "2500", "2700", "3000",
            "200", "#445566",
            "--place-text", "1", "30000", "45000", "ñn\u{303}",
        ]);
        if format == "png" {
            saved.extend(args(&["--dpi", "72"]));
            direct.extend(args(&["--dpi", "72"]));
        }
        assert_eq!(execute_notebook(&saved)
            .expect("saved native output").expect("bytes"),
            execute_notebook(&direct)
                .expect("independent native ink").expect("bytes"),
            "selected path clockwise rotation {format} parity");
    }
    fs::remove_file(source).expect("remove owned source");
    fs::remove_file(output).expect("remove owned output");
}

#[test]
fn native_selected_path_rotation_refuses_without_overwriting() {
    for flags in [
        args(&["--profile", "-", "--rotate-path-ink"]),
        args(&["--profile", "-", "--rotate-path-ink", "ñ"]),
        args(&["--profile", "-", "--rotate-path-ink", "ñ", "1"]),
        args(&["--profile", "-", "--rotate-path-ink", "ñ", "1", "10"]),
        args(&["--profile", "-", "--rotate-path-ink", "ñ", "1", "10",
            "1750"]),
    ] {
        let error = preflight_glyph_profile_sources(&flags)
            .expect_err("incomplete path rotation before stdin");
        assert!(error.contains("--rotate-path-ink requires 5 operands"),
            "{error}");
    }
    for (glyph, index, angle, x, y) in [
        ("ñ", "0", "10", "1750", "2250"),
        ("ñ", "4097", "10", "1750", "2250"),
        ("ab", "1", "10", "1750", "2250"),
        ("ñ", "1", "0", "1750", "2250"),
        ("ñ", "1", "21", "1750", "2250"),
        ("ñ", "1", "-21", "1750", "2250"),
        ("ñ", "1", "+10", "1750", "2250"),
        ("ñ", "1", "1.5", "1750", "2250"),
        ("ñ", "1", "10", "-1", "2250"),
        ("ñ", "1", "10", "3501", "2250"),
        ("ñ", "1", "10", "1750", "4501"),
        ("ñ", "1", "10", "1750", "1.5"),
    ] {
        assert!(preflight_glyph_profile_sources(&args(&[
            "--profile", "-", "--rotate-path-ink", glyph,
            index, angle, x, y,
        ])).is_err(), "{glyph} {index} {angle} {x} {y}");
    }
    let source = test_path().with_extension("tsv");
    let output = test_path().with_extension("tsv");
    let source_name = source.to_str().expect("source");
    let output_name = output.to_str().expect("target");
    let original = execute_glyph_library(&args(&[
        "--glyph-stroke", "ñ", "500", "700", "900", "1200",
        "251", "#112233",
        "--glyph-stroke", "∞", "800", "800", "1700", "1500",
        "200", "#445566",
    ])).expect("source").expect("ink");
    fs::write(&source, &original).expect("save source");
    fs::write(&output, b"approved TSV sentinel").expect("save sentinel");
    for (key, index, angle, x, y, reason) in [
        ("ñ", "1", "20", "3500", "4500", "clips"),
        ("ñ", "2", "10", "1750", "2250", "path index"),
        ("á", "1", "10", "1750", "2250", "no authored strokes"),
    ] {
        let error = execute_glyph_library(&args(&[
            "--profile", source_name, "--rotate-path-ink",
            key, index, angle, x, y,
            "--output", output_name, "--replace",
        ])).expect_err("bad rotation must refuse without file changes");
        assert!(error.contains(reason), "{error}");
        assert_eq!(fs::read(&source).expect("unchanged source"), original);
        assert_eq!(fs::read(&output).expect("unchanged output"),
            b"approved TSV sentinel");
    }
    let unchanged = execute_glyph_library(&args(&[
        "--glyph-stroke", "ñ", "1750", "2250", "1751", "2250",
        "80", "#112233",
        "--rotate-path-ink", "ñ", "1", "1", "1750", "2250",
    ])).expect_err("rounded no-op refuses");
    assert!(unchanged.contains("does not change"), "{unchanged}");
    assert_eq!(fs::read(&source).expect("source still unchanged"), original);
    assert_eq!(fs::read(&output).expect("sentinel still unchanged"),
        b"approved TSV sentinel");
    fs::remove_file(source).expect("remove owned source");
    fs::remove_file(output).expect("remove owned output");
}

#[test]
fn native_selected_path_rotation_keeps_closed_and_full_budget_ink() {
    let nfd = "n\u{303}";
    let loop_rows = args(&[
        "--glyph-stroke", nfd, "1000", "1000", "1400", "1000",
        "200", "#112233",
        "--glyph-stroke", "ñ", "1000", "1000", "1100", "1200",
        "200", "#445566",
        "--glyph-stroke", nfd, "1400", "1000", "1400", "1400",
        "200", "#112233",
        "--glyph-stroke", nfd, "1400", "1400", "1000", "1400",
        "200", "#112233",
        "--glyph-stroke", nfd, "1000", "1400", "1000", "1000",
        "200", "#112233",
        "--rotate-path-ink", nfd, "1", "15", "1750", "2250",
        "--inspect-paths", nfd,
    ]);
    let inspected = execute_glyph_library(&loop_rows)
        .expect("rotated closed path").expect("inspector");
    let report = String::from_utf8(inspected).expect("UTF-8");
    assert!(report.contains("Strokes: 4\nPaths: 1\nPen lifts: 0"),
        "{report}");
    assert!(report.contains("(1349, 848) -> (1349, 848)"),
        "closed path remains closed: {report}");
    let source = test_path().with_extension("tsv");
    let source_name = source.to_str().expect("source name");
    let mut authored = String::from("atrament.glyph-strokes.v1\n");
    for index in 0..4096 {
        let from = 1000 + 2 * (index % 2);
        let to = 1000 + 2 * ((index + 1) % 2);
        authored.push_str(&format!(
            "n\u{303}\t{from}\t1000\t{to}\t1000\t200\t#112233\n",
        ));
    }
    fs::write(&source, authored.as_bytes()).expect("save full library");
    let edited = execute_glyph_library(&args(&[
        "--profile", source_name,
        "--rotate-path-ink", nfd, "1", "20", "1750", "2250",
    ])).expect("rotate full path").expect("TSV");
    assert_eq!(edited.iter().filter(|byte| **byte == b'\n').count(), 4097);
    let inspected_full = execute_glyph_library(&args(&[
        "--profile", source_name,
        "--rotate-path-ink", nfd, "1", "20", "1750", "2250",
        "--inspect-paths", nfd,
    ])).expect("inspector").expect("report");
    assert!(String::from_utf8_lossy(&inspected_full)
        .contains("Strokes: 4096\nPaths: 1\nPen lifts: 0"));
    assert_eq!(fs::read(&source).expect("unmodified source"),
        authored.as_bytes());
    fs::remove_file(source).expect("remove owned source");
}

#[test]
fn native_selected_path_shear_preserves_exact_unicode_and_exported_ink() {
    let nfd = "n\u{303}";
    let source = test_path().with_extension("tsv");
    let output = test_path().with_extension("tsv");
    let source_name = source.to_str().expect("source");
    let output_name = output.to_str().expect("target");
    let original = execute_glyph_library(&args(&[
        "--glyph-stroke", nfd, "800", "800", "1100", "1400",
        "251", "#112233",
        "--glyph-stroke", "ñ", "2200", "600", "2200", "3300",
        "300", "#778899",
        "--glyph-stroke", nfd, "1100", "1400", "1400", "2000",
        "251", "#112233",
        "--glyph-stroke", "∞", "800", "800", "1700", "1500",
        "200", "#445566",
        "--glyph-stroke", nfd, "2200", "2500", "2700", "3000",
        "200", "#445566",
    ])).expect("source accepted").expect("source TSV");
    fs::write(&source, &original).expect("save source");
    let edited = execute_glyph_library(&args(&[
        "--profile", source_name, "--shear-path", nfd, "1", "20", "2600",
    ])).expect("bounded slant accepted").expect("TSV");
    let expected = concat!(
        "atrament.glyph-strokes.v1\n",
        "n\u{303}\t1455\t800\t1537\t1400\t251\t#112233\n",
        "ñ\t2200\t600\t2200\t3300\t300\t#778899\n",
        "n\u{303}\t1537\t1400\t1618\t2000\t251\t#112233\n",
        "∞\t800\t800\t1700\t1500\t200\t#445566\n",
        "n\u{303}\t2200\t2500\t2700\t3000\t200\t#445566\n",
    );
    assert_eq!(edited, expected.as_bytes());
    let report = execute_glyph_library(&args(&[
        "--profile", source_name,
        "--shear-path", nfd, "1", "20", "2600",
        "--inspect-paths", nfd,
    ])).expect("inspected").expect("report");
    assert!(String::from_utf8_lossy(&report)
        .contains("Strokes: 3\nPaths: 2\nPen lifts: 1"));
    assert!(execute_glyph_library(&args(&[
        "--profile", source_name, "--shear-path", nfd, "1", "20", "2600",
        "--output", output_name,
    ])).expect("save explicit target").is_none());
    assert_eq!(fs::read(&source).expect("source unchanged"), original);
    assert_eq!(fs::read(&output).expect("saved edited TSV"), edited);
    for format in ["pdf", "png"] {
        let mut saved = args(&[
            "--export-paper", format, "--printed-sheet", "true",
            "--glyph-file", output_name,
            "--place-text", "1", "30000", "45000", "ñn\u{303}",
        ]);
        let mut direct = args(&[
            "--export-paper", format, "--printed-sheet", "true",
            "--glyph-stroke", nfd, "1455", "800", "1537", "1400",
            "251", "#112233",
            "--glyph-stroke", "ñ", "2200", "600", "2200", "3300",
            "300", "#778899",
            "--glyph-stroke", nfd, "1537", "1400", "1618", "2000",
            "251", "#112233",
            "--glyph-stroke", "∞", "800", "800", "1700", "1500",
            "200", "#445566",
            "--glyph-stroke", nfd, "2200", "2500", "2700", "3000",
            "200", "#445566",
            "--place-text", "1", "30000", "45000", "ñn\u{303}",
        ]);
        if format == "png" {
            saved.extend(args(&["--dpi", "72"]));
            direct.extend(args(&["--dpi", "72"]));
        }
        let expected_page = execute_notebook(&saved)
            .expect("saved page").expect("bytes");
        assert_eq!(expected_page,
            execute_notebook(&direct).expect("independent ink").expect("bytes"),
            "saved selected-path slant differs in {format}");
    }
    fs::remove_file(source).expect("remove owned source");
    fs::remove_file(output).expect("remove owned output");
}

#[test]
fn native_selected_path_shear_preflights_and_refuses_without_clobber() {
    for incomplete in [
        args(&["--profile", "-", "--shear-path"]),
        args(&["--profile", "-", "--shear-path", "ñ"]),
        args(&["--profile", "-", "--shear-path", "ñ", "1"]),
        args(&["--profile", "-", "--shear-path", "ñ", "1", "10"]),
    ] {
        let error = preflight_glyph_profile_sources(&incomplete)
            .expect_err("incomplete slant before reading stdin");
        assert!(error.contains("--shear-path requires 4 operands"), "{error}");
    }
    for (glyph, index, angle, baseline) in [
        ("ñ", "0", "10", "2600"),
        ("ñ", "4097", "10", "2600"),
        ("ab", "1", "10", "2600"),
        ("ñ", "1", "0", "2600"),
        ("ñ", "1", "21", "2600"),
        ("ñ", "1", "-21", "2600"),
        ("ñ", "1", "+10", "2600"),
        ("ñ", "1", "1.5", "2600"),
        ("ñ", "1", "10", "-1"),
        ("ñ", "1", "10", "4501"),
        ("ñ", "1", "10", "1.5"),
    ] {
        assert!(preflight_glyph_profile_sources(&args(&[
            "--profile", "-", "--shear-path",
            glyph, index, angle, baseline,
        ])).is_err(), "{glyph} {index} {angle} {baseline}");
    }
    let source = test_path().with_extension("tsv");
    let output = test_path().with_extension("tsv");
    let source_name = source.to_str().expect("source");
    let output_name = output.to_str().expect("output");
    let original = execute_glyph_library(&args(&[
        "--glyph-stroke", "ñ", "800", "800", "1100", "1400",
        "251", "#112233",
        "--glyph-stroke", "∞", "800", "800", "1700", "1500",
        "200", "#445566",
    ])).expect("source").expect("TSV");
    fs::write(&source, &original).expect("source");
    fs::write(&output, b"approved slant sentinel").expect("sentinel");
    for (glyph, index, angle, baseline, reason) in [
        ("ñ", "1", "-20", "4500", "clip"),
        ("ñ", "2", "10", "2600", "path index"),
        ("á", "1", "10", "2600", "no authored strokes"),
    ] {
        let error = execute_glyph_library(&args(&[
            "--profile", source_name, "--shear-path",
            glyph, index, angle, baseline,
            "--output", output_name, "--replace",
        ])).expect_err("bad slant cannot overwrite output");
        assert!(error.contains(reason), "{error}");
        assert_eq!(fs::read(&source).expect("source"), original);
        assert_eq!(fs::read(&output).expect("output"),
            b"approved slant sentinel");
    }
    let unchanged = execute_glyph_library(&args(&[
        "--glyph-stroke", "ñ", "1000", "2000", "1500", "2000",
        "200", "#112233",
        "--shear-path", "ñ", "1", "-10", "2000",
    ])).expect_err("baseline-aligned horizontal stroke unchanged");
    assert!(unchanged.contains("does not change"), "{unchanged}");
    fs::remove_file(source).expect("remove owned source");
    fs::remove_file(output).expect("remove owned output");
}

#[test]
fn native_selected_path_shear_preserves_closed_and_full_budget_paths() {
    let key = "n\u{303}";
    let loop_report = execute_glyph_library(&args(&[
        "--glyph-stroke", key, "1000", "1000", "1400", "1000",
        "200", "#112233",
        "--glyph-stroke", "ñ", "1200", "1400", "1200", "1800",
        "200", "#445566",
        "--glyph-stroke", key, "1400", "1000", "1400", "1400",
        "200", "#112233",
        "--glyph-stroke", key, "1400", "1400", "1000", "1400",
        "200", "#112233",
        "--glyph-stroke", key, "1000", "1400", "1000", "1000",
        "200", "#112233",
        "--shear-path", key, "1", "-15", "2000",
        "--inspect-paths", key,
    ])).expect("closed loop edit").expect("report");
    let report = String::from_utf8(loop_report).expect("UTF-8");
    assert!(report.contains("Strokes: 4\nPaths: 1\nPen lifts: 0"),
        "{report}");
    assert!(report.contains("(732, 1000) -> (732, 1000)")
        && report.contains("[closed loop]"),
        "slant preserves closed path endpoints: {report}");
    let source = test_path().with_extension("tsv");
    let source_name = source.to_str().expect("source");
    let mut authored = String::from("atrament.glyph-strokes.v1\n");
    for index in 0..4096 {
        let from = 1000 + 2 * (index % 2);
        let to = 1000 + 2 * ((index + 1) % 2);
        authored.push_str(&format!(
            "n\u{303}\t{from}\t1000\t{to}\t1000\t200\t#112233\n",
        ));
    }
    fs::write(&source, authored.as_bytes()).expect("full source");
    let output = execute_glyph_library(&args(&[
        "--profile", source_name,
        "--shear-path", key, "1", "20", "2000",
    ])).expect("slanted 4096 stroke path").expect("TSV");
    assert_eq!(output.iter().filter(|byte| **byte == b'\n').count(),
        4097);
    assert_eq!(fs::read(&source).expect("saved original"),
        authored.as_bytes());
    fs::remove_file(source).expect("owned source");
}

#[test]
fn native_curved_closure_replays_saved_unicode_ink_and_pdf_png() {
    let nfd = "n\u{303}";
    let source = test_path().with_extension("tsv");
    let saved = test_path().with_extension("tsv");
    let source_name = source.to_str().expect("source path");
    let saved_name = saved.to_str().expect("saved path");
    let original = execute_glyph_library(&args(&[
        "--glyph-stroke", nfd, "500", "700", "850", "750",
        "250", "#243647",
        "--glyph-stroke", "ñ", "1200", "1200", "1250", "1350",
        "250", "#243647",
        "--glyph-stroke", nfd, "850", "750", "1100", "850",
        "250", "#243647",
    ])).expect("original library").expect("TSV bytes");
    fs::write(&source, &original).expect("persist original");
    let result = execute_glyph_library(&args(&[
        "--profile", source_name, "--close-path-curved",
        nfd, "1", "0", "-200", "--output", saved_name,
    ])).expect("save curved closure");
    assert!(result.is_none(), "explicit --output writes no stdout");
    let edited = fs::read(&saved).expect("edited TSV");
    let expected = [
        "atrament.glyph-strokes.v1".to_owned(),
        format!("{nfd}\t500\t700\t850\t750\t250\t#243647"),
        "ñ\t1200\t1200\t1250\t1350\t250\t#243647".to_owned(),
        format!("{nfd}\t850\t750\t1100\t850\t250\t#243647"),
        format!("{nfd}\t1100\t850\t950\t738\t250\t#243647"),
        format!("{nfd}\t950\t738\t800\t675\t250\t#243647"),
        format!("{nfd}\t800\t675\t650\t663\t250\t#243647"),
        format!("{nfd}\t650\t663\t500\t700\t250\t#243647"),
        String::new(),
    ].join("\n");
    assert_eq!(edited, expected.as_bytes());
    assert_eq!(fs::read(&source).expect("source unchanged"), original);
    let inspected = String::from_utf8(execute_glyph_library(&args(&[
        "--profile", saved_name, "--inspect-paths", nfd,
    ])).expect("inspect saved library").expect("report"))
        .expect("UTF-8 report");
    assert!(inspected.contains("Strokes: 6\nPaths: 1\nPen lifts: 0"),
        "{inspected}");
    assert!(inspected.contains("[closed loop]"), "{inspected}");
    for format in ["pdf", "png"] {
        let mut imported = args(&[
            "--export-paper", format, "--printed-sheet", "true",
            "--glyph-file", saved_name,
            "--place-text", "1", "30000", "45000", nfd,
        ]);
        let mut inline = args(&[
            "--export-paper", format, "--printed-sheet", "true",
            "--glyph-stroke", nfd, "500", "700", "850", "750",
            "250", "#243647",
            "--glyph-stroke", nfd, "850", "750", "1100", "850",
            "250", "#243647",
            "--glyph-stroke", nfd, "1100", "850", "950", "738",
            "250", "#243647",
            "--glyph-stroke", nfd, "950", "738", "800", "675",
            "250", "#243647",
            "--glyph-stroke", nfd, "800", "675", "650", "663",
            "250", "#243647",
            "--glyph-stroke", nfd, "650", "663", "500", "700",
            "250", "#243647",
            "--place-text", "1", "30000", "45000", nfd,
        ]);
        if format == "png" {
            imported.extend(args(&["--dpi", "72"]));
            inline.extend(args(&["--dpi", "72"]));
        }
        assert_eq!(execute_notebook(&imported).expect("imported export"),
            execute_notebook(&inline).expect("inline export"),
            "curved closed glyph must render identical native {format}");
    }
    let failures = [
        args(&["--close-path-curved", nfd, "1", "0", "0"]),
        args(&["--close-path-curved", nfd, "1", "0", "601"]),
        args(&["--close-path-curved", nfd, "1", "-601", "0"]),
        args(&["--close-path-curved", nfd, "1", "-400", "-100"]),
        args(&["--close-path-curved", nfd, "0", "0", "-200"]),
        args(&["--close-path-curved", "ñ", "1", "0", "-200"]),
        args(&["--close-path-curved", "nn", "1", "0", "-200"]),
        args(&["--close-path-curved", nfd, "1", "0", "-200",
            "--close-path-curved", nfd, "1", "0", "-200"]),
    ];
    for failure in failures {
        let mut command = args(&["--profile", source_name]);
        command.extend(failure);
        command.extend(args(&["--output", saved_name, "--replace"]));
        assert!(execute_glyph_library(&command).is_err(),
            "invalid curved closure must refuse without changing saved ink");
        assert_eq!(fs::read(&saved).expect("saved stays intact"), edited);
    }
    fs::remove_file(source).expect("remove source");
    fs::remove_file(saved).expect("remove edited file");
}

#[test]
fn native_curved_closure_caps_four_segments_without_partial_edit() {
    let nfd = "n\u{303}";
    let base = args(&[
        "--glyph-stroke", nfd, "500", "700", "850", "750",
        "250", "#243647",
        "--glyph-stroke", nfd, "850", "750", "1100", "850",
        "250", "#243647",
    ]);
    for (count, admitted) in [(4090usize, true), (4091usize, false)] {
        let mut source = base.clone();
        for _ in 0..count {
            source.extend(args(&[
                "--glyph-stroke", "ñ", "1200", "1200", "1250", "1350",
                "250", "#243647",
            ]));
        }
        source.extend(args(&[
            "--close-path-curved", nfd, "1", "0", "-200",
        ]));
        match execute_glyph_library(&source) {
            Ok(Some(bytes)) if admitted => {
                let lines = String::from_utf8(bytes).expect("UTF-8 library");
                assert_eq!(lines.lines().count(), 4097,
                    "header plus full 4096 authored stroke budget");
            }
            Err(error) if !admitted => {
                assert!(error.contains("4096 authored strokes"), "{error}");
            }
            actual => panic!("unexpected capacity result: {}",
                if actual.is_ok() { "accepted" } else { "rejected" }),
        }
    }
}

#[test]
fn notebook_ruler_cli_composes_real_authored_flow_over_varied_pdf_png() {
    let common = args(&[
        "--printed-sheet", "false", "--page-numbers", "true",
        "--glyph-stroke", "a", "500", "500", "2600", "2800",
        "250", "#345678",
        "--flow-text", "aaa\naaa",
        "--ink-line", "2", "31000", "46000", "34000", "47000",
        "250", "#EE1200",
    ]);
    let variation = args(&[
        "--ruler-seed", "2026", "--ruler-error-mm", "0.150",
    ]);
    let other = args(&[
        "--ruler-seed", "2027", "--ruler-error-mm", "0.150",
    ]);
    for format in ["pdf", "png"] {
        let mut base = common.clone();
        base.extend(args(&["--export-paper", format]));
        if format == "pdf" {
            base.extend(args(&["--pages", "2"]));
        } else {
            base.extend(args(&["--dpi", "72", "--png-page", "2"]));
        }
        let original = execute_notebook(&base)
            .expect("default native flow export").expect("bytes");
        let mut chosen = base.clone();
        chosen.extend(variation.clone());
        let rendered = execute_notebook(&chosen)
            .expect("varied native flow export").expect("bytes");
        let expected_prefix = if format == "pdf" {
            b"%PDF-1.4".as_slice()
        } else {
            b"\x89PNG\r\n\x1a\n".as_slice()
        };
        assert!(rendered.starts_with(expected_prefix));
        assert_ne!(rendered, original, "{format} varied physical ruling");
        assert_eq!(rendered, execute_notebook(&chosen)
            .expect("repeat same seed").expect("identical output"));
        let mut changed = base;
        changed.extend(other.clone());
        assert_ne!(rendered, execute_notebook(&changed)
            .expect("distinct seed accepted").expect("bytes"));
        if format == "pdf" {
            assert!(String::from_utf8_lossy(&rendered)
                .contains("/Count 2"));
        }
    }
}

#[test]
fn notebook_ruler_flags_preflight_before_external_sources() {
    for (flags, diagnostic) in [
        (vec!["--ruler-seed", "1"], "must be supplied together"),
        (vec!["--ruler-error-mm", "0.100"],
            "must be supplied together"),
        (vec!["--ruler-seed", "-1", "--ruler-error-mm", "0.1"],
            "ruler seed needs an unsigned"),
        (vec!["--ruler-seed", "4294967296", "--ruler-error-mm",
            "0.1"], "ruler seed exceeds"),
        (vec!["--ruler-seed", "1", "--ruler-error-mm", "0.251"],
            "ruler error needs"),
        (vec!["--ruler-seed", "1", "--ruler-error-mm", "0.0001"],
            "millimeters require"),
        (vec!["--ruler-seed", "1", "--ruler-seed", "2",
            "--ruler-error-mm", "0.1"], "--ruler-seed may appear"),
        (vec!["--ruler-error-mm", "0.1", "--ruler-error-mm",
            "0.2", "--ruler-seed", "1"],
            "--ruler-error-mm may appear"),
        (vec!["--ruler-seed", "1", "--ruler-error-mm", "0.1",
            "--emit-ink-file"], "requires --export-paper"),
    ] {
        let mut arguments = args(&[
            "--profile", "missing-selected-notebook-ruler.json",
        ]);
        arguments.extend(flags.into_iter().map(str::to_owned));
        let error = execute_notebook(&arguments)
            .expect_err("invalid ruler flags before selected input read");
        assert!(error.contains(diagnostic), "{error:?}");
    }
    let missing = execute_notebook(&args(&[
        "--export-paper", "png", "--ruler-seed",
    ])).expect_err("missing flag value refuses");
    assert!(missing.contains("--ruler-seed requires a decimal value"));
}

#[test]
fn notebook_ruler_refusal_preserves_selected_pdf_and_png() {
    for format in ["pdf", "png"] {
        let output = test_path().with_extension(format);
        let name = output.to_str().expect("test-owned target path");
        fs::write(&output, b"preserve previously accepted page")
            .expect("selected output sentinel");
        let mut command = args(&[
            "--printed-sheet", "true", "--export-paper", format,
            "--ruler-seed", "2026", "--ruler-error-mm", "0.150",
            "--output", name, "--replace",
        ]);
        if format == "png" {
            command.extend(args(&["--dpi", "72"]));
        }
        let error = execute_notebook(&command)
            .expect_err("preprinted paper cannot have sampled ruling");
        assert!(error.contains("ruler variation requires printed paper"),
            "{error}");
        assert_eq!(fs::read(&output).expect("unchanged bytes"),
            b"preserve previously accepted page");
        fs::remove_file(output).expect("remove test-owned target");
    }
}

#[test]
fn notebook_ruler_saved_profile_cli_matches_typed_pdf_and_selected_png() {
    let profile = NotebookProfile::default();
    let source = profile.canonical_json().expect("saved notebook metadata");
    let input = test_path();
    let name = input.to_str().expect("test-owned saved profile path");
    fs::write(&input, source.as_bytes()).expect("save canonical source");
    let sample = RulerVariation::from_decimal_mm("2026", "0.075")
        .expect("bounded ruler input");
    let ink = [NotebookInkLine {
        page: 2,
        start_um: (30_000, 40_000),
        end_um: (36_000, 46_000),
        width_um: 250,
        rgb: [52, 86, 120],
    }];
    for format in ["pdf", "png"] {
        let mut flags = args(&[
            "--profile", name, "--export-paper", format,
            "--ruler-seed", "2026", "--ruler-error-mm", "0.075",
            "--ink-line", "2", "30000", "40000", "36000",
            "46000", "250", "#345678",
        ]);
        let expected = if format == "pdf" {
            flags.extend(args(&["--pages", "2"]));
            profile.paper_layer_pdf_with_ink_ruler(2, &ink, Some(sample))
                .expect("typed two-page PDF")
        } else {
            flags.extend(args(&["--png-page", "2", "--dpi", "72"]));
            profile.paper_layer_png_page_with_ink_ruler(
                72, 2, &ink, Some(sample),
            ).expect("typed second-page PNG")
        };
        let actual = execute_notebook(&flags)
            .expect("native saved source admitted").expect("export bytes");
        assert_eq!(actual, expected, "{format} exact native/typed parity");
        assert_eq!(actual, execute_notebook(&flags)
            .expect("same seed replay").expect("export bytes"));
        assert_eq!(fs::read(&input).expect("read source untouched"),
            source.as_bytes());
    }
    fs::remove_file(input).expect("remove test-owned saved source");
}

#[test]
fn notebook_inspect_flow_reports_nominal_pages_without_glyph_strokes() {
    let report = execute_notebook(&args(&[
        "--inspect-flow", "--flow-region", "30000", "40000",
        "32000", "36000", "--flow-text", "abcdefg",
    ])).expect("admitted inspection").expect("stdout report");
    assert_eq!(String::from_utf8(report).expect("UTF-8 report"), concat!(
        "Nominal flow (glyph coverage and ink clearance unchecked)\n",
        "Pages: 1\nRows: 2\nOrnaments: 0\n",
        "Page 1 x=32500 um y=42500 um: \"abcdef\"\n",
        "Page 1 x=32500 um y=51500 um: \"g\"\n",
    ));

    // Heading, collapsed blank and synthetic bullet remain measurable
    // without any authored glyph library, but the report has no ink.
    let markdown = execute_notebook(&args(&[
        "--inspect-flow", "--flow-markdown", "# a\n\n- a",
    ])).expect("nominal Markdown").expect("stdout report");
    let markdown = String::from_utf8(markdown).expect("UTF-8 report");
    assert!(markdown.contains("Pages: 1\nRows: 2\nOrnaments: 2\n"),
        "{markdown}");
    assert!(markdown.contains("Page 1 x=22500 um y=24500 um: \"a\""),
        "{markdown}");
    assert!(markdown.contains("Page 1 x=28500 um y=37500 um: \"a\""),
        "{markdown}");
    let profile = execute_notebook(&args(&["--inspect"]))
        .expect("metadata inspect unchanged").expect("profile report");
    assert!(String::from_utf8_lossy(&profile)
        .starts_with("Version: atrament.notebook.v1\n"));
}

#[test]
fn notebook_inspect_flow_preflights_before_selected_file_io() {
    for (extra, expected) in [
        (vec!["--inspect-flow"], "--inspect-flow requires"),
        (vec!["--inspect-flow", "--export-paper", "pdf"],
            "--inspect-flow is stdout only"),
        (vec!["--inspect-flow", "--output", "unused.pdf"],
            "--inspect-flow is stdout only"),
        (vec!["--inspect-flow", "--inspect"],
            "--inspect-flow is stdout only"),
        (vec!["--inspect-flow", "--glyph-file", "missing.tsv"],
            "--inspect-flow cannot include authored ink"),
        (vec!["--inspect-flow", "--glyph-stroke", "a", "0", "0",
            "1000", "1000", "250", "#123456"],
            "--inspect-flow cannot include authored ink"),
        (vec!["--inspect-flow", "--ink-line", "1", "30000",
            "40000", "33000", "43000", "250", "#123456"],
            "--inspect-flow cannot include authored ink"),
        (vec!["--inspect-flow", "--inspect-flow"],
            "--inspect-flow may appear only once"),
    ] {
        let mut command = args(&[
            "--profile", "missing-notebook-inspection-file.json",
        ]);
        if extra != vec!["--inspect-flow"] {
            command.extend(args(&["--flow-text", "a"]));
        }
        command.extend(extra.into_iter().map(str::to_owned));
        let error = execute_notebook(&command)
            .expect_err("invalid inspection preflights source IO");
        assert!(error.contains(expected), "{error:?}");
    }
    let error = execute_notebook(&args(&[
        "--flow-text", "a",
    ])).expect_err("plain source still cannot be saved as profile");
    assert!(error.contains("flow source needs --export-paper"),
        "{error}");
}

#[test]
fn notebook_inspect_flow_uses_selected_statistics_without_ink() {
    let profile = execute_handwriting(&args(&[
        "--set-global", "width", "8000", "8000", "8000", "8000", "0",
    ])).expect("valid statistics").expect("canonical JSON");
    let input = test_path();
    let selected = input.to_str().expect("test-owned profile path");
    fs::write(&input, &profile).expect("write test-owned statistics");
    let report = execute_notebook(&args(&[
        "--inspect-flow", "--handwriting-profile", selected,
        "--flow-text", "abcdefg", "--flow-region", "30000",
        "40000", "32000", "36000",
    ])).expect("measured statistics plan").expect("stdout report");
    let report = String::from_utf8(report).expect("UTF-8 report");
    assert!(report.contains("Pages: 1\nRows: 3\nOrnaments: 0\n"),
        "{report}");
    for (y, text) in [(42500, "abc"), (51500, "def"), (60500, "g")] {
        assert!(report.contains(&format!(
            "Page 1 x=32500 um y={y} um: {text:?}\n",
        )), "{report}");
    }
    assert_eq!(fs::read(&input).expect("read selected profile"), profile);
    fs::remove_file(input).expect("remove test-owned statistics");
}

#[test]
fn notebook_inspect_flow_keeps_unicode_identity_across_physical_pages() {
    let source = "\u{feff}ó\r\nn\u{303}\r\n…";
    let input = test_path().with_extension("txt");
    let selected = input.to_str().expect("test-owned text path");
    fs::write(&input, source.as_bytes()).expect("write exact source text");
    let report = execute_notebook(&args(&[
        "--inspect-flow", "--flow-text-file", selected,
        "--flow-region", "30000", "40000", "32000", "25000",
    ])).expect("bounded file inspection").expect("stdout report");
    assert_eq!(String::from_utf8(report).expect("UTF-8 report"), concat!(
        "Nominal flow (glyph coverage and ink clearance unchecked)\n",
        "Pages: 2\nRows: 3\nOrnaments: 0\n",
        "Page 1 x=32500 um y=42500 um: \"ó\"\n",
        "Page 1 x=32500 um y=51500 um: \"n\\u{303}\"\n",
        "Page 2 x=32500 um y=42500 um: \"…\"\n",
    ));
    assert_eq!(fs::read(&input).expect("source unchanged"),
        source.as_bytes());
    fs::remove_file(input).expect("remove test-owned input");
}

#[test]
fn notebook_inspect_flow_inventory_counts_only_authored_visible_graphemes()
-> Result<(), String> {
    let source = "# a\n\n- a\n> ó n\u{303}\n1. a";
    let report = execute_notebook(&args(&[
        "--inspect-flow", "--inspect-flow-inventory",
        "--flow-markdown", source,
    ]))?.ok_or("missing read-only inventory report")?;
    let report = String::from_utf8(report)
        .map_err(|error| error.to_string())?;
    assert!(report.contains("Pages: 1\nRows: 4\n"), "{report}");
    let entries = report.split("Glyph inventory: ").nth(1)
        .ok_or("missing inventory section")?;
    assert_eq!(entries, concat!(
        "5 exact forms, 7 uses\n",
        "\".\": 1\n",
        "\"1\": 1\n",
        "\"a\": 3\n",
        "\"n\\u{303}\": 1\n",
        "\"ó\": 1\n",
    ));
    assert!(!entries.contains("\"#\""));
    assert!(!entries.contains("\"-\""));
    assert!(!entries.contains("\">\""));

    // The opt-in report must not change the initial inspection contract.
    let original = execute_notebook(&args(&[
        "--inspect-flow", "--flow-markdown", source,
    ]))?.ok_or("missing ordinary inspection")?;
    assert_eq!(report.split("Glyph inventory: ").next()
        .ok_or("missing report prefix")?.as_bytes(), original);
    Ok(())
}

#[test]
fn notebook_inspect_flow_inventory_preserves_exact_unicode_and_physical_wraps()
-> Result<(), String> {
    // A literal no-break space is not an implicit ASCII gap,
    // while grapheme-level NFD and NFC keys remain distinct across sheets.
    let source = "a\u{a0}ó\no\u{301}\nó";
    let report = execute_notebook(&args(&[
        "--inspect-flow-inventory", "--inspect-flow", "--flow-text",
        source, "--flow-region", "30000", "40000", "32000", "25000",
    ]))?.ok_or("missing Unicode inventory")?;
    let report = String::from_utf8(report)
        .map_err(|error| error.to_string())?;
    assert!(report.contains("Pages: 2\nRows: 3\n"), "{report}");
    let entries = report.split("Glyph inventory: ").nth(1)
        .ok_or("missing inventory section")?;
    assert!(entries.starts_with("4 exact forms, 5 uses\n"), "{entries}");
    for item in ["\"a\": 1", "\"o\\u{301}\": 1", "\"ó\": 2"] {
        assert!(entries.contains(item), "missing {item}: {entries}");
    }
    assert!(entries.contains("\\u{a0}"), "{entries}");
    Ok(())
}

#[test]
fn notebook_inspect_flow_inventory_refuses_conflicts_before_source_reads() {
    for (extra, expected) in [
        (vec!["--inspect-flow-inventory"],
            "--inspect-flow-inventory requires --inspect-flow"),
        (vec!["--inspect-flow-inventory", "--inspect-flow-inventory"],
            "--inspect-flow-inventory may appear only once"),
        (vec!["--inspect-flow-inventory", "--inspect-flow",
            "--output", "unwanted.tsv"],
            "--inspect-flow is stdout only"),
        (vec!["--inspect-flow-inventory", "--inspect-flow",
            "--glyph-file", "missing-glyphs.tsv"],
            "--inspect-flow cannot include authored ink"),
    ] {
        let mut command = args(&[
            "--profile", "missing-inspect-inventory-notebook.json",
            "--flow-text", "a",
        ]);
        command.extend(extra.into_iter().map(str::to_owned));
        let error = execute_notebook(&command)
            .expect_err("inventory conflict must refuse before source IO");
        assert!(error.contains(expected), "{error:?}");
    }
}

#[test]
fn notebook_inspect_flow_inventory_preserves_literal_fence_ink()
-> Result<(), String> {
    let source = "# a\n```txt\n# a\n```\n";
    let report = execute_notebook(&args(&[
        "--inspect-flow", "--inspect-flow-inventory",
        "--flow-markdown", source,
    ]))?.ok_or("missing read-only fenced-code inventory")?;
    let report = String::from_utf8(report)
        .map_err(|error| error.to_string())?;
    assert!(report.contains("Pages: 1\nRows: 2\nOrnaments: 1\n"),
        "{report}");
    let entries = report.split("Glyph inventory: ").nth(1)
        .ok_or("missing inventory section")?;
    assert_eq!(entries, concat!(
        "2 exact forms, 3 uses\n",
        "\"#\": 1\n",
        "\"a\": 2\n",
    ));
    // The fence itself and its metadata are syntax, but the identical
    // hash inside literal content is an authored pen-shape obligation.
    assert!(!entries.contains("\"`\""), "{entries}");
    assert!(!entries.contains("\"t\""), "{entries}");
    Ok(())
}

#[test]
fn notebook_inspect_flow_page_inventory_reports_exact_sheet_membership()
-> Result<(), String> {
    let source = "a\u{a0}ó\r\nn\u{303}\r\nó\r\na";
    let flags = args(&[
        "--inspect-flow", "--inspect-flow-inventory",
        "--inspect-flow-page-inventory", "--flow-text", source,
        "--flow-region", "30000", "40000", "32000", "25000",
    ]);
    let report = execute_notebook(&flags)?
        .ok_or("missing read-only page inventory")?;
    let report = String::from_utf8(report)
        .map_err(|error| error.to_string())?;
    assert!(report.contains("Pages: 2\nRows: 4\nOrnaments: 0\n"),
        "{report}");
    let pages = report.split("Page glyph inventory: ").nth(1)
        .ok_or("missing page inventory section")?;
    assert_eq!(pages, concat!(
        "2 physical pages\n",
        "Page 1: 4 exact forms, 4 uses\n",
        "  \"a\": 1\n",
               "  \"n\\u{303}\": 1\n",
        "  \"\\u{a0}\": 1\n",
        "  \"ó\": 1\n",
        "Page 2: 2 exact forms, 2 uses\n",
        "  \"a\": 1\n",
        "  \"ó\": 1\n",
    ));
    let global = report.split("Glyph inventory: ").nth(1)
        .ok_or("missing global inventory")?;
    assert!(global.starts_with("4 exact forms, 6 uses\n"),
        "{global}");

    let global_only = execute_notebook(&args(&[
        "--inspect-flow", "--inspect-flow-inventory", "--flow-text",
        source, "--flow-region", "30000", "40000", "32000", "25000",
    ]))?.ok_or("missing existing inventory")?;
    assert!(report.as_bytes().starts_with(&global_only),
        "page option must extend, not alter, existing reports");
    Ok(())
}

#[test]
fn notebook_inspect_flow_page_inventory_excludes_synthetic_markers()
-> Result<(), String> {
    let report = execute_notebook(&args(&[
        "--inspect-flow-page-inventory", "--inspect-flow-inventory",
        "--inspect-flow", "--flow-markdown",
        "# ó\n- ó\n> n\u{303}\n1. ó",
    ]))?.ok_or("missing synthetic-free page inventory")?;
    let report = String::from_utf8(report)
        .map_err(|error| error.to_string())?;
    let pages = report.split("Page glyph inventory: ").nth(1)
        .ok_or("missing page section")?;
    assert_eq!(pages, concat!(
        "1 physical page\n",
        "Page 1: 4 exact forms, 6 uses\n",
        "  \".\": 1\n",
        "  \"1\": 1\n",
        "  \"n\\u{303}\": 1\n",
        "  \"ó\": 3\n",
    ));
    Ok(())
}

#[test]
fn notebook_inspect_flow_page_inventory_preflights_before_source_io() {
    for (extra, expected) in [
        (vec!["--inspect-flow-page-inventory"],
            "--inspect-flow-page-inventory requires"),
        (vec!["--inspect-flow", "--inspect-flow-page-inventory"],
            "--inspect-flow-page-inventory requires"),
        (vec!["--inspect-flow", "--inspect-flow-inventory",
            "--inspect-flow-page-inventory", "--inspect-flow-page-inventory"],
            "--inspect-flow-page-inventory may appear only once"),
        (vec!["--inspect-flow", "--inspect-flow-inventory",
            "--inspect-flow-page-inventory", "--export-paper", "pdf"],
            "--inspect-flow is stdout only"),
    ] {
        let mut command = args(&[
            "--profile", "missing-flow-page-inventory-notebook.json",
            "--flow-text-file", "missing-flow-page-inventory-source.txt",
        ]);
        command.extend(extra.into_iter().map(str::to_owned));
        let error = execute_notebook(&command)
            .expect_err("invalid report must refuse before selected IO");
        assert!(error.contains(expected), "{error:?}");
    }
}

#[test]
fn notebook_inspect_flow_page_inventory_includes_blank_physical_sheets()
-> Result<(), String> {
    let report = execute_notebook(&args(&[
        "--inspect-flow-page-inventory", "--inspect-flow-inventory",
        "--inspect-flow", "--flow-text", "\n\na",
        "--flow-region", "30000", "40000", "32000", "25000",
    ]))?.ok_or("missing read-only page report")?;
    let report = String::from_utf8(report)
        .map_err(|error| error.to_string())?;
    assert!(report.contains("Pages: 2\nRows: 1\nOrnaments: 0\n"),
        "{report}");
    assert!(report.ends_with(concat!(
        "Page glyph inventory: 2 physical pages\n",
        "Page 1: 0 exact forms, 0 uses\n",
        "Page 2: 1 exact forms, 1 uses\n",
        "  \"a\": 1\n",
    )), "{report}");
    Ok(())
}

#[test]
fn notebook_inspect_flow_page_inventory_matches_independent_row_oracle()
-> Result<(), String> {
    // Each source row consumes one physical 9-mm slot. The selected
    // 25-mm region holds exactly two slots on a sheet, including blanks.
    // Avoid reusing the implementation's grapheme indexer or page maps.
    let forms = ["a", "n\u{303}", "ó", "…", ""];
    let mut seed = 0x1f02_573c_8a4b_d129_u64;
    for case in 0..256 {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        let count = 1 + (seed as usize % 60);
        let mut lines = Vec::with_capacity(count);
        for line in 0..count {
            seed ^= seed << 13;
            seed ^= seed >> 7;
            seed ^= seed << 17;
            lines.push(if line + 1 == count {
                "a"
            } else {
                forms[(seed as usize) % forms.len()]
            });
        }
        let source = lines.join("\n");
        let report = execute_notebook(&args(&[
            "--inspect-flow-page-inventory", "--inspect-flow-inventory",
            "--inspect-flow", "--flow-text", &source,
            "--flow-region", "30000", "40000", "32000", "25000",
        ]))?.ok_or("missing seeded page inventory")?;
        let report = String::from_utf8(report)
            .map_err(|error| error.to_string())?;
        let pages = count.div_ceil(2);
        let noun = if pages == 1 { "page" } else { "pages" };
        let mut expected = format!("{pages} physical {noun}\n");
        for page in 0..pages {
            let chunk = &lines[page * 2..(page * 2 + 2).min(count)];
            let matches = |s| chunk.iter().filter(|&&item| item == s).count();
            let uses = chunk.iter().filter(|&&item| !item.is_empty()).count();
            let distinct = forms[..4].iter().filter(|&&s| matches(s) > 0)
                .count();
            expected.push_str(&format!(
                "Page {}: {distinct} exact forms, {uses} uses\n",
                page + 1,
            ));
            for form in &forms[..4] {
                let quantity = matches(form);
                if quantity != 0 {
                    expected.push_str(&format!(
                        "  {form:?}: {quantity}\n",
                    ));
                }
            }
        }
        let observed = report.split("Page glyph inventory: ").nth(1)
            .ok_or("missing per-sheet seeded inventory")?;
        if observed != expected {
            return Err(format!(
                "case {case} ({source:?}): {observed:?} != {expected:?}",
            ));
        }
    }
    Ok(())
}
