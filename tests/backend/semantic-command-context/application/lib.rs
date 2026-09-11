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
//   - Regression evidence for bounded semantic command-context scope.
// - Must-Not:
//   - Parse wire envelopes, compute context identity, choose context content,
//     mutate notebooks, normalize batches, or implement retry behavior.
// - Allows:
//   - Inputs: Deterministic backend-owned context and scope-use fixtures.
//   - Outputs: Assertions over retained context authority and scope admission.
//   - Side effects: Process-local identity allocation only.
// - Split-When:
//   - Context construction or serialized envelope behavior gains fixtures.
// - Merge-When:
//   - Command-context scope becomes inseparable from full protocol admission.
// - Summary:
//   - Proves readable context cannot silently widen semantic write scope.
// - Description:
//   - Covers capability, target, anchor, family, and context payload behavior.
// - Usage:
//   - Compile against the command-context application and inbound port.
// - Defaults:
//   - Missing capability, family, or location authority stays unadmitted.
//
use atrament_semantic_command_context::{
    semantic_command_application_admission,
    semantic_command_capability_behavior_admission,
    semantic_command_context_binding_admission,
    semantic_command_envelope_admission,
    semantic_command_envelope_context_admission,
    semantic_command_family_behavior_admission,
    semantic_command_protocol_admission, semantic_command_resource_admission,
    semantic_command_scope_admission,
};
use atrament_semantic_notebook::IdentityAllocator;
use atrament_semantic_notebook_port::{
    CommandApplicationCapability, CommandBehaviorVersion,
    CommandCapabilityCompatibilityOutcome, CommandFamilyCapability,
    CommandResourceLimits, CommandTargetPreconditions,
    DirectEditBatchCommand, EditableSemanticValue, IdentityOwnerExpectation,
    IdentityPrecondition, SemanticCommandApplicationAdmission,
    SemanticCommandBatchEnvelope, SemanticCommandCapabilitySnapshot,
    SemanticCommandContext,
    SemanticCommandContextBinding, SemanticCommandContextBindingAdmission,
    SemanticCommandContextMatch, SemanticCommandEnvelopeAdmission,
    SemanticCommandEnvelopeCommandAdmission,
    SemanticCommandEnvelopeContextAdmission, SemanticCommandFamily,
    SemanticCommandFamilyBehaviorAdmission, SemanticCommandProtocolAdmission,
    SemanticCommandResourceAdmission, SemanticCommandResourceLimitAdmission,
    SemanticCommandScopeAdmission, SemanticCommandScopeLocation,
};

const fn expected_match(matches: bool) -> SemanticCommandContextMatch {
    if matches {
        SemanticCommandContextMatch::Matched
    } else {
        SemanticCommandContextMatch::Mismatched
    }
}

fn unbounded_limits() -> CommandResourceLimits {
    CommandResourceLimits {
        commands_per_batch: None,
        dependency_edges: None,
        envelope_bytes: None,
        readable_context_bytes: None,
        writable_targets: None,
    }
}

#[test]
fn command_context_retains_exact_backend_selected_authority() {
    let identities = IdentityAllocator::new();
    let notebook = identities.allocate_accepted().expect("notebook identity");
    let writable = identities.allocate_accepted().expect("writable identity");
    let constraint = identities
        .allocate_accepted()
        .expect("constraint identity");
    let readable = identities.allocate_accepted().expect("readable identity");
    let base = identities.allocate_revision().expect("base revision");
    let context = SemanticCommandContext {
        admitted_families: vec![SemanticCommandFamily::TextContent],
        base,
        behavior_version: CommandBehaviorVersion(94),
        context_identity: String::from("context-owned-17"),
        insertion_anchors: vec![String::from("after-body")],
        local_preconditions: vec![String::from("body-text-v3")],
        notebook,
        readable_context: vec![(readable, String::from("neighbor evidence"))],
        relevant_constraints: vec![constraint],
        requested_intent: String::from("replace body phrase"),
        resource_limits: CommandResourceLimits {
            commands_per_batch: Some(4),
            dependency_edges: Some(3),
            envelope_bytes: Some(2048),
            readable_context_bytes: Some(4096),
            writable_targets: Some(2),
        },
        writable_targets: vec![writable],
    };
    assert_eq!(context.context_identity, "context-owned-17");
    assert_eq!(context.base, base);
    assert_eq!(context.notebook, notebook);
    assert_eq!(context.readable_context[0].0, readable);
    assert_eq!(context.relevant_constraints, [constraint]);
    assert_eq!(context.requested_intent, "replace body phrase");
    assert_eq!(context.local_preconditions, ["body-text-v3"]);
}

