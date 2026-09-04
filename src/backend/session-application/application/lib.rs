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
//   - Active-process pre-acceptance draft and accepted semantic notebook state.
// - Must-Not:
//   - Persist session state, parse transport input, or duplicate semantic
//     rules.
// - Allows:
//   - Inputs: Existing draft and semantic application-service operations.
//   - Outputs: Typed semantic outcomes, borrowed accepted state, and the draft
//     port.
//   - Side effects: Process-local mutation through owned application services.
// - Split-When:
//   - Assets or derived state require independently bounded owners.
// - Merge-When:
//   - One broader application authority owns every active-session capability.
// - Summary:
//   - Owns mutable Atrament application state for one disposable process.
// - Description:
//   - Gives draft and accepted notebook state one process-lifetime owner while
//     preserving their established application contracts.
// - Usage:
//   - Construct one instance in the runtime composition root and drop it when
//     the active localhost session ends.
// - Defaults:
//   - Starts with empty draft fields and no accepted semantic revision.
//

//! Process-lifetime owner for disposable Atrament application state.

use std::collections::BTreeSet;
use std::fmt;

use atrament_semantic_notebook::{
    AcceptedIdentity, AcceptedRevision, CandidateIdentity, FormulaMode,
    Notebook, PhysicalPageProfile, RevisionIdentity, TableCellSpan,
    TableRowRole,
};
use atrament_semantic_notebook_port::{
    AcceptanceOutcome, CommandCapabilityCompatibilityOutcome,
    CommandGraphLimits,
    CommandFamilyAdmissionOutcome, CommandTargetMaterialOutcome,
    CommandTargetPreconditionOutcome, CommandTargetPreconditions,
    DirectEditBatchApplyOutcome, DirectEditBatchGraphLimitsOutcome,
    DirectEditBatchGraphSizeOutcome, DirectEditBatchProposal,
    DirectEditBatchSelectionBoundedOutcome,
    DirectEditBatchSelectionRequirementsOutcome,
    DirectEditBatchSelectionSummaryOutcome, DirectEditBatchSimulationOutcome,
    DirectEditChangePreviewOutcome,
    DirectEditProposal, DirectEditProposalOutcome, DirectEditSimulationOutcome,
    EditableSemanticValue, EditableValuePreconditionOutcome, FormulaEditOutcome,
    HistoryAvailabilityOutcome, HistoryDirection, HistoryTraversalOutcome,
    IdentityAncestryInspectOutcome, IdentityInspectOutcome,
    IdentityKindInspectOutcome, IdentityPrecondition,
    IdentityPreconditionOutcome, PageProfileEditOutcome,
    SemanticCommandCapabilitySnapshot, SemanticCommandFamily,
    SemanticNotebookHistory as _,
    SemanticNotebookSession as _, TableCellSpanEditOutcome,
    TableRowRoleEditOutcome, TextEditOutcome,
};
use atrament_semantic_notebook_session::SemanticNotebookSessionService;
use atrament_session_draft::SessionDraftService;
use atrament_session_draft_port::{DraftField, DraftMutation, SessionDraft};

/// Mutable application state owned by one active Atrament process.
#[derive(Default)]
pub struct SessionApplication {
    draft: SessionDraftService,
    semantic: SemanticNotebookSessionService,
}

impl fmt::Debug for SessionApplication {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SessionApplication").finish_non_exhaustive()
    }
}

impl SessionApplication {
    /// Accept one complete candidate through the owned semantic authority.
    pub fn accept_candidate(
        &mut self,
        candidate: Notebook<CandidateIdentity>,
    ) -> AcceptanceOutcome {
        self.semantic.accept(candidate)
    }

    /// Read the current accepted semantic revision without creating another.
    #[must_use]
    pub fn accepted_revision(&self) -> Option<&AcceptedRevision> {
        self.semantic.current()
    }

    /// Apply one transport-neutral semantic batch through the owned authority.
    pub fn apply_direct_edit_batch<CommandIdentity>(
        &mut self,
        batch: DirectEditBatchProposal<CommandIdentity>,
    ) -> DirectEditBatchApplyOutcome<CommandIdentity>
    where
        CommandIdentity: Clone + Ord,
    {
        self.semantic.apply_direct_edit_batch(batch)
    }

    /// Apply one caller-bounded semantic batch through the owned authority.
    pub fn apply_direct_edit_batch_bounded<CommandIdentity>(
        &mut self,
        batch: DirectEditBatchProposal<CommandIdentity>,
        limits: CommandGraphLimits,
    ) -> DirectEditBatchApplyOutcome<CommandIdentity>
    where
        CommandIdentity: Clone + Ord,
    {
        self.semantic.apply_direct_edit_batch_bounded(batch, limits)
    }

    /// Check one previously bound semantic command behavior version.
    #[must_use]
    pub fn check_command_capability_compatibility(
        &self,
        expected: atrament_semantic_notebook_port::CommandBehaviorVersion,
    ) -> CommandCapabilityCompatibilityOutcome {
        self.semantic.check_command_capability_compatibility(expected)
    }

