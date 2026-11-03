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
//   - Bounded transparent PNG projection of compiled Rust paper marks.
// - Must-Not:
//   - Infer handwriting, embed images, implement general vector rasterization,
//     persist output, or replace PDF vector serialization authority.
// - Allows:
//   - Inputs: Physical dimensions, explicit DPI, bounded paper mark geometry.
//   - Outputs: One RGBA PNG with physical resolution and transparent canvas.
//   - Side effects: Bounded in-memory allocations only.
// - Split-When:
//   - Production ink/image PNG rendering requires a general raster backend.
// - Merge-When:
//   - The paper-only preview becomes part of a production renderer.
// - Summary:
//   - Projects the same Rust paper marks used for PDF into transparent pixels.
// - Description:
//   - Converts admitted ruled, grid, blank and dotted paper geometry to
//     8-bit RGBA PNG with a bounded stored-DEFLATE stream.
// - Usage:
//   - Called by the existing paper-only native CLI's PNG option.
// - Defaults:
//   - Unmarked pixels stay fully transparent; no white underpainting.
//

//! Paper-only transparent raster export, without external image subprocesses.

use atrament_pdf_vector_serializer::{
    PdfPathCommand, PdfPathPaint, PdfPoint, PdfRgb, PdfVectorPath,
};

use super::{MAX_OUTPUT_BYTES, NOTEBOOK_BORDER_STROKE_UM, RoundedPaperBorder};

const MICROMETRES_PER_INCH: u64 = 25_400;
const MAX_PIXELS: usize = 5_000_000;

#[derive(Clone, Copy)]
struct Rectangle {
    bottom: i64,
    left: i64,
    right: i64,
    top: i64,
}

const fn rectangular_corners(
    first: PdfPoint,
    second: PdfPoint,
    third: PdfPoint,
    fourth: PdfPoint,
) -> bool {
    first.x_um == fourth.x_um
        && first.y_um == second.y_um
        && second.x_um == third.x_um
        && third.y_um == fourth.y_um
        && first.x_um != third.x_um
        && first.y_um != third.y_um
}

fn rectangle_for_path(
    path: &PdfVectorPath,
) -> Result<(Rectangle, PdfRgb), String> {
    match (path.commands.as_slice(), path.paint) {
        (
            [PdfPathCommand::MoveTo(first), PdfPathCommand::LineTo(last)],
            PdfPathPaint::Stroke { color, width_um },
        ) if width_um > 0
            && (first.x_um == last.x_um || first.y_um == last.y_um) =>
        {
            let radius = i64::from(width_um.div_ceil(2));
            Ok((
                Rectangle {
                    bottom: i64::from(first.y_um.max(last.y_um))
                        .saturating_add(radius),
                    left: i64::from(first.x_um.min(last.x_um))
                        .saturating_sub(radius),
                    right: i64::from(first.x_um.max(last.x_um))
                        .saturating_add(radius),
                    top: i64::from(first.y_um.min(last.y_um))
                        .saturating_sub(radius),
                },
                color,
            ))
        }
        (
            [
                PdfPathCommand::MoveTo(first),
                PdfPathCommand::LineTo(second),
                PdfPathCommand::LineTo(third),
                PdfPathCommand::LineTo(fourth),
                PdfPathCommand::Close,
            ],
            PdfPathPaint::Fill { color, .. },
        ) if rectangular_corners(*first, *second, *third, *fourth) => Ok((
            Rectangle {
                bottom: i64::from(first.y_um.max(third.y_um)),
                left: i64::from(first.x_um.min(third.x_um)),
                right: i64::from(first.x_um.max(third.x_um)),
                top: i64::from(first.y_um.min(third.y_um)),
            },
            color,
        )),
        _ => {
            Err("PNG supports only the current paper lines and square dots"
                .into())
        }
    }
}

fn pixel_coordinate(
    micrometres: i64,
    dpi: u32,
    ceil: bool,
) -> Result<usize, String> {
    let physical = u64::try_from(micrometres.max(0))
        .map_err(|error| format!("PNG coordinate refused: {error}"))?;
    let scaled = physical
        .checked_mul(u64::from(dpi))
        .ok_or("PNG coordinate overflow")?;
    let rounded = if ceil {
        scaled
            .checked_add(MICROMETRES_PER_INCH - 1)
            .ok_or("PNG rounded coordinate overflow")?
    } else {
        scaled
    };
    let pixels = rounded
        .checked_div(MICROMETRES_PER_INCH)
        .ok_or("PNG resolution cannot be zero")?;
    usize::try_from(pixels)
        .map_err(|error| format!("PNG coordinate refused: {error}"))
}

