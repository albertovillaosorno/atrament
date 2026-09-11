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
//   - Read-only MCP projection of semantic-command capability admission.
// - Must-Not:
//   - Admit MCP sessions, expose tools, create contexts, or validate/apply
//     batches,
//     normalize protocol data, mutate notebooks, or infer hidden capability.
// - Allows:
//   - Inputs: One generic MCP capability and one backend-owned command
//     snapshot.
//   - Outputs: The matching command application capability and exact snapshot
//     membership result when that mapping exists.
//   - Side effects: None.
// - Split-When:
//   - Executable MCP command admission gains independent adapter authority.
// - Merge-When:
//   - MCP capability discovery directly owns semantic-command admission.
// - Summary:
//   - Keeps MCP vocabulary subordinate to live backend capability discovery.
// - Description:
//   - Maps only CommandContext, Validate, and Apply to snapshot-owned
//     admission.
// - Usage:
//   - Check live command capability before projecting an MCP command operation.
// - Defaults:
//   - Unmapped MCP capabilities imply no semantic-command application request.
//

//! Read-only MCP projection onto semantic-command capability admission.

use atrament_mcp_capability_effect::{
    McpApplicationCapabilityClass, McpApplicationEffectClass,
};
use atrament_mcp_effect_admission::{McpEffectAdmission, mcp_effect_admission};
use atrament_semantic_command_context::semantic_command_application_admission;
use atrament_semantic_notebook_port::{
    CommandApplicationCapability, SemanticCommandApplicationAdmission,
    SemanticCommandCapabilitySnapshot,
};

/// Independent MCP authorization and semantic-command snapshot facts.
///
/// Keeping both facts prevents effect authorization from fabricating live
/// command capability, and prevents snapshot discovery from bypassing session
/// effect restrictions.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct McpSemanticCommandAdmissionFacts {
    /// Exact command application membership in the backend-owned snapshot.
    pub application: SemanticCommandApplicationAdmission,
    /// Exact generic MCP effect-class authorization for the same capability.
    pub effect: McpEffectAdmission,
}

/// Return the semantic-command application capability for one MCP capability.
#[must_use]
pub const fn mcp_semantic_command_application_capability(
    capability: McpApplicationCapabilityClass,
) -> Option<CommandApplicationCapability> {
    match capability {
        McpApplicationCapabilityClass::Apply => {
            Some(CommandApplicationCapability::Apply)
        },
        McpApplicationCapabilityClass::CommandContext => {
            Some(CommandApplicationCapability::CommandContext)
        },
        McpApplicationCapabilityClass::Validate => {
            Some(CommandApplicationCapability::Validate)
        },
        McpApplicationCapabilityClass::Export
        | McpApplicationCapabilityClass::HistoryTraversal
        | McpApplicationCapabilityClass::Inspect
        | McpApplicationCapabilityClass::Plan
        | McpApplicationCapabilityClass::Render => None,
    }
}

/// Check one mapped MCP command capability against exact snapshot membership.
///
/// Returning `Admitted` reflects only the supplied backend-owned snapshot. It
/// does not admit an MCP connection, create a command context, or execute work.
#[must_use]
pub fn mcp_semantic_command_application_admission(
    snapshot: &SemanticCommandCapabilitySnapshot,
    capability: McpApplicationCapabilityClass,
) -> Option<SemanticCommandApplicationAdmission> {
    mcp_semantic_command_application_capability(capability).map(|requested| {
        semantic_command_application_admission(snapshot, requested)
    })
}

/// Compose independent effect and command-snapshot admission facts.
///
/// Only `CommandContext`, Validate, and Apply map into semantic-command
/// application capability. This helper does not choose rejection precedence,
/// authenticate a caller, create context, normalize protocol data, or execute.
#[must_use]
pub fn mcp_semantic_command_admission_facts(
    admitted_effects: &[McpApplicationEffectClass],
    snapshot: &SemanticCommandCapabilitySnapshot,
    capability: McpApplicationCapabilityClass,
) -> Option<McpSemanticCommandAdmissionFacts> {
    let application = mcp_semantic_command_application_admission(
        snapshot,
        capability,
    )?;
    Some(McpSemanticCommandAdmissionFacts {
        application,
        effect: mcp_effect_admission(admitted_effects, capability),
    })
}