#[test]
fn readable_only_identity_never_becomes_writable_scope() {
    let identities = IdentityAllocator::new();
    let notebook = identities.allocate_accepted().expect("notebook identity");
    let writable = identities.allocate_accepted().expect("writable identity");
    let readable = identities.allocate_accepted().expect("readable identity");
    let context = SemanticCommandContext {
        admitted_families: vec![SemanticCommandFamily::TextContent],
        base: identities.allocate_revision().expect("base revision"),
        behavior_version: CommandBehaviorVersion(94),
        context_identity: "context-1",
        insertion_anchors: Vec::<u8>::new(),
        local_preconditions: Vec::<u8>::new(),
        notebook,
        readable_context: vec![readable],
        relevant_constraints: Vec::new(),
        requested_intent: "edit writable text",
        resource_limits: unbounded_limits(),
        writable_targets: vec![writable],
    };
    assert_eq!(
        semantic_command_scope_admission(
            &context,
            SemanticCommandFamily::TextContent,
            &SemanticCommandScopeLocation::Existing(readable),
        ),
        SemanticCommandScopeAdmission {
            family_admitted: true,
            location_admitted: false,
        },
    );
}

#[test]
fn family_and_location_admission_are_independent_for_targets_and_anchors() {
    let identities = IdentityAllocator::new();
    let notebook = identities.allocate_accepted().expect("notebook identity");
    let writable = identities.allocate_accepted().expect("writable identity");
    let other = identities.allocate_accepted().expect("other identity");
    let anchor = String::from("after-body");
    let other_anchor = String::from("after-title");
    let context = SemanticCommandContext {
        admitted_families: vec![SemanticCommandFamily::TextContent],
        base: identities.allocate_revision().expect("base revision"),
        behavior_version: CommandBehaviorVersion(94),
        context_identity: 17_u8,
        insertion_anchors: vec![anchor.clone()],
        local_preconditions: Vec::<u8>::new(),
        notebook,
        readable_context: (),
        relevant_constraints: Vec::new(),
        requested_intent: (),
        resource_limits: unbounded_limits(),
        writable_targets: vec![writable],
    };
    let cases = [
        (
            SemanticCommandFamily::TextContent,
            SemanticCommandScopeLocation::Existing(writable),
            true,
            true,
        ),
        (
            SemanticCommandFamily::Provenance,
            SemanticCommandScopeLocation::Existing(writable),
            false,
            true,
        ),
        (
            SemanticCommandFamily::TextContent,
            SemanticCommandScopeLocation::Existing(other),
            true,
            false,
        ),
        (
            SemanticCommandFamily::Provenance,
            SemanticCommandScopeLocation::Existing(other),
            false,
            false,
        ),
    ];
    for (family, location, family_admitted, location_admitted) in cases {
        let admission = semantic_command_scope_admission(
            &context,
            family,
            &location,
        );
        assert_eq!(
            admission,
            SemanticCommandScopeAdmission {
                family_admitted,
                location_admitted,
            },
        );
        assert_eq!(
            family_admitted && location_admitted,
            admission.family_admitted && admission.location_admitted,
        );
    }
    let anchor_cases = [
        (
            SemanticCommandFamily::TextContent,
            &anchor,
            true,
            true,
        ),
        (
            SemanticCommandFamily::Provenance,
            &anchor,
            false,
            true,
        ),
        (
            SemanticCommandFamily::TextContent,
            &other_anchor,
            true,
            false,
        ),
        (
            SemanticCommandFamily::Provenance,
            &other_anchor,
            false,
            false,
        ),
    ];
    for (family, location, family_admitted, location_admitted) in anchor_cases {
        assert_eq!(
            semantic_command_scope_admission(
                &context,
                family,
                &SemanticCommandScopeLocation::Insertion(location),
            ),
            SemanticCommandScopeAdmission {
                family_admitted,
                location_admitted,
            },
        );
    }
}

