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
//   - Regression evidence for frozen application-operation effect boundaries.
// - Must-Not:
//   - Simulate cancellation, progress, retry, transport, or operation
//     execution.
// - Allows:
//   - Inputs: Every frozen first-release lifecycle operation class.
//   - Outputs: Exact effect-boundary assertions for all six operations.
//   - Side effects: None.
// - Split-When:
//   - Executable cancellation/progress fixtures gain independent authority.
// - Merge-When:
//   - One application lifecycle suite subsumes this structural mapping.
// - Summary:
//   - Proves each operation retains its owning commit or completion boundary.
// - Description:
//   - Prevents progress or cancellation from changing operation effect meaning.
// - Usage:
//   - Exhaustively compare all operation classes with the frozen contract.
// - Defaults:
//   - No asynchronous API or cancellation capability is implied.
//
use atrament_application_operation_lifecycle::{
    ApplicationOperationClass, ApplicationOperationEffectBoundary,
    application_operation_effect_boundary,
};

#[test]
fn all_six_operation_effect_boundaries_match_frozen_contract() {
    let cases = [
        (
            ApplicationOperationClass::Apply,
            ApplicationOperationEffectBoundary::AcceptedSemanticCommit,
        ),
        (
            ApplicationOperationClass::Export,
            ApplicationOperationEffectBoundary::FileCommit,
        ),
        (
            ApplicationOperationClass::HistoryTraversal,
            ApplicationOperationEffectBoundary::HistoryTraversalCommit,
        ),
        (
            ApplicationOperationClass::Plan,
            ApplicationOperationEffectBoundary::ReadOnlyCompletion,
        ),
        (
            ApplicationOperationClass::Render,
            ApplicationOperationEffectBoundary::ReadOnlyCompletion,
        ),
        (
            ApplicationOperationClass::Validate,
            ApplicationOperationEffectBoundary::ReadOnlyCompletion,
        ),
    ];
    assert_eq!(cases.len(), 6);
    for (operation, expected) in cases {
        assert_eq!(application_operation_effect_boundary(operation), expected);
    }
}
