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
//   - Rust notebook profile canonical roundtrip and typed refusal fixtures.
// - Must-Not:
//   - Imply dates, borders or printed-page metadata already appear in PDFs.
// - Allows:
//   - Inputs: Bounded in-memory JSON and native paper profile snippets.
//   - Outputs: Exact Rust roundtrip, validation and rejection assertions.
//   - Side effects: None.
// - Split-When:
//   - The local notebook profile editor gains its own integration fixtures.
// - Merge-When:
//   - Complete notebook PDF acceptance subsumes these checks.
// - Summary:
//   - Proves reusable typed notebook fields without duplicate paper semantics.
// - Description:
//   - Covers date placement, page flags, border bounds and malformed inputs.
// - Usage:
//   - cargo test -p atrament_notebook_profile
// - Defaults:
//   - In-memory inputs only.
//

//! Notebook v1 profile regression tests.

#![expect(clippy::expect_used, reason = "fail-fast test assertions")]

use super::*;

#[test]
fn default_profile_reopens_byte_identically() {
    let original = NotebookProfile::default()
        .canonical_json()
        .expect("default canonical");
    let reopened = NotebookProfile::from_json(original.as_bytes())
        .expect("default reopens");
    assert_eq!(reopened, NotebookProfile::default());
    assert_eq!(reopened.canonical_json().expect("canonical"), original);
    assert!(original.contains("\"schema\": \"atrament.notebook.v1\""));
    assert!(original.contains("\"schema\": \"atrament.paper.v1\""));
}

#[test]
fn edited_notebook_preserves_native_paper_and_metadata() {
    let paper = default_paper_profile_json()
        .replace("\"style\": \"grid\"", "\"style\": \"ruled\"")
        .replace("\"spacing_um\": 5000", "\"spacing_um\": 7000");
    let profile = NotebookProfile {
        paper_json: paper,
        date_style: DateStyle::Box,
        date_placement: DatePlacement::TopRight,
        border_radius_um: 4_000,
        page_numbers: true,
        printed_sheet: true,
    };
    let encoded = profile.canonical_json().expect("admitted");
    let reopened = NotebookProfile::from_json(encoded.as_bytes())
        .expect("edited notebook reopens");
    assert_eq!(reopened, profile);
    assert!(encoded.contains("\"spacing_um\": 7000"));
    assert!(encoded.contains("\"printed_sheet\": true"));
    assert_eq!(reopened.canonical_json().expect("stable"), encoded);
}

#[test]
fn contradictory_metadata_and_impossible_border_refuse() {
    let mut notebook = NotebookProfile {
        date_style: DateStyle::Line,
        ..NotebookProfile::default()
    };
    assert!(notebook.validate().is_err());
    notebook.date_placement = DatePlacement::TopLeft;
    assert!(notebook.validate().is_ok());
    notebook.border_radius_um = 100_000;
    assert!(notebook.validate().is_err());
    notebook.border_radius_um = 5_000;
    notebook.paper_json = notebook
        .paper_json
        .replace("\"margin_left_um\": 20000", "\"margin_left_um\": 220000");
    assert!(notebook.canonical_json().is_err());
}

#[test]
fn wrong_schema_unknown_field_and_forged_types_refuse() {
    let original = NotebookProfile::default()
        .canonical_json()
        .expect("canonical");
    for corrupted in [
        original.replace("atrament.notebook.v1", "atrament.notebook.v2"),
        original.replace("\"printed_sheet\"", "\"strange\""),
        original.replace("\"page_numbers\": false", "\"page_numbers\": 0"),
        original.replace("\"border_radius_um\": 0", "\"border_radius_um\": -1"),
        original.replace("\"date_style\": \"none\"", "\"date_style\": \"box\""),
        original.replace("\"width_um\": 210000", "\"width_um\": 0"),
        original.replace(
            "\"schema\": \"atrament.paper.v1\"",
            "\"schema\": \"invalid\"",
        ),
        original.replace(
            "\"printed_sheet\": false",
            "\"printed_sheet\": false, \"printed_sheet\": false",
        ),
    ] {
        assert!(
            NotebookProfile::from_json(corrupted.as_bytes()).is_err(),
            "{corrupted}"
        );
    }
}

#[test]
fn duplicate_fields_oversized_input_and_invalid_paper_refuse() {
    let original = NotebookProfile::default()
        .canonical_json()
        .expect("canonical");
    let oversized =
        format!("{}{}", " ".repeat(MAX_NOTEBOOK_JSON_BYTES), original);
    assert!(NotebookProfile::from_json(oversized.as_bytes()).is_err());
    assert!(
        NotebookProfile::from_json(br#"{"schema":"x","schema":"x"}"#).is_err()
    );
    let invalid_nested_paper = original
        .replace("\"margin_left_um\": 20000", "\"margin_left_um\": 300000");
    assert!(
        NotebookProfile::from_json(invalid_nested_paper.as_bytes()).is_err()
    );
}

#[test]
fn notebook_paper_layer_honors_preprinted_and_bounds() {
    let marked = NotebookProfile::default();
    let mut preprinted = marked.clone();
    preprinted.printed_sheet = true;
    assert_ne!(
        marked.paper_layer_pdf(2).expect("marked sheet"),
        preprinted.paper_layer_pdf(2).expect("preprinted sheet"),
    );
    assert_ne!(
        marked.paper_layer_png(72).expect("marked layer"),
        preprinted.paper_layer_png(72).expect("transparent layer"),
    );
    assert!(preprinted.paper_layer_pdf(0).is_err());
    assert!(preprinted.paper_layer_pdf(33).is_err());
    assert!(preprinted.paper_layer_png(71).is_err());
    preprinted.date_style = DateStyle::Box;
    assert!(preprinted.paper_layer_pdf(1).is_err());
    assert!(preprinted.paper_layer_png(72).is_err());
}
