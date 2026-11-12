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
    clippy::expect_used,
    reason = "empty-page bytes anchor the minimum per-path paint allowance"
)]
#[test]
fn a_path_without_commands_cannot_hide_exhausted_paint_budget() {
    let mut composition = PdfCompositionPlan {
        assets: Vec::new(),
        pages: vec![page(Vec::new())],
        render_manifest_identity: "empty-path-paint-budget.v1",
    };
    let blank = serialize_vector_pdf(&composition, limits())
        .expect("a page without drawing commands fits");
    for paint in [stroke().paint, fill().paint] {
        composition.pages = vec![page(vec![PdfVectorPath {
            commands: Vec::new(),
            paint,
        }])];
        assert_eq!(
            serialize_vector_pdf(&composition, PdfVectorLimits {
                maximum_pdf_bytes: blank.len(),
                ..limits()
            },),
            Err(PdfVectorError::ResourceLimit),
            "the paint framing cannot fit even if geometry is malformed",
        );
        assert_eq!(
            serialize_vector_pdf(&composition, limits()),
            Err(PdfVectorError::InvalidPath),
            "without a byte shortage the path remains invalid",
        );
    }
}

#[expect(
    clippy::expect_used,
    clippy::indexing_slicing,
    reason = "accepted PDF bytes establish a command-only shortage"
)]
#[test]
fn impossible_command_framing_refuses_before_invalid_coordinates() {
    for (mut path, invalid_command) in [
        (stroke(), PdfPathCommand::LineTo(point(-1, 40_000))),
        (
            stroke(),
            PdfPathCommand::CubicTo(
                point(-1, 40_000),
                point(60_000, 40_000),
                point(70_000, 40_000),
            ),
        ),
        (stroke(), PdfPathCommand::MoveTo(point(-1, 40_000))),
        (fill(), PdfPathCommand::Close),
    ] {
        path.commands = match path.paint {
            PdfPathPaint::Stroke { .. } => vec![
                PdfPathCommand::MoveTo(point(20_000, 30_000)),
                PdfPathCommand::LineTo(point(50_000, 40_000)),
            ],
            PdfPathPaint::Fill { .. } => fill().commands,
        };
        let mut composition = PdfCompositionPlan {
            assets: Vec::new(),
            pages: vec![page(vec![path])],
            render_manifest_identity: "command-framing-budget.v1",
        };
        let accepted = serialize_vector_pdf(&composition, limits())
            .expect("baseline fully painted path is valid");
        composition.pages[0].vector_authority[0]
            .commands
            .push(invalid_command);
        assert_eq!(
            serialize_vector_pdf(&composition, limits()),
            Err(PdfVectorError::InvalidPath),
            "a malformed trailing command is invalid with enough room",
        );
        assert_eq!(
            serialize_vector_pdf(
                &composition,
                PdfVectorLimits {
                    maximum_pdf_bytes: accepted.len(),
                    ..limits()
                },
            ),
            Err(PdfVectorError::ResourceLimit),
            "no space for another command must refuse before its coordinates",
        );
    }
}

#[expect(
    clippy::expect_used,
    reason = "a blank PDF anchors a shortage of total command framing",
)]
#[test]
fn total_path_command_framing_preempts_invalid_first_command() {
    for paint in [stroke().paint, fill().paint] {
        let mut composition = PdfCompositionPlan {
            assets: Vec::new(),
            pages: vec![page(Vec::new())],
            render_manifest_identity: "total-command-framing.v1",
        };
        let blank = serialize_vector_pdf(&composition, limits())
            .expect("blank page fixes the mandatory document framing");
        let commands = match paint {
            PdfPathPaint::Stroke { .. } => vec![
                PdfPathCommand::MoveTo(point(-1, 30_000)),
                PdfPathCommand::LineTo(point(50_000, 40_000)),
            ],
            PdfPathPaint::Fill { .. } => vec![
                PdfPathCommand::MoveTo(point(-1, 30_000)),
                PdfPathCommand::LineTo(point(50_000, 30_000)),
                PdfPathCommand::LineTo(point(50_000, 40_000)),
                PdfPathCommand::Close,
            ],
        };
        composition.pages = vec![page(vec![PdfVectorPath {
            commands,
            paint,
        }])];
        assert_eq!(
            serialize_vector_pdf(&composition, limits()),
            Err(PdfVectorError::InvalidPath),
            "with ample bytes the first negative coordinate is invalid",
        );
        assert_eq!(
            serialize_vector_pdf(&composition, PdfVectorLimits {
                maximum_pdf_bytes: blank.len().saturating_add(60),
                ..limits()
            }),
            Err(PdfVectorError::ResourceLimit),
            "all mandatory command syntax must fit before inspecting ink",
        );
    }
}

#[expect(
    clippy::expect_used,
    reason = "a blank page anchors the future-path minimum byte budget",
)]
#[test]
fn later_path_framing_preempts_invalid_earlier_path() {
    let mut composition = PdfCompositionPlan {
        assets: Vec::new(),
        pages: vec![page(Vec::new())],
        render_manifest_identity: "later-path-framing.v1",
    };
    let blank = serialize_vector_pdf(&composition, limits())
        .expect("a blank page gives the exact minimal document length");
    let mut invalid = stroke();
    invalid.commands = vec![
        PdfPathCommand::MoveTo(point(-1, 30_000)),
        PdfPathCommand::LineTo(point(50_000, 40_000)),
    ];
    for later in [stroke(), fill()] {
        composition.pages = vec![page(vec![invalid.clone(), later])];
        assert_eq!(
            serialize_vector_pdf(&composition, limits()),
            Err(PdfVectorError::InvalidPath),
            "with room for both paths the negative point is invalid",
        );
        assert_eq!(
            serialize_vector_pdf(&composition, PdfVectorLimits {
                maximum_pdf_bytes: blank.len().saturating_add(100),
                ..limits()
            }),
            Err(PdfVectorError::ResourceLimit),
            "later paths must reserve their minimum bytes up front",
        );
    }
}

#[test]
fn later_command_count_limit_preempts_invalid_earlier_path() {
    let mut invalid = stroke();
    invalid.commands = vec![
        PdfPathCommand::MoveTo(point(-1, 30_000)),
        PdfPathCommand::LineTo(point(50_000, 40_000)),
    ];
    let mut later = stroke();
    later.commands = vec![PdfPathCommand::MoveTo(point(20_000, 30_000))];
    for index in 0i32..100i32 {
        later.commands.push(PdfPathCommand::LineTo(point(
            30_000 + index * 100, 40_000,
        )));
    }
    let composition = PdfCompositionPlan {
        assets: Vec::new(),
        pages: vec![page(vec![invalid, later])],
        render_manifest_identity: "later-command-limit.v1",
    };
    assert_eq!(
        serialize_vector_pdf(&composition, limits()),
        Err(PdfVectorError::ResourceLimit),
        "a page with too many commands must refuse before malformed ink",
    );
}

#[expect(
    clippy::indexing_slicing,
    reason = "two-page fixture has a known first path",
)]
#[test]
fn later_page_path_and_command_limits_preempt_invalid_earlier_ink() {
    let mut invalid = stroke();
    invalid.commands[0] = PdfPathCommand::MoveTo(point(-1, 30_000));
    let cases = [
        (
            vec![stroke(), stroke()],
            PdfVectorLimits {
                maximum_paths_per_page: 1,
                ..limits()
            },
        ),
        (
            vec![stroke()],
            PdfVectorLimits {
                maximum_commands_per_page: 2,
                ..limits()
            },
        ),
    ];
    for (later_paths, limits_with_later_overrun) in cases {
        let composition = PdfCompositionPlan {
            assets: Vec::new(),
            pages: vec![page(vec![invalid.clone()]), page(later_paths)],
            render_manifest_identity: "later-page-count-budget.v1",
        };
        assert_eq!(
            serialize_vector_pdf(&composition, limits()),
            Err(PdfVectorError::InvalidPath),
            "without a page-level overrun the first point is invalid",
        );
        assert_eq!(
            serialize_vector_pdf(&composition, limits_with_later_overrun),
            Err(PdfVectorError::ResourceLimit),
            "later-page count limits must be checked before drawing page one",
        );
    }
}

#[test]
fn every_stroked_subpath_requires_real_physical_extent() {
    let origin = point(20_000, 30_000);
    let second = point(50_000, 60_000);
    let other = point(80_000, 90_000);
    let zero_line = [
        PdfPathCommand::MoveTo(origin),
        PdfPathCommand::LineTo(origin),
    ];
    let zero_cubic = [
        PdfPathCommand::MoveTo(origin),
        PdfPathCommand::CubicTo(origin, origin, origin),
    ];
    let empty_second = [
        PdfPathCommand::MoveTo(origin),
        PdfPathCommand::LineTo(second),
        PdfPathCommand::MoveTo(other),
        PdfPathCommand::LineTo(other),
    ];
    let closed_empty_second = [
        PdfPathCommand::MoveTo(origin),
        PdfPathCommand::LineTo(second),
        PdfPathCommand::Close,
        PdfPathCommand::MoveTo(other),
        PdfPathCommand::LineTo(other),
        PdfPathCommand::Close,
    ];
    let zero_inside_nonempty = [
        PdfPathCommand::MoveTo(origin),
        PdfPathCommand::LineTo(second),
        PdfPathCommand::LineTo(second),
        PdfPathCommand::LineTo(other),
    ];
    let zero_cubic_inside_nonempty = [
        PdfPathCommand::MoveTo(origin),
        PdfPathCommand::LineTo(second),
        PdfPathCommand::CubicTo(second, second, second),
        PdfPathCommand::LineTo(other),
    ];
    for commands in [
        zero_inside_nonempty.as_slice(),
        zero_cubic_inside_nonempty.as_slice(),
        zero_line.as_slice(),
        zero_cubic.as_slice(),
        empty_second.as_slice(),
        closed_empty_second.as_slice(),
    ] {
        let mut composition = document();
        let mut empty_stroke = stroke();
        empty_stroke.commands = commands.to_vec();
        composition.pages = vec![page(vec![empty_stroke])];
        assert_eq!(
            serialize_vector_pdf(&composition, limits()),
            Err(PdfVectorError::InvalidPath),
            "zero-length stroke command must not become invisible or dot ink",
        );
    }

    // A cubic loop may start and end at the same point while still tracing
    // actual physical geometry. A second ordinary subpath is valid too.
    let mut composition = document();
    let mut nonzero_stroke = stroke();
    nonzero_stroke.commands = vec![
        PdfPathCommand::MoveTo(origin),
        PdfPathCommand::CubicTo(
            point(30_000, 50_000), point(50_000, 50_000), origin,
        ),
        PdfPathCommand::MoveTo(second),
        PdfPathCommand::LineTo(other),
    ];
    composition.pages = vec![page(vec![nonzero_stroke])];
    let pdf = serialize_vector_pdf(&composition, limits());
    assert!(pdf.is_ok(), "real cubic loops and multiple strokes remain valid");
}

