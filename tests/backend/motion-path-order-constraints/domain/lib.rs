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
use atrament_motion_path_order_constraints::{
    MotionOrderConstraint, MotionOrderConstraintSet,
    MotionOrderPreservationReason, MotionOrderValidationError,
    validate_candidate_operation_order,
    validate_candidate_operation_order_against_validated,
    validate_candidate_operation_order_view, validate_motion_order_constraints,
    validate_motion_order_constraints_view,
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
        visit_permutations(values, index + 1, visit);
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
        let earlier_position = candidate
            .iter()
            .position(|&item| item == constraint.earlier_operation)
            .expect("permutation contains earlier operation");
        let later_position = candidate
            .iter()
            .position(|&item| item == constraint.later_operation)
            .expect("permutation contains later operation");
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
    let validated = validate_motion_order_constraints_view(&constraints)
        .expect("valid constraints produce sealed evidence");
    assert!(std::ptr::eq(validated.constraints(), &constraints));
    assert_eq!(validated.constraints().plan_identity, "plan-42");
}

#[test]
fn invalid_constraints_never_produce_validated_constraint_evidence() {
    let mut constraints = constraints();
    constraints.constraints[2].later_operation = 9;
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
    let validated =
        validate_candidate_operation_order_view(&constraints, &candidate)
            .expect("valid permutation produces read-only evidence");
    assert!(std::ptr::eq(validated.candidate(), candidate.as_slice()));
    assert!(std::ptr::eq(validated.constraints(), &constraints));
    assert_eq!(validated.constraints().plan_identity, "plan-42");
    assert_eq!(validated.candidate(), candidate);
}

#[test]
fn validated_constraints_can_check_multiple_candidates_without_revalidation() {
    let constraints = constraints();
    let validated = validate_motion_order_constraints_view(&constraints)
        .expect("valid constraints produce sealed evidence");
    let first = [0, 3, 2, 1, 4, 5];
    let second = [1, 0, 3, 2, 4, 5];
    for candidate in [&first[..], &second[..]] {
        let order = validate_candidate_operation_order_against_validated(
            &validated, candidate,
        )
        .expect("candidate preserves validated constraints");
        assert!(std::ptr::eq(order.constraints(), &constraints));
        assert!(std::ptr::eq(
            order.validated_constraints().constraints(),
            &constraints,
        ));
        assert!(std::ptr::eq(order.candidate(), candidate));
    }
}

#[test]
fn every_six_operation_permutation_matches_independent_constraint_oracle() {
    let constraints = constraints();
    let validated = validate_motion_order_constraints_view(&constraints)
        .expect("valid constraints produce sealed evidence");
    let mut candidate = [0, 1, 2, 3, 4, 5];
    let mut cases = 0_u32;
    let mut admitted = 0_u32;
    let mut rejected = 0_u32;

    visit_permutations(&mut candidate, 0, &mut |candidate| {
        cases += 1;
        let expected = expected_constraint_result(&constraints, candidate);
        assert_eq!(
            validate_candidate_operation_order(&constraints, candidate),
            expected,
            "direct validation mismatch for {candidate:?}",
        );
        match validate_candidate_operation_order_against_validated(
            &validated, candidate,
        ) {
            Ok(order) => {
                assert_eq!(expected, Ok(()));
                assert_eq!(order.candidate(), candidate);
                assert!(std::ptr::eq(order.constraints(), &constraints));
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
    invalid.constraints[1].later_operation = 2;
    assert_eq!(
        validate_motion_order_constraints(&invalid),
        Err(MotionOrderValidationError::ConstraintNotForward {
            constraint_index: 1,
            earlier_operation: 2,
            later_operation: 2,
        }),
    );

    invalid.constraints[1].later_operation = 7;
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
    invalid.constraints[0].later_operation = invalid.operation_count;
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
