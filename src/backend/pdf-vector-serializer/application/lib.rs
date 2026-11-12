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
//   - Bounded PDF 1.4 byte serialization of already resolved vector paths.
// - Must-Not:
//   - Choose layout, perform text shaping, rasterize, write files, infer
//     authorization, or discard accepted assets/searchable semantic text.
// - Allows:
//   - Inputs: Explicit vector-only PDF composition and output byte limits.
//   - Outputs: Deterministic PDF bytes or typed refusal before file effects.
//   - Side effects: Bounded process-local allocation only.
// - Split-When:
//   - Text, images, or color profiles require independent resource adapters.
// - Merge-When:
//   - Complete PDF composition can subsume this narrow serialization codec.
// - Summary:
//   - Proves real vector PDF bytes can be emitted without a file export claim.
// - Description:
//   - Requires resolved physical paths and refuses unsupported resources.
// - Usage:
//   - Consume only validated vector page plans after future export preflight.
// - Defaults:
//   - No implicit assets, searchable text, or unbounded output allocation.
//

//! Deterministic in-memory PDF 1.4 for vector-only, fully resolved pages.

use std::fmt;

use atrament_pdf_composition_plan::{
    PdfAssetResource, PdfCompositionPlan, PdfPagePlan, SearchableSemanticText,
};

const MAX_PAGE_SIZE_UM: u32 = 2_000_000;
const MAX_MANIFEST_ID_BYTES: usize = 128;
const PDF_POINTS_PER_UM: f64 = 72.0 / 25_400.0;

/// Explicit RGB color for an already resolved vector path.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PdfRgb {
    /// Blue sRGB channel.
    pub blue: u8,
    /// Green sRGB channel.
    pub green: u8,
    /// Red sRGB channel.
    pub red: u8,
}

/// This adapter supports only uncalibrated device RGB, not a color profile.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PdfVectorColorIntent {
    /// Use PDF `DeviceRGB` for colored paths.
    DeviceRgb,
}

/// Exact physical page size in micrometers (not pixels or CSS units).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PdfVectorPageBox {
    /// Positive page height in micrometers.
    pub height_um: u32,
    /// Positive page width in micrometers.
    pub width_um: u32,
}

/// Absolute point in micrometers, measured from the top-left page corner.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PdfPoint {
    /// Horizontal offset in micrometers.
    pub x_um: i32,
    /// Vertical offset in micrometers.
    pub y_um: i32,
}

/// Resolved PDF path geometry; curves use exact caller-owned control points.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PdfPathCommand {
    /// Close the current subpath.
    Close,
    /// Draw a cubic Bézier from current point to the third control point.
    CubicTo(PdfPoint, PdfPoint, PdfPoint),
    /// Draw a straight segment from the current point.
    LineTo(PdfPoint),
    /// Start a new subpath.
    MoveTo(PdfPoint),
}

/// Explicit winding interpretation of closed vector contours.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PdfFillRule {
    /// Even-odd parity preserves holes without winding-orientation inference.
    EvenOdd,
    /// Nonzero winding preserves the supplied contour orientations.
    NonZero,
}

/// A vector paint with no implicit style or fill-rule fallback.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PdfPathPaint {
    /// Filled, fully closed subpaths with caller-selected winding semantics.
    Fill {
        /// Exact solid fill color.
        color: PdfRgb,
        /// Rule for combining nested contours and holes.
        rule: PdfFillRule,
    },
    /// Solid stroke with positive width in micrometers.
    Stroke {
        /// Exact stroke ink color.
        color: PdfRgb,
        /// Exact stroke width in micrometers.
        width_um: u32,
    },
}

/// One path already accepted by a vector renderer or semantic converter.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PdfVectorPath {
    /// Nonempty sequence of valid path commands.
    pub commands: Vec<PdfPathCommand>,
    /// Explicit paint used for this path only.
    pub paint: PdfPathPaint,
}

/// Vector-only projection of the transport-neutral PDF composition plan.
///
/// Asset entries and preserved searchable text are refused rather than lost.
/// Manifest identity is a constrained identifier in PDF Info, not the complete
/// reproducibility manifest. No filesystem Export capability is implied.
pub type PdfVectorComposition<'identity> = PdfCompositionPlan<
    PdfPagePlan<PdfVectorColorIntent, PdfVectorPageBox, (), Vec<PdfVectorPath>>,
    PdfAssetResource<&'identity str>,
    &'identity str,
>;

/// Independent budgets for resource-safe byte serialization.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PdfVectorLimits {
    /// Maximum total drawing commands on any page.
    pub maximum_commands_per_page: usize,
    /// Maximum accepted page count.
    pub maximum_pages: usize,
    /// Maximum paths on any one page.
    pub maximum_paths_per_page: usize,
    /// Maximum total serialized PDF bytes.
    pub maximum_pdf_bytes: usize,
}

