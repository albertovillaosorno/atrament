// Copyright:
//   - Copyright © 2026 Alberto Villa Osorno.
// SPDX-License-Identifier:
//   - LicenseRef-Proprietary
// Confidential:
//   - true
// License-File:
//   - LICENSE
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

use std::iter;

use atrament_pdf_vector_serializer::{
    PdfPathCommand, PdfPathPaint, PdfPoint, PdfRgb, PdfVectorPath,
};

use super::{MAX_OUTPUT_BYTES, NOTEBOOK_BORDER_STROKE_UM, RoundedPaperBorder};

const MICROMETRES_PER_INCH: u64 = 25_400;
const MAX_PIXELS: usize = 5_000_000;

// Standard reflected PNG CRC-32 lookup values, computed for each byte.
// Decoding tests recompute checksums independently using bitwise CRC.
const CRC32_TABLE: [u32; 256] = [
    0x0000_0000, 0x7707_3096, 0xee0e_612c, 0x9909_51ba, 0x076d_c419,
    0x706a_f48f, 0xe963_a535, 0x9e64_95a3, 0x0edb_8832, 0x79dc_b8a4,
    0xe0d5_e91e, 0x97d2_d988, 0x09b6_4c2b, 0x7eb1_7cbd, 0xe7b8_2d07,
    0x90bf_1d91, 0x1db7_1064, 0x6ab0_20f2, 0xf3b9_7148, 0x84be_41de,
    0x1ada_d47d, 0x6ddd_e4eb, 0xf4d4_b551, 0x83d3_85c7, 0x136c_9856,
    0x646b_a8c0, 0xfd62_f97a, 0x8a65_c9ec, 0x1401_5c4f, 0x6306_6cd9,
    0xfa0f_3d63, 0x8d08_0df5, 0x3b6e_20c8, 0x4c69_105e, 0xd560_41e4,
    0xa267_7172, 0x3c03_e4d1, 0x4b04_d447, 0xd20d_85fd, 0xa50a_b56b,
    0x35b5_a8fa, 0x42b2_986c, 0xdbbb_c9d6, 0xacbc_f940, 0x32d8_6ce3,
    0x45df_5c75, 0xdcd6_0dcf, 0xabd1_3d59, 0x26d9_30ac, 0x51de_003a,
    0xc8d7_5180, 0xbfd0_6116, 0x21b4_f4b5, 0x56b3_c423, 0xcfba_9599,
    0xb8bd_a50f, 0x2802_b89e, 0x5f05_8808, 0xc60c_d9b2, 0xb10b_e924,
    0x2f6f_7c87, 0x5868_4c11, 0xc161_1dab, 0xb666_2d3d, 0x76dc_4190,
    0x01db_7106, 0x98d2_20bc, 0xefd5_102a, 0x71b1_8589, 0x06b6_b51f,
    0x9fbf_e4a5, 0xe8b8_d433, 0x7807_c9a2, 0x0f00_f934, 0x9609_a88e,
    0xe10e_9818, 0x7f6a_0dbb, 0x086d_3d2d, 0x9164_6c97, 0xe663_5c01,
    0x6b6b_51f4, 0x1c6c_6162, 0x8565_30d8, 0xf262_004e, 0x6c06_95ed,
    0x1b01_a57b, 0x8208_f4c1, 0xf50f_c457, 0x65b0_d9c6, 0x12b7_e950,
    0x8bbe_b8ea, 0xfcb9_887c, 0x62dd_1ddf, 0x15da_2d49, 0x8cd3_7cf3,
    0xfbd4_4c65, 0x4db2_6158, 0x3ab5_51ce, 0xa3bc_0074, 0xd4bb_30e2,
    0x4adf_a541, 0x3dd8_95d7, 0xa4d1_c46d, 0xd3d6_f4fb, 0x4369_e96a,
    0x346e_d9fc, 0xad67_8846, 0xda60_b8d0, 0x4404_2d73, 0x3303_1de5,
    0xaa0a_4c5f, 0xdd0d_7cc9, 0x5005_713c, 0x2702_41aa, 0xbe0b_1010,
    0xc90c_2086, 0x5768_b525, 0x206f_85b3, 0xb966_d409, 0xce61_e49f,
    0x5ede_f90e, 0x29d9_c998, 0xb0d0_9822, 0xc7d7_a8b4, 0x59b3_3d17,
    0x2eb4_0d81, 0xb7bd_5c3b, 0xc0ba_6cad, 0xedb8_8320, 0x9abf_b3b6,
    0x03b6_e20c, 0x74b1_d29a, 0xead5_4739, 0x9dd2_77af, 0x04db_2615,
    0x73dc_1683, 0xe363_0b12, 0x9464_3b84, 0x0d6d_6a3e, 0x7a6a_5aa8,
    0xe40e_cf0b, 0x9309_ff9d, 0x0a00_ae27, 0x7d07_9eb1, 0xf00f_9344,
    0x8708_a3d2, 0x1e01_f268, 0x6906_c2fe, 0xf762_575d, 0x8065_67cb,
    0x196c_3671, 0x6e6b_06e7, 0xfed4_1b76, 0x89d3_2be0, 0x10da_7a5a,
    0x67dd_4acc, 0xf9b9_df6f, 0x8ebe_eff9, 0x17b7_be43, 0x60b0_8ed5,
    0xd6d6_a3e8, 0xa1d1_937e, 0x38d8_c2c4, 0x4fdf_f252, 0xd1bb_67f1,
    0xa6bc_5767, 0x3fb5_06dd, 0x48b2_364b, 0xd80d_2bda, 0xaf0a_1b4c,
    0x3603_4af6, 0x4104_7a60, 0xdf60_efc3, 0xa867_df55, 0x316e_8eef,
    0x4669_be79, 0xcb61_b38c, 0xbc66_831a, 0x256f_d2a0, 0x5268_e236,
    0xcc0c_7795, 0xbb0b_4703, 0x2202_16b9, 0x5505_262f, 0xc5ba_3bbe,
    0xb2bd_0b28, 0x2bb4_5a92, 0x5cb3_6a04, 0xc2d7_ffa7, 0xb5d0_cf31,
    0x2cd9_9e8b, 0x5bde_ae1d, 0x9b64_c2b0, 0xec63_f226, 0x756a_a39c,
    0x026d_930a, 0x9c09_06a9, 0xeb0e_363f, 0x7207_6785, 0x0500_5713,
    0x95bf_4a82, 0xe2b8_7a14, 0x7bb1_2bae, 0x0cb6_1b38, 0x92d2_8e9b,
    0xe5d5_be0d, 0x7cdc_efb7, 0x0bdb_df21, 0x86d3_d2d4, 0xf1d4_e242,
    0x68dd_b3f8, 0x1fda_836e, 0x81be_16cd, 0xf6b9_265b, 0x6fb0_77e1,
    0x18b7_4777, 0x8808_5ae6, 0xff0f_6a70, 0x6606_3bca, 0x1101_0b5c,
    0x8f65_9eff, 0xf862_ae69, 0x616b_ffd3, 0x166c_cf45, 0xa00a_e278,
    0xd70d_d2ee, 0x4e04_8354, 0x3903_b3c2, 0xa767_2661, 0xd060_16f7,
    0x4969_474d, 0x3e6e_77db, 0xaed1_6a4a, 0xd9d6_5adc, 0x40df_0b66,
    0x37d8_3bf0, 0xa9bc_ae53, 0xdebb_9ec5, 0x47b2_cf7f, 0x30b5_ffe9,
    0xbdbd_f21c, 0xcaba_c28a, 0x53b3_9330, 0x24b4_a3a6, 0xbad0_3605,
    0xcdd7_0693, 0x54de_5729, 0x23d9_67bf, 0xb366_7a2e, 0xc461_4ab8,
    0x5d68_1b02, 0x2a6f_2b94, 0xb40b_be37, 0xc30c_8ea1, 0x5a05_df1b,
    0x2d02_ef8d,
];

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
            && first != last
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