#[test]
fn filled_contours_refuse_collinear_ink_but_accept_curved_ink() {
    let color = PdfRgb {
        red: 190,
        green: 50,
        blue: 80,
    };
    let square = vec![
        PdfPathCommand::MoveTo(point(20_000, 20_000)),
        PdfPathCommand::LineTo(point(80_000, 20_000)),
        PdfPathCommand::LineTo(point(80_000, 80_000)),
        PdfPathCommand::Close,
    ];
    let degenerate_contours = [
        vec![
            PdfPathCommand::MoveTo(point(20_000, 20_000)),
            PdfPathCommand::LineTo(point(80_000, 20_000)),
            PdfPathCommand::Close,
        ],
        vec![
            PdfPathCommand::MoveTo(point(20_000, 20_000)),
            PdfPathCommand::LineTo(point(50_000, 20_000)),
            PdfPathCommand::LineTo(point(80_000, 20_000)),
            PdfPathCommand::Close,
        ],
        vec![
            PdfPathCommand::MoveTo(point(20_000, 20_000)),
            PdfPathCommand::CubicTo(
                point(30_000, 20_000),
                point(60_000, 20_000),
                point(80_000, 20_000),
            ),
            PdfPathCommand::Close,
        ],
        {
            let mut compound = square.clone();
            compound.extend([
                PdfPathCommand::MoveTo(point(20_000, 100_000)),
                PdfPathCommand::LineTo(point(80_000, 100_000)),
                PdfPathCommand::Close,
            ]);
            compound
        },
    ];
    let mut composition = document();
    for commands in degenerate_contours {
        composition.pages = vec![page(vec![PdfVectorPath {
            commands,
            paint: PdfPathPaint::Fill {
                color,
                rule: PdfFillRule::EvenOdd,
            },
        }])];
        assert_eq!(
            serialize_vector_pdf(&composition, limits()),
            Err(PdfVectorError::InvalidPath),
            "an unpaintable fill contour must not disappear silently",
        );
    }
    for commands in [
        square,
        vec![
            PdfPathCommand::MoveTo(point(20_000, 20_000)),
            PdfPathCommand::LineTo(point(80_000, 20_000)),
            PdfPathCommand::LineTo(point(80_000, 20_000)),
            PdfPathCommand::LineTo(point(80_000, 80_000)),
            PdfPathCommand::Close,
        ],
        vec![
            PdfPathCommand::MoveTo(point(20_000, 20_000)),
            PdfPathCommand::CubicTo(
                point(30_000, 50_000),
                point(60_000, 50_000),
                point(80_000, 20_000),
            ),
            PdfPathCommand::Close,
        ],
        vec![
            PdfPathCommand::MoveTo(point(20_000, 20_000)),
            PdfPathCommand::LineTo(point(80_000, 80_000)),
            PdfPathCommand::LineTo(point(20_000, 80_000)),
            PdfPathCommand::LineTo(point(80_000, 20_000)),
            PdfPathCommand::Close,
        ],
    ] {
        composition.pages = vec![page(vec![PdfVectorPath {
            commands,
            paint: PdfPathPaint::Fill {
                color,
                rule: PdfFillRule::EvenOdd,
            },
        }])];
        assert!(
            serialize_vector_pdf(&composition, limits()).is_ok(),
            "curved and crossed filled contours must remain accepted",
        );
    }
}

#[expect(
    clippy::indexing_slicing,
    reason = "the fixture has exactly one page, one path and three vertices"
)]
#[test]
fn rounded_pdf_fill_contours_must_retain_noncollinear_ink() {
    let start = point(20_000, 20_000);
    let along_a = point(20_014, 20_016);
    let along_b = point(20_017, 20_019);
    // Distinct source vectors make a non-collinear triangle in micrometers,
    // but its three PDF 0.001-point coordinates fall on one exact line.
    // The nominal triangle must not silently disappear on PDF export.
    let mut composition = document();
    let commands = [
        PdfPathCommand::MoveTo(start),
        PdfPathCommand::LineTo(along_a),
        PdfPathCommand::LineTo(along_b),
        PdfPathCommand::Close,
    ];
    composition.pages = vec![page(vec![PdfVectorPath {
        commands: commands.to_vec(),
        paint: fill().paint,
    }])];
    assert_eq!(
        serialize_vector_pdf(&composition, limits()),
        Err(PdfVectorError::InvalidPath),
        "source area that collapses under PDF rounding must refuse",
    );

    // One micrometer of displacement restores a noncollinear serialized
    // contour, so the guard must not reject all very small filled shapes.
    composition.pages[0].vector_authority[0].commands[2] =
        PdfPathCommand::LineTo(point(20_017, 20_020));
    assert!(
        serialize_vector_pdf(&composition, limits()).is_ok(),
        "a tiny but still two-dimensional PDF contour remains printable",
    );

    // A cubic loop can also lose all its area after quantizing its control
    // points, even when its start and end are identical and source controls
    // span the plane. The same admission applies to curved contours.
    composition.pages[0].vector_authority[0].commands = vec![
        PdfPathCommand::MoveTo(start),
        PdfPathCommand::CubicTo(along_a, along_b, start),
        PdfPathCommand::Close,
    ];
    assert_eq!(
        serialize_vector_pdf(&composition, limits()),
        Err(PdfVectorError::InvalidPath),
        "rounding cannot turn a two-dimensional cubic loop into a line",
    );
    composition.pages[0].vector_authority[0].commands[1] =
        PdfPathCommand::CubicTo(along_a, point(20_017, 20_020), start);
    assert!(
        serialize_vector_pdf(&composition, limits()).is_ok(),
        "a small cubic loop with two-dimensional PDF controls remains valid",
    );

    // Compound fills must not hide one quantization-collapsed member just
    // because an earlier and larger contour is printable.
    let mut compound = fill();
    compound.commands.extend(commands);
    composition.pages[0].vector_authority = vec![compound];
    assert_eq!(
        serialize_vector_pdf(&composition, limits()),
        Err(PdfVectorError::InvalidPath),
        "every filled subpath must retain its own serialized area witness",
    );
}

#[expect(
    clippy::expect_used,
    clippy::indexing_slicing,
    reason = "an exact two-page PDF anchors mandatory decimal coordinate widths"
)]
#[test]
fn future_page_coordinate_widths_preempt_invalid_earlier_ink() {
    let first = PdfVectorPath {
        commands: vec![
            PdfPathCommand::MoveTo(point(50_000, 40_000)),
            PdfPathCommand::LineTo(point(60_000, 50_000)),
        ],
        paint: stroke().paint,
    };
    let second = PdfVectorPath {
        commands: vec![
            PdfPathCommand::MoveTo(point(150_000, 180_000)),
            PdfPathCommand::LineTo(point(160_000, 190_000)),
        ],
        paint: stroke().paint,
    };
    let mut composition = PdfCompositionPlan {
        assets: Vec::new(),
        pages: vec![page(vec![first]), page(vec![second])],
        render_manifest_identity: "future-coordinate-widths.v1",
    };
    let accepted = serialize_vector_pdf(&composition, limits())
        .expect("both positive-coordinate strokes fit a full PDF");
    assert_xref_integrity(&accepted);
    assert_eq!(
        serialize_vector_pdf(&composition, PdfVectorLimits {
            maximum_pdf_bytes: accepted.len(),
            ..limits()
        }),
        Ok(accepted.clone()),
        "exact document budget still admits unchanged PDF bytes",
    );
    // This keeps both decimal widths unchanged but makes page 1's line
    // zero length. The fully known page-2 coordinates require more than
    // the generic five-byte minimum of each PDF number.
    composition.pages[0].vector_authority[0].commands[1] =
        PdfPathCommand::LineTo(point(50_000, 40_000));
    assert_eq!(
        serialize_vector_pdf(&composition, limits()),
        Err(PdfVectorError::InvalidPath),
        "enough bytes reveal the collapsed first-page stroke",
    );
    assert_eq!(
        serialize_vector_pdf(&composition, PdfVectorLimits {
            maximum_pdf_bytes: accepted.len().saturating_sub(1),
            ..limits()
        }),
        Err(PdfVectorError::ResourceLimit),
        "an unfinishable later page must reserve its real numeric syntax",
    );
}

#[expect(
    clippy::expect_used,
    clippy::indexing_slicing,
    reason = "a wide valid stroke anchors the PDF width decimal spelling"
)]
#[test]
fn future_page_stroke_width_digits_preempt_invalid_earlier_ink() {
    let first = PdfVectorPath {
        commands: vec![
            PdfPathCommand::MoveTo(point(50_000, 40_000)),
            PdfPathCommand::LineTo(point(60_000, 50_000)),
        ],
        paint: stroke().paint,
    };
    let later = PdfVectorPath {
        commands: vec![
            PdfPathCommand::MoveTo(point(100_000, 180_000)),
            PdfPathCommand::LineTo(point(110_000, 190_000)),
        ],
        paint: PdfPathPaint::Stroke {
            color: PdfRgb {
                red: 5,
                green: 22,
                blue: 240,
            },
            width_um: 100_000,
        },
    };
    let mut composition = PdfCompositionPlan {
        assets: Vec::new(),
        pages: vec![page(vec![first]), page(vec![later])],
        render_manifest_identity: "future-stroke-width-digits.v1",
    };
    let accepted = serialize_vector_pdf(&composition, limits())
        .expect("wide in-page stroke fits the complete document");
    assert_xref_integrity(&accepted);
    assert!(pdf_string(&accepted).contains("283.465 w\n"));
    assert_eq!(
        serialize_vector_pdf(&composition, PdfVectorLimits {
            maximum_pdf_bytes: accepted.len(),
            ..limits()
        }),
        Ok(accepted.clone()),
    );
    composition.pages[0].vector_authority[0].commands[1] =
        PdfPathCommand::LineTo(point(50_000, 40_000));
    assert_eq!(
        serialize_vector_pdf(&composition, limits()),
        Err(PdfVectorError::InvalidPath),
    );
    assert_eq!(
        serialize_vector_pdf(&composition, PdfVectorLimits {
            maximum_pdf_bytes: accepted.len().saturating_sub(1),
            ..limits()
        }),
        Err(PdfVectorError::ResourceLimit),
        "later stroke width digits cannot borrow missing framing bytes",
    );
}

#[expect(
    clippy::expect_used,
    clippy::indexing_slicing,
    reason = "fixed-coordinate oracle mutates one known valid page/path"
)]
#[test]
fn varied_pdf_decimal_widths_preserve_exact_budget_and_xref() {
    // Each selected x crosses or approaches a rounded PDF integer width,
    // from 0.000 to the supported two-metre page edge. The fill's cubic
    // checks the six-coordinate command as well as ordinary line syntax.
    for x in [
        0i32, 1i32, 352i32, 353i32, 354i32, 3_527i32, 3_528i32,
        3_529i32, 35_277i32, 35_278i32, 35_279i32, 100_000i32,
        999_999i32, 1_999_996i32,
    ] {
        let first = PdfVectorPath {
            commands: vec![
                PdfPathCommand::MoveTo(point(50_000, 40_000)),
                PdfPathCommand::LineTo(point(60_000, 50_000)),
            ],
            paint: stroke().paint,
        };
        let contour = PdfVectorPath {
            commands: vec![
                PdfPathCommand::MoveTo(point(x, 120_000)),
                PdfPathCommand::LineTo(point(x.saturating_add(1), 120_000)),
                PdfPathCommand::CubicTo(
                    point(x.saturating_add(2), 120_003),
                    point(x.saturating_add(3), 120_002),
                    point(x, 120_005),
                ),
                PdfPathCommand::Close,
            ],
            paint: PdfPathPaint::Fill {
                color: PdfRgb {
                    red: 0,
                    green: 0,
                    blue: 0,
                },
                rule: PdfFillRule::EvenOdd,
            },
        };
        let thick_stroke = PdfVectorPath {
            commands: vec![
                PdfPathCommand::MoveTo(point(1_000_000, 1_000_000)),
                PdfPathCommand::LineTo(point(1_010_000, 1_010_000)),
            ],
            paint: PdfPathPaint::Stroke {
                color: PdfRgb {
                    red: 5,
                    green: 22,
                    blue: 240,
                },
                width_um: 100_000,
            },
        };
        let mut large_page = page(vec![contour, thick_stroke]);
        large_page.physical_page_box = PdfVectorPageBox {
            width_um: 2_000_000,
            height_um: 2_000_000,
        };
        let mut composition = PdfCompositionPlan {
            assets: Vec::new(),
            pages: vec![page(vec![first]), large_page],
            render_manifest_identity: "varied-decimal-geometry.v1",
        };
        let accepted = serialize_vector_pdf(&composition, limits())
            .expect("bounded geometry must serialize at varied widths");
        assert_xref_integrity(&accepted);
        assert_eq!(
            serialize_vector_pdf(&composition, PdfVectorLimits {
                maximum_pdf_bytes: accepted.len(),
                ..limits()
            }),
            Ok(accepted.clone()),
            "x={x} exact output bytes remain sufficient",
        );
        composition.pages[0].vector_authority[0].commands[1] =
            PdfPathCommand::LineTo(point(50_000, 40_000));
        assert_eq!(
            serialize_vector_pdf(&composition, PdfVectorLimits {
                maximum_pdf_bytes: accepted.len().saturating_sub(1),
                ..limits()
            }),
            Err(PdfVectorError::ResourceLimit),
            "x={x} known future coordinates and width preempt bad ink",
        );
    }
}

