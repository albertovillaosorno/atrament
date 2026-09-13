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
//   - Read-only MCP projection of frozen semantic-command result meaning.
// - Must-Not:
//   - Admit sessions, expose tools, execute Validate/Apply, normalize batches,
//     persist retry state, schedule cancellation, or infer transport outcomes.
// - Allows:
//   - Inputs: One generic MCP capability plus a frozen semantic result or
//     qualified Apply cancellation observation.
//   - Outputs: Operation-compatible commit disposition or Apply cancellation
//     result only through Validate or Apply.
//   - Side effects: None.
// - Split-When:
//   - Executable MCP command-result handling gains adapter authority.
// - Merge-When:
//   - Final MCP command adapter directly owns semantic result projection.
// - Summary:
//   - Keeps MCP semantic results subordinate to shared application taxonomy.
// - Description:
//   - Maps only Validate and Apply and preserves exact result applicability.
// - Usage:
//   - Project typed semantic results after an independently admitted operation.
// - Defaults:
//   - Other MCP capabilities and incompatible results return no projection.
//

//! Read-only MCP projection onto semantic-command application results.

use atrament_application_operation_lifecycle as lifecycle;
use atrament_mcp_capability_effect::McpApplicationCapabilityClass;
use atrament_semantic_command_result::{
    SemanticCommandOperation, classify_semantic_apply_cancellation_result,
    semantic_command_commit_disposition, semantic_command_result_applies_to,
};
use atrament_semantic_notebook_port::{
    SemanticCommandCommitDisposition, SemanticCommandResultClass,
};

/// Project qualified Apply cancellation only through the MCP Apply capability.
///
/// A request alone remains unresolved. This does not admit Apply, signal or
/// schedule cancellation, execute a command batch, or recover a retry.
#[must_use]
pub const fn mcp_semantic_apply_cancellation_result(
    capability: McpApplicationCapabilityClass,
    observation: lifecycle::ApplicationCancellationObservation,
) -> Option<SemanticCommandResultClass> {
    match capability {
        McpApplicationCapabilityClass::Apply => {
            classify_semantic_apply_cancellation_result(observation)
        },
        McpApplicationCapabilityClass::CommandContext
        | McpApplicationCapabilityClass::Export
        | McpApplicationCapabilityClass::HistoryTraversal
        | McpApplicationCapabilityClass::Inspect
        | McpApplicationCapabilityClass::Plan
        | McpApplicationCapabilityClass::Render
        | McpApplicationCapabilityClass::Validate => None,
    }
}

/// Project commit meaning only for a compatible MCP semantic-command result.
///
/// Applicability is checked before commit disposition so Validate cannot
/// acquire Apply-only success, replay, retry, no-op, or cancellation meaning.
#[must_use]
pub const fn mcp_semantic_command_commit_disposition(
    capability: McpApplicationCapabilityClass,
    result: SemanticCommandResultClass,
) -> Option<SemanticCommandCommitDisposition> {
    let Some(operation) = mcp_semantic_command_result_operation(
        capability,
    ) else {
        return None;
    };
    if semantic_command_result_applies_to(operation, result) {
        Some(semantic_command_commit_disposition(result))
    } else {
        None
    }
}

/// Return the semantic-result operation owned by one generic MCP capability.
#[must_use]
pub const fn mcp_semantic_command_result_operation(
    capability: McpApplicationCapabilityClass,
) -> Option<SemanticCommandOperation> {
    match capability {
        McpApplicationCapabilityClass::Apply => {
            Some(SemanticCommandOperation::Apply)
        },
        McpApplicationCapabilityClass::Validate => {
            Some(SemanticCommandOperation::Validate)
        },
        McpApplicationCapabilityClass::CommandContext
        | McpApplicationCapabilityClass::Export
        | McpApplicationCapabilityClass::HistoryTraversal
        | McpApplicationCapabilityClass::Inspect
        | McpApplicationCapabilityClass::Plan
        | McpApplicationCapabilityClass::Render => None,
    }
}