// Rasterize caller-authored ink at physical pixel centers. The PDF writer
// uses round caps for these paths, including horizontal and vertical lines.
// The bounded segment box avoids scanning the entire page per stroke.
fn paint_segment_ink(
    pixels: &mut [u8],
    width: usize,
    height: usize,
    dpi: u32,
    (start, end): (PdfPoint, PdfPoint),
    stroke_width_um: u32,
    color: PdfRgb,
) -> Result<(), String> {
    if stroke_width_um == 0 || start == end {
        return Err(
            "authored ink needs a nonempty positive-width segment".into()
        );
    }
    let pitch = 25_400.0f64 / f64::from(dpi);
    let x1 = f64::from(start.x_um);
    let y1 = f64::from(start.y_um);
    let x2 = f64::from(end.x_um);
    let y2 = f64::from(end.y_um);
    let dx = x2 - x1;
    let dy = y2 - y1;
    let length_squared = dx.mul_add(dx, dy * dy);
    let threshold = f64::from(stroke_width_um) / 2.0f64 + pitch / 2.0f64;
    let padding = i64::from(stroke_width_um.div_ceil(2))
        .saturating_add(i64::from(25_400u32.div_ceil(dpi.saturating_mul(2))));
    let left = pixel_coordinate(
        i64::from(start.x_um.min(end.x_um)).saturating_sub(padding), dpi, false,
    )?;
    let top = pixel_coordinate(
        i64::from(start.y_um.min(end.y_um)).saturating_sub(padding), dpi, false,
    )?;
    let right = pixel_coordinate(
        i64::from(start.x_um.max(end.x_um)).saturating_add(padding), dpi, true,
    )?;
    let bottom = pixel_coordinate(
        i64::from(start.y_um.max(end.y_um)).saturating_add(padding), dpi, true,
    )?;
    for row in top.min(height)..bottom.min(height) {
        let y_index = u32::try_from(row)
            .map_err(|error| format!("ink row index refused: {error}"))?;
        let py = (f64::from(y_index) + 0.5f64) * pitch;
        for column in left.min(width)..right.min(width) {
            let x_index = u32::try_from(column)
                .map_err(|error| format!("ink column index refused: {error}"))?;
            let px = (f64::from(x_index) + 0.5f64) * pitch;
            let fraction = ((px - x1).mul_add(dx, (py - y1) * dy)
                / length_squared).clamp(0.0f64, 1.0f64);
            let nearest_x = fraction.mul_add(dx, x1);
            let nearest_y = fraction.mul_add(dy, y1);
            if (px - nearest_x).hypot(py - nearest_y) <= threshold {
                let offset = row.checked_mul(width)
                    .and_then(|index| index.checked_add(column))
                    .and_then(|index| index.checked_mul(4))
                    .ok_or("diagonal ink raster offset overflow")?;
                let pixel = pixels.get_mut(offset..offset.saturating_add(4))
                    .ok_or("diagonal ink raster exceeds pixel budget")?;
                pixel.copy_from_slice(&[
                    color.red, color.green, color.blue, 255,
                ]);
            }
        }
    }
    Ok(())
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

// Refuse paths extending beyond the physical page before allocating a raster.
// The vector PDF applies the same half-stroke clearance to every path point;
// PNG must not silently clip invalid paper, printed, or authored ink geometry.
fn validate_page_path_bounds(
    path: &PdfVectorPath,
    width_um: u32,
    height_um: u32,
) -> Result<(), String> {
    let clearance = match path.paint {
        PdfPathPaint::Stroke { width_um: pen_width_um, .. }
            if pen_width_um > 0 =>
        {
            pen_width_um.div_ceil(2)
        }
        PdfPathPaint::Stroke { .. } => {
            return Err("PNG stroke width must be positive".into());
        }
        PdfPathPaint::Fill { .. } => 0,
    };
    let right = width_um.checked_sub(clearance);
    let bottom = height_um.checked_sub(clearance);
    let inside = |point: PdfPoint| -> bool {
        u32::try_from(point.x_um).is_ok_and(|x| {
            right.is_some_and(|edge| x >= clearance && x <= edge)
        }) && u32::try_from(point.y_um).is_ok_and(|y| {
            bottom.is_some_and(|edge| y >= clearance && y <= edge)
        })
    };
    for command in &path.commands {
        let valid = match *command {
            PdfPathCommand::MoveTo(point) | PdfPathCommand::LineTo(point) => {
                inside(point)
            }
            PdfPathCommand::CubicTo(first, second, third) => {
                inside(first) && inside(second) && inside(third)
            }
            PdfPathCommand::Close => true,
        };
        if !valid {
            return Err("PNG path extends outside the physical sheet".into());
        }
    }
    Ok(())
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
    // Between the top and bottom stroke bands, a rounded rectangle can
    // paint only its two side/corner bands. Skip its empty horizontal
    // center before running the comparatively expensive signed distance.
    let top_band_end = f64::from(border.top_um) + threshold;
    let bottom_band_start = f64::from(border.bottom_um) - threshold;
    let left_corner_end = f64::from(border.left_um) + radius + threshold;
    let right_corner_start = f64::from(border.right_um) - radius - threshold;
    // Only the stroke's physical outline can paint. Keep a conservative
    // half-pixel bound, then retain repeated-addition pixel centers inside
    // that window so the previous raster's exact pigment stays unchanged.
    let padding = i64::from(NOTEBOOK_BORDER_STROKE_UM.div_ceil(2))
        .saturating_add(i64::from(25_400u32.div_ceil(dpi.saturating_mul(2))));
    let top_row = pixel_coordinate(
        i64::from(border.top_um).saturating_sub(padding), dpi, false,
    )?.min(height);
    let bottom_row = pixel_coordinate(
        i64::from(border.bottom_um).saturating_add(padding), dpi, true,
    )?.min(height);
    let left_column = pixel_coordinate(
        i64::from(border.left_um).saturating_sub(padding), dpi, false,
    )?.min(width);
    let right_column = pixel_coordinate(
        i64::from(border.right_um).saturating_add(padding), dpi, true,
    )?.min(width);
    let mut y = pitch / 2.0f64;
    for _ in 0..top_row {
        y += pitch;
    }
    let mut first_x = pitch / 2.0f64;
    for _ in 0..left_column {
        first_x += pitch;
    }
    // In the vertical sides of a rounded rectangle, the center strip
    // cannot paint. Find its exact pixel-center interval just once and
    // jump over it for each interior row, preserving the old accumulated
    // X coordinate at the first pixel after the transparent strip.
    let mut skip_start = right_column;
    let mut skip_end = right_column;
    let mut skip_end_x = first_x;
    let mut scan_x = first_x;
    for column in left_column..right_column {
        if scan_x > left_corner_end && scan_x < right_corner_start {
            if skip_start == right_column {
                skip_start = column;
            }
            skip_end = column.checked_add(1)
                .ok_or("border raster skip overflow")?;
            skip_end_x = scan_x + pitch;
        }
        scan_x += pitch;
    }
    for row in top_row..bottom_row {
        if (y - center_y).abs() > half_height + threshold {
            y += pitch;
            continue;
        }
        let between_horizontal_bands =
            y > top_band_end && y < bottom_band_start;
        let mut x = first_x;
        let mut column = left_column;
        while column < right_column {
            if between_horizontal_bands && column == skip_start {
                column = skip_end;
                x = skip_end_x;
                continue;
            }
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
            column = column.checked_add(1)
                .ok_or("border raster column overflow")?;
            x += pitch;
        }
        y += pitch;
    }
    Ok(())
}

fn crc32_part(mut crc: u32, data: &[u8]) -> Result<u32, String> {
    for byte in data {
        let index = usize::from(crc.to_le_bytes()[0] ^ *byte);
        let part = CRC32_TABLE.get(index)
            .ok_or("PNG CRC lookup outside the byte table")?;
        crc = (crc >> 8u32) ^ part;
    }
    Ok(crc)
}

fn update_zlib_checksums(
    first: &mut u32,
    second: &mut u32,
    crc: &mut u32,
    data: &[u8],
) -> Result<(), String> {
    // A 2,000-byte chunk keeps even the worst-case Adler sums below u32::MAX.
    // Compute the IDAT CRC over the same bytes in this single raster pass.
    for chunk in data.chunks(2_000) {
        for byte in chunk {
            *first = first.wrapping_add(u32::from(*byte));
            *second = second.wrapping_add(*first);
            let index = usize::from(crc.to_le_bytes()[0] ^ *byte);
            let part = CRC32_TABLE.get(index)
                .ok_or("PNG CRC lookup outside the byte table")?;
            *crc = (*crc >> 8u32) ^ part;
        }
        *first %= 65_521;
        *second %= 65_521;
    }
    Ok(())
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
    let crc = !crc32_part(crc32_part(!0u32, &tag)?, data)?;
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
    let rows = usize::try_from(height)
        .map_err(|error| format!("PNG height refused: {error}"))?;
    let expected_pixels =
        stride.checked_mul(rows).ok_or("PNG raster size overflow")?;
    if pixels.len() != expected_pixels {
        return Err("PNG raster does not match physical dimensions".into());
    }
    let raw_len = stride
        .checked_add(1)
        .and_then(|size| size.checked_mul(rows))
        .ok_or("PNG raster size overflow")?;
    let blocks = raw_len.div_ceil(usize::from(u16::MAX));
    let compression_bytes = raw_len
        .checked_add(blocks.saturating_mul(5))
        .and_then(|size| size.checked_add(6))
        .ok_or("PNG compression size overflow")?;
    if compression_bytes.saturating_add(100) > MAX_OUTPUT_BYTES {
        return Err("PNG exceeds output byte limit".into());
    }
    // Build directly into the final PNG. The caller's RGBA canvas is the
    // only full-size input buffer; IDAT no longer needs a second zlib copy.
    let mut output = Vec::new();
    output
        .try_reserve_exact(compression_bytes.saturating_add(100))
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

    // IDAT length and type precede the stored-DEFLATE stream. Write its CRC
    // after the stream so the already-serialized bytes need not be copied.
    let idat_len = u32::try_from(compression_bytes)
        .map_err(|error| format!("PNG IDAT length refused: {error}"))?;
    output.extend_from_slice(&idat_len.to_be_bytes());
    output.extend_from_slice(b"IDAT");
    let idat_start = output.len();
    let mut crc = crc32_part(!0u32, b"IDAT")?;
    let zlib_header = [0x78, 0x01]; // zlib, stored DEFLATE.
    output.extend_from_slice(&zlib_header);
    crc = crc32_part(crc, &zlib_header)?;
    let mut filtered = pixels.chunks_exact(stride).flat_map(|scanline| {
        iter::once(0u8).chain(scanline.iter().copied())
    });
    let mut first = 1u32;
    let mut second = 0u32;
    for index in 0..blocks {
        let length = raw_len
            .checked_sub(index.saturating_mul(usize::from(u16::MAX)))
            .ok_or("PNG block offset overflow")?
            .min(usize::from(u16::MAX));
        let len = u16::try_from(length)
            .map_err(|error| format!("PNG block refused: {error}"))?;
        let block_header = [
            u8::from(index.saturating_add(1) == blocks),
            len.to_le_bytes()[0], len.to_le_bytes()[1],
            (!len).to_le_bytes()[0], (!len).to_le_bytes()[1],
        ];
        output.extend_from_slice(&block_header);
        crc = crc32_part(crc, &block_header)?;
        let start = output.len();
        output.extend(filtered.by_ref().take(length));
        let end = start
            .checked_add(length)
            .ok_or("PNG block length overflow")?;
        let content = output
            .get(start..end)
            .ok_or("PNG filtered raster ended before block boundary")?;
        update_zlib_checksums(
            &mut first, &mut second, &mut crc, content,
        )?;
    }
    let adler = ((second << 16) | first).to_be_bytes();
    output.extend_from_slice(&adler);
    crc = crc32_part(crc, &adler)?;
    let idat_end = idat_start
        .checked_add(compression_bytes)
        .ok_or("PNG IDAT boundary overflow")?;
    if output.len() != idat_end {
        return Err("PNG IDAT differs from declared compressed size".into());
    }
    output.extend_from_slice(&(!crc).to_be_bytes());
    png_chunk(&mut output, *b"IEND", &[])?;
    Ok(output)
}

/// Rasterize one paper-only page from shared Rust paper geometry.
///
/// # Errors
///
/// Refuses invalid DPI, oversized raster/output or unsupported path shapes.
pub(super) fn render_paper_png(
    width_um: u32,
    height_um: u32,
    dpi: u32,
    paths: &[PdfVectorPath],
    ink_start: usize,
    border: Option<RoundedPaperBorder>,
) -> Result<Vec<u8>, String> {
    if ink_start > paths.len() {
        return Err("PNG ink path boundary is outside page paths".into());
    }
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
    for path in paths {
        validate_page_path_bounds(path, width_um, height_um)?;
    }
    let bytes = pixels.checked_mul(4).ok_or("PNG raster byte overflow")?;
    let mut raster = Vec::new();
    raster
        .try_reserve_exact(bytes)
        .map_err(|error| format!("PNG raster allocation refused: {error}"))?;
    raster.resize(bytes, 0);
    for (index, path) in paths.iter().enumerate() {
        // PDF paints the rounded border before authored ink. Without ink,
        // keep the historical paper-only paint order and byte output.
        if index == ink_start && let Some(outline) = border {
            paint_rounded_border(
                &mut raster, width, height, dpi, outline,
            )?;
        }
        // Retain exact legacy rectangle rasterization for paper marks and
        // printed annotations. Only explicitly appended pen paths use the
        // PDF's round-capped segment semantics, independent of angle.
        if index >= ink_start {
            if let (
                [PdfPathCommand::MoveTo(start), PdfPathCommand::LineTo(end)],
                PdfPathPaint::Stroke {
                    color, width_um: pen_width_um,
                },
            ) = (path.commands.as_slice(), path.paint) {
                paint_segment_ink(
                    &mut raster, width, height, dpi, (*start, *end),
                    pen_width_um, color,
                )?;
                continue;
            }
            return Err("PNG authored ink requires a single pen segment".into());
        }
        if let Ok((rectangle, color)) = rectangle_for_path(path) {
            paint_rectangle(&mut raster, width, height, dpi, rectangle, color)?;
        } else if let (
            [PdfPathCommand::MoveTo(start), PdfPathCommand::LineTo(end)],
            PdfPathPaint::Stroke { color, width_um: line_width_um },
        ) = (path.commands.as_slice(), path.paint) {
            if start.x_um == end.x_um || start.y_um == end.y_um {
                return Err("invalid zero-length paper stroke".into());
            }
            paint_segment_ink(
                &mut raster, width, height, dpi, (*start, *end),
                line_width_um, color,
            )?;
        } else {
            return Err(
                "PNG path requires a paper rectangle or one ink segment".into()
            );
        }
    }
    if ink_start == paths.len() && let Some(outline) = border {
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
