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
//   - Exhaustive evidence for generic MCP effect-class authorization.
// - Must-Not:
//   - Authenticate MCP, bind sessions, expose tools, or execute capabilities.
// - Allows:
//   - Inputs: Every seven-effect admission mask crossed with eight
//     capabilities.
//   - Outputs: Exact 1,024-case membership assertions.
//   - Side effects: None.
// - Split-When:
//   - Stateful admission or adapter fixtures require independent evidence.
// - Merge-When:
//   - Live MCP admission tests fully subsume static effect authorization.
// - Summary:
//   - Pins exact effect membership without treating schemas as permission.
// - Description:
//   - Proves duplicate/order-free admitted sets cannot widen effect meaning.
// - Usage:
//   - Compare all finite masks against an independent declared-effect oracle.
// - Defaults:
//   - The empty mask rejects every capability.
//
use atrament_mcp_capability_effect::{
    McpApplicationCapabilityClass, McpApplicationEffectClass,
};
use atrament_mcp_effect_admission::{McpEffectAdmission, mcp_effect_admission};

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

const EFFECTS: [McpApplicationEffectClass; 7] = [
    McpApplicationEffectClass::AcceptedHistoryMutation,
    McpApplicationEffectClass::AcceptedRevisionMutation,
    McpApplicationEffectClass::DerivedComputation,
    McpApplicationEffectClass::DerivedDeviceNeutralComputation,
    McpApplicationEffectClass::ExplicitPersistentSideEffect,
    McpApplicationEffectClass::ReadOnly,
    McpApplicationEffectClass::ReadOnlyCandidateSimulation,
];

fn expected_effect(
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

#[test]
fn all_1024_effect_masks_and_capabilities_match_exact_membership() {
    let mut cases = 0_usize;
    for mask in 0_u8..128 {
        let admitted: Vec<_> = EFFECTS
            .into_iter()
            .enumerate()
            .filter_map(|(index, effect)| {
                (mask & (1_u8 << index) != 0).then_some(effect)
            })
            .collect();
        for capability in CAPABILITIES {
            let effect = expected_effect(capability);
            let expected = if admitted.contains(&effect) {
                McpEffectAdmission::Admitted { capability, effect }
            } else {
                McpEffectAdmission::NotAdmitted { capability, effect }
            };
            assert_eq!(
                mcp_effect_admission(&admitted, capability),
                expected,
                "mask={mask:#09b} capability={capability:?}",
            );
            cases += 1;
        }
    }
    assert_eq!(cases, 1_024);
}

#[test]
fn duplicate_effect_entries_do_not_change_authorization_meaning() {
    let admitted = [
        McpApplicationEffectClass::ReadOnly,
        McpApplicationEffectClass::ReadOnly,
    ];
    assert_eq!(
        mcp_effect_admission(
            &admitted,
            McpApplicationCapabilityClass::CommandContext,
        ),
        McpEffectAdmission::Admitted {
            capability: McpApplicationCapabilityClass::CommandContext,
            effect: McpApplicationEffectClass::ReadOnly,
        },
    );
    assert_eq!(
        mcp_effect_admission(&admitted, McpApplicationCapabilityClass::Apply),
        McpEffectAdmission::NotAdmitted {
            capability: McpApplicationCapabilityClass::Apply,
            effect: McpApplicationEffectClass::AcceptedRevisionMutation,
        },
    );
}
