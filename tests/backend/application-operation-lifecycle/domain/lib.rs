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
//   - Regression evidence for frozen effect boundaries and cancellation
//     resolution.
// - Must-Not:
//   - Simulate cancellation, progress, retry, transport, or operation
//     execution.
// - Allows:
//   - Inputs: Every operation crossed with all three cancellation observations.
//   - Outputs: Exact effect-boundary and cancellation-resolution assertions.
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
    ApplicationCancellationDisposition, ApplicationCancellationObservation,
    ApplicationCancellationResolution, ApplicationOperationClass,
    ApplicationOperationEffectBoundary, application_cancellation_resolution,
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

#[test]
fn all_18_operation_cancellation_states_match_frozen_boundary_rule() {
    let operations = [
        ApplicationOperationClass::Apply,
        ApplicationOperationClass::Export,
        ApplicationOperationClass::HistoryTraversal,
        ApplicationOperationClass::Plan,
        ApplicationOperationClass::Render,
        ApplicationOperationClass::Validate,
    ];
    let observations = [
        ApplicationCancellationObservation::EffectBoundaryCrossed,
        ApplicationCancellationObservation::RequestOnly,
        ApplicationCancellationObservation::TookEffectBeforeBoundary,
    ];
    let mut cases = 0_usize;
    for operation in operations {
        let boundary = application_operation_effect_boundary(operation);
        for observation in observations {
            let expected = match observation {
                ApplicationCancellationObservation::RequestOnly => None,
                ApplicationCancellationObservation::
                    TookEffectBeforeBoundary => {
                    Some(ApplicationCancellationResolution {
                        boundary,
                        disposition: ApplicationCancellationDisposition::
                            CancelledBeforeEffect,
                    })
                },
                ApplicationCancellationObservation::EffectBoundaryCrossed => {
                    Some(ApplicationCancellationResolution {
                        boundary,
                        disposition: ApplicationCancellationDisposition::
                            EffectRemainsAuthoritative,
                    })
                },
            };
            assert_eq!(
                application_cancellation_resolution(operation, observation),
                expected,
                "operation={operation:?} observation={observation:?}",
            );
            cases += 1;
        }
    }
    assert_eq!(cases, 18);
}
