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
//   - Active-session accepted semantic revision, promotion, and history.
// - Must-Not:
//   - Persist notebooks, parse transport text, perform layout, or reuse IDs.
// - Allows:
//   - Inputs: Complete candidate semantic notebook values from prior
//     validation.
//   - Outputs: Accepted revision, identity mapping, and history traversal.
//   - Side effects: Atomic process-memory revision and history mutation only.
// - Split-When:
//   - Semantic command Apply requires independently bounded transaction state.
// - Merge-When:
//   - One application authority subsumes all accepted semantic transactions.
// - Summary:
//   - Owns accepted semantic authority and in-memory Undo/Redo state.
// - Description:
//   - Validates candidate identity graph before allocating accepted authority.
// - Usage:
//   - Own one service instance for one active disposable Atrament session.
// - Defaults:
//   - Starts without an accepted revision and never persists session state.
//

//! In-memory accepted semantic authority and history traversal.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::slice::from_ref;

use atrament_mathematics_source::analyze;
use atrament_semantic_command_graph::{
    BoundedDependencyRequirementsError, CommandDependencyNode,
    CommandGraphLimitError, CommandGraphLimits, CommandGraphSize,
    DependencyRequirementsError, DependencySummaryError,
    dependency_selection_requirements,
    dependency_selection_requirements_bounded, dependency_selection_summary,
    validate_command_graph,
};
use atrament_semantic_notebook::{
    AcceptedIdentity, AcceptedRevision, Asset, Block, BlockContent,
    CandidateIdentity, Constraint, ConstraintKind, Figure, Flow, Formula,
    FormulaMode,
    IdentityAllocator, IdentityExhausted, InlineSpan, List, ListItem, Notebook,
    OutputProfile, Page, PaperProfile, Provenance, ProvenanceKind,
    SemanticBlockKind, SemanticIdentityDescriptor, SemanticIdentityKind, Style,
    Table, TableCell,
    TableCellSpan,
    TableGridError, TableRow,
    TableRowRole,
    semantic_identity_descriptor,
};
use atrament_semantic_notebook_port::{
    AcceptanceOutcome, CANDIDATE_BLOCK_NESTING_LIMIT, CandidateGraphError,
    CandidateReferenceKind, CommandBehaviorVersion,
    CommandCapabilityCompatibilityOutcome, CommandFamilyAdmissionOutcome,
    CommandFamilyCapability, CommandResourceLimits, CommandTargetMaterial,
    CommandTargetMaterialOutcome, CommandTargetPreconditionOutcome,
    CommandTargetPreconditions, DirectEditBatchApplyOutcome,
    DirectEditBatchCommand, DirectEditBatchCommandPrediction,
    DirectEditBatchCommandRejection,
    DirectEditBatchGraphLimitsOutcome, DirectEditBatchGraphSizeOutcome,
    DirectEditBatchProposal,
    DirectEditBatchSelectionBoundedOutcome as BoundedSelectionOutcome,
    DirectEditBatchSelectionRequirementsOutcome as BatchSelectionOutcome,
    DirectEditBatchSelectionSummaryOutcome as BatchSelectionSummaryOutcome,
    DirectEditBatchSimulationOutcome, DirectEditChangePreviewOutcome,
    DirectEditDerivedAuthority, DirectEditEffectClass, DirectEditImpactScope,
    DirectEditImpactSeed, DirectEditProposal, DirectEditProposalOutcome,
    DirectEditSemanticChange, DirectEditSimulationOutcome,
    EditableSemanticValue, EditableSemanticValueKind,
    EditableValuePreconditionOutcome, FormulaEditOutcome,
    HistoryAvailability, HistoryAvailabilityOutcome, HistoryDirection,
    HistoryTraversalOutcome, IdentityAncestryCompleteness,
    IdentityAncestryEntry, IdentityAncestryInspectOutcome,
    IdentityInspectOutcome, IdentityKindInspectOutcome, IdentityMapping,
    IdentityOwnerExpectation, IdentityPrecondition,
    IdentityPreconditionOutcome, PageProfileEditOutcome,
    SemanticCommandCapabilitySnapshot, SemanticCommandFamily,
    SemanticNotebookHistory, SemanticNotebookSession,
    TableCellSpanEditOutcome, TableRowRoleEditOutcome, TextEditOutcome,
};

type DirectEditMaterialKey = (AcceptedIdentity, SemanticCommandFamily);

#[derive(Default)]
struct DirectEditBatchIndex {
    impacts: BTreeMap<DirectEditMaterialKey, DirectEditImpactScope>,
    materials: BTreeMap<DirectEditMaterialKey, CommandTargetMaterial>,
    table_by_cell: BTreeMap<AcceptedIdentity, AcceptedIdentity>,
    table_overlays: BTreeMap<AcceptedIdentity, Table<AcceptedIdentity>>,
}

#[derive(Clone, Copy)]
struct DirectEditBatchIndexRequest<'request> {
    families_by_target:
        &'request BTreeMap<AcceptedIdentity, BTreeSet<SemanticCommandFamily>>,
    material_count: usize,
}

impl DirectEditBatchIndexRequest<'_> {
    fn contains_family(
        self,
        target: AcceptedIdentity,
        family: SemanticCommandFamily,
    ) -> bool {
        self.families_by_target
            .get(&target)
            .is_some_and(|families| families.contains(&family))
    }

    fn contains_target(self, target: AcceptedIdentity) -> bool {
        self.families_by_target.contains_key(&target)
    }

    fn is_complete(self, index: &DirectEditBatchIndex) -> bool {
        index.materials.len() == self.material_count
    }
}

#[derive(Clone, Copy)]
struct DirectEditBatchBlockContext {
    block: AcceptedIdentity,
    flow: AcceptedIdentity,
    page: AcceptedIdentity,
}

#[derive(Clone, Copy)]
struct DirectEditBatchTableContext<'notebook> {
    block: AcceptedIdentity,
    flow: AcceptedIdentity,
    page: AcceptedIdentity,
    table: &'notebook Table<AcceptedIdentity>,
}

#[derive(Clone, Copy)]
enum DirectEditBatchIndexFrame<'notebook> {
    Blocks {
        blocks: &'notebook [Block<AcceptedIdentity>],
        flow: AcceptedIdentity,
        page: AcceptedIdentity,
    },
    ListItems {
        flow: AcceptedIdentity,
        items: &'notebook [ListItem<AcceptedIdentity>],
        page: AcceptedIdentity,
    },
    TableCells {
        cells: &'notebook [TableCell<AcceptedIdentity>],
        context: DirectEditBatchTableContext<'notebook>,
        row: AcceptedIdentity,
    },
    TableRows {
        block: AcceptedIdentity,
        flow: AcceptedIdentity,
        page: AcceptedIdentity,
        rows: &'notebook [TableRow<AcceptedIdentity>],
        table: &'notebook Table<AcceptedIdentity>,
    },
}

#[derive(Clone, Copy)]
struct DirectEditBatchMaterialMetadata {
    descriptor: SemanticIdentityDescriptor<AcceptedIdentity>,
    indexed: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum DirectEditBatchAuthorityError {
    CapabilityMismatch {
        current: CommandBehaviorVersion,
        expected: CommandBehaviorVersion,
    },
    NoAcceptedRevision,
    StaleBase {
        current: atrament_semantic_notebook::RevisionIdentity,
    },
}

#[derive(Debug, Default)]
struct CandidateGraph {
    owners: Vec<CandidateIdentity>,
    references: Vec<(CandidateIdentity, CandidateReferenceKind)>,
    seen: BTreeMap<CandidateIdentity, CandidateReferenceKind>,
}

#[derive(Clone, Copy)]
enum CandidateGraphFrame<'candidate> {
    Blocks {
        blocks: &'candidate [Block<CandidateIdentity>],
        depth: usize,
    },
    ListItems {
        child_depth: usize,
        items: &'candidate [ListItem<CandidateIdentity>],
    },
    TableCells {
        cells: &'candidate [TableCell<CandidateIdentity>],
        child_depth: usize,
    },
    TableRows {
        child_depth: usize,
        rows: &'candidate [TableRow<CandidateIdentity>],
    },
}

struct DirectEditSimulation {
    before: Option<EditableSemanticValue>,
    outcome: DirectEditSimulationOutcome,
}

#[derive(Clone, Copy)]
struct DirectEditCommandGraphNode<'command, CommandIdentity> {
    command: &'command DirectEditBatchCommand<CommandIdentity>,
}

impl<CommandIdentity> CommandDependencyNode
    for DirectEditCommandGraphNode<'_, CommandIdentity>
where
    CommandIdentity: Ord,
{
    type Identity = CommandIdentity;

    fn dependencies(&self) -> &[Self::Identity] {
        &self.command.dependencies
    }

    fn id(&self) -> &Self::Identity {
        &self.command.id
    }
}

impl CandidateGraph {
    fn finish(self) -> Result<Vec<CandidateIdentity>, CandidateGraphError> {
        for (reference, expected) in self.references {
            let Some(observed) = self.seen.get(&reference) else {
                return Err(CandidateGraphError::MissingReference {
                    candidate: reference,
                });
            };
            if expected != CandidateReferenceKind::Semantic
                && *observed != expected
            {
                return Err(CandidateGraphError::ReferenceKindMismatch {
                    candidate: reference,
                    expected,
                });
            }
        }
        Ok(self.owners)
    }

    fn reference(
        &mut self,
        identity: Option<CandidateIdentity>,
        kind: CandidateReferenceKind,
    ) {
        if let Some(reference) = identity {
            self.references.push((reference, kind));
        }
    }

    fn register(
        &mut self,
        identity: CandidateIdentity,
        kind: CandidateReferenceKind,
    ) -> Result<(), CandidateGraphError> {
        let previous = self.seen.insert(identity, kind);
        if previous.is_some() {
            return Err(CandidateGraphError::Duplicate { candidate: identity });
        }
        self.owners.push(identity);
        Ok(())
    }
}

/// Process-local accepted semantic notebook authority for one active session.
#[derive(Default)]
pub struct SemanticNotebookSessionService {
    current: Option<AcceptedRevision>,
    identities: IdentityAllocator,
    redo_notebooks: Vec<Notebook<AcceptedIdentity>>,
    undo_notebooks: Vec<Notebook<AcceptedIdentity>>,
}

impl fmt::Debug for SemanticNotebookSessionService {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SemanticNotebookSessionService")
            .finish_non_exhaustive()
    }
}

impl SemanticNotebookSession for SemanticNotebookSessionService {
    fn accept(
        &mut self,
        candidate: Notebook<CandidateIdentity>,
    ) -> AcceptanceOutcome {
        let owners = match candidate_identities(&candidate) {
            Ok(owners) => owners,
            Err(reason) => {
                discard_candidate_notebook(candidate);
                return AcceptanceOutcome::InvalidCandidate { reason };
            },
        };
        let (identity_map, mapping) = match self.allocate_mapping(&owners) {
            Ok(mapping) => mapping,
            Err(sequence) => {
                return AcceptanceOutcome::IdentityExhausted { sequence };
            },
        };
        let notebook = match accept_notebook(candidate, &identity_map) {
            Ok(notebook) => notebook,
            Err(reason) => {
                return AcceptanceOutcome::InvalidCandidate { reason };
            },
        };
        let revision = match self.identities.allocate_revision() {
            Ok(revision) => revision,
            Err(sequence) => {
                return AcceptanceOutcome::IdentityExhausted { sequence };
            },
        };
        if let Some(current) = self.current.take() {
            self.undo_notebooks.push(current.notebook);
        }
        self.redo_notebooks.clear();
        self.current = Some(AcceptedRevision { id: revision, notebook });
        AcceptanceOutcome::Accepted { mapping, revision }
    }

    fn apply_direct_edit_batch<CommandIdentity>(
        &mut self,
        batch: DirectEditBatchProposal<CommandIdentity>,
    ) -> DirectEditBatchApplyOutcome<CommandIdentity>
    where
        CommandIdentity: Clone + Ord,
    {
        let simulation = self.simulate_direct_edit_batch(batch);
        self.apply_direct_edit_batch_simulation(simulation)
    }

    fn apply_direct_edit_batch_bounded<CommandIdentity>(
        &mut self,
        batch: DirectEditBatchProposal<CommandIdentity>,
        limits: CommandGraphLimits,
    ) -> DirectEditBatchApplyOutcome<CommandIdentity>
    where
        CommandIdentity: Clone + Ord,
    {
        let simulation = self.simulate_direct_edit_batch_bounded(batch, limits);
        self.apply_direct_edit_batch_simulation(simulation)
    }

    fn check_command_capability_compatibility(
        &self,
        expected: CommandBehaviorVersion,
    ) -> CommandCapabilityCompatibilityOutcome {
        let snapshot = self.command_capability_snapshot();
        if snapshot.behavior_version != expected {
            return CommandCapabilityCompatibilityOutcome::Mismatch {
                current: snapshot.behavior_version,
                expected,
            };
        }
        CommandCapabilityCompatibilityOutcome::Compatible { snapshot }
    }

    fn check_command_family_admission(
        &self,
        revision: atrament_semantic_notebook::RevisionIdentity,
        target: AcceptedIdentity,
        requested: SemanticCommandFamily,
    ) -> CommandFamilyAdmissionOutcome {
        let material = match self.command_target_material_for_family(
            revision, target, requested,
        ) {
            CommandTargetMaterialOutcome::NoAcceptedRevision => {
                return CommandFamilyAdmissionOutcome::NoAcceptedRevision;
            },
            CommandTargetMaterialOutcome::Prepared { material } => material,
            CommandTargetMaterialOutcome::StaleBase { current } => {
                return CommandFamilyAdmissionOutcome::StaleBase { current };
            },
            CommandTargetMaterialOutcome::TargetNotFound {
                revision: inspected_revision,
                target: missing_target,
            } => {
                return CommandFamilyAdmissionOutcome::TargetNotFound {
                    revision: inspected_revision,
                    target: missing_target,
                };
            },
        };
        if material.direct_edit_family != Some(requested) {
            return CommandFamilyAdmissionOutcome::FamilyNotExecutable {
                available: material.direct_edit_family,
                requested,
                revision,
                target,
            };
        }
        CommandFamilyAdmissionOutcome::Admitted { material }
    }

    fn check_command_target_preconditions(
        &self,
        revision: atrament_semantic_notebook::RevisionIdentity,
        target: AcceptedIdentity,
        preconditions: CommandTargetPreconditions,
    ) -> CommandTargetPreconditionOutcome {
        let material = match self.command_target_material_for_family(
            revision,
            target,
            preconditions.requested_family,
        ) {
            CommandTargetMaterialOutcome::NoAcceptedRevision => {
                return CommandTargetPreconditionOutcome::NoAcceptedRevision;
            },
            CommandTargetMaterialOutcome::Prepared { material } => material,
            CommandTargetMaterialOutcome::StaleBase { current } => {
                return CommandTargetPreconditionOutcome::StaleBase { current };
            },
            CommandTargetMaterialOutcome::TargetNotFound {
                revision: inspected_revision,
                target: missing_target,
            } => {
                return CommandTargetPreconditionOutcome::TargetNotFound {
                    revision: inspected_revision,
                    target: missing_target,
                };
            },
        };
        check_command_target_preconditions_material(material, &preconditions)
    }

    fn check_editable_value_precondition(
        &self,
        revision: atrament_semantic_notebook::RevisionIdentity,
        target: AcceptedIdentity,
        expected: EditableSemanticValue,
    ) -> EditableValuePreconditionOutcome {
        let family = direct_edit_family(&expected);
        let material = match self.command_target_material_for_family(
            revision, target, family,
        ) {
            CommandTargetMaterialOutcome::NoAcceptedRevision => {
                return EditableValuePreconditionOutcome::NoAcceptedRevision;
            },
            CommandTargetMaterialOutcome::Prepared { material } => material,
            CommandTargetMaterialOutcome::StaleBase { current } => {
                return EditableValuePreconditionOutcome::StaleBase { current };
            },
            CommandTargetMaterialOutcome::TargetNotFound {
                revision: inspected_revision,
                target: missing_target,
            } => {
                return EditableValuePreconditionOutcome::TargetNotFound {
                    revision: inspected_revision,
                    target: missing_target,
                };
            },
        };
        let Some(actual) = material.editable_value else {
            return EditableValuePreconditionOutcome::TargetNotEditableValue {
                kind: material.descriptor.kind,
                revision,
                target,
            };
        };
        if actual != expected {
            return EditableValuePreconditionOutcome::ValueMismatch {
                actual,
                expected,
                revision,
                target,
            };
        }
        EditableValuePreconditionOutcome::Satisfied { actual, revision, target }
    }

    fn check_identity_precondition(
        &self,
        revision: atrament_semantic_notebook::RevisionIdentity,
        target: AcceptedIdentity,
        precondition: IdentityPrecondition,
    ) -> IdentityPreconditionOutcome {
        let descriptor = match self.inspect_identity(revision, target) {
            IdentityInspectOutcome::Inspected { descriptor, .. } => descriptor,
            IdentityInspectOutcome::NoAcceptedRevision => {
                return IdentityPreconditionOutcome::NoAcceptedRevision;
            },
            IdentityInspectOutcome::StaleBase { current } => {
                return IdentityPreconditionOutcome::StaleBase { current };
            },
            IdentityInspectOutcome::TargetNotFound {
                revision: inspected_revision,
                target: missing_target,
            } => {
                return IdentityPreconditionOutcome::TargetNotFound {
                    revision: inspected_revision,
                    target: missing_target,
                };
            },
        };
        if let Some(expected) = precondition.expected_kind
            && descriptor.kind != expected
        {
            return IdentityPreconditionOutcome::KindMismatch {
                actual: descriptor.kind,
                expected,
                revision,
                target,
            };
        }
        let owner_matches = match precondition.expected_owner {
            IdentityOwnerExpectation::Any => true,
            IdentityOwnerExpectation::Direct(expected) => {
                descriptor.owner == Some(expected)
            },
            IdentityOwnerExpectation::Root => descriptor.owner.is_none(),
        };
        if !owner_matches {
            return IdentityPreconditionOutcome::OwnerMismatch {
                actual: descriptor.owner,
                expected: precondition.expected_owner,
                revision,
                target,
            };
        }
        IdentityPreconditionOutcome::Satisfied {
            descriptor,
            revision,
            target,
        }
    }

    fn command_capability_snapshot(&self) -> SemanticCommandCapabilitySnapshot {
        const VERSION: CommandBehaviorVersion = CommandBehaviorVersion(53);
        const FAMILY_CAPABILITIES: [CommandFamilyCapability; 7] = [
            CommandFamilyCapability {
                behavior_version: CommandBehaviorVersion(1),
                family: SemanticCommandFamily::AssetReference,
            },
            CommandFamilyCapability {
                behavior_version: CommandBehaviorVersion(3),
                family: SemanticCommandFamily::DocumentConstraint,
            },
            CommandFamilyCapability {
                behavior_version: CommandBehaviorVersion(1),
                family: SemanticCommandFamily::OrderingAndGrouping,
            },
            CommandFamilyCapability {
                behavior_version: CommandBehaviorVersion(2),
                family: SemanticCommandFamily::Provenance,
            },
            CommandFamilyCapability {
                behavior_version: CommandBehaviorVersion(46),
                family: SemanticCommandFamily::StructuredContent,
            },
            CommandFamilyCapability {
                behavior_version: CommandBehaviorVersion(2),
                family: SemanticCommandFamily::StyleRole,
            },
            CommandFamilyCapability {
                behavior_version: CommandBehaviorVersion(1),
                family: SemanticCommandFamily::TextContent,
            },
        ];
        SemanticCommandCapabilitySnapshot {
            admitted_applications: &[],
            behavior_version: VERSION,
            family_capabilities: &FAMILY_CAPABILITIES,
            normalization_version: None,
            protocol_versions: &[],
            resource_limits: CommandResourceLimits {
                commands_per_batch: None,
                dependency_edges: None,
                envelope_bytes: None,
                readable_context_bytes: None,
                writable_targets: None,
            },
            typed_result_version: VERSION,
        }
    }

    fn command_target_material(
        &self,
        revision: atrament_semantic_notebook::RevisionIdentity,
        target: AcceptedIdentity,
    ) -> CommandTargetMaterialOutcome {
        let Some(current) = self.current.as_ref() else {
            return CommandTargetMaterialOutcome::NoAcceptedRevision;
        };
        if current.id != revision {
            return CommandTargetMaterialOutcome::StaleBase {
                current: current.id,
            };
        }
        command_target_material_from_notebook(
            &current.notebook,
            revision,
            target,
        )
    }

    fn command_target_material_for_family(
        &self,
        revision: atrament_semantic_notebook::RevisionIdentity,
        target: AcceptedIdentity,
        family: SemanticCommandFamily,
    ) -> CommandTargetMaterialOutcome {
        let Some(current) = self.current.as_ref() else {
            return CommandTargetMaterialOutcome::NoAcceptedRevision;
        };
        if current.id != revision {
            return CommandTargetMaterialOutcome::StaleBase {
                current: current.id,
            };
        }
        command_target_material_for_family_from_notebook(
            &current.notebook,
            revision,
            target,
            family,
        )
    }

    fn current(&self) -> Option<&AcceptedRevision> {
        self.current.as_ref()
    }

    fn direct_edit_batch_graph_limits<CommandIdentity>(
        &self,
        batch: &DirectEditBatchProposal<CommandIdentity>,
        limits: CommandGraphLimits,
    ) -> DirectEditBatchGraphLimitsOutcome {
        let current = match direct_edit_batch_authority(self, batch) {
            Ok(current) => current,
            Err(DirectEditBatchAuthorityError::CapabilityMismatch {
                current,
                expected,
            }) => {
                return DirectEditBatchGraphLimitsOutcome::CapabilityMismatch {
                    current,
                    expected,
                };
            },
            Err(DirectEditBatchAuthorityError::NoAcceptedRevision) => {
                return DirectEditBatchGraphLimitsOutcome::NoAcceptedRevision;
            },
            Err(DirectEditBatchAuthorityError::StaleBase { current }) => {
                return DirectEditBatchGraphLimitsOutcome::StaleBase {
                    current,
                };
            },
        };
        match direct_edit_batch_graph_limits_from_commands(
            &batch.commands,
            limits,
        ) {
            Ok(size) => DirectEditBatchGraphLimitsOutcome::Admitted {
                revision: current.id,
                size,
            },
            Err(reason) => {
                DirectEditBatchGraphLimitsOutcome::Rejected { reason }
            },
        }
    }

    fn direct_edit_batch_graph_size<CommandIdentity>(
        &self,
        batch: &DirectEditBatchProposal<CommandIdentity>,
    ) -> DirectEditBatchGraphSizeOutcome {
        let current = match direct_edit_batch_authority(self, batch) {
            Ok(current) => current,
            Err(DirectEditBatchAuthorityError::CapabilityMismatch {
                current,
                expected,
            }) => {
                return DirectEditBatchGraphSizeOutcome::CapabilityMismatch {
                    current,
                    expected,
                };
            },
            Err(DirectEditBatchAuthorityError::NoAcceptedRevision) => {
                return DirectEditBatchGraphSizeOutcome::NoAcceptedRevision;
            },
            Err(DirectEditBatchAuthorityError::StaleBase { current }) => {
                return DirectEditBatchGraphSizeOutcome::StaleBase { current };
            },
        };
        direct_edit_batch_graph_size_from_commands(&batch.commands).map_or(
            DirectEditBatchGraphSizeOutcome::SizeOverflow,
            |size| DirectEditBatchGraphSizeOutcome::Sized {
                revision: current.id,
                size,
            },
        )
    }

