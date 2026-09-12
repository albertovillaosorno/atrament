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
    validate_candidate_operation_order, validate_candidate_operation_order_view,
    validate_motion_order_constraints,
};

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
    let validated = validate_candidate_operation_order_view(
        &constraints,
        &candidate,
    )
    .expect("valid permutation produces read-only evidence");
    assert!(std::ptr::eq(validated.candidate(), candidate.as_slice()));
    assert!(std::ptr::eq(validated.constraints(), &constraints));
    assert_eq!(validated.constraints().plan_identity, "plan-42");
    assert_eq!(validated.candidate(), candidate);
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
