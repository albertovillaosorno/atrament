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
//   - Native CLI projecting admitted paper marks to existing Rust outputs.
// - Must-Not:
//   - Claim notebook text rendering, image embedding, or profile-file loading.
// - Allows:
//   - Inputs: Explicit page options and bounded versioned paper-only JSON.
//   - Outputs: Canonical paper JSON, vector PDF/PNG or explicit CLI refusal.
//   - Side effects: Optional caller-selected profile read and explicit file
//     commit, or stdout write.
// - Split-When:
//   - Production notebook export integrates real user content and file effects.
// - Merge-When:
//   - A production Rust notebook renderer subsumes this demonstration.
// - Summary:
//   - Reuses Rust page geometry for PDF and transparent PNG projection.
// - Description:
//   - Projects blank, ruled, squared and dotted paper to physical PDF/PNG.
// - Usage:
//   - cargo run -q -p atrament_notebook_paper_cli -- --style grid
//   - --spacing-mm 5 > sheet.pdf
// - Defaults:
//   - A4 grid, 5 mm spacing, 22/16/20/14 mm margins, blue-grey marks.
//

//! Shared Rust paper-profile, CLI, PDF and transparent PNG projection.

mod date;
mod png;

pub use date::validate_printed_date_text;

use std::env;
use std::error::Error;
use std::fs::{File, metadata};
use std::io::{self, Write as _};
use std::mem::swap;
use std::path::Path;
use std::str::from_utf8;

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
use atrament_profile_file_commit::{
    ProfileFileCommitDisposition, commit_profile_file,
};
use atrament_profile_file_reader::read_bounded_profile_file;
// The separate print binary owns safe selected-file opening.
use atrament_profile_selected_file_open as _;
use atrament_profile_json_syntax::{
    ProfileJsonSyntaxLimits, ValidatedProfileJsonObject,
    validate_profile_json_object,
};

/// Maximum caller-authored pen segments per document export.
pub const MAX_NOTEBOOK_INK_LINES: usize = 512;

const MAX_MARKS: u64 = 10_000;
/// Maximum admitted versioned paper-profile JSON bytes.
pub const MAX_PROFILE_BYTES: usize = 4_096;
const MAX_PAGES: usize = 32;
const DEFAULT_PNG_DPI: u32 = 150;
const MAX_OUTPUT_BYTES: usize = 24_000_000;
/// Maximum bytes for a physical paper PDF or PNG export.
pub const MAX_PAPER_OUTPUT_BYTES: usize = MAX_OUTPUT_BYTES;
const PAPER_PROFILE_SCHEMA: &str = "atrament.paper.v1";
const DATE_WIDTH_UM: u64 = 36_000;
const DATE_HEIGHT_UM: u64 = 8_000;
const PAGE_NUMBER_CELL_UM: u64 = 600;
const PAGE_NUMBER_STEP_UM: u64 = 750;
const PAGE_NUMBER_BOTTOM_MARGIN_UM: u32 = 7_000;
const NOTEBOOK_BORDER_STROKE_UM: u32 = 120;
// Three columns by five rows, with a bounded, legible printed numeral.
// This is intentionally a print annotation, not synthesized handwriting.
const PAGE_NUMBER_DIGITS: [[u8; 5]; 10] = [
    [0b111, 0b101, 0b101, 0b101, 0b111],
    [0b010, 0b110, 0b010, 0b010, 0b111],
    [0b111, 0b001, 0b111, 0b100, 0b111],
    [0b111, 0b001, 0b111, 0b001, 0b111],
    [0b101, 0b101, 0b111, 0b001, 0b001],
    [0b111, 0b100, 0b111, 0b001, 0b111],
    [0b111, 0b100, 0b111, 0b101, 0b111],
    [0b111, 0b001, 0b001, 0b001, 0b001],
    [0b111, 0b101, 0b111, 0b101, 0b111],
    [0b111, 0b101, 0b111, 0b001, 0b111],
];
const DEFAULT_COLOR: PdfRgb = PdfRgb {
    red: 197,
    green: 212,
    blue: 231,
};

const USAGE: &str = "Usage: atrament-paper [--profile paper.json] \
    [--style blank|ruled|grid|dot] [--paper-size a3|a4|a5|b5|letter|legal] \
    [--landscape|--portrait] [--width-mm N] [--height-mm N] [--spacing-mm N] \
    [--margin-top-mm N] [--margin-bottom-mm N] \
    [--margin-left-mm N] [--margin-right-mm N] \
    [--line-color '#RRGGBB'] [--pages 1..32] \
    [--format pdf|png] [--dpi 72..300 (PNG only)] \
    [--emit-profile] [--output FILE [--replace]] > output-file";

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

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum OutputFormat {
    Pdf,
    Png,
}

/// A top-margin writing placeholder on a notebook page (not actual date text).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PaperDatePlaceholder {
    /// An outlined date rectangle at the left of the top margin.
    LeftBox,
    /// One writable line at the left of the top margin.
    LeftLine,
    /// An outlined date rectangle at the right of the top margin.
    RightBox,
    /// One writable line at the right of the top margin.
    RightLine,
}

/// One caller-authored straight physical pen stroke on a numbered page.
///
/// This is explicit user ink, not glyph synthesis or statistical handwriting.
/// Coordinates use micrometres from the top-left sheet edge.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NotebookInkLine {
    /// Physical ending position (x, y).
    pub end_um: (u32, u32),
    /// One-based document page number.
    pub page: usize,
    /// Selected ink RGB, independent of paper ruling.
    pub rgb: [u8; 3],
    /// Physical starting position (x, y).
    pub start_um: (u32, u32),
    /// Pen thickness in micrometres (80 to 1500).
    pub width_um: u32,
}

// An admitted printable-region outline shared by PDF and transparent PNG.
// Radius zero is intentionally borderless in the notebook v1 profile.
#[derive(Clone, Copy)]
struct RoundedPaperBorder {
    bottom_um: u32,
    color: PdfRgb,
    left_um: u32,
    radius_um: u32,
    right_um: u32,
    top_um: u32,
}

