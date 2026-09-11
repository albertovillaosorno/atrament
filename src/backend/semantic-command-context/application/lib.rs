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
//   - Read-only admission checks for one backend-owned semantic command
//     context.
// - Must-Not:
//   - Construct context, compute context identity, parse envelopes, normalize
//     batches, mutate notebooks, choose scope, or implement retry behavior.
// - Allows:
//   - Inputs: One declared command context plus requested family and location.
//   - Outputs: Independent capability, context, scope, and resource facts.
//   - Side effects: None.
// - Split-When:
//   - Context construction or protocol compatibility gains executable
//     authority.
// - Merge-When:
//   - Scope admission becomes inseparable from a full command Apply boundary.
// - Summary:
//   - Prevents readable semantic context from silently widening write scope.
// - Description:
//   - Checks backend-declared capability, authority, and count limits.
// - Usage:
//   - Evaluate parsed command use before semantic simulation or accepted Apply.
// - Defaults:
//   - Missing capability, family, or location authority stays unadmitted.
//

//! Read-only semantic command-context scope admission.

use atrament_semantic_notebook_port::{
    CommandApplicationCapability, CommandBehaviorVersion,
    SemanticCommandApplicationAdmission, SemanticCommandBatchEnvelope,
    SemanticCommandCapabilitySnapshot, SemanticCommandContext,
    SemanticCommandContextBinding, SemanticCommandContextBindingAdmission,
    SemanticCommandContextMatch, SemanticCommandEnvelopeCommandAdmission,
    SemanticCommandEnvelopeContextAdmission, SemanticCommandFamily,
    SemanticCommandFamilyBehaviorAdmission, SemanticCommandProtocolAdmission,
    SemanticCommandResourceAdmission, SemanticCommandResourceLimitAdmission,
    SemanticCommandScopeAdmission, SemanticCommandScopeLocation,
};


const fn match_state(matches: bool) -> SemanticCommandContextMatch {
    if matches {
        SemanticCommandContextMatch::Matched
    } else {
        SemanticCommandContextMatch::Mismatched
    }
}

/// Check whether returned batch authority still names this exact command
/// context.
///
/// Each equality fact remains independent so a later typed result mapper can
/// preserve the actual mismatch instead of relying on an invented precedence.
#[must_use]
pub fn semantic_command_context_binding_admission<
    ContextIdentity,
    InsertionAnchor,
    Intent,
    PreconditionMaterial,
    ReadableContext,
>(
    context: &SemanticCommandContext<
        ContextIdentity,
        InsertionAnchor,
        Intent,
        PreconditionMaterial,
        ReadableContext,
    >,
    binding: &SemanticCommandContextBinding<ContextIdentity>,
) -> SemanticCommandContextBindingAdmission
where
    ContextIdentity: PartialEq,
{
    SemanticCommandContextBindingAdmission {
        base: match_state(binding.base == context.base),
        behavior: match_state(
            binding.behavior_version == context.behavior_version,
        ),
        context: match_state(
            binding.context_identity == context.context_identity,
        ),
        notebook: match_state(binding.notebook == context.notebook),
    }
}


/// Review one parsed envelope against one backend-owned command context.
///
/// This checks only context binding plus per-command family and existing-target
/// scope. Protocol-version admission, command semantics, normalization, batch
/// identity, retry equality, and Apply remain separate authorities.
#[must_use]
pub fn semantic_command_envelope_context_admission<
    'command,
    CommandIdentity,
    ContextIdentity,
    InsertionAnchor,
    Intent,
    PreconditionMaterial,
    ReadableContext,
    RetryIdentity,
>(
    context: &SemanticCommandContext<
        ContextIdentity,
        InsertionAnchor,
        Intent,
        PreconditionMaterial,
        ReadableContext,
    >,
    envelope: &'command SemanticCommandBatchEnvelope<
        CommandIdentity,
        ContextIdentity,
        RetryIdentity,
    >,
) -> SemanticCommandEnvelopeContextAdmission<'command, CommandIdentity>
where
    ContextIdentity: PartialEq,
    InsertionAnchor: PartialEq,
{
    let binding = semantic_command_context_binding_admission(
        context,
        &envelope.binding,
    );
    let commands = envelope
        .commands
        .iter()
        .map(|command| {
            let scope = semantic_command_scope_admission(
                context,
                command.preconditions.requested_family,
                &SemanticCommandScopeLocation::Existing(command.target),
            );
            SemanticCommandEnvelopeCommandAdmission {
                command: &command.id,
                family_admitted: scope.family_admitted,
                location_admitted: scope.location_admitted,
            }
        })
        .collect();
    SemanticCommandEnvelopeContextAdmission { binding, commands }
}



/// Check exact application-capability membership in one snapshot.
///
/// This is a read-only discovery check. It does not make an application
/// operation executable when the snapshot omits it.
#[must_use]
pub fn semantic_command_application_admission(
    snapshot: &SemanticCommandCapabilitySnapshot,
    requested: CommandApplicationCapability,
) -> SemanticCommandApplicationAdmission {
    if snapshot.admitted_applications.contains(&requested) {
        SemanticCommandApplicationAdmission::Admitted {
            capability: requested,
        }
    } else {
        SemanticCommandApplicationAdmission::Unsupported { requested }
    }
}

