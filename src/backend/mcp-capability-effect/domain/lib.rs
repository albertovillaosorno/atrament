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
//   - Frozen MCP application capability classes and declared effect classes.
// - Must-Not:
//   - Define MCP tool names, schemas, transport, credentials, receipts, retry,
//     application execution, filesystem access, or physical-device authority.
// - Allows:
//   - Inputs: One frozen generic MCP application capability class.
//   - Outputs: Its exact declared application effect class.
//   - Side effects: None.
// - Split-When:
//   - MCP adapter admission or concrete tool projection gains executable owner.
// - Merge-When:
//   - Shared application capability discovery directly owns effect vocabulary.
// - Summary:
//   - Freezes MCP effect meaning without creating MCP-specific domain behavior.
// - Description:
//   - Keeps read-only, mutation, derived, and persistent effects distinct.
// - Usage:
//   - Project effect metadata from live application capability discovery.
// - Defaults:
//   - No capability class implies authorization or implementation availability.
//

//! Frozen effect classes for generic MCP application capabilities.

/// Generic application capability class described by the frozen MCP contract.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum McpApplicationCapabilityClass {
    /// Atomic semantic command application.
    Apply,
    /// Backend-owned bounded command-context projection.
    CommandContext,
    /// Explicit caller-authorized persistent output.
    Export,
    /// Dedicated Undo or Redo accepted-history traversal.
    HistoryTraversal,
    /// Bounded accepted-state and capability inspection.
    Inspect,
    /// Device-neutral motion-plan compilation.
    Plan,
    /// Accepted-revision derived rendering.
    Render,
    /// Read-only semantic candidate validation.
    Validate,
}

/// Application effect class declared for one generic MCP capability class.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum McpApplicationEffectClass {
    /// Mutation of accepted application-history position.
    AcceptedHistoryMutation,
    /// Atomic accepted semantic revision mutation.
    AcceptedRevisionMutation,
    /// Read-only derived computation over accepted source.
    DerivedComputation,
    /// Read-only device-neutral derived computation.
    DerivedDeviceNeutralComputation,
    /// Explicit persistent side effect through an owning output boundary.
    ExplicitPersistentSideEffect,
    /// Read-only application inspection or context projection.
    ReadOnly,
    /// Read-only simulation of one semantic mutation candidate.
    ReadOnlyCandidateSimulation,
}

/// Return the exact frozen effect class for one generic MCP capability class.
#[must_use]
pub const fn mcp_application_capability_effect(
    capability: McpApplicationCapabilityClass,
) -> McpApplicationEffectClass {
    match capability {
        McpApplicationCapabilityClass::Apply => {
            McpApplicationEffectClass::AcceptedRevisionMutation
        },
        McpApplicationCapabilityClass::CommandContext
        | McpApplicationCapabilityClass::Inspect => {
            McpApplicationEffectClass::ReadOnly
        },
        McpApplicationCapabilityClass::Export => {
            McpApplicationEffectClass::ExplicitPersistentSideEffect
        },
        McpApplicationCapabilityClass::HistoryTraversal => {
            McpApplicationEffectClass::AcceptedHistoryMutation
        },
        McpApplicationCapabilityClass::Plan => {
            McpApplicationEffectClass::DerivedDeviceNeutralComputation
        },
        McpApplicationCapabilityClass::Render => {
            McpApplicationEffectClass::DerivedComputation
        },
        McpApplicationCapabilityClass::Validate => {
            McpApplicationEffectClass::ReadOnlyCandidateSimulation
        },
    }
}

/// Report whether a generic MCP application capability grants physical-device
/// authority.
///
/// The frozen local MCP contract keeps connect, home, arm, start, pause,
/// resume, cancel, and safe-stop behind a separate future physical capability.
/// Therefore every generic application capability returns `false` here.
#[must_use]
pub const fn mcp_application_capability_grants_physical_authority(
    _capability: McpApplicationCapabilityClass,
) -> bool {
    false
}
