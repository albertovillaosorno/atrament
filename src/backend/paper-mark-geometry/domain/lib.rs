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
//   - Exact nominal paper-mark series and bounded ruler-deviation validation.
// - Must-Not:
//   - Invent random noise, move nominal anchors, choose color, or render
//     pixels.
// - Allows:
//   - Inputs: Explicit physical mark region, paper pattern, and appearance.
//   - Outputs: Compact exact line/dot series and validated ruler samples.
//   - Side effects: None.
// - Split-When:
//   - Seeded ruler-path synthesis becomes an independently calibrated model.
// - Merge-When:
//   - Paper marks stop requiring geometry independent from page profiles.
// - Summary:
//   - Preserves exact grid spacing while admitting bounded visual imperfection.
// - Description:
//   - Keeps nominal anchors authoritative and ruler error presentation-only.
// - Usage:
//   - Compile an explicit mark region, then validate calibrated stroke samples.
// - Defaults:
//   - The supplied region origin is the first admitted nominal mark anchor.
//

//! Exact nominal paper-mark geometry with bounded ruler-style deviations.

use atrament_physical_page_profile::{
    Length, PageProfile, PageProfileError, PaperMarkAppearance, PaperMarkLayer,
    PaperPattern, Rect,
};

/// One compact arithmetic series of exact physical mark coordinates.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AxisSeries {
    /// Number of admitted nominal coordinates in this series.
    pub count: u64,
    /// First nominal coordinate in oriented sheet coordinates.
    pub first: Length,
    /// Exact nominal spacing between adjacent coordinates.
    pub spacing: Length,
}

impl AxisSeries {
    /// Return one exact nominal coordinate without allocating the full series.
    #[must_use]
    pub fn coordinate(self, index: u64) -> Option<Length> {
        if index >= self.count {
            return None;
        }
        let offset = self.spacing.micrometres().checked_mul(index)?;
        self.first
            .micrometres()
            .checked_add(offset)
            .map(Length::from_micrometres)
    }
}

/// Typed paper-mark geometry compilation failure.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GeometryError {
    /// Custom paper requires explicit geometry owned by its profile.
    CustomGeometryRequired,
    /// Mark region has zero physical width or height.
    EmptyRegion,
    /// Mark-region coordinate arithmetic overflowed canonical physical units.
    RegionOverflow,
    /// A repeated paper pattern has zero nominal spacing.
    SpacingIsZero,
}

/// Compact exact geometry for one admitted paper-mark family.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PaperMarkGeometry {
    /// No repeated physical page marks.
    Blank,
    /// Nominal dot intersections from exact horizontal and vertical series.
    Dotted {
        /// Exact horizontal dot-row coordinates.
        horizontal: AxisSeries,
        /// Exact vertical dot-column coordinates.
        vertical: AxisSeries,
    },
    /// Nominal horizontal rules.
    Ruled {
        /// Exact horizontal rule coordinates.
        horizontal: AxisSeries,
        /// Exact horizontal physical span for every rule.
        span: Span,
    },
    /// Nominal square grid from two series with identical spacing.
    Squared {
        /// Exact horizontal grid-line coordinates.
        horizontal: AxisSeries,
        /// Exact horizontal physical span for horizontal grid lines.
        horizontal_span: Span,
        /// Exact vertical grid-line coordinates.
        vertical: AxisSeries,
        /// Exact vertical physical span for vertical grid lines.
        vertical_span: Span,
    },
}

/// One complete profile-owned nominal paper-mark plan.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ProfilePaperMarks {
    /// Profile-owned ruler deviation and intersection treatment.
    pub appearance: PaperMarkAppearance,
    /// Exact nominal physical mark geometry.
    pub geometry: PaperMarkGeometry,
    /// Profile-owned compositing relationship to simulated ink.
    pub layer: PaperMarkLayer,
}

