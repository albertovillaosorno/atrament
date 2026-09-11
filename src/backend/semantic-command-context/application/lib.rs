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
    SemanticCommandContext, SemanticCommandFamily,
    SemanticCommandScopeAdmission, SemanticCommandScopeLocation,
};

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