#[test]
fn all_16_context_binding_match_states_remain_independent() {
    let identities = IdentityAllocator::new();
    let notebook = identities.allocate_accepted().expect("notebook identity");
    let other_notebook = identities
        .allocate_accepted()
        .expect("other notebook identity");
    let base = identities.allocate_revision().expect("base revision");
    let other_base = identities
        .allocate_revision()
        .expect("other base revision");
    let context = SemanticCommandContext {
        admitted_families: Vec::new(),
        base,
        behavior_version: CommandBehaviorVersion(94),
        context_identity: String::from("context-current"),
        insertion_anchors: Vec::<u8>::new(),
        local_preconditions: Vec::<u8>::new(),
        notebook,
        readable_context: (),
        relevant_constraints: Vec::new(),
        requested_intent: (),
        resource_limits: unbounded_limits(),
        writable_targets: Vec::new(),
    };
    let mut cases = 0_u8;
    for mask in 0_u8..16 {
        let base_matches = mask & 0b0001 != 0;
        let behavior_matches = mask & 0b0010 != 0;
        let context_matches = mask & 0b0100 != 0;
        let notebook_matches = mask & 0b1000 != 0;
        let binding = SemanticCommandContextBinding {
            base: if base_matches { base } else { other_base },
            behavior_version: if behavior_matches {
                CommandBehaviorVersion(94)
            } else {
                CommandBehaviorVersion(93)
            },
            context_identity: if context_matches {
                String::from("context-current")
            } else {
                String::from("context-other")
            },
            notebook: if notebook_matches {
                notebook
            } else {
                other_notebook
            },
        };
        assert_eq!(
            semantic_command_context_binding_admission(&context, &binding),
            SemanticCommandContextBindingAdmission {
                base: expected_match(base_matches),
                behavior: expected_match(behavior_matches),
                context: expected_match(context_matches),
                notebook: expected_match(notebook_matches),
            },
            "binding match mask {mask:#06b}",
        );
        cases = cases.saturating_add(1);
    }
    assert_eq!(cases, 16);
}

fn command(
    id: u8,
    family: SemanticCommandFamily,
    target: atrament_semantic_notebook::AcceptedIdentity,
) -> DirectEditBatchCommand<u8> {
    let requested = match family {
        SemanticCommandFamily::StyleRole => {
            EditableSemanticValue::StyleReference(None)
        },
        _ => EditableSemanticValue::Text(format!("requested-{id}")),
    };
    DirectEditBatchCommand {
        dependencies: Vec::new(),
        id,
        preconditions: CommandTargetPreconditions {
            expected_value: None,
            identity: IdentityPrecondition {
                expected_kind: None,
                expected_owner: IdentityOwnerExpectation::Any,
            },
            requested_family: family,
        },
        requested,
        target,
    }
}

#[test]
fn parsed_envelope_retains_protocol_retry_binding_and_command_order() {
    let identities = IdentityAllocator::new();
    let notebook = identities.allocate_accepted().expect("notebook identity");
    let target = identities.allocate_accepted().expect("target identity");
    let base = identities.allocate_revision().expect("base revision");
    let envelope = SemanticCommandBatchEnvelope {
        binding: SemanticCommandContextBinding {
            base,
            behavior_version: CommandBehaviorVersion(94),
            context_identity: String::from("context-17"),
            notebook,
        },
        commands: vec![
            command(3, SemanticCommandFamily::TextContent, target),
            command(1, SemanticCommandFamily::StyleRole, target),
        ],
        protocol_version: CommandBehaviorVersion(7),
        retry_identity: String::from("retry-owned-9"),
    };
    assert_eq!(envelope.protocol_version, CommandBehaviorVersion(7));
    assert_eq!(envelope.retry_identity, "retry-owned-9");
    assert_eq!(envelope.commands[0].id, 3);
    assert_eq!(envelope.commands[1].id, 1);
    assert_eq!(envelope.binding.context_identity, "context-17");
}

