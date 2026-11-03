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
//   - Independent byte-format and refusal regressions for vector PDF output.
// - Must-Not:
//   - Claim notebook Export, font, image, or physical renderer functionality.
// - Allows:
//   - Inputs: Resolved synthetic vector paths and bounded PDF composition.
//   - Outputs: Assertions for bytes, xref integrity, size and typed refusal.
//   - Side effects: Process-local test allocation only.
// - Split-When:
//   - Font or image resources require independent conformance fixtures.
// - Merge-When:
//   - Complete PDF output gains an end-to-end browser integration suite.
// - Summary:
//   - Pins actual PDF bytes and fail-closed unsupported-resource behavior.
// - Description:
//   - Tests page geometry, vector commands, metadata and xref offsets.
// - Usage:
//   - Run the pdf-vector-serializer package test harness.
// - Defaults:
//   - No fixture silently discards content or creates a file.
//

//! PDF vector-only byte-format and refusal regression fixtures.

use std::str::from_utf8;

use atrament_pdf_composition_plan::{
    PdfAssetDisposition, PdfAssetResource, PdfCompositionPlan, PdfPagePlan,
    SearchableSemanticText,
};
use atrament_pdf_vector_serializer::{
    PdfFillRule, PdfPathCommand, PdfPathPaint, PdfPoint, PdfRgb,
    PdfVectorColorIntent, PdfVectorComposition, PdfVectorError,
    PdfVectorLimits, PdfVectorPageBox, PdfVectorPath, serialize_vector_pdf,
};

const fn limits() -> PdfVectorLimits {
    PdfVectorLimits {
        maximum_pages: 4,
        maximum_paths_per_page: 20,
        maximum_commands_per_page: 100,
        maximum_pdf_bytes: 20_000,
    }
}

const fn point(x_um: i32, y_um: i32) -> PdfPoint {
    PdfPoint { x_um, y_um }
}

fn stroke() -> PdfVectorPath {
    PdfVectorPath {
        commands: vec![
            PdfPathCommand::MoveTo(point(20_000, 30_000)),
            PdfPathCommand::LineTo(point(50_000, 40_000)),
            PdfPathCommand::CubicTo(
                point(55_000, 50_000),
                point(65_000, 20_000),
                point(80_000, 35_000),
            ),
        ],
        paint: PdfPathPaint::Stroke {
            color: PdfRgb {
                red: 5,
                green: 22,
                blue: 240,
            },
            width_um: 350,
        },
    }
}

fn fill() -> PdfVectorPath {
    PdfVectorPath {
        commands: vec![
            PdfPathCommand::MoveTo(point(100_000, 100_000)),
            PdfPathCommand::LineTo(point(150_000, 100_000)),
            PdfPathCommand::LineTo(point(125_000, 120_000)),
            PdfPathCommand::Close,
        ],
        paint: PdfPathPaint::Fill {
            color: PdfRgb {
                red: 210,
                green: 10,
                blue: 50,
            },
            rule: PdfFillRule::NonZero,
        },
    }
}

const fn page(
    paths: Vec<PdfVectorPath>,
) -> PdfPagePlan<PdfVectorColorIntent, PdfVectorPageBox, (), Vec<PdfVectorPath>>
{
    PdfPagePlan {
        color_intent: PdfVectorColorIntent::DeviceRgb,
        physical_page_box: PdfVectorPageBox {
            width_um: 210_000,
            height_um: 297_000,
        },
        searchable_text: SearchableSemanticText::OmittedAsIncompatible,
        vector_authority: paths,
    }
}

fn document() -> PdfVectorComposition<'static> {
    PdfCompositionPlan {
        assets: Vec::new(),
        pages: vec![page(vec![stroke(), fill()])],
        render_manifest_identity: "render-identity.v1",
    }
}

fn pdf_string(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}