#[expect(
    clippy::expect_used,
    clippy::indexing_slicing,
    reason = "a minimal two-page PDF anchors cross-page drawing syntax"
)]
#[test]
fn later_page_drawing_minima_refuse_before_earlier_invalid_geometry() {
    let tiny_stroke = PdfVectorPath {
        commands: vec![
            PdfPathCommand::MoveTo(point(1, 1)),
            PdfPathCommand::LineTo(point(2, 1)),
        ],
        paint: PdfPathPaint::Stroke {
            color: PdfRgb {
                red: 0,
                green: 0,
                blue: 0,
            },
            width_um: 1,
        },
    };
    // The last page's /Length changes from two to three and then from
    // three to four digits; every real command has minimum-length syntax.
    for (line_count, length_digits) in [
        (1usize, 2usize), (2, 3), (65, 3), (66, 4),
    ] {
        let mut later_stroke = tiny_stroke.clone();
        for index in 1..line_count {
            let x = if index & 1 == 0 { 2i32 } else { 1i32 };
            later_stroke
                .commands
                .push(PdfPathCommand::LineTo(point(x, 1)));
        }
        let mut composition = PdfCompositionPlan {
            assets: Vec::new(),
            pages: vec![
                page(vec![tiny_stroke.clone()]),
                page(vec![later_stroke]),
            ],
            render_manifest_identity: "cross-page-ink-framing.v1",
        };
        let accepted = serialize_vector_pdf(&composition, limits())
            .expect("two pages with minimal round strokes fit exactly");
        assert_xref_integrity(&accepted);
        let last_stream_bytes = pdf_string(&accepted)
            .split("6 0 obj\n<< /Length ")
            .nth(1)
            .expect("second page stream header")
            .split(" >>")
            .next()
            .expect("stream length")
            .parse::<usize>()
            .expect("decimal stream byte length");
        assert_eq!(last_stream_bytes, 79 + 14 * line_count);
        assert_eq!(last_stream_bytes.to_string().len(), length_digits);
        assert_eq!(
            serialize_vector_pdf(
                &composition,
                PdfVectorLimits {
                    maximum_pdf_bytes: accepted.len(),
                    ..limits()
                },
            ),
            Ok(accepted.clone()),
            "the two-page PDF fits at its exact byte limit",
        );
        composition.pages[0].vector_authority[0].commands[0] =
            PdfPathCommand::MoveTo(point(-1, 1));
        assert_eq!(
            serialize_vector_pdf(&composition, limits()),
            Err(PdfVectorError::InvalidPath),
            "with sufficient bytes malformed earlier geometry refuses",
        );
        assert_eq!(
            serialize_vector_pdf(
                &composition,
                PdfVectorLimits {
                    maximum_pdf_bytes: accepted.len().saturating_sub(1),
                    ..limits()
                },
            ),
            Err(PdfVectorError::ResourceLimit),
            "later page ink syntax and its length digits must fit first",
        );
    }
}

#[expect(
    clippy::expect_used,
    clippy::indexing_slicing,
    reason = "a small compound fill anchors mandatory later-page syntax",
)]
#[test]
fn later_page_fill_framing_preempts_earlier_invalid_stroke() {
    let first = PdfVectorPath {
        commands: vec![
            PdfPathCommand::MoveTo(point(1, 1)),
            PdfPathCommand::LineTo(point(2, 1)),
        ],
        paint: PdfPathPaint::Stroke {
            color: PdfRgb {
                red: 0,
                green: 0,
                blue: 0,
            },
            width_um: 1,
        },
    };
    let tiny_fill = PdfVectorPath {
        commands: vec![
            PdfPathCommand::MoveTo(point(0, 0)),
            PdfPathCommand::LineTo(point(1, 0)),
            PdfPathCommand::LineTo(point(0, 1)),
            PdfPathCommand::Close,
        ],
        paint: PdfPathPaint::Fill {
            color: PdfRgb {
                red: 0,
                green: 0,
                blue: 0,
            },
            rule: PdfFillRule::EvenOdd,
        },
    };
    let mut composition = PdfCompositionPlan {
        assets: Vec::new(),
        pages: vec![page(vec![first]), page(vec![tiny_fill])],
        render_manifest_identity: "later-fill-framing.v1",
    };
    let accepted = serialize_vector_pdf(&composition, limits())
        .expect("valid stroke and tiny triangular fill must serialize");
    assert_xref_integrity(&accepted);
    assert_eq!(
        serialize_vector_pdf(
            &composition,
            PdfVectorLimits {
                maximum_pdf_bytes: accepted.len(),
                ..limits()
            },
        ),
        Ok(accepted.clone()),
    );
    composition.pages[0].vector_authority[0].commands[0] =
        PdfPathCommand::MoveTo(point(-1, 1));
    assert_eq!(
        serialize_vector_pdf(&composition, limits()),
        Err(PdfVectorError::InvalidPath),
    );
    assert_eq!(
        serialize_vector_pdf(
            &composition,
            PdfVectorLimits {
                maximum_pdf_bytes: accepted.len().saturating_sub(1),
                ..limits()
            },
        ),
        Err(PdfVectorError::ResourceLimit),
        "the later filled contour cannot borrow bytes from invalid ink",
    );
}

#[expect(
    clippy::expect_used,
    clippy::indexing_slicing,
    reason = "an exactly fitting two-page fixture anchors future-page allowance"
)]
#[test]
fn earlier_page_cannot_spend_minimum_framing_reserved_for_later_pages() {
    let mut composition = PdfCompositionPlan {
        assets: Vec::new(),
        pages: vec![page(vec![stroke()]), page(Vec::new())],
        render_manifest_identity: "future-page-framing.v1",
    };
    let accepted = serialize_vector_pdf(&composition, limits())
        .expect("valid first-page ink and an empty second page fit");
    assert_xref_integrity(&accepted);
    assert_eq!(
        serialize_vector_pdf(
            &composition,
            PdfVectorLimits {
                maximum_pdf_bytes: accepted.len(),
                ..limits()
            },
        ),
        Ok(accepted.clone()),
        "the original complete PDF must fit at its exact byte limit",
    );
    // The first page's valid ink is already one byte too large once the
    // minimum second-page objects are reserved. Its additional bad command
    // must not mask the document's impossible byte budget.
    composition.pages[0].vector_authority[0]
        .commands
        .push(PdfPathCommand::LineTo(point(-1, 40_000)));
    assert_eq!(
        serialize_vector_pdf(
            &composition,
            PdfVectorLimits {
                maximum_pdf_bytes: accepted.len().saturating_sub(1),
                ..limits()
            },
        ),
        Err(PdfVectorError::ResourceLimit),
        "a future page's mandatory bytes cannot be used by earlier ink",
    );
}

#[expect(
    clippy::expect_used,
    clippy::indexing_slicing,
    reason = "bounded multi-page fixtures exercise exact object sizes"
)]
#[test]
fn future_page_framing_counts_variable_boxes_and_two_digit_object_numbers() {
    for drawn_index in [0usize, 6usize] {
        let mut pages = vec![page(Vec::new()); 12];
        for page in pages.iter_mut().step_by(2) {
            page.physical_page_box.width_um = 148_000;
            page.physical_page_box.height_um = 210_000;
        }
        pages[drawn_index].vector_authority.push(stroke());
        let mut composition = PdfCompositionPlan {
            assets: Vec::new(),
            pages,
            render_manifest_identity: "mixed-page-framing.v1",
        };
        let high_limits = PdfVectorLimits {
            maximum_pages: 12,
            maximum_pdf_bytes: 40_000,
            ..limits()
        };
        let accepted = serialize_vector_pdf(&composition, high_limits)
            .expect("mixed-size twelve-page PDF must serialize");
        assert_xref_integrity(&accepted);
        assert_eq!(
            serialize_vector_pdf(
                &composition,
                PdfVectorLimits {
                    maximum_pdf_bytes: accepted.len(),
                    ..high_limits
                }
            ),
            Ok(accepted.clone()),
            "all twelve pages must fit their exact combined byte budget",
        );
        composition.pages[drawn_index].vector_authority[0]
            .commands
            .push(PdfPathCommand::LineTo(point(-1, 40_000)));
        assert_eq!(
            serialize_vector_pdf(
                &composition,
                PdfVectorLimits {
                    maximum_pdf_bytes: accepted.len().saturating_sub(1),
                    ..high_limits
                }
            ),
            Err(PdfVectorError::ResourceLimit),
            "later pages and object numbers reserve their exact framing",
        );
    }
}

#[test]
fn oversized_page_stream_refuses_before_later_invalid_page() {
    let mut commands = vec![PdfPathCommand::MoveTo(point(20_000, 30_000))];
    for index in 1i32..100i32 {
        commands.push(PdfPathCommand::LineTo(point(
            20_000i32.saturating_add(index),
            30_000,
        )));
    }
    let first_page = page(vec![PdfVectorPath {
        commands,
        paint: PdfPathPaint::Stroke {
            color: PdfRgb {
                red: 5,
                green: 22,
                blue: 240,
            },
            width_um: 350,
        },
    }]);
    let mut invalid_later_page = page(Vec::new());
    invalid_later_page.physical_page_box.width_um = 0;
    let composition = PdfCompositionPlan {
        assets: Vec::new(),
        pages: vec![first_page, invalid_later_page],
        render_manifest_identity: "oversized-stream.v1",
    };
    // The first page alone exceeds 500 bytes. It must stop before the
    // invalid second page can change the outcome to InvalidPageBox.
    assert_eq!(
        serialize_vector_pdf(
            &composition,
            PdfVectorLimits {
                maximum_pdf_bytes: 500,
                ..limits()
            },
        ),
        Err(PdfVectorError::ResourceLimit),
        "oversized page content must refuse before later-page rendering",
    );
}

#[test]
fn final_pdf_overhead_refuses_before_later_invalid_page() {
    let mut invalid_later_page = page(Vec::new());
    invalid_later_page.physical_page_box.height_um = 0;
    let composition = PdfCompositionPlan {
        assets: Vec::new(),
        pages: vec![page(Vec::new()), invalid_later_page],
        render_manifest_identity: "bounded-output.v1",
    };
    // The two content streams fit 300 bytes in total, but the PDF header,
    // page tree and first page object do not. Refuse before rendering page 2.
    assert_eq!(
        serialize_vector_pdf(
            &composition,
            PdfVectorLimits {
                maximum_pdf_bytes: 300,
                ..limits()
            },
        ),
        Err(PdfVectorError::ResourceLimit),
        "final output budget must be enforced before later-page rendering",
    );
}

