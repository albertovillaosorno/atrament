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
//   - Active-process draft, raw asset bytes, and accepted semantic notebook
//     state.
// - Must-Not:
//   - Persist session state, parse transport input, or duplicate semantic
//     rules.
// - Allows:
//   - Inputs: Existing draft and semantic application-service operations plus
//     raw bytes already validated by an ingestion boundary.
//   - Outputs: Typed outcomes plus borrowed accepted state and raw asset bytes.
//   - Side effects: Process-local mutation through owned application services.
// - Split-When:
//   - Derived state or bounded media policy requires an independent owner.
// - Merge-When:
//   - One broader application authority owns every active-session capability.
// - Summary:
//   - Owns mutable Atrament application state for one disposable process.
// - Description:
//   - Gives draft, accepted notebook, and retained asset bytes one process
//     owner while preserving their established application contracts.
// - Usage:
//   - Construct one instance in the runtime composition root and drop it when
//     the active localhost session ends.
// - Defaults:
//   - Starts with empty draft fields, no accepted revision, and no asset bytes.
//

//! Process-lifetime owner for disposable Atrament application state.

use std::collections::btree_map::Entry;
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use atrament_semantic_notebook::{
    AcceptedIdentity, AcceptedRevision, CandidateIdentity, FormulaMode,
    Notebook, PhysicalPageProfile, RevisionIdentity, SemanticIdentityKind,
    TableCellSpan, TableRowRole,
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

/// Typed failure to retain or inspect process-owned raw asset bytes.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AssetBytesError {
    /// The semantic asset exists, but this process retains no bytes for it.
    BytesNotRetained {
        /// Accepted semantic asset identity without retained bytes.
        asset: AcceptedIdentity,
        /// Current accepted revision that admits the asset identity.
        revision: RevisionIdentity,
    },
    /// Session has no accepted semantic revision.
    NoAcceptedRevision,
    /// Caller names an accepted revision that is no longer current.
    StaleBase {
        /// Current accepted revision that rejected the stale request.
        current: RevisionIdentity,
    },
    /// Requested identity exists but is not a semantic asset.
    TargetNotAsset {
        /// Actual semantic kind owned by the requested identity.
        actual: SemanticIdentityKind,
        /// Accepted revision inspected without mutation.
        revision: RevisionIdentity,
        /// Existing non-asset semantic identity.
        target: AcceptedIdentity,
    },
    /// Requested accepted identity is absent from the named revision.
    TargetNotFound {
        /// Accepted revision inspected without mutation.
        revision: RevisionIdentity,
        /// Requested identity absent from that revision.
        target: AcceptedIdentity,
    },
}

/// Result of retaining bytes for one already-accepted semantic asset.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AssetBytesRetention {
    /// The asset already owned bytes; its original sequence was preserved.
    AlreadyRetained {
        /// Accepted semantic asset identity whose bytes were left unchanged.
        asset: AcceptedIdentity,
        /// Current accepted revision that still admits the asset identity.
        revision: RevisionIdentity,
    },
    /// The asset had no retained bytes and now owns this exact byte sequence.
    Retained {
        /// Accepted semantic asset identity that owns the bytes.
        asset: AcceptedIdentity,
        /// Complete retained byte count.
        byte_count: usize,
        /// Current accepted revision that admitted the asset identity.
        revision: RevisionIdentity,
    },
}

/// Mutable application state owned by one active Atrament process.
#[derive(Default)]
pub struct SessionApplication {
    asset_bytes: BTreeMap<AcceptedIdentity, Vec<u8>>,
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

    /// Borrow raw bytes retained for one current accepted semantic asset.
    ///
    /// # Errors
    ///
    /// Returns a typed error when the revision is unavailable or stale, when
    /// the target is missing or not an asset, or when that asset has no
    /// retained byte sequence in this process.
    pub fn asset_bytes(
        &self,
        revision: RevisionIdentity,
        asset: AcceptedIdentity,
    ) -> Result<&[u8], AssetBytesError> {
        self.validate_asset_identity(revision, asset)?;
        self.asset_bytes.get(&asset).map(Vec::as_slice).ok_or(
            AssetBytesError::BytesNotRetained { asset, revision },
        )
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

    /// Retain already-validated raw bytes for one accepted semantic asset.
    ///
    /// Existing bytes are immutable for that accepted asset identity. Repeated
    /// retention preserves the original bytes instead of changing render input
    /// behind a stable semantic reference. This is not a media-ingestion or
    /// media-format validation capability.
    ///
    /// # Errors
    ///
    /// Returns a typed error when the revision is unavailable or stale, when
    /// the target is missing or not an asset, or when no accepted revision
    /// exists.
    pub fn retain_asset_bytes(
        &mut self,
        revision: RevisionIdentity,
        asset: AcceptedIdentity,
        bytes: Vec<u8>,
    ) -> Result<AssetBytesRetention, AssetBytesError> {
        self.validate_asset_identity(revision, asset)?;
        match self.asset_bytes.entry(asset) {
            Entry::Occupied(_) => {
                Ok(AssetBytesRetention::AlreadyRetained { asset, revision })
            },
            Entry::Vacant(entry) => {
                let byte_count = bytes.len();
                let _bytes = entry.insert(bytes);
                Ok(AssetBytesRetention::Retained {
                    asset,
                    byte_count,
                    revision,
                })
            },
        }
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

    fn validate_asset_identity(
        &self,
        revision: RevisionIdentity,
        asset: AcceptedIdentity,
    ) -> Result<(), AssetBytesError> {
        match self.semantic.inspect_identity_kind(revision, asset) {
            IdentityKindInspectOutcome::Inspected {
                kind: SemanticIdentityKind::Asset,
                ..
            } => Ok(()),
            IdentityKindInspectOutcome::Inspected {
                kind: actual,
                revision: inspected_revision,
                target,
            } => Err(AssetBytesError::TargetNotAsset {
                actual,
                revision: inspected_revision,
                target,
            }),
            IdentityKindInspectOutcome::NoAcceptedRevision => {
                Err(AssetBytesError::NoAcceptedRevision)
            },
            IdentityKindInspectOutcome::StaleBase { current } => {
                Err(AssetBytesError::StaleBase { current })
            },
            IdentityKindInspectOutcome::TargetNotFound {
                revision: inspected_revision,
                target,
            } => Err(AssetBytesError::TargetNotFound {
                revision: inspected_revision,
                target,
            }),
        }
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