    fn direct_edit_batch_selection_requirements<CommandIdentity>(
        &self,
        batch: &DirectEditBatchProposal<CommandIdentity>,
        selected: &BTreeSet<CommandIdentity>,
    ) -> BatchSelectionOutcome<CommandIdentity>
    where
        CommandIdentity: Clone + Ord,
    {
        let current = match direct_edit_batch_authority(self, batch) {
            Ok(current) => current,
            Err(DirectEditBatchAuthorityError::CapabilityMismatch {
                current,
                expected,
            }) => {
                return BatchSelectionOutcome::CapabilityMismatch {
                    current,
                    expected,
                };
            },
            Err(DirectEditBatchAuthorityError::NoAcceptedRevision) => {
                return BatchSelectionOutcome::NoAcceptedRevision;
            },
            Err(DirectEditBatchAuthorityError::StaleBase { current }) => {
                return BatchSelectionOutcome::StaleBase { current };
            },
        };
        let nodes = direct_edit_command_graph_nodes(&batch.commands);
        match dependency_selection_requirements(&nodes, selected) {
            Ok(missing) => BatchSelectionOutcome::Requirements {
                missing,
                revision: current.id,
            },
            Err(DependencyRequirementsError::Graph { reason }) => {
                BatchSelectionOutcome::DependencyGraphRejected { reason }
            },
            Err(DependencyRequirementsError::UnknownSelection { command }) => {
                BatchSelectionOutcome::UnknownSelection { command }
            },
        }
    }

    fn direct_edit_batch_selection_requirements_bounded<CommandIdentity>(
        &self,
        batch: &DirectEditBatchProposal<CommandIdentity>,
        selected: &BTreeSet<CommandIdentity>,
        maximum_missing_edges: usize,
    ) -> BoundedSelectionOutcome<CommandIdentity>
    where
        CommandIdentity: Clone + Ord,
    {
        let current = match direct_edit_batch_authority(self, batch) {
            Ok(current) => current,
            Err(DirectEditBatchAuthorityError::CapabilityMismatch {
                current,
                expected,
            }) => {
                return BoundedSelectionOutcome::CapabilityMismatch {
                    current,
                    expected,
                };
            },
            Err(DirectEditBatchAuthorityError::NoAcceptedRevision) => {
                return BoundedSelectionOutcome::NoAcceptedRevision;
            },
            Err(DirectEditBatchAuthorityError::StaleBase { current }) => {
                return BoundedSelectionOutcome::StaleBase { current };
            },
        };
        let nodes = direct_edit_command_graph_nodes(&batch.commands);
        match dependency_selection_requirements_bounded(
            &nodes,
            selected,
            maximum_missing_edges,
        ) {
            Ok(missing) => BoundedSelectionOutcome::Requirements {
                missing,
                revision: current.id,
            },
            Err(BoundedDependencyRequirementsError::Graph { reason }) => {
                BoundedSelectionOutcome::DependencyGraphRejected { reason }
            },
            Err(
                BoundedDependencyRequirementsError::RequirementCountExceeded {
                    actual,
                    limit,
                },
            ) => BoundedSelectionOutcome::RequirementCountExceeded {
                actual,
                limit,
            },
            Err(
                BoundedDependencyRequirementsError::RequirementCountOverflow,
            ) => BoundedSelectionOutcome::RequirementCountOverflow,
            Err(BoundedDependencyRequirementsError::UnknownSelection {
                command,
            }) => BoundedSelectionOutcome::UnknownSelection { command },
        }
    }

    fn direct_edit_batch_selection_summary<CommandIdentity>(
        &self,
        batch: &DirectEditBatchProposal<CommandIdentity>,
        selected: &BTreeSet<CommandIdentity>,
    ) -> BatchSelectionSummaryOutcome<CommandIdentity>
    where
        CommandIdentity: Clone + Ord,
    {
        let current = match direct_edit_batch_authority(self, batch) {
            Ok(current) => current,
            Err(DirectEditBatchAuthorityError::CapabilityMismatch {
                current,
                expected,
            }) => {
                return BatchSelectionSummaryOutcome::CapabilityMismatch {
                    current,
                    expected,
                };
            },
            Err(DirectEditBatchAuthorityError::NoAcceptedRevision) => {
                return BatchSelectionSummaryOutcome::NoAcceptedRevision;
            },
            Err(DirectEditBatchAuthorityError::StaleBase { current }) => {
                return BatchSelectionSummaryOutcome::StaleBase { current };
            },
        };
        let nodes = direct_edit_command_graph_nodes(&batch.commands);
        match dependency_selection_summary(&nodes, selected) {
            Ok(summary) => BatchSelectionSummaryOutcome::Summarized {
                revision: current.id,
                summary,
            },
            Err(DependencySummaryError::Graph { reason }) => {
                BatchSelectionSummaryOutcome::DependencyGraphRejected { reason }
            },
            Err(DependencySummaryError::RequirementCountOverflow) => {
                BatchSelectionSummaryOutcome::RequirementCountOverflow
            },
            Err(DependencySummaryError::UnknownSelection { command }) => {
                BatchSelectionSummaryOutcome::UnknownSelection { command }
            },
        }
    }

    fn inspect_identity(
        &self,
        revision: atrament_semantic_notebook::RevisionIdentity,
        target: AcceptedIdentity,
    ) -> IdentityInspectOutcome {
        let Some(current) = self.current.as_ref() else {
            return IdentityInspectOutcome::NoAcceptedRevision;
        };
        if current.id != revision {
            return IdentityInspectOutcome::StaleBase { current: current.id };
        }
        let Some(descriptor) =
            semantic_identity_descriptor(&current.notebook, target)
        else {
            return IdentityInspectOutcome::TargetNotFound { revision, target };
        };
        IdentityInspectOutcome::Inspected {
            descriptor,
            revision,
            target,
        }
    }

    fn inspect_identity_ancestry_bounded(
        &self,
        revision: atrament_semantic_notebook::RevisionIdentity,
        target: AcceptedIdentity,
        maximum_results: usize,
    ) -> IdentityAncestryInspectOutcome {
        let Some(current) = self.current.as_ref() else {
            return IdentityAncestryInspectOutcome::NoAcceptedRevision;
        };
        if current.id != revision {
            return IdentityAncestryInspectOutcome::StaleBase {
                current: current.id,
            };
        }
        let mut identity = target;
        let Some(mut descriptor) =
            semantic_identity_descriptor(&current.notebook, identity)
        else {
            return IdentityAncestryInspectOutcome::TargetNotFound {
                revision,
                target,
            };
        };
        let mut entries = Vec::new();
        loop {
            if entries.len() == maximum_results {
                return IdentityAncestryInspectOutcome::Inspected {
                    completeness: IdentityAncestryCompleteness::Incomplete {
                        remaining_identity: identity,
                    },
                    entries,
                    revision,
                    target,
                };
            }
            entries.push(IdentityAncestryEntry { descriptor, identity });
            let Some(owner) = descriptor.owner else {
                return IdentityAncestryInspectOutcome::Inspected {
                    completeness: IdentityAncestryCompleteness::Complete,
                    entries,
                    revision,
                    target,
                };
            };
            identity = owner;
            let Some(owner_descriptor) =
                semantic_identity_descriptor(&current.notebook, identity)
            else {
                return IdentityAncestryInspectOutcome::TargetNotFound {
                    revision,
                    target: identity,
                };
            };
            descriptor = owner_descriptor;
        }
    }

    fn inspect_identity_kind(
        &self,
        revision: atrament_semantic_notebook::RevisionIdentity,
        target: AcceptedIdentity,
    ) -> IdentityKindInspectOutcome {
        match self.inspect_identity(revision, target) {
            IdentityInspectOutcome::Inspected {
                descriptor,
                revision: inspected_revision,
                target: inspected_target,
            } => IdentityKindInspectOutcome::Inspected {
                kind: descriptor.kind,
                revision: inspected_revision,
                target: inspected_target,
            },
            IdentityInspectOutcome::NoAcceptedRevision => {
                IdentityKindInspectOutcome::NoAcceptedRevision
            },
            IdentityInspectOutcome::StaleBase { current } => {
                IdentityKindInspectOutcome::StaleBase { current }
            },
            IdentityInspectOutcome::TargetNotFound {
                revision: inspected_revision,
                target: missing_target,
            } => IdentityKindInspectOutcome::TargetNotFound {
                revision: inspected_revision,
                target: missing_target,
            },
        }
    }

    fn preview_direct_edit_changes(
        &self,
        revision: atrament_semantic_notebook::RevisionIdentity,
        target: AcceptedIdentity,
        requested: EditableSemanticValue,
    ) -> DirectEditChangePreviewOutcome {
        let requested_family = direct_edit_family(&requested);
        let simulation = simulate_direct_edit_material_in_notebook(
            self.current.as_ref().map(|current| &current.notebook),
            self.command_target_material_for_family(
                revision, target, requested_family,
            ),
            requested,
        );
        match (simulation.before, simulation.outcome) {
            (
                Some(before),
                DirectEditSimulationOutcome::Applicable {
                    family,
                    requested: simulated_requested,
                    revision: simulated_revision,
                    target: simulated_target,
                },
            ) => {
                let change = DirectEditSemanticChange {
                    after: simulated_requested,
                    before,
                    family,
                    target: simulated_target,
                };
                let Some(current) = self.current.as_ref() else {
                    return DirectEditChangePreviewOutcome::Rejected {
                        outcome: Box::new(
                            DirectEditSimulationOutcome::NoAcceptedRevision,
                        ),
                    };
                };
                let impact_seeds = direct_edit_impact_seeds(
                    &current.notebook,
                    from_ref(&change),
                );
                DirectEditChangePreviewOutcome::Predicted {
                    changes: vec![change],
                    effect: DirectEditEffectClass::Mutation,
                    impact_seeds,
                    revision: simulated_revision,
                }
            },
            (
                _,
                DirectEditSimulationOutcome::NoOp {
                    revision: simulated_revision,
                    ..
                },
            ) => DirectEditChangePreviewOutcome::Predicted {
                changes: Vec::new(),
                effect: DirectEditEffectClass::NoOp,
                impact_seeds: Vec::new(),
                revision: simulated_revision,
            },
            (_, outcome) => DirectEditChangePreviewOutcome::Rejected {
                outcome: Box::new(outcome),
            },
        }
    }

    fn replace_formula(
        &mut self,
        base: atrament_semantic_notebook::RevisionIdentity,
        target: AcceptedIdentity,
        mode: FormulaMode,
        source: String,
    ) -> FormulaEditOutcome {
        let simulation = self.simulate_direct_edit(
            base,
            target,
            EditableSemanticValue::Formula { mode, source },
        );
        let (replacement_mode, replacement_source) = match simulation {
            DirectEditSimulationOutcome::Applicable {
                requested:
                    EditableSemanticValue::Formula {
                        mode: requested_mode,
                        source: requested_source,
                    },
                ..
            } => (requested_mode, requested_source),
            DirectEditSimulationOutcome::Applicable { .. }
            | DirectEditSimulationOutcome::InvalidAssetReference { .. }
            | DirectEditSimulationOutcome::InvalidPageProfileReference { .. }
            | DirectEditSimulationOutcome::InvalidProvenanceReference { .. }
            | DirectEditSimulationOutcome::InvalidStyleReference { .. }
            | DirectEditSimulationOutcome::InvalidPageProfile { .. }
            | DirectEditSimulationOutcome::InvalidTableGrid { .. }
            | DirectEditSimulationOutcome::TargetNotEditableValue { .. }
            | DirectEditSimulationOutcome::ValueFamilyMismatch { .. } => {
                return FormulaEditOutcome::TargetNotFormula {
                    revision: base,
                    target,
                };
            },
            DirectEditSimulationOutcome::InvalidMathematics {
                reason,
                revision,
                target: simulated_target,
            } => {
                return FormulaEditOutcome::InvalidMathematics {
                    reason,
                    revision,
                    target: simulated_target,
                };
            },
            DirectEditSimulationOutcome::NoAcceptedRevision => {
                return FormulaEditOutcome::NoAcceptedRevision;
            },
            DirectEditSimulationOutcome::NoOp {
                revision,
                target: simulated_target,
                ..
            } => {
                return FormulaEditOutcome::NoOp {
                    revision,
                    target: simulated_target,
                };
            },
            DirectEditSimulationOutcome::StaleBase { current } => {
                return FormulaEditOutcome::StaleBase { current };
            },
            DirectEditSimulationOutcome::TargetNotFound {
                revision,
                target: missing_target,
            } => {
                return FormulaEditOutcome::TargetNotFound {
                    revision,
                    target: missing_target,
                };
            },
            DirectEditSimulationOutcome::UnsupportedMathematics {
                revision,
                target: simulated_target,
            } => {
                return FormulaEditOutcome::UnsupportedMathematics {
                    revision,
                    target: simulated_target,
                };
            },
        };
        let Some(current) = self.current.as_ref() else {
            return FormulaEditOutcome::NoAcceptedRevision;
        };
        let mut notebook = current.notebook.clone();
        if !replace_formula_value(
            &mut notebook,
            target,
            replacement_mode,
            replacement_source,
        ) {
            return FormulaEditOutcome::TargetNotFound {
                revision: current.id,
                target,
            };
        }
        let revision = match self.commit_semantic_edit(notebook) {
            Ok(revision) => revision,
            Err(sequence) => {
                return FormulaEditOutcome::IdentityExhausted { sequence };
            },
        };
        FormulaEditOutcome::Applied { base, revision, target }
    }

    fn replace_page_profile(
        &mut self,
        base: atrament_semantic_notebook::RevisionIdentity,
        target: AcceptedIdentity,
        geometry: atrament_semantic_notebook::PhysicalPageProfile,
    ) -> PageProfileEditOutcome {
        let simulation = self.simulate_direct_edit(
            base,
            target,
            EditableSemanticValue::PageProfile(geometry),
        );
        let replacement = match simulation {
            DirectEditSimulationOutcome::Applicable {
                requested: EditableSemanticValue::PageProfile(requested),
                ..
            } => requested,
            DirectEditSimulationOutcome::Applicable { .. }
            | DirectEditSimulationOutcome::InvalidAssetReference { .. }
            | DirectEditSimulationOutcome::InvalidPageProfileReference { .. }
            | DirectEditSimulationOutcome::InvalidProvenanceReference { .. }
            | DirectEditSimulationOutcome::InvalidStyleReference { .. }
            | DirectEditSimulationOutcome::InvalidMathematics { .. }
            | DirectEditSimulationOutcome::InvalidTableGrid { .. }
            | DirectEditSimulationOutcome::TargetNotEditableValue { .. }
            | DirectEditSimulationOutcome::UnsupportedMathematics { .. }
            | DirectEditSimulationOutcome::ValueFamilyMismatch { .. } => {
                return PageProfileEditOutcome::TargetNotPageProfile {
                    revision: base,
                    target,
                };
            },
            DirectEditSimulationOutcome::InvalidPageProfile {
                reason,
                revision,
                target: simulated_target,
            } => {
                return PageProfileEditOutcome::InvalidProfile {
                    reason,
                    revision,
                    target: simulated_target,
                };
            },
            DirectEditSimulationOutcome::NoAcceptedRevision => {
                return PageProfileEditOutcome::NoAcceptedRevision;
            },
            DirectEditSimulationOutcome::NoOp {
                revision,
                target: simulated_target,
                ..
            } => {
                return PageProfileEditOutcome::NoOp {
                    revision,
                    target: simulated_target,
                };
            },
            DirectEditSimulationOutcome::StaleBase { current } => {
                return PageProfileEditOutcome::StaleBase { current };
            },
            DirectEditSimulationOutcome::TargetNotFound {
                revision,
                target: missing_target,
            } => {
                return PageProfileEditOutcome::TargetNotFound {
                    revision,
                    target: missing_target,
                };
            },
        };
        let Some(current) = self.current.as_ref() else {
            return PageProfileEditOutcome::NoAcceptedRevision;
        };
        let mut notebook = current.notebook.clone();
        if !replace_page_profile_value(&mut notebook, target, replacement) {
            return PageProfileEditOutcome::TargetNotFound {
                revision: current.id,
                target,
            };
        }
        let revision = match self.commit_semantic_edit(notebook) {
            Ok(revision) => revision,
            Err(sequence) => {
                return PageProfileEditOutcome::IdentityExhausted { sequence };
            },
        };
        PageProfileEditOutcome::Applied { base, revision, target }
    }

    fn replace_table_cell_span(
        &mut self,
        base: atrament_semantic_notebook::RevisionIdentity,
        target: AcceptedIdentity,
        span: TableCellSpan,
    ) -> TableCellSpanEditOutcome {
        let simulation = self.simulate_direct_edit(
            base,
            target,
            EditableSemanticValue::TableCellSpan(span),
        );
        let replacement = match simulation {
            DirectEditSimulationOutcome::Applicable {
                requested: EditableSemanticValue::TableCellSpan(requested),
                ..
            } => requested,
            DirectEditSimulationOutcome::Applicable { .. }
            | DirectEditSimulationOutcome::InvalidAssetReference { .. }
            | DirectEditSimulationOutcome::InvalidPageProfileReference { .. }
            | DirectEditSimulationOutcome::InvalidProvenanceReference { .. }
            | DirectEditSimulationOutcome::InvalidStyleReference { .. }
            | DirectEditSimulationOutcome::InvalidMathematics { .. }
            | DirectEditSimulationOutcome::InvalidPageProfile { .. }
            | DirectEditSimulationOutcome::TargetNotEditableValue { .. }
            | DirectEditSimulationOutcome::UnsupportedMathematics { .. }
            | DirectEditSimulationOutcome::ValueFamilyMismatch { .. } => {
                return TableCellSpanEditOutcome::TargetNotTableCell {
                    revision: base,
                    target,
                };
            },
            DirectEditSimulationOutcome::InvalidTableGrid {
                reason,
                revision,
                target: simulated_target,
            } => {
                return TableCellSpanEditOutcome::InvalidTableGrid {
                    reason,
                    revision,
                    target: simulated_target,
                };
            },
            DirectEditSimulationOutcome::NoAcceptedRevision => {
                return TableCellSpanEditOutcome::NoAcceptedRevision;
            },
            DirectEditSimulationOutcome::NoOp {
                revision,
                target: simulated_target,
                ..
            } => {
                return TableCellSpanEditOutcome::NoOp {
                    revision,
                    target: simulated_target,
                };
            },
            DirectEditSimulationOutcome::StaleBase { current } => {
                return TableCellSpanEditOutcome::StaleBase { current };
            },
            DirectEditSimulationOutcome::TargetNotFound {
                revision,
                target: missing_target,
            } => {
                return TableCellSpanEditOutcome::TargetNotFound {
                    revision,
                    target: missing_target,
                };
            },
        };
        let Some(current) = self.current.as_ref() else {
            return TableCellSpanEditOutcome::NoAcceptedRevision;
        };
        let mut notebook = current.notebook.clone();
        match replace_table_cell_span_value(
            &mut notebook,
            target,
            replacement,
        ) {
            Ok(true) => {},
            Ok(false) => {
                return TableCellSpanEditOutcome::TargetNotFound {
                    revision: current.id,
                    target,
                };
            },
            Err(reason) => {
                return TableCellSpanEditOutcome::InvalidTableGrid {
                    reason,
                    revision: current.id,
                    target,
                };
            },
        }
        let revision = match self.commit_semantic_edit(notebook) {
            Ok(revision) => revision,
            Err(sequence) => {
                return TableCellSpanEditOutcome::IdentityExhausted { sequence };
            },
        };
        TableCellSpanEditOutcome::Applied { base, revision, target }
    }

    fn replace_table_row_role(
        &mut self,
        base: atrament_semantic_notebook::RevisionIdentity,
        target: AcceptedIdentity,
        role: TableRowRole,
    ) -> TableRowRoleEditOutcome {
        let simulation = self.simulate_direct_edit(
            base,
            target,
            EditableSemanticValue::TableRowRole(role),
        );
        let replacement = match simulation {
            DirectEditSimulationOutcome::Applicable {
                requested: EditableSemanticValue::TableRowRole(requested_role),
                ..
            } => requested_role,
            DirectEditSimulationOutcome::Applicable { .. }
            | DirectEditSimulationOutcome::InvalidAssetReference { .. }
            | DirectEditSimulationOutcome::InvalidPageProfileReference { .. }
            | DirectEditSimulationOutcome::InvalidProvenanceReference { .. }
            | DirectEditSimulationOutcome::InvalidStyleReference { .. }
            | DirectEditSimulationOutcome::InvalidMathematics { .. }
            | DirectEditSimulationOutcome::InvalidPageProfile { .. }
            | DirectEditSimulationOutcome::InvalidTableGrid { .. }
            | DirectEditSimulationOutcome::TargetNotEditableValue { .. }
            | DirectEditSimulationOutcome::UnsupportedMathematics { .. }
            | DirectEditSimulationOutcome::ValueFamilyMismatch { .. } => {
                return TableRowRoleEditOutcome::TargetNotTableRow {
                    revision: base,
                    target,
                };
            },
            DirectEditSimulationOutcome::NoAcceptedRevision => {
                return TableRowRoleEditOutcome::NoAcceptedRevision;
            },
            DirectEditSimulationOutcome::NoOp {
                revision,
                target: simulated_target,
                ..
            } => {
                return TableRowRoleEditOutcome::NoOp {
                    revision,
                    target: simulated_target,
                };
            },
            DirectEditSimulationOutcome::StaleBase { current } => {
                return TableRowRoleEditOutcome::StaleBase { current };
            },
            DirectEditSimulationOutcome::TargetNotFound {
                revision,
                target: missing_target,
            } => {
                return TableRowRoleEditOutcome::TargetNotFound {
                    revision,
                    target: missing_target,
                };
            },
        };
        let Some(current) = self.current.as_ref() else {
            return TableRowRoleEditOutcome::NoAcceptedRevision;
        };
        let mut notebook = current.notebook.clone();
        if !replace_table_row_role_value(&mut notebook, target, replacement) {
            return TableRowRoleEditOutcome::TargetNotFound {
                revision: current.id,
                target,
            };
        }
        let revision = match self.commit_semantic_edit(notebook) {
            Ok(revision) => revision,
            Err(sequence) => {
                return TableRowRoleEditOutcome::IdentityExhausted { sequence };
            },
        };
        TableRowRoleEditOutcome::Applied { base, revision, target }
    }

