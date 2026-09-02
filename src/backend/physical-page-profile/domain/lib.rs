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
//   - Exact physical paper-profile values and geometry validation.
// - Must-Not:
//   - Choose pixels, render colors, storage syntax, or ruler imperfections.
// - Allows:
//   - Inputs: Physical dimensions, paper family, margins, and border intent.
//   - Outputs: Validated page profile and exact writable physical region.
//   - Side effects: None.
// - Split-When:
//   - Calibrated paper-mark geometry needs independent math authority.
// - Merge-When:
//   - Physical page geometry stops being shared across output modes.
// - Summary:
//   - Defines validated real-unit page profiles independent from rendering.
// - Description:
//   - Keeps paper authority in canonical micrometres and checked geometry.
// - Usage:
//   - Validate a complete profile before layout or output compilation.
// - Defaults:
//   - No implicit clipping or repair is performed for invalid dimensions.
//

//! Exact physical page-profile values for semantic layout and output planning.

/// Which physical edge carries notebook binding or margin semantics.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum BindingEdge {
    /// Binding is on the bottom sheet edge.
    Bottom,
    /// Binding is on the left sheet edge.
    Left,
    /// Binding is on the right sheet edge.
    Right,
    /// Binding is on the top sheet edge.
    Top,
}

/// Border geometry requested by a physical page profile.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum BorderShape {
    /// No border geometry is requested.
    None,
    /// Rectangular border with square corners.
    Rectangle,
    /// Rectangular border with an explicit physical corner radius.
    RoundedRectangle,
}

/// Non-negative canonical physical length in micrometres.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct Length(u64);

impl Length {
    /// Zero physical length.
    pub const ZERO: Self = Self(0);

    /// Construct a physical length from canonical micrometres.
    #[must_use]
    pub const fn from_micrometres(micrometres: u64) -> Self {
        Self(micrometres)
    }

    /// Return this length in canonical micrometres.
    #[must_use]
    pub const fn micrometres(self) -> u64 {
        self.0
    }
}

/// Physical orientation applied to nominal sheet dimensions.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum Orientation {
    /// Swap nominal width and height for the physical sheet.
    Landscape,
    /// Use nominal width and height as declared.
    Portrait,
}

/// Nominal page-mark family before calibrated drawing geometry is compiled.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PaperPattern {
    /// No repeated page marks.
    Blank,
    /// Custom digital paper marks whose detailed geometry belongs elsewhere.
    Custom,
    /// Nominally square dot lattice.
    Dotted {
        /// Nominal dot spacing in both physical axes.
        spacing: Length,
    },
    /// Nominal horizontal rules.
    Ruled {
        /// Nominal vertical distance between rules.
        spacing: Length,
    },
    /// Nominally square ruled grid.
    Squared {
        /// Nominal cell width and height.
        spacing: Length,
    },
}

/// Whether page marks are composited below or above simulated ink.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum PaperMarkLayer {
    /// Page marks render above simulated ink when the output mode admits it.
    AboveInk,
    /// Page marks render below simulated ink.
    BelowInk,
}

/// Complete physical page profile consumed by layout and output capabilities.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PageProfile {
    /// Physical edge carrying binding-side margin semantics.
    pub binding_edge: BindingEdge,
    /// Requested border geometry.
    pub border_shape: BorderShape,
    /// Physical corner radius used only by rounded borders.
    pub corner_roundness: Length,
    /// Physical orientation applied to nominal sheet dimensions.
    pub orientation: Orientation,
    /// Distance from binding edge to the notebook margin reference.
    pub outer_margin: Length,
    /// Page-mark compositing relationship to simulated ink.
    pub paper_mark_layer: PaperMarkLayer,
    /// Nominal page-mark family.
    pub paper_pattern: PaperPattern,
    /// Physical printable area in oriented sheet coordinates.
    pub printable_region: Rect,
    /// Nominal physical sheet dimensions before orientation is applied.
    pub sheet: SheetSize,
    /// Reserved physical clearance from the printable top edge.
    pub top_clearance: Length,
    /// Additional writing inset after the binding-side margin reference.
    pub writing_inset: Length,
}

