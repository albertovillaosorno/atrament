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
//   - Regression evidence for exact post-placement fixed-region overflow.
// - Must-Not:
//   - Freeze placement anchors, solver policy, diagnostic prose, or UI actions.
// - Allows:
//   - Inputs: Explicit physical writable and object rectangles.
//   - Outputs: Assertions over violated edge and exact physical amount.
//   - Side effects: None beyond process-local test allocation.
// - Split-When:
//   - Collision or solver fixtures gain independent acceptance contracts.
// - Merge-When:
//   - Fixed bounds checking becomes part of complete spatial-layout fixtures.
// - Summary:
//   - Verifies fixed geometry never crosses page bounds invisibly.
// - Description:
//   - Covers exact fit, each edge, corners, six-mm overflow, and overflow math.
// - Usage:
//   - Compile directly against fixed-region-bounds and page physical units.
// - Defaults:
//   - Rectangles use canonical micrometres and may have zero extent.
//
use atrament_fixed_region_bounds::{
    BoundaryEdge, BoundaryViolation, BoundsError, check_bounds,
};
use atrament_physical_page_profile::{Length, Rect};

fn rect(x: u64, y: u64, width: u64, height: u64) -> Rect {
    Rect {
        height: Length::from_micrometres(height),
        width: Length::from_micrometres(width),
        x: Length::from_micrometres(x),
        y: Length::from_micrometres(y),
    }
}

#[test]
fn exact_fit_has_no_overflow() {
    let writable = rect(10_000, 20_000, 100_000, 200_000);
    let report = check_bounds(writable, writable).expect("exact bounds");
    assert!(report.is_within_bounds());
    assert!(report.violations.is_empty());
}

#[test]
fn first_journey_six_mm_bottom_overflow_is_exact() {
    let writable = rect(10_000, 20_000, 100_000, 200_000);
    let object = rect(20_000, 180_000, 50_000, 46_000);
    let report = check_bounds(writable, object).expect("bottom overflow");
    assert_eq!(report.violations, vec![BoundaryViolation {
        amount: Length::from_micrometres(6_000),
        edge: BoundaryEdge::Bottom,
    }]);
}

#[test]
fn each_edge_reports_only_its_exact_excess() {
    let writable = rect(100, 200, 300, 400);
    let cases = [
        (rect(150, 250, 50, 351), BoundaryEdge::Bottom, 1),
        (rect(99, 250, 50, 50), BoundaryEdge::Left, 1),
        (rect(350, 250, 51, 50), BoundaryEdge::Right, 1),
        (rect(150, 199, 50, 50), BoundaryEdge::Top, 1),
    ];
    for (object, edge, amount) in cases {
        let report = check_bounds(writable, object).expect("single edge");
        assert_eq!(report.violations, vec![BoundaryViolation {
            amount: Length::from_micrometres(amount),
            edge,
        }]);
    }
}

#[test]
fn corner_crossing_reports_both_edges_in_stable_order() {
    let writable = rect(100, 200, 300, 400);
    let object = rect(90, 190, 20, 20);
    let report = check_bounds(writable, object).expect("top-left corner");
    assert_eq!(report.violations, vec![
        BoundaryViolation {
            amount: Length::from_micrometres(10),
            edge: BoundaryEdge::Left,
        },
        BoundaryViolation {
            amount: Length::from_micrometres(10),
            edge: BoundaryEdge::Top,
        },
    ]);
}

#[test]
fn object_larger_than_writable_region_can_cross_opposite_edges() {
    let writable = rect(100, 200, 300, 400);
    let object = rect(90, 190, 420, 520);
    let report = check_bounds(writable, object).expect("multi-edge overflow");
    assert_eq!(report.violations, vec![
        BoundaryViolation {
            amount: Length::from_micrometres(110),
            edge: BoundaryEdge::Bottom,
        },
        BoundaryViolation {
            amount: Length::from_micrometres(10),
            edge: BoundaryEdge::Left,
        },
        BoundaryViolation {
            amount: Length::from_micrometres(110),
            edge: BoundaryEdge::Right,
        },
        BoundaryViolation {
            amount: Length::from_micrometres(10),
            edge: BoundaryEdge::Top,
        },
    ]);
}

#[test]
fn zero_extent_point_inside_bounds_is_not_a_minimum_size_decision() {
    let writable = rect(100, 200, 300, 400);
    let object = rect(200, 300, 0, 0);
    assert!(
        check_bounds(writable, object)
            .expect("zero extent bounds only")
            .is_within_bounds()
    );
}

#[test]
fn coordinate_overflow_is_typed_before_comparison() {
    let valid = rect(0, 0, 1, 1);
    let overflowing = rect(u64::MAX, 0, 1, 1);
    assert_eq!(
        check_bounds(valid, overflowing),
        Err(BoundsError::ObjectCoordinateOverflow),
    );
    assert_eq!(
        check_bounds(overflowing, valid),
        Err(BoundsError::WritableCoordinateOverflow),
    );
}
