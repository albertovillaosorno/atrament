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
//   - Regression evidence for frozen generic MCP capability effect classes.
// - Must-Not:
//   - Implement MCP tools, schemas, transport, admission, retry, or execution.
// - Allows:
//   - Inputs: Every frozen generic MCP application capability class.
//   - Outputs: Exact effect-class assertions for all capability classes.
//   - Side effects: None.
// - Split-When:
//   - Concrete MCP adapter tests gain executable authority.
// - Merge-When:
//   - Capability-discovery parity tests fully subsume this structural mapping.
// - Summary:
//   - Proves MCP capability classes preserve application effect boundaries.
// - Description:
//   - Prevents read-only or derived capabilities from being labeled mutations.
// - Usage:
//   - Exhaustively compare the frozen capability/effect matrix.
// - Defaults:
//   - The matrix does not imply that an MCP capability is implemented.
//
use atrament_mcp_capability_effect::{
    McpApplicationCapabilityClass, McpApplicationEffectClass,
    mcp_application_capability_effect,
};

#[test]
fn all_eight_capability_effect_classes_match_frozen_contract() {
    let cases = [
        (
            McpApplicationCapabilityClass::Apply,
            McpApplicationEffectClass::AcceptedRevisionMutation,
        ),
        (
            McpApplicationCapabilityClass::CommandContext,
            McpApplicationEffectClass::ReadOnly,
        ),
        (
            McpApplicationCapabilityClass::Export,
            McpApplicationEffectClass::ExplicitPersistentSideEffect,
        ),
        (
            McpApplicationCapabilityClass::HistoryTraversal,
            McpApplicationEffectClass::AcceptedHistoryMutation,
        ),
        (
            McpApplicationCapabilityClass::Inspect,
            McpApplicationEffectClass::ReadOnly,
        ),
        (
            McpApplicationCapabilityClass::Plan,
            McpApplicationEffectClass::DerivedDeviceNeutralComputation,
        ),
        (
            McpApplicationCapabilityClass::Render,
            McpApplicationEffectClass::DerivedComputation,
        ),
        (
            McpApplicationCapabilityClass::Validate,
            McpApplicationEffectClass::ReadOnlyCandidateSimulation,
        ),
    ];
    assert_eq!(cases.len(), 8);
    for (capability, expected) in cases {
        assert_eq!(mcp_application_capability_effect(capability), expected);
    }
}
