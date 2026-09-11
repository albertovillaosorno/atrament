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
//   - Transport-neutral MCP-to-application operation lifecycle projection.
// - Must-Not:
//   - Expose MCP tools, admit adapters, schedule work, report progress, cancel
//     execution, recover retries, assign operation IDs, or perform effects.
// - Allows:
//   - Inputs: One frozen generic MCP application capability class.
//   - Outputs: Its shared lifecycle operation and authoritative effect boundary
//     when that capability participates in the lifecycle contract.
//   - Side effects: None.
// - Split-When:
//   - Executable MCP lifecycle behavior gains independent adapter authority.
// - Merge-When:
//   - MCP capability projection directly owns every shared lifecycle mapping.
// - Summary:
//   - Shares lifecycle meaning without turning discovery into execution.
// - Description:
//   - Keeps Inspect/context outside operation lifecycle while mapping six
//     operation capabilities onto the shared application contract.
// - Usage:
//   - Project lifecycle metadata only after a capability is actually admitted.
// - Defaults:
//   - Missing lifecycle mapping means no operation lifecycle is implied.
//

//! Frozen MCP capability projection onto shared application operation
//! lifecycle.

use atrament_application_operation_lifecycle::{
    ApplicationOperationClass, ApplicationOperationEffectBoundary,
    application_operation_effect_boundary,
};
use atrament_mcp_capability_effect::McpApplicationCapabilityClass;

/// Return the shared lifecycle operation for one generic MCP capability.
///
/// Inspect and Command context remain read-only discovery/context capabilities
/// outside the six-operation lifecycle contract, so they return `None`.
#[must_use]
pub const fn mcp_application_operation_class(
    capability: McpApplicationCapabilityClass,
) -> Option<ApplicationOperationClass> {
    match capability {
        McpApplicationCapabilityClass::Apply => {
            Some(ApplicationOperationClass::Apply)
        },
        McpApplicationCapabilityClass::CommandContext
        | McpApplicationCapabilityClass::Inspect => None,
        McpApplicationCapabilityClass::Export => {
            Some(ApplicationOperationClass::Export)
        },
        McpApplicationCapabilityClass::HistoryTraversal => {
            Some(ApplicationOperationClass::HistoryTraversal)
        },
        McpApplicationCapabilityClass::Plan => {
            Some(ApplicationOperationClass::Plan)
        },
        McpApplicationCapabilityClass::Render => {
            Some(ApplicationOperationClass::Render)
        },
        McpApplicationCapabilityClass::Validate => {
            Some(ApplicationOperationClass::Validate)
        },
    }
}

/// Return the authoritative lifecycle effect boundary for an MCP capability.
///
/// This only composes frozen structural vocabulary. It does not claim the
/// capability is implemented, admitted, running, cancellable, or complete.
#[must_use]
pub const fn mcp_application_operation_effect_boundary(
    capability: McpApplicationCapabilityClass,
) -> Option<ApplicationOperationEffectBoundary> {
    match mcp_application_operation_class(capability) {
        Some(operation) => {
            Some(application_operation_effect_boundary(operation))
        },
        None => None,
    }
}