/// Safe, credential-free refusal before output bytes are published.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PdfVectorError {
    /// An output buffer allocation failed.
    AllocationFailed,
    /// A caller supplied no pages.
    EmptyDocument,
    /// The manifest identity is not a bounded ASCII identifier.
    InvalidManifestIdentity,
    /// Page dimensions are invalid or exceed the supported physical limit.
    InvalidPageBox,
    /// A drawing path is malformed, empty, or extends outside the page.
    InvalidPath,
    /// The path stroke has no positive, bounded width.
    InvalidStrokeWidth,
    /// A page, command, or output byte budget was exceeded.
    ResourceLimit,
    /// An asset would be silently lost with this vector-only codec.
    UnsupportedAssets,
    /// Searchable semantic text requires a separate font/text resource codec.
    UnsupportedSearchableText,
}

// A filled contour with all line endpoints and curve controls on one axis
// cannot enclose ink. This is a necessary area witness, not a general
// intersection, winding, or image-coverage predicate; crossing contours may
// have zero signed area while still legitimately painting visible regions.
#[derive(Default)]
struct ContourAreaWitness {
    axis: Option<(i64, i64)>,
    first: Option<(i64, i64)>,
    spans_plane: bool,
}

impl ContourAreaWitness {
    fn include(&mut self, point: (i64, i64)) {
        let Some(first) = self.first else {
            self.first = Some(point);
            return;
        };
        if let Some(axis) = self.axis {
            if self.spans_plane {
                return;
            }
            let base_x = axis.0.saturating_sub(first.0);
            let base_y = axis.1.saturating_sub(first.1);
            let point_x = point.0.saturating_sub(first.0);
            let point_y = point.1.saturating_sub(first.1);
            self.spans_plane = base_x.saturating_mul(point_y)
                != base_y.saturating_mul(point_x);
        } else if point != first {
            self.axis = Some(point);
        }
    }

    fn include_physical(&mut self, point: PdfPoint) {
        self.include((i64::from(point.x_um), i64::from(point.y_um)));
    }

    fn include_serialized(&mut self, point: PdfPoint) {
        self.include((
            serialized_millipoints(point.x_um),
            serialized_millipoints(point.y_um),
        ));
    }
}

// A filled contour must span a plane both at source micrometer precision and
// after PDF coordinate rounding. Otherwise valid physical geometry could
// silently collapse into an unpaintable serialized line.
#[derive(Default)]
struct FilledContourWitness {
    physical: ContourAreaWitness,
    serialized: ContourAreaWitness,
}

impl FilledContourWitness {
    fn include(&mut self, point: PdfPoint) {
        self.physical.include_physical(point);
        self.serialized.include_serialized(point);
    }

    fn include_cubic(&mut self, points: [PdfPoint; 3]) {
        for point in points {
            self.include(point);
        }
    }

    const fn spans_plane(&self) -> bool {
        self.physical.spans_plane && self.serialized.spans_plane
    }
}

// Formatting into String cannot normally fail, but retain typed refusal if
// the sink reports an error rather than silently losing a path command.
fn push_formatted(
    stream: &mut String,
    arguments: fmt::Arguments<'_>,
) -> Result<(), PdfVectorError> {
    fmt::write(stream, arguments)
        .map_err(|_error| PdfVectorError::AllocationFailed)
}

fn points(value_um: i32) -> String {
    format!("{:.3}", f64::from(value_um) * PDF_POINTS_PER_UM)
}

// PDF content coordinates are rounded to exactly three decimal places.
// The positive page-admitted coordinates use the same nearest-integer ratio
// as points(); there are no ties because the denominator is odd.
fn serialized_millipoints(value_um: i32) -> i64 {
    i64::from(value_um)
        .saturating_mul(360)
        .saturating_add(63)
        .div_euclid(127)
}

fn page_points(value_um: u32) -> String {
    format!("{:.3}", f64::from(value_um) * PDF_POINTS_PER_UM)
}

fn color_channels(color: PdfRgb) -> String {
    format!(
        "{:.3} {:.3} {:.3}",
        f64::from(color.red) / 255.0,
        f64::from(color.green) / 255.0,
        f64::from(color.blue) / 255.0,
    )
}

// PDF numbers are rounded to three decimal points. In thousandths of a
// point, 1 micrometer is exactly 360/127, and an exact half-way tie is
// impossible because the denominator is odd. Match the serialized numbers
// rather than accepting a stroke that the PDF quantization clips.
#[expect(
    clippy::integer_division,
    clippy::integer_division_remainder_used,
    reason = "fixed-point rounding must match emitted PDF numbers"
)]
fn pdf_millipoints(value_um: u32) -> u64 {
    u64::from(value_um).saturating_mul(360).saturating_add(63) / 127
}

