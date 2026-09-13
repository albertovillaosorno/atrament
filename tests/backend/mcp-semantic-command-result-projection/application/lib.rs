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
//   - Exhaustive evidence for MCP semantic-command result projection.
// - Must-Not:
//   - Start MCP, expose tools, execute Validate/Apply, retry, or cancellation.
// - Allows:
//   - Inputs: All eight MCP capabilities, 15 results, and cancellation facts.
//   - Outputs: Exact operation, compatible result, and cancellation projection.
//   - Side effects: None.
// - Split-When:
//   - Live MCP semantic-result parity gains independent adapter fixtures.
// - Merge-When:
//   - Executable MCP command tests fully subsume this structural projection.
// - Summary:
//   - Pins result meaning only to MCP Validate and Apply.
// - Description:
//   - Rejects operation-incompatible result classes before commit projection.
// - Usage:
//   - Compare complete finite cross-products with an independent oracle.
// - Defaults:
//   - Non-command MCP capabilities receive no semantic-command result meaning.
//
use atrament_application_operation_lifecycle::{
    ApplicationCancellationObservation,
};
use atrament_mcp_capability_effect::McpApplicationCapabilityClass;
use atrament_mcp_semantic_command_result_projection::{
    mcp_semantic_apply_cancellation_result,
    mcp_semantic_command_commit_disposition,
    mcp_semantic_command_result_operation,
};
use atrament_semantic_command_result::SemanticCommandOperation;
use atrament_semantic_notebook_port::{
    SemanticCommandCommitDisposition, SemanticCommandResultClass,
};

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

const RESULTS: [SemanticCommandResultClass; 15] = [
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

fn expected_operation(
    capability: McpApplicationCapabilityClass,
) -> Option<SemanticCommandOperation> {
    match capability {
        McpApplicationCapabilityClass::Apply => {
            Some(SemanticCommandOperation::Apply)
        },
        McpApplicationCapabilityClass::Validate => {
            Some(SemanticCommandOperation::Validate)
        },
        _ => None,
    }
}

fn result_applies(
    operation: SemanticCommandOperation,
    result: SemanticCommandResultClass,
) -> bool {
    match result {
        SemanticCommandResultClass::Applied
        | SemanticCommandResultClass::CancelledBeforeCommit
        | SemanticCommandResultClass::IdempotentReplay
        | SemanticCommandResultClass::NoOp
        | SemanticCommandResultClass::RetryConflict => {
            operation == SemanticCommandOperation::Apply
        },
        SemanticCommandResultClass::SuccessfulValidation => {
            operation == SemanticCommandOperation::Validate
        },
        _ => true,
    }
}

fn expected_disposition(
    result: SemanticCommandResultClass,
) -> SemanticCommandCommitDisposition {
    match result {
        SemanticCommandResultClass::Applied => {
            SemanticCommandCommitDisposition::CommittedThisCall
        },
        SemanticCommandResultClass::IdempotentReplay => {
            SemanticCommandCommitDisposition::RecoveredPriorCompletion
        },
        _ => SemanticCommandCommitDisposition::KnownNoNewCommit,
    }
}

#[test]
fn all_eight_capabilities_have_exact_semantic_result_operation_mapping() {
    for capability in CAPABILITIES {
        assert_eq!(
            mcp_semantic_command_result_operation(capability),
            expected_operation(capability),
        );
    }
}

#[test]
fn all_120_capability_result_pairs_preserve_operation_applicability() {
    let mut cases = 0_usize;
    for capability in CAPABILITIES {
        for result in RESULTS {
            let expected = expected_operation(capability).and_then(|operation| {
                result_applies(operation, result)
                    .then(|| expected_disposition(result))
            });
            assert_eq!(
                mcp_semantic_command_commit_disposition(capability, result),
                expected,
                "capability={capability:?} result={result:?}",
            );
            cases += 1;
        }
    }
    assert_eq!(cases, 120);
}

#[test]
fn all_24_capability_cancellation_pairs_project_only_through_apply() {
    const OBSERVATIONS: [ApplicationCancellationObservation; 3] = [
        ApplicationCancellationObservation::EffectBoundaryCrossed,
        ApplicationCancellationObservation::RequestOnly,
        ApplicationCancellationObservation::TookEffectBeforeBoundary,
    ];
    let mut cases = 0_usize;
    for capability in CAPABILITIES {
        for observation in OBSERVATIONS {
            let expected = if capability
                != McpApplicationCapabilityClass::Apply
            {
                None
            } else {
                match observation {
                    ApplicationCancellationObservation::
                        EffectBoundaryCrossed => {
                        Some(SemanticCommandResultClass::Applied)
                    },
                    ApplicationCancellationObservation::RequestOnly => None,
                    ApplicationCancellationObservation::
                        TookEffectBeforeBoundary => {
                        Some(SemanticCommandResultClass::CancelledBeforeCommit)
                    },
                }
            };
            assert_eq!(
                mcp_semantic_apply_cancellation_result(capability, observation),
                expected,
                "capability={capability:?} observation={observation:?}",
            );
            cases += 1;
        }
    }
    assert_eq!(cases, 24);
}
