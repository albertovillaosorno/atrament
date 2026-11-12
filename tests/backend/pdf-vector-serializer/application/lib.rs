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
