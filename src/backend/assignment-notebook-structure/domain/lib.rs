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
    Block, Notebook, SemanticBlockKind, SemanticIdentityKind, semantic_block,
    semantic_identity_kind,
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

/// Exact semantic blocks referenced by one validated assignment structure.
pub type AssignmentNotebookStructureBlocks<'notebook, Identity> =
    Vec<&'notebook Block<Identity>>;

/// Constructor-sealed evidence binding one exact notebook snapshot and
/// assignment plan to its fully validated block projection.
#[derive(Debug)]
pub struct ValidatedAssignmentNotebookStructure<
    'notebook,
    'plan,
    Identity,
> {
    blocks: AssignmentNotebookStructureBlocks<'notebook, Identity>,
    notebook: &'notebook Notebook<Identity>,
    plan: &'plan AssignmentNotebookStructurePlan<Identity>,
}

impl<'notebook, 'plan, Identity>
    ValidatedAssignmentNotebookStructure<'notebook, 'plan, Identity>
{
    /// Return exact projected blocks in caller plan order.
    #[must_use]
    pub fn blocks(&self) -> &[&'notebook Block<Identity>] {
        &self.blocks
    }

    /// Return the exact semantic notebook snapshot used for admission.
    #[must_use]
    pub const fn notebook(&self) -> &'notebook Notebook<Identity> {
        self.notebook
    }

    /// Return the exact caller-produced assignment plan that was admitted.
    #[must_use]
    pub const fn plan(
        &self,
    ) -> &'plan AssignmentNotebookStructurePlan<Identity> {
        self.plan
    }
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

const fn assignment_entry_block<Identity>(
    entry: &AssignmentNotebookStructureEntry<Identity>,
) -> Identity
where
    Identity: Copy,
{
    match entry.disposition {
        AssignmentNotebookDisposition::Represented { block }
        | AssignmentNotebookDisposition::UnresolvedMissingFact { block } => {
            block
        },
    }
}

/// Project exact semantic blocks only after complete structural validation.
///
/// The result preserves caller plan order and repeated entries. It borrows the
/// authoritative notebook blocks without rewriting content, assigning roles,
/// choosing structure, or turning unresolved material into resolved content.
///
/// # Errors
///
/// Returns the same first structural failure as
/// [`validate_assignment_notebook_structure`] and exposes no partial
/// projection.
pub fn project_assignment_notebook_structure_blocks<'notebook, Identity>(
    notebook: &'notebook Notebook<Identity>,
    plan: &AssignmentNotebookStructurePlan<Identity>,
) -> Result<
    AssignmentNotebookStructureBlocks<'notebook, Identity>,
    AssignmentNotebookStructureError,
>
where
    Identity: Copy + Eq,
{
    validate_assignment_notebook_structure(notebook, plan)?;
    plan.entries
        .iter()
        .enumerate()
        .map(|(entry_index, entry)| {
            semantic_block(
                notebook,
                assignment_entry_block(entry),
            )
            .ok_or(
                AssignmentNotebookStructureError::UnknownIdentity {
                    entry_index,
                },
            )
        })
        .collect()
}

/// Validate one assignment structure and seal its exact notebook/plan
/// projection.
///
/// # Errors
///
/// Returns exactly the same first structural failure as
/// [`validate_assignment_notebook_structure`].
pub fn validate_assignment_notebook_structure_view<
    'notebook,
    'plan,
    Identity,
>(
    notebook: &'notebook Notebook<Identity>,
    plan: &'plan AssignmentNotebookStructurePlan<Identity>,
) -> Result<
    ValidatedAssignmentNotebookStructure<'notebook, 'plan, Identity>,
    AssignmentNotebookStructureError,
>
where
    Identity: Copy + Eq,
{
    let blocks = project_assignment_notebook_structure_blocks(notebook, plan)?;
    Ok(ValidatedAssignmentNotebookStructure {
        blocks,
        notebook,
        plan,
    })
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
        let block = assignment_entry_block(entry);
        let expects_unresolved = matches!(
            entry.disposition,
            AssignmentNotebookDisposition::UnresolvedMissingFact { .. },
        );
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
