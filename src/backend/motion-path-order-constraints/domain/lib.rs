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
//   - Exact-plan operation-order preservation constraints and candidate checks.
// - Must-Not:
//   - Optimize routes, compute distance/drying cost, infer constraints, change
//     stroke geometry, choose motion dynamics, or authorize physical execution.
// - Allows:
//   - Inputs: Plan identity/count, caller-owned preservation constraints, and a
//     candidate operation-index permutation.
//   - Outputs: Typed validation that the candidate preserves constrained order.
//   - Side effects: Process-local validation allocation only.
// - Split-When:
//   - Route optimization or drying-conflict scoring gains executable authority.
// - Merge-When:
//   - Order constraints become inseparable from one motion-plan compiler.
// - Summary:
//   - Defines what a future optimizer may reorder without changing handwriting.
// - Description:
//   - Keeps joins, ink behavior, semantics, and profile order explicit while
//     permitting caller-selected movement of unconstrained operations.
// - Usage:
//   - Validate any proposed optimized operation order before replacing plan
//     order.
// - Defaults:
//   - No operation is considered constrained unless the caller declares it.
//

//! Exact-plan order-preservation constraints without a routing optimizer.

/// Why the original relative order of two plan operations must be preserved.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum MotionOrderPreservationReason {
    /// Handwriting profile requires this relative stroke/motion order.
    HandwritingProfile,
    /// Ink or drying behavior requires this relative order.
    InkBehavior,
    /// Join continuity requires this relative stroke order.
    Join,
    /// Semantic meaning requires this relative operation order.
    Semantic,
}

/// One required earlier/later relation from the original exact plan order.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MotionOrderConstraint {
    /// Original operation index that must remain earlier in any candidate
    /// order.
    pub earlier_operation: usize,
    /// Original operation index that must remain later in any candidate order.
    pub later_operation: usize,
    /// Explicit reason this original relative order must be preserved.
    pub reason: MotionOrderPreservationReason,
}

/// All declared order-preservation constraints for one exact plan identity.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MotionOrderConstraintSet<PlanIdentity> {
    /// Caller-owned constraints bound to this exact plan order.
    pub constraints: Vec<MotionOrderConstraint>,
    /// Number of operations in the exact source plan.
    pub operation_count: usize,
    /// Exact backend-owned plan identity to which these indexes belong.
    pub plan_identity: PlanIdentity,
}

/// Why an order constraint set or proposed operation permutation is invalid.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MotionOrderValidationError {
    /// Candidate permutation repeats one original operation index.
    CandidateDuplicateOperation {
        /// Repeated original operation index.
        operation_index: usize,
    },
    /// Candidate permutation does not contain the exact source operation count.
    CandidateLengthMismatch {
        /// Number of candidate positions supplied.
        observed: usize,
        /// Exact source plan operation count.
        required: usize,
    },
    /// Candidate contains an operation index outside the exact source plan.
    CandidateOperationOutOfRange {
        /// Candidate sequence position carrying the invalid index.
        candidate_position: usize,
        /// Invalid original operation index.
        operation_index: usize,
    },
    /// A preservation relation does not follow the original plan order.
    ConstraintNotForward {
        /// Zero-based declared constraint index.
        constraint_index: usize,
        /// Original operation index declared as earlier.
        earlier_operation: usize,
        /// Original operation index declared as later.
        later_operation: usize,
    },
    /// One constraint references an operation outside the exact source plan.
    ConstraintOperationOutOfRange {
        /// Zero-based declared constraint index.
        constraint_index: usize,
        /// Invalid original operation index.
        operation_index: usize,
    },
    /// Candidate reorders one explicitly preserved earlier/later relation.
    ConstraintViolated {
        /// Zero-based declared constraint index.
        constraint_index: usize,
    },
}

/// Validate one declared constraint set independently from any proposed order.
///
/// # Errors
///
/// Returns the first out-of-range or non-forward relation in declaration order.
pub fn validate_motion_order_constraints<PlanIdentity>(
    constraints: &MotionOrderConstraintSet<PlanIdentity>,
) -> Result<(), MotionOrderValidationError> {
    for (constraint_index, constraint) in constraints
        .constraints
        .iter()
        .enumerate()
    {
        for operation_index in [
            constraint.earlier_operation,
            constraint.later_operation,
        ] {
            if operation_index >= constraints.operation_count {
                return Err(
                    MotionOrderValidationError::ConstraintOperationOutOfRange {
                        constraint_index,
                        operation_index,
                    },
                );
            }
        }
        if constraint.earlier_operation >= constraint.later_operation {
            return Err(MotionOrderValidationError::ConstraintNotForward {
                constraint_index,
                earlier_operation: constraint.earlier_operation,
                later_operation: constraint.later_operation,
            });
        }
    }
    Ok(())
}

/// Validate a complete proposed permutation against explicit preservation
/// rules.
///
/// # Errors
///
/// Returns the first structural candidate failure or violated constraint.
pub fn validate_candidate_operation_order<PlanIdentity>(
    constraints: &MotionOrderConstraintSet<PlanIdentity>,
    candidate: &[usize],
) -> Result<(), MotionOrderValidationError> {
    validate_motion_order_constraints(constraints)?;
    if candidate.len() != constraints.operation_count {
        return Err(MotionOrderValidationError::CandidateLengthMismatch {
            observed: candidate.len(),
            required: constraints.operation_count,
        });
    }

    let mut candidate_positions = vec![usize::MAX; constraints.operation_count];
    for (candidate_position, &operation_index) in candidate.iter().enumerate() {
        let Some(position) = candidate_positions.get_mut(operation_index) else {
            return Err(
                MotionOrderValidationError::CandidateOperationOutOfRange {
                    candidate_position,
                    operation_index,
                },
            );
        };
        if *position != usize::MAX {
            return Err(MotionOrderValidationError::CandidateDuplicateOperation {
                operation_index,
            });
        }
        *position = candidate_position;
    }

    for (constraint_index, constraint) in constraints
        .constraints
        .iter()
        .enumerate()
    {
        let Some(&earlier_position) =
            candidate_positions.get(constraint.earlier_operation)
        else {
            return Err(
                MotionOrderValidationError::ConstraintOperationOutOfRange {
                    constraint_index,
                    operation_index: constraint.earlier_operation,
                },
            );
        };
        let Some(&later_position) =
            candidate_positions.get(constraint.later_operation)
        else {
            return Err(
                MotionOrderValidationError::ConstraintOperationOutOfRange {
                    constraint_index,
                    operation_index: constraint.later_operation,
                },
            );
        };
        if earlier_position >= later_position {
            return Err(MotionOrderValidationError::ConstraintViolated {
                constraint_index,
            });
        }
    }
    Ok(())
}