fn point_inside(
    point: PdfPoint,
    page: PdfVectorPageBox,
    stroke_width_um: u32,
) -> bool {
    let stroke_margin_um = stroke_width_um.div_ceil(2);
    let Some(right) = page.width_um.checked_sub(stroke_margin_um) else {
        return false;
    };
    let Some(bottom) = page.height_um.checked_sub(stroke_margin_um) else {
        return false;
    };
    let (Ok(x), Ok(y)) = (u32::try_from(point.x_um), u32::try_from(point.y_um))
    else {
        return false;
    };
    if !(x >= stroke_margin_um
        && x <= right
        && y >= stroke_margin_um
        && y <= bottom)
    {
        return false;
    }
    if stroke_width_um == 0 {
        return true;
    }
    let width = pdf_millipoints(stroke_width_um);
    let page_width = pdf_millipoints(page.width_um);
    let page_height = pdf_millipoints(page.height_um);
    let x_points = pdf_millipoints(x);
    let y_points = pdf_millipoints(y);
    width
        <= x_points
            .min(page_width.saturating_sub(x_points))
            .saturating_mul(2)
        && width
            <= y_points
                .min(page_height.saturating_sub(y_points))
                .saturating_mul(2)
}

/// Check physical point clearance after the vector PDF's three-decimal
/// point and stroke-width serialization. A stroke that fits in micrometres
/// may still clip at the rounded PDF page edge.
///
/// Pass a zero stroke width for filled paths; fill edges can touch the sheet.
/// Transparent raster projections can use this same admission rule so PNG
/// does not accept marks that would be refused in physical PDF output.
#[must_use]
pub fn pdf_painted_point_inside_page(
    point: PdfPoint,
    page: PdfVectorPageBox,
    stroke_width_um: u32,
) -> bool {
    point_inside(point, page, stroke_width_um)
}

fn apply_path_paint(
    stream: &mut String,
    paint: PdfPathPaint,
    maximum_stream_bytes: usize,
) -> Result<(&'static str, u32), PdfVectorError> {
    let (operator, stroke_width_um) = match paint {
        PdfPathPaint::Fill { color, rule } => {
            push_formatted(
                stream,
                format_args!("{} rg\n", color_channels(color)),
            )?;
            let operator = match rule {
                PdfFillRule::EvenOdd => "f*\n",
                PdfFillRule::NonZero => "f\n",
            };
            (operator, 0)
        },
        PdfPathPaint::Stroke { color, width_um } => {
            // Reserve a minimal color, positive width, cap/join setup and
            // paint operator before checking malformed width authority.
            // Otherwise an unpaintable output could report InvalidStrokeWidth
            // even though no possible valid stroke would fit its byte budget.
            let minimum_paint_bytes =
                b"0.000 0.000 0.000 RG\n0.003 w\n1 J\n1 j\nS\n".len();
            if stream
                .len()
                .checked_add(minimum_paint_bytes)
                .is_none_or(|size| size > maximum_stream_bytes)
            {
                return Err(PdfVectorError::ResourceLimit);
            }
            if width_um == 0 || width_um > MAX_PAGE_SIZE_UM {
                return Err(PdfVectorError::InvalidStrokeWidth);
            }
            push_formatted(
                stream,
                format_args!(
                    "{} RG\n{} w\n1 J\n1 j\n",
                    color_channels(color),
                    page_points(width_um)
                ),
            )?;
            ("S\n", width_um)
        },
    };
    if stream.len() > maximum_stream_bytes {
        return Err(PdfVectorError::ResourceLimit);
    }
    Ok((operator, stroke_width_um))
}

// Keep the path writer focused on subpath admission; the cubic command
// independently checks its convex-hull control points against the page.
fn append_cubic_command(
    stream: &mut String,
    first: PdfPoint,
    second: PdfPoint,
    last: PdfPoint,
    page: PdfVectorPageBox,
    stroke_width_um: u32,
) -> Result<(), PdfVectorError> {
    if [first, second, last]
        .into_iter()
        .any(|point| !point_inside(point, page, stroke_width_um))
    {
        return Err(PdfVectorError::InvalidPath);
    }
    push_formatted(
        stream,
        format_args!(
            "{} {} {} {} {} {} c\n",
            points(first.x_um),
            points(first.y_um),
            points(second.x_um),
            points(second.y_um),
            points(last.x_um),
            points(last.y_um),
        ),
    )
}

fn append_line_command(
    stream: &mut String,
    point: PdfPoint,
    current: PdfPoint,
    page: PdfVectorPageBox,
    stroke_width_um: u32,
    fill: bool,
) -> Result<(), PdfVectorError> {
    if (!fill && point == current)
        || !point_inside(point, page, stroke_width_um)
    {
        return Err(PdfVectorError::InvalidPath);
    }
    push_formatted(
        stream,
        format_args!("{} {} l\n", points(point.x_um), points(point.y_um)),
    )
}

