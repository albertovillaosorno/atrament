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

use std::fmt::{Arguments, Write as _};

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
    axis: Option<PdfPoint>,
    first: Option<PdfPoint>,
    spans_plane: bool,
}

impl ContourAreaWitness {
    fn include(&mut self, point: PdfPoint) {
        let Some(first) = self.first else {
            self.first = Some(point);
            return;
        };
        if let Some(axis) = self.axis {
            if self.spans_plane {
                return;
            }
            let base_x =
                i64::from(axis.x_um).saturating_sub(i64::from(first.x_um));
            let base_y =
                i64::from(axis.y_um).saturating_sub(i64::from(first.y_um));
            let point_x =
                i64::from(point.x_um).saturating_sub(i64::from(first.x_um));
            let point_y =
                i64::from(point.y_um).saturating_sub(i64::from(first.y_um));
            self.spans_plane = base_x.saturating_mul(point_y)
                != base_y.saturating_mul(point_x);
        } else if point != first {
            self.axis = Some(point);
        }
    }

    fn include_cubic(&mut self, points: [PdfPoint; 3]) {
        for point in points {
            self.include(point);
        }
    }
}

// Formatting into String cannot normally fail, but retain typed refusal if
// the sink reports an error rather than silently losing a path command.
fn push_formatted(
    stream: &mut String,
    arguments: Arguments<'_>,
) -> Result<(), PdfVectorError> {
    stream
        .write_fmt(arguments)
        .map_err(|_error| PdfVectorError::AllocationFailed)
}

fn points(value_um: i32) -> String {
    format!("{:.3}", f64::from(value_um) * PDF_POINTS_PER_UM)
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

fn point_inside(
    point: PdfPoint,
    page: PdfVectorPageBox,
    stroke_margin_um: u32,
) -> bool {
    let right = page.width_um.checked_sub(stroke_margin_um);
    let bottom = page.height_um.checked_sub(stroke_margin_um);
    u32::try_from(point.x_um).is_ok_and(|x| {
        right.is_some_and(|right_edge| x >= stroke_margin_um && x <= right_edge)
    }) && u32::try_from(point.y_um).is_ok_and(|y| {
        bottom.is_some_and(|bottom_edge| {
            y >= stroke_margin_um && y <= bottom_edge
        })
    })
}

fn apply_path_paint(
    stream: &mut String,
    paint: PdfPathPaint,
    maximum_stream_bytes: usize,
) -> Result<(&'static str, u32), PdfVectorError> {
    let (operator, stroke_margin_um) = match paint {
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
            ("S\n", width_um.div_ceil(2))
        },
    };
    if stream.len() > maximum_stream_bytes {
        return Err(PdfVectorError::ResourceLimit);
    }
    Ok((operator, stroke_margin_um))
}

