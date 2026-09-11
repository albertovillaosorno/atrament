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
//   - Transport-neutral MCP projection of frozen derived/output result meaning.
// - Must-Not:
//   - Expose MCP tools, execute Render/Plan/Export, touch files, or recover
//     retries,
//     emit diagnostics, admit adapters, or infer unknown transport outcomes.
// - Allows:
//   - Inputs: One generic MCP capability and one derived/output result class.
//   - Outputs: Its derived/output operation and effect disposition only when
//     both capability and result belong to the frozen projection.
//   - Side effects: None.
// - Split-When:
//   - Executable MCP derived/output result handling gains adapter authority.
// - Merge-When:
//   - MCP capability projection directly owns derived/output result semantics.
// - Summary:
//   - Prevents unrelated MCP capabilities from acquiring output-result meaning.
// - Description:
//   - Maps only Render, Plan, and Export and preserves result applicability.
// - Usage:
//   - Project machine-readable output semantics after capability execution.
// - Defaults:
//   - Missing mapping or incompatible result returns no effect disposition.
//

//! Frozen MCP projection onto shared derived/output result vocabulary.

use atrament_derived_output_result::{
    DerivedOutputEffectDisposition, DerivedOutputOperation,
    DerivedOutputResultClass, derived_output_effect_disposition_for_operation,
};
use atrament_mcp_capability_effect::McpApplicationCapabilityClass;

/// Return the derived/output operation owned by one generic MCP capability.
///
/// Only Render, Plan, and Export use the shared derived/output taxonomy.
#[must_use]
pub const fn mcp_derived_output_operation(
    capability: McpApplicationCapabilityClass,
) -> Option<DerivedOutputOperation> {
    match capability {
        McpApplicationCapabilityClass::Export => {
            Some(DerivedOutputOperation::Export)
        },
        McpApplicationCapabilityClass::Plan => {
            Some(DerivedOutputOperation::Plan)
        },
        McpApplicationCapabilityClass::Render => {
            Some(DerivedOutputOperation::Render)
        },
        McpApplicationCapabilityClass::Apply
        | McpApplicationCapabilityClass::CommandContext
        | McpApplicationCapabilityClass::HistoryTraversal
        | McpApplicationCapabilityClass::Inspect
        | McpApplicationCapabilityClass::Validate => None,
    }
}

/// Project a result effect only for a compatible MCP derived/output capability.
///
/// This composes frozen vocabulary only. It does not claim the capability is
/// implemented, admitted, executed, or represented by any particular MCP tool.
#[must_use]
pub const fn mcp_derived_output_effect_disposition(
    capability: McpApplicationCapabilityClass,
    result: DerivedOutputResultClass,
) -> Option<DerivedOutputEffectDisposition> {
    match mcp_derived_output_operation(capability) {
        Some(operation) => {
            derived_output_effect_disposition_for_operation(operation, result)
        },
        None => None,
    }
}
