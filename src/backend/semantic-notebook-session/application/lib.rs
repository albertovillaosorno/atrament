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
    CandidateIdentity, Constraint, Figure, Flow, Formula, FormulaMode,
    IdentityAllocator, IdentityExhausted, InlineSpan, List, ListItem, Notebook,
    OutputProfile, Page, PaperProfile, Provenance, SemanticIdentityDescriptor,
    SemanticIdentityKind, Style, Table, TableCell, TableCellSpan,
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
    CommandTargetPreconditions, DirectEditBatchCommand,
    DirectEditBatchCommandPrediction, DirectEditBatchCommandRejection,
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
    HistoryTraversalOutcome, IdentityInspectOutcome,
    IdentityKindInspectOutcome, IdentityMapping,
    IdentityOwnerExpectation, IdentityPrecondition,
    IdentityPreconditionOutcome, PageProfileEditOutcome,
    SemanticCommandCapabilitySnapshot, SemanticCommandFamily,
    SemanticNotebookHistory, SemanticNotebookSession,
    TableCellSpanEditOutcome, TableRowRoleEditOutcome, TextEditOutcome,
};

#[derive(Default)]
struct DirectEditBatchIndex {
    impacts: BTreeMap<AcceptedIdentity, DirectEditImpactScope>,
    materials: BTreeMap<AcceptedIdentity, CommandTargetMaterial>,
}

#[derive(Clone, Copy)]
struct DirectEditBatchTableContext {
    block: AcceptedIdentity,
    flow: AcceptedIdentity,
    page: AcceptedIdentity,
    table: AcceptedIdentity,
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
        flow: AcceptedIdentity,
        page: AcceptedIdentity,
    },
    TableRows {
        block: AcceptedIdentity,
        flow: AcceptedIdentity,
        page: AcceptedIdentity,
        rows: &'notebook [TableRow<AcceptedIdentity>],
        table: AcceptedIdentity,
    },
}