#[test]
fn envelope_context_review_preserves_order_and_all_scope_fact_combinations() {
    let identities = IdentityAllocator::new();
    let notebook = identities.allocate_accepted().expect("notebook identity");
    let writable = identities.allocate_accepted().expect("writable identity");
    let other = identities.allocate_accepted().expect("other identity");
    let base = identities.allocate_revision().expect("base revision");
    let context = SemanticCommandContext {
        admitted_families: vec![SemanticCommandFamily::TextContent],
        base,
        behavior_version: CommandBehaviorVersion(94),
        context_identity: String::from("context-current"),
        insertion_anchors: Vec::<String>::new(),
        local_preconditions: Vec::<String>::new(),
        notebook,
        readable_context: vec![other],
        relevant_constraints: Vec::new(),
        requested_intent: String::from("bounded edit"),
        resource_limits: unbounded_limits(),
        writable_targets: vec![writable],
    };
    let envelope = SemanticCommandBatchEnvelope {
        binding: SemanticCommandContextBinding {
            base,
            behavior_version: CommandBehaviorVersion(94),
            context_identity: String::from("context-current"),
            notebook,
        },
        commands: vec![
            command(9, SemanticCommandFamily::TextContent, writable),
            command(7, SemanticCommandFamily::StyleRole, writable),
            command(5, SemanticCommandFamily::TextContent, other),
            command(3, SemanticCommandFamily::StyleRole, other),
        ],
        protocol_version: CommandBehaviorVersion(7),
        retry_identity: String::from("retry-1"),
    };
    let admission = semantic_command_envelope_context_admission(
        &context,
        &envelope,
    );
    assert_eq!(
        admission,
        SemanticCommandEnvelopeContextAdmission {
            binding: SemanticCommandContextBindingAdmission {
                base: SemanticCommandContextMatch::Matched,
                behavior: SemanticCommandContextMatch::Matched,
                context: SemanticCommandContextMatch::Matched,
                notebook: SemanticCommandContextMatch::Matched,
            },
            commands: vec![
                SemanticCommandEnvelopeCommandAdmission {
                    command: &9,
                    family_admitted: true,
                    location_admitted: true,
                },
                SemanticCommandEnvelopeCommandAdmission {
                    command: &7,
                    family_admitted: false,
                    location_admitted: true,
                },
                SemanticCommandEnvelopeCommandAdmission {
                    command: &5,
                    family_admitted: true,
                    location_admitted: false,
                },
                SemanticCommandEnvelopeCommandAdmission {
                    command: &3,
                    family_admitted: false,
                    location_admitted: false,
                },
            ],
        },
    );
}

const fn expected_limit(
    actual: usize,
    limit: Option<usize>,
) -> SemanticCommandResourceLimitAdmission {
    match limit {
        None => SemanticCommandResourceLimitAdmission::Unspecified,
        Some(limit) if actual > limit => {
            SemanticCommandResourceLimitAdmission::Exceeded { actual, limit }
        },
        Some(limit) => {
            SemanticCommandResourceLimitAdmission::Within { actual, limit }
        },
    }
}

#[test]
fn all_27_count_resource_limit_states_are_independent() {
    let identities = IdentityAllocator::new();
    let notebook = identities.allocate_accepted().expect("notebook identity");
    let writable = identities.allocate_accepted().expect("writable identity");
    let base = identities.allocate_revision().expect("base revision");
    let mut cases = 0_u8;
    for command_limit in [None, Some(2), Some(1)] {
        for edge_limit in [None, Some(1), Some(0)] {
            for writable_limit in [None, Some(2), Some(1)] {
                let context = SemanticCommandContext {
                    admitted_families: vec![SemanticCommandFamily::TextContent],
                    base,
                    behavior_version: CommandBehaviorVersion(94),
                    context_identity: String::from("context-resource"),
                    insertion_anchors: vec![String::from("after-body")],
                    local_preconditions: Vec::<String>::new(),
                    notebook,
                    readable_context: (),
                    relevant_constraints: Vec::new(),
                    requested_intent: (),
                    resource_limits: CommandResourceLimits {
                        commands_per_batch: command_limit,
                        dependency_edges: edge_limit,
                        envelope_bytes: None,
                        readable_context_bytes: None,
                        writable_targets: writable_limit,
                    },
                    writable_targets: vec![writable],
                };
                let mut second = command(
                    2,
                    SemanticCommandFamily::TextContent,
                    writable,
                );
                second.dependencies = vec![1];
                let envelope = SemanticCommandBatchEnvelope {
                    binding: SemanticCommandContextBinding {
                        base,
                        behavior_version: CommandBehaviorVersion(94),
                        context_identity: String::from("context-resource"),
                        notebook,
                    },
                    commands: vec![
                        command(
                            1,
                            SemanticCommandFamily::TextContent,
                            writable,
                        ),
                        second,
                    ],
                    protocol_version: CommandBehaviorVersion(7),
                    retry_identity: String::from("retry-resource"),
                };
                assert_eq!(
                    semantic_command_resource_admission(&context, &envelope),
                    SemanticCommandResourceAdmission {
                        commands_per_batch: expected_limit(2, command_limit),
                        dependency_edges: expected_limit(1, edge_limit),
                        writable_targets: expected_limit(2, writable_limit),
                    },
                );
                cases = cases.saturating_add(1);
            }
        }
    }
    assert_eq!(cases, 27);
}

