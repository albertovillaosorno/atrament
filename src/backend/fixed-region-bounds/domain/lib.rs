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
//   - Exact post-placement physical bounds comparison for one fixed rectangle.
// - Must-Not:
//   - Choose anchors, alignment, minimum size, collision policy, or
//     remediation.
// - Allows:
//   - Inputs: Explicit writable and placed physical rectangles.
//   - Outputs: Stable violated edges and exact physical overflow amounts.
//   - Side effects: Process-local result allocation only.
// - Split-When:
//   - Collision geometry or placement solving becomes independently complex.
// - Merge-When:
//   - Fixed-region validation is fully subsumed by a spatial layout domain.
// - Summary:
//   - Measures fixed-region overflow without inventing placement constraints.
// - Description:
//   - Reports every crossed writable edge in canonical physical micrometres.
// - Usage:
//   - Solve placement elsewhere, then compare its rectangle with page bounds.
// - Defaults:
//   - Zero extents are admitted here; minimum-size policy belongs to the
//     solver.
//

//! Exact post-placement physical bounds checking for fixed page regions.

use atrament_physical_page_profile::{Length, Rect};

/// Physical writable-region edge crossed by placed fixed geometry.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum BoundaryEdge {
    /// Placed geometry extends below the writable rectangle.
    Bottom,
    /// Placed geometry extends left of the writable rectangle.
    Left,
    /// Placed geometry extends right of the writable rectangle.
    Right,
    /// Placed geometry extends above the writable rectangle.
    Top,
}

/// One exact physical bounds violation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BoundaryViolation {
    /// Exact distance beyond the violated writable edge.
    pub amount: Length,
    /// Writable edge crossed by the placed geometry.
    pub edge: BoundaryEdge,
}

/// Complete post-placement bounds result for one fixed rectangle.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BoundsReport {
    /// All crossed edges in stable [`BoundaryEdge`] order.
    pub violations: Vec<BoundaryViolation>,
}

impl BoundsReport {
    /// Whether placed geometry remains completely within the writable
    /// rectangle.
    #[must_use]
    pub const fn is_within_bounds(&self) -> bool {
        self.violations.is_empty()
    }
}

/// Typed arithmetic failure before physical bounds can be compared.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BoundsError {
    /// Placed rectangle end coordinate is not representable.
    ObjectCoordinateOverflow,
    /// Writable rectangle end coordinate is not representable.
    WritableCoordinateOverflow,
}

/// Measure every writable edge crossed by one explicit placed rectangle.
///
/// This function does not repair placement or choose a remediation. A caller
/// can map the exact edge and amount into diagnostics while
/// source-authoritative movement, crop, resize, or reflow remains a separate
/// application operation.
///
/// # Errors
///
/// Returns a typed failure if either rectangle's right or bottom coordinate
/// cannot be represented in canonical physical units.
pub fn check_bounds(
    writable: Rect,
    object: Rect,
) -> Result<BoundsReport, BoundsError> {
    let (writable_right, writable_bottom) =
        rectangle_end(writable, BoundsError::WritableCoordinateOverflow)?;
    let (object_right, object_bottom) =
        rectangle_end(object, BoundsError::ObjectCoordinateOverflow)?;
    let mut violations = Vec::new();
    if object_bottom > writable_bottom {
        violations.push(BoundaryViolation {
            amount: difference(object_bottom, writable_bottom),
            edge: BoundaryEdge::Bottom,
        });
    }
    if object.x < writable.x {
        violations.push(BoundaryViolation {
            amount: difference(writable.x, object.x),
            edge: BoundaryEdge::Left,
        });
    }
    if object_right > writable_right {
        violations.push(BoundaryViolation {
            amount: difference(object_right, writable_right),
            edge: BoundaryEdge::Right,
        });
    }
    if object.y < writable.y {
        violations.push(BoundaryViolation {
            amount: difference(writable.y, object.y),
            edge: BoundaryEdge::Top,
        });
    }
    Ok(BoundsReport { violations })
}

const fn difference(greater: Length, lesser: Length) -> Length {
    Length::from_micrometres(
        greater.micrometres().saturating_sub(lesser.micrometres()),
    )
}

const fn rectangle_end(
    rectangle: Rect,
    error: BoundsError,
) -> Result<(Length, Length), BoundsError> {
    let Some(right) = rectangle
        .x
        .micrometres()
        .checked_add(rectangle.width.micrometres())
    else {
        return Err(error);
    };
    let Some(bottom) = rectangle
        .y
        .micrometres()
        .checked_add(rectangle.height.micrometres())
    else {
        return Err(error);
    };
    Ok((
        Length::from_micrometres(right),
        Length::from_micrometres(bottom),
    ))
}