#[expect(
    clippy::expect_used,
    reason = "accepted PDF framing identifies the exact next-page boundary"
)]
#[test]
fn exhausted_output_budget_refuses_before_rendering_later_page() {
    let mut composition = PdfCompositionPlan {
        assets: Vec::new(),
        pages: vec![page(Vec::new()), page(Vec::new())],
        render_manifest_identity: "exhausted-before-next-page.v1",
    };
    let accepted = serialize_vector_pdf(&composition, limits())
        .expect("a pair of blank pages is valid");
    assert_eq!(
        serialize_vector_pdf(
            &composition,
            PdfVectorLimits {
                maximum_pdf_bytes: accepted.len(),
                ..limits()
            },
        ),
        Ok(accepted.clone()),
        "two-page PDF must fit its exact total byte budget",
    );
    let next_page_offset = accepted
        .windows(b"5 0 obj\n".len())
        .position(|bytes| bytes == b"5 0 obj\n")
        .expect("second page object follows the first page content");
    composition
        .pages
        .get_mut(1)
        .expect("the fixture has two pages")
        .physical_page_box
        .width_um = 0;
    assert_eq!(
        serialize_vector_pdf(
            &composition,
            PdfVectorLimits {
                maximum_pdf_bytes: next_page_offset,
                ..limits()
            },
        ),
        Err(PdfVectorError::ResourceLimit),
        "no room for another object must refuse before inspecting that page",
    );

    // A positive but insufficient remaining allowance must also stop a
    // later page before its trailing, independently invalid command.
    let mut commands = vec![PdfPathCommand::MoveTo(point(20_000, 30_000))];
    for index in 0i32..10i32 {
        commands.push(PdfPathCommand::LineTo(point(
            30_000i32.saturating_add(index),
            40_000,
        )));
    }
    commands.push(PdfPathCommand::LineTo(point(-1, 40_000)));
    let second_page = composition
        .pages
        .get_mut(1)
        .expect("the fixture has two pages");
    second_page.physical_page_box.width_um = 210_000;
    second_page.vector_authority = vec![PdfVectorPath {
        commands,
        paint: stroke().paint,
    }];
    assert_eq!(
        serialize_vector_pdf(
            &composition,
            PdfVectorLimits {
                maximum_pdf_bytes: next_page_offset.saturating_add(150),
                ..limits()
            },
        ),
        Err(PdfVectorError::ResourceLimit),
        "remaining stream budget must stop before malformed later geometry",
    );
}

#[expect(
    clippy::expect_used,
    reason = "a valid stream anchors the exact stroke operator byte budget"
)]
#[test]
fn path_paint_operator_budget_refuses_before_later_bad_segment() {
    let mut valid_stroke = stroke();
    valid_stroke.commands = vec![
        PdfPathCommand::MoveTo(point(20_000, 30_000)),
        PdfPathCommand::LineTo(point(50_000, 40_000)),
    ];
    let mut composition = PdfCompositionPlan {
        assets: Vec::new(),
        pages: vec![page(vec![valid_stroke])],
        render_manifest_identity: "reserve-stroke-operator.v1",
    };
    let bytes = serialize_vector_pdf(&composition, limits())
        .expect("the valid one-line stroke has an emitted operator");
    let page_start = bytes
        .windows(b"3 0 obj\n".len())
        .position(|record| record == b"3 0 obj\n")
        .expect("the first page object exists");
    let content = pdf_string(&bytes);
    let stream = content
        .split_once("stream\n")
        .expect("the stroke content stream exists")
        .1
        .split_once("endstream\n")
        .expect("the stroke content stream has a terminator")
        .0;
    let drawn = stream
        .strip_suffix("S\nQ\n")
        .expect("stroke and page operators end the stream");
    composition
        .pages
        .get_mut(0)
        .expect("the fixture has one page")
        .vector_authority
        .get_mut(0)
        .expect("the fixture has one stroke")
        .commands
        .push(PdfPathCommand::LineTo(point(-1, 40_000)));
    assert_eq!(
        serialize_vector_pdf(
            &composition,
            PdfVectorLimits {
                maximum_pdf_bytes: page_start.saturating_add(drawn.len())
                    .saturating_add(b"Q\n".len()),
                ..limits()
            },
        ),
        Err(PdfVectorError::ResourceLimit),
        "the missing final S operator must refuse before later bad geometry",
    );
}

#[expect(
    clippy::expect_used,
    clippy::indexing_slicing,
    reason = "the one-page fixture has a known offset and page index"
)]
#[test]
fn page_closing_operator_budget_refuses_before_invalid_paths() {
    let mut composition = PdfCompositionPlan {
        assets: Vec::new(),
        pages: vec![page(Vec::new())],
        render_manifest_identity: "reserve-page-closing-operator.v1",
    };
    let bytes = serialize_vector_pdf(&composition, limits())
        .expect("blank page fits the PDF budget");
    let page_start = bytes
        .windows(b"3 0 obj\n".len())
        .position(|record| record == b"3 0 obj\n")
        .expect("the first page object exists");
    let initial_stream = b"q\n1 0 0 -1 0 841.890 cm\n";
    composition.pages[0].vector_authority = vec![PdfVectorPath {
        commands: Vec::new(),
        paint: stroke().paint,
    }];
    for extra_byte in [0, 1] {
        let insufficient = PdfVectorLimits {
            maximum_pdf_bytes: page_start
                .saturating_add(initial_stream.len())
                .saturating_add(extra_byte),
            ..limits()
        };
        assert_eq!(
            serialize_vector_pdf(&composition, insufficient),
            Err(PdfVectorError::ResourceLimit),
            "a page that cannot close must refuse before malformed paths",
        );
    }
}

#[expect(
    clippy::expect_used,
    reason = "accepted PDF framing identifies the next-page byte boundary"
)]
#[test]
fn remaining_budget_smaller_than_page_header_refuses_before_path() {
    let mut composition = PdfCompositionPlan {
        assets: Vec::new(),
        pages: vec![page(Vec::new()), page(Vec::new())],
        render_manifest_identity: "smaller-than-page-header.v1",
    };
    let accepted = serialize_vector_pdf(&composition, limits())
        .expect("both empty pages fit the baseline PDF budget");
    let next_page_offset = accepted
        .windows(b"5 0 obj\n".len())
        .position(|bytes| bytes == b"5 0 obj\n")
        .expect("second page object exists");
    let mut invalid_stroke = stroke();
    invalid_stroke.commands.clear();
    composition
        .pages
        .get_mut(1)
        .expect("the fixture has two pages")
        .vector_authority = vec![invalid_stroke];

    assert_eq!(
        serialize_vector_pdf(
            &composition,
            PdfVectorLimits {
                maximum_pdf_bytes: next_page_offset.saturating_add(1),
                ..limits()
            },
        ),
        Err(PdfVectorError::ResourceLimit),
        "the page header alone is too large to inspect any path",
    );
}

#[expect(
    clippy::expect_used,
    reason = "real PDF objects anchor the page-framing byte boundary"
)]
#[test]
fn page_object_framing_refuses_before_invalid_path() {
    let mut composition = PdfCompositionPlan {
        assets: Vec::new(),
        pages: vec![page(Vec::new())],
        render_manifest_identity: "page-object-budget.v1",
    };
    let accepted = serialize_vector_pdf(&composition, limits())
        .expect("a blank page produces inspectable PDF object framing");
    let page_start = accepted
        .windows(b"3 0 obj\n".len())
        .position(|part| part == b"3 0 obj\n")
        .expect("the page object exists");
    let content_start = accepted
        .windows(b"4 0 obj\n".len())
        .position(|part| part == b"4 0 obj\n")
        .expect("the content object follows the page object");
    let page_header_len = content_start.saturating_sub(page_start);
    let mut invalid = stroke();
    invalid.commands.clear();
    composition
        .pages
        .get_mut(0)
        .expect("the fixture has one page")
        .vector_authority
        .push(invalid);
    let minimum_stream = b"q\n1 0 0 -1 0 841.890 cm\nQ\n";
    let content_header =
        format!("4 0 obj\n<< /Length {} >>\nstream\n", minimum_stream.len());
    let minimum_framing = page_header_len
        .saturating_add(content_header.len())
        .saturating_add(minimum_stream.len())
        .saturating_add(b"endstream\nendobj\n".len());
    for allowance in [
        page_header_len.saturating_sub(1),
        minimum_framing.saturating_sub(1),
    ] {
        assert_eq!(
            serialize_vector_pdf(&composition, PdfVectorLimits {
                maximum_pdf_bytes: page_start.saturating_add(allowance),
                ..limits()
            },),
            Err(PdfVectorError::ResourceLimit),
            "mandatory PDF framing refuses before invalid paths",
        );
    }
}

#[expect(
    clippy::expect_used,
    clippy::indexing_slicing,
    reason = "fixture offsets expose PDF /Length digit boundaries"
)]
#[test]
fn stream_length_digit_growth_reserves_dictionary_bytes_before_invalid_ink() {
    for (line_count, minimum_size, maximum_size, expected_growth) in [
        (6i32, 100usize, 1_000usize, 1usize),
        (80i32, 1_000usize, 10_000usize, 2usize),
    ] {
        let mut commands = vec![PdfPathCommand::MoveTo(point(20_000, 30_000))];
        for index in 1i32..line_count {
            commands.push(PdfPathCommand::LineTo(point(
                20_000i32.saturating_add(index.saturating_mul(1_000)),
                30_000,
            )));
        }
        let mut composition = PdfCompositionPlan {
            assets: Vec::new(),
            pages: vec![page(vec![PdfVectorPath {
                commands,
                paint: stroke().paint,
            }])],
            render_manifest_identity: "length-digit-boundary.v1",
        };
        let accepted = serialize_vector_pdf(&composition, limits())
            .expect("valid stroke crosses target /Length boundary");
        let content_start = accepted
            .windows(b"4 0 obj\n".len())
            .position(|part| part == b"4 0 obj\n")
            .expect("first content object is present");
        let stream_start = accepted
            .windows(b"stream\n".len())
            .position(|part| part == b"stream\n")
            .expect("stream keyword is present")
            .saturating_add(b"stream\n".len());
        let stream_end = accepted
            .windows(b"endstream\n".len())
            .position(|part| part == b"endstream\n")
            .expect("stream terminator is present");
        let stream_len = stream_end.saturating_sub(stream_start);
        assert!(
            (minimum_size..maximum_size).contains(&stream_len),
            "fixture must cross the targeted /Length digit boundary",
        );
        let content_header_len = stream_start.saturating_sub(content_start);
        let minimum_header_len = format!(
            "4 0 obj\n<< /Length {} >>\nstream\n",
            b"q\n1 0 0 -1 0 841.890 cm\nQ\n".len(),
        )
        .len();
        assert_eq!(
            content_header_len,
            minimum_header_len.saturating_add(expected_growth),
            "/Length field expands by the exact new digit count",
        );
        let one_byte_short = PdfVectorLimits {
            maximum_pdf_bytes: stream_end
                .saturating_add(b"endstream\nendobj\n".len())
                .saturating_sub(1),
            ..limits()
        };
        assert_eq!(
            serialize_vector_pdf(&composition, one_byte_short),
            Err(PdfVectorError::ResourceLimit),
            "one byte less than the real stream object refuses",
        );
        composition.pages[0].vector_authority[0]
            .commands
            .push(PdfPathCommand::LineTo(point(-1, 30_000)));
        assert_eq!(
            serialize_vector_pdf(&composition, one_byte_short),
            Err(PdfVectorError::ResourceLimit),
            "grown /Length must refuse before invalid trailing ink",
        );
    }
}

#[expect(
    clippy::expect_used,
    clippy::indexing_slicing,
    reason = "real PDF bytes anchor the final metadata and xref budget"
)]
#[test]
fn final_document_framing_is_reserved_before_invalid_trailing_ink() {
    let mut composition = PdfCompositionPlan {
        assets: Vec::new(),
        pages: vec![page(Vec::new()), page(vec![stroke()])],
        render_manifest_identity: "document-tail-budget.v1",
    };
    let accepted = serialize_vector_pdf(&composition, limits())
        .expect("the two-page PDF has a complete Info and xref trailer");
    assert_xref_integrity(&accepted);
    let exact = PdfVectorLimits {
        maximum_pdf_bytes: accepted.len(),
        ..limits()
    };
    assert_eq!(
        serialize_vector_pdf(&composition, exact),
        Ok(accepted.clone()),
        "the full-document byte budget still admits the exact PDF",
    );
    // The original drawing fits; only the mandatory Info/xref/trailer
    // framing is one byte short. Invalid geometry appended *after* all
    // original drawing must not mask that unfinishable document budget.
    composition.pages[1].vector_authority[0]
        .commands
        .push(PdfPathCommand::LineTo(point(-1, 40_000)));
    assert_eq!(
        serialize_vector_pdf(&composition, PdfVectorLimits {
            maximum_pdf_bytes: accepted.len().saturating_sub(1),
            ..limits()
        }),
        Err(PdfVectorError::ResourceLimit),
        "mandatory final PDF objects refuse before trailing invalid ink",
    );
}