fn render_path(
    stream: &mut String,
    path: &PdfVectorPath,
    page: PdfVectorPageBox,
    maximum_stream_bytes: usize,
) -> Result<(), PdfVectorError> {
    if path.commands.is_empty() {
        return Err(PdfVectorError::InvalidPath);
    }
    let (operator, stroke_margin_um) =
        apply_path_paint(stream, path.paint, maximum_stream_bytes)?;
    let fill = matches!(path.paint, PdfPathPaint::Fill { .. });
    let mut active = false;
    let mut drawn = false;
    let mut saw_segment = false;
    let mut contour_area = ContourAreaWitness::default();
    for command in &path.commands {
        match *command {
            PdfPathCommand::MoveTo(point) => {
                if !point_inside(point, page, stroke_margin_um)
                    || (active && !drawn)
                {
                    return Err(PdfVectorError::InvalidPath);
                }
                if active && fill {
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
                contour_area = ContourAreaWitness::default();
                contour_area.include(point);
            },
            PdfPathCommand::LineTo(point) if active => {
                if !point_inside(point, page, stroke_margin_um) {
                    return Err(PdfVectorError::InvalidPath);
                }
                push_formatted(
                    stream,
                    format_args!(
                        "{} {} l\n",
                        points(point.x_um),
                        points(point.y_um)
                    ),
                )?;
                drawn = true;
                saw_segment = true;
                if fill {
                    contour_area.include(point);
                }
            },
            PdfPathCommand::CubicTo(first, second, last) if active => {
                if !point_inside(first, page, stroke_margin_um)
                    || !point_inside(second, page, stroke_margin_um)
                    || !point_inside(last, page, stroke_margin_um)
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
                )?;
                drawn = true;
                saw_segment = true;
                if fill {
                    contour_area.include_cubic([first, second, last]);
                }
            },
            PdfPathCommand::Close if active && drawn => {
                if fill && !contour_area.spans_plane {
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
        if stream.len() > maximum_stream_bytes {
            return Err(PdfVectorError::ResourceLimit);
        }
    }
    if !saw_segment || (active && !drawn) || (fill && active) {
        return Err(PdfVectorError::InvalidPath);
    }
    stream.push_str(operator);
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
    if page.vector_authority.len() > limits.maximum_paths_per_page {
        return Err(PdfVectorError::ResourceLimit);
    }
    let mut command_count = 0usize;
    let mut stream =
        format!("q\n1 0 0 -1 0 {} cm\n", page_points(dimensions.height_um));
    for path in &page.vector_authority {
        command_count = command_count
            .checked_add(path.commands.len())
            .ok_or(PdfVectorError::ResourceLimit)?;
        if command_count > limits.maximum_commands_per_page {
            return Err(PdfVectorError::ResourceLimit);
        }
        render_path(&mut stream, path, dimensions, limits.maximum_pdf_bytes)?;
        if stream.len() > limits.maximum_pdf_bytes {
            return Err(PdfVectorError::ResourceLimit);
        }
    }
    stream.push_str("Q\n");
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

fn append_xref_and_trailer(
    bytes: &mut Vec<u8>,
    offsets: &[usize],
    info_object: usize,
    maximum_pdf_bytes: usize,
) -> Result<(), PdfVectorError> {
    let limit = maximum_pdf_bytes;
    let startxref = bytes.len();
    append(
        bytes,
        format!(
            "xref\n0 {}\n0000000000 65535 f \n",
            offsets.len().saturating_add(1)
        )
        .as_bytes(),
        limit,
    )?;
    for offset in offsets {
        append(bytes, format!("{offset:010} 00000 n \n").as_bytes(), limit)?;
    }
    append(
        bytes,
        format!(
            "trailer\n<< /Size {} /Root 1 0 R /Info {info_object} 0 R >>\n\
             startxref\n{startxref}\n%%EOF\n",
            offsets.len().saturating_add(1),
        )
        .as_bytes(),
        limit,
    )?;
    Ok(())
}

// Every PDF page stream is embedded verbatim in the final output. Stop
// retaining them as soon as their sum exceeds the entire PDF byte budget;
// otherwise many individually bounded pages can exhaust memory first.
fn render_bounded_streams(
    composition: &PdfVectorComposition<'_>,
    limits: PdfVectorLimits,
) -> Result<Vec<String>, PdfVectorError> {
    let mut streams = Vec::new();
    let mut cumulative_bytes = 0usize;
    for page in &composition.pages {
        let stream = render_page(page, limits)?;
        cumulative_bytes = cumulative_bytes
            .checked_add(stream.len())
            .ok_or(PdfVectorError::ResourceLimit)?;
        if cumulative_bytes > limits.maximum_pdf_bytes {
            return Err(PdfVectorError::ResourceLimit);
        }
        streams
            .try_reserve(1)
            .map_err(|_error| PdfVectorError::AllocationFailed)?;
        streams.push(stream);
    }
    Ok(streams)
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
    let streams = render_bounded_streams(composition, limits)?;
    let mut bytes = Vec::new();
    let mut offsets = Vec::new();
    let limit = limits.maximum_pdf_bytes;
    append(&mut bytes, b"%PDF-1.4\n%\xD0\xD4\xC5\xD8\n", limit)?;
    append_object(
        &mut bytes,
        &mut offsets,
        b"1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n",
        limit,
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
        limit,
    )?;
    for (index, (page, stream)) in
        composition.pages.iter().zip(&streams).enumerate()
    {
        let object = 3usize.saturating_add(index.saturating_mul(2));
        let width = page_points(page.physical_page_box.width_um);
        let height = page_points(page.physical_page_box.height_um);
        append_object(
            &mut bytes,
            &mut offsets,
            format!(
                "{object} 0 obj\n<< /Type /Page /Parent 2 0 R \
                 /MediaBox [0 0 {width} {height}] /Resources << >> \
                 /Contents {} 0 R >>\nendobj\n",
                object.saturating_add(1),
            )
            .as_bytes(),
            limit,
        )?;
        append_object(
            &mut bytes,
            &mut offsets,
            format!(
                "{} 0 obj\n<< /Length {} >>\nstream\n{}endstream\nendobj\n",
                object.saturating_add(1),
                stream.len(),
                stream,
            )
            .as_bytes(),
            limit,
        )?;
    }
    let info_object =
        3usize.saturating_add(composition.pages.len().saturating_mul(2));
    append_object(
        &mut bytes,
        &mut offsets,
        format!(
            "{info_object} 0 obj\n<< /Producer (Atrament vector-only PDF v1) \
             /AtramentManifestID ({identity}) >>\nendobj\n",
        )
        .as_bytes(),
        limit,
    )?;
    append_xref_and_trailer(&mut bytes, &offsets, info_object, limit)?;
    Ok(bytes)
}