// In-page PDF coordinates use three decimal places, but a variable-width
// integer part. Reserve their exact textual width; invalid coordinates retain
// the shortest valid spelling so geometry still has its own refusal path.
fn minimum_coordinate_bytes(value_um: i32, bound_um: u32) -> usize {
    let Ok(value) = u32::try_from(value_um) else {
        return b"0.000".len();
    };
    if value > bound_um {
        return b"0.000".len();
    }
    let mut integer_part = pdf_millipoints(value).div_euclid(1_000);
    let mut digits = 1usize;
    while integer_part >= 10 {
        integer_part = integer_part.div_euclid(10);
        digits = digits.saturating_add(1);
    }
    digits.saturating_add(b".000".len())
}

fn minimum_point_bytes(point: PdfPoint, page: PdfVectorPageBox) -> usize {
    minimum_coordinate_bytes(point.x_um, page.width_um)
        .saturating_add(b" ".len())
        .saturating_add(minimum_coordinate_bytes(point.y_um, page.height_um))
}

fn minimum_command_bytes(
    command: PdfPathCommand,
    page: PdfVectorPageBox,
) -> usize {
    match command {
        PdfPathCommand::MoveTo(point) => {
            minimum_point_bytes(point, page).saturating_add(b" m\n".len())
        },
        PdfPathCommand::LineTo(point) => {
            minimum_point_bytes(point, page).saturating_add(b" l\n".len())
        },
        PdfPathCommand::CubicTo(first, second, last) => {
            minimum_point_bytes(first, page)
                .saturating_add(b" ".len())
                .saturating_add(minimum_point_bytes(second, page))
                .saturating_add(b" ".len())
                .saturating_add(minimum_point_bytes(last, page))
                .saturating_add(b" c\n".len())
        },
        PdfPathCommand::Close => b"h\n".len(),
    }
}

fn reserve_command_framing(
    stream_bytes: usize,
    command: PdfPathCommand,
    page: PdfVectorPageBox,
    command_budget: usize,
) -> Result<(), PdfVectorError> {
    if stream_bytes
        .checked_add(minimum_command_bytes(command, page))
        .is_none_or(|size| size > command_budget)
    {
        return Err(PdfVectorError::ResourceLimit);
    }
    Ok(())
}

// Reserve later command syntax before inspecting the first path coordinate.
fn reserve_path_command_framing(
    stream_bytes: usize,
    commands: &[PdfPathCommand],
    page: PdfVectorPageBox,
    command_budget: usize,
) -> Result<(), PdfVectorError> {
    let minimum_path_bytes = commands
        .iter()
        .try_fold(0usize, |total, command| {
            total.checked_add(minimum_command_bytes(*command, page))
        })
        .ok_or(PdfVectorError::ResourceLimit)?;
    if stream_bytes
        .checked_add(minimum_path_bytes)
        .is_none_or(|size| size > command_budget)
    {
        return Err(PdfVectorError::ResourceLimit);
    }
    if commands.is_empty() {
        return Err(PdfVectorError::InvalidPath);
    }
    Ok(())
}

fn render_path(
    stream: &mut String,
    path: &PdfVectorPath,
    page: PdfVectorPageBox,
    maximum_stream_bytes: usize,
) -> Result<(), PdfVectorError> {
    let (operator, stroke_width_um) =
        apply_path_paint(stream, path.paint, maximum_stream_bytes)?;
    // Both paint state and the final operator are mandatory even if the
    // path contains malformed or missing drawing commands.
    let command_budget = maximum_stream_bytes
        .checked_sub(operator.len())
        .filter(|budget| stream.len() <= *budget)
        .ok_or(PdfVectorError::ResourceLimit)?;
    reserve_path_command_framing(
        stream.len(), &path.commands, page, command_budget,
    )?;
    let fill = matches!(path.paint, PdfPathPaint::Fill { .. });
    let mut active = false;
    let mut drawn = false;
    let mut saw_segment = false;
    let mut current = PdfPoint { x_um: 0, y_um: 0 };
    let mut subpath_has_extent = false;
    let mut contour_area = FilledContourWitness::default();
    for command in &path.commands {
        reserve_command_framing(stream.len(), *command, page, command_budget)?;
        match *command {
            PdfPathCommand::MoveTo(point) => {
                if !point_inside(point, page, stroke_width_um)
                    || (active && !drawn)
                {
                    return Err(PdfVectorError::InvalidPath);
                }
                if active && (fill || !subpath_has_extent) {
                    return Err(PdfVectorError::InvalidPath);
                }
                push_formatted(
                    stream,
                    format_args!(
                        "{} {} m\n",
                        points(point.x_um),
                        points(point.y_um)
                    ),
                )?;
                active = true;
                drawn = false;
                current = point;
                subpath_has_extent = false;
                contour_area = FilledContourWitness::default();
                contour_area.include(point);
            },
            PdfPathCommand::LineTo(point) if active => {
                append_line_command(
                    stream, point, current, page, stroke_width_um, fill,
                )?;
                drawn = true;
                saw_segment = true;
                subpath_has_extent |= current != point;
                current = point;
                if fill {
                    contour_area.include(point);
                }
            },
            PdfPathCommand::CubicTo(first, second, last) if active => {
                if !fill && first == current && second == current
                    && last == current
                {
                    return Err(PdfVectorError::InvalidPath);
                }
                append_cubic_command(
                    stream, first, second, last, page, stroke_width_um,
                )?;
                drawn = true;
                saw_segment = true;
                subpath_has_extent |=
                    first != current || second != current || last != current;
                current = last;
                if fill {
                    contour_area.include_cubic([first, second, last]);
                }
            },
            PdfPathCommand::Close if active && drawn => {
                if (fill && !contour_area.spans_plane())
                    || (!fill && !subpath_has_extent)
                {
                    return Err(PdfVectorError::InvalidPath);
                }
                stream.push_str("h\n");
                active = false;
                drawn = false;
            },
            PdfPathCommand::LineTo(_)
            | PdfPathCommand::CubicTo(..)
            | PdfPathCommand::Close => {
                return Err(PdfVectorError::InvalidPath);
            },
        }
        if stream.len() > command_budget {
            return Err(PdfVectorError::ResourceLimit);
        }
    }
    if !saw_segment || (active && (!drawn || !subpath_has_extent))
        || (fill && active)
    {
        return Err(PdfVectorError::InvalidPath);
    }
    stream.push_str(operator);
    Ok(())
}