static TEST_PROTOCOL_LIMITS: CommandResourceLimits = CommandResourceLimits {
    commands_per_batch: None,
    dependency_edges: None,
    envelope_bytes: None,
    readable_context_bytes: None,
    writable_targets: None,
};

fn protocol_snapshot(
    versions: &'static [CommandBehaviorVersion],
) -> SemanticCommandCapabilitySnapshot {
    SemanticCommandCapabilitySnapshot {
        admitted_applications: &[] as &[CommandApplicationCapability],
        behavior_version: CommandBehaviorVersion(94),
        family_capabilities: &[] as &[CommandFamilyCapability],
        normalization_version: None,
        protocol_versions: versions,
        resource_limits: &TEST_PROTOCOL_LIMITS,
        typed_result_version: CommandBehaviorVersion(94),
    }
}

#[test]
fn protocol_admission_is_exact_membership_without_downgrade_guessing() {
    static VERSIONS: [CommandBehaviorVersion; 2] = [
        CommandBehaviorVersion(7),
        CommandBehaviorVersion(9),
    ];
    let snapshot = protocol_snapshot(&VERSIONS);
    assert_eq!(
        semantic_command_protocol_admission(
            &snapshot,
            CommandBehaviorVersion(7),
        ),
        SemanticCommandProtocolAdmission::Admitted {
            version: CommandBehaviorVersion(7),
        },
    );
    for requested in [6_u32, 8, 10] {
        assert_eq!(
            semantic_command_protocol_admission(
                &snapshot,
                CommandBehaviorVersion(requested),
            ),
            SemanticCommandProtocolAdmission::Unsupported {
                requested: CommandBehaviorVersion(requested),
            },
        );
    }
}

#[test]
fn empty_protocol_snapshot_rejects_every_requested_version() {
    let snapshot = protocol_snapshot(&[]);
    for requested in [0_u32, 1, 7, u32::MAX] {
        assert_eq!(
            semantic_command_protocol_admission(
                &snapshot,
                CommandBehaviorVersion(requested),
            ),
            SemanticCommandProtocolAdmission::Unsupported {
                requested: CommandBehaviorVersion(requested),
            },
        );
    }
}

#[test]
fn application_capability_admission_is_exact_snapshot_membership() {
    static APPLICATIONS: [CommandApplicationCapability; 2] = [
        CommandApplicationCapability::CommandContext,
        CommandApplicationCapability::Validate,
    ];
    let mut snapshot = protocol_snapshot(&[]);
    snapshot.admitted_applications = &APPLICATIONS;
    for capability in APPLICATIONS {
        assert_eq!(
            semantic_command_application_admission(&snapshot, capability),
            SemanticCommandApplicationAdmission::Admitted { capability },
        );
    }
    for requested in [
        CommandApplicationCapability::Apply,
        CommandApplicationCapability::SelectiveRebatching,
    ] {
        assert_eq!(
            semantic_command_application_admission(&snapshot, requested),
            SemanticCommandApplicationAdmission::Unsupported { requested },
        );
    }
}

#[test]
fn empty_application_snapshot_rejects_every_operation() {
    let snapshot = protocol_snapshot(&[]);
    for requested in [
        CommandApplicationCapability::Apply,
        CommandApplicationCapability::CommandContext,
        CommandApplicationCapability::SelectiveRebatching,
        CommandApplicationCapability::Validate,
    ] {
        assert_eq!(
            semantic_command_application_admission(&snapshot, requested),
            SemanticCommandApplicationAdmission::Unsupported { requested },
        );
    }
}

#[test]
fn family_behavior_admission_distinguishes_exact_drift_and_absence() {
    static FAMILIES: [CommandFamilyCapability; 2] = [
        CommandFamilyCapability {
            behavior_version: CommandBehaviorVersion(1),
            family: SemanticCommandFamily::TextContent,
        },
        CommandFamilyCapability {
            behavior_version: CommandBehaviorVersion(2),
            family: SemanticCommandFamily::StyleRole,
        },
    ];
    let mut snapshot = protocol_snapshot(&[]);
    snapshot.family_capabilities = &FAMILIES;
    assert_eq!(
        semantic_command_family_behavior_admission(
            &snapshot,
            SemanticCommandFamily::TextContent,
            CommandBehaviorVersion(1),
        ),
        SemanticCommandFamilyBehaviorAdmission::Admitted {
            behavior_version: CommandBehaviorVersion(1),
            family: SemanticCommandFamily::TextContent,
        },
    );
    assert_eq!(
        semantic_command_family_behavior_admission(
            &snapshot,
            SemanticCommandFamily::StyleRole,
            CommandBehaviorVersion(1),
        ),
        SemanticCommandFamilyBehaviorAdmission::BehaviorMismatch {
            current: CommandBehaviorVersion(2),
            expected: CommandBehaviorVersion(1),
            family: SemanticCommandFamily::StyleRole,
        },
    );
    assert_eq!(
        semantic_command_family_behavior_admission(
            &snapshot,
            SemanticCommandFamily::Provenance,
            CommandBehaviorVersion(2),
        ),
        SemanticCommandFamilyBehaviorAdmission::UnsupportedFamily {
            requested: SemanticCommandFamily::Provenance,
        },
    );
}