    fn replace_text(
        &mut self,
        base: atrament_semantic_notebook::RevisionIdentity,
        target: AcceptedIdentity,
        value: String,
    ) -> TextEditOutcome {
        let simulation = self.simulate_direct_edit(
            base,
            target,
            EditableSemanticValue::Text(value),
        );
        let replacement = match simulation {
            DirectEditSimulationOutcome::Applicable {
                requested: EditableSemanticValue::Text(requested_text),
                ..
            } => requested_text,
            DirectEditSimulationOutcome::Applicable { .. }
            | DirectEditSimulationOutcome::InvalidAssetReference { .. }
            | DirectEditSimulationOutcome::InvalidPageProfileReference { .. }
            | DirectEditSimulationOutcome::InvalidProvenanceReference { .. }
            | DirectEditSimulationOutcome::InvalidStyleReference { .. }
            | DirectEditSimulationOutcome::InvalidMathematics { .. }
            | DirectEditSimulationOutcome::InvalidPageProfile { .. }
            | DirectEditSimulationOutcome::InvalidTableGrid { .. }
            | DirectEditSimulationOutcome::TargetNotEditableValue { .. }
            | DirectEditSimulationOutcome::UnsupportedMathematics { .. }
            | DirectEditSimulationOutcome::ValueFamilyMismatch { .. } => {
                return TextEditOutcome::TargetNotText {
                    revision: base,
                    target,
                };
            },
            DirectEditSimulationOutcome::NoAcceptedRevision => {
                return TextEditOutcome::NoAcceptedRevision;
            },
            DirectEditSimulationOutcome::NoOp {
                revision,
                target: simulated_target,
                ..
            } => {
                return TextEditOutcome::NoOp {
                    revision,
                    target: simulated_target,
                };
            },
            DirectEditSimulationOutcome::StaleBase { current } => {
                return TextEditOutcome::StaleBase { current };
            },
            DirectEditSimulationOutcome::TargetNotFound {
                revision,
                target: missing_target,
            } => {
                return TextEditOutcome::TargetNotFound {
                    revision,
                    target: missing_target,
                };
            },
        };
        let Some(current) = self.current.as_ref() else {
            return TextEditOutcome::NoAcceptedRevision;
        };
        let mut notebook = current.notebook.clone();
        if !replace_text_value(&mut notebook, target, replacement) {
            return TextEditOutcome::TargetNotFound {
                revision: current.id,
                target,
            };
        }
        let revision = match self.commit_semantic_edit(notebook) {
            Ok(revision) => revision,
            Err(sequence) => {
                return TextEditOutcome::IdentityExhausted { sequence };
            },
        };
        TextEditOutcome::Applied { base, revision, target }
    }

    fn simulate_direct_edit(
        &self,
        revision: atrament_semantic_notebook::RevisionIdentity,
        target: AcceptedIdentity,
        requested: EditableSemanticValue,
    ) -> DirectEditSimulationOutcome {
        let family = direct_edit_family(&requested);
        simulate_direct_edit_material_in_notebook(
            self.current.as_ref().map(|current| &current.notebook),
            self.command_target_material_for_family(revision, target, family),
            requested,
        )
        .outcome
    }

    fn simulate_direct_edit_batch<CommandIdentity>(
        &self,
        batch: DirectEditBatchProposal<CommandIdentity>,
    ) -> DirectEditBatchSimulationOutcome<CommandIdentity>
    where
        CommandIdentity: Clone + Ord,
    {
        let current = match direct_edit_batch_authority(self, &batch) {
            Ok(current) => current,
            Err(DirectEditBatchAuthorityError::CapabilityMismatch {
                current,
                expected,
            }) => {
                return DirectEditBatchSimulationOutcome::CapabilityMismatch {
                    current,
                    expected,
                };
            },
            Err(DirectEditBatchAuthorityError::NoAcceptedRevision) => {
                return DirectEditBatchSimulationOutcome::NoAcceptedRevision;
            },
            Err(DirectEditBatchAuthorityError::StaleBase { current }) => {
                return DirectEditBatchSimulationOutcome::StaleBase { current };
            },
        };
        simulate_direct_edit_batch_after_base(current, batch.commands)
    }

    fn simulate_direct_edit_batch_bounded<CommandIdentity>(
        &self,
        batch: DirectEditBatchProposal<CommandIdentity>,
        limits: CommandGraphLimits,
    ) -> DirectEditBatchSimulationOutcome<CommandIdentity>
    where
        CommandIdentity: Clone + Ord,
    {
        let current = match direct_edit_batch_authority(self, &batch) {
            Ok(current) => current,
            Err(DirectEditBatchAuthorityError::CapabilityMismatch {
                current,
                expected,
            }) => {
                return DirectEditBatchSimulationOutcome::CapabilityMismatch {
                    current,
                    expected,
                };
            },
            Err(DirectEditBatchAuthorityError::NoAcceptedRevision) => {
                return DirectEditBatchSimulationOutcome::NoAcceptedRevision;
            },
            Err(DirectEditBatchAuthorityError::StaleBase { current }) => {
                return DirectEditBatchSimulationOutcome::StaleBase { current };
            },
        };
        if let Err(reason) = direct_edit_batch_graph_limits_from_commands(
            &batch.commands,
            limits,
        ) {
            return DirectEditBatchSimulationOutcome::ResourceRejected {
                reason,
            };
        }
        simulate_direct_edit_batch_after_base(current, batch.commands)
    }

    fn simulate_direct_edit_proposal(
        &self,
        proposal: DirectEditProposal,
    ) -> DirectEditProposalOutcome {
        let compatibility = self.check_command_capability_compatibility(
            proposal.capability_version,
        );
        if let CommandCapabilityCompatibilityOutcome::Mismatch {
            current,
            expected,
        } = compatibility
        {
            return DirectEditProposalOutcome::CapabilityMismatch {
                current,
                expected,
            };
        }
        let preconditions = self.check_command_target_preconditions(
            proposal.revision,
            proposal.target,
            proposal.preconditions,
        );
        if !matches!(
            preconditions,
            CommandTargetPreconditionOutcome::Satisfied { .. }
        ) {
            return DirectEditProposalOutcome::PreconditionRejected {
                outcome: preconditions,
            };
        }
        DirectEditProposalOutcome::Simulated {
            outcome: self.simulate_direct_edit(
                proposal.revision,
                proposal.target,
                proposal.requested,
            ),
        }
    }
}

impl SemanticNotebookHistory for SemanticNotebookSessionService {
    fn history_availability(&self) -> HistoryAvailabilityOutcome {
        let Some(current) = self.current.as_ref() else {
            return HistoryAvailabilityOutcome::NoAcceptedRevision;
        };
        HistoryAvailabilityOutcome::Available(HistoryAvailability {
            can_redo: !self.redo_notebooks.is_empty(),
            can_undo: !self.undo_notebooks.is_empty(),
            revision: current.id,
        })
    }

    fn traverse_history(
        &mut self,
        base: atrament_semantic_notebook::RevisionIdentity,
        direction: HistoryDirection,
    ) -> HistoryTraversalOutcome {
        let Some(current) = self.current.as_ref() else {
            return HistoryTraversalOutcome::NoAcceptedRevision;
        };
        if current.id != base {
            return HistoryTraversalOutcome::StaleBase {
                current: current.id,
                requested: base,
            };
        }
        let source_is_empty = match direction {
            HistoryDirection::Redo => self.redo_notebooks.is_empty(),
            HistoryDirection::Undo => self.undo_notebooks.is_empty(),
        };
        if source_is_empty {
            return HistoryTraversalOutcome::Boundary {
                direction,
                revision: current.id,
            };
        }
        let revision = match self.identities.allocate_revision() {
            Ok(revision) => revision,
            Err(sequence) => {
                return HistoryTraversalOutcome::IdentityExhausted { sequence };
            },
        };
        let current_id = current.id;
        let Some(current_revision) = self.current.take() else {
            return HistoryTraversalOutcome::NoAcceptedRevision;
        };
        let restored = match direction {
            HistoryDirection::Redo => {
                let Some(restored) = self.redo_notebooks.pop() else {
                    self.current = Some(current_revision);
                    return HistoryTraversalOutcome::Boundary {
                        direction,
                        revision: current_id,
                    };
                };
                self.undo_notebooks.push(current_revision.notebook);
                restored
            },
            HistoryDirection::Undo => {
                let Some(restored) = self.undo_notebooks.pop() else {
                    self.current = Some(current_revision);
                    return HistoryTraversalOutcome::Boundary {
                        direction,
                        revision: current_id,
                    };
                };
                self.redo_notebooks.push(current_revision.notebook);
                restored
            },
        };
        self.current = Some(AcceptedRevision {
            id: revision,
            notebook: restored,
        });
        HistoryTraversalOutcome::Traversed {
            base,
            direction,
            revision,
        }
    }
}

impl SemanticNotebookSessionService {
    fn allocate_mapping(
        &self,
        owners: &[CandidateIdentity],
    ) -> Result<
        (
            BTreeMap<CandidateIdentity, AcceptedIdentity>,
            Vec<IdentityMapping>,
        ),
        IdentityExhausted,
    > {
        let mut identity_map = BTreeMap::new();
        let mut mapping = Vec::with_capacity(owners.len());
        for candidate in owners {
            let accepted = self.identities.allocate_accepted()?;
            let _previous: Option<AcceptedIdentity> =
                identity_map.insert(*candidate, accepted);
            mapping.push(IdentityMapping {
                accepted,
                candidate: *candidate,
            });
        }
        Ok((identity_map, mapping))
    }

    fn apply_direct_edit_batch_simulation<CommandIdentity>(
        &mut self,
        simulation: DirectEditBatchSimulationOutcome<CommandIdentity>,
    ) -> DirectEditBatchApplyOutcome<CommandIdentity> {
        let (base, changes, commands, impact_seeds) = match simulation {
            DirectEditBatchSimulationOutcome::CapabilityMismatch {
                current,
                expected,
            } => {
                return DirectEditBatchApplyOutcome::CapabilityMismatch {
                    current,
                    expected,
                };
            },
            DirectEditBatchSimulationOutcome::DependencyGraphRejected {
                reason,
            } => {
                return DirectEditBatchApplyOutcome::DependencyGraphRejected {
                    reason,
                };
            },
            DirectEditBatchSimulationOutcome::NoAcceptedRevision => {
                return DirectEditBatchApplyOutcome::NoAcceptedRevision;
            },
            DirectEditBatchSimulationOutcome::Predicted {
                commands,
                effect: DirectEditEffectClass::NoOp,
                revision,
                ..
            } => {
                return DirectEditBatchApplyOutcome::NoOp {
                    commands,
                    revision,
                };
            },
            DirectEditBatchSimulationOutcome::Predicted {
                changes,
                commands,
                effect: DirectEditEffectClass::Mutation,
                impact_seeds,
                revision,
            } => (revision, changes, commands, impact_seeds),
            DirectEditBatchSimulationOutcome::Rejected {
                command,
                evaluated,
                not_evaluated,
                reason,
                revision,
            } => {
                return DirectEditBatchApplyOutcome::Rejected {
                    command,
                    evaluated,
                    not_evaluated,
                    reason,
                    revision,
                };
            },
            DirectEditBatchSimulationOutcome::ResourceRejected { reason } => {
                return DirectEditBatchApplyOutcome::ResourceRejected { reason };
            },
            DirectEditBatchSimulationOutcome::StaleBase { current } => {
                return DirectEditBatchApplyOutcome::StaleBase { current };
            },
        };
        let Some(current) = self.current.as_ref() else {
            return DirectEditBatchApplyOutcome::NoAcceptedRevision;
        };
        if current.id != base {
            return DirectEditBatchApplyOutcome::StaleBase {
                current: current.id,
            };
        }
        let mut notebook = current.notebook.clone();
        if let Err(target) =
            apply_direct_edit_changes(&mut notebook, &changes)
        {
            return DirectEditBatchApplyOutcome::CandidateReplayFailed {
                revision: base,
                target,
            };
        }
        let revision = match self.commit_semantic_edit(notebook) {
            Ok(revision) => revision,
            Err(sequence) => {
                return DirectEditBatchApplyOutcome::IdentityExhausted {
                    sequence,
                };
            },
        };
        DirectEditBatchApplyOutcome::Applied {
            base,
            changes,
            commands,
            impact_seeds,
            revision,
        }
    }

    fn commit_semantic_edit(
        &mut self,
        notebook: Notebook<AcceptedIdentity>,
    ) -> Result<
        atrament_semantic_notebook::RevisionIdentity,
        IdentityExhausted,
    > {
        let revision = self.identities.allocate_revision()?;
        if let Some(current) = self.current.take() {
            self.undo_notebooks.push(current.notebook);
        }
        self.redo_notebooks.clear();
        self.current = Some(AcceptedRevision { id: revision, notebook });
        Ok(revision)
    }
}

fn accept_asset(
    asset: Asset<CandidateIdentity>,
    identities: &BTreeMap<CandidateIdentity, AcceptedIdentity>,
) -> Result<Asset<AcceptedIdentity>, CandidateGraphError> {
    Ok(Asset {
        id: accepted_id(asset.id, identities)?,
        media_type: asset.media_type,
    })
}

fn accept_block(
    block: Block<CandidateIdentity>,
    identities: &BTreeMap<CandidateIdentity, AcceptedIdentity>,
) -> Result<Block<AcceptedIdentity>, CandidateGraphError> {
    Ok(Block {
        content: accept_block_content(block.content, identities)?,
        extensions: block.extensions,
        id: accepted_id(block.id, identities)?,
        provenance: accepted_reference(block.provenance, identities)?,
        style: accepted_reference(block.style, identities)?,
    })
}

fn accept_block_content(
    content: BlockContent<CandidateIdentity>,
    identities: &BTreeMap<CandidateIdentity, AcceptedIdentity>,
) -> Result<BlockContent<AcceptedIdentity>, CandidateGraphError> {
    match content {
        BlockContent::Callout(blocks) => {
            Ok(BlockContent::Callout(accept_blocks(blocks, identities)?))
        },
        BlockContent::Date(spans) => {
            Ok(BlockContent::Date(accept_spans(spans, identities)?))
        },
        BlockContent::Figure(figure) => {
            Ok(BlockContent::Figure(accept_figure(figure, identities)?))
        },
        BlockContent::Freeform(blocks) => {
            Ok(BlockContent::Freeform(accept_blocks(blocks, identities)?))
        },
        BlockContent::Heading(spans) => {
            Ok(BlockContent::Heading(accept_spans(spans, identities)?))
        },
        BlockContent::List(list) => {
            Ok(BlockContent::List(accept_list(list, identities)?))
        },
        BlockContent::Mathematics(formula) => Ok(BlockContent::Mathematics(
            accept_formula(formula, identities)?,
        )),
        BlockContent::Paragraph(spans) => {
            Ok(BlockContent::Paragraph(accept_spans(spans, identities)?))
        },
        BlockContent::Rule => Ok(BlockContent::Rule),
        BlockContent::Table(table) => {
            Ok(BlockContent::Table(accept_table(table, identities)?))
        },
        BlockContent::Unresolved(unresolved) => {
            Ok(BlockContent::Unresolved(unresolved))
        },
    }
}

fn accept_blocks(
    blocks: Vec<Block<CandidateIdentity>>,
    identities: &BTreeMap<CandidateIdentity, AcceptedIdentity>,
) -> Result<Vec<Block<AcceptedIdentity>>, CandidateGraphError> {
    blocks
        .into_iter()
        .map(|block| accept_block(block, identities))
        .collect()
}

fn accept_constraint(
    constraint: &Constraint<CandidateIdentity>,
    identities: &BTreeMap<CandidateIdentity, AcceptedIdentity>,
) -> Result<Constraint<AcceptedIdentity>, CandidateGraphError> {
    Ok(Constraint {
        id: accepted_id(constraint.id, identities)?,
        kind: constraint.kind,
        target: accepted_id(constraint.target, identities)?,
    })
}

fn accept_figure(
    figure: Figure<CandidateIdentity>,
    identities: &BTreeMap<CandidateIdentity, AcceptedIdentity>,
) -> Result<Figure<AcceptedIdentity>, CandidateGraphError> {
    Ok(Figure {
        asset: accepted_reference(figure.asset, identities)?,
        caption: accept_spans(figure.caption, identities)?,
        id: accepted_id(figure.id, identities)?,
    })
}

fn accept_flow(
    flow: Flow<CandidateIdentity>,
    identities: &BTreeMap<CandidateIdentity, AcceptedIdentity>,
) -> Result<Flow<AcceptedIdentity>, CandidateGraphError> {
    Ok(Flow {
        blocks: accept_blocks(flow.blocks, identities)?,
        id: accepted_id(flow.id, identities)?,
    })
}

fn accept_formula(
    formula: Formula<CandidateIdentity>,
    identities: &BTreeMap<CandidateIdentity, AcceptedIdentity>,
) -> Result<Formula<AcceptedIdentity>, CandidateGraphError> {
    Ok(Formula {
        id: accepted_id(formula.id, identities)?,
        mode: formula.mode,
        source: formula.source,
    })
}

fn accept_list(
    list: List<CandidateIdentity>,
    identities: &BTreeMap<CandidateIdentity, AcceptedIdentity>,
) -> Result<List<AcceptedIdentity>, CandidateGraphError> {
    let items = list
        .items
        .into_iter()
        .map(|item| accept_list_item(item, identities))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(List {
        id: accepted_id(list.id, identities)?,
        items,
        ordered: list.ordered,
    })
}

fn accept_list_item(
    item: ListItem<CandidateIdentity>,
    identities: &BTreeMap<CandidateIdentity, AcceptedIdentity>,
) -> Result<ListItem<AcceptedIdentity>, CandidateGraphError> {
    Ok(ListItem {
        blocks: accept_blocks(item.blocks, identities)?,
        id: accepted_id(item.id, identities)?,
    })
}

fn accept_notebook(
    notebook: Notebook<CandidateIdentity>,
    identities: &BTreeMap<CandidateIdentity, AcceptedIdentity>,
) -> Result<Notebook<AcceptedIdentity>, CandidateGraphError> {
    Ok(Notebook {
        assets: notebook
            .assets
            .into_iter()
            .map(|asset| accept_asset(asset, identities))
            .collect::<Result<Vec<_>, _>>()?,
        constraints: notebook
            .constraints
            .iter()
            .map(|constraint| accept_constraint(constraint, identities))
            .collect::<Result<Vec<_>, _>>()?,
        extensions: notebook.extensions,
        id: accepted_id(notebook.id, identities)?,
        output_profiles: notebook
            .output_profiles
            .into_iter()
            .map(|profile| accept_output_profile(profile, identities))
            .collect::<Result<Vec<_>, _>>()?,
        page_profiles: notebook
            .page_profiles
            .iter()
            .map(|profile| accept_paper_profile(profile, identities))
            .collect::<Result<Vec<_>, _>>()?,
        pages: notebook
            .pages
            .into_iter()
            .map(|page| accept_page(page, identities))
            .collect::<Result<Vec<_>, _>>()?,
        provenance: notebook
            .provenance
            .into_iter()
            .map(|provenance| accept_provenance(provenance, identities))
            .collect::<Result<Vec<_>, _>>()?,
        styles: notebook
            .styles
            .into_iter()
            .map(|style| accept_style(style, identities))
            .collect::<Result<Vec<_>, _>>()?,
    })
}

fn accept_output_profile(
    profile: OutputProfile<CandidateIdentity>,
    identities: &BTreeMap<CandidateIdentity, AcceptedIdentity>,
) -> Result<OutputProfile<AcceptedIdentity>, CandidateGraphError> {
    Ok(OutputProfile {
        id: accepted_id(profile.id, identities)?,
        name: profile.name,
    })
}

fn accept_page(
    page: Page<CandidateIdentity>,
    identities: &BTreeMap<CandidateIdentity, AcceptedIdentity>,
) -> Result<Page<AcceptedIdentity>, CandidateGraphError> {
    Ok(Page {
        flows: page
            .flows
            .into_iter()
            .map(|flow| accept_flow(flow, identities))
            .collect::<Result<Vec<_>, _>>()?,
        id: accepted_id(page.id, identities)?,
        page_profile: accepted_id(page.page_profile, identities)?,
    })
}

fn accept_paper_profile(
    profile: &PaperProfile<CandidateIdentity>,
    identities: &BTreeMap<CandidateIdentity, AcceptedIdentity>,
) -> Result<PaperProfile<AcceptedIdentity>, CandidateGraphError> {
    Ok(PaperProfile {
        geometry: profile.geometry,
        id: accepted_id(profile.id, identities)?,
    })
}

fn accept_provenance(
    provenance: Provenance<CandidateIdentity>,
    identities: &BTreeMap<CandidateIdentity, AcceptedIdentity>,
) -> Result<Provenance<AcceptedIdentity>, CandidateGraphError> {
    Ok(Provenance {
        id: accepted_id(provenance.id, identities)?,
        kind: provenance.kind,
        reference: provenance.reference,
    })
}

fn accept_spans(
    spans: Vec<InlineSpan<CandidateIdentity>>,
    identities: &BTreeMap<CandidateIdentity, AcceptedIdentity>,
) -> Result<Vec<InlineSpan<AcceptedIdentity>>, CandidateGraphError> {
    spans
        .into_iter()
        .map(|span| {
            Ok(InlineSpan {
                id: accepted_id(span.id, identities)?,
                provenance: accepted_reference(span.provenance, identities)?,
                style: accepted_reference(span.style, identities)?,
                text: span.text,
            })
        })
        .collect()
}

fn accept_style(
    style: Style<CandidateIdentity>,
    identities: &BTreeMap<CandidateIdentity, AcceptedIdentity>,
) -> Result<Style<AcceptedIdentity>, CandidateGraphError> {
    Ok(Style {
        id: accepted_id(style.id, identities)?,
        name: style.name,
    })
}

fn accept_table(
    table: Table<CandidateIdentity>,
    identities: &BTreeMap<CandidateIdentity, AcceptedIdentity>,
) -> Result<Table<AcceptedIdentity>, CandidateGraphError> {
    Ok(Table {
        id: accepted_id(table.id, identities)?,
        rows: table
            .rows
            .into_iter()
            .map(|row| accept_table_row(row, identities))
            .collect::<Result<Vec<_>, _>>()?,
    })
}

fn accept_table_cell(
    cell: TableCell<CandidateIdentity>,
    identities: &BTreeMap<CandidateIdentity, AcceptedIdentity>,
) -> Result<TableCell<AcceptedIdentity>, CandidateGraphError> {
    Ok(TableCell {
        blocks: accept_blocks(cell.blocks, identities)?,
        id: accepted_id(cell.id, identities)?,
        span: cell.span,
    })
}

fn accept_table_row(
    row: TableRow<CandidateIdentity>,
    identities: &BTreeMap<CandidateIdentity, AcceptedIdentity>,
) -> Result<TableRow<AcceptedIdentity>, CandidateGraphError> {
    Ok(TableRow {
        cells: row
            .cells
            .into_iter()
            .map(|cell| accept_table_cell(cell, identities))
            .collect::<Result<Vec<_>, _>>()?,
        id: accepted_id(row.id, identities)?,
        role: row.role,
    })
}

fn accepted_id(
    candidate: CandidateIdentity,
    identities: &BTreeMap<CandidateIdentity, AcceptedIdentity>,
) -> Result<AcceptedIdentity, CandidateGraphError> {
    identities
        .get(&candidate)
        .copied()
        .ok_or(CandidateGraphError::MissingReference { candidate })
}

fn accepted_reference(
    candidate: Option<CandidateIdentity>,
    identities: &BTreeMap<CandidateIdentity, AcceptedIdentity>,
) -> Result<Option<AcceptedIdentity>, CandidateGraphError> {
    candidate
        .map(|identity| accepted_id(identity, identities))
        .transpose()
}

