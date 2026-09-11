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
//   - Transport-neutral MCP effect-class authorization vocabulary and check.
// - Must-Not:
//   - Authenticate callers, bind sessions, expose tools, inspect live package
//     availability, execute capabilities, or waive capability-specific checks.
// - Allows:
//   - Inputs: One generic MCP capability and caller-supplied admitted effects.
//   - Outputs: Exact admitted/not-admitted effect-class membership evidence.
//   - Side effects: None.
// - Split-When:
//   - Concrete MCP session admission gains credential or transport authority.
// - Merge-When:
//   - One live MCP admission owner directly performs this exact effect check.
// - Summary:
//   - Keeps discovered capability vocabulary separate from invocation
//     authority.
// - Description:
//   - Authorizes only exact declared effect membership for a capability.
// - Usage:
//   - Apply after inbound session identity is established and before
//     invocation.
// - Defaults:
//   - Empty admitted-effect input authorizes no generic application capability.
//

//! Exact effect-class authorization for generic MCP application capabilities.

use atrament_mcp_capability_effect::{
    McpApplicationCapabilityClass, McpApplicationEffectClass,
    mcp_application_capability_effect,
};

/// Result of one read-only MCP effect-class authorization check.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum McpEffectAdmission {
    /// The capability's exact declared effect class is explicitly admitted.
    Admitted {
        /// Generic capability whose effect class was checked.
        capability: McpApplicationCapabilityClass,
        /// Exact declared effect class found in the admitted set.
        effect: McpApplicationEffectClass,
    },
    /// The capability's exact declared effect class is absent from admission.
    NotAdmitted {
        /// Generic capability whose effect class was checked.
        capability: McpApplicationCapabilityClass,
        /// Exact declared effect class absent from the admitted set.
        effect: McpApplicationEffectClass,
    },
}

/// Check exact effect-class membership for one generic MCP capability.
///
/// This proves only one authorization axis. It does not authenticate an MCP
/// caller, establish session binding, prove the capability is implemented, or
/// waive revision, scope, path, provenance, retry, output, or device checks.
#[must_use]
pub fn mcp_effect_admission(
    admitted_effects: &[McpApplicationEffectClass],
    capability: McpApplicationCapabilityClass,
) -> McpEffectAdmission {
    let effect = mcp_application_capability_effect(capability);
    if admitted_effects.contains(&effect) {
        McpEffectAdmission::Admitted { capability, effect }
    } else {
        McpEffectAdmission::NotAdmitted { capability, effect }
    }
}