/// Typed failure to compile marks from one complete physical page profile.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProfilePaperMarksError {
    /// Nominal mark geometry could not be compiled for the selected region.
    Geometry(GeometryError),
    /// The complete physical page profile is invalid.
    InvalidProfile(PageProfileError),
    /// Explicit mark region extends outside the oriented physical sheet.
    RegionOutsideSheet,
}

/// One signed presentation displacement in canonical micrometres.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct RulerOffset(i64);

impl RulerOffset {
    /// Construct a signed presentation offset in canonical micrometres.
    #[must_use]
    pub const fn from_micrometres(micrometres: i64) -> Self {
        Self(micrometres)
    }

    /// Return the signed offset in canonical micrometres.
    #[must_use]
    pub const fn micrometres(self) -> i64 {
        self.0
    }
}

/// One calibrated ruler-path sample relative to an unchanged nominal line.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RulerSample {
    /// Distance from the nominal line's span start.
    pub along: Length,
    /// Signed normal presentation displacement from the nominal line.
    pub normal_offset: RulerOffset,
}

/// Exact one-dimensional physical span.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Span {
    /// Inclusive physical end coordinate.
    pub end: Length,
    /// Inclusive physical start coordinate.
    pub start: Length,
}

/// Typed rejection of one calibrated ruler sample.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RulerSampleError {
    /// Absolute normal displacement exceeds the profile-owned bound.
    ErrorBoundExceeded,
    /// Supplied ruler appearance is not a valid physical-profile appearance.
    InvalidAppearance(PageProfileError),
    /// Signed offset magnitude cannot be represented as a physical length.
    OffsetMagnitudeOverflow,
    /// Sample distance lies beyond the nominal line span.
    OutsideSpan,
}

/// Compile exact nominal marks for an explicitly selected physical mark region.
///
/// The region origin is the first nominal anchor. Ruler error is deliberately
/// absent from this function so calibrated appearance cannot change cell size.
///
/// # Errors
///
/// Returns a typed failure for empty/overflowing regions, zero repeated
/// spacing, or custom paper whose detailed geometry has not been supplied.
pub fn compile_nominal_marks(
    region: Rect,
    pattern: PaperPattern,
) -> Result<PaperMarkGeometry, GeometryError> {
    validate_region(region)?;
    match pattern {
        PaperPattern::Blank => Ok(PaperMarkGeometry::Blank),
        PaperPattern::Custom => Err(GeometryError::CustomGeometryRequired),
        PaperPattern::Dotted { spacing } => {
            validate_spacing(spacing)?;
            Ok(PaperMarkGeometry::Dotted {
                horizontal: series(region.y, region.height, spacing)?,
                vertical: series(region.x, region.width, spacing)?,
            })
        },
        PaperPattern::Ruled { spacing } => {
            validate_spacing(spacing)?;
            Ok(PaperMarkGeometry::Ruled {
                horizontal: series(region.y, region.height, spacing)?,
                span: horizontal_span(region)?,
            })
        },
        PaperPattern::Squared { spacing } => {
            validate_spacing(spacing)?;
            Ok(PaperMarkGeometry::Squared {
                horizontal: series(region.y, region.height, spacing)?,
                horizontal_span: horizontal_span(region)?,
                vertical: series(region.x, region.width, spacing)?,
                vertical_span: vertical_span(region)?,
            })
        },
    }
}