fn formula_blocks_value(
    blocks: &[Block<AcceptedIdentity>],
    target: AcceptedIdentity,
) -> Option<&Formula<AcceptedIdentity>> {
    for block in blocks {
        if let Some(formula) = formula_content_value(&block.content, target) {
            return Some(formula);
        }
    }
    None
}

fn formula_content_value(
    content: &BlockContent<AcceptedIdentity>,
    target: AcceptedIdentity,
) -> Option<&Formula<AcceptedIdentity>> {
    match content {
        BlockContent::Callout(blocks) | BlockContent::Freeform(blocks) => {
            formula_blocks_value(blocks, target)
        },
        BlockContent::List(list) => {
            for item in &list.items {
                if let Some(formula) =
                    formula_blocks_value(&item.blocks, target)
                {
                    return Some(formula);
                }
            }
            None
        },
        BlockContent::Mathematics(formula) if formula.id == target => {
            Some(formula)
        },
        BlockContent::Table(table) => {
            for row in &table.rows {
                for cell in &row.cells {
                    if let Some(formula) =
                        formula_blocks_value(&cell.blocks, target)
                    {
                        return Some(formula);
                    }
                }
            }
            None
        },
        BlockContent::Date(_)
        | BlockContent::Figure(_)
        | BlockContent::Heading(_)
        | BlockContent::Mathematics(_)
        | BlockContent::Paragraph(_)
        | BlockContent::Rule
        | BlockContent::Unresolved(_) => None,
    }
}

fn direct_edit_batch_authority<'session, CommandIdentity>(
    session: &'session SemanticNotebookSessionService,
    batch: &DirectEditBatchProposal<CommandIdentity>,
) -> Result<&'session AcceptedRevision, DirectEditBatchAuthorityError> {
    let snapshot = session.command_capability_snapshot();
    if snapshot.behavior_version != batch.capability_version {
        return Err(DirectEditBatchAuthorityError::CapabilityMismatch {
            current: snapshot.behavior_version,
            expected: batch.capability_version,
        });
    }
    let Some(current) = session.current.as_ref() else {
        return Err(DirectEditBatchAuthorityError::NoAcceptedRevision);
    };
    if current.id != batch.base {
        return Err(DirectEditBatchAuthorityError::StaleBase {
            current: current.id,
        });
    }
    Ok(current)
}

fn direct_edit_batch_graph_limits_from_commands<CommandIdentity>(
    commands: &[DirectEditBatchCommand<CommandIdentity>],
    limits: CommandGraphLimits,
) -> Result<CommandGraphSize, CommandGraphLimitError> {
    if commands.len() > limits.commands {
        return Err(CommandGraphLimitError::CommandCountExceeded {
            actual: commands.len(),
            limit: limits.commands,
        });
    }
    let size = direct_edit_batch_graph_size_from_commands(commands)?;
    if size.dependency_edges > limits.dependency_edges {
        return Err(CommandGraphLimitError::DependencyEdgeCountExceeded {
            actual: size.dependency_edges,
            limit: limits.dependency_edges,
        });
    }
    Ok(size)
}

fn direct_edit_batch_graph_size_from_commands<CommandIdentity>(
    commands: &[DirectEditBatchCommand<CommandIdentity>],
) -> Result<CommandGraphSize, CommandGraphLimitError> {
    let mut dependency_edges = 0usize;
    for command in commands {
        let Some(next) =
            dependency_edges.checked_add(command.dependencies.len())
        else {
            return Err(CommandGraphLimitError::DependencyEdgeCountOverflow);
        };
        dependency_edges = next;
    }
    Ok(CommandGraphSize {
        commands: commands.len(),
        dependency_edges,
    })
}

fn direct_edit_command_graph_nodes<CommandIdentity>(
    commands: &[DirectEditBatchCommand<CommandIdentity>],
) -> Vec<DirectEditCommandGraphNode<'_, CommandIdentity>> {
    commands
        .iter()
        .map(|command| DirectEditCommandGraphNode { command })
        .collect()
}

fn command_target_material_from_notebook(
    notebook: &Notebook<AcceptedIdentity>,
    revision: atrament_semantic_notebook::RevisionIdentity,
    target: AcceptedIdentity,
) -> CommandTargetMaterialOutcome {
    if let Some(provenance) = provenance_value(notebook, target) {
        return CommandTargetMaterialOutcome::Prepared {
            material: CommandTargetMaterial {
                descriptor: SemanticIdentityDescriptor {
                    kind: SemanticIdentityKind::Provenance,
                    owner: Some(notebook.id),
                },
                direct_edit_family: Some(SemanticCommandFamily::Provenance),
                editable_value: Some(editable_provenance_value(provenance)),
                revision,
                target,
            },
        };
    }
    let Some(descriptor) = semantic_identity_descriptor(notebook, target)
    else {
        return CommandTargetMaterialOutcome::TargetNotFound {
            revision,
            target,
        };
    };
    let editable_value =
        editable_semantic_value(notebook, target, descriptor.kind);
    let direct_edit_family = editable_value.as_ref().map(direct_edit_family);
    CommandTargetMaterialOutcome::Prepared {
        material: CommandTargetMaterial {
            direct_edit_family,
            descriptor,
            editable_value,
            revision,
            target,
        },
    }
}

fn command_target_material_for_family_from_notebook(
    notebook: &Notebook<AcceptedIdentity>,
    revision: atrament_semantic_notebook::RevisionIdentity,
    target: AcceptedIdentity,
    family: SemanticCommandFamily,
) -> CommandTargetMaterialOutcome {
    let Some(descriptor) = semantic_identity_descriptor(notebook, target) else {
        return CommandTargetMaterialOutcome::TargetNotFound {
            revision,
            target,
        };
    };
    let requested_value = editable_semantic_value_for_family(
        notebook,
        target,
        descriptor.kind,
        family,
    );
    let editable_value = requested_value.or_else(|| {
        editable_semantic_value(notebook, target, descriptor.kind)
    });
    let direct_edit_family = editable_value.as_ref().map(direct_edit_family);
    CommandTargetMaterialOutcome::Prepared {
        material: CommandTargetMaterial {
            descriptor,
            direct_edit_family,
            editable_value,
            revision,
            target,
        },
    }
}

fn check_command_target_preconditions_material(
    material: CommandTargetMaterial,
    preconditions: &CommandTargetPreconditions,
) -> CommandTargetPreconditionOutcome {
    let revision = material.revision;
    let target = material.target;
    if material.direct_edit_family != Some(preconditions.requested_family) {
        return CommandTargetPreconditionOutcome::FamilyNotExecutable {
            available: material.direct_edit_family,
            requested: preconditions.requested_family,
            revision,
            target,
        };
    }
    if let Some(expected) = preconditions.identity.expected_kind
        && material.descriptor.kind != expected
    {
        return CommandTargetPreconditionOutcome::KindMismatch {
            actual: material.descriptor.kind,
            expected,
            revision,
            target,
        };
    }
    let owner_matches = match preconditions.identity.expected_owner {
        IdentityOwnerExpectation::Any => true,
        IdentityOwnerExpectation::Direct(expected) => {
            material.descriptor.owner == Some(expected)
        },
        IdentityOwnerExpectation::Root => material.descriptor.owner.is_none(),
    };
    if !owner_matches {
        return CommandTargetPreconditionOutcome::OwnerMismatch {
            actual: material.descriptor.owner,
            expected: preconditions.identity.expected_owner,
            revision,
            target,
        };
    }
    if let Some(expected) = preconditions.expected_value.as_ref() {
        let Some(actual) = material.editable_value.as_ref() else {
            return CommandTargetPreconditionOutcome::TargetNotEditableValue {
                kind: material.descriptor.kind,
                revision,
                target,
            };
        };
        if actual != expected {
            return CommandTargetPreconditionOutcome::ValueMismatch {
                actual: actual.clone(),
                expected: expected.clone(),
                revision,
                target,
            };
        }
    }
    CommandTargetPreconditionOutcome::Satisfied { material }
}

fn direct_edit_material_index(
    notebook: &Notebook<AcceptedIdentity>,
    request: DirectEditBatchIndexRequest<'_>,
    revision: atrament_semantic_notebook::RevisionIdentity,
) -> DirectEditBatchIndex {
    let mut index = DirectEditBatchIndex::default();
    index_direct_edit_constraints(
        notebook,
        request,
        revision,
        &mut index,
    );
    if request.is_complete(&index) {
        return index;
    }
    index_direct_edit_page_profiles(notebook, request, revision, &mut index);
    if request.is_complete(&index) {
        return index;
    }
    index_direct_edit_pages(notebook, request, revision, &mut index);
    if request.is_complete(&index) {
        return index;
    }
    for provenance in &notebook.provenance {
        if !request.contains_family(
            provenance.id,
            SemanticCommandFamily::Provenance,
        ) {
            continue;
        }
        insert_direct_edit_material(
            &mut index,
            provenance.id,
            SemanticIdentityDescriptor {
                kind: SemanticIdentityKind::Provenance,
                owner: Some(notebook.id),
            },
            editable_provenance_value(provenance),
            DirectEditImpactScope::Notebook { notebook: notebook.id },
            revision,
        );
    }
    if request.is_complete(&index) {
        return index;
    }
    let mut stack = Vec::new();
    'pages: for page in &notebook.pages {
        for flow in &page.flows {
            if !flow.blocks.is_empty() {
                stack.push(DirectEditBatchIndexFrame::Blocks {
                    blocks: &flow.blocks,
                    flow: flow.id,
                    page: page.id,
                });
            }
            while let Some(frame) = stack.pop() {
                index_direct_edit_frame(
                    frame, &mut index, request, revision, &mut stack,
                );
                if request.is_complete(&index) {
                    break 'pages;
                }
            }
        }
    }
    index
}

fn index_direct_edit_page_profiles(
    notebook: &Notebook<AcceptedIdentity>,
    request: DirectEditBatchIndexRequest<'_>,
    revision: atrament_semantic_notebook::RevisionIdentity,
    index: &mut DirectEditBatchIndex,
) {
    let has_targeted_profile = notebook.page_profiles.iter().any(|profile| {
        request.contains_family(
            profile.id,
            SemanticCommandFamily::DocumentConstraint,
        )
    });
    if !has_targeted_profile {
        return;
    }
    let mut profile_pages =
        BTreeMap::<AcceptedIdentity, Vec<AcceptedIdentity>>::new();
    for page in &notebook.pages {
        if request.contains_family(
            page.page_profile,
            SemanticCommandFamily::DocumentConstraint,
        ) {
            profile_pages
                .entry(page.page_profile)
                .or_default()
                .push(page.id);
        }
    }
    for profile in &notebook.page_profiles {
        if !request.contains_family(
            profile.id,
            SemanticCommandFamily::DocumentConstraint,
        ) {
            continue;
        }
        let pages = profile_pages.remove(&profile.id).unwrap_or_default();
        let impact = if pages.is_empty() {
            DirectEditImpactScope::Notebook { notebook: notebook.id }
        } else {
            DirectEditImpactScope::Pages { pages }
        };
        insert_direct_edit_material(
            index,
            profile.id,
            SemanticIdentityDescriptor {
                kind: SemanticIdentityKind::PageProfile,
                owner: Some(notebook.id),
            },
            EditableSemanticValue::PageProfile(profile.geometry),
            impact,
            revision,
        );
    }
}

fn index_direct_edit_pages(
    notebook: &Notebook<AcceptedIdentity>,
    request: DirectEditBatchIndexRequest<'_>,
    revision: atrament_semantic_notebook::RevisionIdentity,
    index: &mut DirectEditBatchIndex,
) {
    for page in &notebook.pages {
        if !request.contains_family(
            page.id,
            SemanticCommandFamily::DocumentConstraint,
        ) {
            continue;
        }
        insert_direct_edit_material(
            index,
            page.id,
            SemanticIdentityDescriptor {
                kind: SemanticIdentityKind::Page,
                owner: Some(notebook.id),
            },
            EditableSemanticValue::PageProfileReference(page.page_profile),
            DirectEditImpactScope::Pages { pages: vec![page.id] },
            revision,
        );
    }
}

fn index_direct_edit_constraints(
    notebook: &Notebook<AcceptedIdentity>,
    request: DirectEditBatchIndexRequest<'_>,
    revision: atrament_semantic_notebook::RevisionIdentity,
    index: &mut DirectEditBatchIndex,
) {
    for constraint in &notebook.constraints {
        if !request.contains_family(
            constraint.id,
            SemanticCommandFamily::DocumentConstraint,
        ) {
            continue;
        }
        insert_direct_edit_material(
            index,
            constraint.id,
            SemanticIdentityDescriptor {
                kind: SemanticIdentityKind::Constraint,
                owner: Some(notebook.id),
            },
            EditableSemanticValue::ConstraintKind(constraint.kind),
            DirectEditImpactScope::Notebook { notebook: notebook.id },
            revision,
        );
    }
}

fn index_direct_edit_frame<'notebook>(
    frame: DirectEditBatchIndexFrame<'notebook>,
    index: &mut DirectEditBatchIndex,
    request: DirectEditBatchIndexRequest<'_>,
    revision: atrament_semantic_notebook::RevisionIdentity,
    stack: &mut Vec<DirectEditBatchIndexFrame<'notebook>>,
) {
    match frame {
        DirectEditBatchIndexFrame::Blocks { blocks, flow, page } => {
            index_direct_edit_blocks_frame(
                blocks, flow, page, index, request, revision, stack,
            );
        },
        DirectEditBatchIndexFrame::ListItems { flow, items, page } => {
            index_direct_edit_list_items_frame(items, flow, page, stack);
        },
        DirectEditBatchIndexFrame::TableCells {
            cells,
            context,
            row,
        } => {
            index_direct_edit_table_cells_frame(
                cells, context, row, index, request, revision, stack,
            );
        },
        DirectEditBatchIndexFrame::TableRows {
            block,
            flow,
            page,
            rows,
            table,
        } => {
            index_direct_edit_table_rows_frame(
                rows,
                DirectEditBatchTableContext { block, flow, page, table },
                index,
                request,
                revision,
                stack,
            );
        },
    }
}

const fn direct_edit_block_kind(
    content: &BlockContent<AcceptedIdentity>,
) -> SemanticBlockKind {
    match content {
        BlockContent::Callout(_) => SemanticBlockKind::Callout,
        BlockContent::Date(_) => SemanticBlockKind::Date,
        BlockContent::Figure(_) => SemanticBlockKind::Figure,
        BlockContent::Freeform(_) => SemanticBlockKind::Freeform,
        BlockContent::Heading(_) => SemanticBlockKind::Heading,
        BlockContent::List(_) => SemanticBlockKind::List,
        BlockContent::Mathematics(_) => SemanticBlockKind::Mathematics,
        BlockContent::Paragraph(_) => SemanticBlockKind::Paragraph,
        BlockContent::Rule => SemanticBlockKind::Rule,
        BlockContent::Table(_) => SemanticBlockKind::Table,
        BlockContent::Unresolved(_) => SemanticBlockKind::Unresolved,
    }
}

fn index_direct_edit_blocks_frame<'notebook>(
    current: &'notebook [Block<AcceptedIdentity>],
    flow: AcceptedIdentity,
    page: AcceptedIdentity,
    index: &mut DirectEditBatchIndex,
    request: DirectEditBatchIndexRequest<'_>,
    revision: atrament_semantic_notebook::RevisionIdentity,
    stack: &mut Vec<DirectEditBatchIndexFrame<'notebook>>,
) {
    let Some((block, remaining)) = current.split_first() else {
        return;
    };
    if !remaining.is_empty() {
        stack.push(DirectEditBatchIndexFrame::Blocks {
            blocks: remaining,
            flow,
            page,
        });
    }
    index_direct_edit_block_materials(
        block, flow, page, index, request, revision,
    );
    if request.is_complete(index) {
        return;
    }
    match &block.content {
        BlockContent::Callout(children) | BlockContent::Freeform(children) => {
            if !children.is_empty() {
                stack.push(DirectEditBatchIndexFrame::Blocks {
                    blocks: children,
                    flow,
                    page,
                });
            }
        },
        BlockContent::Date(spans)
        | BlockContent::Heading(spans)
        | BlockContent::Paragraph(spans) => {
            index_direct_edit_spans(
                index,
                spans,
                request,
                block.id,
                DirectEditBatchBlockContext {
                    block: block.id,
                    flow,
                    page,
                },
                revision,
            );
        },
        BlockContent::Figure(figure) => index_direct_edit_figure(
            figure, block.id, flow, page, index, request, revision,
        ),
        BlockContent::List(list) => index_direct_edit_list(
            list,
            DirectEditBatchBlockContext {
                block: block.id,
                flow,
                page,
            },
            index,
            request,
            revision,
            stack,
        ),
        BlockContent::Mathematics(formula) => {
            if request.contains_family(
                formula.id,
                SemanticCommandFamily::StructuredContent,
            ) {
                insert_direct_edit_material(
                    index,
                    formula.id,
                    SemanticIdentityDescriptor {
                        kind: SemanticIdentityKind::Formula,
                        owner: Some(block.id),
                    },
                    EditableSemanticValue::Formula {
                        mode: formula.mode,
                        source: formula.source.clone(),
                    },
                    DirectEditImpactScope::BlockFlow {
                        block: block.id,
                        flow,
                        page,
                    },
                    revision,
                );
            }
        },
        BlockContent::Table(table) => {
            if !table.rows.is_empty() {
                stack.push(DirectEditBatchIndexFrame::TableRows {
                    block: block.id,
                    flow,
                    page,
                    rows: &table.rows,
                    table,
                });
            }
        },
        BlockContent::Rule | BlockContent::Unresolved(_) => {},
    }
}

fn index_direct_edit_block_materials(
    block: &Block<AcceptedIdentity>,
    flow: AcceptedIdentity,
    page: AcceptedIdentity,
    index: &mut DirectEditBatchIndex,
    request: DirectEditBatchIndexRequest<'_>,
    revision: atrament_semantic_notebook::RevisionIdentity,
) {
    if !request.contains_target(block.id) {
        return;
    }
    let descriptor = SemanticIdentityDescriptor {
        kind: SemanticIdentityKind::Block(direct_edit_block_kind(
            &block.content,
        )),
        owner: Some(flow),
    };
    if request.contains_family(block.id, SemanticCommandFamily::StyleRole) {
        insert_direct_edit_material(
            index,
            block.id,
            descriptor,
            EditableSemanticValue::StyleReference(block.style),
            DirectEditImpactScope::BlockFlow {
                block: block.id,
                flow,
                page,
            },
            revision,
        );
    }
    if request.contains_family(block.id, SemanticCommandFamily::Provenance) {
        insert_direct_edit_material(
            index,
            block.id,
            descriptor,
            EditableSemanticValue::ProvenanceReference(block.provenance),
            DirectEditImpactScope::BlockFlow {
                block: block.id,
                flow,
                page,
            },
            revision,
        );
    }
}

fn index_direct_edit_list<'notebook>(
    list: &'notebook List<AcceptedIdentity>,
    context: DirectEditBatchBlockContext,
    index: &mut DirectEditBatchIndex,
    request: DirectEditBatchIndexRequest<'_>,
    revision: atrament_semantic_notebook::RevisionIdentity,
    stack: &mut Vec<DirectEditBatchIndexFrame<'notebook>>,
) {
    let DirectEditBatchBlockContext { block, flow, page } = context;
    if request.contains_family(
        list.id,
        SemanticCommandFamily::OrderingAndGrouping,
    ) {
        insert_direct_edit_material(
            index,
            list.id,
            SemanticIdentityDescriptor {
                kind: SemanticIdentityKind::List,
                owner: Some(block),
            },
            EditableSemanticValue::ListOrdering(list.ordered),
            DirectEditImpactScope::BlockFlow { block, flow, page },
            revision,
        );
        if request.is_complete(index) {
            return;
        }
    }
    if !list.items.is_empty() {
        stack.push(DirectEditBatchIndexFrame::ListItems {
            flow,
            items: &list.items,
            page,
        });
    }
}

fn index_direct_edit_figure(
    figure: &Figure<AcceptedIdentity>,
    block: AcceptedIdentity,
    flow: AcceptedIdentity,
    page: AcceptedIdentity,
    index: &mut DirectEditBatchIndex,
    request: DirectEditBatchIndexRequest<'_>,
    revision: atrament_semantic_notebook::RevisionIdentity,
) {
    if request.contains_family(
        figure.id,
        SemanticCommandFamily::AssetReference,
    ) {
        insert_direct_edit_material(
            index,
            figure.id,
            SemanticIdentityDescriptor {
                kind: SemanticIdentityKind::Figure,
                owner: Some(block),
            },
            EditableSemanticValue::AssetReference(figure.asset),
            DirectEditImpactScope::BlockFlow { block, flow, page },
            revision,
        );
        if request.is_complete(index) {
            return;
        }
    }
    index_direct_edit_spans(
        index,
        &figure.caption,
        request,
        figure.id,
        DirectEditBatchBlockContext { block, flow, page },
        revision,
    );
}

fn index_direct_edit_list_items_frame<'notebook>(
    current: &'notebook [ListItem<AcceptedIdentity>],
    flow: AcceptedIdentity,
    page: AcceptedIdentity,
    stack: &mut Vec<DirectEditBatchIndexFrame<'notebook>>,
) {
    let Some((item, remaining)) = current.split_first() else {
        return;
    };
    if !remaining.is_empty() {
        stack.push(DirectEditBatchIndexFrame::ListItems {
            flow,
            items: remaining,
            page,
        });
    }
    if !item.blocks.is_empty() {
        stack.push(DirectEditBatchIndexFrame::Blocks {
            blocks: &item.blocks,
            flow,
            page,
        });
    }
}

fn index_direct_edit_table_cells_frame<'notebook>(
    current: &'notebook [TableCell<AcceptedIdentity>],
    context: DirectEditBatchTableContext<'notebook>,
    row: AcceptedIdentity,
    index: &mut DirectEditBatchIndex,
    request: DirectEditBatchIndexRequest<'_>,
    revision: atrament_semantic_notebook::RevisionIdentity,
    stack: &mut Vec<DirectEditBatchIndexFrame<'notebook>>,
) {
    let Some((cell, remaining)) = current.split_first() else {
        return;
    };
    if !remaining.is_empty() {
        stack.push(DirectEditBatchIndexFrame::TableCells {
            cells: remaining,
            context,
            row,
        });
    }
    if request.contains_family(
        cell.id,
        SemanticCommandFamily::StructuredContent,
    ) {
        insert_direct_edit_material(
            index,
            cell.id,
            SemanticIdentityDescriptor {
                kind: SemanticIdentityKind::TableCell,
                owner: Some(row),
            },
            EditableSemanticValue::TableCellSpan(cell.span),
            DirectEditImpactScope::BlockFlow {
                block: context.block,
                flow: context.flow,
                page: context.page,
            },
            revision,
        );
        let table_id = context.table.id;
        let _previous_table = index.table_by_cell.insert(cell.id, table_id);
        let _overlay = index
            .table_overlays
            .entry(table_id)
            .or_insert_with(|| context.table.clone());
        if request.is_complete(index) {
            return;
        }
    }
    if !cell.blocks.is_empty() {
        stack.push(DirectEditBatchIndexFrame::Blocks {
            blocks: &cell.blocks,
            flow: context.flow,
            page: context.page,
        });
    }
}

