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

//! Writable-region bounds regression tests.

use atrament_fixed_region_bounds::{
    BoundaryEdge, BoundaryViolation, BoundsError, check_bounds,
};
use atrament_physical_page_profile::{Length, Rect};

const fn rect(x: u64, y: u64, width: u64, height: u64) -> Rect {
    Rect {
        height: Length::from_micrometres(height),
        width: Length::from_micrometres(width),
        x: Length::from_micrometres(x),
        y: Length::from_micrometres(y),
    }
}


// A second, small-coordinate oracle stays independent of the u128
// full-range implementation. Both use checked physical units, so a
// generator bug is a typed failure instead of overflowing the test.
fn reference_bounds_small(
    writable: Rect, object: Rect,
) -> Result<Vec<BoundaryViolation>, BoundsError> {
    let writable_right = writable.x.micrometres()
        .checked_add(writable.width.micrometres())
        .ok_or(BoundsError::WritableCoordinateOverflow)?;
    let writable_bottom = writable.y.micrometres()
        .checked_add(writable.height.micrometres())
        .ok_or(BoundsError::WritableCoordinateOverflow)?;
    let object_right = object.x.micrometres()
        .checked_add(object.width.micrometres())
        .ok_or(BoundsError::ObjectCoordinateOverflow)?;
    let object_bottom = object.y.micrometres()
        .checked_add(object.height.micrometres())
        .ok_or(BoundsError::ObjectCoordinateOverflow)?;
    let mut violations = Vec::new();
    if object_bottom > writable_bottom {
        violations.push(BoundaryViolation {
            amount: Length::from_micrometres(
                object_bottom.checked_sub(writable_bottom)
                    .ok_or(BoundsError::ObjectCoordinateOverflow)?,
            ),
            edge: BoundaryEdge::Bottom,
        });
    }
    if object.x < writable.x {
        violations.push(BoundaryViolation {
            amount: Length::from_micrometres(
                writable.x.micrometres()
                    .checked_sub(object.x.micrometres())
                    .ok_or(BoundsError::ObjectCoordinateOverflow)?,
            ),
            edge: BoundaryEdge::Left,
        });
    }
    if object_right > writable_right {
        violations.push(BoundaryViolation {
            amount: Length::from_micrometres(
                object_right.checked_sub(writable_right)
                    .ok_or(BoundsError::ObjectCoordinateOverflow)?,
            ),
            edge: BoundaryEdge::Right,
        });
    }
    if object.y < writable.y {
        violations.push(BoundaryViolation {
            amount: Length::from_micrometres(
                writable.y.micrometres()
                    .checked_sub(object.y.micrometres())
                    .ok_or(BoundsError::ObjectCoordinateOverflow)?,
            ),
            edge: BoundaryEdge::Top,
        });
    }
    Ok(violations)
}

const fn next_bounds_value(seed: &mut u64) -> u64 {
    *seed = seed
        .wrapping_mul(6_364_136_223_846_793_005)
        .wrapping_add(1_442_695_040_888_963_407);
    *seed >> 32
}

const fn next_full_bounds_value(seed: &mut u64) -> u64 {
    *seed = seed
        .wrapping_mul(2_862_933_555_777_941_757)
        .wrapping_add(3_037_000_493);
    *seed
}