/// Typed physical page-profile validation failure.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PageProfileError {
    /// Binding-side margin plus writing inset leaves no writable width or
    /// height.
    BindingInsetExhaustsPrintableRegion,
    /// Corner radius cannot fit within half of the printable rectangle.
    CornerRoundnessExceedsPrintableRegion,
    /// Nonzero corner radius was supplied for a non-rounded border.
    CornerRoundnessRequiresRoundedBorder,
    /// A repeated paper pattern has zero physical spacing.
    PatternSpacingIsZero,
    /// Printable rectangle is empty.
    PrintableRegionIsEmpty,
    /// Printable rectangle extends outside the oriented physical sheet.
    PrintableRegionOutsideSheet,
    /// Nominal sheet width or height is zero.
    SheetDimensionIsZero,
    /// Top clearance leaves no writable height.
    TopClearanceExhaustsPrintableRegion,
}

/// Exact physical rectangle in oriented sheet coordinates.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Rect {
    /// Rectangle height.
    pub height: Length,
    /// Rectangle width.
    pub width: Length,
    /// Horizontal offset from the oriented sheet's left edge.
    pub x: Length,
    /// Vertical offset from the oriented sheet's top edge.
    pub y: Length,
}

/// Nominal physical sheet dimensions before orientation is applied.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SheetSize {
    /// Nominal sheet height.
    pub height: Length,
    /// Nominal sheet width.
    pub width: Length,
}

impl PageProfile {
    /// Return oriented physical sheet dimensions.
    ///
    /// # Errors
    ///
    /// Returns [`PageProfileError::SheetDimensionIsZero`] when either nominal
    /// sheet dimension is zero.
    pub fn oriented_sheet(self) -> Result<SheetSize, PageProfileError> {
        if self.sheet.width == Length::ZERO || self.sheet.height == Length::ZERO
        {
            return Err(PageProfileError::SheetDimensionIsZero);
        }
        Ok(match self.orientation {
            Orientation::Landscape => SheetSize {
                height: self.sheet.width,
                width: self.sheet.height,
            },
            Orientation::Portrait => self.sheet,
        })
    }

    /// Validate this complete profile without silently repairing geometry.
    ///
    /// # Errors
    ///
    /// Returns a typed failure when physical geometry is empty, out of bounds,
    /// or leaves no writable region.
    pub fn validate(self) -> Result<Self, PageProfileError> {
        let oriented = self.oriented_sheet()?;
        validate_rect_inside(self.printable_region, oriented)?;
        validate_pattern(self.paper_pattern)?;
        validate_corner_roundness(
            self.border_shape,
            self.corner_roundness,
            self.printable_region,
        )?;
        let _writable = self.writable_region()?;
        Ok(self)
    }

    /// Derive the exact writable region from the validated printable area.
    ///
    /// # Errors
    ///
    /// Returns a typed failure if top clearance or binding-side inset consumes
    /// the printable region, or if the printable region itself is invalid.
    pub fn writable_region(self) -> Result<Rect, PageProfileError> {
        let oriented = self.oriented_sheet()?;
        validate_rect_inside(self.printable_region, oriented)?;
        let top = checked_sum(
            self.printable_region.y,
            self.top_clearance,
            PageProfileError::TopClearanceExhaustsPrintableRegion,
        )?;
        let height = checked_difference(
            self.printable_region.height,
            self.top_clearance,
            PageProfileError::TopClearanceExhaustsPrintableRegion,
        )?;
        if height == Length::ZERO {
            return Err(PageProfileError::TopClearanceExhaustsPrintableRegion);
        }
        let binding_inset = checked_sum(
            self.outer_margin,
            self.writing_inset,
            PageProfileError::BindingInsetExhaustsPrintableRegion,
        )?;
        match self.binding_edge {
            BindingEdge::Left => {
                let x = checked_sum(
                    self.printable_region.x,
                    binding_inset,
                    PageProfileError::BindingInsetExhaustsPrintableRegion,
                )?;
                let width = checked_difference(
                    self.printable_region.width,
                    binding_inset,
                    PageProfileError::BindingInsetExhaustsPrintableRegion,
                )?;
                nonempty_binding_rect(x, top, width, height)
            },
            BindingEdge::Right => {
                let width = checked_difference(
                    self.printable_region.width,
                    binding_inset,
                    PageProfileError::BindingInsetExhaustsPrintableRegion,
                )?;
                nonempty_binding_rect(
                    self.printable_region.x,
                    top,
                    width,
                    height,
                )
            },
            BindingEdge::Top => {
                let additional_top = checked_sum(
                    top,
                    binding_inset,
                    PageProfileError::BindingInsetExhaustsPrintableRegion,
                )?;
                let bound_height = checked_difference(
                    height,
                    binding_inset,
                    PageProfileError::BindingInsetExhaustsPrintableRegion,
                )?;
                nonempty_binding_rect(
                    self.printable_region.x,
                    additional_top,
                    self.printable_region.width,
                    bound_height,
                )
            },
            BindingEdge::Bottom => {
                let bound_height = checked_difference(
                    height,
                    binding_inset,
                    PageProfileError::BindingInsetExhaustsPrintableRegion,
                )?;
                nonempty_binding_rect(
                    self.printable_region.x,
                    top,
                    self.printable_region.width,
                    bound_height,
                )
            },
        }
    }
}