#[derive(Clone, Debug)]
struct CliRequest {
    dpi: u32,
    emit_profile: bool,
    format: OutputFormat,
    options: Options,
    output: Option<String>,
    pages: usize,
    replace: bool,
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

fn paper_size(value: &str) -> Result<(u32, u32), String> {
    match value.to_ascii_lowercase().as_str() {
        "a3" => Ok((297_000, 420_000)),
        "a4" => Ok((210_000, 297_000)),
        "a5" => Ok((148_000, 210_000)),
        "b5" => Ok((176_000, 250_000)),
        "letter" => Ok((215_900, 279_400)),
        "legal" => Ok((215_900, 355_600)),
        _ => Err("paper size must be a3, a4, a5, b5, letter or legal".into()),
    }
}

fn paper_style(value: &str) -> Result<Style, String> {
    match value {
        "blank" => Ok(Style::Blank),
        "ruled" => Ok(Style::Ruled),
        "grid" => Ok(Style::Grid),
        "dot" => Ok(Style::Dot),
        _ => Err("style must be blank, ruled, grid or dot".into()),
    }
}

const fn style_name(style: Style) -> &'static str {
    match style {
        Style::Blank => "blank",
        Style::Ruled => "ruled",
        Style::Grid => "grid",
        Style::Dot => "dot",
    }
}

fn profile_text(
    root: ValidatedProfileJsonObject<'_>,
    name: &str,
) -> Result<String, String> {
    let value = root
        .root_member_value(name)
        .ok_or_else(|| format!("missing paper profile field: {name}"))?;
    let mut buffer = [0u8; 80];
    let count = value
        .copy_string_utf8(&mut buffer)
        .map_err(|error| format!("invalid {name}: {error:?}"))?;
    let decoded = buffer.get(..count).ok_or("profile string overflow")?;
    from_utf8(decoded)
        .map(str::to_owned)
        .map_err(|error| format!("invalid {name} UTF-8: {error}"))
}

fn profile_number(
    root: ValidatedProfileJsonObject<'_>,
    name: &str,
) -> Result<u32, String> {
    let value = root
        .root_member_value(name)
        .ok_or_else(|| format!("missing paper profile field: {name}"))?;
    let number = value
        .unsigned_integer()
        .ok_or_else(|| format!("{name} requires integer micrometers"))?;
    u32::try_from(number)
        .map_err(|error| format!("{name} exceeds physical range: {error}"))
}

fn decode_profile(bytes: &[u8]) -> Result<Options, String> {
    let root = validate_profile_json_object(
        bytes,
        ProfileJsonSyntaxLimits {
            maximum_bytes: MAX_PROFILE_BYTES,
            maximum_nesting: 2,
            maximum_object_members: 16,
            maximum_values: 32,
        },
    )
    .map_err(|error| format!("invalid paper profile JSON: {error:?}"))?;
    if root.root_fields().count() != 10 {
        return Err(
            "paper profile must contain exactly ten known fields".into()
        );
    }
    if profile_text(root, "schema")? != PAPER_PROFILE_SCHEMA {
        return Err("unsupported paper profile schema version".into());
    }
    let options = Options {
        color: line_color(&profile_text(root, "line_color")?)?,
        height_um: profile_number(root, "height_um")?,
        margin_bottom_um: profile_number(root, "margin_bottom_um")?,
        margin_left_um: profile_number(root, "margin_left_um")?,
        margin_right_um: profile_number(root, "margin_right_um")?,
        margin_top_um: profile_number(root, "margin_top_um")?,
        spacing_um: profile_number(root, "spacing_um")?,
        style: paper_style(&profile_text(root, "style")?)?,
        width_um: profile_number(root, "width_um")?,
    };
    // Accept only a profile that the current Rust PDF path can really render.
    let _bytes = render(options)?;
    Ok(options)
}

fn encode_profile(options: Options) -> String {
    format!(
        concat!(
            "{{\n",
            "  \"schema\": \"atrament.paper.v1\",\n",
            "  \"style\": \"{style}\",\n",
            "  \"width_um\": {width},\n",
            "  \"height_um\": {height},\n",
            "  \"spacing_um\": {spacing},\n",
            "  \"margin_top_um\": {top},\n",
            "  \"margin_bottom_um\": {bottom},\n",
            "  \"margin_left_um\": {left},\n",
            "  \"margin_right_um\": {right},\n",
            "  \"line_color\": \"#{red:02X}{green:02X}{blue:02X}\"\n",
            "}}\n"
        ),
        style = style_name(options.style),
        width = options.width_um,
        height = options.height_um,
        spacing = options.spacing_um,
        top = options.margin_top_um,
        bottom = options.margin_bottom_um,
        left = options.margin_left_um,
        right = options.margin_right_um,
        red = options.color.red,
        green = options.color.green,
        blue = options.color.blue,
    )
}

fn read_profile(path: &str) -> Result<Options, String> {
    // Preflight avoids blocking on a caller-named FIFO before File::open.
    // The shared reader still checks the actual opened descriptor.
    let source = metadata(path)
        .map_err(|error| format!("cannot inspect paper profile: {error}"))?;
    if !source.is_file() {
        return Err("paper profile file refused: NotRegularFile".into());
    }
    let file = File::open(path)
        .map_err(|error| format!("cannot open paper profile: {error}"))?;
    let bytes = read_bounded_profile_file(file, MAX_PROFILE_BYTES)
        .map_err(|error| format!("paper profile file refused: {error:?}"))?;
    decode_profile(bytes.bytes())
}

fn parse_page_count(value: &str) -> Result<usize, String> {
    if value.is_empty() || !value.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err("page count must be an integer from 1 to 32".into());
    }
    let count = value
        .parse::<usize>()
        .map_err(|error| format!("invalid page count: {error}"))?;
    if !(1..=MAX_PAGES).contains(&count) {
        return Err("page count must be an integer from 1 to 32".into());
    }
    Ok(count)
}