fn index_direct_edit_table_rows_frame<'notebook>(
    current: &'notebook [TableRow<AcceptedIdentity>],
    context: DirectEditBatchTableContext<'notebook>,
    index: &mut DirectEditBatchIndex,
    request: DirectEditBatchIndexRequest<'_>,
    revision: atrament_semantic_notebook::RevisionIdentity,
    stack: &mut Vec<DirectEditBatchIndexFrame<'notebook>>,
) {
    let DirectEditBatchTableContext { block, flow, page, table } = context;
    let Some((row, remaining)) = current.split_first() else {
        return;
    };
    if !remaining.is_empty() {
        stack.push(DirectEditBatchIndexFrame::TableRows {
            block,
            flow,
            page,
            rows: remaining,
            table,
        });
    }
    if request.contains_family(
        row.id,
        SemanticCommandFamily::StructuredContent,
    ) {
        insert_direct_edit_material(
            index,
            row.id,
            SemanticIdentityDescriptor {
                kind: SemanticIdentityKind::TableRow,
                owner: Some(table.id),
            },
            EditableSemanticValue::TableRowRole(row.role),
            DirectEditImpactScope::BlockFlow { block, flow, page },
            revision,
        );
        if request.is_complete(index) {
            return;
        }
    }
    if !row.cells.is_empty() {
        stack.push(DirectEditBatchIndexFrame::TableCells {
            cells: &row.cells,
            context,
            row: row.id,
        });
    }
}

fn index_direct_edit_spans(
    index: &mut DirectEditBatchIndex,
    spans: &[InlineSpan<AcceptedIdentity>],
    request: DirectEditBatchIndexRequest<'_>,
    owner: AcceptedIdentity,
    context: DirectEditBatchBlockContext,
    revision: atrament_semantic_notebook::RevisionIdentity,
) {
    let DirectEditBatchBlockContext { block, flow, page } = context;
    for span in spans {
        if !request.contains_target(span.id) {
            continue;
        }
        let descriptor = SemanticIdentityDescriptor {
            kind: SemanticIdentityKind::InlineSpan,
            owner: Some(owner),
        };
        if request.contains_family(
            span.id,
            SemanticCommandFamily::TextContent,
        ) {
            insert_direct_edit_material(
                index,
                span.id,
                descriptor,
                EditableSemanticValue::Text(span.text.clone()),
                DirectEditImpactScope::Flow { flow, page },
                revision,
            );
        }
        if request.contains_family(
            span.id,
            SemanticCommandFamily::StyleRole,
        ) {
            insert_direct_edit_material(
                index,
                span.id,
                descriptor,
                EditableSemanticValue::StyleReference(span.style),
                DirectEditImpactScope::BlockFlow {
                    block,
                    flow,
                    page,
                },
                revision,
            );
        }
        if request.contains_family(
            span.id,
            SemanticCommandFamily::Provenance,
        ) {
            insert_direct_edit_material(
                index,
                span.id,
                descriptor,
                EditableSemanticValue::ProvenanceReference(span.provenance),
                DirectEditImpactScope::BlockFlow {
                    block,
                    flow,
                    page,
                },
                revision,
            );
        }
        if request.is_complete(index) {
            break;
        }
    }
}

fn insert_direct_edit_material(
    index: &mut DirectEditBatchIndex,
    target: AcceptedIdentity,
    descriptor: SemanticIdentityDescriptor<AcceptedIdentity>,
    editable_value: EditableSemanticValue,
    impact: DirectEditImpactScope,
    revision: atrament_semantic_notebook::RevisionIdentity,
) {
    let family = direct_edit_family(&editable_value);
    let key = (target, family);
    let direct_edit_family = Some(family);
    let _previous_impact = index.impacts.insert(key, impact);
    let _previous_material =
        index.materials.insert(key, CommandTargetMaterial {
            descriptor,
            direct_edit_family,
            editable_value: Some(editable_value),
            revision,
            target,
        });
}

fn simulate_direct_edit_batch_after_base<CommandIdentity>(
    current: &AcceptedRevision,
    commands: Vec<DirectEditBatchCommand<CommandIdentity>>,
) -> DirectEditBatchSimulationOutcome<CommandIdentity>
where
    CommandIdentity: Clone + Ord,
{
    let validation = {
        let nodes = direct_edit_command_graph_nodes(&commands);
        validate_command_graph(&nodes)
    };
    if let Err(reason) = validation {
        return DirectEditBatchSimulationOutcome::DependencyGraphRejected {
            reason,
        };
    }
    simulate_direct_edit_batch_commands(current, commands)
}

fn simulate_direct_edit_batch_commands<CommandIdentity>(
    current: &AcceptedRevision,
    commands: Vec<DirectEditBatchCommand<CommandIdentity>>,
) -> DirectEditBatchSimulationOutcome<CommandIdentity>
where
    CommandIdentity: Clone + Ord,
{
    let revision = current.id;
    if commands.is_empty() {
        return DirectEditBatchSimulationOutcome::Predicted {
            changes: Vec::new(),
            commands: Vec::new(),
            effect: DirectEditEffectClass::NoOp,
            impact_seeds: Vec::new(),
            revision,
        };
    }
    let mut families_by_target = BTreeMap::<
        AcceptedIdentity,
        BTreeSet<SemanticCommandFamily>,
    >::new();
    for command in &commands {
        let _inserted = families_by_target
            .entry(command.target)
            .or_default()
            .insert(command.preconditions.requested_family);
    }
    let material_count = families_by_target
        .values()
        .map(BTreeSet::len)
        .sum();
    let request = DirectEditBatchIndexRequest {
        families_by_target: &families_by_target,
        material_count,
    };
    let mut batch_index =
        direct_edit_material_index(&current.notebook, request, revision);
    let mut evaluated =
        Vec::<DirectEditBatchCommandPrediction<CommandIdentity>>::with_capacity(
            commands.len(),
        );
    let mut changed_targets =
        BTreeMap::<DirectEditMaterialKey, (usize, usize)>::new();
    let mut remaining = commands.into_iter();
    while let Some(command) = remaining.next() {
        let target = command.target;
        let requested_family = command.preconditions.requested_family;
        let material_key = (target, requested_family);
        let previous = changed_targets
            .get(&material_key)
            .and_then(|(_, last)| evaluated.get(*last))
            .map(|prediction| &prediction.command);
        let result = simulate_direct_edit_batch_command(
            &current.notebook,
            &mut batch_index.materials,
            &batch_index.table_by_cell,
            &mut batch_index.table_overlays,
            command,
            previous,
            revision,
        );
        let prediction = match result {
            Ok(prediction) => prediction,
            Err((rejected_command, reason)) => {
                return reject_direct_edit_batch(
                    rejected_command,
                    remaining,
                    evaluated,
                    reason,
                    revision,
                );
            },
        };
        let command_index = evaluated.len();
        if prediction.change.is_some() {
            record_direct_edit_batch_change_index(
                &mut changed_targets,
                command_index,
                material_key,
            );
        }
        evaluated.push(prediction);
    }
    let changes =
        collect_direct_edit_batch_changes(&evaluated, changed_targets);
    let effect = if changes.is_empty() {
        DirectEditEffectClass::NoOp
    } else {
        DirectEditEffectClass::Mutation
    };
    let impact_seeds = direct_edit_impact_seeds_indexed(
        &current.notebook,
        batch_index.impacts,
        &changes,
    );
    DirectEditBatchSimulationOutcome::Predicted {
        changes,
        commands: evaluated,
        effect,
        impact_seeds,
        revision,
    }
}

fn direct_edit_impact_seeds_indexed(
    notebook: &Notebook<AcceptedIdentity>,
    mut impacts: BTreeMap<DirectEditMaterialKey, DirectEditImpactScope>,
    changes: &[DirectEditSemanticChange],
) -> Vec<DirectEditImpactSeed> {
    collect_direct_edit_impact_seeds(changes, |change| {
        let scope = impacts
            .remove(&(change.target, change.family))
            .unwrap_or_else(|| direct_edit_impact_scope(notebook, change));
        (scope, direct_edit_impact_authorities(change.family))
    })
}

fn direct_edit_impact_seeds(
    notebook: &Notebook<AcceptedIdentity>,
    changes: &[DirectEditSemanticChange],
) -> Vec<DirectEditImpactSeed> {
    collect_direct_edit_impact_seeds(changes, |change| {
        (
            direct_edit_impact_scope(notebook, change),
            direct_edit_impact_authorities(change.family),
        )
    })
}

fn collect_direct_edit_impact_seeds(
    changes: &[DirectEditSemanticChange],
    mut seed_for: impl FnMut(
        &DirectEditSemanticChange,
    ) -> (
        DirectEditImpactScope,
        &'static [DirectEditDerivedAuthority],
    ),
) -> Vec<DirectEditImpactSeed> {
    let mut seeds = BTreeMap::<
        DirectEditImpactScope,
        BTreeSet<DirectEditDerivedAuthority>,
    >::new();
    for change in changes {
        let (scope, authorities) = seed_for(change);
        seeds.entry(scope).or_default().extend(authorities);
    }
    seeds
        .into_iter()
        .map(|(scope, authorities)| DirectEditImpactSeed {
            authorities: authorities.into_iter().collect(),
            scope,
        })
        .collect()
}

fn direct_edit_impact_scope(
    notebook: &Notebook<AcceptedIdentity>,
    change: &DirectEditSemanticChange,
) -> DirectEditImpactScope {
    match change.family {
        SemanticCommandFamily::DocumentConstraint => {
            direct_edit_document_constraint_scope(notebook, change.target)
        },
        SemanticCommandFamily::AssetReference
        | SemanticCommandFamily::OrderingAndGrouping
        | SemanticCommandFamily::StructuredContent
        | SemanticCommandFamily::StyleRole => {
            direct_edit_structured_scope(notebook, change.target)
        },
        SemanticCommandFamily::TextContent => {
            direct_edit_text_scope(notebook, change.target)
        },
        SemanticCommandFamily::Provenance => {
            if matches!(
                change.after,
                EditableSemanticValue::ProvenanceReference(_)
            ) {
                direct_edit_structured_scope(notebook, change.target)
            } else {
                DirectEditImpactScope::Notebook { notebook: notebook.id }
            }
        },
        SemanticCommandFamily::BlockInsertionAndDeletion
        | SemanticCommandFamily::SpatialConstraint => {
            DirectEditImpactScope::Notebook { notebook: notebook.id }
        },
    }
}

const fn direct_edit_impact_authorities(
    family: SemanticCommandFamily,
) -> &'static [DirectEditDerivedAuthority] {
    match family {
        SemanticCommandFamily::DocumentConstraint
        | SemanticCommandFamily::AssetReference
        | SemanticCommandFamily::BlockInsertionAndDeletion
        | SemanticCommandFamily::OrderingAndGrouping
        | SemanticCommandFamily::SpatialConstraint
        | SemanticCommandFamily::StyleRole => {
            &[DirectEditDerivedAuthority::AllDerived]
        },
        SemanticCommandFamily::Provenance => &[
            DirectEditDerivedAuthority::Diagnostics,
            DirectEditDerivedAuthority::Output,
        ],
        SemanticCommandFamily::StructuredContent => &[
            DirectEditDerivedAuthority::Layout,
            DirectEditDerivedAuthority::Output,
            DirectEditDerivedAuthority::StructureValidation,
        ],
        SemanticCommandFamily::TextContent => &[
            DirectEditDerivedAuthority::Diagnostics,
            DirectEditDerivedAuthority::FlowGeometry,
            DirectEditDerivedAuthority::Handwriting,
            DirectEditDerivedAuthority::Motion,
            DirectEditDerivedAuthority::Rendering,
            DirectEditDerivedAuthority::Shaping,
            DirectEditDerivedAuthority::Wrapping,
        ],
    }
}

fn direct_edit_document_constraint_scope(
    notebook: &Notebook<AcceptedIdentity>,
    target: AcceptedIdentity,
) -> DirectEditImpactScope {
    if notebook.pages.iter().any(|page| page.id == target) {
        return DirectEditImpactScope::Pages { pages: vec![target] };
    }
    let pages = notebook
        .pages
        .iter()
        .filter(|page| page.page_profile == target)
        .map(|page| page.id)
        .collect::<Vec<_>>();
    if pages.is_empty() {
        DirectEditImpactScope::Notebook { notebook: notebook.id }
    } else {
        DirectEditImpactScope::Pages { pages }
    }
}

fn direct_edit_text_scope(
    notebook: &Notebook<AcceptedIdentity>,
    target: AcceptedIdentity,
) -> DirectEditImpactScope {
    let Some((_, flow, page)) = direct_edit_ancestor_scope(notebook, target)
    else {
        return DirectEditImpactScope::Notebook { notebook: notebook.id };
    };
    DirectEditImpactScope::Flow { flow, page }
}

fn direct_edit_structured_scope(
    notebook: &Notebook<AcceptedIdentity>,
    target: AcceptedIdentity,
) -> DirectEditImpactScope {
    let Some((block_owner, flow, page)) =
        direct_edit_ancestor_scope(notebook, target)
    else {
        return DirectEditImpactScope::Notebook { notebook: notebook.id };
    };
    let Some(block) = block_owner else {
        return DirectEditImpactScope::Flow { flow, page };
    };
    DirectEditImpactScope::BlockFlow { block, flow, page }
}

fn direct_edit_ancestor_scope(
    notebook: &Notebook<AcceptedIdentity>,
    target: AcceptedIdentity,
) -> Option<(Option<AcceptedIdentity>, AcceptedIdentity, AcceptedIdentity)> {
    let mut block = None;
    let mut current = target;
    loop {
        let descriptor = semantic_identity_descriptor(notebook, current)?;
        if matches!(descriptor.kind, SemanticIdentityKind::Block(_))
            && block.is_none()
        {
            block = Some(current);
        }
        if descriptor.kind == SemanticIdentityKind::Flow {
            return descriptor.owner.map(|page| (block, current, page));
        }
        current = descriptor.owner?;
    }
}

fn simulate_direct_edit_batch_command<CommandIdentity>(
    notebook: &Notebook<AcceptedIdentity>,
    materials: &mut BTreeMap<DirectEditMaterialKey, CommandTargetMaterial>,
    table_by_cell: &BTreeMap<AcceptedIdentity, AcceptedIdentity>,
    table_overlays: &mut BTreeMap<AcceptedIdentity, Table<AcceptedIdentity>>,
    command: DirectEditBatchCommand<CommandIdentity>,
    previous: Option<&CommandIdentity>,
    revision: atrament_semantic_notebook::RevisionIdentity,
) -> Result<
    DirectEditBatchCommandPrediction<CommandIdentity>,
    (
        CommandIdentity,
        DirectEditBatchCommandRejection<CommandIdentity>,
    ),
>
where
    CommandIdentity: Clone + Ord,
{
    let DirectEditBatchCommand {
        dependencies,
        id,
        preconditions,
        requested,
        target,
    } = command;
    if let Some(dependency) = previous
        && !dependencies.contains(dependency)
    {
        return Err((
            id,
            DirectEditBatchCommandRejection::MissingPriorTargetDependency {
                dependency: dependency.clone(),
                target,
            },
        ));
    }
    let (prepared, indexed) = match batch_command_target_material(
        notebook,
        materials,
        revision,
        target,
        preconditions.requested_family,
    ) {
        Ok(material) => material,
        Err(reason) => return Err((id, reason)),
    };
    let metadata = DirectEditBatchMaterialMetadata {
        descriptor: prepared.descriptor,
        indexed,
    };
    let checked = match check_command_target_preconditions_material(
        prepared,
        &preconditions,
    ) {
        CommandTargetPreconditionOutcome::Satisfied { material } => material,
        outcome @ (CommandTargetPreconditionOutcome::FamilyNotExecutable { .. }
        | CommandTargetPreconditionOutcome::KindMismatch { .. }
        | CommandTargetPreconditionOutcome::NoAcceptedRevision
        | CommandTargetPreconditionOutcome::OwnerMismatch { .. }
        | CommandTargetPreconditionOutcome::StaleBase { .. }
        | CommandTargetPreconditionOutcome::TargetNotEditableValue { .. }
        | CommandTargetPreconditionOutcome::TargetNotFound { .. }
        | CommandTargetPreconditionOutcome::ValueMismatch { .. }) => {
            return Err((id, DirectEditBatchCommandRejection::Precondition {
                outcome: Box::new(outcome),
            }));
        },
    };
    let simulation = simulate_prepared_direct_edit(checked, requested);
    let asset_checked = validate_asset_reference(notebook, simulation);
    let profile_checked =
        validate_page_profile_reference(notebook, asset_checked);
    let provenance_checked = validate_provenance_reference(
        notebook, profile_checked,
    );
    let style_checked = validate_style_reference(notebook, provenance_checked);
    let validated_simulation = if metadata.indexed {
        validate_batch_table_cell_span(
            table_by_cell,
            table_overlays,
            style_checked,
        )
    } else {
        style_checked
    };
    batch_command_prediction(materials, id, metadata, validated_simulation)
}

fn batch_command_target_material<CommandIdentity>(
    notebook: &Notebook<AcceptedIdentity>,
    materials: &mut BTreeMap<DirectEditMaterialKey, CommandTargetMaterial>,
    revision: atrament_semantic_notebook::RevisionIdentity,
    target: AcceptedIdentity,
    family: SemanticCommandFamily,
) -> Result<
    (CommandTargetMaterial, bool),
    DirectEditBatchCommandRejection<CommandIdentity>,
> {
    let key = (target, family);
    if let Some(material) = materials.remove(&key) {
        return Ok((material, true));
    }
    match command_target_material_for_family_from_notebook(
        notebook, revision, target, family,
    ) {
        CommandTargetMaterialOutcome::Prepared { material } => {
            Ok((material, true))
        },
        CommandTargetMaterialOutcome::TargetNotFound {
            revision: missing_revision,
            target: missing_target,
        } => Err(DirectEditBatchCommandRejection::Simulation {
            outcome: Box::new(DirectEditSimulationOutcome::TargetNotFound {
                revision: missing_revision,
                target: missing_target,
            }),
        }),
        CommandTargetMaterialOutcome::NoAcceptedRevision => {
            Err(DirectEditBatchCommandRejection::Simulation {
                outcome: Box::new(
                    DirectEditSimulationOutcome::NoAcceptedRevision,
                ),
            })
        },
        CommandTargetMaterialOutcome::StaleBase { current } => {
            Err(DirectEditBatchCommandRejection::Simulation {
                outcome: Box::new(DirectEditSimulationOutcome::StaleBase {
                    current,
                }),
            })
        },
    }
}

fn batch_command_prediction<CommandIdentity>(
    materials: &mut BTreeMap<DirectEditMaterialKey, CommandTargetMaterial>,
    command: CommandIdentity,
    metadata: DirectEditBatchMaterialMetadata,
    simulation: DirectEditSimulation,
) -> Result<
    DirectEditBatchCommandPrediction<CommandIdentity>,
    (CommandIdentity, DirectEditBatchCommandRejection<CommandIdentity>),
> {
    match simulation.outcome {
        DirectEditSimulationOutcome::Applicable {
            family,
            requested,
            revision,
            target,
        } => {
            let Some(before) = simulation.before else {
                return Err((
                    command,
                    DirectEditBatchCommandRejection::Simulation {
                        outcome: Box::new(
                            DirectEditSimulationOutcome::
                                TargetNotEditableValue {
                                kind: metadata.descriptor.kind,
                                revision,
                                target,
                            },
                        ),
                    },
                ));
            };
            if !metadata.indexed {
                return Err((
                    command,
                    DirectEditBatchCommandRejection::Simulation {
                        outcome: Box::new(
                            DirectEditSimulationOutcome::TargetNotFound {
                                revision,
                                target,
                            },
                        ),
                    },
                ));
            }
            let change = DirectEditSemanticChange {
                after: requested.clone(),
                before,
                family,
                target,
            };
            restore_direct_edit_batch_material(
                materials, metadata, requested, revision, target,
            );
            Ok(DirectEditBatchCommandPrediction {
                change: Some(change),
                command,
                family,
                target,
            })
        },
        DirectEditSimulationOutcome::NoOp { family, revision, target } => {
            if metadata.indexed {
                let Some(before) = simulation.before else {
                    return Err((
                        command,
                        DirectEditBatchCommandRejection::Simulation {
                            outcome: Box::new(
                                DirectEditSimulationOutcome::
                                    TargetNotEditableValue {
                                    kind: metadata.descriptor.kind,
                                    revision,
                                    target,
                                },
                            ),
                        },
                    ));
                };
                restore_direct_edit_batch_material(
                    materials, metadata, before, revision, target,
                );
            }
            Ok(DirectEditBatchCommandPrediction {
                change: None,
                command,
                family,
                target,
            })
        },
        outcome @ (DirectEditSimulationOutcome::InvalidAssetReference { .. }
        | DirectEditSimulationOutcome::InvalidPageProfileReference { .. }
        | DirectEditSimulationOutcome::InvalidProvenanceReference { .. }
        | DirectEditSimulationOutcome::InvalidStyleReference { .. }
        | DirectEditSimulationOutcome::InvalidMathematics { .. }
        | DirectEditSimulationOutcome::InvalidPageProfile { .. }
        | DirectEditSimulationOutcome::InvalidTableGrid { .. }
        | DirectEditSimulationOutcome::NoAcceptedRevision
        | DirectEditSimulationOutcome::StaleBase { .. }
        | DirectEditSimulationOutcome::TargetNotEditableValue {
            ..
        }
        | DirectEditSimulationOutcome::TargetNotFound { .. }
        | DirectEditSimulationOutcome::UnsupportedMathematics {
            ..
        }
        | DirectEditSimulationOutcome::ValueFamilyMismatch { .. }) => {
            reject_batch_command_simulation(command, outcome)
        },
    }
}

fn reject_batch_command_simulation<CommandIdentity>(
    command: CommandIdentity,
    outcome: DirectEditSimulationOutcome,
) -> Result<
    DirectEditBatchCommandPrediction<CommandIdentity>,
    (CommandIdentity, DirectEditBatchCommandRejection<CommandIdentity>),
> {
    Err((command, DirectEditBatchCommandRejection::Simulation {
        outcome: Box::new(outcome),
    }))
}

fn restore_direct_edit_batch_material(
    materials: &mut BTreeMap<DirectEditMaterialKey, CommandTargetMaterial>,
    metadata: DirectEditBatchMaterialMetadata,
    editable_value: EditableSemanticValue,
    revision: atrament_semantic_notebook::RevisionIdentity,
    target: AcceptedIdentity,
) {
    let family = direct_edit_family(&editable_value);
    let _previous = materials.insert((target, family), CommandTargetMaterial {
        descriptor: metadata.descriptor,
        direct_edit_family: Some(family),
        editable_value: Some(editable_value),
        revision,
        target,
    });
}

fn record_direct_edit_batch_change_index(
    aggregate: &mut BTreeMap<DirectEditMaterialKey, (usize, usize)>,
    index: usize,
    key: DirectEditMaterialKey,
) {
    if let Some((_, last)) = aggregate.get_mut(&key) {
        *last = index;
    } else {
        let _previous = aggregate.insert(key, (index, index));
    }
}