#[expect(
    clippy::expect_used,
    clippy::indexing_slicing,
    reason = "xref offsets and invalid trailing geometry are fixture-owned"
)]
#[test]
fn final_document_budget_tracks_startxref_decimal_growth() {
    let fixtures = [(1i32, 3usize), (40i32, 4usize), (800i32, 5usize)];
    for (line_count, expected_digits) in fixtures {
        let mut commands = vec![PdfPathCommand::MoveTo(point(20_000, 30_000))];
        for index in 1i32..=line_count {
            commands.push(PdfPathCommand::LineTo(point(
                20_000i32.saturating_add(index.saturating_mul(100)),
                40_000,
            )));
        }
        let mut composition = PdfCompositionPlan {
            assets: Vec::new(),
            pages: vec![page(vec![PdfVectorPath {
                commands,
                paint: stroke().paint,
            }])],
            render_manifest_identity: "startxref-digits.v1",
        };
        let high_limits = PdfVectorLimits {
            maximum_commands_per_page: 900,
            maximum_pdf_bytes: 40_000,
            ..limits()
        };
        let accepted = serialize_vector_pdf(&composition, high_limits)
            .expect("startxref digit fixture is a valid PDF");
        assert_xref_integrity(&accepted);
        let xref_offset = accepted
            .windows(b"xref\n".len())
            .position(|window| window == b"xref\n")
            .expect("xref keyword exists");
        assert_eq!(
            xref_offset.to_string().len(),
            expected_digits,
            "the fixture crosses the selected startxref decimal width",
        );
        let exact = PdfVectorLimits {
            maximum_pdf_bytes: accepted.len(),
            ..high_limits
        };
        assert_eq!(
            serialize_vector_pdf(&composition, exact),
            Ok(accepted.clone()),
            "new startxref digit width must fit the exact document budget",
        );
        composition.pages[0].vector_authority[0]
            .commands
            .push(PdfPathCommand::LineTo(point(-1, 40_000)));
        assert_eq!(
            serialize_vector_pdf(&composition, PdfVectorLimits {
                maximum_pdf_bytes: accepted.len().saturating_sub(1),
                ..high_limits
            }),
            Err(PdfVectorError::ResourceLimit),
            "one missing final byte must preempt later invalid geometry",
        );
    }
}

#[expect(
    clippy::expect_used,
    reason = "the exact PDF page ceiling must emit a valid page tree"
)]
#[test]
fn maximum_page_count_keeps_every_page_and_xref_entry() {
    let mut composition = PdfCompositionPlan {
        assets: Vec::new(),
        pages: vec![page(Vec::new()); 1024],
        render_manifest_identity: "maximum-page-count.v1",
    };
    let high_limits = PdfVectorLimits {
        maximum_pages: 1025,
        maximum_pdf_bytes: 1_000_000,
        ..limits()
    };
    let bytes = serialize_vector_pdf(&composition, high_limits)
        .expect("exactly 1024 blank pages fit the serializer safety ceiling");
    assert_xref_integrity(&bytes);
    let text = pdf_string(&bytes);
    assert!(text.contains("/Count 1024"), "page tree counts all pages");
    assert!(text.contains("/Size 2052"), "xref includes all page objects");
    assert_eq!(
        text.matches("/Type /Page /Parent 2 0 R").count(),
        1024,
        "every admitted page must have one exact ordered PDF object",
    );
    composition.pages.push(page(Vec::new()));
    assert_eq!(
        serialize_vector_pdf(&composition, high_limits),
        Err(PdfVectorError::ResourceLimit),
        "the hard 1024-page ceiling overrides a higher caller page limit",
    );
}

#[expect(
    clippy::expect_used,
    reason = "the accepted large stream must have inspectable PDF framing"
)]
#[test]
fn large_page_stream_preserves_exact_length_and_xref_without_truncation() {
    let mut commands = vec![PdfPathCommand::MoveTo(point(20_000, 30_000))];
    for index in 0i32..10_000i32 {
        commands.push(PdfPathCommand::LineTo(point(
            20_001i32.saturating_add(index),
            30_000,
        )));
    }
    let composition = PdfCompositionPlan {
        assets: Vec::new(),
        pages: vec![
            page(vec![PdfVectorPath {
                commands,
                paint: PdfPathPaint::Stroke {
                    color: PdfRgb {
                        red: 5,
                        green: 22,
                        blue: 240,
                    },
                    width_um: 350,
                },
            }]),
            page(Vec::new()),
        ],
        render_manifest_identity: "large-stream.v1",
    };
    let large_limits = PdfVectorLimits {
        maximum_commands_per_page: 10_001,
        maximum_pdf_bytes: 400_000,
        ..limits()
    };
    let bytes = serialize_vector_pdf(&composition, large_limits)
        .expect("10,001-command stream fits its admitted byte budget");
    assert_xref_integrity(&bytes);
    assert_stream_length_integrity(&bytes, 2);
    let pdf = pdf_string(&bytes);
    let stream = pdf
        .split_once("stream\n")
        .expect("first PDF stream exists")
        .1
        .split_once("endstream\n")
        .expect("first PDF stream terminates")
        .0;
    assert!(stream.len() > 100_000, "fixture must have a large stream");
    assert_eq!(
        stream.matches(" l\n").count(),
        10_000,
        "all emitted segments must survive the stream object boundary",
    );
    assert!(
        pdf.contains(&format!("/Length {} >>\nstream\n", stream.len())),
        "PDF /Length must count the exact verbatim content stream",
    );
    assert_eq!(
        serialize_vector_pdf(
            &composition,
            PdfVectorLimits {
                maximum_pdf_bytes: bytes.len().saturating_sub(1),
                ..large_limits
            },
        ),
        Err(PdfVectorError::ResourceLimit),
        "a one-byte-short output budget still refuses the whole PDF",
    );
}

#[test]
fn serialized_stroke_edges_must_not_escape_rounded_pdf_page() {
    // The PDF codec emits points and line width to three decimal places.
    // Physical half-width admission alone permits an edge center whose
    // independently rounded PDF width clips by half a millipoint.
    let cases = [
        (8, point(4, 1_000), point(1_000, 1_000), false, false),
        (8, point(1_000, 4), point(1_000, 1_000), false, false),
        (
            130,
            point(296_935, 1_000),
            point(296_935, 2_000),
            true,
            false,
        ),
        (
            130,
            point(1_000, 296_935),
            point(2_000, 296_935),
            false,
            false,
        ),
        (8, point(5, 1_000), point(1_000, 1_000), false, true),
        (8, point(1_000, 5), point(1_000, 1_000), false, true),
        (
            130,
            point(296_934, 1_000),
            point(296_934, 2_000),
            true,
            true,
        ),
        (
            130,
            point(1_000, 296_934),
            point(2_000, 296_934),
            false,
            true,
        ),
    ];
    for (width_um, start, end, landscape, admitted) in cases {
        let mut composition = document();
        let mut test_page = page(vec![PdfVectorPath {
            commands: vec![
                PdfPathCommand::MoveTo(start),
                PdfPathCommand::LineTo(end),
            ],
            paint: PdfPathPaint::Stroke {
                color: PdfRgb {
                    red: 0,
                    green: 0,
                    blue: 0,
                },
                width_um,
            },
        }]);
        if landscape {
            test_page.physical_page_box.width_um = 297_000;
        }
        composition.pages = vec![test_page];
        let outcome = serialize_vector_pdf(&composition, limits());
        if admitted {
            assert!(
                outcome.is_ok(),
                "a one-micrometer clearance must preserve the vector stroke",
            );
        } else {
            assert_eq!(
                outcome,
                Err(PdfVectorError::InvalidPath),
                "rounded PDF {width_um}um stroke must not clip the page",
            );
        }
    }
}

// Independently parse the same decimal point units emitted by a PDF reader,
// rather than relying on the serializer's integer fixed-point rounding code.
#[expect(
    clippy::expect_used,
    reason = "the oracle parses bounded fixed-three-decimal PDF units",
)]
fn emitted_pdf_millipoints(value_um: u32) -> u64 {
    format!("{:.3}", f64::from(value_um) * (72.0 / 25_400.0))
        .replace('.', "")
        .parse::<u64>()
        .expect("PDF decimal number is a nonnegative millipoint integer")
}

fn oracle_stroke_point_inside(
    x: u32, y: u32, width_um: u32, page_box: PdfVectorPageBox,
) -> bool {
    let margin = width_um.div_ceil(2);
    if x < margin || y < margin
        || x > page_box.width_um.saturating_sub(margin)
        || y > page_box.height_um.saturating_sub(margin)
    {
        return false;
    }
    let width = emitted_pdf_millipoints(width_um);
    let x_pdf = emitted_pdf_millipoints(x);
    let y_pdf = emitted_pdf_millipoints(y);
    let page_width = emitted_pdf_millipoints(page_box.width_um);
    let page_height = emitted_pdf_millipoints(page_box.height_um);
    width <= x_pdf.min(page_width.saturating_sub(x_pdf)).saturating_mul(2)
        && width <= y_pdf.min(page_height.saturating_sub(y_pdf))
            .saturating_mul(2)
}

#[expect(
    clippy::expect_used,
    reason = "the bounded oracle coordinates are representable as i32",
)]
#[test]
fn exhaustive_rounded_stroke_edge_clearance_oracle() {
    let page_box = PdfVectorPageBox {
        width_um: 210_000,
        height_um: 297_000,
    };
    let mut widths: Vec<u32> = (1u32..=256u32).collect();
    widths.extend([350, 700, 1_500, 1_999, 2_000]);
    let mut checked = 0usize;
    for width_um in widths {
        let margin = width_um.div_ceil(2);
        for clearance in [
            margin.saturating_sub(1), margin, margin.saturating_add(1),
            margin.saturating_add(2),
        ] {
            for edge in 0u8..4u8 {
                let (x, y, end_x, end_y) = match edge {
                    0 => (clearance, 20_000, clearance, 30_000),
                    1 => (
                        page_box.width_um.saturating_sub(clearance),
                        20_000,
                        page_box.width_um.saturating_sub(clearance),
                        30_000,
                    ),
                    2 => (20_000, clearance, 30_000, clearance),
                    _ => (
                        20_000,
                        page_box.height_um.saturating_sub(clearance),
                        30_000,
                        page_box.height_um.saturating_sub(clearance),
                    ),
                };
                let mut test_page = page(vec![PdfVectorPath {
                    commands: vec![
                        PdfPathCommand::MoveTo(point(
                            i32::try_from(x).expect("bounded x"),
                            i32::try_from(y).expect("bounded y"),
                        )),
                        PdfPathCommand::LineTo(point(
                            i32::try_from(end_x).expect("bounded end x"),
                            i32::try_from(end_y).expect("bounded end y"),
                        )),
                    ],
                    paint: PdfPathPaint::Stroke {
                        color: PdfRgb {
                            red: 0, green: 0, blue: 0,
                        },
                        width_um,
                    },
                }]);
                test_page.physical_page_box = page_box;
                let composition = PdfCompositionPlan {
                    assets: Vec::new(),
                    pages: vec![test_page],
                    render_manifest_identity: "rounded-edge-sweep.v1",
                };
                let accepted = oracle_stroke_point_inside(
                    x, y, width_um, page_box,
                ) && oracle_stroke_point_inside(
                    end_x, end_y, width_um, page_box,
                );
                let result = serialize_vector_pdf(&composition, limits());
                if accepted {
                    assert!(result.is_ok(),
                        "width {width_um}, edge {edge}, clearance {clearance}");
                } else {
                    assert_eq!(result, Err(PdfVectorError::InvalidPath),
                        "width {width_um}, edge {edge}, clearance {clearance}");
                }
                checked = checked.saturating_add(1);
            }
        }
    }
    assert_eq!(checked, 4_176, "all four-edge cases were exercised");
}

