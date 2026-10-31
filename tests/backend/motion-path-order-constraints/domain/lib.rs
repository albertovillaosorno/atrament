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
//   - Regression evidence for exact-plan path-order preservation constraints.
// - Must-Not:
//   - Optimize routes, score travel/drying, infer constraints, change geometry,
//     choose dynamics, or authorize physical execution.
// - Allows:
//   - Inputs: Deterministic plan identity/count, constraints, and permutations.
//   - Outputs: Assertions over structural and preservation validation.
//   - Side effects: Process-local test allocation only.
// - Split-When:
//   - Route optimization or drying scoring gains independent fixtures.
// - Merge-When:
//   - Order validation moves into the motion-plan compiler harness.
// - Summary:
//   - Proves unconstrained movement cannot violate declared handwriting order.
// - Description:
//   - Covers all preservation reasons, permutation checks, and exact failures.
// - Usage:
//   - Compile directly against the motion-path-order-constraints domain.
// - Defaults:
//   - Only explicitly declared original-order relations are protected.
//

//! Path ordering and safe motion constraints regression tests.

use std::ptr;

use atrament_motion_path_order_constraints::{
    validate_candidate_operation_order,
    validate_candidate_operation_order_against_validated,
    validate_candidate_operation_order_view, validate_motion_order_constraints,
    validate_motion_order_constraints_view, MotionOrderConstraint,
    MotionOrderConstraintSet, MotionOrderPreservationReason,
    MotionOrderValidationError,
};

fn visit_permutations(
    values: &mut [usize],
    index: usize,
    visit: &mut impl FnMut(&[usize]),
) {
    if index == values.len() {
        visit(values);
        return;
    }
    for candidate_index in index..values.len() {
        values.swap(index, candidate_index);
        visit_permutations(values, index.saturating_add(1), visit);
        values.swap(index, candidate_index);
    }
}

fn expected_constraint_result(
    constraints: &MotionOrderConstraintSet<&str>,
    candidate: &[usize],
) -> Result<(), MotionOrderValidationError> {
    for (constraint_index, constraint) in
        constraints.constraints.iter().enumerate()
    {
        let earlier = candidate
            .iter()
            .position(|&item| item == constraint.earlier_operation);
        let later = candidate
            .iter()
            .position(|&item| item == constraint.later_operation);
        let (Some(earlier_position), Some(later_position)) = (earlier, later)
        else {
            return Err(MotionOrderValidationError::CandidateLengthMismatch {
                observed: candidate.len(),
                required: constraints.operation_count,
            });
        };
        if earlier_position >= later_position {
            return Err(MotionOrderValidationError::ConstraintViolated {
                constraint_index,
            });
        }
    }
    Ok(())
}

fn constraints() -> MotionOrderConstraintSet<&'static str> {
    MotionOrderConstraintSet {
        constraints: vec![
            MotionOrderConstraint {
                earlier_operation: 0,
                later_operation: 2,
                reason: MotionOrderPreservationReason::Join,
            },
            MotionOrderConstraint {
                earlier_operation: 2,
                later_operation: 4,
                reason: MotionOrderPreservationReason::InkBehavior,
            },
            MotionOrderConstraint {
                earlier_operation: 1,
                later_operation: 5,
                reason: MotionOrderPreservationReason::Semantic,
            },
            MotionOrderConstraint {
                earlier_operation: 4,
                later_operation: 5,
                reason: MotionOrderPreservationReason::HandwritingProfile,
            },
        ],
        operation_count: 6,
        plan_identity: "plan-42",
    }
}

#[test]
fn all_frozen_preservation_reasons_remain_explicit_and_plan_bound() {
    let constraints = constraints();
    assert_eq!(constraints.plan_identity, "plan-42");
    assert_eq!(constraints.operation_count, 6);
    assert_eq!(
        constraints
            .constraints
            .iter()
            .map(|item| item.reason)
            .collect::<Vec<_>>(),
        [
            MotionOrderPreservationReason::Join,
            MotionOrderPreservationReason::InkBehavior,
            MotionOrderPreservationReason::Semantic,
            MotionOrderPreservationReason::HandwritingProfile,
        ],
    );
    assert_eq!(validate_motion_order_constraints(&constraints), Ok(()));
}