fn checked_difference(
    minuend: Length,
    subtrahend: Length,
    error: PageProfileError,
) -> Result<Length, PageProfileError> {
    minuend
        .micrometres()
        .checked_sub(subtrahend.micrometres())
        .map(Length::from_micrometres)
        .ok_or(error)
}

fn checked_sum(
    left: Length,
    right: Length,
    error: PageProfileError,
) -> Result<Length, PageProfileError> {
    left.micrometres()
        .checked_add(right.micrometres())
        .map(Length::from_micrometres)
        .ok_or(error)
}

fn nonempty_binding_rect(
    x: Length,
    y: Length,
    width: Length,
    height: Length,
) -> Result<Rect, PageProfileError> {
    if width == Length::ZERO || height == Length::ZERO {
        return Err(PageProfileError::BindingInsetExhaustsPrintableRegion);
    }
    Ok(Rect { height, width, x, y })
}

fn validate_corner_roundness(
    border: BorderShape,
    radius: Length,
    printable: Rect,
) -> Result<(), PageProfileError> {
    if border != BorderShape::RoundedRectangle {
        if radius != Length::ZERO {
            return Err(PageProfileError::CornerRoundnessRequiresRoundedBorder);
        }
        return Ok(());
    }
    let doubled = radius
        .micrometres()
        .checked_mul(2)
        .ok_or(PageProfileError::CornerRoundnessExceedsPrintableRegion)?;
    if doubled > printable.width.micrometres()
        || doubled > printable.height.micrometres()
    {
        return Err(PageProfileError::CornerRoundnessExceedsPrintableRegion);
    }
    Ok(())
}

fn validate_pattern(pattern: PaperPattern) -> Result<(), PageProfileError> {
    let spacing = match pattern {
        PaperPattern::Blank | PaperPattern::Custom => return Ok(()),
        PaperPattern::Dotted { spacing }
        | PaperPattern::Ruled { spacing }
        | PaperPattern::Squared { spacing } => spacing,
    };
    if spacing == Length::ZERO {
        return Err(PageProfileError::PatternSpacingIsZero);
    }
    Ok(())
}

fn validate_rect_inside(
    rectangle: Rect,
    sheet: SheetSize,
) -> Result<(), PageProfileError> {
    if rectangle.width == Length::ZERO || rectangle.height == Length::ZERO {
        return Err(PageProfileError::PrintableRegionIsEmpty);
    }
    let right = checked_sum(
        rectangle.x,
        rectangle.width,
        PageProfileError::PrintableRegionOutsideSheet,
    )?;
    let bottom = checked_sum(
        rectangle.y,
        rectangle.height,
        PageProfileError::PrintableRegionOutsideSheet,
    )?;
    if right > sheet.width || bottom > sheet.height {
        return Err(PageProfileError::PrintableRegionOutsideSheet);
    }
    Ok(())
}