// Every path spends a paint setup/operator and its command syntax. Reserve
// all later paths before validating earlier geometry on the same page.
fn minimum_path_framing_bytes(
    path: &PdfVectorPath,
    page: PdfVectorPageBox,
) -> Option<usize> {
    let paint_bytes = match path.paint {
        PdfPathPaint::Fill { rule, .. } => match rule {
            PdfFillRule::EvenOdd => b"0.000 0.000 0.000 rg\nf*\n".len(),
            PdfFillRule::NonZero => b"0.000 0.000 0.000 rg\nf\n".len(),
        },
        PdfPathPaint::Stroke { width_um, .. } => {
            let width_bytes = i32::try_from(width_um)
                .ok()
                .map_or(b"0.003".len(), |value| {
                    minimum_coordinate_bytes(value, MAX_PAGE_SIZE_UM)
                });
            b"0.000 0.000 0.000 RG\n0.003 w\n1 J\n1 j\nS\n"
                .len()
                .saturating_add(width_bytes.saturating_sub(b"0.003".len()))
        },
    };
    path.commands
        .iter()
        .try_fold(paint_bytes, |total, command| {
            total.checked_add(minimum_command_bytes(*command, page))
        })
}

// Count limits apply to the whole composition before any earlier page ink
// is inspected. A malformed page must not hide an oversized later page.
fn enforce_page_count_limits(
    paths: &[PdfVectorPath],
    limits: PdfVectorLimits,
) -> Result<(), PdfVectorError> {
    if paths.len() > limits.maximum_paths_per_page {
        return Err(PdfVectorError::ResourceLimit);
    }
    let command_count = paths.iter().try_fold(
        0usize,
        |total, path| total.checked_add(path.commands.len()),
    ).ok_or(PdfVectorError::ResourceLimit)?;
    if command_count > limits.maximum_commands_per_page {
        return Err(PdfVectorError::ResourceLimit);
    }
    Ok(())
}

// Page authority is checked for the entire composition before rendering any
// paths. An earlier invalid stroke must not hide later unsupported text or an
// invalid physical page box under an otherwise sufficient output allowance.
const fn validate_page_metadata(
    page: &PdfPagePlan<
        PdfVectorColorIntent,
        PdfVectorPageBox,
        (),
        Vec<PdfVectorPath>,
    >,
) -> Result<(), PdfVectorError> {
    if matches!(page.searchable_text, SearchableSemanticText::Preserved(())) {
        return Err(PdfVectorError::UnsupportedSearchableText);
    }
    let dimensions = page.physical_page_box;
    if dimensions.width_um == 0
        || dimensions.height_um == 0
        || dimensions.width_um > MAX_PAGE_SIZE_UM
        || dimensions.height_um > MAX_PAGE_SIZE_UM
    {
        return Err(PdfVectorError::InvalidPageBox);
    }
    Ok(())
}

fn render_page(
    page: &PdfPagePlan<
        PdfVectorColorIntent,
        PdfVectorPageBox,
        (),
        Vec<PdfVectorPath>,
    >,
    limits: PdfVectorLimits,
) -> Result<String, PdfVectorError> {
    validate_page_metadata(page)?;
    enforce_page_count_limits(&page.vector_authority, limits)?;
    let dimensions = page.physical_page_box;
    let mut stream =
        format!("q\n1 0 0 -1 0 {} cm\n", page_points(dimensions.height_um));
    // A page cannot be emitted without closing its graphics state. Reserve
    // that fixed terminator before inspecting any path under the byte limit.
    let drawing_budget = limits
        .maximum_pdf_bytes
        .checked_sub(b"Q\n".len())
        .ok_or(PdfVectorError::ResourceLimit)?;
    // Minimal syntax for the *whole page* belongs to the stream budget,
    // even when a malformed first path would refuse before later paint.
    let minimum_paths = page.vector_authority.iter().try_fold(
        0usize,
        |total, path| {
            total.checked_add(minimum_path_framing_bytes(path, dimensions)?)
        },
    ).ok_or(PdfVectorError::ResourceLimit)?;
    if stream.len().checked_add(minimum_paths)
        .is_none_or(|size| size > drawing_budget)
    {
        return Err(PdfVectorError::ResourceLimit);
    }
    for path in &page.vector_authority {
        render_path(&mut stream, path, dimensions, drawing_budget)?;
        if stream.len() > drawing_budget {
            return Err(PdfVectorError::ResourceLimit);
        }
    }
    stream.push_str("Q\n");
    if stream.len() > limits.maximum_pdf_bytes {
        return Err(PdfVectorError::ResourceLimit);
    }
    Ok(stream)
}