fn reference_bounds_full_range(
    writable: Rect,
    object: Rect,
) -> Result<Vec<BoundaryViolation>, BoundsError> {
    let maximum = u128::from(u64::MAX);
    let writable_right_large = u128::from(writable.x.micrometres())
        .checked_add(u128::from(writable.width.micrometres()))
        .ok_or(BoundsError::WritableCoordinateOverflow)?;
    let writable_bottom_large = u128::from(writable.y.micrometres())
        .checked_add(u128::from(writable.height.micrometres()))
        .ok_or(BoundsError::WritableCoordinateOverflow)?;
    if writable_right_large > maximum || writable_bottom_large > maximum {
        return Err(BoundsError::WritableCoordinateOverflow);
    }
    let object_right_large = u128::from(object.x.micrometres())
        .checked_add(u128::from(object.width.micrometres()))
        .ok_or(BoundsError::ObjectCoordinateOverflow)?;
    let object_bottom_large = u128::from(object.y.micrometres())
        .checked_add(u128::from(object.height.micrometres()))
        .ok_or(BoundsError::ObjectCoordinateOverflow)?;
    if object_right_large > maximum || object_bottom_large > maximum {
        return Err(BoundsError::ObjectCoordinateOverflow);
    }
    let writable_right = u64::try_from(writable_right_large)
        .map_err(|_conversion| BoundsError::WritableCoordinateOverflow)?;
    let writable_bottom = u64::try_from(writable_bottom_large)
        .map_err(|_conversion| BoundsError::WritableCoordinateOverflow)?;
    let object_right = u64::try_from(object_right_large)
        .map_err(|_conversion| BoundsError::ObjectCoordinateOverflow)?;
    let object_bottom = u64::try_from(object_bottom_large)
        .map_err(|_conversion| BoundsError::ObjectCoordinateOverflow)?;
    let mut violations = Vec::new();
    if object_bottom > writable_bottom {
        violations.push(BoundaryViolation {
            amount: Length::from_micrometres(
                object_bottom.checked_sub(writable_bottom)
                    .ok_or(BoundsError::ObjectCoordinateOverflow)?,
            ),
            edge: BoundaryEdge::Bottom,
        });
    }
    if object.x < writable.x {
        violations.push(BoundaryViolation {
            amount: Length::from_micrometres(
                writable.x.micrometres()
                    .checked_sub(object.x.micrometres())
                    .ok_or(BoundsError::ObjectCoordinateOverflow)?,
            ),
            edge: BoundaryEdge::Left,
        });
    }
    if object_right > writable_right {
        violations.push(BoundaryViolation {
            amount: Length::from_micrometres(
                object_right.checked_sub(writable_right)
                    .ok_or(BoundsError::ObjectCoordinateOverflow)?,
            ),
            edge: BoundaryEdge::Right,
        });
    }
    if object.y < writable.y {
        violations.push(BoundaryViolation {
            amount: Length::from_micrometres(
                writable.y.micrometres()
                    .checked_sub(object.y.micrometres())
                    .ok_or(BoundsError::ObjectCoordinateOverflow)?,
            ),
            edge: BoundaryEdge::Top,
        });
    }
    Ok(violations)
}


fn boundary_mask(violations: &[BoundaryViolation]) -> usize {
    violations.iter().fold(0usize, |mask, violation| {
        mask | match violation.edge {
            BoundaryEdge::Bottom => 1,
            BoundaryEdge::Left => 2,
            BoundaryEdge::Right => 4,
            BoundaryEdge::Top => 8,
        }
    })
}

#[test]
fn full_range_rectangles_match_u128_reference_oracle() {
    const CASES: usize = 200_000;
    let mut seed = 0xa24b_aed4_963e_e407u64;
    let mut object_overflows = 0usize;
    let mut valid = 0usize;
    let mut writable_overflows = 0usize;
    let mut seen_boundary_masks = [false; 16];
    for case in 0..CASES {
        let writable = rect(
            next_full_bounds_value(&mut seed),
            next_full_bounds_value(&mut seed),
            next_full_bounds_value(&mut seed),
            next_full_bounds_value(&mut seed),
        );
        let object = rect(
            next_full_bounds_value(&mut seed),
            next_full_bounds_value(&mut seed),
            next_full_bounds_value(&mut seed),
            next_full_bounds_value(&mut seed),
        );
        let expected = reference_bounds_full_range(writable, object);
        if let Ok(violations) = &expected {
            assert!(seen_boundary_masks.get_mut(boundary_mask(violations))
                .is_some_and(|seen| { *seen = true; true }));
        }
        match expected {
            Ok(_) => valid = valid.saturating_add(1),
            Err(BoundsError::ObjectCoordinateOverflow) => {
                object_overflows = object_overflows.saturating_add(1);
            },
            Err(BoundsError::WritableCoordinateOverflow) => {
                writable_overflows = writable_overflows.saturating_add(1);
            },
        }
        assert_eq!(
            check_bounds(writable, object).map(|report| report.violations),
            expected,
            "full-range bounds mismatch in generated case {case}",
        );
    }
    assert!(valid > 1_000);
    assert!(object_overflows > 1_000);
    assert!(writable_overflows > 1_000);
    assert!(seen_boundary_masks.into_iter().all(|seen| seen));
}