fn apply_paper_edit(
    opts: &mut Options,
    name: &str,
    value: &str,
    seen_size_override: &mut bool,
    seen_preset: &mut bool,
) -> Result<(), String> {
    match name {
        "--style" => opts.style = paper_style(value)?,
        "--paper-size" => {
            if *seen_size_override || *seen_preset {
                return Err(
                    "paper size must precede size overrides and occur once"
                        .into(),
                );
            }
            (opts.width_um, opts.height_um) = paper_size(value)?;
            *seen_preset = true;
        }
        "--width-mm" => {
            opts.width_um = millimetres(value)?;
            *seen_size_override = true;
        }
        "--height-mm" => {
            opts.height_um = millimetres(value)?;
            *seen_size_override = true;
        }
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
    Ok(())
}

fn parse_output_format(value: &str) -> Result<OutputFormat, String> {
    match value {
        "pdf" => Ok(OutputFormat::Pdf),
        "png" => Ok(OutputFormat::Png),
        _ => Err("--format must be pdf or png".into()),
    }
}

fn parse_png_dpi(value: &str) -> Result<u32, String> {
    let dpi = value.parse::<u32>()
        .map_err(|error| format!("--dpi requires an integer: {error}"))?;
    if !(72..=300).contains(&dpi) {
        return Err("PNG DPI must be between 72 and 300".into());
    }
    Ok(dpi)
}

fn validate_export_flags(
    format: OutputFormat,
    seen_pages: bool,
    seen_dpi: bool,
    emit_profile: bool,
) -> Result<(), String> {
    if emit_profile && seen_pages {
        return Err("--pages is only valid when exporting a PDF".into());
    }
    if format == OutputFormat::Png && seen_pages {
        return Err("--pages is only valid for PDF export".into());
    }
    if format == OutputFormat::Pdf && seen_dpi {
        return Err("--dpi requires --format png".into());
    }
    if emit_profile && format == OutputFormat::Png {
        return Err("--emit-profile cannot be combined with PNG export".into());
    }
    Ok(())
}

fn orient_page(opts: &mut Options, name: &str) {
    let wider = opts.width_um > opts.height_um;
    if (name == "--landscape" && !wider)
        || (name == "--portrait" && wider)
    {
        swap(&mut opts.width_um, &mut opts.height_um);
    }
}

fn parse_image_export_option(
    name: &str,
    value: &str,
    format: &mut OutputFormat,
    dpi: &mut u32,
    seen_format: &mut bool,
    seen_dpi: &mut bool,
) -> Result<bool, String> {
    match name {
        "--format" => {
            if *seen_format {
                return Err("--format may appear only once".into());
            }
            *format = parse_output_format(value)?;
            *seen_format = true;
            Ok(true)
        }
        "--dpi" => {
            if *seen_dpi {
                return Err("--dpi may appear only once".into());
            }
            *dpi = parse_png_dpi(value)?;
            *seen_dpi = true;
            Ok(true)
        }
        _ => Ok(false),
    }
}

fn parse_options(args: &[String]) -> Result<Option<CliRequest>, String> {
    let mut opts = Options::default();
    let mut dpi = DEFAULT_PNG_DPI;
    let mut seen_dpi = false;
    let mut format = OutputFormat::Pdf;
    let mut seen_format = false;
    let mut emit_profile = false;
    let mut seen_edit = false;
    let mut seen_profile = false;
    let mut seen_size_override = false;
    let mut seen_preset = false;
    let mut seen_orientation = false;
    let mut pages = 1;
    let mut seen_pages = false;
    let mut output = None;
    let mut replace = false;
    let mut items = args.iter();
    while let Some(name) = items.next() {
        if name == "--help" {
            return Ok(None);
        }
        if name == "--emit-profile" {
            emit_profile = true;
            continue;
        }
        if name == "--replace" {
            if replace {
                return Err("--replace may appear only once".into());
            }
            replace = true;
            continue;
        }
        if name == "--landscape" || name == "--portrait" {
            if seen_orientation {
                return Err("choose only one page orientation".into());
            }
            orient_page(&mut opts, name);
            seen_orientation = true;
            seen_size_override = true;
            seen_edit = true;
            continue;
        }
        let value = items.next().ok_or_else(|| {
            format!("missing value for {name}; use --help for arguments")
        })?;
        if name == "--profile" {
            if seen_edit || seen_profile {
                return Err("--profile must appear once, before edits".into());
            }
            opts = read_profile(value)?;
            seen_profile = true;
            continue;
        }
        if name == "--output" {
            if output.is_some() || value.starts_with('-') || value.is_empty() {
                return Err("--output requires one explicit file path".into());
            }
            output = Some(value.clone());
            continue;
        }
        if parse_image_export_option(
            name, value, &mut format, &mut dpi,
            &mut seen_format, &mut seen_dpi,
        )? {
            continue;
        }
        seen_edit = true;
        match name.as_str() {
            "--pages" => {
                if seen_pages {
                    return Err("--pages may appear only once".into());
                }
                pages = parse_page_count(value)?;
                seen_pages = true;
            }
            _ => apply_paper_edit(
                &mut opts,
                name,
                value,
                &mut seen_size_override,
                &mut seen_preset,
            )?,
        }
    }
    validate_export_flags(format, seen_pages, seen_dpi, emit_profile)?;
    if replace && output.is_none() {
        return Err("--replace requires --output".into());
    }
    Ok(Some(CliRequest {
        dpi,
        emit_profile,
        format,
        options: opts,
        output,
        pages,
        replace,
    }))
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

fn filled_square(
    x: u64,
    y: u64,
    side: u64,
    color: PdfRgb,
) -> Result<PdfVectorPath, String> {
    let right = x.checked_add(side).ok_or("number square overflow")?;
    let bottom = y.checked_add(side).ok_or("number square overflow")?;
    Ok(PdfVectorPath {
        commands: vec![
            PdfPathCommand::MoveTo(PdfPoint {
                x_um: position(x)?,
                y_um: position(y)?,
            }),
            PdfPathCommand::LineTo(PdfPoint {
                x_um: position(right)?,
                y_um: position(y)?,
            }),
            PdfPathCommand::LineTo(PdfPoint {
                x_um: position(right)?,
                y_um: position(bottom)?,
            }),
            PdfPathCommand::LineTo(PdfPoint {
                x_um: position(x)?,
                y_um: position(bottom)?,
            }),
            PdfPathCommand::Close,
        ],
        paint: PdfPathPaint::Fill {
            color,
            rule: PdfFillRule::NonZero,
        },
    })
}

fn append_page_number(
    paths: &mut Vec<PdfVectorPath>,
    opts: Options,
    page: usize,
) -> Result<(), String> {
    if !(1..=MAX_PAGES).contains(&page) {
        return Err("page number must be within 1..32".into());
    }
    if opts.margin_bottom_um < PAGE_NUMBER_BOTTOM_MARGIN_UM {
        return Err("page numbers need 7 mm bottom margin".into());
    }
    let printable_width = opts.width_um
        .checked_sub(opts.margin_left_um)
        .and_then(|value| value.checked_sub(opts.margin_right_um))
        .ok_or("page number requires valid horizontal margins")?;
    let digits = page.to_string();
    let count = u64::try_from(digits.len())
        .map_err(|error| format!("page number length refused: {error}"))?;
    let digit_width = PAGE_NUMBER_STEP_UM
        .checked_mul(2)
        .and_then(|value| value.checked_add(PAGE_NUMBER_CELL_UM))
        .ok_or("page number glyph width overflow")?;
    let total_width = count.checked_mul(digit_width)
        .and_then(|width| {
            count.checked_sub(1)
                .and_then(|gaps| gaps.checked_mul(PAGE_NUMBER_STEP_UM))
                .and_then(|gap_width| width.checked_add(gap_width))
        })
        .ok_or("page number width overflow")?;
    let height = PAGE_NUMBER_STEP_UM
        .checked_mul(4)
        .and_then(|value| value.checked_add(PAGE_NUMBER_CELL_UM))
        .ok_or("page number glyph height overflow")?;
    let left = u64::from(printable_width)
        .checked_sub(total_width)
        .and_then(|room| room.checked_div(2))
        .and_then(|offset| u64::from(opts.margin_left_um).checked_add(offset))
        .ok_or("page number exceeds printable width")?;
    let top = opts.height_um.checked_sub(opts.margin_bottom_um)
        .map(u64::from)
        .and_then(|start| {
            u64::from(opts.margin_bottom_um)
                .checked_sub(height)
                .and_then(|room| room.checked_div(2))
                .and_then(|offset| start.checked_add(offset))
        })
        .ok_or("page number exceeds bottom margin")?;
    for (digit_index, digit) in digits.bytes().enumerate() {
        let index = digit.checked_sub(b'0')
            .ok_or("invalid page-number digit")?;
        let pattern = PAGE_NUMBER_DIGITS
            .get(usize::from(index))
            .ok_or("invalid page-number digit")?;
        let x = u64::try_from(digit_index)
            .map_err(|error| format!("page digit index refused: {error}"))?
            .checked_mul(
                digit_width.checked_add(PAGE_NUMBER_STEP_UM)
                    .ok_or("page number glyph spacing overflow")?,
            )
            .and_then(|offset| left.checked_add(offset))
            .ok_or("page number digit position overflow")?;
        for (row, bits) in pattern.iter().enumerate() {
            let y = u64::try_from(row)
                .map_err(|error| format!("page number row refused: {error}"))?
                .checked_mul(PAGE_NUMBER_STEP_UM)
                .and_then(|offset| top.checked_add(offset))
                .ok_or("page number row position overflow")?;
            for column in 0..3u32 {
                if bits & (0b100 >> column) == 0 {
                    continue;
                }
                let pixel_x = u64::from(column)
                    .checked_mul(PAGE_NUMBER_STEP_UM)
                    .and_then(|offset| x.checked_add(offset))
                    .ok_or("page number column position overflow")?;
                paths.push(filled_square(
                    pixel_x, y, PAGE_NUMBER_CELL_UM, opts.color,
                )?);
            }
        }
    }
    Ok(())
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
        orientation: if opts.width_um > opts.height_um {
            Orientation::Landscape
        } else {
            Orientation::Portrait
        },
        outer_margin: Length::ZERO,
        paper_mark_appearance: PaperMarkAppearance {
            join: PaperMarkJoin::Sharp,
            maximum_ruler_error: Length::ZERO,
        },
        paper_mark_layer: PaperMarkLayer::BelowInk,
        paper_pattern: pattern,
        printable_region: region,
        sheet: if opts.width_um > opts.height_um {
            SheetSize {
                width: length(opts.height_um),
                height: length(opts.width_um),
            }
        } else {
            SheetSize {
                width: length(opts.width_um),
                height: length(opts.height_um),
            }
        },
        top_clearance: Length::ZERO,
        writing_inset: Length::ZERO,
    };
    let marks = compile_profile_marks(region, profile)
        .map_err(|error| format!("invalid physical paper: {error:?}"))?;
    Ok(marks.geometry)
}

fn render(opts: Options) -> Result<Vec<u8>, String> {
    render_pages(opts, 1)
}

fn render_pages(opts: Options, page_count: usize) -> Result<Vec<u8>, String> {
    if !(1..=MAX_PAGES).contains(&page_count) {
        return Err("paper PDF needs 1 to 32 pages".into());
    }
    let paths = paper_paths(opts)?;
    serialize_paths(opts, paths, page_count)
}

fn paper_paths(opts: Options) -> Result<Vec<PdfVectorPath>, String> {
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
    Ok(paths)
}

fn append_date_placeholder(
    paths: &mut Vec<PdfVectorPath>,
    opts: Options,
    placeholder: PaperDatePlaceholder,
) -> Result<(), String> {
    let printable_width = opts.width_um
        .checked_sub(opts.margin_left_um)
        .and_then(|remaining| remaining.checked_sub(opts.margin_right_um))
        .ok_or("date placeholder requires valid horizontal margins")?;
    if opts.margin_top_um < 10_000
        || u64::from(printable_width) < DATE_WIDTH_UM
    {
        return Err(
            "date placeholder needs 10 mm top margin and 36 mm printable width"
                .into(),
        );
    }
    let right = matches!(
        placeholder,
        PaperDatePlaceholder::RightLine | PaperDatePlaceholder::RightBox
    );
    let boxed = matches!(
        placeholder,
        PaperDatePlaceholder::LeftBox | PaperDatePlaceholder::RightBox
    );
    let x = if right {
        u64::from(opts.width_um)
            .checked_sub(u64::from(opts.margin_right_um))
            .and_then(|remaining| remaining.checked_sub(DATE_WIDTH_UM))
            .ok_or("date placeholder exceeds right printable inset")?
    } else {
        u64::from(opts.margin_left_um)
    };
    let y = u64::from(
        opts.margin_top_um.checked_sub(8_000)
            .ok_or("date placeholder exceeds top margin")?,
    ).div_ceil(2);
    let end_x = x.checked_add(DATE_WIDTH_UM)
        .ok_or("date placeholder width overflow")?;
    let end_y = y.checked_add(DATE_HEIGHT_UM)
        .ok_or("date placeholder height overflow")?;
    // Every segment is axis aligned, so the shared transparent PNG rasterizer
    // can draw the exact same physical mark as the vector PDF exporter.
    paths.push(stroke(x, end_y, end_x, end_y, opts.color)?);
    if boxed {
        paths.push(stroke(x, y, end_x, y, opts.color)?);
        paths.push(stroke(x, y, x, end_y, opts.color)?);
        paths.push(stroke(end_x, y, end_x, end_y, opts.color)?);
    }
    Ok(())
}

fn rounded_border(
    options: Options, radius_um: u32,
) -> Result<Option<RoundedPaperBorder>, String> {
    if radius_um == 0 {
        return Ok(None);
    }
    let right_um = options.width_um
        .checked_sub(options.margin_right_um)
        .ok_or("notebook border exceeds sheet width")?;
    let bottom_um = options.height_um
        .checked_sub(options.margin_bottom_um)
        .ok_or("notebook border exceeds sheet height")?;
    let width = right_um.checked_sub(options.margin_left_um)
        .ok_or("notebook border has invalid horizontal margins")?;
    let height = bottom_um.checked_sub(options.margin_top_um)
        .ok_or("notebook border has invalid vertical margins")?;
    if u64::from(radius_um).saturating_mul(2)
        > u64::from(width.min(height))
    {
        return Err("border radius exceeds half the printable area".into());
    }
    // The stroke is centered on the printable region. Keep its ink inside
    // the sheet even when page margins are reduced to near zero.
    let clearance = NOTEBOOK_BORDER_STROKE_UM.div_ceil(2);
    if options.margin_left_um < clearance
        || options.margin_right_um < clearance
        || options.margin_top_um < clearance
        || options.margin_bottom_um < clearance
    {
        return Err("rounded border needs 0.06 mm sheet-edge clearance".into());
    }
    Ok(Some(RoundedPaperBorder {
        left_um: options.margin_left_um,
        top_um: options.margin_top_um,
        right_um,
        bottom_um,
        radius_um,
        color: options.color,
    }))
}

#[expect(
    clippy::arithmetic_side_effects,
    clippy::integer_division,
    clippy::integer_division_remainder_used,
    reason = "admitted radius and page dimensions bound curve calculations",
)]
fn border_path(border: RoundedPaperBorder) -> Result<PdfVectorPath, String> {
    let left = u64::from(border.left_um);
    let top = u64::from(border.top_um);
    let right = u64::from(border.right_um);
    let bottom = u64::from(border.bottom_um);
    let radius = u64::from(border.radius_um);
    // Four cubic quarter circles, with physically bounded control points.
    let tangent = (radius * 552_285 + 500_000) / 1_000_000;
    let inner = radius - tangent;
    let point = |x: u64, y: u64| -> Result<PdfPoint, String> {
        Ok(PdfPoint { x_um: position(x)?, y_um: position(y)? })
    };
    Ok(PdfVectorPath {
        commands: vec![
            PdfPathCommand::MoveTo(point(left + radius, top)?),
            PdfPathCommand::LineTo(point(right - radius, top)?),
            PdfPathCommand::CubicTo(
                point(right - inner, top)?,
                point(right, top + inner)?,
                point(right, top + radius)?,
            ),
            PdfPathCommand::LineTo(point(right, bottom - radius)?),
            PdfPathCommand::CubicTo(
                point(right, bottom - inner)?,
                point(right - inner, bottom)?,
                point(right - radius, bottom)?,
            ),
            PdfPathCommand::LineTo(point(left + radius, bottom)?),
            PdfPathCommand::CubicTo(
                point(left + inner, bottom)?,
                point(left, bottom - inner)?,
                point(left, bottom - radius)?,
            ),
            PdfPathCommand::LineTo(point(left, top + radius)?),
            PdfPathCommand::CubicTo(
                point(left, top + inner)?,
                point(left + inner, top)?,
                point(left + radius, top)?,
            ),
            PdfPathCommand::Close,
        ],
        paint: PdfPathPaint::Stroke {
            color: border.color,
            width_um: NOTEBOOK_BORDER_STROKE_UM,
        },
    })
}

