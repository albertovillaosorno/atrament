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
//   - Thin native CLI projecting valid paper marks into existing Rust PDF.
// - Must-Not:
//   - Claim notebook text rendering, image embedding, or profile-file loading.
// - Allows:
//   - Inputs: Explicit sheet, margins, mark style/spacing and RGB color.
//   - Outputs: One printable PDF on standard output or typed refusal.
//   - Side effects: Standard output writes only.
// - Split-When:
//   - Production notebook export integrates real user content and file effects.
// - Merge-When:
//   - A production Rust notebook renderer subsumes this demonstration.
// - Summary:
//   - Exercises actual Rust page geometry and vector PDF capabilities together.
// - Description:
//   - Projects blank, ruled, squared and dotted paper into physical PDF marks.
// - Usage:
//   - cargo run -q -p atrament_notebook_paper_cli -- --style grid
//   - --spacing-mm 5 > sheet.pdf
// - Defaults:
//   - A4 grid, 5 mm spacing, 22/16/20/14 mm margins, blue-grey marks.
//

//! Thin CLI for native physical paper PDFs; no notebook content yet.

use std::env;
use std::error::Error;
use std::io::{self, Write as _};

use atrament_paper_mark_geometry::{
    PaperMarkGeometry, Span, compile_profile_marks,
};
use atrament_pdf_composition_plan::{
    PdfCompositionPlan, PdfPagePlan, SearchableSemanticText,
};
use atrament_pdf_vector_serializer::{
    PdfFillRule, PdfPathCommand, PdfPathPaint, PdfPoint, PdfRgb,
    PdfVectorColorIntent, PdfVectorLimits, PdfVectorPageBox, PdfVectorPath,
    serialize_vector_pdf,
};
use atrament_physical_page_profile::{
    BindingEdge, BorderShape, Length, Orientation, PageProfile,
    PaperMarkAppearance, PaperMarkJoin, PaperMarkLayer, PaperPattern, Rect,
    SheetSize,
};

const MAX_MARKS: u64 = 10_000;
const DEFAULT_COLOR: PdfRgb = PdfRgb {
    red: 197,
    green: 212,
    blue: 231,
};

const USAGE: &str = "Usage: atrament-paper [--style blank|ruled|grid|dot] \
    [--width-mm N] [--height-mm N] [--spacing-mm N] \
    [--margin-top-mm N] [--margin-bottom-mm N] \
    [--margin-left-mm N] [--margin-right-mm N] \
    [--line-color '#RRGGBB'] > sheet.pdf";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Style {
    Blank,
    Dot,
    Grid,
    Ruled,
}

#[derive(Clone, Copy, Debug)]
struct Options {
    color: PdfRgb,
    height_um: u32,
    margin_bottom_um: u32,
    margin_left_um: u32,
    margin_right_um: u32,
    margin_top_um: u32,
    spacing_um: u32,
    style: Style,
    width_um: u32,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            color: DEFAULT_COLOR,
            height_um: 297_000,
            margin_bottom_um: 16_000,
            margin_left_um: 20_000,
            margin_right_um: 14_000,
            margin_top_um: 22_000,
            spacing_um: 5_000,
            style: Style::Grid,
            width_um: 210_000,
        }
    }
}

fn millimetres(text: &str) -> Result<u32, String> {
    let (integral_text, decimals) = text.split_once('.').unwrap_or((text, ""));
    if integral_text.is_empty()
        || !integral_text.bytes().all(|byte| byte.is_ascii_digit())
        || decimals.len() > 3
        || !decimals.bytes().all(|byte| byte.is_ascii_digit())
    {
        return Err("millimeters require up to three decimal places".into());
    }
    let integral = integral_text
        .parse::<u32>()
        .map_err(|error| format!("invalid millimeters: {error}"))?;
    let decimal = if decimals.is_empty() {
        0
    } else {
        let fraction = decimals.parse::<u32>().map_err(|error| {
            format!("invalid fractional millimeters: {error}")
        })?;
        let factor = match decimals.len() {
            1 => 100,
            2 => 10,
            _ => 1,
        };
        fraction
            .checked_mul(factor)
            .ok_or_else(|| "millimeter fraction overflow".to_owned())?
    };
    integral
        .checked_mul(1_000)
        .and_then(|value| value.checked_add(decimal))
        .ok_or_else(|| "millimeter value is out of range".to_owned())
}

fn hex_byte(segment: Option<&str>) -> Result<u8, String> {
    let text = segment.ok_or("line color must be #RRGGBB")?;
    u8::from_str_radix(text, 16)
        .map_err(|error| format!("invalid #RRGGBB color: {error}"))
}

fn line_color(value: &str) -> Result<PdfRgb, String> {
    if value.len() != 7 || !value.starts_with('#') || !value.is_ascii() {
        return Err("line color must be #RRGGBB".into());
    }
    Ok(PdfRgb {
        red: hex_byte(value.get(1..3))?,
        green: hex_byte(value.get(3..5))?,
        blue: hex_byte(value.get(5..7))?,
    })
}