#[test]
fn envelope_preflight_composes_all_current_admission_facts() {
    static APPLICATIONS: [CommandApplicationCapability; 1] = [
        CommandApplicationCapability::Validate,
    ];
    static PROTOCOLS: [CommandBehaviorVersion; 1] = [CommandBehaviorVersion(7)];
    let identities = IdentityAllocator::new();
    let notebook = identities.allocate_accepted().expect("notebook identity");
    let target = identities.allocate_accepted().expect("target identity");
    let base = identities.allocate_revision().expect("base revision");
    let mut snapshot = protocol_snapshot(&PROTOCOLS);
    snapshot.admitted_applications = &APPLICATIONS;
    let context = SemanticCommandContext {
        admitted_families: vec![SemanticCommandFamily::TextContent],
        base,
        behavior_version: CommandBehaviorVersion(94),
        context_identity: String::from("context-preflight"),
        insertion_anchors: Vec::<String>::new(),
        local_preconditions: Vec::<String>::new(),
        notebook,
        readable_context: (),
        relevant_constraints: Vec::new(),
        requested_intent: (),
        resource_limits: CommandResourceLimits {
            commands_per_batch: Some(1),
            dependency_edges: Some(0),
            envelope_bytes: None,
            readable_context_bytes: None,
            writable_targets: Some(1),
        },
        writable_targets: vec![target],
    };
    let envelope = SemanticCommandBatchEnvelope {
        binding: SemanticCommandContextBinding {
            base,
            behavior_version: CommandBehaviorVersion(94),
            context_identity: String::from("context-preflight"),
            notebook,
        },
        commands: vec![command(
            1,
            SemanticCommandFamily::TextContent,
            target,
        )],
        protocol_version: CommandBehaviorVersion(7),
        retry_identity: String::from("retry-preflight"),
    };
    assert_eq!(
        semantic_command_envelope_admission(
            &snapshot,
            CommandApplicationCapability::Validate,
            &context,
            &envelope,
        ),
        SemanticCommandEnvelopeAdmission {
            application: SemanticCommandApplicationAdmission::Admitted {
                capability: CommandApplicationCapability::Validate,
            },
            capability: CommandCapabilityCompatibilityOutcome::Compatible {
                snapshot,
            },
            context: semantic_command_envelope_context_admission(
                &context,
                &envelope,
            ),
            protocol: SemanticCommandProtocolAdmission::Admitted {
                version: CommandBehaviorVersion(7),
            },
            resources: SemanticCommandResourceAdmission {
                commands_per_batch:
                    SemanticCommandResourceLimitAdmission::Within {
                        actual: 1,
                        limit: 1,
                    },
                dependency_edges:
                    SemanticCommandResourceLimitAdmission::Within {
                        actual: 0,
                        limit: 0,
                    },
                writable_targets:
                    SemanticCommandResourceLimitAdmission::Within {
                        actual: 1,
                        limit: 1,
                    },
            },
        },
    );
}