fn collect_direct_edit_batch_changes<CommandIdentity>(
    evaluated: &[DirectEditBatchCommandPrediction<CommandIdentity>],
    aggregate: BTreeMap<DirectEditMaterialKey, (usize, usize)>,
) -> Vec<DirectEditSemanticChange> {
    let mut ordered = aggregate.into_iter().collect::<Vec<_>>();
    ordered.sort_by_key(|(_, (first, _))| *first);
    ordered
        .into_iter()
        .filter_map(|((target, family), (first, last))| {
            let first_change = evaluated.get(first)?.change.as_ref()?;
            let last_change = evaluated.get(last)?.change.as_ref()?;
            if first_change.before == last_change.after {
                return None;
            }
            Some(DirectEditSemanticChange {
                after: last_change.after.clone(),
                before: first_change.before.clone(),
                family,
                target,
            })
        })
        .collect()
}

fn reject_direct_edit_batch<CommandIdentity>(
    command: CommandIdentity,
    remaining: impl Iterator<Item = DirectEditBatchCommand<CommandIdentity>>,
    evaluated: Vec<DirectEditBatchCommandPrediction<CommandIdentity>>,
    reason: DirectEditBatchCommandRejection<CommandIdentity>,
    revision: atrament_semantic_notebook::RevisionIdentity,
) -> DirectEditBatchSimulationOutcome<CommandIdentity> {
    let not_evaluated = remaining
        .map(|remaining_command| remaining_command.id)
        .collect();
    DirectEditBatchSimulationOutcome::Rejected {
        command,
        evaluated,
        not_evaluated,
        reason: Box::new(reason),
        revision,
    }
}

const fn editable_value_kind(
    value: &EditableSemanticValue,
) -> EditableSemanticValueKind {
    match value {
        EditableSemanticValue::AssetReference(_) => {
            EditableSemanticValueKind::AssetReference
        },
        EditableSemanticValue::ConstraintKind(_) => {
            EditableSemanticValueKind::ConstraintKind
        },
        EditableSemanticValue::Formula { .. } => {
            EditableSemanticValueKind::Formula
        },
        EditableSemanticValue::ListOrdering(_) => {
            EditableSemanticValueKind::ListOrdering
        },
        EditableSemanticValue::StyleReference(_) => {
            EditableSemanticValueKind::StyleReference
        },
        EditableSemanticValue::PageProfile(_) => {
            EditableSemanticValueKind::PageProfile
        },
        EditableSemanticValue::PageProfileReference(_) => {
            EditableSemanticValueKind::PageProfileReference
        },
        EditableSemanticValue::Provenance { .. } => {
            EditableSemanticValueKind::Provenance
        },
        EditableSemanticValue::ProvenanceReference(_) => {
            EditableSemanticValueKind::ProvenanceReference
        },
        EditableSemanticValue::TableCellSpan(_) => {
            EditableSemanticValueKind::TableCellSpan
        },
        EditableSemanticValue::TableRowRole(_) => {
            EditableSemanticValueKind::TableRowRole
        },
        EditableSemanticValue::Text(_) => EditableSemanticValueKind::Text,
    }
}

fn simulate_direct_edit_material(
    material_outcome: CommandTargetMaterialOutcome,
    requested: EditableSemanticValue,
) -> DirectEditSimulation {
    match material_outcome {
        CommandTargetMaterialOutcome::NoAcceptedRevision => {
            DirectEditSimulation {
                before: None,
                outcome: DirectEditSimulationOutcome::NoAcceptedRevision,
            }
        },
        CommandTargetMaterialOutcome::Prepared { material: prepared } => {
            simulate_prepared_direct_edit(prepared, requested)
        },
        CommandTargetMaterialOutcome::StaleBase { current } => {
            DirectEditSimulation {
                before: None,
                outcome: DirectEditSimulationOutcome::StaleBase { current },
            }
        },
        CommandTargetMaterialOutcome::TargetNotFound { revision, target } => {
            DirectEditSimulation {
                before: None,
                outcome: DirectEditSimulationOutcome::TargetNotFound {
                    revision,
                    target,
                },
            }
        },
    }
}

fn simulate_direct_edit_material_in_notebook(
    notebook: Option<&Notebook<AcceptedIdentity>>,
    material_outcome: CommandTargetMaterialOutcome,
    requested: EditableSemanticValue,
) -> DirectEditSimulation {
    let simulation = simulate_direct_edit_material(material_outcome, requested);
    let Some(accepted_notebook) = notebook else {
        return simulation;
    };
    let asset_checked = validate_asset_reference(accepted_notebook, simulation);
    let profile_checked =
        validate_page_profile_reference(accepted_notebook, asset_checked);
    let provenance_checked =
        validate_provenance_reference(accepted_notebook, profile_checked);
    let style_checked =
        validate_style_reference(accepted_notebook, provenance_checked);
    validate_single_table_cell_span(accepted_notebook, style_checked)
}

fn validate_asset_reference(
    notebook: &Notebook<AcceptedIdentity>,
    simulation: DirectEditSimulation,
) -> DirectEditSimulation {
    let DirectEditSimulationOutcome::Applicable {
        requested: EditableSemanticValue::AssetReference(Some(reference)),
        revision,
        target,
        ..
    } = &simulation.outcome
    else {
        return simulation;
    };
    let actual = semantic_identity_descriptor(notebook, *reference)
        .map(|descriptor| descriptor.kind);
    if actual == Some(SemanticIdentityKind::Asset) {
        return simulation;
    }
    DirectEditSimulation {
        before: simulation.before,
        outcome: DirectEditSimulationOutcome::InvalidAssetReference {
            actual,
            reference: *reference,
            revision: *revision,
            target: *target,
        },
    }
}

fn validate_page_profile_reference(
    notebook: &Notebook<AcceptedIdentity>,
    simulation: DirectEditSimulation,
) -> DirectEditSimulation {
    let DirectEditSimulationOutcome::Applicable {
        requested: EditableSemanticValue::PageProfileReference(reference),
        revision,
        target,
        ..
    } = &simulation.outcome
    else {
        return simulation;
    };
    let actual = semantic_identity_descriptor(notebook, *reference)
        .map(|descriptor| descriptor.kind);
    if actual == Some(SemanticIdentityKind::PageProfile) {
        return simulation;
    }
    DirectEditSimulation {
        before: simulation.before,
        outcome: DirectEditSimulationOutcome::InvalidPageProfileReference {
            actual,
            reference: *reference,
            revision: *revision,
            target: *target,
        },
    }
}

fn validate_provenance_reference(
    notebook: &Notebook<AcceptedIdentity>,
    simulation: DirectEditSimulation,
) -> DirectEditSimulation {
    let DirectEditSimulationOutcome::Applicable {
        requested: EditableSemanticValue::ProvenanceReference(Some(reference)),
        revision,
        target,
        ..
    } = &simulation.outcome
    else {
        return simulation;
    };
    let actual = semantic_identity_descriptor(notebook, *reference)
        .map(|descriptor| descriptor.kind);
    if actual == Some(SemanticIdentityKind::Provenance) {
        return simulation;
    }
    DirectEditSimulation {
        before: simulation.before,
        outcome: DirectEditSimulationOutcome::InvalidProvenanceReference {
            actual,
            reference: *reference,
            revision: *revision,
            target: *target,
        },
    }
}

fn validate_style_reference(
    notebook: &Notebook<AcceptedIdentity>,
    simulation: DirectEditSimulation,
) -> DirectEditSimulation {
    let DirectEditSimulationOutcome::Applicable {
        requested: EditableSemanticValue::StyleReference(Some(reference)),
        revision,
        target,
        ..
    } = &simulation.outcome
    else {
        return simulation;
    };
    let actual = semantic_identity_descriptor(notebook, *reference)
        .map(|descriptor| descriptor.kind);
    if actual == Some(SemanticIdentityKind::Style) {
        return simulation;
    }
    DirectEditSimulation {
        before: simulation.before,
        outcome: DirectEditSimulationOutcome::InvalidStyleReference {
            actual,
            reference: *reference,
            revision: *revision,
            target: *target,
        },
    }
}

fn validate_batch_table_cell_span(
    table_by_cell: &BTreeMap<AcceptedIdentity, AcceptedIdentity>,
    table_overlays: &mut BTreeMap<AcceptedIdentity, Table<AcceptedIdentity>>,
    simulation: DirectEditSimulation,
) -> DirectEditSimulation {
    let DirectEditSimulationOutcome::Applicable {
        requested: EditableSemanticValue::TableCellSpan(span),
        revision,
        target,
        ..
    } = &simulation.outcome
    else {
        return simulation;
    };
    let Some(table_id) = table_by_cell.get(target) else {
        return DirectEditSimulation {
            before: simulation.before,
            outcome: DirectEditSimulationOutcome::TargetNotFound {
                revision: *revision,
                target: *target,
            },
        };
    };
    let Some(table) = table_overlays.get_mut(table_id) else {
        return DirectEditSimulation {
            before: simulation.before,
            outcome: DirectEditSimulationOutcome::TargetNotFound {
                revision: *revision,
                target: *target,
            },
        };
    };
    match replace_table_cell_span_in_table(table, *target, *span) {
        Ok(true) => simulation,
        Ok(false) => DirectEditSimulation {
            before: simulation.before,
            outcome: DirectEditSimulationOutcome::TargetNotFound {
                revision: *revision,
                target: *target,
            },
        },
        Err(reason) => DirectEditSimulation {
            before: simulation.before,
            outcome: DirectEditSimulationOutcome::InvalidTableGrid {
                reason,
                revision: *revision,
                target: *target,
            },
        },
    }
}

fn validate_single_table_cell_span(
    notebook: &Notebook<AcceptedIdentity>,
    simulation: DirectEditSimulation,
) -> DirectEditSimulation {
    let DirectEditSimulationOutcome::Applicable {
        requested: EditableSemanticValue::TableCellSpan(span),
        revision,
        target,
        ..
    } = &simulation.outcome
    else {
        return simulation;
    };
    let Some(table) = table_containing_cell_value(notebook, *target) else {
        return DirectEditSimulation {
            before: simulation.before,
            outcome: DirectEditSimulationOutcome::TargetNotFound {
                revision: *revision,
                target: *target,
            },
        };
    };
    let mut candidate = table.clone();
    match replace_table_cell_span_in_table(&mut candidate, *target, *span) {
        Ok(true) => simulation,
        Ok(false) => DirectEditSimulation {
            before: simulation.before,
            outcome: DirectEditSimulationOutcome::TargetNotFound {
                revision: *revision,
                target: *target,
            },
        },
        Err(reason) => DirectEditSimulation {
            before: simulation.before,
            outcome: DirectEditSimulationOutcome::InvalidTableGrid {
                reason,
                revision: *revision,
                target: *target,
            },
        },
    }
}

fn simulate_prepared_direct_edit(
    material: CommandTargetMaterial,
    requested: EditableSemanticValue,
) -> DirectEditSimulation {
    let revision = material.revision;
    let target = material.target;
    let Some(actual) = material.editable_value else {
        return DirectEditSimulation {
            before: None,
            outcome: DirectEditSimulationOutcome::TargetNotEditableValue {
                kind: material.descriptor.kind,
                revision,
                target,
            },
        };
    };
    let actual_kind = editable_value_kind(&actual);
    let requested_kind = editable_value_kind(&requested);
    if actual_kind != requested_kind {
        return DirectEditSimulation {
            before: Some(actual),
            outcome: DirectEditSimulationOutcome::ValueFamilyMismatch {
                actual: actual_kind,
                requested: requested_kind,
                revision,
                target,
            },
        };
    }
    match &requested {
        EditableSemanticValue::Formula { mode, source } => {
            let analyzed = match analyze(source, *mode) {
                Ok(analyzed) => analyzed,
                Err(reason) => {
                    return DirectEditSimulation {
                        before: Some(actual),
                        outcome:
                            DirectEditSimulationOutcome::InvalidMathematics {
                                reason,
                                revision,
                                target,
                            },
                    };
                },
            };
            if !analyzed.is_supported() {
                return DirectEditSimulation {
                    before: Some(actual),
                    outcome:
                        DirectEditSimulationOutcome::UnsupportedMathematics {
                            revision,
                            target,
                        },
                };
            }
        },
        EditableSemanticValue::PageProfile(profile) => {
            if let Err(reason) = profile.validate() {
                return DirectEditSimulation {
                    before: Some(actual),
                    outcome: DirectEditSimulationOutcome::InvalidPageProfile {
                        reason,
                        revision,
                        target,
                    },
                };
            }
        },
        EditableSemanticValue::AssetReference(_)
        | EditableSemanticValue::ConstraintKind(_)
        | EditableSemanticValue::ListOrdering(_)
        | EditableSemanticValue::PageProfileReference(_)
        | EditableSemanticValue::ProvenanceReference(_)
        | EditableSemanticValue::StyleReference(_)
        | EditableSemanticValue::Provenance { .. }
        | EditableSemanticValue::TableCellSpan(_)
        | EditableSemanticValue::TableRowRole(_)
        | EditableSemanticValue::Text(_) => {},
    }
    let family = direct_edit_family(&requested);
    if actual == requested {
        DirectEditSimulation {
            before: Some(actual),
            outcome: DirectEditSimulationOutcome::NoOp {
                family,
                revision,
                target,
            },
        }
    } else {
        DirectEditSimulation {
            before: Some(actual),
            outcome: DirectEditSimulationOutcome::Applicable {
                family,
                requested,
                revision,
                target,
            },
        }
    }
}

const fn direct_edit_family(
    value: &EditableSemanticValue,
) -> SemanticCommandFamily {
    match value {
        EditableSemanticValue::AssetReference(_) => {
            SemanticCommandFamily::AssetReference
        },
        EditableSemanticValue::Formula { .. }
        | EditableSemanticValue::TableCellSpan(_)
        | EditableSemanticValue::TableRowRole(_) => {
            SemanticCommandFamily::StructuredContent
        },
        EditableSemanticValue::ConstraintKind(_)
        | EditableSemanticValue::PageProfile(_)
        | EditableSemanticValue::PageProfileReference(_) => {
            SemanticCommandFamily::DocumentConstraint
        },
        EditableSemanticValue::ListOrdering(_) => {
            SemanticCommandFamily::OrderingAndGrouping
        },
        EditableSemanticValue::Provenance { .. }
        | EditableSemanticValue::ProvenanceReference(_) => {
            SemanticCommandFamily::Provenance
        },
        EditableSemanticValue::StyleReference(_) => {
            SemanticCommandFamily::StyleRole
        },
        EditableSemanticValue::Text(_) => SemanticCommandFamily::TextContent,
    }
}

fn editable_semantic_value(
    notebook: &Notebook<AcceptedIdentity>,
    target: AcceptedIdentity,
    kind: SemanticIdentityKind,
) -> Option<EditableSemanticValue> {
    match kind {
        SemanticIdentityKind::Constraint => constraint_value(notebook, target)
            .map(|constraint| {
                EditableSemanticValue::ConstraintKind(constraint.kind)
            }),
        SemanticIdentityKind::Formula => {
            formula_value(notebook, target).map(|formula| {
                EditableSemanticValue::Formula {
                    mode: formula.mode,
                    source: formula.source.clone(),
                }
            })
        },
        SemanticIdentityKind::InlineSpan => text_value(notebook, target)
            .map(|value| EditableSemanticValue::Text(value.to_owned())),
        SemanticIdentityKind::List => list_ordering_value(notebook, target)
            .map(EditableSemanticValue::ListOrdering),
        SemanticIdentityKind::Page => page_profile_reference_value(
            notebook, target,
        )
        .map(EditableSemanticValue::PageProfileReference),
        SemanticIdentityKind::PageProfile => {
            page_profile_value(notebook, target)
                .map(EditableSemanticValue::PageProfile)
        },
        SemanticIdentityKind::Provenance => provenance_value(notebook, target)
            .map(editable_provenance_value),
        SemanticIdentityKind::TableCell => {
            table_cell_span_value(notebook, target)
                .map(EditableSemanticValue::TableCellSpan)
        },
        SemanticIdentityKind::TableRow => {
            table_row_role_value(notebook, target)
                .map(EditableSemanticValue::TableRowRole)
        },
        SemanticIdentityKind::Figure => figure_value(notebook, target)
            .map(|figure| EditableSemanticValue::AssetReference(figure.asset)),
        SemanticIdentityKind::Block(_) => block_style_value(notebook, target),
        SemanticIdentityKind::Asset
        | SemanticIdentityKind::Flow
        | SemanticIdentityKind::ListItem
        | SemanticIdentityKind::Notebook
        | SemanticIdentityKind::OutputProfile
        | SemanticIdentityKind::Style
        | SemanticIdentityKind::Table => None,
    }
}

fn editable_semantic_value_for_family(
    notebook: &Notebook<AcceptedIdentity>,
    target: AcceptedIdentity,
    kind: SemanticIdentityKind,
    family: SemanticCommandFamily,
) -> Option<EditableSemanticValue> {
    match (kind, family) {
        (SemanticIdentityKind::Block(_), SemanticCommandFamily::Provenance) => {
            block_provenance_reference_value(notebook, target)
        },
        (
            SemanticIdentityKind::InlineSpan,
            SemanticCommandFamily::Provenance,
        ) => {
            inline_span_value(notebook, target).map(|span| {
                EditableSemanticValue::ProvenanceReference(span.provenance)
            })
        },
        (
            SemanticIdentityKind::InlineSpan,
            SemanticCommandFamily::StyleRole,
        ) => {
            inline_span_value(notebook, target)
                .map(|span| EditableSemanticValue::StyleReference(span.style))
        },
        _ => {
            let value = editable_semantic_value(notebook, target, kind)?;
            (direct_edit_family(&value) == family).then_some(value)
        },
    }
}

fn block_provenance_blocks_value(
    blocks: &[Block<AcceptedIdentity>],
    target: AcceptedIdentity,
) -> Option<EditableSemanticValue> {
    for block in blocks {
        if block.id == target {
            return Some(EditableSemanticValue::ProvenanceReference(
                block.provenance,
            ));
        }
        match &block.content {
            BlockContent::Callout(children)
            | BlockContent::Freeform(children) => {
                if let Some(reference) =
                    block_provenance_blocks_value(children, target)
                {
                    return Some(reference);
                }
            },
            BlockContent::List(list) => {
                for item in &list.items {
                    if let Some(reference) =
                        block_provenance_blocks_value(&item.blocks, target)
                    {
                        return Some(reference);
                    }
                }
            },
            BlockContent::Table(table) => {
                for row in &table.rows {
                    for cell in &row.cells {
                        if let Some(reference) =
                            block_provenance_blocks_value(&cell.blocks, target)
                        {
                            return Some(reference);
                        }
                    }
                }
            },
            BlockContent::Date(_)
            | BlockContent::Figure(_)
            | BlockContent::Heading(_)
            | BlockContent::Mathematics(_)
            | BlockContent::Paragraph(_)
            | BlockContent::Rule
            | BlockContent::Unresolved(_) => {},
        }
    }
    None
}

fn block_provenance_reference_value(
    notebook: &Notebook<AcceptedIdentity>,
    target: AcceptedIdentity,
) -> Option<EditableSemanticValue> {
    for page in &notebook.pages {
        for flow in &page.flows {
            if let Some(reference) =
                block_provenance_blocks_value(&flow.blocks, target)
            {
                return Some(reference);
            }
        }
    }
    None
}

fn inline_span_blocks_value(
    blocks: &[Block<AcceptedIdentity>],
    target: AcceptedIdentity,
) -> Option<&InlineSpan<AcceptedIdentity>> {
    for block in blocks {
        if let Some(span) = inline_span_content_value(&block.content, target) {
            return Some(span);
        }
    }
    None
}

fn inline_span_content_value(
    content: &BlockContent<AcceptedIdentity>,
    target: AcceptedIdentity,
) -> Option<&InlineSpan<AcceptedIdentity>> {
    match content {
        BlockContent::Callout(blocks) | BlockContent::Freeform(blocks) => {
            inline_span_blocks_value(blocks, target)
        },
        BlockContent::Date(spans)
        | BlockContent::Heading(spans)
        | BlockContent::Paragraph(spans) => {
            spans.iter().find(|span| span.id == target)
        },
        BlockContent::Figure(figure) => {
            figure.caption.iter().find(|span| span.id == target)
        },
        BlockContent::List(list) => {
            for item in &list.items {
                if let Some(span) =
                    inline_span_blocks_value(&item.blocks, target)
                {
                    return Some(span);
                }
            }
            None
        },
        BlockContent::Table(table) => {
            for row in &table.rows {
                for cell in &row.cells {
                    if let Some(span) =
                        inline_span_blocks_value(&cell.blocks, target)
                    {
                        return Some(span);
                    }
                }
            }
            None
        },
        BlockContent::Mathematics(_)
        | BlockContent::Rule
        | BlockContent::Unresolved(_) => None,
    }
}

fn inline_span_value(
    notebook: &Notebook<AcceptedIdentity>,
    target: AcceptedIdentity,
) -> Option<&InlineSpan<AcceptedIdentity>> {
    for page in &notebook.pages {
        for flow in &page.flows {
            if let Some(span) = inline_span_blocks_value(&flow.blocks, target) {
                return Some(span);
            }
        }
    }
    None
}

fn constraint_value(
    notebook: &Notebook<AcceptedIdentity>,
    target: AcceptedIdentity,
) -> Option<&Constraint<AcceptedIdentity>> {
    notebook
        .constraints
        .iter()
        .find(|constraint| constraint.id == target)
}

fn editable_provenance_value(
    provenance: &Provenance<AcceptedIdentity>,
) -> EditableSemanticValue {
    EditableSemanticValue::Provenance {
        kind: provenance.kind,
        reference: provenance.reference.clone(),
    }
}

fn provenance_value(
    notebook: &Notebook<AcceptedIdentity>,
    target: AcceptedIdentity,
) -> Option<&Provenance<AcceptedIdentity>> {
    notebook
        .provenance
        .iter()
        .find(|provenance| provenance.id == target)
}

fn list_ordering_blocks_value(
    blocks: &[Block<AcceptedIdentity>],
    target: AcceptedIdentity,
) -> Option<bool> {
    for block in blocks {
        match &block.content {
            BlockContent::Callout(children)
            | BlockContent::Freeform(children) => {
                if let Some(ordered) =
                    list_ordering_blocks_value(children, target)
                {
                    return Some(ordered);
                }
            },
            BlockContent::List(list) => {
                if list.id == target {
                    return Some(list.ordered);
                }
                for item in &list.items {
                    if let Some(ordered) =
                        list_ordering_blocks_value(&item.blocks, target)
                    {
                        return Some(ordered);
                    }
                }
            },
            BlockContent::Table(table) => {
                for row in &table.rows {
                    for cell in &row.cells {
                        if let Some(ordered) =
                            list_ordering_blocks_value(&cell.blocks, target)
                        {
                            return Some(ordered);
                        }
                    }
                }
            },
            BlockContent::Date(_)
            | BlockContent::Figure(_)
            | BlockContent::Heading(_)
            | BlockContent::Mathematics(_)
            | BlockContent::Paragraph(_)
            | BlockContent::Rule
            | BlockContent::Unresolved(_) => {},
        }
    }
    None
}

fn list_ordering_value(
    notebook: &Notebook<AcceptedIdentity>,
    target: AcceptedIdentity,
) -> Option<bool> {
    for page in &notebook.pages {
        for flow in &page.flows {
            if let Some(ordered) =
                list_ordering_blocks_value(&flow.blocks, target)
            {
                return Some(ordered);
            }
        }
    }
    None
}

