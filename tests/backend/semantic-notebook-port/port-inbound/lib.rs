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
//   - Compile-time evidence for semantic command core result taxonomy.
// - Must-Not:
//   - Assign wire names, transport statuses, diagnostics, or retry behavior.
// - Allows:
//   - Inputs: The frozen transport-neutral core result classes.
//   - Outputs: Exhaustive classification evidence for all core result classes.
//   - Side effects: None.
// - Split-When:
//   - Result projection gains independently executable application behavior.
// - Merge-When:
//   - Application result fixtures completely subsume port compilation evidence.
// - Summary:
//   - Pins core result semantics without treating transport loss as a result.
// - Description:
//   - Keeps typed application outcomes distinct from adapter framing.
// - Usage:
//   - Compile exhaustively against the inbound semantic application port.
// - Defaults:
//   - No final wire enum name or status mapping is implied.
//
use atrament_semantic_notebook_port::SemanticCommandResultClass;

const ALL_CORE_RESULT_CLASSES: [SemanticCommandResultClass; 15] = [
    SemanticCommandResultClass::Applied,
    SemanticCommandResultClass::CancelledBeforeCommit,
    SemanticCommandResultClass::CommandContextMismatch,
    SemanticCommandResultClass::DependencyGraphRejection,
    SemanticCommandResultClass::IdempotentReplay,
    SemanticCommandResultClass::InternalFailureKnownNoCommit,
    SemanticCommandResultClass::NoOp,
    SemanticCommandResultClass::ResourceLimitRejection,
    SemanticCommandResultClass::RetryConflict,
    SemanticCommandResultClass::SemanticValidationRejection,
    SemanticCommandResultClass::StaleBase,
    SemanticCommandResultClass::SuccessfulValidation,
    SemanticCommandResultClass::UnrepresentableOrUnresolved,
    SemanticCommandResultClass::UnsupportedProtocolOrCapability,
    SemanticCommandResultClass::WritableScopeViolation,
];

const fn semantic_index(class: SemanticCommandResultClass) -> usize {
    match class {
        SemanticCommandResultClass::Applied => 0,
        SemanticCommandResultClass::CancelledBeforeCommit => 1,
        SemanticCommandResultClass::CommandContextMismatch => 2,
        SemanticCommandResultClass::DependencyGraphRejection => 3,
        SemanticCommandResultClass::IdempotentReplay => 4,
        SemanticCommandResultClass::InternalFailureKnownNoCommit => 5,
        SemanticCommandResultClass::NoOp => 6,
        SemanticCommandResultClass::ResourceLimitRejection => 7,
        SemanticCommandResultClass::RetryConflict => 8,
        SemanticCommandResultClass::SemanticValidationRejection => 9,
        SemanticCommandResultClass::StaleBase => 10,
        SemanticCommandResultClass::SuccessfulValidation => 11,
        SemanticCommandResultClass::UnrepresentableOrUnresolved => 12,
        SemanticCommandResultClass::UnsupportedProtocolOrCapability => 13,
        SemanticCommandResultClass::WritableScopeViolation => 14,
    }
}

#[test]
fn core_result_taxonomy_has_exactly_15_exhaustive_classes() {
    assert_eq!(ALL_CORE_RESULT_CLASSES.len(), 15);
    for (expected, class) in ALL_CORE_RESULT_CLASSES.into_iter().enumerate() {
        assert_eq!(semantic_index(class), expected);
    }
}