/// Check rounded printable-region borders before accepting a notebook profile.
///
/// # Errors
/// Refuses impossible radius, margins or insufficient edge clearance.
pub fn validate_paper_border_clearance(
    profile: &[u8], radius_um: u32,
) -> Result<(), String> {
    rounded_border(decode_profile(profile)?, radius_um).map(|_| ())
}

fn render_png(opts: Options, dpi: u32) -> Result<Vec<u8>, String> {
    let paths = paper_paths(opts)?;
    png::render_paper_png(opts.width_um, opts.height_um, dpi, &paths, None)
}

fn serialize_paths(
    opts: Options,
    paths: Vec<PdfVectorPath>,
    page_count: usize,
) -> Result<Vec<u8>, String> {
    serialize_page_paths(opts, vec![paths; page_count])
}

fn serialize_page_paths(
    opts: Options,
    pages: Vec<Vec<PdfVectorPath>>,
) -> Result<Vec<u8>, String> {
    let doc = PdfCompositionPlan {
        assets: Vec::new(),
        pages: pages.into_iter().map(|paths| PdfPagePlan {
            color_intent: PdfVectorColorIntent::DeviceRgb,
            physical_page_box: PdfVectorPageBox {
                width_um: opts.width_um,
                height_um: opts.height_um,
            },
            searchable_text: SearchableSemanticText::OmittedAsIncompatible,
            vector_authority: paths,
        }).collect(),
        render_manifest_identity: "paper-profile.v1",
    };
    serialize_vector_pdf(
        &doc,
        PdfVectorLimits {
            maximum_commands_per_page: 50_000,
            maximum_pages: MAX_PAGES,
            maximum_paths_per_page: 10_000,
            maximum_pdf_bytes: MAX_PAPER_OUTPUT_BYTES,
        },
    )
    .map_err(|err| format!("PDF encoding refused: {err:?}"))
}

