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
//   - Covers target, anchor, family, and caller-owned context payload behavior.
// - Usage:
//   - Compile against the command-context application and inbound port.
// - Defaults:
//   - Missing family or location authority remains independently unadmitted.
//
use atrament_semantic_command_context::{
    semantic_command_context_binding_admission,
    semantic_command_scope_admission,
};
use atrament_semantic_notebook::IdentityAllocator;
use atrament_semantic_notebook_port::{
    CommandBehaviorVersion, CommandResourceLimits, SemanticCommandContext,
    SemanticCommandContextBinding, SemanticCommandContextBindingAdmission,
    SemanticCommandFamily, SemanticCommandScopeAdmission,
    SemanticCommandScopeLocation,
};

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
            SemanticCommandScopeLocation::Existing(readable),
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
            location,
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
                SemanticCommandScopeLocation::Insertion(location),
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
                base_matches,
                behavior_matches,
                context_matches,
                notebook_matches,
            },
            "binding match mask {mask:#06b}",
        );
        cases = cases.saturating_add(1);
    }
    assert_eq!(cases, 16);
}