#[derive(Clone, Copy)]
struct DirectEditBatchMaterialMetadata {
    descriptor: SemanticIdentityDescriptor<AcceptedIdentity>,
    direct_edit_family: Option<SemanticCommandFamily>,
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
        if let Some(current) = self.current.as_ref() {
            self.undo_notebooks.push(current.notebook.clone());
        }
        self.redo_notebooks.clear();
        self.current = Some(AcceptedRevision { id: revision, notebook });
        AcceptanceOutcome::Accepted { mapping, revision }
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
        let material = match self.command_target_material(revision, target) {
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
        let material = match self.command_target_material(revision, target) {
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
        let material = match self.command_target_material(revision, target) {
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
        const VERSION: CommandBehaviorVersion = CommandBehaviorVersion(1);
        const FAMILY_CAPABILITIES: [CommandFamilyCapability; 3] = [
            CommandFamilyCapability {
                behavior_version: VERSION,
                family: SemanticCommandFamily::DocumentConstraint,
            },
            CommandFamilyCapability {
                behavior_version: VERSION,
                family: SemanticCommandFamily::StructuredContent,
            },
            CommandFamilyCapability {
                behavior_version: VERSION,
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
        let simulation = simulate_direct_edit_material(
            self.command_target_material(revision, target),
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
            | DirectEditSimulationOutcome::InvalidPageProfile { .. }
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
            | DirectEditSimulationOutcome::InvalidMathematics { .. }
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
        let Some(current) = self.current.as_ref() else {
            return TableCellSpanEditOutcome::NoAcceptedRevision;
        };
        if current.id != base {
            return TableCellSpanEditOutcome::StaleBase {
                current: current.id,
            };
        }
        let Some(descriptor) = semantic_identity_descriptor(
            &current.notebook,
            target,
        ) else {
            return TableCellSpanEditOutcome::TargetNotFound {
                revision: current.id,
                target,
            };
        };
        if descriptor.kind != SemanticIdentityKind::TableCell {
            return TableCellSpanEditOutcome::TargetNotTableCell {
                revision: current.id,
                target,
            };
        }
        let Some(actual) =
            table_cell_span_value(&current.notebook, target)
        else {
            return TableCellSpanEditOutcome::TargetNotFound {
                revision: current.id,
                target,
            };
        };
        if actual == span {
            return TableCellSpanEditOutcome::NoOp {
                revision: current.id,
                target,
            };
        }
        let mut notebook = current.notebook.clone();
        match replace_table_cell_span_value(&mut notebook, target, span) {
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
            | DirectEditSimulationOutcome::InvalidMathematics { .. }
            | DirectEditSimulationOutcome::InvalidPageProfile { .. }
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
            | DirectEditSimulationOutcome::InvalidMathematics { .. }
            | DirectEditSimulationOutcome::InvalidPageProfile { .. }
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
        simulate_direct_edit_material(
            self.command_target_material(revision, target),
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
        let current_notebook = current.notebook.clone();
        let restored = match direction {
            HistoryDirection::Redo => {
                let Some(restored) = self.redo_notebooks.pop() else {
                    return HistoryTraversalOutcome::Boundary {
                        direction,
                        revision: current.id,
                    };
                };
                self.undo_notebooks.push(current_notebook);
                restored
            },
            HistoryDirection::Undo => {
                let Some(restored) = self.undo_notebooks.pop() else {
                    return HistoryTraversalOutcome::Boundary {
                        direction,
                        revision: current.id,
                    };
                };
                self.redo_notebooks.push(current_notebook);
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

    fn commit_semantic_edit(
        &mut self,
        notebook: Notebook<AcceptedIdentity>,
    ) -> Result<
        atrament_semantic_notebook::RevisionIdentity,
        IdentityExhausted,
    > {
        let revision = self.identities.allocate_revision()?;
        if let Some(current) = self.current.as_ref() {
            self.undo_notebooks.push(current.notebook.clone());
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
    targets: &BTreeSet<AcceptedIdentity>,
    revision: atrament_semantic_notebook::RevisionIdentity,
) -> DirectEditBatchIndex {
    let mut index = DirectEditBatchIndex::default();
    let targeted_profiles = notebook
        .page_profiles
        .iter()
        .filter(|profile| targets.contains(&profile.id))
        .count();
    if targeted_profiles > 0 {
        let mut profile_pages =
            BTreeMap::<AcceptedIdentity, Vec<AcceptedIdentity>>::new();
        for page in &notebook.pages {
            if targets.contains(&page.page_profile) {
                profile_pages
                    .entry(page.page_profile)
                    .or_default()
                    .push(page.id);
            }
        }
        for profile in &notebook.page_profiles {
            if !targets.contains(&profile.id) {
                continue;
            }
            let pages = profile_pages.remove(&profile.id).unwrap_or_default();
            let impact = if pages.is_empty() {
                DirectEditImpactScope::Notebook { notebook: notebook.id }
            } else {
                DirectEditImpactScope::Pages { pages }
            };
            insert_direct_edit_material(
                &mut index,
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
        if targeted_profiles == targets.len() {
            return index;
        }
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
                    frame, &mut index, targets, revision, &mut stack,
                );
                if index.materials.len() == targets.len() {
                    break 'pages;
                }
            }
        }
    }
    index
}

fn index_direct_edit_frame<'notebook>(
    frame: DirectEditBatchIndexFrame<'notebook>,
    index: &mut DirectEditBatchIndex,
    targets: &BTreeSet<AcceptedIdentity>,
    revision: atrament_semantic_notebook::RevisionIdentity,
    stack: &mut Vec<DirectEditBatchIndexFrame<'notebook>>,
) {
    match frame {
        DirectEditBatchIndexFrame::Blocks { blocks, flow, page } => {
            index_direct_edit_blocks_frame(
                blocks, flow, page, index, targets, revision, stack,
            );
        },
        DirectEditBatchIndexFrame::ListItems { flow, items, page } => {
            index_direct_edit_list_items_frame(items, flow, page, stack);
        },
        DirectEditBatchIndexFrame::TableCells { cells, flow, page } => {
            index_direct_edit_table_cells_frame(cells, flow, page, stack);
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
                targets,
                revision,
                stack,
            );
        },
    }
}

fn index_direct_edit_blocks_frame<'notebook>(
    current: &'notebook [Block<AcceptedIdentity>],
    flow: AcceptedIdentity,
    page: AcceptedIdentity,
    index: &mut DirectEditBatchIndex,
    targets: &BTreeSet<AcceptedIdentity>,
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
                index, spans, targets, block.id, flow, page, revision,
            );
        },
        BlockContent::Figure(figure) => {
            index_direct_edit_spans(
                index,
                &figure.caption,
                targets,
                figure.id,
                flow,
                page,
                revision,
            );
        },
        BlockContent::List(list) => {
            if !list.items.is_empty() {
                stack.push(DirectEditBatchIndexFrame::ListItems {
                    flow,
                    items: &list.items,
                    page,
                });
            }
        },
        BlockContent::Mathematics(formula) => {
            if targets.contains(&formula.id) {
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
                    table: table.id,
                });
            }
        },
        BlockContent::Rule | BlockContent::Unresolved(_) => {},
    }
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
    flow: AcceptedIdentity,
    page: AcceptedIdentity,
    stack: &mut Vec<DirectEditBatchIndexFrame<'notebook>>,
) {
    let Some((cell, remaining)) = current.split_first() else {
        return;
    };
    if !remaining.is_empty() {
        stack.push(DirectEditBatchIndexFrame::TableCells {
            cells: remaining,
            flow,
            page,
        });
    }
    if !cell.blocks.is_empty() {
        stack.push(DirectEditBatchIndexFrame::Blocks {
            blocks: &cell.blocks,
            flow,
            page,
        });
    }
}

fn index_direct_edit_table_rows_frame<'notebook>(
    current: &'notebook [TableRow<AcceptedIdentity>],
    context: DirectEditBatchTableContext,
    index: &mut DirectEditBatchIndex,
    targets: &BTreeSet<AcceptedIdentity>,
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
    if targets.contains(&row.id) {
        insert_direct_edit_material(
            index,
            row.id,
            SemanticIdentityDescriptor {
                kind: SemanticIdentityKind::TableRow,
                owner: Some(table),
            },
            EditableSemanticValue::TableRowRole(row.role),
            DirectEditImpactScope::BlockFlow { block, flow, page },
            revision,
        );
        if index.materials.len() == targets.len() {
            return;
        }
    }
    if !row.cells.is_empty() {
        stack.push(DirectEditBatchIndexFrame::TableCells {
            cells: &row.cells,
            flow,
            page,
        });
    }
}

fn index_direct_edit_spans(
    index: &mut DirectEditBatchIndex,
    spans: &[InlineSpan<AcceptedIdentity>],
    targets: &BTreeSet<AcceptedIdentity>,
    owner: AcceptedIdentity,
    flow: AcceptedIdentity,
    page: AcceptedIdentity,
    revision: atrament_semantic_notebook::RevisionIdentity,
) {
    for span in spans {
        if !targets.contains(&span.id) {
            continue;
        }
        insert_direct_edit_material(
            index,
            span.id,
            SemanticIdentityDescriptor {
                kind: SemanticIdentityKind::InlineSpan,
                owner: Some(owner),
            },
            EditableSemanticValue::Text(span.text.clone()),
            DirectEditImpactScope::Flow { flow, page },
            revision,
        );
        if index.materials.len() == targets.len() {
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
    let direct_edit_family = Some(direct_edit_family(&editable_value));
    let _previous_impact = index.impacts.insert(target, impact);
    let _previous_material =
        index.materials.insert(target, CommandTargetMaterial {
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
    let targets = commands
        .iter()
        .map(|command| command.target)
        .collect::<BTreeSet<_>>();
    let mut batch_index =
        direct_edit_material_index(&current.notebook, &targets, revision);
    let mut evaluated =
        Vec::<DirectEditBatchCommandPrediction<CommandIdentity>>::with_capacity(
            commands.len(),
        );
    let mut changed_targets =
        BTreeMap::<AcceptedIdentity, (usize, usize)>::new();
    let mut remaining = commands.into_iter();
    while let Some(command) = remaining.next() {
        let target = command.target;
        let previous = changed_targets
            .get(&target)
            .and_then(|(_, last)| evaluated.get(*last))
            .map(|prediction| &prediction.command);
        let result = simulate_direct_edit_batch_command(
            &current.notebook,
            &mut batch_index.materials,
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
                target,
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
    mut impacts: BTreeMap<AcceptedIdentity, DirectEditImpactScope>,
    changes: &[DirectEditSemanticChange],
) -> Vec<DirectEditImpactSeed> {
    collect_direct_edit_impact_seeds(changes, |change| {
        let scope = impacts
            .remove(&change.target)
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
        SemanticCommandFamily::StructuredContent => {
            direct_edit_structured_scope(notebook, change.target)
        },
        SemanticCommandFamily::TextContent => {
            direct_edit_text_scope(notebook, change.target)
        },
        SemanticCommandFamily::AssetReference
        | SemanticCommandFamily::BlockInsertionAndDeletion
        | SemanticCommandFamily::OrderingAndGrouping
        | SemanticCommandFamily::Provenance
        | SemanticCommandFamily::SpatialConstraint
        | SemanticCommandFamily::StyleRole => {
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
        | SemanticCommandFamily::Provenance
        | SemanticCommandFamily::SpatialConstraint
        | SemanticCommandFamily::StyleRole => {
            &[DirectEditDerivedAuthority::AllDerived]
        },
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
    materials: &mut BTreeMap<AcceptedIdentity, CommandTargetMaterial>,
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
        notebook, materials, revision, target,
    ) {
        Ok(material) => material,
        Err(reason) => return Err((id, reason)),
    };
    let metadata = DirectEditBatchMaterialMetadata {
        descriptor: prepared.descriptor,
        direct_edit_family: prepared.direct_edit_family,
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
    batch_command_prediction(materials, id, metadata, simulation)
}

fn batch_command_target_material<CommandIdentity>(
    notebook: &Notebook<AcceptedIdentity>,
    materials: &mut BTreeMap<AcceptedIdentity, CommandTargetMaterial>,
    revision: atrament_semantic_notebook::RevisionIdentity,
    target: AcceptedIdentity,
) -> Result<
    (CommandTargetMaterial, bool),
    DirectEditBatchCommandRejection<CommandIdentity>,
> {
    if let Some(material) = materials.remove(&target) {
        return Ok((material, true));
    }
    match command_target_material_from_notebook(notebook, revision, target) {
        CommandTargetMaterialOutcome::Prepared { material } => {
            Ok((material, false))
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
    materials: &mut BTreeMap<AcceptedIdentity, CommandTargetMaterial>,
    command: CommandIdentity,
    metadata: DirectEditBatchMaterialMetadata,
    simulation: DirectEditSimulation,
) -> Result<
    DirectEditBatchCommandPrediction<CommandIdentity>,
    (
        CommandIdentity,
        DirectEditBatchCommandRejection<CommandIdentity>,
    ),
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
        outcome @ (DirectEditSimulationOutcome::InvalidMathematics {
            ..
        }
        | DirectEditSimulationOutcome::InvalidPageProfile {
            ..
        }
        | DirectEditSimulationOutcome::NoAcceptedRevision
        | DirectEditSimulationOutcome::StaleBase { .. }
        | DirectEditSimulationOutcome::TargetNotEditableValue {
            ..
        }
        | DirectEditSimulationOutcome::TargetNotFound { .. }
        | DirectEditSimulationOutcome::UnsupportedMathematics {
            ..
        }
        | DirectEditSimulationOutcome::ValueFamilyMismatch {
            ..
        }) => Err((command, DirectEditBatchCommandRejection::Simulation {
            outcome: Box::new(outcome),
        })),
    }
}

fn restore_direct_edit_batch_material(
    materials: &mut BTreeMap<AcceptedIdentity, CommandTargetMaterial>,
    metadata: DirectEditBatchMaterialMetadata,
    editable_value: EditableSemanticValue,
    revision: atrament_semantic_notebook::RevisionIdentity,
    target: AcceptedIdentity,
) {
    let _previous = materials.insert(target, CommandTargetMaterial {
        descriptor: metadata.descriptor,
        direct_edit_family: metadata.direct_edit_family,
        editable_value: Some(editable_value),
        revision,
        target,
    });
}

fn record_direct_edit_batch_change_index(
    aggregate: &mut BTreeMap<AcceptedIdentity, (usize, usize)>,
    index: usize,
    target: AcceptedIdentity,
) {
    if let Some((_, last)) = aggregate.get_mut(&target) {
        *last = index;
    } else {
        let _previous = aggregate.insert(target, (index, index));
    }
}

fn collect_direct_edit_batch_changes<CommandIdentity>(
    evaluated: &[DirectEditBatchCommandPrediction<CommandIdentity>],
    aggregate: BTreeMap<AcceptedIdentity, (usize, usize)>,
) -> Vec<DirectEditSemanticChange> {
    let mut ordered = aggregate.into_iter().collect::<Vec<_>>();
    ordered.sort_by_key(|(_, (first, _))| *first);
    ordered
        .into_iter()
        .filter_map(|(target, (first, last))| {
            let first_change = evaluated.get(first)?.change.as_ref()?;
            let last_change = evaluated.get(last)?.change.as_ref()?;
            if first_change.before == last_change.after {
                return None;
            }
            Some(DirectEditSemanticChange {
                after: last_change.after.clone(),
                before: first_change.before.clone(),
                family: first_change.family,
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
        EditableSemanticValue::Formula { .. } => {
            EditableSemanticValueKind::Formula
        },
        EditableSemanticValue::PageProfile(_) => {
            EditableSemanticValueKind::PageProfile
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
        EditableSemanticValue::TableRowRole(_)
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
        EditableSemanticValue::Formula { .. }
        | EditableSemanticValue::TableRowRole(_) => {
            SemanticCommandFamily::StructuredContent
        },
        EditableSemanticValue::PageProfile(_) => {
            SemanticCommandFamily::DocumentConstraint
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
        SemanticIdentityKind::PageProfile => {
            page_profile_value(notebook, target)
                .map(EditableSemanticValue::PageProfile)
        },
        SemanticIdentityKind::TableRow => {
            table_row_role_value(notebook, target)
                .map(EditableSemanticValue::TableRowRole)
        },
        SemanticIdentityKind::Asset
        | SemanticIdentityKind::Block(_)
        | SemanticIdentityKind::Constraint
        | SemanticIdentityKind::Figure
        | SemanticIdentityKind::Flow
        | SemanticIdentityKind::List
        | SemanticIdentityKind::ListItem
        | SemanticIdentityKind::Notebook
        | SemanticIdentityKind::OutputProfile
        | SemanticIdentityKind::Page
        | SemanticIdentityKind::Provenance
        | SemanticIdentityKind::Style
        | SemanticIdentityKind::Table
        | SemanticIdentityKind::TableCell => None,
    }
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