// Verify byte offsets independently of the serializer's in-memory index.
#[expect(
    clippy::expect_used,
    clippy::arithmetic_side_effects,
    clippy::missing_assert_message,
    reason = "test oracle reports precisely the first malformed PDF field"
)]
fn assert_xref_integrity(bytes: &[u8]) {
    let text = pdf_string(bytes);
    let start_label = "startxref\n";
    let start = text
        .rfind(start_label)
        .expect("PDF has a startxref section");
    let xref_offset_text = text
        .get(start + start_label.len()..)
        .expect("valid xref start")
        .lines()
        .next()
        .expect("xref byte offset");
    let xref_offset: usize =
        xref_offset_text.parse().expect("numeric xref byte offset");
    assert_eq!(
        bytes.get(xref_offset..xref_offset + 5),
        Some(b"xref\n".as_slice()),
    );
    let records = from_utf8(bytes.get(xref_offset..).expect("xref body bytes"))
        .expect("xref is ASCII")
        .lines()
        .skip(2);
    let mut object_index = 0usize;
    for record in records {
        if record == "trailer" {
            break;
        }
        let mut tokens = record.split_ascii_whitespace();
        let offset = tokens
            .next()
            .expect("offset")
            .parse::<usize>()
            .expect("numeric xref record offset");
        let generation = tokens.next().expect("generation number");
        let status = tokens.next().expect("object entry status");
        if object_index == 0 {
            assert_eq!(status, "f");
            assert_eq!(generation, "65535");
        } else {
            assert_eq!(status, "n");
            assert_eq!(generation, "00000");
            let header = format!("{object_index} 0 obj\n");
            assert!(
                bytes.get(offset..).is_some_and(|data| {
                    data.starts_with(header.as_bytes())
                }),
                "xref object {object_index} offset"
            );
        }
        object_index += 1;
    }
    let expected = text
        .split("/Size ")
        .nth(1)
        .expect("trailer size")
        .split_ascii_whitespace()
        .next()
        .expect("size number")
        .parse::<usize>()
        .expect("numeric trailer size");
    assert_eq!(object_index, expected);
}

#[expect(
    clippy::expect_used,
    reason = "a failed PDF fixture is a test failure"
)]
#[test]
fn emits_real_two_page_pdf_with_physical_box_order_paths_and_metadata() {
    let mut composition = document();
    composition.pages.push(page(vec![fill()]));
    let first = serialize_vector_pdf(&composition, limits())
        .expect("two page PDF must serialize");
    let second = serialize_vector_pdf(&composition, limits())
        .expect("same resolved vectors produce the same bytes");
    assert_eq!(first, second, "PDF serialization is deterministic");
    assert!(first.starts_with(b"%PDF-1.4\n"));
    assert!(first.ends_with(b"%%EOF\n"));
    assert_xref_integrity(&first);
    let text = pdf_string(&first);
    assert!(text.contains("/Count 2"));
    assert_eq!(text.matches("/MediaBox [0 0 595.276 841.890]").count(), 2);
    assert!(text.contains("/AtramentManifestID (render-identity.v1)"));
    assert!(text.contains("1 0 0 -1 0 841.890 cm"));
    assert!(text.contains("141.732 113.386 l"));
    assert!(text.contains("155.906 141.732 184.252 56.693 226.772 99.213 c"));
    assert!(text.contains("0.020 0.086 0.941 RG"));
    assert!(text.contains("0.824 0.039 0.196 rg"));
    assert!(text.contains("h\nf\n"));
    assert!(text.contains("S\n"));
}

#[expect(
    clippy::indexing_slicing,
    reason = "fixture has one known page"
)]
#[test]
fn unsupported_accepted_content_fails_instead_of_disappearing() {
    let mut composition = document();
    composition.assets = vec![PdfAssetResource {
        asset_identity: "photo-1",
        disposition: PdfAssetDisposition::SafelyReferenced,
    }];
    assert_eq!(
        serialize_vector_pdf(&composition, limits()),
        Err(PdfVectorError::UnsupportedAssets),
    );
    composition.assets.clear();
    composition.pages[0].searchable_text =
        SearchableSemanticText::Preserved(());
    assert_eq!(
        serialize_vector_pdf(&composition, limits()),
        Err(PdfVectorError::UnsupportedSearchableText),
    );
}