fn replace_list_ordering_blocks(
    blocks: &mut [Block<AcceptedIdentity>],
    target: AcceptedIdentity,
    ordered: bool,
) -> bool {
    for block in blocks {
        match &mut block.content {
            BlockContent::Callout(children)
            | BlockContent::Freeform(children) => {
                if replace_list_ordering_blocks(children, target, ordered) {
                    return true;
                }
            },
            BlockContent::List(list) => {
                if list.id == target {
                    list.ordered = ordered;
                    return true;
                }
                for item in &mut list.items {
                    if replace_list_ordering_blocks(
                        &mut item.blocks, target, ordered,
                    ) {
                        return true;
                    }
                }
            },
            BlockContent::Table(table) => {
                for row in &mut table.rows {
                    for cell in &mut row.cells {
                        if replace_list_ordering_blocks(
                            &mut cell.blocks,
                            target,
                            ordered,
                        ) {
                            return true;
                        }
                    }
                }
            },
            BlockContent::Date(_)
            | BlockContent::Figure(_)
            | BlockContent::Heading(_)
            | BlockContent::Mathematics(_)
            | BlockContent::Paragraph(_)
            | BlockContent::Rule
            | BlockContent::Unresolved(_) => {},
        }
    }
    false
}

fn replace_list_ordering_value(
    notebook: &mut Notebook<AcceptedIdentity>,
    target: AcceptedIdentity,
    ordered: bool,
) -> bool {
    notebook.pages.iter_mut().any(|page| {
        page.flows.iter_mut().any(|flow| {
            replace_list_ordering_blocks(&mut flow.blocks, target, ordered)
        })
    })
}

fn block_style_blocks_value(
    blocks: &[Block<AcceptedIdentity>],
    target: AcceptedIdentity,
) -> Option<EditableSemanticValue> {
    for block in blocks {
        if block.id == target {
            return Some(EditableSemanticValue::StyleReference(block.style));
        }
        match &block.content {
            BlockContent::Callout(children)
            | BlockContent::Freeform(children) => {
                if let Some(style) =
                    block_style_blocks_value(children, target)
                {
                    return Some(style);
                }
            },
            BlockContent::List(list) => {
                for item in &list.items {
                    if let Some(style) =
                        block_style_blocks_value(&item.blocks, target)
                    {
                        return Some(style);
                    }
                }
            },
            BlockContent::Table(table) => {
                for row in &table.rows {
                    for cell in &row.cells {
                        if let Some(style) =
                            block_style_blocks_value(&cell.blocks, target)
                        {
                            return Some(style);
                        }
                    }
                }
            },
            BlockContent::Date(_)
            | BlockContent::Figure(_)
            | BlockContent::Heading(_)
            | BlockContent::Mathematics(_)
            | BlockContent::Paragraph(_)
            | BlockContent::Rule
            | BlockContent::Unresolved(_) => {},
        }
    }
    None
}

fn block_style_value(
    notebook: &Notebook<AcceptedIdentity>,
    target: AcceptedIdentity,
) -> Option<EditableSemanticValue> {
    for page in &notebook.pages {
        for flow in &page.flows {
            if let Some(style) =
                block_style_blocks_value(&flow.blocks, target)
            {
                return Some(style);
            }
        }
    }
    None
}

fn replace_block_style_blocks(
    blocks: &mut [Block<AcceptedIdentity>],
    target: AcceptedIdentity,
    style: Option<AcceptedIdentity>,
) -> bool {
    for block in blocks {
        if block.id == target {
            block.style = style;
            return true;
        }
        match &mut block.content {
            BlockContent::Callout(children)
            | BlockContent::Freeform(children) => {
                if replace_block_style_blocks(children, target, style) {
                    return true;
                }
            },
            BlockContent::List(list) => {
                for item in &mut list.items {
                    if replace_block_style_blocks(
                        &mut item.blocks, target, style,
                    ) {
                        return true;
                    }
                }
            },
            BlockContent::Table(table) => {
                for row in &mut table.rows {
                    for cell in &mut row.cells {
                        if replace_block_style_blocks(
                            &mut cell.blocks,
                            target,
                            style,
                        ) {
                            return true;
                        }
                    }
                }
            },
            BlockContent::Date(_)
            | BlockContent::Figure(_)
            | BlockContent::Heading(_)
            | BlockContent::Mathematics(_)
            | BlockContent::Paragraph(_)
            | BlockContent::Rule
            | BlockContent::Unresolved(_) => {},
        }
    }
    false
}

fn replace_block_style_value(
    notebook: &mut Notebook<AcceptedIdentity>,
    target: AcceptedIdentity,
    style: Option<AcceptedIdentity>,
) -> bool {
    notebook.pages.iter_mut().any(|page| {
        page.flows.iter_mut().any(|flow| {
            replace_block_style_blocks(&mut flow.blocks, target, style)
        })
    })
}

fn replace_inline_span_style_blocks(
    blocks: &mut [Block<AcceptedIdentity>],
    target: AcceptedIdentity,
    style: Option<AcceptedIdentity>,
) -> bool {
    for block in blocks {
        match &mut block.content {
            BlockContent::Callout(children)
            | BlockContent::Freeform(children) => {
                if replace_inline_span_style_blocks(children, target, style) {
                    return true;
                }
            },
            BlockContent::Date(spans)
            | BlockContent::Heading(spans)
            | BlockContent::Paragraph(spans) => {
                if let Some(span) =
                    spans.iter_mut().find(|span| span.id == target)
                {
                    span.style = style;
                    return true;
                }
            },
            BlockContent::Figure(figure) => {
                if let Some(span) =
                    figure.caption.iter_mut().find(|span| span.id == target)
                {
                    span.style = style;
                    return true;
                }
            },
            BlockContent::List(list) => {
                for item in &mut list.items {
                    if replace_inline_span_style_blocks(
                        &mut item.blocks,
                        target,
                        style,
                    ) {
                        return true;
                    }
                }
            },
            BlockContent::Table(table) => {
                for row in &mut table.rows {
                    for cell in &mut row.cells {
                        if replace_inline_span_style_blocks(
                            &mut cell.blocks,
                            target,
                            style,
                        ) {
                            return true;
                        }
                    }
                }
            },
            BlockContent::Mathematics(_)
            | BlockContent::Rule
            | BlockContent::Unresolved(_) => {},
        }
    }
    false
}

fn replace_inline_span_style_value(
    notebook: &mut Notebook<AcceptedIdentity>,
    target: AcceptedIdentity,
    style: Option<AcceptedIdentity>,
) -> bool {
    notebook.pages.iter_mut().any(|page| {
        page.flows.iter_mut().any(|flow| {
            replace_inline_span_style_blocks(&mut flow.blocks, target, style)
        })
    })
}

fn replace_style_reference_value(
    notebook: &mut Notebook<AcceptedIdentity>,
    target: AcceptedIdentity,
    style: Option<AcceptedIdentity>,
) -> bool {
    replace_block_style_value(notebook, target, style)
        || replace_inline_span_style_value(notebook, target, style)
}

fn replace_provenance_reference_blocks(
    blocks: &mut [Block<AcceptedIdentity>],
    target: AcceptedIdentity,
    provenance: Option<AcceptedIdentity>,
) -> bool {
    for block in blocks {
        if block.id == target {
            block.provenance = provenance;
            return true;
        }
        match &mut block.content {
            BlockContent::Callout(children)
            | BlockContent::Freeform(children) => {
                if replace_provenance_reference_blocks(
                    children,
                    target,
                    provenance,
                ) {
                    return true;
                }
            },
            BlockContent::Date(spans)
            | BlockContent::Heading(spans)
            | BlockContent::Paragraph(spans) => {
                if let Some(span) =
                    spans.iter_mut().find(|span| span.id == target)
                {
                    span.provenance = provenance;
                    return true;
                }
            },
            BlockContent::Figure(figure) => {
                if let Some(span) =
                    figure.caption.iter_mut().find(|span| span.id == target)
                {
                    span.provenance = provenance;
                    return true;
                }
            },
            BlockContent::List(list) => {
                for item in &mut list.items {
                    if replace_provenance_reference_blocks(
                        &mut item.blocks,
                        target,
                        provenance,
                    ) {
                        return true;
                    }
                }
            },
            BlockContent::Table(table) => {
                for row in &mut table.rows {
                    for cell in &mut row.cells {
                        if replace_provenance_reference_blocks(
                            &mut cell.blocks,
                            target,
                            provenance,
                        ) {
                            return true;
                        }
                    }
                }
            },
            BlockContent::Mathematics(_)
            | BlockContent::Rule
            | BlockContent::Unresolved(_) => {},
        }
    }
    false
}

fn replace_provenance_reference_value(
    notebook: &mut Notebook<AcceptedIdentity>,
    target: AcceptedIdentity,
    provenance: Option<AcceptedIdentity>,
) -> bool {
    notebook.pages.iter_mut().any(|page| {
        page.flows.iter_mut().any(|flow| {
            replace_provenance_reference_blocks(
                &mut flow.blocks,
                target,
                provenance,
            )
        })
    })
}

fn figure_blocks_value(
    blocks: &[Block<AcceptedIdentity>],
    target: AcceptedIdentity,
) -> Option<&Figure<AcceptedIdentity>> {
    for block in blocks {
        match &block.content {
            BlockContent::Callout(children)
            | BlockContent::Freeform(children) => {
                if let Some(figure) = figure_blocks_value(children, target) {
                    return Some(figure);
                }
            },
            BlockContent::Figure(figure) if figure.id == target => {
                return Some(figure);
            },
            BlockContent::List(list) => {
                for item in &list.items {
                    if let Some(figure) =
                        figure_blocks_value(&item.blocks, target)
                    {
                        return Some(figure);
                    }
                }
            },
            BlockContent::Table(table) => {
                for row in &table.rows {
                    for cell in &row.cells {
                        if let Some(figure) =
                            figure_blocks_value(&cell.blocks, target)
                        {
                            return Some(figure);
                        }
                    }
                }
            },
            BlockContent::Date(_)
            | BlockContent::Figure(_)
            | BlockContent::Heading(_)
            | BlockContent::Mathematics(_)
            | BlockContent::Paragraph(_)
            | BlockContent::Rule
            | BlockContent::Unresolved(_) => {},
        }
    }
    None
}

fn figure_value(
    notebook: &Notebook<AcceptedIdentity>,
    target: AcceptedIdentity,
) -> Option<&Figure<AcceptedIdentity>> {
    for page in &notebook.pages {
        for flow in &page.flows {
            if let Some(figure) = figure_blocks_value(&flow.blocks, target) {
                return Some(figure);
            }
        }
    }
    None
}

fn replace_figure_asset_blocks(
    blocks: &mut [Block<AcceptedIdentity>],
    target: AcceptedIdentity,
    asset: Option<AcceptedIdentity>,
) -> bool {
    for block in blocks {
        match &mut block.content {
            BlockContent::Callout(children)
            | BlockContent::Freeform(children) => {
                if replace_figure_asset_blocks(children, target, asset) {
                    return true;
                }
            },
            BlockContent::Figure(figure) if figure.id == target => {
                figure.asset = asset;
                return true;
            },
            BlockContent::List(list) => {
                for item in &mut list.items {
                    if replace_figure_asset_blocks(
                        &mut item.blocks,
                        target,
                        asset,
                    ) {
                        return true;
                    }
                }
            },
            BlockContent::Table(table) => {
                for row in &mut table.rows {
                    for cell in &mut row.cells {
                        if replace_figure_asset_blocks(
                            &mut cell.blocks,
                            target,
                            asset,
                        ) {
                            return true;
                        }
                    }
                }
            },
            BlockContent::Date(_)
            | BlockContent::Figure(_)
            | BlockContent::Heading(_)
            | BlockContent::Mathematics(_)
            | BlockContent::Paragraph(_)
            | BlockContent::Rule
            | BlockContent::Unresolved(_) => {},
        }
    }
    false
}

fn replace_figure_asset_value(
    notebook: &mut Notebook<AcceptedIdentity>,
    target: AcceptedIdentity,
    asset: Option<AcceptedIdentity>,
) -> bool {
    for page in &mut notebook.pages {
        for flow in &mut page.flows {
            if replace_figure_asset_blocks(&mut flow.blocks, target, asset) {
                return true;
            }
        }
    }
    false
}

fn formula_value(
    notebook: &Notebook<AcceptedIdentity>,
    target: AcceptedIdentity,
) -> Option<&Formula<AcceptedIdentity>> {
    for page in &notebook.pages {
        for flow in &page.flows {
            if let Some(formula) = formula_blocks_value(&flow.blocks, target) {
                return Some(formula);
            }
        }
    }
    None
}

fn page_profile_reference_value(
    notebook: &Notebook<AcceptedIdentity>,
    target: AcceptedIdentity,
) -> Option<AcceptedIdentity> {
    notebook
        .pages
        .iter()
        .find(|page| page.id == target)
        .map(|page| page.page_profile)
}

fn page_profile_value(
    notebook: &Notebook<AcceptedIdentity>,
    target: AcceptedIdentity,
) -> Option<atrament_semantic_notebook::PhysicalPageProfile> {
    notebook
        .page_profiles
        .iter()
        .find(|profile| profile.id == target)
        .map(|profile| profile.geometry)
}

fn replace_formula_blocks(
    blocks: &mut [Block<AcceptedIdentity>],
    target: AcceptedIdentity,
    replacement: &mut Option<(FormulaMode, String)>,
) -> bool {
    for block in blocks {
        if replace_formula_content(&mut block.content, target, replacement) {
            return true;
        }
    }
    false
}

fn replace_formula_content(
    content: &mut BlockContent<AcceptedIdentity>,
    target: AcceptedIdentity,
    replacement: &mut Option<(FormulaMode, String)>,
) -> bool {
    match content {
        BlockContent::Callout(blocks) | BlockContent::Freeform(blocks) => {
            replace_formula_blocks(blocks, target, replacement)
        },
        BlockContent::List(list) => {
            for item in &mut list.items {
                if replace_formula_blocks(&mut item.blocks, target, replacement)
                {
                    return true;
                }
            }
            false
        },
        BlockContent::Mathematics(formula) if formula.id == target => {
            let Some((mode, source)) = replacement.take() else {
                return false;
            };
            formula.mode = mode;
            formula.source = source;
            true
        },
        BlockContent::Table(table) => {
            for row in &mut table.rows {
                for cell in &mut row.cells {
                    if replace_formula_blocks(
                        &mut cell.blocks,
                        target,
                        replacement,
                    ) {
                        return true;
                    }
                }
            }
            false
        },
        BlockContent::Date(_)
        | BlockContent::Figure(_)
        | BlockContent::Heading(_)
        | BlockContent::Mathematics(_)
        | BlockContent::Paragraph(_)
        | BlockContent::Rule
        | BlockContent::Unresolved(_) => false,
    }
}

fn apply_direct_edit_changes(
    notebook: &mut Notebook<AcceptedIdentity>,
    changes: &[DirectEditSemanticChange],
) -> Result<(), AcceptedIdentity> {
    let mut affected_tables =
        BTreeMap::<AcceptedIdentity, AcceptedIdentity>::new();
    for change in changes {
        if matches!(change.after, EditableSemanticValue::TableCellSpan(_)) {
            let Some(table) =
                table_containing_cell_value(notebook, change.target)
            else {
                return Err(change.target);
            };
            let _first_target =
                affected_tables.entry(table.id).or_insert(change.target);
        }
        if !apply_direct_edit_change(notebook, change) {
            return Err(change.target);
        }
    }
    for target in affected_tables.into_values() {
        let Some(table) = table_containing_cell_value(notebook, target) else {
            return Err(target);
        };
        if table.validate_grid().is_err() {
            return Err(target);
        }
    }
    Ok(())
}

fn apply_direct_edit_change(
    notebook: &mut Notebook<AcceptedIdentity>,
    change: &DirectEditSemanticChange,
) -> bool {
    match &change.after {
        EditableSemanticValue::AssetReference(reference) => {
            replace_figure_asset_value(notebook, change.target, *reference)
        },
        EditableSemanticValue::ConstraintKind(kind) => {
            replace_constraint_kind_value(notebook, change.target, *kind)
        },
        EditableSemanticValue::Formula { mode, source } => {
            replace_formula_value(
                notebook,
                change.target,
                *mode,
                source.clone(),
            )
        },
        EditableSemanticValue::ListOrdering(ordered) => {
            replace_list_ordering_value(notebook, change.target, *ordered)
        },
        EditableSemanticValue::PageProfile(profile) => {
            replace_page_profile_value(
                notebook,
                change.target,
                *profile,
            )
        },
        EditableSemanticValue::PageProfileReference(reference) => {
            replace_page_profile_reference_value(
                notebook,
                change.target,
                *reference,
            )
        },
        EditableSemanticValue::Provenance { kind, reference } => {
            replace_provenance_value(
                notebook,
                change.target,
                *kind,
                reference.clone(),
            )
        },
        EditableSemanticValue::ProvenanceReference(reference) => {
            replace_provenance_reference_value(
                notebook,
                change.target,
                *reference,
            )
        },
        EditableSemanticValue::StyleReference(reference) => {
            replace_style_reference_value(notebook, change.target, *reference)
        },
        EditableSemanticValue::TableCellSpan(span) => {
            replace_table_cell_span_raw_value(notebook, change.target, *span)
        },
        EditableSemanticValue::TableRowRole(role) => {
            replace_table_row_role_value(notebook, change.target, *role)
        },
        EditableSemanticValue::Text(text) => {
            replace_text_value(notebook, change.target, text.clone())
        },
    }
}

fn replace_constraint_kind_value(
    notebook: &mut Notebook<AcceptedIdentity>,
    target: AcceptedIdentity,
    kind: ConstraintKind,
) -> bool {
    let Some(constraint) = notebook
        .constraints
        .iter_mut()
        .find(|constraint| constraint.id == target)
    else {
        return false;
    };
    constraint.kind = kind;
    true
}

fn replace_formula_value(
    notebook: &mut Notebook<AcceptedIdentity>,
    target: AcceptedIdentity,
    mode: FormulaMode,
    source: String,
) -> bool {
    let mut replacement = Some((mode, source));
    for page in &mut notebook.pages {
        for flow in &mut page.flows {
            if replace_formula_blocks(
                &mut flow.blocks,
                target,
                &mut replacement,
            ) {
                return true;
            }
        }
    }
    false
}

fn replace_provenance_value(
    notebook: &mut Notebook<AcceptedIdentity>,
    target: AcceptedIdentity,
    kind: ProvenanceKind,
    reference: Option<String>,
) -> bool {
    let Some(provenance) = notebook
        .provenance
        .iter_mut()
        .find(|provenance| provenance.id == target)
    else {
        return false;
    };
    provenance.kind = kind;
    provenance.reference = reference;
    true
}

fn replace_page_profile_reference_value(
    notebook: &mut Notebook<AcceptedIdentity>,
    target: AcceptedIdentity,
    reference: AcceptedIdentity,
) -> bool {
    let Some(page) = notebook.pages.iter_mut().find(|page| page.id == target)
    else {
        return false;
    };
    page.page_profile = reference;
    true
}

fn replace_page_profile_value(
    notebook: &mut Notebook<AcceptedIdentity>,
    target: AcceptedIdentity,
    geometry: atrament_semantic_notebook::PhysicalPageProfile,
) -> bool {
    let Some(profile) = notebook
        .page_profiles
        .iter_mut()
        .find(|profile| profile.id == target)
    else {
        return false;
    };
    profile.geometry = geometry;
    true
}

fn replace_table_cell_span_in_table(
    table: &mut Table<AcceptedIdentity>,
    target: AcceptedIdentity,
    span: TableCellSpan,
) -> Result<bool, TableGridError<AcceptedIdentity>> {
    let previous_span = table.rows.iter_mut().find_map(|row| {
        row.cells.iter_mut().find_map(|cell| {
            if cell.id != target {
                return None;
            }
            let previous = cell.span;
            cell.span = span;
            Some(previous)
        })
    });
    let Some(previous) = previous_span else {
        return Ok(false);
    };
    if let Err(reason) = table.validate_grid() {
        for row in &mut table.rows {
            if let Some(cell) =
                row.cells.iter_mut().find(|cell| cell.id == target)
            {
                cell.span = previous;
                break;
            }
        }
        return Err(reason);
    }
    Ok(true)
}

fn table_containing_cell_blocks_value(
    blocks: &[Block<AcceptedIdentity>],
    target: AcceptedIdentity,
) -> Option<&Table<AcceptedIdentity>> {
    for block in blocks {
        if let Some(table) = table_containing_cell_content_value(
            &block.content,
            target,
        ) {
            return Some(table);
        }
    }
    None
}

fn table_containing_cell_content_value(
    content: &BlockContent<AcceptedIdentity>,
    target: AcceptedIdentity,
) -> Option<&Table<AcceptedIdentity>> {
    match content {
        BlockContent::Callout(blocks) | BlockContent::Freeform(blocks) => {
            table_containing_cell_blocks_value(blocks, target)
        },
        BlockContent::List(list) => {
            for item in &list.items {
                if let Some(table) =
                    table_containing_cell_blocks_value(&item.blocks, target)
                {
                    return Some(table);
                }
            }
            None
        },
        BlockContent::Table(table) => {
            if table
                .rows
                .iter()
                .any(|row| row.cells.iter().any(|cell| cell.id == target))
            {
                return Some(table);
            }
            for row in &table.rows {
                for cell in &row.cells {
                    if let Some(nested) =
                        table_containing_cell_blocks_value(&cell.blocks, target)
                    {
                        return Some(nested);
                    }
                }
            }
            None
        },
        BlockContent::Date(_)
        | BlockContent::Figure(_)
        | BlockContent::Heading(_)
        | BlockContent::Mathematics(_)
        | BlockContent::Paragraph(_)
        | BlockContent::Rule
        | BlockContent::Unresolved(_) => None,
    }
}

fn table_containing_cell_value(
    notebook: &Notebook<AcceptedIdentity>,
    target: AcceptedIdentity,
) -> Option<&Table<AcceptedIdentity>> {
    for page in &notebook.pages {
        for flow in &page.flows {
            if let Some(table) =
                table_containing_cell_blocks_value(&flow.blocks, target)
            {
                return Some(table);
            }
        }
    }
    None
}

fn replace_table_cell_span_raw_blocks(
    blocks: &mut [Block<AcceptedIdentity>],
    target: AcceptedIdentity,
    span: TableCellSpan,
) -> bool {
    blocks.iter_mut().any(|block| {
        replace_table_cell_span_raw_content(&mut block.content, target, span)
    })
}

fn replace_table_cell_span_raw_content(
    content: &mut BlockContent<AcceptedIdentity>,
    target: AcceptedIdentity,
    span: TableCellSpan,
) -> bool {
    match content {
        BlockContent::Callout(blocks) | BlockContent::Freeform(blocks) => {
            replace_table_cell_span_raw_blocks(blocks, target, span)
        },
        BlockContent::List(list) => list.items.iter_mut().any(|item| {
            replace_table_cell_span_raw_blocks(&mut item.blocks, target, span)
        }),
        BlockContent::Table(table) => {
            for row in &mut table.rows {
                if let Some(cell) =
                    row.cells.iter_mut().find(|cell| cell.id == target)
                {
                    cell.span = span;
                    return true;
                }
            }
            table.rows.iter_mut().any(|row| {
                row.cells.iter_mut().any(|cell| {
                    replace_table_cell_span_raw_blocks(
                        &mut cell.blocks,
                        target,
                        span,
                    )
                })
            })
        },
        BlockContent::Date(_)
        | BlockContent::Figure(_)
        | BlockContent::Heading(_)
        | BlockContent::Mathematics(_)
        | BlockContent::Paragraph(_)
        | BlockContent::Rule
        | BlockContent::Unresolved(_) => false,
    }
}