// Only paper marks are rasterized: axis-aligned strokes and square dots.
// Keeping unmarked pixels RGBA(0,0,0,0) permits digital overlays.
fn paint_rectangle(
    pixels: &mut [u8],
    width: usize,
    height: usize,
    dpi: u32,
    rectangle: Rectangle,
    color: PdfRgb,
) -> Result<(), String> {
    let left = pixel_coordinate(rectangle.left, dpi, false)?.min(width);
    let top = pixel_coordinate(rectangle.top, dpi, false)?.min(height);
    let right = pixel_coordinate(rectangle.right, dpi, true)?.min(width);
    let bottom = pixel_coordinate(rectangle.bottom, dpi, true)?.min(height);
    for row in top..bottom {
        for column in left..right {
            let pixel_index = row
                .checked_mul(width)
                .and_then(|offset| offset.checked_add(column))
                .and_then(|pixel| pixel.checked_mul(4))
                .ok_or("PNG pixel offset overflow")?;
            let end = pixel_index
                .checked_add(4)
                .ok_or("PNG channel offset overflow")?;
            let pixel = pixels
                .get_mut(pixel_index..end)
                .ok_or("PNG channel access outside raster")?;
            pixel.copy_from_slice(&[color.red, color.green, color.blue, 255]);
        }
    }
    Ok(())
}

// Unlike the rectangular paper marks, the optional notebook border is a
// rounded contour. Its signed distance is evaluated at physical pixel centers
// so its circular corners use the same admitted geometry at any DPI. The
// half-pixel margin prevents hairline strokes from disappearing at 72 DPI.
fn paint_rounded_border(
    pixels: &mut [u8],
    width: usize,
    height: usize,
    dpi: u32,
    border: RoundedPaperBorder,
) -> Result<(), String> {
    let pitch = 25_400.0f64 / f64::from(dpi);
    let half_width = (f64::from(border.right_um)
        - f64::from(border.left_um)) / 2.0f64;
    let half_height = (f64::from(border.bottom_um)
        - f64::from(border.top_um)) / 2.0f64;
    let center_x = f64::midpoint(
        f64::from(border.left_um), f64::from(border.right_um),
    );
    let center_y = f64::midpoint(
        f64::from(border.top_um), f64::from(border.bottom_um),
    );
    let radius = f64::from(border.radius_um);
    let threshold = f64::from(NOTEBOOK_BORDER_STROKE_UM) / 2.0f64
        + pitch / 2.0f64;
    let mut y = pitch / 2.0f64;
    for row in 0..height {
        if (y - center_y).abs() > half_height + threshold {
            y += pitch;
            continue;
        }
        let mut x = pitch / 2.0f64;
        for column in 0..width {
            if (x - center_x).abs() <= half_width + threshold {
                let dx = (x - center_x).abs() - (half_width - radius);
                let dy = (y - center_y).abs() - (half_height - radius);
                let distance = dx.max(0.0).hypot(dy.max(0.0))
                    + dx.max(dy).min(0.0) - radius;
                if distance.abs() <= threshold {
                    let offset = row.checked_mul(width)
                        .and_then(|index| index.checked_add(column))
                        .and_then(|index| index.checked_mul(4))
                        .ok_or("border raster offset overflow")?;
                    let end = offset.checked_add(4)
                        .ok_or("border channel offset overflow")?;
                    let pixel = pixels.get_mut(offset..end)
                        .ok_or("border raster exceeds pixel budget")?;
                    pixel.copy_from_slice(&[
                        border.color.red, border.color.green,
                        border.color.blue, 255,
                    ]);
                }
            }
            x += pitch;
        }
        y += pitch;
    }
    Ok(())
}

fn crc32_part(mut crc: u32, data: &[u8]) -> u32 {
    for byte in data {
        crc ^= u32::from(*byte);
        for _ in 0u32..8u32 {
            crc = (crc >> 1u32)
                ^ (0xedb8_8320u32 & 0u32.wrapping_sub(crc & 1u32));
        }
    }
    crc
}

fn checksum_adler32(data: &[u8]) -> u32 {
    let mut first = 1u32;
    let mut second = 0u32;
    // Reduce each accumulated sum modulo 65_521 before a u32 can overflow.
    for chunk in data.chunks(2_000) {
        for byte in chunk {
            first = first.saturating_add(u32::from(*byte));
            second = second.saturating_add(first);
        }
        first %= 65_521;
        second %= 65_521;
    }
    (second << 16) | first
}

fn png_chunk(
    output: &mut Vec<u8>,
    tag: [u8; 4],
    data: &[u8],
) -> Result<(), String> {
    let len = u32::try_from(data.len())
        .map_err(|error| format!("PNG chunk too large: {error}"))?;
    let required = output
        .len()
        .checked_add(data.len().saturating_add(12))
        .ok_or("PNG output size overflow")?;
    if required > MAX_OUTPUT_BYTES {
        return Err("PNG exceeds output byte limit".into());
    }
    output.extend_from_slice(&len.to_be_bytes());
    output.extend_from_slice(&tag);
    output.extend_from_slice(data);
    // PNG CRC includes chunk type and data but excludes the length field.
    let crc = !crc32_part(crc32_part(!0u32, &tag), data);
    output.extend_from_slice(&crc.to_be_bytes());
    Ok(())
}