#[expect(
    clippy::expect_used,
    reason = "blank PDF establishes a budget with no room for stroke paint"
)]
#[test]
fn impossible_stroke_paint_framing_refuses_before_invalid_width() {
    let mut composition = PdfCompositionPlan {
        assets: Vec::new(),
        pages: vec![page(Vec::new())],
        render_manifest_identity: "stroke-width-paint-budget.v1",
    };
    let blank = serialize_vector_pdf(&composition, limits())
        .expect("a blank page fits and establishes its exact byte length");
    for width_um in [0, 2_000_001] {
        composition.pages = vec![page(vec![PdfVectorPath {
            commands: stroke().commands,
            paint: PdfPathPaint::Stroke {
                color: PdfRgb {
                    red: 5,
                    green: 22,
                    blue: 240,
                },
                width_um,
            },
        }])];
        assert_eq!(
            serialize_vector_pdf(&composition, limits()),
            Err(PdfVectorError::InvalidStrokeWidth),
            "an out-of-range stroke width is invalid with adequate room",
        );
        assert_eq!(
            serialize_vector_pdf(
                &composition,
                PdfVectorLimits {
                    maximum_pdf_bytes: blank.len(),
                    ..limits()
                },
            ),
            Err(PdfVectorError::ResourceLimit),
            "no paint can fit, regardless of the invalid stroke width",
        );
    }
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

const fn matrix_expected_error(
    resource_case: usize,
    identity_case: usize,
    preserved_text: bool,
    box_case: usize,
    path_case: usize,
) -> Option<PdfVectorError> {
    if resource_case != 0 {
        Some(PdfVectorError::UnsupportedAssets)
    } else if identity_case == 1 {
        Some(PdfVectorError::InvalidManifestIdentity)
    } else if preserved_text {
        Some(PdfVectorError::UnsupportedSearchableText)
    } else if box_case >= 2 {
        Some(PdfVectorError::InvalidPageBox)
    } else if path_case == 2 {
        Some(PdfVectorError::InvalidPath)
    } else {
        None
    }
}

#[expect(
    clippy::expect_used,
    reason = "a successful synthetic fixture must return inspectable PDF bytes"
)]
#[test]
fn complete_compact_resource_text_box_path_and_identity_matrix() {
    let mut checked = 0usize;
    let mut accepted = 0usize;
    for box_case in 0usize..4 {
        for preserved_text in [false, true] {
            for resource_case in 0usize..3 {
                for path_case in 0usize..3 {
                    for identity_case in 0usize..2 {
                        let mut painted_path = if path_case == 0 {
                            stroke()
                        } else {
                            fill()
                        };
                        if path_case == 2 {
                            painted_path.commands =
                                vec![PdfPathCommand::LineTo(point(
                                    30_000, 50_000,
                                ))];
                        }
                        let mut page_value = page(vec![painted_path]);
                        page_value.physical_page_box = match box_case {
                            0 => PdfVectorPageBox {
                                width_um: 210_000,
                                height_um: 297_000,
                            },
                            1 => PdfVectorPageBox {
                                width_um: 215_900,
                                height_um: 279_400,
                            },
                            2 => PdfVectorPageBox {
                                width_um: 0,
                                height_um: 297_000,
                            },
                            _ => PdfVectorPageBox {
                                width_um: 2_000_001,
                                height_um: 297_000,
                            },
                        };
                        if preserved_text {
                            page_value.searchable_text =
                                SearchableSemanticText::Preserved(());
                        }
                        let mut composition = document();
                        composition.pages = vec![page_value];
                        if resource_case != 0 {
                            composition.assets = vec![PdfAssetResource {
                                asset_identity: "resource-1",
                                disposition: if resource_case == 1 {
                                    PdfAssetDisposition::Embedded
                                } else {
                                    PdfAssetDisposition::SafelyReferenced
                                },
                            }];
                        }
                        if identity_case == 1 {
                            composition.render_manifest_identity =
                                "invalid) /OpenAction (unsafe";
                        }
                        let expected_error = matrix_expected_error(
                            resource_case,
                            identity_case,
                            preserved_text,
                            box_case,
                            path_case,
                        );
                        let result =
                            serialize_vector_pdf(&composition, limits());
                        if let Some(error) = expected_error {
                            assert_eq!(
                                result,
                                Err(error),
                                "invalid matrix state must refuse",
                            );
                        } else {
                            let bytes =
                                result.expect("complete fixture emits PDF");
                            assert_xref_integrity(&bytes);
                            assert_eq!(
                                serialize_vector_pdf(&composition, limits()),
                                Ok(bytes.clone()),
                                "admitted combination must be deterministic",
                            );
                            let expected_box = if box_case == 1 {
                                "/MediaBox [0 0 612.000 792.000]"
                            } else {
                                "/MediaBox [0 0 595.276 841.890]"
                            };
                            assert!(
                                pdf_string(&bytes).contains(expected_box),
                                "physical page box retained exactly",
                            );
                            accepted = accepted.saturating_add(1);
                        }
                        checked = checked.saturating_add(1);
                    }
                }
            }
        }
    }
    assert_eq!(checked, 144, "matrix must cover 144 combinations");
    assert_eq!(accepted, 4, "only four combinations are admissible");
}

#[expect(
    clippy::expect_used,
    reason = "admitted metadata identities must produce inspectable PDF bytes"
)]
#[test]
fn manifest_identity_ascii_admission_and_length_boundaries() {
    let mut checked = 0usize;
    let mut admitted = 0usize;
    for byte in 0u8..=127u8 {
        let identity = char::from(byte).to_string();
        let composition = PdfCompositionPlan {
            assets: Vec::new(),
            pages: vec![page(Vec::new())],
            render_manifest_identity: identity.as_str(),
        };
        let permitted = byte.is_ascii_alphanumeric()
            || matches!(byte, b'-' | b'_' | b'.');
        let result = serialize_vector_pdf(&composition, limits());
        if permitted {
            let bytes = result.expect("admitted ASCII identity must serialize");
            let expected = format!("/AtramentManifestID ({identity})");
            assert!(
                pdf_string(&bytes).contains(&expected),
                "ASCII byte {byte} must retain its exact metadata spelling",
            );
            assert_xref_integrity(&bytes);
            admitted = admitted.saturating_add(1);
        } else {
            assert_eq!(
                result,
                Err(PdfVectorError::InvalidManifestIdentity),
                "ASCII byte {byte} must not escape the PDF Info string",
            );
        }
        checked = checked.saturating_add(1);
    }
    assert_eq!(checked, 128, "every ASCII byte has a classification");
    assert_eq!(admitted, 65, "only ASCII alphanumeric, -, _, . are safe");

    for (identity, is_valid) in [
        ("A".repeat(127), true),
        ("A".repeat(128), true),
        ("A".repeat(129), false),
        (String::from("\u{f1}"), false),
        (String::new(), false),
    ] {
        let composition = PdfCompositionPlan {
            assets: Vec::new(),
            pages: vec![page(Vec::new())],
            render_manifest_identity: identity.as_str(),
        };
        let result = serialize_vector_pdf(&composition, limits());
        if is_valid {
            let bytes = result.expect("bounded identity must serialize");
            assert!(pdf_string(&bytes).contains(&format!(
                "/AtramentManifestID ({identity})"
            )));
            assert_xref_integrity(&bytes);
            assert_eq!(serialize_vector_pdf(&composition, limits()), Ok(bytes));
        } else {
            assert_eq!(result, Err(PdfVectorError::InvalidManifestIdentity));
        }
    }
}

#[test]
fn later_page_authority_refuses_before_earlier_malformed_path() {
    // A vector-only export cannot quietly skip a later page's semantic text
    // or physical page box because an earlier path is already malformed.
    let mut malformed = stroke();
    malformed.commands = vec![PdfPathCommand::LineTo(point(40_000, 50_000))];
    let first = page(vec![malformed]);
    let mut second = page(vec![stroke()]);
    second.searchable_text = SearchableSemanticText::Preserved(());
    let mut composition = PdfCompositionPlan {
        assets: Vec::new(),
        pages: vec![first, second],
        render_manifest_identity: "later-page-authority.v1",
    };
    assert_eq!(
        serialize_vector_pdf(&composition, limits()),
        Err(PdfVectorError::UnsupportedSearchableText),
        "unsupported later semantic text must be reported before earlier ink",
    );
    assert_eq!(composition.pages.len(), 2, "two-page test fixture");
    for later in composition.pages.iter_mut().skip(1) {
        later.searchable_text = SearchableSemanticText::OmittedAsIncompatible;
    }
    for invalid_size in [0, 2_000_001] {
        for later in composition.pages.iter_mut().skip(1) {
            later.physical_page_box.width_um = invalid_size;
        }
        assert_eq!(
            serialize_vector_pdf(&composition, limits()),
            Err(PdfVectorError::InvalidPageBox),
            "invalid later page width must preempt earlier malformed ink",
        );
        for later in composition.pages.iter_mut().skip(1) {
            later.physical_page_box.width_um = 210_000;
            later.physical_page_box.height_um = invalid_size;
        }
        assert_eq!(
            serialize_vector_pdf(&composition, limits()),
            Err(PdfVectorError::InvalidPageBox),
            "invalid later page height must preempt earlier malformed ink",
        );
        for later in composition.pages.iter_mut().skip(1) {
            later.physical_page_box.height_um = 297_000;
        }
    }
    assert_eq!(
        serialize_vector_pdf(&composition, limits()),
        Err(PdfVectorError::InvalidPath),
        "without a later authority error the earlier malformed ink refuses",
    );
}

#[expect(
    clippy::expect_used,
    reason = "bounded maximum PDF page tree must be fully inspectable"
)]
#[test]
fn maximum_supported_page_tree_retains_all_page_streams_and_offsets() {
    // Exercise the hard 1024-page serializer ceiling, not only caller budgets.
    // Object IDs cross four digits and the cross-reference table grows past
    // 2000 entries while every page remains a separate physical page.
    let composition = PdfCompositionPlan {
        assets: Vec::new(),
        pages: (0usize..1024)
            .map(|index| {
                let mut sheet = page(Vec::new());
                if index & 1 == 1 {
                    sheet.physical_page_box = PdfVectorPageBox {
                        width_um: 215_900,
                        height_um: 279_400,
                    };
                }
                sheet
            })
            .collect(),
        render_manifest_identity: "maximum-page-tree.v1",
    };
    let extended = PdfVectorLimits {
        maximum_pages: 1024,
        maximum_pdf_bytes: 1_000_000,
        ..limits()
    };
    let bytes = serialize_vector_pdf(&composition, extended)
        .expect("1024 blank physical PDF pages must serialize");
    assert_xref_integrity(&bytes);
    assert_stream_length_integrity(&bytes, 1024);
    let text = pdf_string(&bytes);
    assert!(text.contains("/Count 1024"));
    assert_eq!(text.matches("/Type /Page /Parent").count(), 1024);
    assert_eq!(text.matches("/MediaBox [0 0 595.276 841.890]").count(), 512);
    assert_eq!(text.matches("/MediaBox [0 0 612.000 792.000]").count(), 512);
    assert_eq!(text.matches("stream\nq\n").count(), 1024);
    assert_eq!(
        serialize_vector_pdf(
            &composition,
            PdfVectorLimits {
                maximum_pdf_bytes: bytes.len(),
                ..extended
            },
        ),
        Ok(bytes.clone()),
        "the maximum bounded document must fit exactly",
    );
    assert_eq!(
        serialize_vector_pdf(
            &composition,
            PdfVectorLimits {
                maximum_pdf_bytes: bytes.len().saturating_sub(1),
                ..extended
            },
        ),
        Err(PdfVectorError::ResourceLimit),
    );
    assert_eq!(
        serialize_vector_pdf(
            &composition,
            PdfVectorLimits {
                maximum_pages: 1023,
                ..extended
            },
        ),
        Err(PdfVectorError::ResourceLimit),
    );
    let mut too_many = composition;
    too_many.pages.push(page(Vec::new()));
    assert_eq!(
        serialize_vector_pdf(
            &too_many,
            PdfVectorLimits {
                maximum_pages: 1025,
                ..extended
            },
        ),
        Err(PdfVectorError::ResourceLimit),
        "the hard 1024-page limit cannot be bypassed by caller limits",
    );
}

