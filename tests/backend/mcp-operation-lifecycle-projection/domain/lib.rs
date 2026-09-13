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
//   - Regression evidence for the frozen MCP lifecycle crosswalk.
// - Must-Not:
//   - Start MCP, infer tool admission, simulate progress/cancellation, or
//     execute application operations.
// - Allows:
//   - Inputs: All eight generic MCP application capability classes.
//   - Outputs: Exact lifecycle operation/effect-boundary projection assertions.
//   - Side effects: None.
// - Split-When:
//   - Executable adapter lifecycle fixtures gain independent authority.
// - Merge-When:
//   - MCP adapter parity tests fully subsume this structural crosswalk.
// - Summary:
//   - Pins six lifecycle mappings and two intentionally absent mappings.
// - Description:
//   - Prevents Inspect/context from acquiring operation lifecycle implicitly.
// - Usage:
//   - Exhaustively compare every generic MCP capability with the frozen maps.
// - Defaults:
//   - No mapping means no shared application operation lifecycle is implied.
//
use atrament_application_operation_lifecycle::{
    ApplicationOperationClass, ApplicationOperationCompletionDisposition,
    ApplicationOperationCompletionObservation,
    ApplicationOperationEffectBoundary,
};
use atrament_mcp_capability_effect::McpApplicationCapabilityClass;
use atrament_mcp_operation_lifecycle_projection::{
    mcp_application_operation_class,
    mcp_application_operation_completion_disposition,
    mcp_application_operation_effect_boundary,
};

#[test]
fn all_eight_mcp_capabilities_have_exact_lifecycle_projection() {
    let cases = [
        (
            McpApplicationCapabilityClass::Apply,
            Some((
                ApplicationOperationClass::Apply,
                ApplicationOperationEffectBoundary::AcceptedSemanticCommit,
            )),
        ),
        (McpApplicationCapabilityClass::CommandContext, None),
        (
            McpApplicationCapabilityClass::Export,
            Some((
                ApplicationOperationClass::Export,
                ApplicationOperationEffectBoundary::FileCommit,
            )),
        ),
        (
            McpApplicationCapabilityClass::HistoryTraversal,
            Some((
                ApplicationOperationClass::HistoryTraversal,
                ApplicationOperationEffectBoundary::HistoryTraversalCommit,
            )),
        ),
        (McpApplicationCapabilityClass::Inspect, None),
        (
            McpApplicationCapabilityClass::Plan,
            Some((
                ApplicationOperationClass::Plan,
                ApplicationOperationEffectBoundary::ReadOnlyCompletion,
            )),
        ),
        (
            McpApplicationCapabilityClass::Render,
            Some((
                ApplicationOperationClass::Render,
                ApplicationOperationEffectBoundary::ReadOnlyCompletion,
            )),
        ),
        (
            McpApplicationCapabilityClass::Validate,
            Some((
                ApplicationOperationClass::Validate,
                ApplicationOperationEffectBoundary::ReadOnlyCompletion,
            )),
        ),
    ];
    assert_eq!(cases.len(), 8);
    for (capability, expected) in cases {
        assert_eq!(
            mcp_application_operation_class(capability),
            expected.map(|(operation, _boundary)| operation),
        );
        assert_eq!(
            mcp_application_operation_effect_boundary(capability),
            expected.map(|(_operation, boundary)| boundary),
        );
    }
}


#[test]
fn all_40_capability_completion_pairs_preserve_lifecycle_authority() {
    const CAPABILITIES: [McpApplicationCapabilityClass; 8] = [
        McpApplicationCapabilityClass::Apply,
        McpApplicationCapabilityClass::CommandContext,
        McpApplicationCapabilityClass::Export,
        McpApplicationCapabilityClass::HistoryTraversal,
        McpApplicationCapabilityClass::Inspect,
        McpApplicationCapabilityClass::Plan,
        McpApplicationCapabilityClass::Render,
        McpApplicationCapabilityClass::Validate,
    ];
    const OBSERVATIONS: [ApplicationOperationCompletionObservation; 5] = [
        ApplicationOperationCompletionObservation::CancellationRequest,
        ApplicationOperationCompletionObservation::FinalTypedResultOrReceipt,
        ApplicationOperationCompletionObservation::ProgressObservation,
        ApplicationOperationCompletionObservation::SameRetryRecovery,
        ApplicationOperationCompletionObservation::TransportTermination,
    ];
    let mut cases = 0_usize;
    for capability in CAPABILITIES {
        for observation in OBSERVATIONS {
            let expected = match capability {
                McpApplicationCapabilityClass::CommandContext
                | McpApplicationCapabilityClass::Inspect => None,
                McpApplicationCapabilityClass::Apply
                | McpApplicationCapabilityClass::Export
                | McpApplicationCapabilityClass::HistoryTraversal => {
                    Some(match observation {
                        ApplicationOperationCompletionObservation::
                            FinalTypedResultOrReceipt => {
                            ApplicationOperationCompletionDisposition::
                                CompleteByFinalResult
                        },
                        ApplicationOperationCompletionObservation::
                            SameRetryRecovery => {
                            ApplicationOperationCompletionDisposition::
                                CompleteByRecoveredMutatingOutcome
                        },
                        ApplicationOperationCompletionObservation::
                            CancellationRequest
                        | ApplicationOperationCompletionObservation::
                            ProgressObservation
                        | ApplicationOperationCompletionObservation::
                            TransportTermination => {
                            ApplicationOperationCompletionDisposition::
                                NotEstablished
                        },
                    })
                },
                McpApplicationCapabilityClass::Plan
                | McpApplicationCapabilityClass::Render
                | McpApplicationCapabilityClass::Validate => match observation {
                    ApplicationOperationCompletionObservation::
                        SameRetryRecovery => None,
                    ApplicationOperationCompletionObservation::
                        FinalTypedResultOrReceipt => Some(
                        ApplicationOperationCompletionDisposition::
                            CompleteByFinalResult,
                    ),
                    ApplicationOperationCompletionObservation::
                        CancellationRequest
                    | ApplicationOperationCompletionObservation::
                        ProgressObservation
                    | ApplicationOperationCompletionObservation::
                        TransportTermination => Some(
                        ApplicationOperationCompletionDisposition::
                            NotEstablished,
                    ),
                },
            };
            assert_eq!(
                mcp_application_operation_completion_disposition(
                    capability,
                    observation,
                ),
                expected,
                "capability={capability:?} observation={observation:?}",
            );
            cases += 1;
        }
    }
    assert_eq!(cases, 40);
}