    /// Check whether one semantic command family is executable for a target.
    #[must_use]
    pub fn check_command_family_admission(
        &self,
        revision: RevisionIdentity,
        target: AcceptedIdentity,
        requested: SemanticCommandFamily,
    ) -> CommandFamilyAdmissionOutcome {
        self.semantic
            .check_command_family_admission(revision, target, requested)
    }

    /// Check complete local command-target preconditions without mutation.
    #[must_use]
    pub fn check_command_target_preconditions(
        &self,
        revision: RevisionIdentity,
        target: AcceptedIdentity,
        preconditions: CommandTargetPreconditions,
    ) -> CommandTargetPreconditionOutcome {
        self.semantic
            .check_command_target_preconditions(revision, target, preconditions)
    }

    /// Compare one exact editable semantic value without mutation.
    #[must_use]
    pub fn check_editable_value_precondition(
        &self,
        revision: RevisionIdentity,
        target: AcceptedIdentity,
        expected: EditableSemanticValue,
    ) -> EditableValuePreconditionOutcome {
        self.semantic
            .check_editable_value_precondition(revision, target, expected)
    }

    /// Check one exact local semantic identity precondition without mutation.
    #[must_use]
    pub fn check_identity_precondition(
        &self,
        revision: RevisionIdentity,
        target: AcceptedIdentity,
        precondition: IdentityPrecondition,
    ) -> IdentityPreconditionOutcome {
        self.semantic
            .check_identity_precondition(revision, target, precondition)
    }

    /// Discover current transport-neutral semantic command behavior.
    #[must_use]
    pub fn command_capability_snapshot(
        &self,
    ) -> SemanticCommandCapabilitySnapshot {
        self.semantic.command_capability_snapshot()
    }

    /// Derive exact local command material for one accepted semantic target.
    #[must_use]
    pub fn command_target_material(
        &self,
        revision: RevisionIdentity,
        target: AcceptedIdentity,
    ) -> CommandTargetMaterialOutcome {
        self.semantic.command_target_material(revision, target)
    }

    /// Derive exact local command material for one target and family.
    #[must_use]
    pub fn command_target_material_for_family(
        &self,
        revision: RevisionIdentity,
        target: AcceptedIdentity,
        family: SemanticCommandFamily,
    ) -> CommandTargetMaterialOutcome {
        self.semantic
            .command_target_material_for_family(revision, target, family)
    }

    /// Enforce caller-supplied coarse batch graph bounds without mutation.
    #[must_use]
    pub fn direct_edit_batch_graph_limits<CommandIdentity>(
        &self,
        batch: &DirectEditBatchProposal<CommandIdentity>,
        limits: CommandGraphLimits,
    ) -> DirectEditBatchGraphLimitsOutcome {
        self.semantic.direct_edit_batch_graph_limits(batch, limits)
    }

    /// Derive exact coarse batch graph size without mutation.
    #[must_use]
    pub fn direct_edit_batch_graph_size<CommandIdentity>(
        &self,
        batch: &DirectEditBatchProposal<CommandIdentity>,
    ) -> DirectEditBatchGraphSizeOutcome {
        self.semantic.direct_edit_batch_graph_size(batch)
    }

    /// Analyze omitted dependencies for one in-memory batch selection.
    #[must_use]
    pub fn direct_edit_batch_selection_requirements<CommandIdentity>(
        &self,
        batch: &DirectEditBatchProposal<CommandIdentity>,
        selected: &BTreeSet<CommandIdentity>,
    ) -> DirectEditBatchSelectionRequirementsOutcome<CommandIdentity>
    where
        CommandIdentity: Clone + Ord,
    {
        self.semantic
            .direct_edit_batch_selection_requirements(batch, selected)
    }

    /// Analyze omitted dependencies under one caller-supplied report bound.
    #[must_use]
    pub fn direct_edit_batch_selection_requirements_bounded<CommandIdentity>(
        &self,
        batch: &DirectEditBatchProposal<CommandIdentity>,
        selected: &BTreeSet<CommandIdentity>,
        maximum_missing_edges: usize,
    ) -> DirectEditBatchSelectionBoundedOutcome<CommandIdentity>
    where
        CommandIdentity: Clone + Ord,
    {
        self.semantic.direct_edit_batch_selection_requirements_bounded(
            batch,
            selected,
            maximum_missing_edges,
        )
    }

    /// Summarize one in-memory batch selection without materializing edges.
    #[must_use]
    pub fn direct_edit_batch_selection_summary<CommandIdentity>(
        &self,
        batch: &DirectEditBatchProposal<CommandIdentity>,
        selected: &BTreeSet<CommandIdentity>,
    ) -> DirectEditBatchSelectionSummaryOutcome<CommandIdentity>
    where
        CommandIdentity: Clone + Ord,
    {
        self.semantic
            .direct_edit_batch_selection_summary(batch, selected)
    }

    /// Inspect in-memory semantic Undo and Redo availability.
    #[must_use]
    pub fn history_availability(&self) -> HistoryAvailabilityOutcome {
        self.semantic.history_availability()
    }

