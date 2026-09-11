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
//   - Exhaustive evidence for MCP semantic-command snapshot admission.
// - Must-Not:
//   - Start MCP, expose tools, generate contexts, validate/apply, or mutate.
// - Allows:
//   - Inputs: Every four-bit command-capability mask and eight MCP classes.
//   - Outputs: Exact mapping and snapshot-membership admission assertions.
//   - Side effects: None.
// - Split-When:
//   - Executable MCP capability discovery requires adapter parity fixtures.
// - Merge-When:
//   - Live MCP discovery tests fully subsume this structural projection.
// - Summary:
//   - Covers all 128 snapshot-mask by MCP-capability combinations.
// - Description:
//   - Proves vocabulary never bypasses backend-owned capability membership.
// - Usage:
//   - Compare every mapped capability with an independent membership oracle.
// - Defaults:
//   - Unmapped MCP classes have no semantic-command admission result.
//
use atrament_mcp_capability_effect::{
    McpApplicationCapabilityClass, McpApplicationEffectClass,
};
use atrament_mcp_effect_admission::McpEffectAdmission;
use atrament_mcp_semantic_command_admission_projection::{
    McpSemanticCommandAdmissionFacts, mcp_semantic_command_admission_facts,
    mcp_semantic_command_application_admission,
    mcp_semantic_command_application_capability,
};
use atrament_semantic_notebook_port::{
    CommandApplicationCapability, CommandBehaviorVersion,
    CommandResourceLimits, SemanticCommandApplicationAdmission,
    SemanticCommandCapabilitySnapshot,
};

const MCP_CAPABILITIES: [McpApplicationCapabilityClass; 8] = [
    McpApplicationCapabilityClass::Apply,
    McpApplicationCapabilityClass::CommandContext,
    McpApplicationCapabilityClass::Export,
    McpApplicationCapabilityClass::HistoryTraversal,
    McpApplicationCapabilityClass::Inspect,
    McpApplicationCapabilityClass::Plan,
    McpApplicationCapabilityClass::Render,
    McpApplicationCapabilityClass::Validate,
];

const COMMAND_CAPABILITIES: [CommandApplicationCapability; 4] = [
    CommandApplicationCapability::Apply,
    CommandApplicationCapability::CommandContext,
    CommandApplicationCapability::SelectiveRebatching,
    CommandApplicationCapability::Validate,
];

const MCP_EFFECTS: [McpApplicationEffectClass; 7] = [
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

fn expected_mapping(
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
        _ => None,
    }
}

fn snapshot(
    admitted_applications: &'static [CommandApplicationCapability],
) -> SemanticCommandCapabilitySnapshot {
    SemanticCommandCapabilitySnapshot {
        admitted_applications,
        behavior_version: CommandBehaviorVersion(94),
        family_capabilities: &[],
        normalization_version: None,
        protocol_versions: &[],
        resource_limits: &CommandResourceLimits {
            commands_per_batch: None,
            dependency_edges: None,
            envelope_bytes: None,
            readable_context_bytes: None,
            writable_targets: None,
        },
        typed_result_version: CommandBehaviorVersion(94),
    }
}

#[test]
fn all_eight_mcp_capabilities_have_exact_command_application_mapping() {
    for capability in MCP_CAPABILITIES {
        assert_eq!(
            mcp_semantic_command_application_capability(capability),
            expected_mapping(capability),
        );
    }
}

#[test]
fn all_128_snapshot_masks_and_mcp_capabilities_match_membership_oracle() {
    let mut cases = 0_usize;
    for mask in 0_u8..16 {
        let admitted: Vec<_> = COMMAND_CAPABILITIES
            .into_iter()
            .enumerate()
            .filter_map(|(index, capability)| {
                (mask & (1_u8 << index) != 0).then_some(capability)
            })
            .collect();
        let admitted: &'static [CommandApplicationCapability] =
            Box::leak(admitted.into_boxed_slice());
        let snapshot = snapshot(admitted);
        for capability in MCP_CAPABILITIES {
            let expected = expected_mapping(capability).map(|requested| {
                if admitted.contains(&requested) {
                    SemanticCommandApplicationAdmission::Admitted {
                        capability: requested,
                    }
                } else {
                    SemanticCommandApplicationAdmission::Unsupported {
                        requested,
                    }
                }
            });
            assert_eq!(
                mcp_semantic_command_application_admission(
                    &snapshot,
                    capability,
                ),
                expected,
                "mask={mask:#06b} capability={capability:?}",
            );
            cases += 1;
        }
    }
    assert_eq!(cases, 128);
}

#[test]
fn all_16384_effect_snapshot_and_capability_states_preserve_both_facts() {
    let mut cases = 0_usize;
    for effect_mask in 0_u8..128 {
        let admitted_effects: Vec<_> = MCP_EFFECTS
            .into_iter()
            .enumerate()
            .filter_map(|(index, effect)| {
                (effect_mask & (1_u8 << index) != 0).then_some(effect)
            })
            .collect();
        for application_mask in 0_u8..16 {
            let applications: Vec<_> = COMMAND_CAPABILITIES
                .into_iter()
                .enumerate()
                .filter_map(|(index, application)| {
                    (application_mask & (1_u8 << index) != 0)
                        .then_some(application)
                })
                .collect();
            let applications: &'static [CommandApplicationCapability] =
                Box::leak(applications.into_boxed_slice());
            let snapshot = snapshot(applications);
            for capability in MCP_CAPABILITIES {
                let expected = expected_mapping(capability).map(|requested| {
                    let application = if applications.contains(&requested) {
                        SemanticCommandApplicationAdmission::Admitted {
                            capability: requested,
                        }
                    } else {
                        SemanticCommandApplicationAdmission::Unsupported {
                            requested,
                        }
                    };
                    let effect = expected_effect(capability);
                    let effect = if admitted_effects.contains(&effect) {
                        McpEffectAdmission::Admitted { capability, effect }
                    } else {
                        McpEffectAdmission::NotAdmitted { capability, effect }
                    };
                    McpSemanticCommandAdmissionFacts {
                        application,
                        effect,
                    }
                });
                assert_eq!(
                    mcp_semantic_command_admission_facts(
                        &admitted_effects,
                        &snapshot,
                        capability,
                    ),
                    expected,
                    "effect={effect_mask:#09b} app={application_mask:#06b} \
                     capability={capability:?}",
                );
                cases += 1;
            }
        }
    }
    assert_eq!(cases, 16_384);
}