/// Return the canonical versioned Rust default paper profile.
#[must_use]
pub fn default_paper_profile_json() -> String {
    encode_profile(Options::default())
}

/// Validate and normalize paper profile JSON into canonical versioned bytes.
///
/// # Errors
///
/// Refuses invalid JSON, unknown fields or versions, and invalid geometry.
pub fn canonical_paper_profile_json(profile: &[u8]) -> Result<String, String> {
    let options = decode_profile(profile)?;
    Ok(encode_profile(options))
}

/// Compile one bounded paper-only JSON profile into a physical vector PDF.
///
/// This endpoint does not render notebook text, handwriting or images.
///
/// # Errors
///
/// Refuses invalid JSON, version, paper geometry, or PDF budgets.
pub fn paper_profile_pdf(profile: &[u8]) -> Result<Vec<u8>, String> {
    paper_profile_pdf_pages(profile, 1, true)
}

/// Render 1–32 paper-only pages from a bounded profile.
///
/// Set `include_marks` to false for a sheet with existing printed ruling:
/// the resulting physical PDF pages contain no new paper marks. Notebook
/// metadata, date boxes, page numbers and handwriting are not rendered.
///
/// # Errors
/// Refuses invalid profiles, geometry, page counts or PDF budgets.
pub fn paper_profile_pdf_pages(
    profile: &[u8],
    pages: usize,
    include_marks: bool,
) -> Result<Vec<u8>, String> {
    paper_profile_pdf_pages_with_date(profile, pages, include_marks, None)
}

