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
//   - Test-only A4/Letter PDF fixture composed from synthetic vectors.
// - Must-Not:
//   - Read notebooks, write files, infer layout, or claim usable Export.
// - Allows:
//   - Inputs: Fixed physical page boxes and vector path geometry.
//   - Outputs: Binary PDF bytes on standard output.
//   - Side effects: Standard output writes only.
// - Split-When:
//   - A notebook renderer can provide accepted page geometry.
// - Merge-When:
//   - End-to-end PDF tests render equivalent fixture content.
// - Summary:
//   - Reproduces a vector PDF smoke without production fixture dependencies.
// - Description:
//   - Creates one A4 stroke page and one Letter even-odd contour page.
// - Usage:
//   - Run the Cargo example and redirect stdout to a test-owned PDF path.
// - Defaults:
//   - No user data, filesystem writes, or notebook conversion.
//

//! Independent, deterministic vector-only PDF smoke example.

use std::error::Error;
use std::io::{self, Write as _};

use atrament_pdf_composition_plan::PdfCompositionPlan;
use atrament_pdf_composition_plan::PdfPagePlan;
use atrament_pdf_composition_plan::SearchableSemanticText;
use atrament_pdf_vector_serializer::{PdfFillRule, PdfPathCommand, PdfPathPaint};
use atrament_pdf_vector_serializer::{PdfPoint, PdfRgb, PdfVectorColorIntent};
use atrament_pdf_vector_serializer::{PdfVectorLimits, PdfVectorPageBox};
use atrament_pdf_vector_serializer::{PdfVectorPath, serialize_vector_pdf};

type Paths = Vec<PdfVectorPath>;
type Page = PdfPagePlan<PdfVectorColorIntent, PdfVectorPageBox, (), Paths>;

const fn point(x_um: i32, y_um: i32) -> PdfPoint {
    PdfPoint { x_um, y_um }
}

fn page(width_um: u32, height_um: u32, path: PdfVectorPath) -> Page {
    PdfPagePlan {
        color_intent: PdfVectorColorIntent::DeviceRgb,
        physical_page_box: PdfVectorPageBox {
            height_um,
            width_um,
        },
        searchable_text: SearchableSemanticText::OmittedAsIncompatible,
        vector_authority: vec![path],
    }
}

fn a4_stroke() -> Page {
    page(
        210_000,
        297_000,
        PdfVectorPath {
            commands: vec![
                PdfPathCommand::MoveTo(point(20_000, 20_000)),
                PdfPathCommand::LineTo(point(80_000, 20_000)),
            ],
            paint: PdfPathPaint::Stroke {
                color: PdfRgb {
                    red: 5,
                    green: 22,
                    blue: 240,
                },
                width_um: 2_000,
            },
        },
    )
}

fn letter_even_odd_fill() -> Page {
    page(
        215_900,
        279_400,
        PdfVectorPath {
            commands: vec![
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
            ],
            paint: PdfPathPaint::Fill {
                color: PdfRgb {
                    red: 190,
                    green: 50,
                    blue: 80,
                },
                rule: PdfFillRule::EvenOdd,
            },
        },
    )
}

/// Reproduce an independent reader fixture with no notebook or file I/O.
///
/// # Errors
///
/// Reports fixed-geometry admission or stdout failures.
fn main() -> Result<(), Box<dyn Error>> {
    let composition = PdfCompositionPlan {
        assets: Vec::new(),
        pages: vec![a4_stroke(), letter_even_odd_fill()],
        render_manifest_identity: "synthetic-vector-demo.v1",
    };
    let bytes = serialize_vector_pdf(
        &composition,
        PdfVectorLimits {
            maximum_commands_per_page: 100,
            maximum_pages: 2,
            maximum_paths_per_page: 1,
            maximum_pdf_bytes: 50_000,
        },
    )
    .map_err(|error| io::Error::other(format!("{error:?}")))?;
    io::stdout().lock().write_all(&bytes)?;
    Ok(())
}