#[expect(
    clippy::expect_used,
    reason = "the bounded miniature-page oracle uses representable micrometers"
)]
#[test]
fn rounded_stroke_clearance_oracle_covers_miniature_and_maximum_page_boxes() {
    // Clipping is relative to the serialized page box, not only A4. Include
    // physical dimensions smaller than the pen width and both digit edges.
    let dimensions = [
        1u32, 2, 3, 4, 5, 7, 8, 9, 10, 15, 16, 17, 31, 32, 33,
        63, 64, 65, 126, 127, 128, 129, 255, 256, 257, 350,
        999, 1_000, 1_001, 10_000, 100_000, 1_999_999, 2_000_000,
    ];
    let widths = [1u32, 2, 3, 4, 5, 7, 8, 9, 15, 16, 31, 32, 65, 130, 350];
    let mut checked = 0usize;
    let mut admitted = 0usize;
    for dimension in dimensions {
        for width_um in widths {
            let margin = width_um.div_ceil(2);
            for clearance in [
                margin.saturating_sub(1), margin, margin.saturating_add(1),
            ] {
                for edge in 0u8..4u8 {
                    let box_size = if edge < 2 {
                        PdfVectorPageBox {
                            width_um: dimension,
                            height_um: 1_000,
                        }
                    } else {
                        PdfVectorPageBox {
                            width_um: 1_000,
                            height_um: dimension,
                        }
                    };
                    let start = match edge {
                        0 => (clearance, 400),
                        1 => (dimension.saturating_sub(clearance), 400),
                        2 => (400, clearance),
                        _ => (400, dimension.saturating_sub(clearance)),
                    };
                    let end = if edge < 2 {
                        (start.0, 600)
                    } else {
                        (600, start.1)
                    };
                    let test_page = PdfPagePlan {
                        color_intent: PdfVectorColorIntent::DeviceRgb,
                        physical_page_box: box_size,
                        searchable_text:
                            SearchableSemanticText::OmittedAsIncompatible,
                        vector_authority: vec![PdfVectorPath {
                            commands: vec![
                                PdfPathCommand::MoveTo(point(
                                    i32::try_from(start.0).expect("bounded x"),
                                    i32::try_from(start.1).expect("bounded y"),
                                )),
                                PdfPathCommand::LineTo(point(
                                    i32::try_from(end.0).expect("bounded x"),
                                    i32::try_from(end.1).expect("bounded y"),
                                )),
                            ],
                            paint: PdfPathPaint::Stroke {
                                color: PdfRgb {
                                    red: 0, green: 0, blue: 0,
                                },
                                width_um,
                            },
                        }],
                    };
                    let composition = PdfCompositionPlan {
                        assets: Vec::new(),
                        pages: vec![test_page],
                        render_manifest_identity: "miniature-page-edge.v1",
                    };
                    let should_accept = oracle_stroke_point_inside(
                        start.0, start.1, width_um, box_size,
                    ) && oracle_stroke_point_inside(
                        end.0, end.1, width_um, box_size,
                    );
                    let result = serialize_vector_pdf(&composition, limits());
                    if should_accept {
                        let bytes = result.expect("rounded stroke must fit");
                        assert_xref_integrity(&bytes);
                        admitted = admitted.saturating_add(1);
                    } else {
                        assert_eq!(
                            result,
                            Err(PdfVectorError::InvalidPath),
                            "p={dimension} w={width_um} e={edge} c={clearance}",
                        );
                    }
                    checked = checked.saturating_add(1);
                }
            }
        }
    }
    assert_eq!(checked, 5_940, "all small and maximum page edges covered");
    assert!(admitted > 0, "the sweep must include drawable ink");
    assert!(admitted < checked, "the sweep must refuse clipped ink");
}

// Parse the exact /Length of each emitted stream independently of the page
// writer. A correct xref table alone cannot reveal a truncated stream or a
// stale dictionary length after cumulative page-budget reservations.
#[expect(
    clippy::expect_used,
    clippy::arithmetic_side_effects,
    clippy::indexing_slicing,
    reason = "test-only byte parser requires exact PDF offsets and lengths"
)]
fn assert_stream_length_integrity(bytes: &[u8], expected_pages: usize) {
    const LENGTH: &[u8] = b"/Length ";
    const STREAM: &[u8] = b" >>\nstream\n";
    const END: &[u8] = b"endstream\nendobj\n";
    let mut remaining = bytes;
    let mut observed = 0usize;
    while let Some(start) = remaining
        .windows(LENGTH.len())
        .position(|window| window == LENGTH)
    {
        let digits_start = start + LENGTH.len();
        let digits_end = remaining[digits_start..]
            .iter()
            .position(|byte| *byte == b' ')
            .expect("a decimal /Length must terminate");
        let length =
            from_utf8(&remaining[digits_start..digits_start + digits_end])
                .expect("stream length must be ASCII")
                .parse::<usize>()
                .expect("stream length must be decimal");
        let marker = digits_start + digits_end;
        assert!(
            remaining[marker..].starts_with(STREAM),
            "stream {observed} dictionary must end before stream data",
        );
        let stream_start = marker + STREAM.len();
        let stream_end = stream_start + length;
        let contents = remaining
            .get(stream_start..stream_end)
            .expect("declared stream length fits PDF bytes");
        assert!(contents.starts_with(b"q\n"), "page graphics state opens");
        assert!(contents.ends_with(b"Q\n"), "page graphics state closes");
        assert_eq!(
            remaining.get(stream_end..stream_end + END.len()),
            Some(END),
            "declared length exactly precedes the endstream operator",
        );
        remaining = &remaining[stream_end + END.len()..];
        observed += 1;
    }
    assert_eq!(observed, expected_pages, "one stream per ordered page");
}

#[expect(
    clippy::expect_used,
    clippy::integer_division_remainder_used,
    reason = "bounded deterministic PDF matrix checks independent byte framing"
)]
#[test]
fn mixed_page_stream_lengths_and_exact_document_budgets_remain_consistent() {
    // Vary the page tree across object-ID digit boundaries, stream-length
    // digit boundaries, blank/ink pages, both paint rules, and maximum boxes.
    // The parser independently checks that every /Length ends at endstream;
    // the existing xref oracle checks all object offsets and trailer size.
    let counts = [1usize, 2, 3, 4, 9, 10, 15, 31, 32, 99, 100, 129];
    for count in counts {
        for variant in 0usize..4 {
            let pages = (0..count)
                .map(|index| {
                    let mut sheet = match (index + variant) % 5 {
                        0 => page(Vec::new()),
                        1 => page(vec![stroke()]),
                        2 => page(vec![fill()]),
                        3 => {
                            let mut path = fill();
                            path.paint = PdfPathPaint::Fill {
                                color: PdfRgb {
                                    red: 255,
                                    green: 0,
                                    blue: 128,
                                },
                                rule: PdfFillRule::EvenOdd,
                            };
                            page(vec![stroke(), path])
                        },
                        _ => page(vec![stroke(), fill(), stroke()]),
                    };
                    if index % 3 == 1 {
                        sheet.physical_page_box = PdfVectorPageBox {
                            width_um: 215_900,
                            height_um: 279_400,
                        };
                    } else if index % 3 == 2 {
                        sheet.physical_page_box = PdfVectorPageBox {
                            width_um: 2_000_000,
                            height_um: 2_000_000,
                        };
                    }
                    sheet
                })
                .collect();
            let identity = format!("page-stream-length.{count}.{variant}");
            let composition = PdfCompositionPlan {
                assets: Vec::new(),
                pages,
                render_manifest_identity: identity.as_str(),
            };
            let generous = PdfVectorLimits {
                maximum_pages: count,
                maximum_paths_per_page: 4,
                maximum_commands_per_page: 40,
                maximum_pdf_bytes: 500_000,
            };
            let actual = serialize_vector_pdf(&composition, generous)
                .expect("mixed physical pages serialize within high limits");
            assert_xref_integrity(&actual);
            assert_stream_length_integrity(&actual, count);
            assert_eq!(
                serialize_vector_pdf(&composition, PdfVectorLimits {
                    maximum_pdf_bytes: actual.len(),
                    ..generous
                },),
                Ok(actual.clone()),
                "exact document budget: {count}/{variant}",
            );
            assert_eq!(
                serialize_vector_pdf(&composition, PdfVectorLimits {
                    maximum_pdf_bytes: actual.len() - 1,
                    ..generous
                },),
                Err(PdfVectorError::ResourceLimit),
                "one-byte shortage: {count}/{variant}",
            );
        }
    }
}

// The stream-length and xref oracles verify framing, but cannot detect a
// valid-looking PDF that silently changes the caller's ordered paint/path
// authority. These independently fixed streams pin the actual PDF grammar,
// physical coordinates, per-page graphics state and overlay precedence.
fn ordered_multicolor_fixture() -> PdfVectorComposition<'static> {
    let rose_stroke = PdfVectorPath {
        commands: vec![
            PdfPathCommand::MoveTo(point(12_700, 25_400)),
            PdfPathCommand::LineTo(point(25_400, 38_100)),
            PdfPathCommand::CubicTo(
                point(38_100, 25_400),
                point(50_800, 50_800),
                point(63_500, 38_100),
            ),
            PdfPathCommand::MoveTo(point(25_400, 63_500)),
            PdfPathCommand::LineTo(point(38_100, 76_200)),
            PdfPathCommand::Close,
        ],
        paint: PdfPathPaint::Stroke {
            color: PdfRgb {
                red: 255,
                green: 0,
                blue: 128,
            },
            width_um: 1_270,
        },
    };
    let green_hole = PdfVectorPath {
        commands: vec![
            PdfPathCommand::MoveTo(point(12_700, 12_700)),
            PdfPathCommand::LineTo(point(38_100, 12_700)),
            PdfPathCommand::LineTo(point(38_100, 38_100)),
            PdfPathCommand::LineTo(point(12_700, 38_100)),
            PdfPathCommand::Close,
            PdfPathCommand::MoveTo(point(19_050, 19_050)),
            PdfPathCommand::LineTo(point(31_750, 19_050)),
            PdfPathCommand::LineTo(point(31_750, 31_750)),
            PdfPathCommand::LineTo(point(19_050, 31_750)),
            PdfPathCommand::Close,
        ],
        paint: PdfPathPaint::Fill {
            color: PdfRgb {
                red: 0,
                green: 255,
                blue: 0,
            },
            rule: PdfFillRule::EvenOdd,
        },
    };
    let yellow_triangle = PdfVectorPath {
        commands: vec![
            PdfPathCommand::MoveTo(point(25_400, 50_800)),
            PdfPathCommand::LineTo(point(50_800, 50_800)),
            PdfPathCommand::LineTo(point(38_100, 76_200)),
            PdfPathCommand::Close,
        ],
        paint: PdfPathPaint::Fill {
            color: PdfRgb {
                red: 255,
                green: 255,
                blue: 0,
            },
            rule: PdfFillRule::NonZero,
        },
    };
    let red_stroke = PdfVectorPath {
        commands: vec![
            PdfPathCommand::MoveTo(point(76_200, 101_600)),
            PdfPathCommand::LineTo(point(101_600, 127_000)),
        ],
        paint: PdfPathPaint::Stroke {
            color: PdfRgb {
                red: 255,
                green: 0,
                blue: 0,
            },
            width_um: 254,
        },
    };
    let mut letter = page(vec![yellow_triangle, red_stroke]);
    letter.physical_page_box = PdfVectorPageBox {
        width_um: 215_900,
        height_um: 279_400,
    };
    let mut large_blank = page(Vec::new());
    large_blank.physical_page_box = PdfVectorPageBox {
        width_um: 2_000_000,
        height_um: 2_000_000,
    };
    PdfCompositionPlan {
        assets: Vec::new(),
        pages: vec![page(vec![rose_stroke, green_hole]), letter, large_blank],
        render_manifest_identity: "ordered-path-authority.v1",
    }
}