#[test]
fn validated_constraint_view_borrows_exact_constraint_set() {
    let constraints = constraints();
    let validation = validate_motion_order_constraints_view(&constraints);
    assert!(validation.is_ok(), "valid constraints seal");
    if let Ok(sealed) = validation {
        assert!(ptr::eq(sealed.constraints(), ptr::from_ref(&constraints)));
        assert_eq!(sealed.constraints().plan_identity, "plan-42");
    }
}

#[test]
fn invalid_constraints_never_produce_validated_constraint_evidence() {
    let mut constraints = constraints();
    if let Some(invalid) = constraints.constraints.get_mut(2) {
        invalid.later_operation = 9;
    }
    assert_eq!(
        validate_motion_order_constraints_view(&constraints),
        Err(MotionOrderValidationError::ConstraintOperationOutOfRange {
            constraint_index: 2,
            operation_index: 9,
        }),
    );
}

#[test]
fn unconstrained_operations_may_move_when_every_preserved_pair_stays_ordered() {
    let constraints = constraints();
    let candidate = [0, 3, 2, 1, 4, 5];
    assert_eq!(
        validate_candidate_operation_order(&constraints, &candidate),
        Ok(()),
    );
}

#[test]
fn validated_order_view_borrows_exact_candidate_and_constraint_set() {
    let constraints = constraints();
    let candidate = [0, 3, 2, 1, 4, 5];
    let validation =
        validate_candidate_operation_order_view(&constraints, &candidate);
    assert!(validation.is_ok(), "valid order seals");
    if let Ok(sealed) = validation {
        assert!(ptr::eq(sealed.candidate(), candidate.as_slice()));
        assert!(ptr::eq(sealed.constraints(), ptr::from_ref(&constraints)));
        assert_eq!(sealed.constraints().plan_identity, "plan-42");
        assert_eq!(sealed.candidate(), candidate);
    }
}

#[test]
fn validated_constraints_can_check_multiple_candidates_without_revalidation() {
    let constraints = constraints();
    let validation = validate_motion_order_constraints_view(&constraints);
    assert!(validation.is_ok(), "valid constraints seal");
    if let Ok(sealed) = validation {
        let first = [0, 3, 2, 1, 4, 5];
        let second = [1, 0, 3, 2, 4, 5];
        for candidate in [&first[..], &second[..]] {
            let checked = validate_candidate_operation_order_against_validated(
                &sealed, candidate,
            );
            assert!(checked.is_ok(), "candidate preserves constraints");
            if let Ok(order) = checked {
                assert!(ptr::eq(
                    order.constraints(),
                    ptr::from_ref(&constraints)
                ));
                assert!(ptr::eq(
                    order.validated_constraints().constraints(),
                    ptr::from_ref(&constraints),
                ));
                assert!(ptr::eq(order.candidate(), candidate));
            }
        }
    }
}

#[test]
fn every_six_operation_permutation_matches_independent_constraint_oracle() {
    let constraints = constraints();
    let validation = validate_motion_order_constraints_view(&constraints);
    assert!(validation.is_ok(), "all valid constraints seal");
    let Ok(validated) = validation else {
        return;
    };
    let mut candidate = [0, 1, 2, 3, 4, 5];
    let mut cases = 0u32;
    let mut admitted = 0u32;
    let mut rejected = 0u32;

    visit_permutations(&mut candidate, 0, &mut |permutation| {
        cases += 1;
        let expected = expected_constraint_result(&constraints, permutation);
        assert_eq!(
            validate_candidate_operation_order(&constraints, permutation),
            expected,
            "direct validation mismatch for {permutation:?}",
        );
        match validate_candidate_operation_order_against_validated(
            &validated,
            permutation,
        ) {
            Ok(order) => {
                assert_eq!(expected, Ok(()));
                assert_eq!(order.candidate(), permutation);
                assert!(ptr::eq(
                    order.constraints(),
                    ptr::from_ref(&constraints)
                ));
                admitted += 1;
            }
            Err(error) => {
                assert_eq!(Err(error), expected);
                rejected += 1;
            }
        }
    });

    assert_eq!(cases, 720);
    assert_eq!(admitted, 24);
    assert_eq!(rejected, 696);
}