fn encode_rgba(
    width: u32,
    height: u32,
    dpi: u32,
    pixels: &[u8],
) -> Result<Vec<u8>, String> {
    let stride = usize::try_from(width)
        .map_err(|error| format!("PNG width refused: {error}"))?
        .checked_mul(4)
        .ok_or("PNG row size overflow")?;
    let raw_len = stride
        .checked_add(1)
        .and_then(|size| size.checked_mul(usize::try_from(height).ok()?))
        .ok_or("PNG raster size overflow")?;
    let mut raw = Vec::new();
    raw.try_reserve_exact(raw_len)
        .map_err(|error| format!("PNG raster allocation refused: {error}"))?;
    for scanline in pixels.chunks_exact(stride) {
        raw.push(0); // PNG filter type None.
        raw.extend_from_slice(scanline);
    }
    let blocks = raw.len().div_ceil(usize::from(u16::MAX));
    let compression_bytes = raw
        .len()
        .checked_add(blocks.saturating_mul(5))
        .and_then(|size| size.checked_add(6))
        .ok_or("PNG compression size overflow")?;
    if compression_bytes.saturating_add(100) > MAX_OUTPUT_BYTES {
        return Err("PNG exceeds output byte limit".into());
    }
    let mut zlib = Vec::new();
    zlib.try_reserve_exact(compression_bytes).map_err(|error| {
        format!("PNG compression allocation refused: {error}")
    })?;
    zlib.extend_from_slice(&[0x78, 0x01]); // zlib stream, stored DEFLATE.
    for (index, block) in raw.chunks(usize::from(u16::MAX)).enumerate() {
        zlib.push(u8::from(index.saturating_add(1) == blocks));
        let len = u16::try_from(block.len())
            .map_err(|error| format!("PNG block refused: {error}"))?;
        zlib.extend_from_slice(&len.to_le_bytes());
        zlib.extend_from_slice(&(!len).to_le_bytes());
        zlib.extend_from_slice(block);
    }
    zlib.extend_from_slice(&checksum_adler32(&raw).to_be_bytes());

    let mut output = Vec::new();
    output
        .try_reserve_exact(zlib.len().saturating_add(100))
        .map_err(|error| format!("PNG output allocation refused: {error}"))?;
    output.extend_from_slice(b"\x89PNG\r\n\x1a\n");
    let mut ihdr = Vec::with_capacity(13);
    ihdr.extend_from_slice(&width.to_be_bytes());
    ihdr.extend_from_slice(&height.to_be_bytes());
    ihdr.extend_from_slice(&[8, 6, 0, 0, 0]); // 8-bit RGBA, no interlace.
    png_chunk(&mut output, *b"IHDR", &ihdr)?;
    let pixels_per_metre = u64::from(dpi)
        .saturating_mul(10_000)
        .saturating_add(127)
        .checked_div(254)
        .ok_or("PNG resolution refused")?;
    let ppm = u32::try_from(pixels_per_metre)
        .map_err(|error| format!("PNG resolution refused: {error}"))?;
    let mut phys = Vec::with_capacity(9);
    phys.extend_from_slice(&ppm.to_be_bytes());
    phys.extend_from_slice(&ppm.to_be_bytes());
    phys.push(1); // Pixels per metre.
    png_chunk(&mut output, *b"pHYs", &phys)?;
    png_chunk(&mut output, *b"IDAT", &zlib)?;
    png_chunk(&mut output, *b"IEND", &[])?;
    Ok(output)
}

/// Rasterize one paper-only page from shared Rust paper geometry.
///
/// # Errors
///
/// Refuses invalid DPI, oversized raster/output or unsupported path shapes.
#[expect(
    clippy::redundant_pub_crate,
    reason = "child module exports only to its parent, not as a public API"
)]
pub(super) fn render_paper_png(
    width_um: u32,
    height_um: u32,
    dpi: u32,
    paths: &[PdfVectorPath],
    border: Option<RoundedPaperBorder>,
) -> Result<Vec<u8>, String> {
    if !(72..=300).contains(&dpi) {
        return Err("PNG DPI must be between 72 and 300".into());
    }
    let width = pixel_coordinate(i64::from(width_um), dpi, true)?;
    let height = pixel_coordinate(i64::from(height_um), dpi, true)?;
    let pixels = width
        .checked_mul(height)
        .ok_or("PNG pixel count overflow")?;
    if width == 0 || height == 0 || pixels > MAX_PIXELS {
        return Err("PNG pixel budget exceeded; lower the DPI".into());
    }
    let bytes = pixels.checked_mul(4).ok_or("PNG raster byte overflow")?;
    let mut raster = Vec::new();
    raster
        .try_reserve_exact(bytes)
        .map_err(|error| format!("PNG raster allocation refused: {error}"))?;
    raster.resize(bytes, 0);
    for path in paths {
        let (rectangle, color) = rectangle_for_path(path)?;
        paint_rectangle(&mut raster, width, height, dpi, rectangle, color)?;
    }
    if let Some(outline) = border {
        paint_rounded_border(&mut raster, width, height, dpi, outline)?;
    }
    encode_rgba(
        u32::try_from(width)
            .map_err(|error| format!("PNG width refused: {error}"))?,
        u32::try_from(height)
            .map_err(|error| format!("PNG height refused: {error}"))?,
        dpi,
        &raster,
    )
}