/// Validate whether a date line or box fits the existing top margin.
///
/// # Errors
/// Refuses invalid paper or less than 10 mm of top margin or 36 mm of width.
pub fn validate_paper_date_clearance(profile: &[u8]) -> Result<(), String> {
    let options = decode_profile(profile)?;
    append_date_placeholder(
        &mut Vec::new(), options, PaperDatePlaceholder::LeftLine,
    )
}

/// Render paper and an optional notebook date placeholder on each PDF page.
///
/// The date is an empty line or box in the top margin; no date text, page
/// numbers, rounded borders, images, or handwriting are rendered. A too-small
/// top margin or printable width refuses rather than overlapping the ruling.
///
/// # Errors
/// Refuses invalid profiles, missing date clearance or output limits.
pub fn paper_profile_pdf_pages_with_date(
    profile: &[u8],
    pages: usize,
    include_marks: bool,
    date: Option<PaperDatePlaceholder>,
) -> Result<Vec<u8>, String> {
    paper_profile_pdf_pages_with_annotations(
        profile, pages, include_marks, date, false,
    )
}

/// Validate that a page number can fit in the bottom margin.
///
/// # Errors
/// Refuses insufficient clearance or invalid profile geometry.
pub fn validate_paper_page_number_clearance(
    profile: &[u8],
) -> Result<(), String> {
    let options = decode_profile(profile)?;
    append_page_number(&mut Vec::new(), options, MAX_PAGES)
}

/// Render separate page-number annotations with optional dates and ruling.
///
/// Numbering begins at 1 for each export; PDF pages may differ even when
/// their source paper layer is identical. No user handwriting is synthesized.
///
/// # Errors
/// Refuses invalid geometry, page count, or insufficient bottom clearance.
pub fn paper_profile_pdf_pages_with_annotations(
    profile: &[u8],
    pages: usize,
    include_marks: bool,
    date: Option<PaperDatePlaceholder>,
    page_numbers: bool,
) -> Result<Vec<u8>, String> {
    paper_profile_pdf_pages_with_border(
        profile, pages, include_marks, date, page_numbers, 0,
    )
}

/// Render notebook annotations and an optional rounded printable border.
///
/// Radius zero keeps legacy borderless output byte-for-byte unchanged.
/// The 120-micrometre outline is independently rendered on preprinted sheets.
///
/// # Errors
/// Refuses invalid geometry, sheet-edge clearance or PDF limits.
pub fn paper_profile_pdf_pages_with_border(
    profile: &[u8],
    pages: usize,
    include_marks: bool,
    date: Option<PaperDatePlaceholder>,
    page_numbers: bool,
    border_radius_um: u32,
) -> Result<Vec<u8>, String> {
    paper_profile_pdf_pages_with_printed_date(
        profile, pages, include_marks, date, page_numbers, border_radius_um,
        None,
    )
}

/// Render notebook paper with an optional validated printed date string.
///
/// The date is rendered as legible fixed-width digits/punctuation; this does
/// not synthesize handwriting or emit searchable PDF text.
///
/// # Errors
/// Refuses impossible date values, missing placeholders or output limits.
pub fn paper_profile_pdf_pages_with_printed_date(
    profile: &[u8],
    pages: usize,
    include_marks: bool,
    date: Option<PaperDatePlaceholder>,
    page_numbers: bool,
    border_radius_um: u32,
    date_text: Option<&str>,
) -> Result<Vec<u8>, String> {
    paper_profile_pdf_pages_with_ink(
        profile,
        pages,
        include_marks,
        date,
        page_numbers,
        border_radius_um,
        date_text,
        &[],
    )
}