#[test]
fn envelope_preflight_preserves_simultaneous_independent_failures() {
    let identities = IdentityAllocator::new();
    let notebook = identities.allocate_accepted().expect("notebook identity");
    let other_notebook = identities
        .allocate_accepted()
        .expect("other notebook identity");
    let target = identities.allocate_accepted().expect("target identity");
    let other_target = identities.allocate_accepted().expect("other target");
    let base = identities.allocate_revision().expect("base revision");
    let other_base = identities
        .allocate_revision()
        .expect("other base revision");
    let mut snapshot = protocol_snapshot(&[]);
    snapshot.behavior_version = CommandBehaviorVersion(95);
    let context = SemanticCommandContext {
        admitted_families: vec![SemanticCommandFamily::TextContent],
        base,
        behavior_version: CommandBehaviorVersion(94),
        context_identity: String::from("context-current"),
        insertion_anchors: Vec::<String>::new(),
        local_preconditions: Vec::<String>::new(),
        notebook,
        readable_context: (),
        relevant_constraints: Vec::new(),
        requested_intent: (),
        resource_limits: CommandResourceLimits {
            commands_per_batch: Some(0),
            dependency_edges: Some(0),
            envelope_bytes: None,
            readable_context_bytes: None,
            writable_targets: Some(0),
        },
        writable_targets: vec![target],
    };
    let envelope = SemanticCommandBatchEnvelope {
        binding: SemanticCommandContextBinding {
            base: other_base,
            behavior_version: CommandBehaviorVersion(93),
            context_identity: String::from("context-stale"),
            notebook: other_notebook,
        },
        commands: vec![command(
            9,
            SemanticCommandFamily::StyleRole,
            other_target,
        )],
        protocol_version: CommandBehaviorVersion(7),
        retry_identity: String::from("retry-stale"),
    };
    let admission = semantic_command_envelope_admission(
        &snapshot,
        CommandApplicationCapability::Validate,
        &context,
        &envelope,
    );
    assert_eq!(
        admission.capability,
        CommandCapabilityCompatibilityOutcome::Mismatch {
            current: CommandBehaviorVersion(95),
            expected: CommandBehaviorVersion(94),
        },
    );
    assert_eq!(
        admission.application,
        SemanticCommandApplicationAdmission::Unsupported {
            requested: CommandApplicationCapability::Validate,
        },
    );
    assert_eq!(
        admission.protocol,
        SemanticCommandProtocolAdmission::Unsupported {
            requested: CommandBehaviorVersion(7),
        },
    );
    assert_eq!(
        admission.context.binding,
        SemanticCommandContextBindingAdmission {
            base: SemanticCommandContextMatch::Mismatched,
            behavior: SemanticCommandContextMatch::Mismatched,
            context: SemanticCommandContextMatch::Mismatched,
            notebook: SemanticCommandContextMatch::Mismatched,
        },
    );
    assert_eq!(
        admission.context.commands,
        vec![SemanticCommandEnvelopeCommandAdmission {
            command: &9,
            family_admitted: false,
            location_admitted: false,
        }],
    );
    assert_eq!(
        admission.resources,
        SemanticCommandResourceAdmission {
            commands_per_batch:
                SemanticCommandResourceLimitAdmission::Exceeded {
                    actual: 1,
                    limit: 0,
                },
            dependency_edges: SemanticCommandResourceLimitAdmission::Within {
                actual: 0,
                limit: 0,
            },
            writable_targets:
                SemanticCommandResourceLimitAdmission::Exceeded {
                    actual: 1,
                    limit: 0,
                },
        },
    );
}

#[test]
fn capability_behavior_admission_detects_stale_context_version() {
    let mut snapshot = protocol_snapshot(&[]);
    snapshot.behavior_version = CommandBehaviorVersion(95);
    assert_eq!(
        semantic_command_capability_behavior_admission(
            &snapshot,
            CommandBehaviorVersion(95),
        ),
        CommandCapabilityCompatibilityOutcome::Compatible { snapshot },
    );
    assert_eq!(
        semantic_command_capability_behavior_admission(
            &snapshot,
            CommandBehaviorVersion(94),
        ),
        CommandCapabilityCompatibilityOutcome::Mismatch {
            current: CommandBehaviorVersion(95),
            expected: CommandBehaviorVersion(94),
        },
    );
}