fn parse_options(args: &[String]) -> Result<Option<Options>, String> {
    let mut opts = Options::default();
    let mut items = args.iter();
    while let Some(name) = items.next() {
        if name == "--help" {
            return Ok(None);
        }
        let value = items.next().ok_or_else(|| {
            format!("missing value for {name}; use --help for arguments")
        })?;
        match name.as_str() {
            "--style" => {
                opts.style = match value.as_str() {
                    "blank" => Style::Blank,
                    "ruled" => Style::Ruled,
                    "grid" => Style::Grid,
                    "dot" => Style::Dot,
                    _ => {
                        return Err(
                            "style must be blank, ruled, grid or dot".into()
                        );
                    }
                };
            }
            "--width-mm" => opts.width_um = millimetres(value)?,
            "--height-mm" => opts.height_um = millimetres(value)?,
            "--spacing-mm" => opts.spacing_um = millimetres(value)?,
            "--margin-top-mm" => opts.margin_top_um = millimetres(value)?,
            "--margin-bottom-mm" => {
                opts.margin_bottom_um = millimetres(value)?;
            }
            "--margin-left-mm" => opts.margin_left_um = millimetres(value)?,
            "--margin-right-mm" => {
                opts.margin_right_um = millimetres(value)?;
            }
            "--line-color" => opts.color = line_color(value)?,
            _ => return Err(format!("unsupported option {name}; use --help")),
        }
    }
    Ok(Some(opts))
}

fn position(value: u64) -> Result<i32, String> {
    i32::try_from(value)
        .map_err(|error| format!("coordinate exceeds PDF limits: {error}"))
}

fn stroke(
    x1: u64,
    y1: u64,
    x2: u64,
    y2: u64,
    color: PdfRgb,
) -> Result<PdfVectorPath, String> {
    Ok(PdfVectorPath {
        commands: vec![
            PdfPathCommand::MoveTo(PdfPoint {
                x_um: position(x1)?,
                y_um: position(y1)?,
            }),
            PdfPathCommand::LineTo(PdfPoint {
                x_um: position(x2)?,
                y_um: position(y2)?,
            }),
        ],
        paint: PdfPathPaint::Stroke {
            color,
            width_um: 120,
        },
    })
}

fn dot(
    x: u64,
    y: u64,
    width: u64,
    height: u64,
    color: PdfRgb,
) -> Result<PdfVectorPath, String> {
    let left = position(x.saturating_sub(90))?;
    let top = position(y.saturating_sub(90))?;
    let right = position(x.saturating_add(90).min(width))?;
    let bottom = position(y.saturating_add(90).min(height))?;
    Ok(PdfVectorPath {
        commands: vec![
            PdfPathCommand::MoveTo(PdfPoint {
                x_um: left,
                y_um: top,
            }),
            PdfPathCommand::LineTo(PdfPoint {
                x_um: right,
                y_um: top,
            }),
            PdfPathCommand::LineTo(PdfPoint {
                x_um: right,
                y_um: bottom,
            }),
            PdfPathCommand::LineTo(PdfPoint {
                x_um: left,
                y_um: bottom,
            }),
            PdfPathCommand::Close,
        ],
        paint: PdfPathPaint::Fill {
            color,
            rule: PdfFillRule::NonZero,
        },
    })
}

fn append_lines(
    paths: &mut Vec<PdfVectorPath>,
    series: atrament_paper_mark_geometry::AxisSeries,
    span: Span,
    horizontal: bool,
    color: PdfRgb,
) -> Result<(), String> {
    for index in 0..series.count {
        let coord = series
            .coordinate(index)
            .ok_or("nominal mark arithmetic overflow")?
            .micrometres();
        let start = span.start.micrometres();
        let end = span.end.micrometres();
        paths.push(if horizontal {
            stroke(start, coord, end, coord, color)?
        } else {
            stroke(coord, start, coord, end, color)?
        });
    }
    Ok(())
}

