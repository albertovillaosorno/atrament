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
//   - Read-only MCP projection of frozen semantic-history result meaning.
// - Must-Not:
//   - Traverse history, expose MCP tools, admit sessions, persist retry state,
//     execute cancellation, allocate revisions, or infer transport outcomes.
// - Allows:
//   - Inputs: One generic MCP capability plus frozen history result or
//     availability evidence.
//   - Outputs: History commit disposition or directional availability only for
//     the HistoryTraversal capability.
//   - Side effects: None.
// - Split-When:
//   - Executable MCP history behavior gains independent adapter authority.
// - Merge-When:
//   - Final MCP history adapter directly owns all result/availability mapping.
// - Summary:
//   - Keeps Undo/Redo automation subordinate to shared history authority.
// - Description:
//   - Projects history meaning only through the dedicated MCP history class.
// - Usage:
//   - Interpret admitted history results or inspect directional availability.
// - Defaults:
//   - Every non-history MCP capability returns no history projection.
//

//! Read-only MCP projection onto shared semantic-history application meaning.

use atrament_application_operation_lifecycle as lifecycle;
use atrament_mcp_capability_effect::McpApplicationCapabilityClass;
use atrament_semantic_history_result::{
    SemanticHistoryCommitDisposition, SemanticHistoryResultClass,
    classify_history_cancellation_result, semantic_history_commit_disposition,
    semantic_history_direction_is_available,
};
use atrament_semantic_notebook_port::{
    HistoryAvailabilityOutcome, HistoryDirection,
};

/// Project one qualified cancellation observation through `HistoryTraversal`
/// only.
///
/// A request alone remains unresolved. This maps shared lifecycle evidence
/// only;
/// it does not admit, signal, schedule, or execute cancellation.
#[must_use]
pub const fn mcp_history_cancellation_result(
    capability: McpApplicationCapabilityClass,
    observation: lifecycle::ApplicationCancellationObservation,
) -> Option<SemanticHistoryResultClass> {
    match capability {
        McpApplicationCapabilityClass::HistoryTraversal => {
            classify_history_cancellation_result(observation)
        },
        McpApplicationCapabilityClass::Apply
        | McpApplicationCapabilityClass::CommandContext
        | McpApplicationCapabilityClass::Export
        | McpApplicationCapabilityClass::Inspect
        | McpApplicationCapabilityClass::Plan
        | McpApplicationCapabilityClass::Render
        | McpApplicationCapabilityClass::Validate => None,
    }
}

/// Project one history result only through the MCP `HistoryTraversal`
/// capability.
#[must_use]
pub const fn mcp_history_commit_disposition(
    capability: McpApplicationCapabilityClass,
    result: SemanticHistoryResultClass,
) -> Option<SemanticHistoryCommitDisposition> {
    match capability {
        McpApplicationCapabilityClass::HistoryTraversal => {
            Some(semantic_history_commit_disposition(result))
        },
        McpApplicationCapabilityClass::Apply
        | McpApplicationCapabilityClass::CommandContext
        | McpApplicationCapabilityClass::Export
        | McpApplicationCapabilityClass::Inspect
        | McpApplicationCapabilityClass::Plan
        | McpApplicationCapabilityClass::Render
        | McpApplicationCapabilityClass::Validate => None,
    }
}

/// Read one Undo/Redo availability fact through `HistoryTraversal` only.
///
/// This exposes backend-owned read-only availability. It does not attempt a
/// traversal, reserve history position, or make retry/cancellation executable.
#[must_use]
pub const fn mcp_history_direction_is_available(
    capability: McpApplicationCapabilityClass,
    availability: &HistoryAvailabilityOutcome,
    direction: HistoryDirection,
) -> Option<bool> {
    match capability {
        McpApplicationCapabilityClass::HistoryTraversal => Some(
            semantic_history_direction_is_available(availability, direction),
        ),
        McpApplicationCapabilityClass::Apply
        | McpApplicationCapabilityClass::CommandContext
        | McpApplicationCapabilityClass::Export
        | McpApplicationCapabilityClass::Inspect
        | McpApplicationCapabilityClass::Plan
        | McpApplicationCapabilityClass::Render
        | McpApplicationCapabilityClass::Validate => None,
    }
}