    /// Inspect one accepted semantic identity without mutation.
    #[must_use]
    pub fn inspect_identity(
        &self,
        revision: RevisionIdentity,
        target: AcceptedIdentity,
    ) -> IdentityInspectOutcome {
        self.semantic.inspect_identity(revision, target)
    }

    /// Inspect one bounded target-first semantic owner chain without mutation.
    #[must_use]
    pub fn inspect_identity_ancestry_bounded(
        &self,
        revision: RevisionIdentity,
        target: AcceptedIdentity,
        maximum_results: usize,
    ) -> IdentityAncestryInspectOutcome {
        self.semantic.inspect_identity_ancestry_bounded(
            revision,
            target,
            maximum_results,
        )
    }

    /// Inspect one accepted semantic identity kind without mutation.
    #[must_use]
    pub fn inspect_identity_kind(
        &self,
        revision: RevisionIdentity,
        target: AcceptedIdentity,
    ) -> IdentityKindInspectOutcome {
        self.semantic.inspect_identity_kind(revision, target)
    }

    /// Preview one exact direct semantic change without mutation.
    #[must_use]
    pub fn preview_direct_edit_changes(
        &self,
        revision: RevisionIdentity,
        target: AcceptedIdentity,
        requested: EditableSemanticValue,
    ) -> DirectEditChangePreviewOutcome {
        self.semantic
            .preview_direct_edit_changes(revision, target, requested)
    }

    /// Replace one mathematical source through the owned semantic authority.
    pub fn replace_formula(
        &mut self,
        base: RevisionIdentity,
        target: AcceptedIdentity,
        mode: FormulaMode,
        source: String,
    ) -> FormulaEditOutcome {
        self.semantic.replace_formula(base, target, mode, source)
    }

    /// Replace one physical page profile through the owned semantic authority.
    pub fn replace_page_profile(
        &mut self,
        base: RevisionIdentity,
        target: AcceptedIdentity,
        geometry: PhysicalPageProfile,
    ) -> PageProfileEditOutcome {
        self.semantic.replace_page_profile(base, target, geometry)
    }

    /// Replace one table-cell span through the owned semantic authority.
    pub fn replace_table_cell_span(
        &mut self,
        base: RevisionIdentity,
        target: AcceptedIdentity,
        span: TableCellSpan,
    ) -> TableCellSpanEditOutcome {
        self.semantic.replace_table_cell_span(base, target, span)
    }

    /// Replace one table-row role through the owned semantic authority.
    pub fn replace_table_row_role(
        &mut self,
        base: RevisionIdentity,
        target: AcceptedIdentity,
        role: TableRowRole,
    ) -> TableRowRoleEditOutcome {
        self.semantic.replace_table_row_role(base, target, role)
    }

    /// Replace one inline text value through the owned semantic authority.
    pub fn replace_text(
        &mut self,
        base: RevisionIdentity,
        target: AcceptedIdentity,
        value: String,
    ) -> TextEditOutcome {
        self.semantic.replace_text(base, target, value)
    }

    /// Simulate one established direct semantic edit without mutation.
    #[must_use]
    pub fn simulate_direct_edit(
        &self,
        revision: RevisionIdentity,
        target: AcceptedIdentity,
        requested: EditableSemanticValue,
    ) -> DirectEditSimulationOutcome {
        self.semantic.simulate_direct_edit(revision, target, requested)
    }

    /// Simulate one transport-neutral direct-edit batch without mutation.
    #[must_use]
    pub fn simulate_direct_edit_batch<CommandIdentity>(
        &self,
        batch: DirectEditBatchProposal<CommandIdentity>,
    ) -> DirectEditBatchSimulationOutcome<CommandIdentity>
    where
        CommandIdentity: Clone + Ord,
    {
        self.semantic.simulate_direct_edit_batch(batch)
    }

    /// Simulate one caller-bounded direct-edit batch without mutation.
    #[must_use]
    pub fn simulate_direct_edit_batch_bounded<CommandIdentity>(
        &self,
        batch: DirectEditBatchProposal<CommandIdentity>,
        limits: CommandGraphLimits,
    ) -> DirectEditBatchSimulationOutcome<CommandIdentity>
    where
        CommandIdentity: Clone + Ord,
    {
        self.semantic.simulate_direct_edit_batch_bounded(batch, limits)
    }

    /// Validate and simulate one version-bound direct-edit proposal read-only.
    #[must_use]
    pub fn simulate_direct_edit_proposal(
        &self,
        proposal: DirectEditProposal,
    ) -> DirectEditProposalOutcome {
        self.semantic.simulate_direct_edit_proposal(proposal)
    }

    /// Traverse one in-memory semantic history transaction.
    pub fn traverse_history(
        &mut self,
        base: RevisionIdentity,
        direction: HistoryDirection,
    ) -> HistoryTraversalOutcome {
        self.semantic.traverse_history(base, direction)
    }
}

impl SessionDraft for SessionApplication {
    fn replace(&mut self, field: DraftField, value: String) -> DraftMutation {
        self.draft.replace(field, value)
    }

    fn value(&self, field: DraftField) -> &str {
        self.draft.value(field)
    }
}