#[expect(
    clippy::indexing_slicing,
    reason = "fixture has two known paths"
)]
#[test]
fn malformed_geometry_and_spatial_overflow_are_rejected() {
    let mut composition = document();
    let invalid_paths = [
        vec![],
        vec![PdfPathCommand::LineTo(point(12_000, 20_000))],
        vec![PdfPathCommand::MoveTo(point(12_000, 20_000))],
        vec![
            PdfPathCommand::MoveTo(point(0, 0)),
            PdfPathCommand::LineTo(point(210_001, 0)),
        ],
        vec![
            PdfPathCommand::MoveTo(point(10_000, 10_000)),
            PdfPathCommand::CubicTo(
                point(-1, 12_000),
                point(10_000, 12_000),
                point(20_000, 20_000),
            ),
        ],
        vec![
            PdfPathCommand::MoveTo(point(10_000, 10_000)),
            PdfPathCommand::Close,
        ],
    ];
    for commands in invalid_paths {
        composition.pages[0].vector_authority[0].commands = commands;
        assert_eq!(
            serialize_vector_pdf(&composition, limits()),
            Err(PdfVectorError::InvalidPath),
        );
    }
    let mut invalid_fill_document = document();
    assert_eq!(
        invalid_fill_document.pages[0].vector_authority[1]
            .commands
            .pop(),
        Some(PdfPathCommand::Close),
    );
    assert_eq!(
        serialize_vector_pdf(&invalid_fill_document, limits()),
        Err(PdfVectorError::InvalidPath),
        "fills must have closed paths",
    );
}

#[expect(
    clippy::indexing_slicing,
    reason = "fixture has one known page and two paths"
)]
#[test]
fn physical_dimensions_stroke_width_and_all_budgets_fail_closed() {
    let mut composition = document();
    composition.pages[0].physical_page_box.width_um = 0;
    assert_eq!(
        serialize_vector_pdf(&composition, limits()),
        Err(PdfVectorError::InvalidPageBox),
    );
    composition.pages[0].physical_page_box.width_um = 210_000;
    composition.pages[0].vector_authority[0].paint = PdfPathPaint::Stroke {
        color: PdfRgb {
            red: 0,
            green: 0,
            blue: 0,
        },
        width_um: 0,
    };
    assert_eq!(
        serialize_vector_pdf(&composition, limits()),
        Err(PdfVectorError::InvalidStrokeWidth),
    );
    let bounded_document = document();
    for restricted in [
        PdfVectorLimits {
            maximum_pages: 0,
            ..limits()
        },
        PdfVectorLimits {
            maximum_paths_per_page: 1,
            ..limits()
        },
        PdfVectorLimits {
            maximum_commands_per_page: 2,
            ..limits()
        },
        PdfVectorLimits {
            maximum_pdf_bytes: 100,
            ..limits()
        },
    ] {
        assert_eq!(
            serialize_vector_pdf(&bounded_document, restricted),
            Err(PdfVectorError::ResourceLimit),
        );
    }
    let mut invalid_manifest_document = document();
    invalid_manifest_document.render_manifest_identity = "unknown) /X (unsafe";
    assert_eq!(
        serialize_vector_pdf(&invalid_manifest_document, limits()),
        Err(PdfVectorError::InvalidManifestIdentity),
    );
    invalid_manifest_document.pages.clear();
    assert_eq!(
        serialize_vector_pdf(&invalid_manifest_document, limits()),
        Err(PdfVectorError::EmptyDocument),
    );
}