fn append(
    bytes: &mut Vec<u8>,
    chunk: &[u8],
    maximum_pdf_bytes: usize,
) -> Result<(), PdfVectorError> {
    let total = bytes
        .len()
        .checked_add(chunk.len())
        .ok_or(PdfVectorError::ResourceLimit)?;
    if total > maximum_pdf_bytes || u32::try_from(total).is_err() {
        return Err(PdfVectorError::ResourceLimit);
    }
    bytes
        .try_reserve(chunk.len())
        .map_err(|_error| PdfVectorError::AllocationFailed)?;
    bytes.extend_from_slice(chunk);
    Ok(())
}

fn append_object(
    bytes: &mut Vec<u8>,
    offsets: &mut Vec<usize>,
    object: &[u8],
    maximum_pdf_bytes: usize,
) -> Result<(), PdfVectorError> {
    offsets.push(bytes.len());
    append(bytes, object, maximum_pdf_bytes)
}

// Keep a completed page stream borrowed while serializing its PDF object.
// Formatting the whole object at once would allocate another full-size copy
// of the stream before the caller's final byte limit could be enforced.
fn append_stream_object(
    bytes: &mut Vec<u8>,
    offsets: &mut Vec<usize>,
    object: usize,
    stream: &str,
    maximum_pdf_bytes: usize,
) -> Result<(), PdfVectorError> {
    append_object(
        bytes,
        offsets,
        format!(
            "{object} 0 obj\n<< /Length {} >>\nstream\n",
            stream.len(),
        )
        .as_bytes(),
        maximum_pdf_bytes,
    )?;
    append(bytes, stream.as_bytes(), maximum_pdf_bytes)?;
    append(bytes, b"endstream\nendobj\n", maximum_pdf_bytes)
}

// The final Info, cross-reference rows and trailer are mandatory even
// though they come after every page. Reserve their *exact* byte allowance
// before rendering paths, including startxref decimal-width transitions.
fn xref_header(offset_count: usize) -> String {
    format!(
        "xref\n0 {}\n0000000000 65535 f \n",
        offset_count.saturating_add(1),
    )
}

fn xref_trailer(
    offset_count: usize,
    info_object: usize,
    startxref: usize,
) -> String {
    format!(
        "trailer\n<< /Size {} /Root 1 0 R /Info {info_object} 0 R >>\n\
         startxref\n{startxref}\n%%EOF\n",
        offset_count.saturating_add(1),
    )
}

fn complete_pdf_size(
    prefix_bytes: usize,
    info_bytes: usize,
    info_object: usize,
) -> Option<usize> {
    let startxref = prefix_bytes.checked_add(info_bytes)?;
    prefix_bytes
        .checked_add(info_bytes)?
        .checked_add(xref_header(info_object).len())?
        // Every accepted offset fits exactly ten PDF decimal digits.
        .checked_add(info_object.checked_mul(b"0000000000 00000 n \n".len())?)?
        .checked_add(xref_trailer(info_object, info_object, startxref).len())
}

fn reserve_document_tail(
    maximum_pdf_bytes: usize,
    info_bytes: usize,
    info_object: usize,
) -> Result<usize, PdfVectorError> {
    // PDF 1.4 xref entries use ten decimal digits; append() also enforces
    // this u32 ceiling on every intermediate buffer.
    let document_limit =
        maximum_pdf_bytes.min(usize::try_from(u32::MAX).unwrap_or(usize::MAX));
    if complete_pdf_size(0, info_bytes, info_object)
        .is_none_or(|size| size > document_limit)
    {
        return Err(PdfVectorError::ResourceLimit);
    }
    let mut low = 0usize;
    let mut high = document_limit;
    while low < high {
        let middle = low.saturating_add(high.saturating_sub(low).div_ceil(2));
        if complete_pdf_size(middle, info_bytes, info_object)
            .is_some_and(|size| size <= document_limit)
        {
            low = middle;
        } else {
            high = middle.saturating_sub(1);
        }
    }
    Ok(low)
}

