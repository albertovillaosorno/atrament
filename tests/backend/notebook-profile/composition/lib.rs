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

#[test]
fn notebook_dates_and_printed_page_numbers_render() {
    let mut notebook = NotebookProfile {
        printed_sheet: true,
        ..NotebookProfile::default()
    };
    let bare_pdf = notebook.paper_layer_pdf(2).expect("bare pages");
    let bare_png = notebook.paper_layer_png(72).expect("bare overlay");
    notebook.date_style = DateStyle::Line;
    notebook.date_placement = DatePlacement::TopLeft;
    let left_line_pdf = notebook.paper_layer_pdf(2).expect("date line PDF");
    let left_line_png = notebook.paper_layer_png(72).expect("date line PNG");
    assert_ne!(bare_pdf, left_line_pdf);
    assert_ne!(bare_png, left_line_png);
    assert_eq!(
        String::from_utf8_lossy(&left_line_pdf).matches(" m\n").count(), 2,
        "one date line per physical page and no printed grid",
    );
    notebook.date_placement = DatePlacement::TopRight;
    assert_ne!(left_line_png, notebook.paper_layer_png(72).expect("right"));
    notebook.date_style = DateStyle::Box;
    assert_eq!(
        String::from_utf8_lossy(&notebook.paper_layer_pdf(2)
            .expect("box PDF")).matches(" m\n").count(), 8,
    );
    let without_numbering = notebook.paper_layer_pdf(2).expect("no number");
    let without_number_png = notebook.paper_layer_png(72).expect("no number");
    notebook.page_numbers = true;
    let numbered = notebook.paper_layer_pdf(2).expect("numbered pages");
    assert_ne!(without_numbering, numbered);
    assert_ne!(
        without_number_png,
        notebook.paper_layer_png(72).expect("first page PNG number"),
    );
    let pdf_text = String::from_utf8_lossy(&numbered);
    let page_streams = pdf_text.split("\nstream\n").skip(1)
        .map(|entry| entry.split("\nendstream").next().unwrap_or(""))
        .collect::<Vec<_>>();
    assert_eq!(page_streams.len(), 2);
    assert_ne!(
        page_streams.first().expect("PDF page stream"),
        page_streams.get(1).expect("PDF page stream"),
    );
    assert!(page_streams.iter().all(|stream| stream.contains("\nf\n")));
    notebook.paper_json = notebook.paper_json.replace(
        "\"margin_top_um\": 22000", "\"margin_top_um\": 9000",
    );
    assert!(notebook.validate().is_err(), "refuse date overlap at admission");
    assert!(notebook.paper_layer_pdf(2).is_err());
    assert!(notebook.paper_layer_png(72).is_err());
}

#[test]
fn selected_png_page_matches_printed_pdf_index() {
    let mut notebook = NotebookProfile {
        printed_sheet: true,
        page_numbers: true,
        date_style: DateStyle::Line,
        date_placement: DatePlacement::TopRight,
        ..NotebookProfile::default()
    };
    let first = notebook.paper_layer_png(72).expect("default PNG page");
    assert_eq!(
        first,
        notebook.paper_layer_png_page(72, 1).expect("first page"),
    );
    let ninth = notebook.paper_layer_png_page(72, 9).expect("ninth page");
    let tenth = notebook.paper_layer_png_page(72, 10).expect("tenth page");
    assert_ne!(first, ninth);
    assert_ne!(ninth, tenth, "two-digit printed marker differs");
    assert_ne!(
        tenth,
        notebook.paper_layer_png_page(72, 32).expect("last page"),
    );
    assert!(notebook.paper_layer_png_page(72, 0).is_err());
    assert!(notebook.paper_layer_png_page(72, 33).is_err());
    notebook.page_numbers = false;
    assert_eq!(
        notebook.paper_layer_png_page(72, 1).expect("unmarked first"),
        notebook.paper_layer_png_page(72, 32).expect("unmarked last"),
        "when numbering is off, identical physical pages agree",
    );
}

#[test]
fn page_number_clearance_refuses_at_admission_and_two_digit_pages_differ() {
    let mut notebook = NotebookProfile {
        printed_sheet: true,
        page_numbers: true,
        ..NotebookProfile::default()
    };
    notebook.paper_json = notebook.paper_json.replace(
        "\"margin_bottom_um\": 16000", "\"margin_bottom_um\": 6000",
    );
    assert!(notebook.validate().is_err(), "7 mm clearance required");
    assert!(notebook.paper_layer_pdf(1).is_err());
    assert!(notebook.paper_layer_png(72).is_err());
    notebook.page_numbers = false;
    assert!(notebook.validate().is_ok());
    notebook.paper_json = NotebookProfile::default().paper_json;
    notebook.page_numbers = true;
    let pdf = notebook.paper_layer_pdf(32).expect("all 32 pages numbered");
    let text = String::from_utf8_lossy(&pdf);
    let streams = text.split("\nstream\n").skip(1)
        .map(|entry| entry.split("\nendstream").next().unwrap_or(""))
        .collect::<Vec<_>>();
    assert_eq!(streams.len(), 32);
    assert_ne!(
        streams.get(8).expect("PDF page stream"),
        streams.get(9).expect("PDF page stream"),
        "9 differs from 10",
    );
    assert_ne!(
        streams.get(30).expect("PDF page stream"),
        streams.get(31).expect("PDF page stream"),
        "31 differs from 32",
    );
    assert!(streams.get(9).expect("PDF page stream").matches("\nf\n").count()
        > streams.first().expect("PDF page stream").matches("\nf\n").count());
}
