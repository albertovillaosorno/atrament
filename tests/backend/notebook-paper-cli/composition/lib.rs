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
        .expect("not help");
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
            Ok(Some(options)) => assert!(render(options).is_err(), "{args:?}"),
            Err(_error) => {}
            Ok(None) => panic!("invalid options must not show help: {args:?}"),
        }
    }
}