#[test]
fn invalid_candidate_never_produces_validated_order_evidence() {
    let constraints = constraints();
    assert_eq!(
        validate_candidate_operation_order_view(
            &constraints,
            &[3, 2, 0, 1, 4, 5],
        ),
        Err(MotionOrderValidationError::ConstraintViolated {
            constraint_index: 0,
        }),
    );
}

#[test]
fn candidate_that_reverses_one_preserved_pair_fails_at_exact_constraint() {
    let constraints = constraints();
    let candidate = [3, 2, 0, 1, 4, 5];
    assert_eq!(
        validate_candidate_operation_order(&constraints, &candidate),
        Err(MotionOrderValidationError::ConstraintViolated {
            constraint_index: 0,
        }),
    );
}

#[test]
fn candidate_must_be_exact_permutation_of_source_operations() {
    let constraints = constraints();
    assert_eq!(
        validate_candidate_operation_order(&constraints, &[0, 1, 2, 3, 4]),
        Err(MotionOrderValidationError::CandidateLengthMismatch {
            observed: 5,
            required: 6,
        }),
    );
    assert_eq!(
        validate_candidate_operation_order(&constraints, &[0, 1, 2, 3, 4, 6]),
        Err(MotionOrderValidationError::CandidateOperationOutOfRange {
            candidate_position: 5,
            operation_index: 6,
        }),
    );
    assert_eq!(
        validate_candidate_operation_order(&constraints, &[0, 1, 2, 3, 4, 4]),
        Err(MotionOrderValidationError::CandidateDuplicateOperation {
            operation_index: 4,
        }),
    );
}

#[test]
fn constraints_must_reference_forward_relations_in_exact_source_plan() {
    let mut invalid = constraints();
    if let Some(item) = invalid.constraints.get_mut(1) {
        item.later_operation = 2;
    }
    assert_eq!(
        validate_motion_order_constraints(&invalid),
        Err(MotionOrderValidationError::ConstraintNotForward {
            constraint_index: 1,
            earlier_operation: 2,
            later_operation: 2,
        }),
    );

    if let Some(item) = invalid.constraints.get_mut(1) {
        item.later_operation = 7;
    }
    assert_eq!(
        validate_motion_order_constraints(&invalid),
        Err(MotionOrderValidationError::ConstraintOperationOutOfRange {
            constraint_index: 1,
            operation_index: 7,
        }),
    );
}

#[test]
fn invalid_constraint_set_rejects_before_candidate_shape() {
    let mut invalid = constraints();
    if let Some(item) = invalid.constraints.get_mut(0) {
        item.later_operation = invalid.operation_count;
    }
    assert_eq!(
        validate_candidate_operation_order(&invalid, &[0]),
        Err(MotionOrderValidationError::ConstraintOperationOutOfRange {
            constraint_index: 0,
            operation_index: invalid.operation_count,
        }),
    );
}

#[test]
fn impossible_operation_count_rejects_before_candidate_position_allocation() {
    let constraints = MotionOrderConstraintSet {
        constraints: Vec::new(),
        operation_count: usize::MAX,
        plan_identity: "unmaterialized-plan",
    };
    assert_eq!(
        validate_candidate_operation_order(&constraints, &[]),
        Err(MotionOrderValidationError::CandidateLengthMismatch {
            observed: 0,
            required: usize::MAX,
        }),
    );
}