fn mark_geometry(opts: Options) -> Result<PaperMarkGeometry, String> {
    if !(40_000..=500_000).contains(&opts.width_um)
        || !(40_000..=700_000).contains(&opts.height_um)
        || !(1_000..=50_000).contains(&opts.spacing_um)
    {
        return Err("invalid sheet dimensions or spacing".into());
    }
    let width = opts
        .width_um
        .checked_sub(opts.margin_left_um)
        .and_then(|remaining| remaining.checked_sub(opts.margin_right_um))
        .filter(|remaining| *remaining > 0)
        .ok_or("horizontal margins consume the sheet")?;
    let height = opts
        .height_um
        .checked_sub(opts.margin_top_um)
        .and_then(|remaining| remaining.checked_sub(opts.margin_bottom_um))
        .filter(|remaining| *remaining > 0)
        .ok_or("vertical margins consume the sheet")?;
    let length = |value| Length::from_micrometres(u64::from(value));
    let region = Rect {
        x: length(opts.margin_left_um),
        y: length(opts.margin_top_um),
        width: length(width),
        height: length(height),
    };
    let pattern = match opts.style {
        Style::Blank => PaperPattern::Blank,
        Style::Ruled => PaperPattern::Ruled {
            spacing: length(opts.spacing_um),
        },
        Style::Grid => PaperPattern::Squared {
            spacing: length(opts.spacing_um),
        },
        Style::Dot => PaperPattern::Dotted {
            spacing: length(opts.spacing_um),
        },
    };
    let profile = PageProfile {
        binding_edge: BindingEdge::Left,
        border_shape: BorderShape::None,
        corner_roundness: Length::ZERO,
        orientation: Orientation::Portrait,
        outer_margin: Length::ZERO,
        paper_mark_appearance: PaperMarkAppearance {
            join: PaperMarkJoin::Sharp,
            maximum_ruler_error: Length::ZERO,
        },
        paper_mark_layer: PaperMarkLayer::BelowInk,
        paper_pattern: pattern,
        printable_region: region,
        sheet: SheetSize {
            width: length(opts.width_um),
            height: length(opts.height_um),
        },
        top_clearance: Length::ZERO,
        writing_inset: Length::ZERO,
    };
    let marks = compile_profile_marks(region, profile)
        .map_err(|error| format!("invalid physical paper: {error:?}"))?;
    Ok(marks.geometry)
}

fn render(opts: Options) -> Result<Vec<u8>, String> {
    let geometry = mark_geometry(opts)?;
    let count = match geometry {
        PaperMarkGeometry::Blank => 0,
        PaperMarkGeometry::Ruled { horizontal, .. } => horizontal.count,
        PaperMarkGeometry::Squared {
            horizontal,
            vertical,
            ..
        } => horizontal
            .count
            .checked_add(vertical.count)
            .ok_or("paper mark budget exceeded")?,
        PaperMarkGeometry::Dotted {
            horizontal,
            vertical,
        } => horizontal
            .count
            .checked_mul(vertical.count)
            .ok_or("paper mark budget exceeded")?,
    };
    if count > MAX_MARKS {
        return Err("paper mark budget exceeded; increase spacing".into());
    }
    let mut paths = Vec::new();
    paths
        .try_reserve_exact(
            usize::try_from(count).map_err(|error| error.to_string())?,
        )
        .map_err(|error| format!("paper mark allocation failed: {error}"))?;
    match geometry {
        PaperMarkGeometry::Blank => {}
        PaperMarkGeometry::Ruled { horizontal, span } => {
            append_lines(&mut paths, horizontal, span, true, opts.color)?;
        }
        PaperMarkGeometry::Squared {
            horizontal,
            horizontal_span,
            vertical,
            vertical_span,
        } => {
            append_lines(
                &mut paths,
                horizontal,
                horizontal_span,
                true,
                opts.color,
            )?;
            append_lines(
                &mut paths,
                vertical,
                vertical_span,
                false,
                opts.color,
            )?;
        }
        PaperMarkGeometry::Dotted {
            horizontal,
            vertical,
        } => {
            for row in 0..horizontal.count {
                let y = horizontal
                    .coordinate(row)
                    .ok_or("dot row overflow")?
                    .micrometres();
                for column in 0..vertical.count {
                    let x = vertical
                        .coordinate(column)
                        .ok_or("dot column overflow")?
                        .micrometres();
                    paths.push(dot(
                        x,
                        y,
                        u64::from(opts.width_um),
                        u64::from(opts.height_um),
                        opts.color,
                    )?);
                }
            }
        }
    }
    serialize_paths(opts, paths)
}

fn serialize_paths(
    opts: Options,
    paths: Vec<PdfVectorPath>,
) -> Result<Vec<u8>, String> {
    let doc = PdfCompositionPlan {
        assets: Vec::new(),
        pages: vec![PdfPagePlan {
            color_intent: PdfVectorColorIntent::DeviceRgb,
            physical_page_box: PdfVectorPageBox {
                width_um: opts.width_um,
                height_um: opts.height_um,
            },
            searchable_text: SearchableSemanticText::OmittedAsIncompatible,
            vector_authority: paths,
        }],
        render_manifest_identity: "paper-profile.v1",
    };
    serialize_vector_pdf(
        &doc,
        PdfVectorLimits {
            maximum_commands_per_page: 50_000,
            maximum_pages: 1,
            maximum_paths_per_page: 10_000,
            maximum_pdf_bytes: 8_000_000,
        },
    )
    .map_err(|err| format!("PDF encoding refused: {err:?}"))
}

/// Generate a reproducible physical-size sheet with Rust geometry and PDF.
///
/// # Errors
///
/// Invalid parameters or output errors abort before reporting success.
fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<_> = env::args().skip(1).collect();
    let Some(opts) = parse_options(&args)? else {
        writeln!(io::stderr().lock(), "{USAGE}")?;
        return Ok(());
    };
    let bytes = render(opts)?;
    io::stdout().lock().write_all(&bytes)?;
    Ok(())
}

#[cfg(test)]
#[path = "../../../../tests/backend/notebook-paper-cli/composition/lib.rs"]
mod tests;
