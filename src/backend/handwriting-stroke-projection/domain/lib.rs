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
//   - Exact physical-ink projection of authored glyph segments and joins.
// - Must-Not:
//   - Invent shapes, learn handwriting, round-trip codecs, rasterize, emit PDF,
//     access files, infer a writer's identity, or choose layout/page policy.
// - Allows:
//   - Inputs: Explicit local stroke geometry, caller-chosen mean dimensions,
//     shear, headings, absolute placement, and opt-in adjacent pen joins.
//   - Outputs: Deterministic physical segment coordinates or precise refusal.
//   - Side effects: None.
// - Split-When:
//   - Contour expansion, fitted pen response, or a motion solver becomes real.
// - Merge-When:
//   - Exact pen geometry ceases to be independent of export codecs.
// - Summary:
//   - Projects actual handwriting segment geometry before PDF/PNG conversion.
// - Description:
//   - Preserves source strokes and rejects overflow and nonphysical joins.
// - Usage:
//   - Called by the native notebook CLI for every placed glyph and join.
// - Defaults:
//   - Uncalibrated local geometry and no variation/joins without selection.
//

//! Renderer-independent, exact local-to-page handwriting ink synthesis.
//!
//! The caller chooses each glyph contour. This engine resolves physical pen
//! coordinates in whole micrometres and rejects unsafe joins. There is no
//! handwriting recognition, trained model, rasterizer, or external I/O here.

// Match the reference 3500x4500 micrometre authored cell and 300um pen.
const TEMPLATE_WIDTH_UM: u32 = 3_500;
const TEMPLATE_HEIGHT_UM: u32 = 4_500;
const TEMPLATE_INK_UM: u32 = 300;

/// One exact local pen segment, kept separate from page and output codecs.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AuthoredInkSegment {
    /// Local end in whole micrometres.
    pub end_um: (u32, u32),
    /// Authored RGB ink color.
    pub rgb: [u8; 3],
    /// Local start in whole micrometres.
    pub start_um: (u32, u32),
    /// Local line width in whole micrometres.
    pub width_um: u32,
}

/// Explicit statistical mean parameters; neither samples nor learned values.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MeasuredGlyphDimensions {
    /// Target glyph cell height.
    pub height_um: u32,
    /// Target ink width for the 300 micrometre reference pen.
    pub ink_thickness_um: u32,
    /// Right-positive tangent shear in millidegrees from the lower baseline.
    pub slant_millidegrees: i32,
    /// Target glyph cell width.
    pub width_um: u32,
}

/// Page and heading treatment for a single already selected glyph segment.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct InkPlacement {
    /// Heading ink replacement, or None to retain the authored color.
    pub color_override: Option<[u8; 3]>,
    /// Optional admitted mean dimensions. None retains authored coordinates.
    pub dimensions: Option<MeasuredGlyphDimensions>,
    /// Heading-only pen-width multiplier, 100 for no override.
    pub ink_scale_percent: u32,
    /// Absolute target origin; signed to reject off-page coordinates exactly.
    pub origin_um: (i64, i64),
    /// Physical page index supplied by layout.
    pub page: usize,
}

/// Result after geometry is resolved, before any PDF/PNG-specific encoding.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PhysicalInkSegment {
    /// Absolute pen end.
    pub end_um: (u32, u32),
    /// Physical page index.
    pub page: usize,
    /// Resulting RGB pen color.
    pub rgb: [u8; 3],
    /// Absolute pen start.
    pub start_um: (u32, u32),
    /// Resulting pen width.
    pub width_um: u32,
}

fn scale(
    local_um: u32, target_um: u32, reference_um: u32,
) -> Result<u32, &'static str> {
    let scaled = u64::from(local_um)
        .checked_mul(u64::from(target_um))
        .ok_or("glyph scale overflow")?
        .checked_add(u64::from(
            reference_um
                .checked_div(2)
                .ok_or("glyph scale reference is zero")?,
        ))
        .ok_or("glyph rounding overflow")?
        .checked_div(u64::from(reference_um))
        .ok_or("glyph scale denominator is zero")?;
    u32::try_from(scaled).map_err(|_error| "glyph scale too large")
}

fn coordinate(origin_um: i64, local_um: u32) -> Result<u32, &'static str> {
    let translated = origin_um
        .checked_add(i64::from(local_um))
        .ok_or("glyph placement overflow")?;
    let physical = u32::try_from(translated)
        .map_err(|_error| "glyph placement needs nonnegative coordinates")?;
    Ok(physical)
}

// Shear around the lower baseline. Never clip/wrap invalid geometry.
fn slanted_x(
    origin_um: i64,
    local_x: u32,
    local_y: u32,
    height: u32,
    slant_millidegrees: i32,
) -> Result<u32, &'static str> {
    let radians = (f64::from(slant_millidegrees) / 1_000.0).to_radians();
    let delta = (f64::from(height) - f64::from(local_y)) * radians.tan();
    if !delta.is_finite() || delta.abs() > f64::from(i32::MAX) {
        return Err("glyph slant exceeds coordinate limits");
    }
    #[expect(
        clippy::as_conversions,
        clippy::cast_possible_truncation,
        reason = "bounded millidegree shear rounded to a whole micrometre"
    )]
    let shift_um = delta.round() as i32;
    let absolute_um = origin_um
        .checked_add(i64::from(local_x))
        .and_then(|x| x.checked_add(i64::from(shift_um)))
        .ok_or("slanted glyph x position overflow")?;
    let physical = u32::try_from(absolute_um)
        .map_err(|_error| "slanted glyph must stay on page")?;
    Ok(physical)
}