#[test]
fn small_rectangles_match_reference_bounds_oracle() {
    const CASES: usize = 20_000;
    let mut seed = 0x5eed_b0ad_2026u64;
    let mut seen_boundary_masks = [false; 16];
    for case in 0..CASES {
        let sample = |rng_seed: &mut u64, modulus: u64| {
            next_bounds_value(rng_seed)
                .checked_rem(modulus).unwrap_or_default()
        };
        let writable = rect(
            sample(&mut seed, 21), sample(&mut seed, 21),
            sample(&mut seed, 21), sample(&mut seed, 21),
        );
        let object = rect(
            sample(&mut seed, 31), sample(&mut seed, 31),
            sample(&mut seed, 31), sample(&mut seed, 31),
        );
        let expected = reference_bounds_small(writable, object);
        if let Ok(violations) = &expected {
            assert!(seen_boundary_masks.get_mut(boundary_mask(violations))
                .is_some_and(|seen| { *seen = true; true }));
        }
        assert_eq!(
            check_bounds(writable, object).map(|report| report.violations),
            expected,
            "bounds oracle mismatch in generated case {case}",
        );
    }
    assert!(seen_boundary_masks.into_iter().all(|seen| seen));
}

#[test]
fn exact_fit_has_no_overflow() {
    let writable = rect(10_000, 20_000, 100_000, 200_000);
    assert!(check_bounds(writable, writable)
        .is_ok_and(|report| report.is_within_bounds()));
    assert_eq!(
        check_bounds(writable, writable).map(|report| report.violations),
        Ok(Vec::new()),
    );
}

#[test]
fn first_journey_six_mm_bottom_overflow_is_exact() {
    let writable = rect(10_000, 20_000, 100_000, 200_000);
    let object = rect(20_000, 180_000, 50_000, 46_000);
    assert_eq!(
        check_bounds(writable, object).map(|report| report.violations),
        Ok(vec![BoundaryViolation {
            amount: Length::from_micrometres(6_000),
            edge: BoundaryEdge::Bottom,
        }]),
    );
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
        assert_eq!(
            check_bounds(writable, object).map(|report| report.violations),
            Ok(vec![BoundaryViolation {
                amount: Length::from_micrometres(amount),
                edge,
            }]),
        );
    }
}

#[test]
fn corner_crossing_reports_both_edges_in_stable_order() {
    let writable = rect(100, 200, 300, 400);
    let object = rect(90, 190, 20, 20);
    assert_eq!(
        check_bounds(writable, object).map(|report| report.violations),
        Ok(vec![
        BoundaryViolation {
            amount: Length::from_micrometres(10),
            edge: BoundaryEdge::Left,
        },
        BoundaryViolation {
            amount: Length::from_micrometres(10),
            edge: BoundaryEdge::Top,
        },
        ]),
    );
}

#[test]
fn object_larger_than_writable_region_can_cross_opposite_edges() {
    let writable = rect(100, 200, 300, 400);
    let object = rect(90, 190, 420, 520);
    assert_eq!(
        check_bounds(writable, object).map(|report| report.violations),
        Ok(vec![
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
        ]),
    );
}

#[test]
fn zero_extent_point_inside_bounds_is_not_a_minimum_size_decision() {
    let writable = rect(100, 200, 300, 400);
    let object = rect(200, 300, 0, 0);
    assert!(
        check_bounds(writable, object)
            .is_ok_and(|report| report.is_within_bounds())
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