#[test]
fn all_64_envelope_preflight_axis_masks_match_independent_oracle() {
    static APPLICATIONS: [CommandApplicationCapability; 1] = [
        CommandApplicationCapability::Validate,
    ];
    static PROTOCOLS: [CommandBehaviorVersion; 1] = [CommandBehaviorVersion(7)];
    let identities = IdentityAllocator::new();
    let notebook = identities.allocate_accepted().expect("notebook identity");
    let target = identities.allocate_accepted().expect("target identity");
    let base = identities.allocate_revision().expect("base revision");
    let mut cases = 0_u8;
    for mask in 0_u8..64 {
        let behavior_matches = mask & 0b00_0001 != 0;
        let application_admitted = mask & 0b00_0010 != 0;
        let protocol_admitted = mask & 0b00_0100 != 0;
        let family_admitted = mask & 0b00_1000 != 0;
        let target_admitted = mask & 0b01_0000 != 0;
        let command_limit_admitted = mask & 0b10_0000 != 0;
        let mut snapshot = protocol_snapshot(if protocol_admitted {
            &PROTOCOLS
        } else {
            &[]
        });
        snapshot.behavior_version = if behavior_matches {
            CommandBehaviorVersion(94)
        } else {
            CommandBehaviorVersion(95)
        };
        snapshot.admitted_applications = if application_admitted {
            &APPLICATIONS
        } else {
            &[]
        };
        let context = SemanticCommandContext {
            admitted_families: if family_admitted {
                vec![SemanticCommandFamily::TextContent]
            } else {
                Vec::new()
            },
            base,
            behavior_version: CommandBehaviorVersion(94),
            context_identity: String::from("context-oracle"),
            insertion_anchors: Vec::<String>::new(),
            local_preconditions: Vec::<String>::new(),
            notebook,
            readable_context: (),
            relevant_constraints: Vec::new(),
            requested_intent: (),
            resource_limits: CommandResourceLimits {
                commands_per_batch: Some(if command_limit_admitted {
                    1
                } else {
                    0
                }),
                dependency_edges: None,
                envelope_bytes: None,
                readable_context_bytes: None,
                writable_targets: None,
            },
            writable_targets: if target_admitted {
                vec![target]
            } else {
                Vec::new()
            },
        };
        let envelope = SemanticCommandBatchEnvelope {
            binding: SemanticCommandContextBinding {
                base,
                behavior_version: CommandBehaviorVersion(94),
                context_identity: String::from("context-oracle"),
                notebook,
            },
            commands: vec![command(
                3,
                SemanticCommandFamily::TextContent,
                target,
            )],
            protocol_version: CommandBehaviorVersion(7),
            retry_identity: String::from("retry-oracle"),
        };
        let admission = semantic_command_envelope_admission(
            &snapshot,
            CommandApplicationCapability::Validate,
            &context,
            &envelope,
        );
        assert_eq!(
            admission.capability,
            if behavior_matches {
                CommandCapabilityCompatibilityOutcome::Compatible { snapshot }
            } else {
                CommandCapabilityCompatibilityOutcome::Mismatch {
                    current: CommandBehaviorVersion(95),
                    expected: CommandBehaviorVersion(94),
                }
            },
            "capability mismatch for mask {mask:#08b}",
        );
        assert_eq!(
            admission.application,
            if application_admitted {
                SemanticCommandApplicationAdmission::Admitted {
                    capability: CommandApplicationCapability::Validate,
                }
            } else {
                SemanticCommandApplicationAdmission::Unsupported {
                    requested: CommandApplicationCapability::Validate,
                }
            },
            "application mismatch for mask {mask:#08b}",
        );
        assert_eq!(
            admission.protocol,
            if protocol_admitted {
                SemanticCommandProtocolAdmission::Admitted {
                    version: CommandBehaviorVersion(7),
                }
            } else {
                SemanticCommandProtocolAdmission::Unsupported {
                    requested: CommandBehaviorVersion(7),
                }
            },
            "protocol mismatch for mask {mask:#08b}",
        );
        assert_eq!(
            admission.context.binding,
            SemanticCommandContextBindingAdmission {
                base: SemanticCommandContextMatch::Matched,
                behavior: SemanticCommandContextMatch::Matched,
                context: SemanticCommandContextMatch::Matched,
                notebook: SemanticCommandContextMatch::Matched,
            },
            "binding mismatch for mask {mask:#08b}",
        );
        assert_eq!(
            admission.context.commands,
            vec![SemanticCommandEnvelopeCommandAdmission {
                command: &3,
                family_admitted,
                location_admitted: target_admitted,
            }],
            "scope mismatch for mask {mask:#08b}",
        );
        assert_eq!(
            admission.resources.commands_per_batch,
            if command_limit_admitted {
                SemanticCommandResourceLimitAdmission::Within {
                    actual: 1,
                    limit: 1,
                }
            } else {
                SemanticCommandResourceLimitAdmission::Exceeded {
                    actual: 1,
                    limit: 0,
                }
            },
            "resource mismatch for mask {mask:#08b}",
        );
        assert_eq!(
            admission.resources.dependency_edges,
            SemanticCommandResourceLimitAdmission::Unspecified,
        );
        assert_eq!(
            admission.resources.writable_targets,
            SemanticCommandResourceLimitAdmission::Unspecified,
        );
        cases = cases.saturating_add(1);
    }
    assert_eq!(cases, 64);
}