#[expect(
    clippy::indexing_slicing,
    clippy::expect_used,
    reason = "bounded one-page fixture exercises exact byte and path limits"
)]
#[test]
fn exact_byte_budget_accepts_and_one_byte_short_refuses() {
    let composition = document();
    let bytes = serialize_vector_pdf(&composition, limits())
        .expect("baseline vector PDF must serialize");
    let exact = PdfVectorLimits {
        maximum_pdf_bytes: bytes.len(),
        ..limits()
    };
    assert_eq!(
        serialize_vector_pdf(&composition, exact),
        Ok(bytes.clone()),
        "byte budget at the exact serialized length must fit",
    );
    let one_short = PdfVectorLimits {
        maximum_pdf_bytes: bytes.len().saturating_sub(1),
        ..limits()
    };
    assert_eq!(
        serialize_vector_pdf(&composition, one_short),
        Err(PdfVectorError::ResourceLimit),
        "one byte short cannot report a truncated success",
    );
    let mut clipped_stroke = document();
    clipped_stroke.pages[0].vector_authority[0].commands = vec![
        PdfPathCommand::MoveTo(point(174, 30_000)),
        PdfPathCommand::LineTo(point(60_000, 30_000)),
    ];
    assert_eq!(
        serialize_vector_pdf(&clipped_stroke, limits()),
        Err(PdfVectorError::InvalidPath),
        "350um round stroke clipped by left edge must refuse",
    );
    clipped_stroke.pages[0].vector_authority[0].commands = vec![
        PdfPathCommand::MoveTo(point(175, 30_000)),
        PdfPathCommand::LineTo(point(60_000, 30_000)),
    ];
    assert!(
        serialize_vector_pdf(&clipped_stroke, limits()).is_ok(),
        "exact half-width stroke margin is admitted",
    );
    clipped_stroke.pages[0].vector_authority[0].commands = vec![
        PdfPathCommand::MoveTo(point(20_000, 30_000)),
        PdfPathCommand::CubicTo(
            point(20_000, 30_000),
            point(20_000, 296_826),
            point(50_000, 60_000),
        ),
    ];
    assert_eq!(
        serialize_vector_pdf(&clipped_stroke, limits()),
        Err(PdfVectorError::InvalidPath),
        "out-of-bounds cubic control point must refuse",
    );
}

#[expect(
    clippy::expect_used,
    reason = "valid synthetic compound contours are a test invariant"
)]
#[test]
fn compound_contours_preserve_explicit_even_odd_or_nonzero_fill_rule() {
    // Both nested squares use the same winding direction. An even-odd fill
    // punches a hole, while nonzero fill covers the nested region.
    let contours = vec![
        PdfPathCommand::MoveTo(point(20_000, 20_000)),
        PdfPathCommand::LineTo(point(100_000, 20_000)),
        PdfPathCommand::LineTo(point(100_000, 100_000)),
        PdfPathCommand::LineTo(point(20_000, 100_000)),
        PdfPathCommand::Close,
        PdfPathCommand::MoveTo(point(40_000, 40_000)),
        PdfPathCommand::LineTo(point(80_000, 40_000)),
        PdfPathCommand::LineTo(point(80_000, 80_000)),
        PdfPathCommand::LineTo(point(40_000, 80_000)),
        PdfPathCommand::Close,
    ];
    let color = PdfRgb {
        red: 190,
        green: 50,
        blue: 80,
    };
    let mut vector_path = PdfVectorPath {
        commands: contours,
        paint: PdfPathPaint::Fill {
            color,
            rule: PdfFillRule::EvenOdd,
        },
    };
    let mut composition = document();
    composition.pages = vec![page(vec![vector_path.clone()])];
    let parity_bytes = serialize_vector_pdf(&composition, limits())
        .expect("even-odd compound fill must serialize");
    let parity = pdf_string(&parity_bytes);
    assert_eq!(
        parity.matches("h\n").count(),
        2,
        "both caller-provided contours must be preserved",
    );
    assert!(
        parity.contains("h\nf*\n"),
        "even-odd rule must emit the PDF parity fill operator",
    );
    vector_path.paint = PdfPathPaint::Fill {
        color,
        rule: PdfFillRule::NonZero,
    };
    composition.pages = vec![page(vec![vector_path])];
    let winding_bytes = serialize_vector_pdf(&composition, limits())
        .expect("nonzero compound fill must serialize");
    let winding = pdf_string(&winding_bytes);
    assert_eq!(
        winding.matches("h\n").count(),
        2,
        "nonzero fill keeps both caller contours",
    );
    assert!(
        winding.contains("h\nf\n"),
        "nonzero rule must emit the PDF winding fill operator",
    );
    assert_ne!(
        parity_bytes, winding_bytes,
        "different caller fill rules must produce distinct output",
    );
}