/// Check one semantic family's exact advertised behavior version.
///
/// Family discovery and family behavior compatibility remain separate from
/// target-specific executable admission and from serialized protocol support.
#[must_use]
pub fn semantic_command_family_behavior_admission(
    snapshot: &SemanticCommandCapabilitySnapshot,
    family: SemanticCommandFamily,
    expected: CommandBehaviorVersion,
) -> SemanticCommandFamilyBehaviorAdmission {
    let Some(capability) = snapshot
        .family_capabilities
        .iter()
        .find(|capability| capability.family == family)
    else {
        return SemanticCommandFamilyBehaviorAdmission::UnsupportedFamily {
            requested: family,
        };
    };
    if capability.behavior_version == expected {
        SemanticCommandFamilyBehaviorAdmission::Admitted {
            behavior_version: expected,
            family,
        }
    } else {
        SemanticCommandFamilyBehaviorAdmission::BehaviorMismatch {
            current: capability.behavior_version,
            expected,
            family,
        }
    }
}

/// Check exact protocol-version membership in one capability snapshot.
///
/// The check never guesses a downgrade or treats the capability behavior
/// version as a command protocol version. An empty advertised protocol set
/// therefore rejects every requested protocol token.
#[must_use]
pub fn semantic_command_protocol_admission(
    snapshot: &SemanticCommandCapabilitySnapshot,
    requested: CommandBehaviorVersion,
) -> SemanticCommandProtocolAdmission {
    if snapshot.protocol_versions.contains(&requested) {
        SemanticCommandProtocolAdmission::Admitted { version: requested }
    } else {
        SemanticCommandProtocolAdmission::Unsupported { requested }
    }
}

const fn count_limit_admission(
    actual: usize,
    configured_limit: Option<usize>,
) -> SemanticCommandResourceLimitAdmission {
    let Some(limit) = configured_limit else {
        return SemanticCommandResourceLimitAdmission::Unspecified;
    };
    if actual > limit {
        SemanticCommandResourceLimitAdmission::Exceeded { actual, limit }
    } else {
        SemanticCommandResourceLimitAdmission::Within { actual, limit }
    }
}

fn dependency_edge_limit_admission<CommandIdentity>(
    commands: &[atrament_semantic_notebook_port::DirectEditBatchCommand<
        CommandIdentity,
    >],
    configured_limit: Option<usize>,
) -> SemanticCommandResourceLimitAdmission {
    let Some(limit) = configured_limit else {
        return SemanticCommandResourceLimitAdmission::Unspecified;
    };
    let mut actual = 0usize;
    for command in commands {
        let Some(next) = actual.checked_add(command.dependencies.len()) else {
            return SemanticCommandResourceLimitAdmission::Overflow;
        };
        actual = next;
    }
    count_limit_admission(actual, Some(limit))
}

/// Check count-based envelope and writable-scope limits bound by one context.
///
/// This covers only counts represented exactly by current transport-neutral
/// types. Serialized envelope bytes, readable-context bytes, structured depth,
/// and family-specific payload sizes remain outside this admission until their
/// owning representation is defined.
#[must_use]
pub fn semantic_command_resource_admission<
    CommandIdentity,
    ContextIdentity,
    InsertionAnchor,
    Intent,
    PreconditionMaterial,
    ReadableContext,
    RetryIdentity,
>(
    context: &SemanticCommandContext<
        ContextIdentity,
        InsertionAnchor,
        Intent,
        PreconditionMaterial,
        ReadableContext,
    >,
    envelope: &SemanticCommandBatchEnvelope<
        CommandIdentity,
        ContextIdentity,
        RetryIdentity,
    >,
) -> SemanticCommandResourceAdmission {
    let commands_per_batch = count_limit_admission(
        envelope.commands.len(),
        context.resource_limits.commands_per_batch,
    );
    let dependency_edges = dependency_edge_limit_admission(
        &envelope.commands,
        context.resource_limits.dependency_edges,
    );
    let writable_targets =
        match context.resource_limits.writable_targets {
            None => SemanticCommandResourceLimitAdmission::Unspecified,
            Some(limit) => {
                let Some(actual) = context
                    .writable_targets
                    .len()
                    .checked_add(context.insertion_anchors.len())
                else {
                    return SemanticCommandResourceAdmission {
                        commands_per_batch,
                        dependency_edges,
                        writable_targets:
                            SemanticCommandResourceLimitAdmission::Overflow,
                    };
                };
                count_limit_admission(actual, Some(limit))
            },
        };
    SemanticCommandResourceAdmission {
        commands_per_batch,
        dependency_edges,
        writable_targets,
    }
}

/// Check bounded command scope without widening readable context into
/// authority.
///
/// Family and location membership are reported independently so callers do not
/// need an invented error-precedence rule. Readable context, context identity,
/// intent, constraints, and precondition material never grant write permission
/// merely because they are present in the context value.
#[must_use]
pub fn semantic_command_scope_admission<
    ContextIdentity,
    InsertionAnchor,
    Intent,
    PreconditionMaterial,
    ReadableContext,
>(
    context: &SemanticCommandContext<
        ContextIdentity,
        InsertionAnchor,
        Intent,
        PreconditionMaterial,
        ReadableContext,
    >,
    family: SemanticCommandFamily,
    location: &SemanticCommandScopeLocation<'_, InsertionAnchor>,
) -> SemanticCommandScopeAdmission
where
    InsertionAnchor: PartialEq,
{
    let family_admitted = context.admitted_families.contains(&family);
    let location_admitted = match location {
        SemanticCommandScopeLocation::Existing(target) => {
            context.writable_targets.contains(target)
        },
        SemanticCommandScopeLocation::Insertion(anchor) => {
            context.insertion_anchors.contains(anchor)
        },
    };
    SemanticCommandScopeAdmission {
        family_admitted,
        location_admitted,
    }
}