#[expect(
    clippy::expect_used,
    clippy::indexing_slicing,
    reason = "test-only PDF grammar snapshots have three known page streams"
)]
#[test]
fn ordered_multi_page_paint_and_path_authority_has_exact_content_streams() {
    let composition = ordered_multicolor_fixture();
    let pdf = serialize_vector_pdf(&composition, limits())
        .expect("all original paint and path authority is valid");
    assert_xref_integrity(&pdf);
    assert_stream_length_integrity(&pdf, 3);
    let text = pdf_string(&pdf);
    let streams: Vec<&str> = text
        .split("\nstream\n")
        .skip(1)
        .map(|tail| {
            tail.split_once("endstream\n").expect("stream terminator").0
        })
        .collect();
    assert_eq!(streams.len(), 3, "one independent stream per page");
    assert_eq!(
        streams[0],
        concat!(
            "q\n1 0 0 -1 0 841.890 cm\n",
            "1.000 0.000 0.502 RG\n3.600 w\n1 J\n1 j\n",
            "36.000 72.000 m\n72.000 108.000 l\n",
            "108.000 72.000 144.000 144.000 180.000 108.000 c\n",
            "72.000 180.000 m\n108.000 216.000 l\nh\nS\n",
            "0.000 1.000 0.000 rg\n",
            "36.000 36.000 m\n108.000 36.000 l\n",
            "108.000 108.000 l\n36.000 108.000 l\nh\n",
            "54.000 54.000 m\n90.000 54.000 l\n",
            "90.000 90.000 l\n54.000 90.000 l\nh\nf*\nQ\n",
        ),
        "first page retains stroke subpaths and both fill contours"
    );
    assert_eq!(
        streams[1],
        concat!(
            "q\n1 0 0 -1 0 792.000 cm\n",
            "1.000 1.000 0.000 rg\n",
            "72.000 144.000 m\n144.000 144.000 l\n",
            "108.000 216.000 l\nh\nf\n",
            "1.000 0.000 0.000 RG\n0.720 w\n1 J\n1 j\n",
            "216.000 288.000 m\n288.000 360.000 l\nS\nQ\n",
        ),
        "second page retains original different physical box and paint order"
    );
    assert_eq!(
        streams[2], "q\n1 0 0 -1 0 5669.291 cm\nQ\n",
        "the blank maximum-size page has its own closed graphics state"
    );
    assert_eq!(
        serialize_vector_pdf(&composition, PdfVectorLimits {
            maximum_pdf_bytes: pdf.len(),
            ..limits()
        }),
        Ok(pdf.clone()),
        "complete ordered stream content must survive the exact byte budget",
    );
    assert_eq!(
        serialize_vector_pdf(&composition, PdfVectorLimits {
            maximum_pdf_bytes: pdf.len() - 1,
            ..limits()
        }),
        Err(PdfVectorError::ResourceLimit),
    );
}

// The PDF DeviceRGB operands are rounded to three decimal places. Test all
// byte values against an integer oracle, including combinations that reveal
// swapped red/blue channels and the sharpest rounding boundaries.
#[expect(
    clippy::expect_used,
    clippy::integer_division,
    clippy::integer_division_remainder_used,
    reason = "bounded color oracle compares an exact rational with PDF numbers"
)]
#[test]
fn all_eight_bit_rgb_channels_preserve_order_and_rounded_values() {
    fn expected_channel(byte: u8) -> String {
        let milli = (u32::from(byte) * 1_000 + 127) / 255;
        format!("{}.{:03}", milli / 1_000, milli % 1_000)
    }
    let mut cases = 0usize;
    for value in 0u8..=255 {
        let color = PdfRgb {
            red: value,
            green: 255 - value,
            blue: value / 2,
        };
        let operands = format!(
            "{} {} {}",
            expected_channel(color.red),
            expected_channel(color.green),
            expected_channel(color.blue),
        );
        for paint in [
            PdfPathPaint::Stroke { color, width_um: 127 },
            PdfPathPaint::Fill {
                color,
                rule: PdfFillRule::NonZero,
            },
        ] {
            let commands = if matches!(paint, PdfPathPaint::Stroke { .. }) {
                vec![
                    PdfPathCommand::MoveTo(point(1_270, 1_270)),
                    PdfPathCommand::LineTo(point(2_540, 2_540)),
                ]
            } else {
                vec![
                    PdfPathCommand::MoveTo(point(1_270, 1_270)),
                    PdfPathCommand::LineTo(point(2_540, 1_270)),
                    PdfPathCommand::LineTo(point(1_270, 2_540)),
                    PdfPathCommand::Close,
                ]
            };
            let document = PdfCompositionPlan {
                assets: Vec::new(),
                pages: vec![page(vec![PdfVectorPath { commands, paint }])],
                render_manifest_identity: "all-device-rgb-channels.v1",
            };
            let bytes = serialize_vector_pdf(&document, limits())
                .expect("valid three-channel RGB paint must serialize");
            let operator = if matches!(paint, PdfPathPaint::Stroke { .. }) {
                "RG\n"
            } else {
                "rg\n"
            };
            assert!(
                pdf_string(&bytes).contains(&format!("{operands} {operator}")),
                "RGB components must remain in order and exactly rounded: \
                 channel={value} paint={paint:?}",
            );
            cases += 1;
        }
    }
    assert_eq!(cases, 512, "all byte values in both PDF paint families");
}

// Locate a PDF page content stream by its real byte offsets. The binary
// PDF header makes lossy UTF-8 string indices unsuitable for this oracle.
#[expect(
    clippy::expect_used,
    clippy::arithmetic_side_effects,
    reason = "test-only byte parser fails explicitly on missing PDF framing"
)]
fn raw_page_stream_bounds(pdf: &[u8], page_index: usize) -> (usize, usize) {
    let marker = b"\nstream\n";
    let begin = pdf
        .windows(marker.len())
        .enumerate()
        .filter(|(_offset, window)| *window == marker)
        .nth(page_index)
        .map(|(offset, _window)| offset + marker.len())
        .expect("page has a content stream");
    let terminator = b"endstream\n";
    let end = pdf
        .get(begin..)
        .expect("page content start")
        .windows(terminator.len())
        .position(|window| window == terminator)
        .map(|offset| begin + offset)
        .expect("page stream ends");
    (begin, end)
}

// Create four different physical page boxes and both paint families
// without sharing mutable resource or color state between pages.
fn paint_locality_fixture(
    seed: u8,
    identity: &str,
) -> PdfVectorComposition<'_> {
    let pages = (0u8..4)
        .map(|index| {
            let mut sheet = page(vec![stroke(), fill()]);
            sheet.physical_page_box = match index {
                0 => PdfVectorPageBox {
                    width_um: 210_000,
                    height_um: 297_000,
                },
                1 => PdfVectorPageBox {
                    width_um: 215_900,
                    height_um: 279_400,
                },
                2 => PdfVectorPageBox {
                    width_um: 500_000,
                    height_um: 500_000,
                },
                _ => PdfVectorPageBox {
                    width_um: 2_000_000,
                    height_um: 2_000_000,
                },
            };
            for path in &mut sheet.vector_authority {
                let color = PdfRgb {
                    red: seed.wrapping_add(index.wrapping_mul(37)),
                    green: seed.wrapping_mul(3).wrapping_add(index),
                    blue: 255u8.wrapping_sub(seed).wrapping_sub(index),
                };
                match &mut path.paint {
                    PdfPathPaint::Stroke { color: ink, .. }
                    | PdfPathPaint::Fill { color: ink, .. } => {
                        *ink = color;
                    },
                }
            }
            sheet
        })
        .collect();
    PdfCompositionPlan {
        pages,
        assets: Vec::new(),
        render_manifest_identity: identity,
    }
}

// PDF resources are deliberately page-local here. Changing one paint or the
// manifest identity must not leak into another page or the document index.
// Unlike a fixed operator snapshot, this tests noninterference across many
// distinct colors, page boxes, path families, and unchanged xref offsets.
#[expect(
    clippy::expect_used,
    clippy::indexing_slicing,
    reason = "test fixture owns exact bounded page and path indices"
)]
#[test]
fn page_paint_and_manifest_edits_are_byte_local() {
    let mut paint_cases = 0usize;
    let mut identity_cases = 0usize;
    for seed in 0u8..64 {
        let identity = format!("manifest-locality-A-{seed:03}");
        let other_identity = format!("manifest-locality-B-{seed:03}");
        let source = paint_locality_fixture(seed, &identity);
        let generous = PdfVectorLimits {
            maximum_pages: 4,
            maximum_paths_per_page: 2,
            maximum_commands_per_page: 10,
            maximum_pdf_bytes: 20_000,
        };
        let original = serialize_vector_pdf(&source, generous)
            .expect("valid varied pages serialize");
        assert_xref_integrity(&original);
        assert_stream_length_integrity(&original, 4);
        for page_index in 0..4 {
            for path_index in 0..2 {
                let mut edited = source.clone();
                let selected = &mut edited.pages[page_index].vector_authority;
                let paint = &mut selected[path_index].paint;
                match paint {
                    PdfPathPaint::Stroke { color, .. }
                    | PdfPathPaint::Fill { color, .. } => {
                        color.red = !color.red;
                    },
                }
                let changed = serialize_vector_pdf(&edited, generous)
                    .expect("single paint edit remains valid");
                assert_eq!(changed.len(), original.len());
                assert_xref_integrity(&changed);
                assert_stream_length_integrity(&changed, 4);
                let (begin, end) =
                    raw_page_stream_bounds(&original, page_index);
                assert_eq!(
                    &original[..begin],
                    &changed[..begin],
                    "earlier objects changed: {seed}/{page_index}/{path_index}"
                );
                assert_ne!(
                    &original[begin..end],
                    &changed[begin..end],
                    "selected page paint must change"
                );
                assert_eq!(
                    &original[end..],
                    &changed[end..],
                    "later pages, Info and xref must be unchanged"
                );
                paint_cases += 1;
            }
        }

        let mut edited = source.clone();
        edited.render_manifest_identity = other_identity.as_str();
        let changed = serialize_vector_pdf(&edited, generous)
            .expect("same-length manifest change remains valid");
        let label = b"/AtramentManifestID (";
        let begin = original
            .windows(label.len())
            .position(|window| window == label)
            .map(|offset| offset + label.len())
            .expect("original PDF has manifest Info field");
        let end = begin + identity.len();
        assert_eq!(identity.len(), other_identity.len());
        assert_eq!(&original[begin..end], identity.as_bytes());
        assert_eq!(&changed[begin..end], other_identity.as_bytes());
        assert_eq!(&original[..begin], &changed[..begin]);
        assert_eq!(&original[end..], &changed[end..]);
        assert_xref_integrity(&changed);
        assert_stream_length_integrity(&changed, 4);
        identity_cases += 1;
    }
    assert_eq!(paint_cases, 512);
    assert_eq!(identity_cases, 64);
}