fn replace_table_cell_span_raw_value(
    notebook: &mut Notebook<AcceptedIdentity>,
    target: AcceptedIdentity,
    span: TableCellSpan,
) -> bool {
    notebook.pages.iter_mut().any(|page| {
        page.flows.iter_mut().any(|flow| {
            replace_table_cell_span_raw_blocks(&mut flow.blocks, target, span)
        })
    })
}

fn replace_table_cell_span_blocks(
    blocks: &mut [Block<AcceptedIdentity>],
    target: AcceptedIdentity,
    span: TableCellSpan,
) -> Result<bool, TableGridError<AcceptedIdentity>> {
    for block in blocks {
        if replace_table_cell_span_content(&mut block.content, target, span)? {
            return Ok(true);
        }
    }
    Ok(false)
}

fn replace_table_cell_span_content(
    content: &mut BlockContent<AcceptedIdentity>,
    target: AcceptedIdentity,
    span: TableCellSpan,
) -> Result<bool, TableGridError<AcceptedIdentity>> {
    match content {
        BlockContent::Callout(blocks) | BlockContent::Freeform(blocks) => {
            replace_table_cell_span_blocks(blocks, target, span)
        },
        BlockContent::List(list) => {
            for item in &mut list.items {
                if replace_table_cell_span_blocks(
                    &mut item.blocks,
                    target,
                    span,
                )? {
                    return Ok(true);
                }
            }
            Ok(false)
        },
        BlockContent::Table(table) => {
            let mut changed = false;
            for row in &mut table.rows {
                for cell in &mut row.cells {
                    if cell.id == target {
                        cell.span = span;
                        changed = true;
                        break;
                    }
                }
                if changed {
                    break;
                }
            }
            if changed {
                table.validate_grid()?;
                return Ok(true);
            }
            for row in &mut table.rows {
                for cell in &mut row.cells {
                    if replace_table_cell_span_blocks(
                        &mut cell.blocks,
                        target,
                        span,
                    )? {
                        return Ok(true);
                    }
                }
            }
            Ok(false)
        },
        BlockContent::Date(_)
        | BlockContent::Figure(_)
        | BlockContent::Heading(_)
        | BlockContent::Mathematics(_)
        | BlockContent::Paragraph(_)
        | BlockContent::Rule
        | BlockContent::Unresolved(_) => Ok(false),
    }
}

fn replace_table_cell_span_value(
    notebook: &mut Notebook<AcceptedIdentity>,
    target: AcceptedIdentity,
    span: TableCellSpan,
) -> Result<bool, TableGridError<AcceptedIdentity>> {
    for page in &mut notebook.pages {
        for flow in &mut page.flows {
            if replace_table_cell_span_blocks(
                &mut flow.blocks,
                target,
                span,
            )? {
                return Ok(true);
            }
        }
    }
    Ok(false)
}

fn table_cell_span_blocks_value(
    blocks: &[Block<AcceptedIdentity>],
    target: AcceptedIdentity,
) -> Option<TableCellSpan> {
    for block in blocks {
        if let Some(span) =
            table_cell_span_content_value(&block.content, target)
        {
            return Some(span);
        }
    }
    None
}

fn table_cell_span_content_value(
    content: &BlockContent<AcceptedIdentity>,
    target: AcceptedIdentity,
) -> Option<TableCellSpan> {
    match content {
        BlockContent::Callout(blocks) | BlockContent::Freeform(blocks) => {
            table_cell_span_blocks_value(blocks, target)
        },
        BlockContent::List(list) => {
            for item in &list.items {
                if let Some(span) =
                    table_cell_span_blocks_value(&item.blocks, target)
                {
                    return Some(span);
                }
            }
            None
        },
        BlockContent::Table(table) => {
            for row in &table.rows {
                for cell in &row.cells {
                    if cell.id == target {
                        return Some(cell.span);
                    }
                }
            }
            for row in &table.rows {
                for cell in &row.cells {
                    if let Some(span) =
                        table_cell_span_blocks_value(&cell.blocks, target)
                    {
                        return Some(span);
                    }
                }
            }
            None
        },
        BlockContent::Date(_)
        | BlockContent::Figure(_)
        | BlockContent::Heading(_)
        | BlockContent::Mathematics(_)
        | BlockContent::Paragraph(_)
        | BlockContent::Rule
        | BlockContent::Unresolved(_) => None,
    }
}

fn table_cell_span_value(
    notebook: &Notebook<AcceptedIdentity>,
    target: AcceptedIdentity,
) -> Option<TableCellSpan> {
    for page in &notebook.pages {
        for flow in &page.flows {
            if let Some(span) =
                table_cell_span_blocks_value(&flow.blocks, target)
            {
                return Some(span);
            }
        }
    }
    None
}

fn replace_table_row_role_blocks(
    blocks: &mut [Block<AcceptedIdentity>],
    target: AcceptedIdentity,
    role: TableRowRole,
) -> bool {
    for block in blocks {
        if replace_table_row_role_content(&mut block.content, target, role) {
            return true;
        }
    }
    false
}

fn replace_table_row_role_content(
    content: &mut BlockContent<AcceptedIdentity>,
    target: AcceptedIdentity,
    role: TableRowRole,
) -> bool {
    match content {
        BlockContent::Callout(blocks) | BlockContent::Freeform(blocks) => {
            replace_table_row_role_blocks(blocks, target, role)
        },
        BlockContent::List(list) => list.items.iter_mut().any(|item| {
            replace_table_row_role_blocks(&mut item.blocks, target, role)
        }),
        BlockContent::Table(table) => {
            for row in &mut table.rows {
                if row.id == target {
                    row.role = role;
                    return true;
                }
                for cell in &mut row.cells {
                    if replace_table_row_role_blocks(
                        &mut cell.blocks,
                        target,
                        role,
                    ) {
                        return true;
                    }
                }
            }
            false
        },
        BlockContent::Date(_)
        | BlockContent::Figure(_)
        | BlockContent::Heading(_)
        | BlockContent::Mathematics(_)
        | BlockContent::Paragraph(_)
        | BlockContent::Rule
        | BlockContent::Unresolved(_) => false,
    }
}

fn replace_table_row_role_value(
    notebook: &mut Notebook<AcceptedIdentity>,
    target: AcceptedIdentity,
    role: TableRowRole,
) -> bool {
    for page in &mut notebook.pages {
        for flow in &mut page.flows {
            if replace_table_row_role_blocks(&mut flow.blocks, target, role) {
                return true;
            }
        }
    }
    false
}

fn table_row_role_blocks_value(
    blocks: &[Block<AcceptedIdentity>],
    target: AcceptedIdentity,
) -> Option<TableRowRole> {
    for block in blocks {
        if let Some(role) = table_row_role_content_value(&block.content, target)
        {
            return Some(role);
        }
    }
    None
}

fn table_row_role_content_value(
    content: &BlockContent<AcceptedIdentity>,
    target: AcceptedIdentity,
) -> Option<TableRowRole> {
    match content {
        BlockContent::Callout(blocks) | BlockContent::Freeform(blocks) => {
            table_row_role_blocks_value(blocks, target)
        },
        BlockContent::List(list) => {
            for item in &list.items {
                if let Some(role) =
                    table_row_role_blocks_value(&item.blocks, target)
                {
                    return Some(role);
                }
            }
            None
        },
        BlockContent::Table(table) => {
            for row in &table.rows {
                if row.id == target {
                    return Some(row.role);
                }
                for cell in &row.cells {
                    if let Some(role) =
                        table_row_role_blocks_value(&cell.blocks, target)
                    {
                        return Some(role);
                    }
                }
            }
            None
        },
        BlockContent::Date(_)
        | BlockContent::Figure(_)
        | BlockContent::Heading(_)
        | BlockContent::Mathematics(_)
        | BlockContent::Paragraph(_)
        | BlockContent::Rule
        | BlockContent::Unresolved(_) => None,
    }
}

fn table_row_role_value(
    notebook: &Notebook<AcceptedIdentity>,
    target: AcceptedIdentity,
) -> Option<TableRowRole> {
    for page in &notebook.pages {
        for flow in &page.flows {
            if let Some(role) =
                table_row_role_blocks_value(&flow.blocks, target)
            {
                return Some(role);
            }
        }
    }
    None
}

fn replace_text_blocks(
    blocks: &mut [Block<AcceptedIdentity>],
    target: AcceptedIdentity,
    value: &mut Option<String>,
) -> bool {
    for block in blocks {
        if replace_text_content(&mut block.content, target, value) {
            return true;
        }
    }
    false
}

fn replace_text_content(
    content: &mut BlockContent<AcceptedIdentity>,
    target: AcceptedIdentity,
    value: &mut Option<String>,
) -> bool {
    match content {
        BlockContent::Callout(blocks) | BlockContent::Freeform(blocks) => {
            replace_text_blocks(blocks, target, value)
        },
        BlockContent::Date(spans)
        | BlockContent::Heading(spans)
        | BlockContent::Paragraph(spans) => {
            replace_text_spans(spans, target, value)
        },
        BlockContent::Figure(figure) => {
            replace_text_spans(&mut figure.caption, target, value)
        },
        BlockContent::List(list) => {
            for item in &mut list.items {
                if replace_text_blocks(&mut item.blocks, target, value) {
                    return true;
                }
            }
            false
        },
        BlockContent::Mathematics(_)
        | BlockContent::Rule
        | BlockContent::Unresolved(_) => false,
        BlockContent::Table(table) => {
            for row in &mut table.rows {
                for cell in &mut row.cells {
                    if replace_text_blocks(&mut cell.blocks, target, value) {
                        return true;
                    }
                }
            }
            false
        },
    }
}

fn replace_text_spans(
    spans: &mut [InlineSpan<AcceptedIdentity>],
    target: AcceptedIdentity,
    value: &mut Option<String>,
) -> bool {
    for span in spans {
        if span.id == target {
            let Some(replacement) = value.take() else {
                return false;
            };
            span.text = replacement;
            return true;
        }
    }
    false
}

fn replace_text_value(
    notebook: &mut Notebook<AcceptedIdentity>,
    target: AcceptedIdentity,
    value: String,
) -> bool {
    let mut replacement = Some(value);
    for page in &mut notebook.pages {
        for flow in &mut page.flows {
            if replace_text_blocks(&mut flow.blocks, target, &mut replacement) {
                return true;
            }
        }
    }
    false
}

fn text_blocks_value(
    blocks: &[Block<AcceptedIdentity>],
    target: AcceptedIdentity,
) -> Option<&str> {
    for block in blocks {
        if let Some(value) = text_content_value(&block.content, target) {
            return Some(value);
        }
    }
    None
}

fn text_content_value(
    content: &BlockContent<AcceptedIdentity>,
    target: AcceptedIdentity,
) -> Option<&str> {
    match content {
        BlockContent::Callout(blocks) | BlockContent::Freeform(blocks) => {
            text_blocks_value(blocks, target)
        },
        BlockContent::Date(spans)
        | BlockContent::Heading(spans)
        | BlockContent::Paragraph(spans) => text_spans_value(spans, target),
        BlockContent::Figure(figure) => {
            text_spans_value(&figure.caption, target)
        },
        BlockContent::List(list) => {
            for item in &list.items {
                if let Some(value) = text_blocks_value(&item.blocks, target) {
                    return Some(value);
                }
            }
            None
        },
        BlockContent::Mathematics(_)
        | BlockContent::Rule
        | BlockContent::Unresolved(_) => None,
        BlockContent::Table(table) => {
            for row in &table.rows {
                for cell in &row.cells {
                    if let Some(value) = text_blocks_value(&cell.blocks, target)
                    {
                        return Some(value);
                    }
                }
            }
            None
        },
    }
}

fn text_spans_value(
    spans: &[InlineSpan<AcceptedIdentity>],
    target: AcceptedIdentity,
) -> Option<&str> {
    spans
        .iter()
        .find(|span| span.id == target)
        .map(|span| span.text.as_str())
}

fn text_value(
    notebook: &Notebook<AcceptedIdentity>,
    target: AcceptedIdentity,
) -> Option<&str> {
    for page in &notebook.pages {
        for flow in &page.flows {
            if let Some(value) = text_blocks_value(&flow.blocks, target) {
                return Some(value);
            }
        }
    }
    None
}

fn discard_candidate_notebook(notebook: Notebook<CandidateIdentity>) {
    let Notebook { pages, .. } = notebook;
    let mut pending = Vec::new();
    for page in pages {
        for flow in page.flows {
            pending.extend(flow.blocks);
        }
    }
    while let Some(block) = pending.pop() {
        match block.content {
            BlockContent::Callout(children)
            | BlockContent::Freeform(children) => pending.extend(children),
            BlockContent::List(list) => {
                for item in list.items {
                    pending.extend(item.blocks);
                }
            },
            BlockContent::Table(table) => {
                for row in table.rows {
                    for cell in row.cells {
                        pending.extend(cell.blocks);
                    }
                }
            },
            BlockContent::Date(_)
            | BlockContent::Figure(_)
            | BlockContent::Heading(_)
            | BlockContent::Mathematics(_)
            | BlockContent::Paragraph(_)
            | BlockContent::Rule
            | BlockContent::Unresolved(_) => {},
        }
    }
}

fn candidate_blocks(
    blocks: &[Block<CandidateIdentity>],
    graph: &mut CandidateGraph,
) -> Result<(), CandidateGraphError> {
    let mut stack = vec![CandidateGraphFrame::Blocks { blocks, depth: 1 }];
    while let Some(frame) = stack.pop() {
        candidate_graph_frame(frame, graph, &mut stack)?;
    }
    Ok(())
}

fn candidate_graph_frame<'candidate>(
    frame: CandidateGraphFrame<'candidate>,
    graph: &mut CandidateGraph,
    stack: &mut Vec<CandidateGraphFrame<'candidate>>,
) -> Result<(), CandidateGraphError> {
    match frame {
        CandidateGraphFrame::Blocks { blocks, depth } => {
            candidate_graph_blocks_frame(blocks, depth, graph, stack)
        },
        CandidateGraphFrame::ListItems { child_depth, items } => {
            candidate_graph_list_items_frame(items, child_depth, graph, stack)
        },
        CandidateGraphFrame::TableCells { cells, child_depth } => {
            candidate_graph_table_cells_frame(cells, child_depth, graph, stack)
        },
        CandidateGraphFrame::TableRows { child_depth, rows } => {
            candidate_graph_table_rows_frame(rows, child_depth, graph, stack)
        },
    }
}

fn candidate_graph_blocks_frame<'candidate>(
    current: &'candidate [Block<CandidateIdentity>],
    depth: usize,
    graph: &mut CandidateGraph,
    stack: &mut Vec<CandidateGraphFrame<'candidate>>,
) -> Result<(), CandidateGraphError> {
    let Some((block, remaining)) = current.split_first() else {
        return Ok(());
    };
    if depth > CANDIDATE_BLOCK_NESTING_LIMIT {
        return Err(CandidateGraphError::NestingLimitExceeded {
            candidate: block.id,
            limit: CANDIDATE_BLOCK_NESTING_LIMIT,
        });
    }
    if !remaining.is_empty() {
        stack.push(CandidateGraphFrame::Blocks { blocks: remaining, depth });
    }
    graph.register(block.id, CandidateReferenceKind::Semantic)?;
    graph.reference(block.provenance, CandidateReferenceKind::Provenance);
    graph.reference(block.style, CandidateReferenceKind::Style);
    let child_depth = depth.saturating_add(1);
    match &block.content {
        BlockContent::Callout(children) | BlockContent::Freeform(children) => {
            if !children.is_empty() {
                stack.push(CandidateGraphFrame::Blocks {
                    blocks: children,
                    depth: child_depth,
                });
            }
        },
        BlockContent::Date(spans)
        | BlockContent::Heading(spans)
        | BlockContent::Paragraph(spans) => {
            candidate_spans(spans, graph)?;
        },
        BlockContent::Figure(figure) => {
            candidate_figure(figure, graph)?;
        },
        BlockContent::List(list) => {
            graph.register(list.id, CandidateReferenceKind::Semantic)?;
            if !list.items.is_empty() {
                stack.push(CandidateGraphFrame::ListItems {
                    child_depth,
                    items: &list.items,
                });
            }
        },
        BlockContent::Mathematics(formula) => {
            let analyzed =
                analyze(&formula.source, formula.mode).map_err(|reason| {
                    CandidateGraphError::InvalidMathematics {
                        candidate: formula.id,
                        reason,
                    }
                })?;
            if !analyzed.is_supported() {
                return Err(CandidateGraphError::UnsupportedMathematics {
                    candidate: formula.id,
                });
            }
            graph.register(formula.id, CandidateReferenceKind::Semantic)?;
        },
        BlockContent::Rule | BlockContent::Unresolved(_) => {},
        BlockContent::Table(table) => {
            graph.register(table.id, CandidateReferenceKind::Semantic)?;
            validate_candidate_table(table)?;
            if !table.rows.is_empty() {
                stack.push(CandidateGraphFrame::TableRows {
                    child_depth,
                    rows: &table.rows,
                });
            }
        },
    }
    Ok(())
}

fn candidate_graph_list_items_frame<'candidate>(
    current: &'candidate [ListItem<CandidateIdentity>],
    child_depth: usize,
    graph: &mut CandidateGraph,
    stack: &mut Vec<CandidateGraphFrame<'candidate>>,
) -> Result<(), CandidateGraphError> {
    let Some((item, remaining)) = current.split_first() else {
        return Ok(());
    };
    if !remaining.is_empty() {
        stack.push(CandidateGraphFrame::ListItems {
            child_depth,
            items: remaining,
        });
    }
    graph.register(item.id, CandidateReferenceKind::Semantic)?;
    if !item.blocks.is_empty() {
        stack.push(CandidateGraphFrame::Blocks {
            blocks: &item.blocks,
            depth: child_depth,
        });
    }
    Ok(())
}

fn validate_candidate_table(
    table: &Table<CandidateIdentity>,
) -> Result<(), CandidateGraphError> {
    table.validate_grid().map_err(|reason| match reason {
        TableGridError::ColumnSpan { cell } => {
            CandidateGraphError::InvalidTableColumnSpan { candidate: cell }
        },
        TableGridError::RowSpan { cell } => {
            CandidateGraphError::InvalidTableRowSpan { candidate: cell }
        },
        TableGridError::RowWidth { row } => {
            CandidateGraphError::InvalidTableRowWidth { candidate: row }
        },
    })
}

fn candidate_graph_table_cells_frame<'candidate>(
    current: &'candidate [TableCell<CandidateIdentity>],
    child_depth: usize,
    graph: &mut CandidateGraph,
    stack: &mut Vec<CandidateGraphFrame<'candidate>>,
) -> Result<(), CandidateGraphError> {
    let Some((cell, remaining)) = current.split_first() else {
        return Ok(());
    };
    if !remaining.is_empty() {
        stack.push(CandidateGraphFrame::TableCells {
            cells: remaining,
            child_depth,
        });
    }
    graph.register(cell.id, CandidateReferenceKind::Semantic)?;
    if !cell.blocks.is_empty() {
        stack.push(CandidateGraphFrame::Blocks {
            blocks: &cell.blocks,
            depth: child_depth,
        });
    }
    Ok(())
}

fn candidate_graph_table_rows_frame<'candidate>(
    current: &'candidate [TableRow<CandidateIdentity>],
    child_depth: usize,
    graph: &mut CandidateGraph,
    stack: &mut Vec<CandidateGraphFrame<'candidate>>,
) -> Result<(), CandidateGraphError> {
    let Some((row, remaining)) = current.split_first() else {
        return Ok(());
    };
    if !remaining.is_empty() {
        stack.push(CandidateGraphFrame::TableRows {
            child_depth,
            rows: remaining,
        });
    }
    graph.register(row.id, CandidateReferenceKind::Semantic)?;
    if !row.cells.is_empty() {
        stack.push(CandidateGraphFrame::TableCells {
            cells: &row.cells,
            child_depth,
        });
    }
    Ok(())
}

fn candidate_figure(
    figure: &Figure<CandidateIdentity>,
    graph: &mut CandidateGraph,
) -> Result<(), CandidateGraphError> {
    graph.register(figure.id, CandidateReferenceKind::Semantic)?;
    graph.reference(figure.asset, CandidateReferenceKind::Asset);
    candidate_spans(&figure.caption, graph)
}

fn candidate_flow(
    flow: &Flow<CandidateIdentity>,
    graph: &mut CandidateGraph,
) -> Result<(), CandidateGraphError> {
    graph.register(flow.id, CandidateReferenceKind::Semantic)?;
    candidate_blocks(&flow.blocks, graph)
}

fn candidate_identities(
    notebook: &Notebook<CandidateIdentity>,
) -> Result<Vec<CandidateIdentity>, CandidateGraphError> {
    let mut graph = CandidateGraph::default();
    graph.register(notebook.id, CandidateReferenceKind::Semantic)?;
    for asset in &notebook.assets {
        graph.register(asset.id, CandidateReferenceKind::Asset)?;
    }
    for constraint in &notebook.constraints {
        graph.register(constraint.id, CandidateReferenceKind::Semantic)?;
        graph.reference(
            Some(constraint.target),
            CandidateReferenceKind::Semantic,
        );
    }
    for profile in &notebook.output_profiles {
        graph.register(profile.id, CandidateReferenceKind::Semantic)?;
    }
    for profile in &notebook.page_profiles {
        graph.register(profile.id, CandidateReferenceKind::PageProfile)?;
        if let Err(reason) = profile.geometry.validate() {
            return Err(CandidateGraphError::InvalidPageProfile {
                candidate: profile.id,
                reason,
            });
        }
    }
    for page in &notebook.pages {
        candidate_page(page, &mut graph)?;
    }
    for provenance in &notebook.provenance {
        graph.register(provenance.id, CandidateReferenceKind::Provenance)?;
    }
    for style in &notebook.styles {
        graph.register(style.id, CandidateReferenceKind::Style)?;
    }
    graph.finish()
}

fn candidate_page(
    page: &Page<CandidateIdentity>,
    graph: &mut CandidateGraph,
) -> Result<(), CandidateGraphError> {
    graph.register(page.id, CandidateReferenceKind::Semantic)?;
    graph.reference(
        Some(page.page_profile),
        CandidateReferenceKind::PageProfile,
    );
    for flow in &page.flows {
        candidate_flow(flow, graph)?;
    }
    Ok(())
}

fn candidate_spans(
    spans: &[InlineSpan<CandidateIdentity>],
    graph: &mut CandidateGraph,
) -> Result<(), CandidateGraphError> {
    for span in spans {
        graph.register(span.id, CandidateReferenceKind::Semantic)?;
        graph.reference(span.provenance, CandidateReferenceKind::Provenance);
        graph.reference(span.style, CandidateReferenceKind::Style);
    }
    Ok(())
}