fn glyph_point(
    origin: (i64, i64),
    point: (u32, u32),
    dimensions: Option<MeasuredGlyphDimensions>,
) -> Result<(u32, u32), &'static str> {
    let (x, y) = if let Some(values) = dimensions {
        (
            scale(point.0, values.width_um, TEMPLATE_WIDTH_UM)?,
            scale(point.1, values.height_um, TEMPLATE_HEIGHT_UM)?,
        )
    } else {
        point
    };
    let projected_x = if let Some(values) = dimensions {
        slanted_x(origin.0, x, y, values.height_um, values.slant_millidegrees)?
    } else {
        coordinate(origin.0, x)?
    };
    Ok((projected_x, coordinate(origin.1, y)?))
}

/// Project one authored pen segment into physical micrometres for PDF/PNG.
///
/// No source shape is ever mutated. Mean scaling, lower-baseline shear,
/// heading width and color are applied in deterministic order. The physical
/// exporter remains responsible for page-size and stroke-margin admission.
///
/// # Errors
/// Refuses overflow, negative page coordinates, singular or periodic slants,
/// oversized shears, and pen widths outside the 80..=1500 um envelope.
pub fn project_ink_segment(
    segment: AuthoredInkSegment,
    placement: InkPlacement,
) -> Result<PhysicalInkSegment, &'static str> {
    if placement.page == 0 {
        return Err("glyph page must be positive");
    }
    if placement.dimensions.is_some_and(|mean| {
        mean.width_um == 0 || mean.height_um == 0
    }) {
        return Err("glyph mean cell width and height must be positive");
    }
    // Tangent repeats every 180 degrees and is singular at +/-90 degrees.
    // Check the angle independently of contour Y: strokes entirely on the
    // lower baseline otherwise appear to accept impossible mean slants.
    if placement.dimensions.is_some_and(|mean| {
        !(-89_999i32..=89_999i32).contains(&mean.slant_millidegrees)
    }) {
        return Err("glyph slant must be between -90 and 90 degrees");
    }
    let mean_width = if let Some(values) = placement.dimensions {
        scale(segment.width_um, values.ink_thickness_um, TEMPLATE_INK_UM)?
    } else {
        segment.width_um
    };
    let width_um = scale(mean_width, placement.ink_scale_percent, 100)?;
    if !(80..=1_500).contains(&width_um) {
        return Err("scaled glyph ink width exceeds 80..1500 um");
    }
    let start_um = glyph_point(
        placement.origin_um, segment.start_um, placement.dimensions,
    )?;
    let end_um = glyph_point(
        placement.origin_um, segment.end_um, placement.dimensions,
    )?;
    if start_um == end_um {
        return Err("projected glyph segment must have distinct endpoints");
    }
    Ok(PhysicalInkSegment {
        page: placement.page,
        start_um,
        end_um,
        width_um,
        rgb: placement.color_override.unwrap_or(segment.rgb),
    })
}

/// Join two actual, already projected directed stroke endpoints.
///
/// Neither a different page, color, or pen width nor an overlong bridge can
/// be silently reconciled. Exact coincidence produces no duplicate segment.
///
/// # Errors
/// Refuses nonphysical inputs, cross-page/cross-pen joins, or excessive gaps.
pub fn connect_ink_segments(
    before: PhysicalInkSegment,
    after: PhysicalInkSegment,
    max_gap_um: u64,
) -> Result<Option<PhysicalInkSegment>, &'static str> {
    if before.page != after.page {
        return Err("text join cannot cross physical pages");
    }
    // Public physical segment values can be supplied without projection.
    // Refuse nonphysical inputs instead of laundering them into joined ink.
    if before.page == 0 {
        return Err("text join source page must be positive");
    }
    if before.rgb != after.rgb || before.width_um != after.width_um {
        return Err("text join requires matching physical pen and RGB");
    }
    if !(80..=1_500).contains(&before.width_um) {
        return Err("text join source pen width exceeds 80..1500 um");
    }
    if before.start_um == before.end_um || after.start_um == after.end_um {
        return Err("text join source segment must have distinct endpoints");
    }
    let (start, end) = (before.end_um, after.start_um);
    if start == end {
        return Ok(None);
    }
    let dx = i64::from(start.0)
        .checked_sub(i64::from(end.0))
        .ok_or("text join distance overflow")?;
    let dy = i64::from(start.1)
        .checked_sub(i64::from(end.1))
        .ok_or("text join distance overflow")?;
    // u128 and checked arithmetic avoid squared-distance wrapping.
    let squared = |value: u64| u128::from(value).checked_mul(u128::from(value));
    let distance_sq = squared(dx.unsigned_abs())
        .and_then(|x| squared(dy.unsigned_abs()).and_then(|y| x.checked_add(y)))
        .ok_or("text join squared distance overflow")?;
    let limit_sq = squared(max_gap_um)
        .ok_or("text join squared distance overflow")?;
    if distance_sq > limit_sq {
        return Err("text join gap exceeds 1200 um");
    }
    Ok(Some(PhysicalInkSegment {
        page: before.page,
        start_um: start,
        end_um: end,
        width_um: before.width_um,
        rgb: before.rgb,
    }))
}