fn append_ink_lines(
    paths: &mut Vec<PdfVectorPath>,
    options: Options,
    page: usize,
    ink: &[NotebookInkLine],
) -> Result<(), String> {
    for line in ink.iter().filter(|line| line.page == page) {
        let clearance = line.width_um.div_ceil(2);
        let min_x = options
            .margin_left_um
            .checked_add(clearance)
            .ok_or("ink horizontal clearance overflow")?;
        let min_y = options
            .margin_top_um
            .checked_add(clearance)
            .ok_or("ink vertical clearance overflow")?;
        let max_x = options
            .width_um
            .checked_sub(options.margin_right_um)
            .and_then(|value| value.checked_sub(clearance))
            .ok_or("ink exceeds printable horizontal region")?;
        let max_y = options
            .height_um
            .checked_sub(options.margin_bottom_um)
            .and_then(|value| value.checked_sub(clearance))
            .ok_or("ink exceeds printable vertical region")?;
        if !(80..=1_500).contains(&line.width_um)
            || line.start_um == line.end_um
            || [line.start_um, line.end_um]
                .iter()
                .any(|(x, y)| {
                    !(min_x..=max_x).contains(x)
                        || !(min_y..=max_y).contains(y)
                })
        {
            return Err(format!(
                "page {page} ink needs printable endpoints and valid width"
            ));
        }
        paths.push(PdfVectorPath {
            commands: vec![
                PdfPathCommand::MoveTo(PdfPoint {
                    x_um: position(u64::from(line.start_um.0))?,
                    y_um: position(u64::from(line.start_um.1))?,
                }),
                PdfPathCommand::LineTo(PdfPoint {
                    x_um: position(u64::from(line.end_um.0))?,
                    y_um: position(u64::from(line.end_um.1))?,
                }),
            ],
            paint: PdfPathPaint::Stroke {
                color: PdfRgb {
                    red: line.rgb[0],
                    green: line.rgb[1],
                    blue: line.rgb[2],
                },
                width_um: line.width_um,
            },
        });
    }
    Ok(())
}

/// Overlay explicit physical pen segments onto normal notebook paper.
///
/// This does not shape Unicode glyphs, sample statistics, or persist strokes.
/// All requested ink must fit the selected 1–32 PDF pages.
///
/// # Errors
/// Refuses stroke limits, invalid coordinates, thickness or pages.
#[expect(
    clippy::too_many_arguments,
    reason = "extends the printed date API with ink"
)]
pub fn paper_profile_pdf_pages_with_ink(
    profile: &[u8],
    pages: usize,
    include_marks: bool,
    date: Option<PaperDatePlaceholder>,
    page_numbers: bool,
    border_radius_um: u32,
    date_text: Option<&str>,
    ink: &[NotebookInkLine],
) -> Result<Vec<u8>, String> {
    let options = decode_profile(profile)?;
    if !(1..=MAX_PAGES).contains(&pages) {
        return Err("paper PDF needs 1 to 32 pages".into());
    }
    if date.is_none() && date_text.is_some_and(|text| !text.is_empty()) {
        return Err("printed date text needs a date placeholder".into());
    }
    if ink.len() > MAX_NOTEBOOK_INK_LINES
        || ink.iter().any(|line| line.page == 0 || line.page > pages)
    {
        return Err("ink exceeds 512 segments or declared PDF pages".into());
    }
    let mut base = if include_marks {
        paper_paths(options)?
    } else {
        Vec::new()
    };
    if let Some(placeholder) = date {
        append_date_placeholder(&mut base, options, placeholder)?;
        if let Some(text) = date_text {
            date::append_date_text(&mut base, options, placeholder, text)?;
        }
    }
    if let Some(border) = rounded_border(options, border_radius_um)? {
        base.push(border_path(border)?);
    }
    if !page_numbers && ink.is_empty() {
        return serialize_paths(options, base, pages);
    }
    let mut numbered = Vec::new();
    numbered
        .try_reserve_exact(pages)
        .map_err(|error| format!("page allocation refused: {error}"))?;
    for page in 1..=pages {
        let mut paths = base.clone();
        if page_numbers {
            append_page_number(&mut paths, options, page)?;
        }
        append_ink_lines(&mut paths, options, page, ink)?;
        numbered.push(paths);
    }
    serialize_page_paths(options, numbered)
}

/// Render one versioned paper-only profile as a transparent RGBA PNG.
///
/// `dpi` determines pixel dimensions and the physical PNG resolution tag.
/// This does not render notebook text, handwriting, or embedded images.
///
/// # Errors
///
/// Refuses invalid profiles, geometry, DPI or raster/output budgets.
pub fn paper_profile_png(profile: &[u8], dpi: u32) -> Result<Vec<u8>, String> {
    paper_profile_png_layer(profile, dpi, true)
}

/// Render a physical transparent RGBA paper layer at 72–300 DPI.
///
/// For a preprinted sheet, set `include_marks` to false to obtain a fully
/// transparent, correctly sized page without duplicating existing ruling.
///
/// # Errors
/// Refuses invalid profiles, geometry, DPI or raster/output budgets.
pub fn paper_profile_png_layer(
    profile: &[u8],
    dpi: u32,
    include_marks: bool,
) -> Result<Vec<u8>, String> {
    paper_profile_png_layer_with_date(profile, dpi, include_marks, None)
}

/// Rasterize paper and an optional top-margin date line/box as RGBA PNG.
///
/// # Errors
/// Refuses invalid profiles, missing date clearance or raster/output limits.
pub fn paper_profile_png_layer_with_date(
    profile: &[u8],
    dpi: u32,
    include_marks: bool,
    date: Option<PaperDatePlaceholder>,
) -> Result<Vec<u8>, String> {
    paper_profile_png_layer_with_annotations(
        profile, dpi, include_marks, date, false,
    )
}

/// Rasterize notebook paper, optional date and first-page print number.
///
/// # Errors
/// Refuses invalid profile, missing annotation clearance or raster limits.
pub fn paper_profile_png_layer_with_annotations(
    profile: &[u8],
    dpi: u32,
    include_marks: bool,
    date: Option<PaperDatePlaceholder>,
    page_numbers: bool,
) -> Result<Vec<u8>, String> {
    paper_profile_png_page_with_annotations(
        profile, dpi, include_marks, date, page_numbers, 1,
    )
}

