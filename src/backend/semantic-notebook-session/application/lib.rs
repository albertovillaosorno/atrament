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
//   - Active-session accepted semantic revision and candidate promotion.
// - Must-Not:
//   - Persist notebooks, parse transport text, perform layout, or reuse IDs.
// - Allows:
//   - Inputs: Complete candidate semantic notebook values from prior
//     validation.
//   - Outputs: Current accepted revision and candidate-to-accepted ID mapping.
//   - Side effects: Atomic process-memory accepted revision replacement only.
// - Split-When:
//   - Semantic command Apply or history requires independent transaction state.
// - Merge-When:
//   - One application authority subsumes all accepted semantic transactions.
// - Summary:
//   - Promotes one candidate notebook into accepted in-memory session
//     authority.
// - Description:
//   - Validates candidate identity graph before allocating accepted authority.
// - Usage:
//   - Own one service instance for one active disposable Atrament session.
// - Defaults:
//   - Starts without an accepted revision and never persists session state.
//

//! Atomic in-memory acceptance of complete semantic notebook candidates.

use std::collections::BTreeMap;
use std::fmt;

use atrament_mathematics_source::analyze;
use atrament_semantic_notebook::{
    AcceptedIdentity, AcceptedRevision, Asset, Block, BlockContent,
    CandidateIdentity, Constraint, Figure, Flow, Formula, FormulaMode,
    IdentityAllocator, IdentityExhausted, InlineSpan, List, ListItem, Notebook,
    OutputProfile, Page, PaperProfile, Provenance, Style, Table, TableCell,
    TableRow, TableRowRole, semantic_identity_descriptor,
    semantic_identity_kind,
};
use atrament_semantic_notebook_port::{
    AcceptanceOutcome, CANDIDATE_BLOCK_NESTING_LIMIT, CandidateGraphError,
    CandidateReferenceKind, EditableSemanticValue,
    EditableValuePreconditionOutcome, FormulaEditOutcome,
    IdentityInspectOutcome, IdentityKindInspectOutcome, IdentityMapping,
    IdentityOwnerExpectation, IdentityPrecondition,
    IdentityPreconditionOutcome, PageProfileEditOutcome,
    SemanticNotebookSession, TableRowRoleEditOutcome, TextEditOutcome,
};

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
        self.current = Some(AcceptedRevision { id: revision, notebook });
        AcceptanceOutcome::Accepted { mapping, revision }
    }

    fn check_editable_value_precondition(
        &self,
        revision: atrament_semantic_notebook::RevisionIdentity,
        target: AcceptedIdentity,
        expected: EditableSemanticValue,
    ) -> EditableValuePreconditionOutcome {
        let Some(current) = self.current.as_ref() else {
            return EditableValuePreconditionOutcome::NoAcceptedRevision;
        };
        if current.id != revision {
            return EditableValuePreconditionOutcome::StaleBase {
                current: current.id,
            };
        }
        let Some(descriptor) =
            semantic_identity_descriptor(&current.notebook, target)
        else {
            return EditableValuePreconditionOutcome::TargetNotFound {
                revision,
                target,
            };
        };
        let Some(actual) =
            editable_semantic_value(&current.notebook, target, descriptor.kind)
        else {
            return EditableValuePreconditionOutcome::TargetNotEditableValue {
                kind: descriptor.kind,
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

    fn current(&self) -> Option<&AcceptedRevision> {
        self.current.as_ref()
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

    fn replace_formula(
        &mut self,
        base: atrament_semantic_notebook::RevisionIdentity,
        target: AcceptedIdentity,
        mode: FormulaMode,
        source: String,
    ) -> FormulaEditOutcome {
        let Some(current) = self.current.as_ref() else {
            return FormulaEditOutcome::NoAcceptedRevision;
        };
        if current.id != base {
            return FormulaEditOutcome::StaleBase { current: current.id };
        }
        let Some(existing) = formula_value(&current.notebook, target) else {
            if semantic_identity_kind(&current.notebook, target).is_some() {
                return FormulaEditOutcome::TargetNotFormula {
                    revision: current.id,
                    target,
                };
            }
            return FormulaEditOutcome::TargetNotFound {
                revision: current.id,
                target,
            };
        };
        let analyzed = match analyze(&source, mode) {
            Ok(analyzed) => analyzed,
            Err(reason) => {
                return FormulaEditOutcome::InvalidMathematics {
                    reason,
                    revision: current.id,
                    target,
                };
            },
        };
        if !analyzed.is_supported() {
            return FormulaEditOutcome::UnsupportedMathematics {
                revision: current.id,
                target,
            };
        }
        if existing.mode == mode && existing.source == source {
            return FormulaEditOutcome::NoOp {
                revision: current.id,
                target,
            };
        }
        let mut notebook = current.notebook.clone();
        if !replace_formula_value(&mut notebook, target, mode, source) {
            return FormulaEditOutcome::TargetNotFound {
                revision: current.id,
                target,
            };
        }
        let revision = match self.identities.allocate_revision() {
            Ok(revision) => revision,
            Err(sequence) => {
                return FormulaEditOutcome::IdentityExhausted { sequence };
            },
        };
        self.current = Some(AcceptedRevision { id: revision, notebook });
        FormulaEditOutcome::Applied { base, revision, target }
    }

    fn replace_page_profile(
        &mut self,
        base: atrament_semantic_notebook::RevisionIdentity,
        target: AcceptedIdentity,
        geometry: atrament_semantic_notebook::PhysicalPageProfile,
    ) -> PageProfileEditOutcome {
        let Some(current) = self.current.as_ref() else {
            return PageProfileEditOutcome::NoAcceptedRevision;
        };
        if current.id != base {
            return PageProfileEditOutcome::StaleBase { current: current.id };
        }
        let Some(existing) = page_profile_value(&current.notebook, target)
        else {
            if semantic_identity_kind(&current.notebook, target).is_some() {
                return PageProfileEditOutcome::TargetNotPageProfile {
                    revision: current.id,
                    target,
                };
            }
            return PageProfileEditOutcome::TargetNotFound {
                revision: current.id,
                target,
            };
        };
        if let Err(reason) = geometry.validate() {
            return PageProfileEditOutcome::InvalidProfile {
                reason,
                revision: current.id,
                target,
            };
        }
        if existing == geometry {
            return PageProfileEditOutcome::NoOp {
                revision: current.id,
                target,
            };
        }
        let mut notebook = current.notebook.clone();
        let changed =
            replace_page_profile_value(&mut notebook, target, geometry);
        if !changed {
            return PageProfileEditOutcome::TargetNotFound {
                revision: current.id,
                target,
            };
        }
        let revision = match self.identities.allocate_revision() {
            Ok(revision) => revision,
            Err(sequence) => {
                return PageProfileEditOutcome::IdentityExhausted { sequence };
            },
        };
        self.current = Some(AcceptedRevision { id: revision, notebook });
        PageProfileEditOutcome::Applied { base, revision, target }
    }

    fn replace_table_row_role(
        &mut self,
        base: atrament_semantic_notebook::RevisionIdentity,
        target: AcceptedIdentity,
        role: TableRowRole,
    ) -> TableRowRoleEditOutcome {
        let Some(current) = self.current.as_ref() else {
            return TableRowRoleEditOutcome::NoAcceptedRevision;
        };
        if current.id != base {
            return TableRowRoleEditOutcome::StaleBase { current: current.id };
        }
        let Some(existing) = table_row_role_value(&current.notebook, target)
        else {
            if semantic_identity_kind(&current.notebook, target).is_some() {
                return TableRowRoleEditOutcome::TargetNotTableRow {
                    revision: current.id,
                    target,
                };
            }
            return TableRowRoleEditOutcome::TargetNotFound {
                revision: current.id,
                target,
            };
        };
        if existing == role {
            return TableRowRoleEditOutcome::NoOp {
                revision: current.id,
                target,
            };
        }
        let mut notebook = current.notebook.clone();
        if !replace_table_row_role_value(&mut notebook, target, role) {
            return TableRowRoleEditOutcome::TargetNotFound {
                revision: current.id,
                target,
            };
        }
        let revision = match self.identities.allocate_revision() {
            Ok(revision) => revision,
            Err(sequence) => {
                return TableRowRoleEditOutcome::IdentityExhausted { sequence };
            },
        };
        self.current = Some(AcceptedRevision { id: revision, notebook });
        TableRowRoleEditOutcome::Applied { base, revision, target }
    }

    fn replace_text(
        &mut self,
        base: atrament_semantic_notebook::RevisionIdentity,
        target: AcceptedIdentity,
        value: String,
    ) -> TextEditOutcome {
        let Some(current) = self.current.as_ref() else {
            return TextEditOutcome::NoAcceptedRevision;
        };
        if current.id != base {
            return TextEditOutcome::StaleBase { current: current.id };
        }
        let Some(existing) = text_value(&current.notebook, target) else {
            if semantic_identity_kind(&current.notebook, target).is_some() {
                return TextEditOutcome::TargetNotText {
                    revision: current.id,
                    target,
                };
            }
            return TextEditOutcome::TargetNotFound {
                revision: current.id,
                target,
            };
        };
        if existing == value {
            return TextEditOutcome::NoOp {
                revision: current.id,
                target,
            };
        }
        let mut notebook = current.notebook.clone();
        let edited = replace_text_value(&mut notebook, target, value);
        if !edited {
            return TextEditOutcome::TargetNotFound {
                revision: current.id,
                target,
            };
        }
        let revision = match self.identities.allocate_revision() {
            Ok(revision) => revision,
            Err(sequence) => {
                return TextEditOutcome::IdentityExhausted { sequence };
            },
        };
        self.current = Some(AcceptedRevision { id: revision, notebook });
        TextEditOutcome::Applied { base, revision, target }
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

fn editable_semantic_value(
    notebook: &Notebook<AcceptedIdentity>,
    target: AcceptedIdentity,
    kind: atrament_semantic_notebook::SemanticIdentityKind,
) -> Option<EditableSemanticValue> {
    match kind {
        atrament_semantic_notebook::SemanticIdentityKind::Formula => {
            formula_value(notebook, target).map(|formula| {
                EditableSemanticValue::Formula {
                    mode: formula.mode,
                    source: formula.source.clone(),
                }
            })
        },
        atrament_semantic_notebook::SemanticIdentityKind::InlineSpan => {
            text_value(notebook, target)
                .map(|value| EditableSemanticValue::Text(value.to_owned()))
        },
        atrament_semantic_notebook::SemanticIdentityKind::PageProfile => {
            page_profile_value(notebook, target)
                .map(EditableSemanticValue::PageProfile)
        },
        atrament_semantic_notebook::SemanticIdentityKind::TableRow => {
            table_row_role_value(notebook, target)
                .map(EditableSemanticValue::TableRowRole)
        },
        atrament_semantic_notebook::SemanticIdentityKind::Asset
        | atrament_semantic_notebook::SemanticIdentityKind::Block(_)
        | atrament_semantic_notebook::SemanticIdentityKind::Constraint
        | atrament_semantic_notebook::SemanticIdentityKind::Figure
        | atrament_semantic_notebook::SemanticIdentityKind::Flow
        | atrament_semantic_notebook::SemanticIdentityKind::List
        | atrament_semantic_notebook::SemanticIdentityKind::ListItem
        | atrament_semantic_notebook::SemanticIdentityKind::Notebook
        | atrament_semantic_notebook::SemanticIdentityKind::OutputProfile
        | atrament_semantic_notebook::SemanticIdentityKind::Page
        | atrament_semantic_notebook::SemanticIdentityKind::Provenance
        | atrament_semantic_notebook::SemanticIdentityKind::Style
        | atrament_semantic_notebook::SemanticIdentityKind::Table
        | atrament_semantic_notebook::SemanticIdentityKind::TableCell => None,
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