fn append_xref_and_trailer(
    bytes: &mut Vec<u8>,
    offsets: &[usize],
    info_object: usize,
    maximum_pdf_bytes: usize,
) -> Result<(), PdfVectorError> {
    let limit = maximum_pdf_bytes;
    let startxref = bytes.len();
    append(bytes, xref_header(offsets.len()).as_bytes(), limit)?;
    for offset in offsets {
        append(bytes, format!("{offset:010} 00000 n \n").as_bytes(), limit)?;
    }
    append(
        bytes,
        xref_trailer(offsets.len(), info_object, startxref).as_bytes(),
        limit,
    )?;
    Ok(())
}

// Keep minimum object framing consistent with the exact generated page header
// and the page transform. This is required for every page in the PDF, even
// when there are no authored drawing paths.
fn page_object_header(dimensions: PdfVectorPageBox, object: usize) -> String {
    let content_object = object.saturating_add(1);
    let width = page_points(dimensions.width_um);
    let height = page_points(dimensions.height_um);
    format!(
        "{object} 0 obj\n<< /Type /Page /Parent 2 0 R \
         /MediaBox [0 0 {width} {height}] /Resources << >> \
         /Contents {content_object} 0 R >>\nendobj\n",
    )
}

fn minimum_page_stream(dimensions: PdfVectorPageBox) -> String {
    format!(
        "q\n1 0 0 -1 0 {} cm\nQ\n",
        page_points(dimensions.height_um)
    )
}

fn minimum_page_object_bytes(
    dimensions: PdfVectorPageBox,
    object: usize,
    paths: &[PdfVectorPath],
) -> Option<usize> {
    let content_object = object.checked_add(1)?;
    let minimum_stream_bytes = paths.iter().try_fold(
        minimum_page_stream(dimensions).len(),
        |total, path| {
            total.checked_add(minimum_path_framing_bytes(path, dimensions)?)
        },
    )?;
    page_object_header(dimensions, object)
        .len()
        .checked_add(
            format!(
                "{content_object} 0 obj\n<< /Length \
                 {minimum_stream_bytes} >>\nstream\n"
            )
            .len(),
        )?
        .checked_add(minimum_stream_bytes)?
        .checked_add(b"endstream\nendobj\n".len())
}

// A page consumes its own PDF object and stream dictionary/terminator in
// addition to the drawing instructions. Reserve the mandatory empty-page
// framing before examining potentially malformed geometry.
fn reserve_page_object_framing(
    dimensions: PdfVectorPageBox,
    object: usize,
    remaining_bytes: usize,
) -> Result<(String, usize), PdfVectorError> {
    let content_object = object.saturating_add(1);
    let page_header = page_object_header(dimensions, object);
    let minimum_stream = minimum_page_stream(dimensions);
    // The decimal /Length field grows with the rendered stream. Reserving
    // only the empty-stream header would give paths one extra byte at 100,
    // 1_000, etc., allowing invalid geometry to mask a ResourceLimit.
    let content_header_without_length =
        format!("{content_object} 0 obj\n<< /Length  >>\nstream\n");
    let stream_and_length_budget = remaining_bytes
        .checked_sub(page_header.len())
        .and_then(|remaining| {
            remaining.checked_sub(content_header_without_length.len())
        })
        .and_then(|remaining| {
            remaining.checked_sub(b"endstream\nendobj\n".len())
        })
        .ok_or(PdfVectorError::ResourceLimit)?;
    // Find the largest stream length whose decimal width and data fit.
    // The predicate is monotonic, including decimal-width transitions.
    let mut low = 0usize;
    let mut high = stream_and_length_budget;
    while low < high {
        let middle = low.saturating_add(high.saturating_sub(low).div_ceil(2));
        let digits = middle.to_string().len();
        if middle
            .checked_add(digits)
            .is_some_and(|total| total <= stream_and_length_budget)
        {
            low = middle;
        } else {
            high = middle.saturating_sub(1);
        }
    }
    let stream_budget = low;
    if stream_budget < minimum_stream.len() {
        return Err(PdfVectorError::ResourceLimit);
    }
    Ok((page_header, stream_budget))
}