/// Compile one explicit physical mark region using profile-owned paper state.
///
/// The caller still selects the physical mark region; this function ensures the
/// pattern, ruler appearance, and layer all come from one validated profile.
///
/// # Errors
///
/// Returns a typed profile or nominal geometry failure without partial output.
pub fn compile_profile_marks(
    region: Rect,
    profile: PageProfile,
) -> Result<ProfilePaperMarks, ProfilePaperMarksError> {
    let valid = profile
        .validate()
        .map_err(ProfilePaperMarksError::InvalidProfile)?;
    validate_region(region).map_err(ProfilePaperMarksError::Geometry)?;
    let sheet = valid
        .oriented_sheet()
        .map_err(ProfilePaperMarksError::InvalidProfile)?;
    let right = horizontal_span(region)
        .map_err(ProfilePaperMarksError::Geometry)?
        .end;
    let bottom = vertical_span(region)
        .map_err(ProfilePaperMarksError::Geometry)?
        .end;
    if right > sheet.width || bottom > sheet.height {
        return Err(ProfilePaperMarksError::RegionOutsideSheet);
    }
    let geometry = compile_nominal_marks(region, valid.paper_pattern)
        .map_err(ProfilePaperMarksError::Geometry)?;
    Ok(ProfilePaperMarks {
        appearance: valid.paper_mark_appearance,
        geometry,
        layer: valid.paper_mark_layer,
    })
}

/// Validate one ruler-style sample against explicit appearance bounds.
///
/// This validation never mutates the nominal line or grid series.
///
/// # Errors
///
/// Returns a typed failure when a sample lies beyond the line span, exceeds the
/// admitted normal error bound, or has an unrepresentable signed magnitude.
pub fn validate_ruler_sample(
    sample: RulerSample,
    line_length: Length,
    appearance: PaperMarkAppearance,
) -> Result<RulerSample, RulerSampleError> {
    let valid_appearance = appearance
        .validate()
        .map_err(RulerSampleError::InvalidAppearance)?;
    if sample.along > line_length {
        return Err(RulerSampleError::OutsideSpan);
    }
    let signed_magnitude = sample
        .normal_offset
        .micrometres()
        .checked_abs()
        .ok_or(RulerSampleError::OffsetMagnitudeOverflow)?;
    let Ok(magnitude) = u64::try_from(signed_magnitude) else {
        return Err(RulerSampleError::OffsetMagnitudeOverflow);
    };
    if magnitude > valid_appearance.maximum_ruler_error.micrometres() {
        return Err(RulerSampleError::ErrorBoundExceeded);
    }
    Ok(sample)
}

fn horizontal_span(region: Rect) -> Result<Span, GeometryError> {
    let end = region
        .x
        .micrometres()
        .checked_add(region.width.micrometres())
        .map(Length::from_micrometres)
        .ok_or(GeometryError::RegionOverflow)?;
    Ok(Span { end, start: region.x })
}

fn series(
    first: Length,
    extent: Length,
    spacing: Length,
) -> Result<AxisSeries, GeometryError> {
    let intervals = extent
        .micrometres()
        .checked_div(spacing.micrometres())
        .ok_or(GeometryError::SpacingIsZero)?;
    let count = intervals
        .checked_add(1)
        .ok_or(GeometryError::RegionOverflow)?;
    let series = AxisSeries { count, first, spacing };
    let last_index =
        count.checked_sub(1).ok_or(GeometryError::RegionOverflow)?;
    let _last = series
        .coordinate(last_index)
        .ok_or(GeometryError::RegionOverflow)?;
    Ok(series)
}

fn validate_region(region: Rect) -> Result<(), GeometryError> {
    if region.width == Length::ZERO || region.height == Length::ZERO {
        return Err(GeometryError::EmptyRegion);
    }
    let _right = region
        .x
        .micrometres()
        .checked_add(region.width.micrometres())
        .ok_or(GeometryError::RegionOverflow)?;
    let _bottom = region
        .y
        .micrometres()
        .checked_add(region.height.micrometres())
        .ok_or(GeometryError::RegionOverflow)?;
    Ok(())
}

fn validate_spacing(spacing: Length) -> Result<(), GeometryError> {
    if spacing == Length::ZERO {
        return Err(GeometryError::SpacingIsZero);
    }
    Ok(())
}

fn vertical_span(region: Rect) -> Result<Span, GeometryError> {
    let end = region
        .y
        .micrometres()
        .checked_add(region.height.micrometres())
        .map(Length::from_micrometres)
        .ok_or(GeometryError::RegionOverflow)?;
    Ok(Span { end, start: region.y })
}