/// Rasterize one selected numbered notebook sheet as transparent RGBA PNG.
///
/// Pages 1–32 use the same compact printed digits as the matching PDF page.
/// Date and paper marks remain identical across pages. Page numbering is
/// optional, but the selected page must always be admitted.
///
/// # Errors
/// Refuses invalid profiles, page indices, annotation clearance or budgets.
pub fn paper_profile_png_page_with_annotations(
    profile: &[u8],
    dpi: u32,
    include_marks: bool,
    date: Option<PaperDatePlaceholder>,
    page_numbers: bool,
    page: usize,
) -> Result<Vec<u8>, String> {
    paper_profile_png_page_with_border(
        profile, dpi, include_marks, date, page_numbers, page, 0,
    )
}

/// Rasterize a selected notebook sheet with optional rounded border.
///
/// Radius zero preserves the original paper-only output; nonzero radius
/// adds the same physical outline as the PDF to transparent RGBA pixels.
///
/// # Errors
/// Refuses invalid geometry, sheet-edge clearance or raster limits.
pub fn paper_profile_png_page_with_border(
    profile: &[u8],
    dpi: u32,
    include_marks: bool,
    date: Option<PaperDatePlaceholder>,
    page_numbers: bool,
    page: usize,
    border_radius_um: u32,
) -> Result<Vec<u8>, String> {
    paper_profile_png_page_with_printed_date(
        profile, dpi, include_marks, date, page_numbers, page,
        border_radius_um, None,
    )
}

/// Render one transparent PNG page with optional printed numeric date text.
///
/// # Errors
/// Refuses unsupported dates or page/profile/raster budget errors.
#[expect(
    clippy::too_many_arguments,
    reason = "retains the existing seven-argument paper-layer API, adding date"
)]
pub fn paper_profile_png_page_with_printed_date(
    profile: &[u8],
    dpi: u32,
    include_marks: bool,
    date: Option<PaperDatePlaceholder>,
    page_numbers: bool,
    page: usize,
    border_radius_um: u32,
    date_text: Option<&str>,
) -> Result<Vec<u8>, String> {
    paper_profile_png_page_with_ink(
        profile,
        dpi,
        include_marks,
        date,
        page_numbers,
        page,
        border_radius_um,
        date_text,
        &[],
    )
}

/// Rasterize explicit page ink above a selected transparent notebook sheet.
/// Other numbered-page strokes remain available for separate PNG exports.
///
/// # Errors
/// Refuses invalid page/stroke/geometry/raster limits before output.
#[expect(
    clippy::too_many_arguments,
    reason = "extends the printed date API with ink"
)]
pub fn paper_profile_png_page_with_ink(
    profile: &[u8],
    dpi: u32,
    include_marks: bool,
    date: Option<PaperDatePlaceholder>,
    page_numbers: bool,
    page: usize,
    border_radius_um: u32,
    date_text: Option<&str>,
    ink: &[NotebookInkLine],
) -> Result<Vec<u8>, String> {
    if !(1..=MAX_PAGES).contains(&page) {
        return Err("PNG notebook page must be within 1..32".into());
    }
    if ink.len() > MAX_NOTEBOOK_INK_LINES
        || ink.iter().any(|line| !(1..=MAX_PAGES).contains(&line.page))
    {
        return Err("ink exceeds 512 segments or valid PNG pages".into());
    }
    if date.is_none() && date_text.is_some_and(|text| !text.is_empty()) {
        return Err("printed date text needs a date placeholder".into());
    }
    let options = decode_profile(profile)?;
    let mut paths = if include_marks {
        paper_paths(options)?
    } else {
        Vec::new()
    };
    if let Some(placeholder) = date {
        append_date_placeholder(&mut paths, options, placeholder)?;
        if let Some(text) = date_text {
            date::append_date_text(&mut paths, options, placeholder, text)?;
        }
    }
    if page_numbers {
        append_page_number(&mut paths, options, page)?;
    }
    // Refuse invalid off-page ink as well: selecting PNG page 1 must not
    // silently accept impossible marks intended for another page.
    for requested_page in 1..=MAX_PAGES {
        if requested_page == page {
            append_ink_lines(&mut paths, options, requested_page, ink)?;
        } else {
            append_ink_lines(&mut Vec::new(), options, requested_page, ink)?;
        }
    }
    let border = rounded_border(options, border_radius_um)?;
    png::render_paper_png(
        options.width_um, options.height_um, dpi, &paths, border,
    )
}

fn execute(request: CliRequest) -> Result<Option<Vec<u8>>, String> {
    // Admission and rendering complete before any selected target can change.
    let bytes = if request.emit_profile {
        let _validated = render_pages(request.options, 1)?;
        encode_profile(request.options).into_bytes()
    } else {
        match request.format {
            OutputFormat::Pdf => render_pages(request.options, request.pages)?,
            OutputFormat::Png => render_png(request.options, request.dpi)?,
        }
    };
    let Some(output) = request.output else {
        return Ok(Some(bytes));
    };
    let disposition = if request.replace {
        ProfileFileCommitDisposition::ReplaceExisting
    } else {
        ProfileFileCommitDisposition::PreserveExisting
    };
    let _evidence = commit_profile_file(
        Path::new(&output),
        &bytes,
        MAX_PAPER_OUTPUT_BYTES,
        disposition,
    )
        .map_err(|error| {
            if error.may_have_committed() {
                format!(
                    "output durability unknown; inspect before retry: {error:?}"
                )
            } else {
                format!("output not saved: {error:?}")
            }
        })?;
    Ok(None)
}

/// Run the shared Rust paper CLI with caller-provided arguments.
///
/// # Errors
///
/// Invalid parameters or output errors abort before reporting success.
pub fn run_cli_args(args: &[String]) -> Result<(), Box<dyn Error>> {
    let Some(request) = parse_options(args)? else {
        writeln!(io::stdout().lock(), "{USAGE}")?;
        return Ok(());
    };
    if let Some(bytes) = execute(request)? {
        io::stdout().lock().write_all(&bytes)?;
    }
    Ok(())
}

/// Run the standalone paper CLI from process arguments.
///
/// # Errors
///
/// Invalid parameters or output errors abort before reporting success.
pub fn run_cli() -> Result<(), Box<dyn Error>> {
    let args: Vec<_> = env::args().skip(1).collect();
    run_cli_args(&args)
}

#[cfg(test)]
#[path = "../../../../tests/backend/notebook-paper-cli/composition/lib.rs"]
mod tests;