// Admit and append each page while retaining the complete framing for later
// pages; the document Info and xref allowance is already reserved by caller.
fn append_pages(
    bytes: &mut Vec<u8>,
    offsets: &mut Vec<usize>,
    composition: &PdfVectorComposition<'_>,
    limits: PdfVectorLimits,
    page_limit: usize,
) -> Result<(), PdfVectorError> {
    // Reserve every page's mandatory object, paint and command syntax, and
    // its resulting /Length field, before inspecting any earlier geometry.
    // A malformed page 1 cannot mask an unfinishable later drawn page.
    let mut remaining_page_minima = composition
        .pages
        .iter()
        .enumerate()
        .try_fold(0usize, |total, (index, page)| {
            enforce_page_count_limits(&page.vector_authority, limits)?;
            let object = 3usize.saturating_add(index.saturating_mul(2));
            let minimum = minimum_page_object_bytes(
                page.physical_page_box,
                object,
                &page.vector_authority,
            ).ok_or(PdfVectorError::ResourceLimit)?;
            total.checked_add(minimum).ok_or(PdfVectorError::ResourceLimit)
        })?;
    if remaining_page_minima > page_limit.saturating_sub(bytes.len()) {
        return Err(PdfVectorError::ResourceLimit);
    }
    // Once mandatory document-wide bytes are accounted for, preflight every
    // page's typed authority before inspecting any drawing coordinates.
    // Malformed earlier ink cannot conceal unsupported text or an invalid
    // physical page box on a later page.
    for page in &composition.pages {
        validate_page_metadata(page)?;
    }
    // Bound each page by the remaining final-output allowance *less* later
    // pages' mandatory framing. Accepted pages keep their exact original bytes.
    for (index, page) in composition.pages.iter().enumerate() {
        let object = 3usize.saturating_add(index.saturating_mul(2));
        remaining_page_minima = remaining_page_minima
            .checked_sub(
                minimum_page_object_bytes(
                    page.physical_page_box, object, &page.vector_authority,
                )
                    .ok_or(PdfVectorError::ResourceLimit)?,
            )
            .ok_or(PdfVectorError::ResourceLimit)?;
        let remaining_bytes = page_limit
            .checked_sub(bytes.len())
            .and_then(|available| available.checked_sub(remaining_page_minima))
            .filter(|remaining| *remaining > 0)
            .ok_or(PdfVectorError::ResourceLimit)?;
        let content_object = object.saturating_add(1);
        let (page_header, stream_budget) = reserve_page_object_framing(
            page.physical_page_box,
            object,
            remaining_bytes,
        )?;
        let page_limits = PdfVectorLimits {
            maximum_pdf_bytes: stream_budget,
            ..limits
        };
        let stream = render_page(page, page_limits)?;
        append_object(
            bytes,
            offsets,
            page_header.as_bytes(),
            page_limit,
        )?;
        append_stream_object(
            bytes,
            offsets,
            content_object,
            &stream,
            page_limit,
        )?;
    }
    Ok(())
}

/// Serialize a complete vector-only PDF without touching filesystem paths.
///
/// The input must already contain accepted path coordinates in micrometers.
/// The projection rejects preserved text and *every* asset (including safe
/// references); no font, image, profile, notebook layout or Export behavior is
/// provided. Caller-owned render identity is linked via PDF Info metadata.
///
/// # Errors
///
/// Returns a typed refusal for invalid paths, unsupported resources, invalid
/// physical page boxes, or any exceeded page/command/byte limit.
pub fn serialize_vector_pdf(
    composition: &PdfVectorComposition<'_>,
    limits: PdfVectorLimits,
) -> Result<Vec<u8>, PdfVectorError> {
    if composition.pages.is_empty() {
        return Err(PdfVectorError::EmptyDocument);
    }
    if composition.pages.len() > limits.maximum_pages
        || composition.pages.len() > 1024
    {
        return Err(PdfVectorError::ResourceLimit);
    }
    if !composition.assets.is_empty() {
        return Err(PdfVectorError::UnsupportedAssets);
    }
    let identity = composition.render_manifest_identity;
    if identity.is_empty()
        || identity.len() > MAX_MANIFEST_ID_BYTES
        || !identity.bytes().all(|byte| {
            byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.')
        })
    {
        return Err(PdfVectorError::InvalidManifestIdentity);
    }
    let info_object =
        3usize.saturating_add(composition.pages.len().saturating_mul(2));
    let info = format!(
        "{info_object} 0 obj\n<< /Producer (Atrament vector-only PDF v1) \
         /AtramentManifestID ({identity}) >>\nendobj\n",
    );
    let limit = limits.maximum_pdf_bytes;
    let page_limit = reserve_document_tail(limit, info.len(), info_object)?;
    let mut bytes = Vec::new();
    let mut offsets = Vec::new();
    append(&mut bytes, b"%PDF-1.4\n%\xD0\xD4\xC5\xD8\n", page_limit)?;
    append_object(
        &mut bytes,
        &mut offsets,
        b"1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n",
        page_limit,
    )?;
    let mut kids = String::new();
    for index in 0..composition.pages.len() {
        push_formatted(
            &mut kids,
            format_args!(
                "{} 0 R ",
                3usize.saturating_add(index.saturating_mul(2))
            ),
        )?;
    }
    append_object(
        &mut bytes,
        &mut offsets,
        format!(
            "2 0 obj\n<< /Type /Pages /Kids [{kids}] /Count {} >>\nendobj\n",
            composition.pages.len(),
        )
        .as_bytes(),
        page_limit,
    )?;
    append_pages(&mut bytes, &mut offsets, composition, limits, page_limit)?;
    append_object(&mut bytes, &mut offsets, info.as_bytes(), limit)?;
    append_xref_and_trailer(&mut bytes, &offsets, info_object, limit)?;
    Ok(bytes)
}
