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
//   - Transport-neutral assignment organization roles over existing semantic
//     notebook blocks.
//   - Explicit no-invention admission for missing task facts.
// - Must-Not:
//   - Generate content, infer facts, choose provenance, or map organizational
//     roles to new semantic block kinds.
//   - Lay out pages, render, or mutate notebooks.
// - Allows:
//   - Inputs: One semantic notebook snapshot and caller-produced ordered
//     assignment-structure entries.
//   - Outputs: Structural validation that represented entries target resolved
//     blocks and missing facts target typed unresolved blocks.
//   - Side effects: None.
// - Split-When:
//   - Structure generation or source/provenance inference gains executable
//     authority.
// - Merge-When:
//   - Assignment organization becomes inseparable from candidate construction.
// - Summary:
//   - Freezes assignment roles without inventing content or semantic kinds.
// - Description:
//   - Uses existing notebook block authority and explicit unresolved content.
// - Usage:
//   - Validate a caller-produced assignment organization before later layout.
// - Defaults:
//   - No assignment role is required or synthesized when source facts omit it.
//

//! Assignment organization over existing semantic notebook block authority.

use atrament_semantic_notebook::{
    Notebook, SemanticBlockKind, SemanticIdentityKind, semantic_identity_kind,
};

/// All organizational roles named by the first-release assignment task.
pub const ASSIGNMENT_NOTEBOOK_ROLES: [AssignmentNotebookRole; 9] = [
    AssignmentNotebookRole::Title,
    AssignmentNotebookRole::Explanation,
    AssignmentNotebookRole::Derivation,
    AssignmentNotebookRole::Table,
    AssignmentNotebookRole::Equation,
    AssignmentNotebookRole::Diagram,
    AssignmentNotebookRole::Citation,
    AssignmentNotebookRole::Example,
    AssignmentNotebookRole::Conclusion,
];

/// Organizational role applied to an existing semantic notebook block.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum AssignmentNotebookRole {
    /// Reviewable citation material.
    Citation,
    /// Concluding assignment material.
    Conclusion,
    /// Mathematical or explanatory derivation material.
    Derivation,
    /// Diagram-oriented assignment material.
    Diagram,
    /// Equation-oriented assignment material.
    Equation,
    /// Worked or supplied example material.
    Example,
    /// Explanatory assignment material.
    Explanation,
    /// Structured table material.
    Table,
    /// Dominant assignment title material.
    Title,
}

/// Whether one organizational role is represented or explicitly unresolved.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AssignmentNotebookDisposition<Identity> {
    /// Existing resolved semantic block represents this organizational role.
    Represented {
        /// Existing resolved semantic block identity.
        block: Identity,
    },
    /// Missing/ambiguous task fact is retained as an unresolved semantic block.
    UnresolvedMissingFact {
        /// Existing typed unresolved semantic block identity.
        block: Identity,
    },
}

/// One caller-produced assignment organization entry in reading intent order.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AssignmentNotebookStructureEntry<Identity> {
    /// Existing semantic block and its explicit resolution disposition.
    pub disposition: AssignmentNotebookDisposition<Identity>,
    /// Organizational role without creating a new semantic content kind.
    pub role: AssignmentNotebookRole,
}

/// Ordered assignment organization over one semantic notebook snapshot.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AssignmentNotebookStructurePlan<Identity> {
    /// Caller-produced entries in intended semantic reading order.
    pub entries: Vec<AssignmentNotebookStructureEntry<Identity>>,
}

/// Why one assignment organization entry cannot be admitted structurally.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AssignmentNotebookStructureError {
    /// Entry points to an identity that is not a semantic block.
    NotBlock {
        /// Zero-based entry index in caller order.
        entry_index: usize,
    },
    /// Entry marked represented points to typed unresolved content.
    RepresentedIsUnresolved {
        /// Zero-based entry index in caller order.
        entry_index: usize,
    },
    /// Entry points to no semantic identity in the supplied notebook snapshot.
    UnknownIdentity {
        /// Zero-based entry index in caller order.
        entry_index: usize,
    },
    /// Missing-fact entry points to a resolved semantic block.
    UnresolvedMissingFactIsResolved {
        /// Zero-based entry index in caller order.
        entry_index: usize,
    },
}

/// Validate one assignment organization against exact notebook block authority.
///
/// Organizational roles remain caller-owned labels. This function does not map
/// title/example/conclusion roles into invented semantic block kinds and does
/// not require all nine roles to occur in every assignment.
///
/// # Errors
///
/// Returns the first entry whose identity is missing/non-block or whose
/// explicit
/// resolved/unresolved disposition disagrees with the notebook's block kind.
pub fn validate_assignment_notebook_structure<Identity>(
    notebook: &Notebook<Identity>,
    plan: &AssignmentNotebookStructurePlan<Identity>,
) -> Result<(), AssignmentNotebookStructureError>
where
    Identity: Copy + Eq,
{
    for (entry_index, entry) in plan.entries.iter().enumerate() {
        let (block, expects_unresolved) = match entry.disposition {
            AssignmentNotebookDisposition::Represented { block } => {
                (block, false)
            }
            AssignmentNotebookDisposition::UnresolvedMissingFact { block } => {
                (block, true)
            }
        };
        let Some(kind) = semantic_identity_kind(notebook, block) else {
            return Err(AssignmentNotebookStructureError::UnknownIdentity {
                entry_index,
            });
        };
        let SemanticIdentityKind::Block(block_kind) = kind else {
            return Err(AssignmentNotebookStructureError::NotBlock {
                entry_index,
            });
        };
        let is_unresolved = block_kind == SemanticBlockKind::Unresolved;
        if expects_unresolved && !is_unresolved {
            return Err(
                AssignmentNotebookStructureError::
                    UnresolvedMissingFactIsResolved { entry_index },
            );
        }
        if !expects_unresolved && is_unresolved {
            return Err(
                AssignmentNotebookStructureError::RepresentedIsUnresolved {
                    entry_index,
                },
            );
        }
    }
    Ok(())
}
