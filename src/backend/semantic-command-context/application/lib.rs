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
//   - Outputs: Independent family and location admission facts.
//   - Side effects: None.
// - Split-When:
//   - Context construction or protocol compatibility gains executable
//     authority.
// - Merge-When:
//   - Scope admission becomes inseparable from a full command Apply boundary.
// - Summary:
//   - Prevents readable semantic context from silently widening write scope.
// - Description:
//   - Checks only backend-declared writable targets, anchors, and families.
// - Usage:
//   - Evaluate parsed command use before semantic simulation or accepted Apply.
// - Defaults:
//   - Missing family or location authority remains independently unadmitted.
//

//! Read-only semantic command-context scope admission.

use atrament_semantic_notebook_port::{
    SemanticCommandContext, SemanticCommandContextBinding,
    SemanticCommandContextBindingAdmission, SemanticCommandFamily,
    SemanticCommandScopeAdmission, SemanticCommandScopeLocation,
};


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
        base_matches: binding.base == context.base,
        behavior_matches: binding.behavior_version == context.behavior_version,
        context_matches: binding.context_identity == context.context_identity,
        notebook_matches: binding.notebook == context.notebook,
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
    location: SemanticCommandScopeLocation<'_, InsertionAnchor>,
) -> SemanticCommandScopeAdmission
where
    InsertionAnchor: PartialEq,
{
    let family_admitted = context.admitted_families.contains(&family);
    let location_admitted = match location {
        SemanticCommandScopeLocation::Existing(target) => {
            context.writable_targets.contains(&target)
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
